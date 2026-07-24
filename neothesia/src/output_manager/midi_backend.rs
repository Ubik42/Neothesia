use std::{cell::RefCell, collections::HashSet, rc::Rc};

use crate::output_manager::OutputDescriptor;

use midi_file::midly::{
    self,
    live::LiveEvent,
    num::{u4, u7},
};

const SUSTAIN_PEDAL: u8 = 64;
const ALL_SOUND_OFF: u8 = 120;
const RESET_ALL_CONTROLLERS: u8 = 121;
const ALL_NOTES_OFF: u8 = 123;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ActiveNote {
    key: u7,
    channel: u4,
}

struct MidiOutputConnectionInner {
    conn: midi_io::MidiOutputConnection,
    active_notes: HashSet<ActiveNote>,
    buf: Vec<u8>,
}

#[derive(Clone)]
pub struct MidiOutputConnection {
    inner: Rc<RefCell<MidiOutputConnectionInner>>,
}

impl From<midi_io::MidiOutputConnection> for MidiOutputConnection {
    fn from(conn: midi_io::MidiOutputConnection) -> Self {
        Self {
            inner: Rc::new(RefCell::new(MidiOutputConnectionInner {
                conn,
                active_notes: Default::default(),
                buf: Vec::with_capacity(8),
            })),
        }
    }
}

pub struct MidiBackend {
    manager: midi_io::MidiOutputManager,
}

impl MidiBackend {
    pub fn new() -> Result<Self, midi_io::InitError> {
        Ok(Self {
            manager: midi_io::MidiOutputManager::new()?,
        })
    }

    pub fn get_outputs(&self) -> Vec<OutputDescriptor> {
        let mut outs = Vec::new();
        for (id, port) in self.manager.outputs().into_iter().enumerate() {
            outs.push(OutputDescriptor::MidiOut(MidiPortInfo { id, port }))
        }
        outs
    }

    pub fn new_output_connection(port: &MidiPortInfo) -> Option<MidiOutputConnection> {
        midi_io::MidiOutputManager::connect_output(port.port.clone())
            .map(MidiOutputConnection::from)
    }
}

impl MidiOutputConnection {
    pub fn midi_event(&self, channel: u4, message: midly::MidiMessage) {
        let inner = &mut *self.inner.borrow_mut();
        track_active_note(&mut inner.active_notes, channel, message);
        let _ = send_message(inner, channel, message);
    }

    pub fn stop_all(&self) {
        let inner = &mut *self.inner.borrow_mut();
        for note in std::mem::take(&mut inner.active_notes).iter() {
            if !send_message(
                inner,
                note.channel,
                midly::MidiMessage::NoteOff {
                    key: note.key,
                    vel: u7::new(0),
                },
            ) {
                return;
            }
        }

        // Explicit note-offs are not enough when a sustain pedal is held, and
        // some instruments do not implement every channel-mode message. Send
        // the complete conservative panic sequence on every MIDI channel.
        for (channel, message) in panic_events() {
            if !send_message(inner, channel, message) {
                return;
            }
        }
    }
}

impl Drop for MidiOutputConnection {
    fn drop(&mut self) {
        // Clones share one physical connection. A temporary clone going out of
        // scope must not silence another live owner.
        if Rc::strong_count(&self.inner) == 1 {
            self.stop_all();
        }
    }
}

fn track_active_note(
    active_notes: &mut HashSet<ActiveNote>,
    channel: u4,
    message: midly::MidiMessage,
) {
    match message {
        midly::MidiMessage::NoteOff { key, .. } => {
            active_notes.remove(&ActiveNote { key, channel });
        }
        midly::MidiMessage::NoteOn { key, vel } => {
            if vel.as_int() == 0 {
                active_notes.remove(&ActiveNote { key, channel });
            } else {
                active_notes.insert(ActiveNote { key, channel });
            }
        }
        _ => {}
    }
}

