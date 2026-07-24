use std::process::ExitCode;

use neothesia_core::config::Config;

#[derive(Debug, Default, Eq, PartialEq)]
struct Arguments {
    expected_output: Option<String>,
    require_saved_output: bool,
    probe_output: bool,
}

fn parse_arguments(args: impl IntoIterator<Item = String>) -> Result<Arguments, String> {
    let mut parsed = Arguments::default();
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--expect-output" => {
                parsed.expected_output = Some(
                    args.next()
                        .ok_or_else(|| "--expect-output requires a port name".to_owned())?,
                );
            }
            "--require-saved-output" => parsed.require_saved_output = true,
            "--probe-output" => parsed.probe_output = true,
            "--help" | "-h" => return Err(String::new()),
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    if (parsed.require_saved_output || parsed.probe_output) && parsed.expected_output.is_none() {
        return Err(
            "--require-saved-output and --probe-output require --expect-output <name>".to_owned(),
        );
    }

    Ok(parsed)
}

fn usage() {
    println!(
        "Usage: midi-diagnostics [--expect-output <exact-name>] \
         [--require-saved-output] [--probe-output]"
    );
}

fn print_ports(label: &str, ports: &[String]) {
    println!("{label} ({}):", ports.len());
    if ports.is_empty() {
        println!("  (none)");
    } else {
        for port in ports {
            println!("  - {port}");
        }
    }
}

fn run(arguments: Arguments) -> Result<(), String> {
    let input_manager = midi_io::MidiInputManager::new()
        .map_err(|error| format!("MIDI input initialization failed: {error}"))?;
    let output_manager = midi_io::MidiOutputManager::new()
        .map_err(|error| format!("MIDI output initialization failed: {error}"))?;
    let inputs: Vec<_> = input_manager
        .inputs()
        .into_iter()
        .map(|port| port.to_string())
        .collect();
    let outputs: Vec<_> = output_manager
        .outputs()
        .into_iter()
        .map(|port| port.to_string())
        .collect();
    let config = Config::new();
    let saved_input = config.input().unwrap_or("(not selected)");
    let saved_output = config.output().unwrap_or("(not selected)");

    println!("Neothesia MIDI diagnostics");
    print_ports("MIDI inputs", &inputs);
    print_ports("MIDI outputs", &outputs);
    println!("Saved input: {saved_input}");
    println!("Saved output: {saved_output}");

    let Some(expected) = arguments.expected_output else {
        return Ok(());
    };

    if !outputs.iter().any(|port| port == &expected) {
        return Err(format!(
            "expected output '{expected}' is not visible to Neothesia"
        ));
    }
    println!("Expected output: FOUND ({expected})");

    if arguments.require_saved_output {
        if config.output() != Some(expected.as_str()) {
            return Err(format!(
                "expected output is visible, but Neothesia saved '{saved_output}' instead"
            ));
        }
        println!("Saved selection: MATCH");
    }

    if arguments.probe_output {
        if !output_manager.probe_output(&expected) {
            return Err(format!(
                "output '{expected}' is visible but could not be opened"
            ));
        }
        println!("Open probe: PASS (no MIDI data sent)");
    }

    Ok(())
}

fn main() -> ExitCode {
    let arguments = match parse_arguments(std::env::args().skip(1)) {
        Ok(arguments) => arguments,
        Err(error) => {
            usage();
            if !error.is_empty() {
                eprintln!("ERROR: {error}");
                return ExitCode::from(2);
            }
            return ExitCode::SUCCESS;
        }
    };

    match run(arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ROUTE NOT READY: {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Result<Arguments, String> {
        parse_arguments(values.iter().map(|value| (*value).to_owned()))
    }

    #[test]
    fn parses_complete_route_check() {
        assert_eq!(
            args(&[
                "--expect-output",
                "Neothesia to Pianoteq",
                "--require-saved-output",
                "--probe-output"
            ]),
            Ok(Arguments {
                expected_output: Some("Neothesia to Pianoteq".to_owned()),
                require_saved_output: true,
                probe_output: true,
            })
        );
    }

    #[test]
    fn rejects_probe_without_named_output() {
        assert!(args(&["--probe-output"]).is_err());
    }
}
