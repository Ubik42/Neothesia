mod midi_backend;
use midi_backend::{MidiBackend, MidiPortInfo};

#[cfg(feature = "synth")]
mod synth_backend;

#[cfg(feature = "vst3-hosting")]
mod vst3_backend;

#[cfg(feature = "synth")]
use synth_backend::SynthBackend;

#[cfg(feature = "vst3-hosting")]
use vst3_backend::Vst3Backend;

use std::fmt::{self, Display, Formatter};

#[cfg(any(feature = "synth", feature = "vst3-hosting"))]
use std::path::PathBuf;

use midi_file::midly::{MidiMessage, num::u4};

#[cfg(test)]
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OutputDescriptor {
    #[cfg(feature = "synth")]
    Synth(Option<PathBuf>),
    #[cfg(feature = "vst3-hosting")]
    Vst3(PathBuf),
    MidiOut(MidiPortInfo),
    DummyOutput,
}

impl OutputDescriptor {
    pub fn is_dummy(&self) -> bool {
        matches!(self, Self::DummyOutput)
    }

    pub fn is_not_dummy(&self) -> bool {
        !self.is_dummy()
    }

    pub fn is_midi(&self) -> bool {
        matches!(self, OutputDescriptor::MidiOut(_))
    }

    pub fn is_synth(&self) -> bool {
        #[cfg(feature = "synth")]
        if matches!(self, OutputDescriptor::Synth(_)) {
            return true;
        }
        false
    }

    pub fn is_vst3(&self) -> bool {
        #[cfg(feature = "vst3-hosting")]
        if matches!(self, OutputDescriptor::Vst3(_)) {
            return true;
        }
        false
    }

    pub fn backend_name(&self) -> &'static str {
        match self {
            #[cfg(feature = "synth")]
            OutputDescriptor::Synth(_) => "SoundFont",
            #[cfg(feature = "vst3-hosting")]
            OutputDescriptor::Vst3(_) => "VST3",
            OutputDescriptor::MidiOut(_) => "MIDI OUT",
            OutputDescriptor::DummyOutput => "NO OUTPUT",
        }
    }

    pub fn status_name(&self) -> String {
        match self {
            #[cfg(feature = "synth")]
            OutputDescriptor::Synth(_) => "SoundFont · Built-in piano".to_owned(),
            #[cfg(feature = "vst3-hosting")]
            OutputDescriptor::Vst3(path) => format!(
                "VST3 · {}",
                path.file_stem().unwrap_or_default().to_string_lossy()
            ),
            OutputDescriptor::MidiOut(info) => format!("MIDI · {info}"),
            OutputDescriptor::DummyOutput => "No output · Click to fix".to_owned(),
        }
    }
}

impl Display for OutputDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "synth")]
            OutputDescriptor::Synth(_) => write!(f, "Buildin Synth"),
            #[cfg(feature = "vst3-hosting")]
            OutputDescriptor::Vst3(path) => write!(
                f,
                "VST3 · {}",
                path.file_stem().unwrap_or_default().to_string_lossy()
            ),
            OutputDescriptor::MidiOut(info) => write!(f, "{info}"),
            OutputDescriptor::DummyOutput => write!(f, "No Output"),
        }
    }
}

#[derive(Clone)]
pub enum OutputConnection {
    Midi(midi_backend::MidiOutputConnection),
    #[cfg(feature = "synth")]
    Synth(synth_backend::SynthOutputConnection),
    #[cfg(feature = "vst3-hosting")]
    Vst3(vst3_backend::Vst3OutputConnection),
    DummyOutput,
    #[cfg(test)]
    Test(Rc<RefCell<Vec<TestOutputEvent>>>),
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TestOutputEvent {
    Midi { channel: u4, message: MidiMessage },
    StopAll,
}

impl OutputConnection {
    pub fn midi_event(&self, channel: u4, msg: MidiMessage) {
        match self {
            OutputConnection::Midi(b) => b.midi_event(channel, msg),
            #[cfg(feature = "synth")]
            OutputConnection::Synth(b) => b.midi_event(channel, msg),
            #[cfg(feature = "vst3-hosting")]
            OutputConnection::Vst3(b) => b.midi_event(channel, msg),
            OutputConnection::DummyOutput => {}
            #[cfg(test)]
            OutputConnection::Test(events) => events.borrow_mut().push(TestOutputEvent::Midi {
                channel,
                message: msg,
            }),
        }
    }
    pub fn set_gain(&self, _gain: f32) {
        match self {
            #[cfg(feature = "synth")]
            OutputConnection::Synth(b) => b.set_gain(_gain),
            #[cfg(feature = "vst3-hosting")]
            OutputConnection::Vst3(_) => {}
            #[cfg(test)]
            OutputConnection::Test(_) => {}
            _ => {}
        }
    }
    pub fn stop_all(&self) {
        match self {
            OutputConnection::Midi(b) => b.stop_all(),
            #[cfg(feature = "synth")]
            OutputConnection::Synth(b) => b.stop_all(),
            #[cfg(feature = "vst3-hosting")]
            OutputConnection::Vst3(b) => b.stop_all(),
            OutputConnection::DummyOutput => {}
            #[cfg(test)]
            OutputConnection::Test(events) => events.borrow_mut().push(TestOutputEvent::StopAll),
        }
    }

    #[cfg(test)]
    pub(crate) fn test() -> (Self, Rc<RefCell<Vec<TestOutputEvent>>>) {
        let events = Rc::new(RefCell::new(Vec::new()));
        (Self::Test(events.clone()), events)
    }
}

pub struct OutputManager {
    #[cfg(feature = "synth")]
    synth_backend: Option<SynthBackend>,
    #[cfg(feature = "vst3-hosting")]
    vst3_backend: Option<Vst3Backend>,
    midi_backend: Option<MidiBackend>,