fn panic_messages() -> [midly::MidiMessage; 4] {
    [
        controller(SUSTAIN_PEDAL, 0),
        controller(ALL_NOTES_OFF, 0),
        controller(ALL_SOUND_OFF, 0),
        controller(RESET_ALL_CONTROLLERS, 0),
    ]
}

fn panic_events() -> impl Iterator<Item = (u4, midly::MidiMessage)> {
    (0..16).flat_map(|channel| {
        panic_messages()
            .into_iter()
            .map(move |message| (u4::new(channel), message))
    })
}

fn controller(number: u8, value: u8) -> midly::MidiMessage {
    midly::MidiMessage::Controller {
        controller: u7::new(number),
        value: u7::new(value),
    }
}

fn send_message(
    inner: &mut MidiOutputConnectionInner,
    channel: u4,
    message: midly::MidiMessage,
) -> bool {
    encode_message(channel, message, &mut inner.buf);
    match inner.conn.send(&inner.buf) {
        Ok(()) => true,
        Err(error) => {
            log::warn!("Could not send MIDI output event: {error}");
            false
        }
    }
}

fn encode_message(channel: u4, message: midly::MidiMessage, buffer: &mut Vec<u8>) {
    buffer.clear();
    LiveEvent::Midi { channel, message }
        .write(buffer)
        .expect("serializing a channel MIDI message into memory cannot fail");
}

#[derive(Clone, Debug, Eq)]
pub struct MidiPortInfo {
    id: usize,
    port: midi_io::MidiOutputPort,
}

impl PartialEq for MidiPortInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.port == other.port
    }
}

impl std::fmt::Display for MidiPortInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_velocity_note_on_releases_active_note() {
        let mut active = HashSet::new();
        let channel = u4::new(3);
        let key = u7::new(60);
        track_active_note(
            &mut active,
            channel,
            midly::MidiMessage::NoteOn {
                key,
                vel: u7::new(100),
            },
        );
        assert_eq!(active.len(), 1);

        track_active_note(
            &mut active,
            channel,
            midly::MidiMessage::NoteOn {
                key,
                vel: u7::new(0),
            },
        );
        assert!(active.is_empty());
    }

    #[test]
    fn panic_releases_pedal_notes_sound_and_controllers() {
        let controllers: Vec<_> = panic_messages()
            .into_iter()
            .map(|message| match message {
                midly::MidiMessage::Controller { controller, value } => {
                    (controller.as_int(), value.as_int())
                }
                _ => panic!("panic sequence must contain only controller messages"),
            })
            .collect();

        assert_eq!(
            controllers,
            vec![
                (SUSTAIN_PEDAL, 0),
                (ALL_NOTES_OFF, 0),
                (ALL_SOUND_OFF, 0),
                (RESET_ALL_CONTROLLERS, 0),
            ]
        );
    }

    #[test]
    fn panic_sequence_covers_every_channel() {
        let events: Vec<_> = panic_events().collect();
        assert_eq!(events.len(), 16 * panic_messages().len());
        for channel in 0..16 {
            assert_eq!(
                events
                    .iter()
                    .filter(|(event_channel, _)| event_channel.as_int() == channel)
                    .count(),
                panic_messages().len()
            );
        }
    }

    #[test]
    fn expressive_messages_serialize_without_losing_values() {
        let channel = u4::new(2);
        let mut bytes = Vec::new();

        encode_message(channel, controller(SUSTAIN_PEDAL, 23), &mut bytes);
        assert_eq!(bytes, [0xB2, SUSTAIN_PEDAL, 23]);

        let bend = midly::PitchBend::from_int(1_337);
        let raw = bend.0.as_int();
        encode_message(channel, midly::MidiMessage::PitchBend { bend }, &mut bytes);
        assert_eq!(bytes, [0xE2, (raw & 0x7F) as u8, (raw >> 7) as u8]);

        encode_message(
            channel,
            midly::MidiMessage::ChannelAftertouch { vel: u7::new(77) },
            &mut bytes,
        );
        assert_eq!(bytes, [0xD2, 77]);
    }
}
