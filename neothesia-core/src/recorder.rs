use midi_file::midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
};
use std::{
    collections::HashSet,
    fmt,
    path::PathBuf,
    time::{Duration, Instant},
};
const TICKS_PER_BEAT: u16 = 480;
const TEMPO_MICROS_PER_BEAT: u32 = 500_000;
const TICKS_PER_SECOND: f64 = TICKS_PER_BEAT as f64 * 1_000_000.0 / TEMPO_MICROS_PER_BEAT as f64;

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum RecorderError {
    #[error("No note events recorded")]
    NoNotesFound,
    #[error("Failed to write MIDI file")]
    Write,
    #[error("{0}")]
    MidiFileParse(String),
}

#[derive(Default, Debug)]
pub enum RecorderStatus {
    #[default]
    Idle,
    RecordingFinished(Duration),
    Saved(PathBuf),
    Error(RecorderError),
}

impl fmt::Display for RecorderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => {}
            Self::RecordingFinished(duration) => {
                write!(f, "Recorded {:.1}s", duration.as_secs_f32())?;
            }
            Self::Error(err) => {
                write!(f, "{err}")?;
            }
            Self::Saved(path) => {
                write!(f, "Saved recording to {}", path.display())?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct RecordedMidiEvent {
    timestamp: Duration,
    channel: u8,
    message: MidiMessage,
}

pub struct RecordingInProgressState {
    started_at: Instant,
    events: Vec<RecordedMidiEvent>,
    active_notes: HashSet<(u8, u8)>,
}

impl RecordingInProgressState {
    fn finish_active_notes(&mut self, timestamp: Duration) {
        let channels: HashSet<_> = self.events.iter().map(|e| e.channel).collect();
        for channel in channels {
            self.events.push(RecordedMidiEvent {
                timestamp,
                channel,
                message: MidiMessage::Controller {
                    controller: 64.into(),
                    value: 0.into(),
                },
            });
        }
        let mut active_notes: Vec<_> = self.active_notes.drain().collect();

        // TODO: What's the point of this sort?
        active_notes.sort_unstable();

        for (channel, key) in active_notes {
            self.events.push(RecordedMidiEvent {
                timestamp,
                channel,
                message: MidiMessage::NoteOff {
                    key: key.into(),
                    vel: 0.into(),
                },
            });
        }
    }
}

pub struct RecordedTake {
    duration: Duration,
    smf: Smf<'static>,
}

#[derive(Default)]
enum RecorderState {
    #[default]
    Idle,
    Recording(RecordingInProgressState),
    Recorded(RecordedTake),
}

#[derive(Default)]
pub struct FreeplayRecorder {
    state: RecorderState,
}

impl FreeplayRecorder {
    pub fn is_recording(&self) -> bool {
        matches!(self.state, RecorderState::Recording(_))
    }

    pub fn start(&mut self) {
        self.state = RecorderState::Recording(RecordingInProgressState {
            started_at: Instant::now(),
            events: Vec::new(),
            active_notes: HashSet::new(),
        });
    }

    pub fn stop(&mut self) -> Result<&Smf<'static>, RecorderError> {
        let state = std::mem::take(&mut self.state);
        let RecorderState::Recording(mut in_progress) = state else {
            return Err(RecorderError::NoNotesFound);
        };

        let stop_time = in_progress.started_at.elapsed();
        in_progress.finish_active_notes(stop_time);

        let smf = to_smf(&in_progress.events)?;

        self.state = RecorderState::Recorded(RecordedTake {
            duration: stop_time,
            smf,
        });

        self.as_smf()
    }

    pub fn duration(&self) -> Duration {
        match &self.state {
            RecorderState::Idle => Duration::ZERO,
            RecorderState::Recording(state) => state.started_at.elapsed(),
            RecorderState::Recorded(recorded_take) => recorded_take.duration,
        }
    }

    pub fn push_event(&mut self, channel: u8, message: MidiMessage) {
        let RecorderState::Recording(in_progress) = &mut self.state else {
            return;
        };

        let timestamp = in_progress.started_at.elapsed();
        in_progress.events.push(RecordedMidiEvent {
            timestamp,
            channel,
            message,
        });

        match message {
            MidiMessage::NoteOn { key, vel } if vel.as_int() != 0 => {
                in_progress.active_notes.insert((channel, key.as_int()));
            }
            MidiMessage::NoteOff { key, .. } | MidiMessage::NoteOn { key, .. } => {
                in_progress.active_notes.remove(&(channel, key.as_int()));
            }
            _ => {}
        }
    }

    pub fn as_smf(&self) -> Result<&Smf<'static>, RecorderError> {
        let RecorderState::Recorded(recorded_take) = &self.state else {
            return Err(RecorderError::NoNotesFound);
        };

        Ok(&recorded_take.smf)
    }
}

