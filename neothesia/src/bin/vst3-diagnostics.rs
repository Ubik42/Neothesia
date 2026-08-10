use std::{path::PathBuf, process::ExitCode, thread, time::Duration};

use cpal::traits::{DeviceTrait, HostTrait};
use vst3_host::{
    Vst3Host,
    midi::{MidiChannel, MidiEvent},
};

#[derive(Debug, Default, Eq, PartialEq)]
struct Arguments {
    plugin: Option<PathBuf>,
    play_note: bool,
}

fn parse_arguments(args: impl IntoIterator<Item = String>) -> Result<Arguments, String> {
    let mut parsed = Arguments::default();
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--plugin" => {
                parsed.plugin = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--plugin requires a VST3 path".to_owned())?,
                ));
            }
            "--play-note" => parsed.play_note = true,
            "--help" | "-h" => return Err(String::new()),
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }
    Ok(parsed)
}

fn usage() {
    println!("Usage: vst3-diagnostics [--plugin <path.vst3>] [--play-note]");
}

fn default_pianoteq_path() -> Option<PathBuf> {
    let common = std::env::var_os("COMMONPROGRAMFILES")?;
    std::fs::read_dir(PathBuf::from(common).join("VST3"))
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("vst3"))
                && path
                    .file_stem()
                    .is_some_and(|name| name.to_string_lossy().contains("Pianoteq"))
        })
}

fn run(arguments: Arguments) -> Result<(), String> {
    let path = arguments
        .plugin
        .or_else(default_pianoteq_path)
        .ok_or_else(|| "Pianoteq VST3 was not found; pass --plugin <path.vst3>".to_owned())?;
    if !path.exists() {
        return Err(format!("plugin does not exist: {}", path.display()));
    }

    let cpal_host = cpal::default_host();
    let device = cpal_host
        .default_output_device()
        .ok_or_else(|| "no default audio output device".to_owned())?;
    let device_config = device
        .default_output_config()
        .map_err(|error| format!("cannot read output configuration: {error}"))?;
    let sample_rate = device_config.sample_rate() as f64;
    let block_size = 256;

    let mut host = Vst3Host::builder()
        .sample_rate(sample_rate)
        .block_size(block_size)
        .input_channels(0)
        .output_channels(2)
        .build()
        .map_err(|error| format!("cannot create VST3 host: {error}"))?;
    let mut plugin = host
        .load_plugin(&path)
        .map_err(|error| format!("cannot load plugin: {error}"))?;
    let info = plugin.info().clone();
    let state = plugin
        .save_state()
        .map_err(|error| format!("cannot snapshot plugin state: {error}"))?;
    plugin
        .load_state(&state)
        .map_err(|error| format!("cannot restore plugin state: {error}"))?;
    let state_size = state.len();

    println!("VST3 diagnostics: PASS");
    println!("Path: {}", path.display());
    println!("Plugin: {} {} by {}", info.name, info.version, info.vendor);
    println!("Category: {}", info.category);
    println!("MIDI input: {}", info.has_midi_input);
    println!(
        "Audio buses: {} in / {} out",
        info.audio_inputs, info.audio_outputs
    );
    println!("Editor: {}", info.has_gui);
    println!("State round trip: PASS ({state_size} bytes)");
    println!("Audio: {sample_rate} Hz / {block_size} samples");

    if arguments.play_note {
        let audio = host
            .play(plugin)
            .map_err(|error| format!("cannot start plugin audio: {error}"))?;
        if !audio.send_midi(MidiEvent::NoteOn {
            channel: MidiChannel::Ch1,
            note: 60,
            velocity: 90,
        }) {
            return Err("MIDI queue rejected note-on".to_owned());
        }
        thread::sleep(Duration::from_millis(900));
        if !audio.send_midi(MidiEvent::NoteOff {
            channel: MidiChannel::Ch1,
            note: 60,
            velocity: 0,
        }) {
            return Err("MIDI queue rejected note-off".to_owned());
        }
        thread::sleep(Duration::from_millis(300));
        println!("Audition: PASS (C4 note-on/note-off)");
    }

    Ok(())
}

fn main() -> ExitCode {
    let arguments = match parse_arguments(std::env::args().skip(1)) {
        Ok(arguments) => arguments,
        Err(error) => {
            usage();
            if error.is_empty() {
                return ExitCode::SUCCESS;
            }
            eprintln!("ERROR: {error}");
            return ExitCode::from(2);
        }
    };

    match run(arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ERROR: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plugin_and_audition_flag() {
        assert_eq!(
            parse_arguments([
                "--plugin".to_owned(),
                "piano.vst3".to_owned(),
                "--play-note".to_owned(),
            ]),
            Ok(Arguments {
                plugin: Some(PathBuf::from("piano.vst3")),
                play_note: true,
            })
        );
    }

    #[test]
    fn rejects_missing_plugin_argument() {
        assert_eq!(
            parse_arguments(["--plugin".to_owned()]),
            Err("--plugin requires a VST3 path".to_owned())
        );
    }

    #[test]
    fn rejects_unknown_argument() {
        assert_eq!(
            parse_arguments(["--wat".to_owned()]),
            Err("unknown argument: --wat".to_owned())
        );
    }
}