    output_connection: (OutputDescriptor, OutputConnection),
    last_error: Option<String>,
}

impl Default for OutputManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputManager {
    pub fn new() -> Self {
        #[cfg(feature = "synth")]
        let synth_backend = match SynthBackend::new() {
            Ok(synth_backend) => Some(synth_backend),
            Err(err) => {
                log::error!("{err:?}");
                None
            }
        };

        let midi_backend = match MidiBackend::new() {
            Ok(midi_device_manager) => Some(midi_device_manager),
            Err(e) => {
                log::error!("{e}");
                None
            }
        };

        #[cfg(feature = "vst3-hosting")]
        let vst3_backend = match Vst3Backend::new() {
            Ok(backend) => Some(backend),
            Err(error) => {
                log::error!("VST3 audio backend unavailable: {error}");
                None
            }
        };

        Self {
            #[cfg(feature = "synth")]
            synth_backend,
            #[cfg(feature = "vst3-hosting")]
            vst3_backend,
            midi_backend,

            output_connection: (OutputDescriptor::DummyOutput, OutputConnection::DummyOutput),
            last_error: None,
        }
    }

    pub fn outputs(&self) -> Vec<OutputDescriptor> {
        let mut outs = Vec::new();

        #[cfg(feature = "synth")]
        if let Some(synth) = &self.synth_backend {
            outs.append(&mut synth.get_outputs());
        }
        #[cfg(feature = "vst3-hosting")]
        if let Some(vst3) = &self.vst3_backend {
            outs.append(&mut vst3.get_outputs());
        }
        if let Some(midi) = &self.midi_backend {
            outs.append(&mut midi.get_outputs());
        }

        outs.push(OutputDescriptor::DummyOutput);

        outs
    }

    pub fn connect(&mut self, desc: OutputDescriptor) {
        if desc != self.output_connection.0 {
            // A VST3 module can fail during third-party code loading. Build the
            // replacement first so a bad or moved plug-in leaves the current
            // output alive instead of turning a recoverable selection error
            // into silence.
            #[cfg(feature = "vst3-hosting")]
            if let OutputDescriptor::Vst3(path) = &desc {
                if let Some(vst3) = &self.vst3_backend {
                    match vst3.new_output_connection(path) {
                        Ok(connection) => {
                            self.output_connection.1.stop_all();
                            self.output_connection = (desc, OutputConnection::Vst3(connection));
                            self.last_error = None;
                        }
                        Err(error) => {
                            let message = format!(
                                "Could not load {}: {error}",
                                path.file_stem().unwrap_or_default().to_string_lossy()
                            );
                            log::error!("{message}");
                            self.last_error = Some(message);
                        }
                    }
                } else {
                    self.last_error = Some("VST3 audio backend is unavailable".to_owned());
                }
                return;
            }

            // Silence the currently selected instrument before replacing its
            // connection. This also reaches any player clone sharing the same
            // MIDI backend and prevents held pedal notes from surviving an
            // output change.
            self.output_connection.1.stop_all();
            match desc {
                #[cfg(feature = "synth")]
                OutputDescriptor::Synth(ref font) => {
                    if let Some(ref mut synth) = self.synth_backend {
                        if let Some(font) = font.clone() {
                            self.output_connection = (
                                desc,
                                OutputConnection::Synth(synth.new_output_connection(&font)),
                            );
                        } else if let Some(path) = crate::utils::resources::default_sf2()
                            && path.exists()
                        {
                            self.output_connection = (
                                desc,
                                OutputConnection::Synth(synth.new_output_connection(&path)),
                            );
                        }
                    }
                }
                #[cfg(feature = "vst3-hosting")]
                OutputDescriptor::Vst3(_) => unreachable!("VST3 is connected before this match"),
                OutputDescriptor::MidiOut(ref info) => {
                    if let Some(conn) = MidiBackend::new_output_connection(info) {
                        self.output_connection = (desc, OutputConnection::Midi(conn));
                    }
                }
                OutputDescriptor::DummyOutput => {
                    self.output_connection = (desc, OutputConnection::DummyOutput);
                }
            }
        }
    }

    pub fn connection(&self) -> &OutputConnection {
        &self.output_connection.1
    }

    pub fn descriptor(&self) -> &OutputDescriptor {
        &self.output_connection.0
    }

    pub fn open_vst3_editor(&mut self) -> Result<(), String> {
        #[cfg(feature = "vst3-hosting")]
        if let OutputConnection::Vst3(connection) = &self.output_connection.1 {
            return match connection.open_editor() {
                Ok(()) => {
                    self.last_error = None;
                    Ok(())
                }
                Err(error) => {
                    let message = format!("Could not open the VST3 editor: {error}");
                    self.last_error = Some(message.clone());
                    Err(message)
                }
            };
        }
        let message = "Select and connect a VST3 instrument first".to_owned();
        self.last_error = Some(message.clone());
        Err(message)
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn update(&self) {
        #[cfg(feature = "vst3-hosting")]
        if let OutputConnection::Vst3(connection) = &self.output_connection.1 {
            connection.service_editor();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_status_distinguishes_silent_and_builtin_backends() {
        assert_eq!(OutputDescriptor::DummyOutput.backend_name(), "NO OUTPUT");
        assert_eq!(
            OutputDescriptor::DummyOutput.status_name(),
            "No output · Click to fix"
        );

        #[cfg(feature = "synth")]
        {
            let synth = OutputDescriptor::Synth(None);
            assert_eq!(synth.backend_name(), "SoundFont");
            assert_eq!(synth.status_name(), "SoundFont · Built-in piano");
        }
    }
}