fn duration_to_ticks(duration: Duration) -> u32 {
    (duration.as_secs_f64() * TICKS_PER_SECOND).round() as u32
}

fn to_smf(events: &[RecordedMidiEvent]) -> Result<Smf<'static>, RecorderError> {
    // Preview/export requires at least one played note, not just release/control data.
    let has_note_events = events
        .iter()
        .any(|event| matches!(event.message, MidiMessage::NoteOn { vel,.. } if vel.as_int()!=0));

    if !has_note_events {
        return Err(RecorderError::NoNotesFound);
    }

    let mut track = vec![
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(TEMPO_MICROS_PER_BEAT.into())),
        },
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
        },
    ];

    let mut previous_ticks = 0u32;
    for event in events {
        let current_ticks = duration_to_ticks(event.timestamp);
        let delta_ticks = current_ticks.saturating_sub(previous_ticks);
        previous_ticks = current_ticks;

        track.push(TrackEvent {
            delta: delta_ticks.into(),
            kind: TrackEventKind::Midi {
                channel: event.channel.into(),
                message: event.message,
            },
        });
    }

    track.push(TrackEvent {
        delta: 1.into(),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });

    Ok(Smf {
        header: Header {
            format: Format::SingleTrack,
            timing: Timing::Metrical(TICKS_PER_BEAT.into()),
        },
        tracks: vec![track],
    })
}

#[cfg(test)]
mod freeplay_recorder_tests {
    use super::*;

    #[test]
    fn restarting_recording_discards_previous_take_and_resets_event_count() {
        let mut recorder = FreeplayRecorder::default();

        recorder.start();
        recorder.push_event(
            0,
            MidiMessage::NoteOn {
                key: 60.into(),
                vel: 100.into(),
            },
        );

        assert!(recorder.stop().is_ok());

        recorder.start();

        assert!(recorder.is_recording());

        let error = recorder.stop().expect_err("Empty");
        assert_eq!(error, RecorderError::NoNotesFound);
    }

    #[test]
    fn pedal_only_recording_is_rejected_for_preview_song() {
        let mut recorder = FreeplayRecorder::default();

        recorder.start();
        recorder.push_event(
            0,
            MidiMessage::Controller {
                controller: 64.into(),
                value: 127.into(),
            },
        );
        recorder.push_event(
            0,
            MidiMessage::Controller {
                controller: 64.into(),
                value: 0.into(),
            },
        );

        let error = recorder
            .stop()
            .expect_err("pedal-only recordings should not create preview songs");
        assert_eq!(error, RecorderError::NoNotesFound);
    }

    #[test]
    fn note_off_only_recording_is_rejected_for_preview_song() {
        let mut recorder = FreeplayRecorder::default();

        recorder.start();
        recorder.push_event(
            0,
            MidiMessage::NoteOff {
                key: 60.into(),
                vel: 0.into(),
            },
        );

        let error = recorder
            .stop()
            .expect_err("note-off-only recordings should not create preview songs");
        assert_eq!(error, RecorderError::NoNotesFound);
    }
}
