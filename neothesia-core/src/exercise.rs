use piano_layout::KeyboardRange;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::practice::PracticePart;
use midi_file::{
    MidiFile,
    midly::{
        Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
        num::{u4, u7, u15, u24, u28},
    },
};

const TICKS_PER_BEAT: u16 = 480;
const NOTE_GATE_TICKS: u32 = 384;
const RIGHT_TRACK_NAME: &[u8] = b"Right Hand";
const LEFT_TRACK_NAME: &[u8] = b"Left Hand";

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub enum ExercisePattern {
    Scale,
    Arpeggio,
    PrimaryChords,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub enum ExerciseTonality {
    Major,
    Minor,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum ExerciseMinorForm {
    #[default]
    Natural,
    Harmonic,
    Melodic,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub enum ExerciseDirection {
    Ascending,
    Descending,
    UpAndDown,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub enum ExerciseHands {
    Right,
    Left,
    Both,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExerciseSpec {
    /// Pitch class where C = 0 and B = 11.
    pub tonic: u8,
    pub tonality: ExerciseTonality,
    #[serde(default)]
    pub minor_form: ExerciseMinorForm,
    pub pattern: ExercisePattern,
    pub direction: ExerciseDirection,
    pub hands: ExerciseHands,
    pub octaves: u8,
    #[serde(default = "default_repetitions")]
    pub repetitions: u8,
    pub tempo_bpm: u16,
}

impl Default for ExerciseSpec {
    fn default() -> Self {
        Self {
            tonic: 0,
            tonality: ExerciseTonality::Major,
            minor_form: ExerciseMinorForm::Natural,
            pattern: ExercisePattern::Scale,
            direction: ExerciseDirection::UpAndDown,
            hands: ExerciseHands::Both,
            octaves: 1,
            repetitions: 1,
            tempo_bpm: 60,
        }
    }
}

impl ExerciseSpec {
    pub fn validate(self) -> Result<(), ExerciseError> {
        validate_spec(self)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ExerciseNote {
    pub midi_note: u8,
    pub part: PracticePart,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExerciseMoment {
    pub notes: Vec<ExerciseNote>,
    pub beats: u8,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExercisePlan {
    pub spec: ExerciseSpec,
    pub moments: Vec<ExerciseMoment>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExerciseFingerings {
    pub right: Vec<u8>,
    pub left: Vec<u8>,
}

#[derive(Debug, Clone, Eq, Error, PartialEq)]
pub enum ExerciseError {
    #[error("tonic must be a pitch class from 0 to 11")]
    InvalidTonic,
    #[error("exercise span must be between one and three octaves")]
    InvalidOctaves,
    #[error("harmonic and melodic minor forms apply only to minor scales")]
    InvalidMinorForm,
    #[error("exercise repetitions must be between one and eight")]
    InvalidRepetitions,
    #[error("exercise tempo must be between 20 and 240 BPM")]
    InvalidTempo,
    #[error("exercise notes do not fit the configured keyboard range")]
    OutsideKeyboardRange,
}

impl ExercisePlan {
    pub fn generate(
        spec: ExerciseSpec,
        keyboard_range: &KeyboardRange,
    ) -> Result<Self, ExerciseError> {
        spec.validate()?;

        let phrase = match spec.pattern {
            ExercisePattern::Scale => {
                scale_moments(spec.tonality, spec.minor_form, spec.octaves, spec.direction)
            }
            ExercisePattern::Arpeggio => melodic_moments(
                arpeggio_intervals(spec.tonality),
                spec.octaves,
                spec.direction,
            ),
            ExercisePattern::PrimaryChords => {
                chord_moments(spec.tonality, spec.octaves, spec.direction)
            }
        };
        let relative_moments: Vec<_> = (0..spec.repetitions)
            .flat_map(|_| phrase.iter().cloned())
            .collect();
        let right_root = 60 + spec.tonic;
        let left_root = 36 + spec.tonic;
        let mut moments = Vec::with_capacity(relative_moments.len());
        for relative in relative_moments {
            let mut notes = Vec::new();
            for &part in selected_parts(spec.hands) {
                let root = match part {
                    PracticePart::RightHand => right_root,
                    PracticePart::LeftHand => left_root,
                    PracticePart::Other => unreachable!(),
                };
                for offset in &relative {
                    let note = root
                        .checked_add(*offset)
                        .ok_or(ExerciseError::OutsideKeyboardRange)?;
                    if !keyboard_range.contains(note) {
                        return Err(ExerciseError::OutsideKeyboardRange);
                    }
                    notes.push(ExerciseNote {
                        midi_note: note,
                        part,
                    });
                }
            }
            notes.sort_unstable_by_key(|note| note.midi_note);
            moments.push(ExerciseMoment {
                notes,
                beats: if spec.pattern == ExercisePattern::PrimaryChords {
                    2
                } else {
                    1
                },
            });
        }

        Ok(Self { spec, moments })
    }

    pub fn display_name(&self) -> String {
        let tonic = tonic_display_name(self.spec.tonic, self.spec.tonality);
        let tonality = match (self.spec.tonality, self.spec.minor_form) {
            (ExerciseTonality::Major, _) => "Major",
            (ExerciseTonality::Minor, ExerciseMinorForm::Natural) => "Natural Minor",
            (ExerciseTonality::Minor, ExerciseMinorForm::Harmonic) => "Harmonic Minor",
            (ExerciseTonality::Minor, ExerciseMinorForm::Melodic) => "Melodic Minor",
        };
        let pattern = match self.spec.pattern {
            ExercisePattern::Scale => "Scale",
            ExercisePattern::Arpeggio => "Arpeggio",
            ExercisePattern::PrimaryChords => "Primary Chords",
        };
        let hands = match self.spec.hands {
            ExerciseHands::Right => "Right Hand",
            ExerciseHands::Left => "Left Hand",
            ExerciseHands::Both => "Both Hands",
        };
        let repetitions = if self.spec.repetitions == 1 {
            String::new()
        } else {
            format!(" · x{}", self.spec.repetitions)
        };
        format!(
            "{tonic} {tonality} {pattern} · {hands}{repetitions} · {} BPM",
            self.spec.tempo_bpm,
        )
    }

    pub fn beats_per_repetition(&self) -> u32 {
        let moments_per_repetition = self.moments.len() / usize::from(self.spec.repetitions);
        self.moments[..moments_per_repetition]
            .iter()
            .map(|moment| u32::from(moment.beats))
            .sum()
    }

    /// Stable learning-history identity for the musical task. Tempo and hand
    /// scope are attempt dimensions, so they intentionally do not split the
    /// history of the same exercise.
    pub fn practice_id(&self) -> String {
        let tonality = match (self.spec.tonality, self.spec.minor_form) {
            (ExerciseTonality::Major, _) => "major",
            (ExerciseTonality::Minor, ExerciseMinorForm::Natural) => "natural-minor",
            (ExerciseTonality::Minor, ExerciseMinorForm::Harmonic) => "harmonic-minor",
            (ExerciseTonality::Minor, ExerciseMinorForm::Melodic) => "melodic-minor",
        };
        let pattern = match self.spec.pattern {
            ExercisePattern::Scale => "scale",
            ExercisePattern::Arpeggio => "arpeggio",
            ExercisePattern::PrimaryChords => "primary-chords",
        };
        let direction = match self.spec.direction {
            ExerciseDirection::Ascending => "ascending",
            ExerciseDirection::Descending => "descending",
            ExerciseDirection::UpAndDown => "up-down",
        };
        let canonical = format!(
            "neothesia-exercise:v1:{}:{tonality}:{pattern}:{direction}:{}",
            self.spec.tonic, self.spec.octaves
        );
        blake3::hash(canonical.as_bytes()).to_hex().to_string()
    }

    /// Returns reviewed fingering for the exact generated note sequence.
    ///
    /// Supported tables cover every major and all three minor forms in every
    /// key. Unsupported patterns deliberately return `None` rather than
    /// guessing.
    pub fn fingerings(&self) -> Option<ExerciseFingerings> {
        let is_reviewed_major = self.spec.tonality == ExerciseTonality::Major;
        let is_reviewed_minor = self.spec.tonality == ExerciseTonality::Minor;
        if self.spec.pattern == ExercisePattern::PrimaryChords
            || (!is_reviewed_major && !is_reviewed_minor)
        {
            return None;
        }
        let fingering_for = |part| match self.spec.pattern {
            ExercisePattern::Scale => (
                ascending_scale_fingering(
                    self.spec.tonic,
                    self.spec.tonality,
                    self.spec.minor_form,
                    part,
                    self.spec.octaves,
                ),
                descending_scale_fingering(
                    self.spec.tonic,
                    self.spec.tonality,
                    self.spec.minor_form,
                    part,
                    self.spec.octaves,
                ),
            ),
            ExercisePattern::Arpeggio => {
                let ascending = ascending_arpeggio_fingering(
                    self.spec.tonic,
                    self.spec.tonality,
                    part,
                    self.spec.octaves,
                );
                (ascending.clone(), ascending)
            }
            ExercisePattern::PrimaryChords => unreachable!("filtered above"),
        };
        let (right_ascending, right_descending) = fingering_for(PracticePart::RightHand);
        let right = directional_fingering(
            right_ascending,
            right_descending,
            self.spec.direction,
            self.spec.repetitions,
        );
        let (left_ascending, left_descending) = fingering_for(PracticePart::LeftHand);
        let left = directional_fingering(
            left_ascending,
            left_descending,
            self.spec.direction,
            self.spec.repetitions,
        );
        Some(ExerciseFingerings {
            right: if self.spec.hands == ExerciseHands::Left {
                Vec::new()
            } else {
                right
            },
            left: if self.spec.hands == ExerciseHands::Right {
                Vec::new()
            } else {
                left
            },
        })
    }

    pub fn to_midi_file(&self) -> Result<MidiFile, String> {
        let total_ticks = self
            .moments
            .iter()
            .map(|moment| u32::from(moment.beats) * u32::from(TICKS_PER_BEAT))
            .sum();
        let micros_per_beat = 60_000_000 / u32::from(self.spec.tempo_bpm);
        let conductor = vec![
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(micros_per_beat))),
            },
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
            },
            TrackEvent {
                delta: u28::new(total_ticks),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            },
        ];
        let mut tracks = vec![conductor];
        match self.spec.hands {
            ExerciseHands::Right => {
                tracks.push(self.midi_track(PracticePart::RightHand, 0, RIGHT_TRACK_NAME));
            }
            ExerciseHands::Left => {
                tracks.push(self.midi_track(PracticePart::LeftHand, 1, LEFT_TRACK_NAME));
            }
            ExerciseHands::Both => {
                tracks.push(self.midi_track(PracticePart::RightHand, 0, RIGHT_TRACK_NAME));
                tracks.push(self.midi_track(PracticePart::LeftHand, 1, LEFT_TRACK_NAME));
            }
        }
        let smf = Smf {
            header: Header {
                format: Format::Parallel,
                timing: Timing::Metrical(u15::new(TICKS_PER_BEAT)),
            },
            tracks,
        };
        MidiFile::from_smf(self.display_name(), &smf)
    }

    fn midi_track(
        &self,
        part: PracticePart,
        channel: u8,
        name: &'static [u8],
    ) -> Vec<TrackEvent<'static>> {
        let mut events = vec![TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::TrackName(name)),
        }];
        let mut cursor = 0u32;
        let mut previous_event = 0u32;
        for moment in &self.moments {
            let notes: Vec<_> = moment
                .notes
                .iter()
                .filter(|note| note.part == part)
                .map(|note| note.midi_note)
                .collect();
            let step_ticks = u32::from(moment.beats) * u32::from(TICKS_PER_BEAT);
            let gate_ticks = NOTE_GATE_TICKS * u32::from(moment.beats);
            for (index, note) in notes.iter().enumerate() {
                events.push(TrackEvent {
                    delta: u28::new(if index == 0 {
                        cursor - previous_event
                    } else {
                        0
                    }),
                    kind: TrackEventKind::Midi {
                        channel: u4::new(channel),
                        message: MidiMessage::NoteOn {
                            key: u7::new(*note),
                            vel: u7::new(80),
                        },
                    },
                });
                previous_event = cursor;
            }
            for (index, note) in notes.iter().enumerate() {
                let note_end = cursor + gate_ticks;
                events.push(TrackEvent {
                    delta: u28::new(if index == 0 {
                        note_end - previous_event
                    } else {
                        0
                    }),
                    kind: TrackEventKind::Midi {
                        channel: u4::new(channel),
                        message: MidiMessage::NoteOff {
                            key: u7::new(*note),
                            vel: u7::new(0),
                        },
                    },
                });
                previous_event = note_end;
            }
            cursor += step_ticks;
        }
        events.push(TrackEvent {
            delta: u28::new(cursor - previous_event),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        events
    }
}

fn ascending_scale_fingering(
    tonic: u8,
    tonality: ExerciseTonality,
    minor_form: ExerciseMinorForm,
    part: PracticePart,
    octaves: u8,
) -> Vec<u8> {
    let note_count = usize::from(octaves) * 7 + 1;
    if tonality == ExerciseTonality::Minor {
        if minor_form == ExerciseMinorForm::Natural && tonic != 0 {
            return ascending_natural_minor_fingering(tonic, part, note_count);
        }
        if minor_form == ExerciseMinorForm::Harmonic {
            return ascending_harmonic_minor_fingering(tonic, part, note_count);
        }
        if minor_form == ExerciseMinorForm::Melodic {
            return ascending_melodic_minor_fingering(tonic, part, note_count);
        }
    }
    ascending_major_fingering(tonic, part, note_count)
}

fn descending_scale_fingering(
    tonic: u8,
    tonality: ExerciseTonality,
    minor_form: ExerciseMinorForm,
    part: PracticePart,
    octaves: u8,
) -> Vec<u8> {
    if tonality == ExerciseTonality::Minor && minor_form == ExerciseMinorForm::Melodic {
        ascending_natural_minor_fingering(tonic, part, usize::from(octaves) * 7 + 1)
    } else {
        ascending_scale_fingering(tonic, tonality, minor_form, part, octaves)
    }
}

fn ascending_major_fingering(tonic: u8, part: PracticePart, note_count: usize) -> Vec<u8> {
    match part {
        PracticePart::RightHand => match tonic {
            0 | 2 | 4 | 7 | 9 | 11 => {
                scale_fingering(note_count, 1, &[2, 3, 1, 2, 3, 4, 1], Some(5))
            }
            1 => scale_fingering(note_count, 2, &[3, 1, 2, 3, 4, 1, 2], None),
            3 => scale_fingering(note_count, 3, &[1, 2, 3, 4, 1, 2, 3], None),
            5 => scale_fingering(note_count, 1, &[2, 3, 4, 1, 2, 3, 1], Some(4)),
            6 => scale_fingering(note_count, 2, &[3, 4, 1, 2, 3, 1, 2], None),
            8 => scale_fingering(note_count, 3, &[4, 1, 2, 3, 1, 2, 3], None),
            10 => scale_fingering(note_count, 2, &[1, 2, 3, 1, 2, 3, 4], None),
            _ => unreachable!("validated tonic must be a pitch class"),
        },
        PracticePart::LeftHand => match tonic {
            0 | 2 | 4 | 5 | 7 | 9 => scale_fingering(note_count, 5, &[4, 3, 2, 1, 3, 2, 1], None),
            11 => scale_fingering(note_count, 4, &[3, 2, 1, 4, 3, 2, 1], None),
            1 | 3 | 8 | 10 => scale_fingering(note_count, 3, &[2, 1, 4, 3, 2, 1, 3], None),
            6 => scale_fingering(note_count, 4, &[3, 2, 1, 3, 2, 1, 4], None),
            _ => unreachable!("validated tonic must be a pitch class"),
        },
        PracticePart::Other => Vec::new(),
    }
}

fn ascending_natural_minor_fingering(tonic: u8, part: PracticePart, note_count: usize) -> Vec<u8> {
    match part {
        PracticePart::RightHand => match tonic {
            0 | 2 | 4 | 7 | 9 | 11 => {
                scale_fingering(note_count, 1, &[2, 3, 1, 2, 3, 4, 1], Some(5))
            }
            1 | 8 => scale_fingering(note_count, 3, &[4, 1, 2, 3, 1, 2, 3], None),
            3 => scale_fingering(note_count, 2, &[1, 2, 3, 4, 1, 2, 3], None),
            5 => scale_fingering(note_count, 1, &[2, 3, 4, 1, 2, 3, 1], Some(4)),
            6 => scale_fingering(note_count, 2, &[3, 1, 2, 3, 4, 1, 2], None),
            10 => scale_fingering(note_count, 2, &[1, 2, 3, 1, 2, 3, 4], None),
            _ => unreachable!("validated tonic must be a pitch class"),
        },
        PracticePart::LeftHand => match tonic {
            0 | 2 | 4 | 5 | 7 | 9 => scale_fingering(note_count, 5, &[4, 3, 2, 1, 3, 2, 1], None),
            11 => scale_fingering(note_count, 4, &[3, 2, 1, 4, 3, 2, 1], None),
            1 => scale_fingering(note_count, 3, &[2, 1, 4, 3, 2, 1, 3], None),
            3 | 10 => scale_fingering(note_count, 2, &[1, 3, 2, 1, 4, 3, 2], None),
            6 => scale_fingering(note_count, 4, &[3, 2, 1, 3, 2, 1, 4], None),
            8 => scale_fingering(note_count, 3, &[2, 1, 3, 2, 1, 4, 3], None),
            _ => unreachable!("validated tonic must be a pitch class"),
        },
        PracticePart::Other => Vec::new(),
    }
}

fn ascending_harmonic_minor_fingering(tonic: u8, part: PracticePart, note_count: usize) -> Vec<u8> {
    match part {
        PracticePart::RightHand => match tonic {
            0 | 2 | 4 | 7 | 9 | 11 => scale_fingering_with_lead_in(
                note_count,
                &[1, 2, 3, 1, 2, 3, 4, 1],
                &[2, 3, 1, 2, 3, 4, 1],
                Some(5),
            ),
            1 | 6 | 8 => scale_fingering_with_lead_in(
                note_count,
                &[2, 3, 1, 2, 3, 1, 2, 3],
                &[4, 1, 2, 3, 1, 2, 3],
                None,
            ),
            3 => scale_fingering_with_lead_in(
                note_count,
                &[2, 1, 2, 3, 4, 1, 2, 3],
                &[1, 2, 3, 4, 1, 2, 3],
                None,
            ),
            5 => scale_fingering_with_lead_in(
                note_count,
                &[1, 2, 3, 4, 1, 2, 3, 1],
                &[2, 3, 4, 1, 2, 3, 1],
                Some(4),
            ),
            10 => scale_fingering_with_lead_in(
                note_count,
                &[2, 1, 2, 3, 1, 2, 3, 4],
                &[1, 2, 3, 1, 2, 3, 4],
                None,
            ),
            _ => unreachable!("validated tonic must be a pitch class"),
        },
        PracticePart::LeftHand => match tonic {
            0 | 2 | 4 | 5 | 7 | 9 => scale_fingering_with_lead_in(
                note_count,
                &[5, 4, 3, 2, 1, 3, 2, 1],
                &[4, 3, 2, 1, 3, 2, 1],
                None,
            ),
            1 | 8 => scale_fingering_with_lead_in(
                note_count,
                &[3, 2, 1, 4, 3, 2, 1, 3],
                &[2, 1, 4, 3, 2, 1, 2],
                Some(2),
            ),
            3 => scale_fingering_with_lead_in(
                note_count,
                &[2, 1, 4, 3, 2, 1, 3, 2],
                &[1, 4, 3, 2, 1, 3, 2],
                None,
            ),
            6 => scale_fingering_with_lead_in(
                note_count,
                &[4, 3, 2, 1, 3, 2, 1, 4],
                &[3, 2, 1, 3, 2, 1, 2],
                Some(2),
            ),
            10 => scale_fingering_with_lead_in(
                note_count,
                &[2, 1, 3, 2, 1, 4, 3, 2],
                &[1, 3, 2, 1, 4, 3, 2],
                None,
            ),
            11 => scale_fingering_with_lead_in(
                note_count,
                &[4, 3, 2, 1, 4, 3, 2, 1],
                &[4, 3, 2, 1, 4, 3, 2],
                Some(2),
            ),
            _ => unreachable!("validated tonic must be a pitch class"),
        },
        PracticePart::Other => Vec::new(),
    }
}

fn ascending_melodic_minor_fingering(tonic: u8, part: PracticePart, note_count: usize) -> Vec<u8> {
    match part {
        PracticePart::RightHand => match tonic {
            0 | 2 | 4 | 9 | 11 => scale_fingering(note_count, 1, &[2, 3, 1, 2, 3, 4, 1], Some(5)),
            1 | 6 => scale_fingering(note_count, 2, &[3, 1, 2, 3, 4, 1, 2], None),
            3 => scale_fingering(note_count, 3, &[1, 2, 3, 4, 1, 2, 3], None),
            5 | 7 => scale_fingering(note_count, 1, &[2, 3, 4, 1, 2, 3, 1], Some(4)),
            8 => scale_fingering(note_count, 3, &[4, 1, 2, 3, 1, 2, 3], None),
            10 => scale_fingering(note_count, 4, &[1, 2, 3, 1, 2, 3, 4], None),
            _ => unreachable!("validated tonic must be a pitch class"),
        },
        PracticePart::LeftHand => match tonic {
            0 | 2 | 4 | 5 | 7 | 9 => scale_fingering(note_count, 5, &[4, 3, 2, 1, 3, 2, 1], None),
            1 | 8 => scale_fingering(note_count, 3, &[2, 1, 4, 3, 2, 1, 3], None),
            3 | 10 => scale_fingering(note_count, 2, &[1, 4, 3, 2, 1, 3, 2], None),
            6 => scale_fingering(note_count, 4, &[3, 2, 1, 3, 2, 1, 4], None),
            11 => scale_fingering(note_count, 4, &[3, 2, 1, 4, 3, 2, 1], None),
            _ => unreachable!("validated tonic must be a pitch class"),
        },
        PracticePart::Other => Vec::new(),
    }
}

fn ascending_arpeggio_fingering(
    tonic: u8,
    tonality: ExerciseTonality,
    part: PracticePart,
    octaves: u8,
) -> Vec<u8> {
    let note_count = usize::from(octaves) * 3 + 1;
    match (tonality, part) {
        (ExerciseTonality::Major, PracticePart::RightHand) => match tonic {
            1 | 3 | 8 | 10 => arpeggio_fingering(note_count, 2, &[1, 2, 4], None),
            _ => arpeggio_fingering(note_count, 1, &[2, 3, 1], Some(5)),
        },
        (ExerciseTonality::Major, PracticePart::LeftHand) => match tonic {
            0 | 5 | 7 => arpeggio_fingering(note_count, 5, &[4, 2, 1], None),
            2 | 4 | 6 | 9 | 11 => arpeggio_fingering(note_count, 5, &[3, 2, 1], None),
            1 | 3 | 8 => arpeggio_fingering(note_count, 2, &[1, 4, 2], None),
            10 => arpeggio_fingering(note_count, 3, &[2, 1, 3], None),
            _ => unreachable!("validated tonic must be a pitch class"),
        },
        (ExerciseTonality::Minor, PracticePart::RightHand) => match tonic {
            1 | 6 | 8 => arpeggio_fingering(note_count, 2, &[1, 2, 4], None),
            10 => arpeggio_fingering(note_count, 2, &[3, 1, 2], None),
            _ => arpeggio_fingering(note_count, 1, &[2, 3, 1], Some(5)),
        },
        (ExerciseTonality::Minor, PracticePart::LeftHand) => match tonic {
            1 | 6 | 8 => arpeggio_fingering(note_count, 2, &[1, 4, 2], None),
            3 => arpeggio_fingering(note_count, 5, &[3, 2, 1], None),
            10 => arpeggio_fingering(note_count, 3, &[2, 1, 3], None),
            _ => arpeggio_fingering(note_count, 5, &[4, 2, 1], None),
        },
        (_, PracticePart::Other) => Vec::new(),
    }
}

fn arpeggio_fingering(
    note_count: usize,
    start: u8,
    continuation: &[u8; 3],
    final_override: Option<u8>,
) -> Vec<u8> {
    (0..note_count)
        .map(|index| {
            if index == 0 {
                start
            } else if index + 1 == note_count {
                final_override.unwrap_or(continuation[(index - 1) % continuation.len()])
            } else {
                continuation[(index - 1) % continuation.len()]
            }
        })
        .collect()
}

fn scale_fingering(
    note_count: usize,
    start: u8,
    continuation: &[u8; 7],
    final_override: Option<u8>,
) -> Vec<u8> {
    (0..note_count)
        .map(|index| {
            if index == 0 {
                start
            } else if index + 1 == note_count {
                final_override.unwrap_or(continuation[(index - 1) % 7])
            } else {
                continuation[(index - 1) % 7]
            }
        })
        .collect()
}

fn scale_fingering_with_lead_in(
    note_count: usize,
    first_octave: &[u8; 8],
    later_octave: &[u8; 7],
    final_override: Option<u8>,
) -> Vec<u8> {
    (0..note_count)
        .map(|index| {
            if index + 1 == note_count {
                final_override.unwrap_or_else(|| {
                    if index < first_octave.len() {
                        first_octave[index]
                    } else {
                        later_octave[(index - first_octave.len()) % later_octave.len()]
                    }
                })
            } else if index < first_octave.len() {
                first_octave[index]
            } else {
                later_octave[(index - first_octave.len()) % later_octave.len()]
            }
        })
        .collect()
}

fn tonic_display_name(tonic: u8, tonality: ExerciseTonality) -> &'static str {
    match (tonic, tonality) {
        (0, _) => "C",
        (1, ExerciseTonality::Major) => "Db",
        (1, ExerciseTonality::Minor) => "C#",
        (2, _) => "D",
        (3, _) => "Eb",
        (4, _) => "E",
        (5, _) => "F",
        (6, _) => "F#",
        (7, _) => "G",
        (8, ExerciseTonality::Major) => "Ab",
        (8, ExerciseTonality::Minor) => "G#",
        (9, _) => "A",
        (10, _) => "Bb",
        (11, _) => "B",
        _ => unreachable!("validated tonic must be a pitch class"),
    }
}

fn directional_fingering(
    ascending: Vec<u8>,
    descending_source: Vec<u8>,
    direction: ExerciseDirection,
    repetitions: u8,
) -> Vec<u8> {
    let phrase = match direction {
        ExerciseDirection::Ascending => ascending,
        ExerciseDirection::Descending => descending_source.into_iter().rev().collect(),
        ExerciseDirection::UpAndDown => {
            let mut phrase = ascending.clone();
            phrase.extend(descending_source.into_iter().rev().skip(1));
            phrase
        }
    };
    phrase.repeat(usize::from(repetitions))
}

fn validate_spec(spec: ExerciseSpec) -> Result<(), ExerciseError> {
    if spec.tonic > 11 {
        return Err(ExerciseError::InvalidTonic);
    }
    if !(1..=3).contains(&spec.octaves) {
        return Err(ExerciseError::InvalidOctaves);
    }
    if spec.minor_form != ExerciseMinorForm::Natural
        && (spec.tonality != ExerciseTonality::Minor || spec.pattern != ExercisePattern::Scale)
    {
        return Err(ExerciseError::InvalidMinorForm);
    }
    if !(1..=8).contains(&spec.repetitions) {
        return Err(ExerciseError::InvalidRepetitions);
    }
    if !(20..=240).contains(&spec.tempo_bpm) {
        return Err(ExerciseError::InvalidTempo);
    }
    Ok(())
}

fn selected_parts(hands: ExerciseHands) -> &'static [PracticePart] {
    match hands {
        ExerciseHands::Right => &[PracticePart::RightHand],
        ExerciseHands::Left => &[PracticePart::LeftHand],
        ExerciseHands::Both => &[PracticePart::LeftHand, PracticePart::RightHand],
    }
}

const fn default_repetitions() -> u8 {
    1
}

fn scale_intervals(tonality: ExerciseTonality, minor_form: ExerciseMinorForm) -> &'static [u8] {
    match (tonality, minor_form) {
        (ExerciseTonality::Major, _) => &[0, 2, 4, 5, 7, 9, 11],
        (ExerciseTonality::Minor, ExerciseMinorForm::Natural) => &[0, 2, 3, 5, 7, 8, 10],
        (ExerciseTonality::Minor, ExerciseMinorForm::Harmonic) => &[0, 2, 3, 5, 7, 8, 11],
        (ExerciseTonality::Minor, ExerciseMinorForm::Melodic) => &[0, 2, 3, 5, 7, 9, 11],
    }
}

fn arpeggio_intervals(tonality: ExerciseTonality) -> &'static [u8] {
    match tonality {
        ExerciseTonality::Major => &[0, 4, 7],
        ExerciseTonality::Minor => &[0, 3, 7],
    }
}

fn scale_moments(
    tonality: ExerciseTonality,
    minor_form: ExerciseMinorForm,
    octaves: u8,
    direction: ExerciseDirection,
) -> Vec<Vec<u8>> {
    if tonality != ExerciseTonality::Minor || minor_form != ExerciseMinorForm::Melodic {
        return melodic_moments(scale_intervals(tonality, minor_form), octaves, direction);
    }

    let ascending = ascending_melodic_moments(scale_intervals(tonality, minor_form), octaves);
    let natural = ascending_melodic_moments(
        scale_intervals(ExerciseTonality::Minor, ExerciseMinorForm::Natural),
        octaves,
    );
    match direction {
        ExerciseDirection::Ascending => ascending,
        ExerciseDirection::Descending => natural.into_iter().rev().collect(),
        ExerciseDirection::UpAndDown => {
            let mut result = ascending;
            result.extend(natural.into_iter().rev().skip(1));
            result
        }
    }
}

fn melodic_moments(intervals: &[u8], octaves: u8, direction: ExerciseDirection) -> Vec<Vec<u8>> {
    apply_direction(ascending_melodic_moments(intervals, octaves), direction)
}

fn ascending_melodic_moments(intervals: &[u8], octaves: u8) -> Vec<Vec<u8>> {
    let mut ascending = Vec::new();
    for octave in 0..octaves {
        ascending.extend(intervals.iter().map(|interval| octave * 12 + interval));
    }
    ascending.push(octaves * 12);
    ascending.into_iter().map(|note| vec![note]).collect()
}

fn chord_moments(
    tonality: ExerciseTonality,
    octaves: u8,
    direction: ExerciseDirection,
) -> Vec<Vec<u8>> {
    let (tonic, subdominant, dominant) = match tonality {
        ExerciseTonality::Major => ([0, 4, 7], [5, 9, 12], [7, 11, 14]),
        // Functional minor cadence: i – iv – V – i.
        ExerciseTonality::Minor => ([0, 3, 7], [5, 8, 12], [7, 11, 14]),
    };
    let mut ascending = vec![tonic.to_vec()];
    for octave in 0..octaves {
        let shift = octave * 12;
        ascending.push(subdominant.map(|note| note + shift).to_vec());
        ascending.push(dominant.map(|note| note + shift).to_vec());
        ascending.push(tonic.map(|note| note + shift + 12).to_vec());
    }
    apply_direction(ascending, direction)
}

fn apply_direction<T: Clone>(ascending: Vec<T>, direction: ExerciseDirection) -> Vec<T> {
    match direction {
        ExerciseDirection::Ascending => ascending,
        ExerciseDirection::Descending => ascending.into_iter().rev().collect(),
        ExerciseDirection::UpAndDown => {
            let mut result = ascending.clone();
            result.extend(ascending.into_iter().rev().skip(1));
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pitches(plan: &ExercisePlan, part: PracticePart) -> Vec<Vec<u8>> {
        plan.moments
            .iter()
            .map(|moment| {
                moment
                    .notes
                    .iter()
                    .filter(|note| note.part == part)
                    .map(|note| note.midi_note)
                    .collect()
            })
            .collect()
    }

    #[test]
    fn c_major_scale_moves_both_hands_up_and_down_without_repeating_the_apex() {
        let plan =
            ExercisePlan::generate(ExerciseSpec::default(), &KeyboardRange::standard_88_keys())
                .unwrap();

        assert_eq!(
            pitches(&plan, PracticePart::RightHand),
            vec![
                vec![60],
                vec![62],
                vec![64],
                vec![65],
                vec![67],
                vec![69],
                vec![71],
                vec![72],
                vec![71],
                vec![69],
                vec![67],
                vec![65],
                vec![64],
                vec![62],
                vec![60],
            ]
        );
        assert_eq!(pitches(&plan, PracticePart::LeftHand)[0], vec![36]);
        assert!(plan.moments.iter().all(|moment| moment.beats == 1));
    }

    #[test]
    fn a_minor_arpeggio_uses_minor_third_and_requested_direction() {
        let plan = ExercisePlan::generate(
            ExerciseSpec {
                tonic: 9,
                tonality: ExerciseTonality::Minor,
                pattern: ExercisePattern::Arpeggio,
                direction: ExerciseDirection::Descending,
                hands: ExerciseHands::Right,
                ..Default::default()
            },
            &KeyboardRange::standard_88_keys(),
        )
        .unwrap();

        assert_eq!(
            pitches(&plan, PracticePart::RightHand),
            vec![vec![81], vec![76], vec![72], vec![69]]
        );
        assert!(
            pitches(&plan, PracticePart::LeftHand)
                .iter()
                .all(Vec::is_empty)
        );
    }

    #[test]
    fn a_melodic_minor_raises_six_and_seven_upward_then_descends_naturally() {
        let plan = ExercisePlan::generate(
            ExerciseSpec {
                tonic: 9,
                tonality: ExerciseTonality::Minor,
                minor_form: ExerciseMinorForm::Melodic,
                hands: ExerciseHands::Right,
                ..Default::default()
            },
            &KeyboardRange::standard_88_keys(),
        )
        .unwrap();

        assert_eq!(
            pitches(&plan, PracticePart::RightHand),
            [69, 71, 72, 74, 76, 78, 80, 81, 79, 77, 76, 74, 72, 71, 69]
                .into_iter()
                .map(|note| vec![note])
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_harmonic_minor_keeps_the_raised_seventh_when_descending() {
        let plan = ExercisePlan::generate(
            ExerciseSpec {
                tonic: 9,
                tonality: ExerciseTonality::Minor,
                minor_form: ExerciseMinorForm::Harmonic,
                direction: ExerciseDirection::Descending,
                hands: ExerciseHands::Right,
                ..Default::default()
            },
            &KeyboardRange::standard_88_keys(),
        )
        .unwrap();

        assert_eq!(
            pitches(&plan, PracticePart::RightHand),
            [81, 80, 77, 76, 74, 72, 71, 69]
                .into_iter()
                .map(|note| vec![note])
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn c_scale_fingering_handles_hands_directions_octaves_and_repetitions() {
        let plan = ExercisePlan::generate(
            ExerciseSpec {
                octaves: 2,
                repetitions: 2,
                ..Default::default()
            },
            &KeyboardRange::standard_88_keys(),
        )
        .unwrap();
        let fingerings = plan.fingerings().unwrap();
        let right_phrase = [1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 5]
            .into_iter()
            .chain([4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1])
            .collect::<Vec<_>>();
        let left_phrase = [5, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1]
            .into_iter()
            .chain([2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 5])
            .collect::<Vec<_>>();

        assert_eq!(fingerings.right, right_phrase.repeat(2));
        assert_eq!(fingerings.left, left_phrase.repeat(2));
    }

    #[test]
    fn fingering_is_explicitly_unavailable_for_primary_chords() {
        let keyboard = KeyboardRange::standard_88_keys();
        let chords = ExercisePlan::generate(
            ExerciseSpec {
                pattern: ExercisePattern::PrimaryChords,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap();

        assert!(chords.fingerings().is_none());
    }

    #[test]
    fn all_major_and_minor_arpeggios_use_reviewed_two_octave_tables() {
        let keyboard = KeyboardRange::standard_88_keys();
        let major_right_common = vec![1, 2, 3, 1, 2, 3, 5];
        let major_right_flat = vec![2, 1, 2, 4, 1, 2, 4];
        let minor_right_common = major_right_common.clone();
        let cases = [
            (
                ExerciseTonality::Major,
                0,
                major_right_common.clone(),
                vec![5, 4, 2, 1, 4, 2, 1],
            ),
            (
                ExerciseTonality::Major,
                1,
                major_right_flat.clone(),
                vec![2, 1, 4, 2, 1, 4, 2],
            ),
            (
                ExerciseTonality::Major,
                2,
                major_right_common.clone(),
                vec![5, 3, 2, 1, 3, 2, 1],
            ),
            (
                ExerciseTonality::Major,
                3,
                major_right_flat.clone(),
                vec![2, 1, 4, 2, 1, 4, 2],
            ),
            (
                ExerciseTonality::Major,
                4,
                major_right_common.clone(),
                vec![5, 3, 2, 1, 3, 2, 1],
            ),
            (
                ExerciseTonality::Major,
                5,
                major_right_common.clone(),
                vec![5, 4, 2, 1, 4, 2, 1],
            ),
            (
                ExerciseTonality::Major,
                6,
                major_right_common.clone(),
                vec![5, 3, 2, 1, 3, 2, 1],
            ),
            (
                ExerciseTonality::Major,
                7,
                major_right_common.clone(),
                vec![5, 4, 2, 1, 4, 2, 1],
            ),
            (
                ExerciseTonality::Major,
                8,
                major_right_flat.clone(),
                vec![2, 1, 4, 2, 1, 4, 2],
            ),
            (
                ExerciseTonality::Major,
                9,
                major_right_common.clone(),
                vec![5, 3, 2, 1, 3, 2, 1],
            ),
            (
                ExerciseTonality::Major,
                10,
                major_right_flat,
                vec![3, 2, 1, 3, 2, 1, 3],
            ),
            (
                ExerciseTonality::Major,
                11,
                major_right_common,
                vec![5, 3, 2, 1, 3, 2, 1],
            ),
            (
                ExerciseTonality::Minor,
                0,
                minor_right_common.clone(),
                vec![5, 4, 2, 1, 4, 2, 1],
            ),
            (
                ExerciseTonality::Minor,
                1,
                vec![2, 1, 2, 4, 1, 2, 4],
                vec![2, 1, 4, 2, 1, 4, 2],
            ),
            (
                ExerciseTonality::Minor,
                2,
                minor_right_common.clone(),
                vec![5, 4, 2, 1, 4, 2, 1],
            ),
            (
                ExerciseTonality::Minor,
                3,
                minor_right_common.clone(),
                vec![5, 3, 2, 1, 3, 2, 1],
            ),
            (
                ExerciseTonality::Minor,
                4,
                minor_right_common.clone(),
                vec![5, 4, 2, 1, 4, 2, 1],
            ),
            (
                ExerciseTonality::Minor,
                5,
                minor_right_common.clone(),
                vec![5, 4, 2, 1, 4, 2, 1],
            ),
            (
                ExerciseTonality::Minor,
                6,
                vec![2, 1, 2, 4, 1, 2, 4],
                vec![2, 1, 4, 2, 1, 4, 2],
            ),
            (
                ExerciseTonality::Minor,
                7,
                minor_right_common.clone(),
                vec![5, 4, 2, 1, 4, 2, 1],
            ),
            (
                ExerciseTonality::Minor,
                8,
                vec![2, 1, 2, 4, 1, 2, 4],
                vec![2, 1, 4, 2, 1, 4, 2],
            ),
            (
                ExerciseTonality::Minor,
                9,
                minor_right_common.clone(),
                vec![5, 4, 2, 1, 4, 2, 1],
            ),
            (
                ExerciseTonality::Minor,
                10,
                vec![2, 3, 1, 2, 3, 1, 2],
                vec![3, 2, 1, 3, 2, 1, 3],
            ),
            (
                ExerciseTonality::Minor,
                11,
                minor_right_common,
                vec![5, 4, 2, 1, 4, 2, 1],
            ),
        ];

        for (tonality, tonic, right, left) in cases {
            let fingerings = ExercisePlan::generate(
                ExerciseSpec {
                    tonic,
                    tonality,
                    pattern: ExercisePattern::Arpeggio,
                    direction: ExerciseDirection::Ascending,
                    hands: ExerciseHands::Both,
                    octaves: 2,
                    ..Default::default()
                },
                &keyboard,
            )
            .unwrap()
            .fingerings()
            .unwrap();
            assert_eq!(fingerings.right, right, "right {tonality:?} tonic {tonic}");
            assert_eq!(fingerings.left, left, "left {tonality:?} tonic {tonic}");
        }
    }

    #[test]
    fn reviewed_major_group_uses_standard_and_f_major_patterns() {
        let keyboard = KeyboardRange::standard_88_keys();
        for tonic in [0, 2, 4, 7, 9] {
            let plan = ExercisePlan::generate(
                ExerciseSpec {
                    tonic,
                    direction: ExerciseDirection::Ascending,
                    hands: ExerciseHands::Both,
                    octaves: 2,
                    ..Default::default()
                },
                &keyboard,
            )
            .unwrap();
            let fingering = plan.fingerings().unwrap();
            assert_eq!(
                fingering.right,
                [1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 5]
            );
            assert_eq!(
                fingering.left,
                [5, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1]
            );
        }

        let f_major = ExercisePlan::generate(
            ExerciseSpec {
                tonic: 5,
                direction: ExerciseDirection::Ascending,
                hands: ExerciseHands::Both,
                octaves: 2,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap()
        .fingerings()
        .unwrap();
        assert_eq!(f_major.right, [1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 4]);
        assert_eq!(f_major.left, [5, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1]);
    }

    #[test]
    fn all_natural_minor_keys_use_reviewed_two_octave_tables() {
        let keyboard = KeyboardRange::standard_88_keys();
        let cases = [
            (
                1,
                vec![3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3],
                vec![3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3],
            ),
            (
                3,
                vec![2, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3],
                vec![2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2],
            ),
            (
                6,
                vec![2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2],
                vec![4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4],
            ),
            (
                8,
                vec![3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3],
                vec![3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3],
            ),
            (
                10,
                vec![2, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4],
                vec![2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2],
            ),
        ];
        for (tonic, right, left) in cases {
            let plan = ExercisePlan::generate(
                ExerciseSpec {
                    tonic,
                    tonality: ExerciseTonality::Minor,
                    direction: ExerciseDirection::Ascending,
                    hands: ExerciseHands::Both,
                    octaves: 2,
                    ..Default::default()
                },
                &keyboard,
            )
            .unwrap()
            .fingerings()
            .unwrap();
            assert_eq!(plan.right, right, "right hand tonic {tonic}");
            assert_eq!(plan.left, left, "left hand tonic {tonic}");
        }

        for tonic in 0..12 {
            let plan = ExercisePlan::generate(
                ExerciseSpec {
                    tonic,
                    tonality: ExerciseTonality::Minor,
                    ..Default::default()
                },
                &keyboard,
            )
            .unwrap();
            assert!(plan.fingerings().is_some(), "natural minor tonic {tonic}");
        }
    }

    #[test]
    fn all_harmonic_minor_keys_use_reviewed_two_octave_tables() {
        let keyboard = KeyboardRange::standard_88_keys();
        let common_right = vec![1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 5];
        let common_left = vec![5, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1];
        let cases = [
            (0, common_right.clone(), common_left.clone()),
            (
                1,
                vec![2, 3, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3],
                vec![3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 2],
            ),
            (2, common_right.clone(), common_left.clone()),
            (
                3,
                vec![2, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3],
                vec![2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2],
            ),
            (4, common_right.clone(), common_left.clone()),
            (
                5,
                vec![1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 4],
                common_left.clone(),
            ),
            (
                6,
                vec![2, 3, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3],
                vec![4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 2],
            ),
            (7, common_right.clone(), common_left.clone()),
            (
                8,
                vec![2, 3, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3],
                vec![3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 2],
            ),
            (9, common_right, common_left),
            (
                10,
                vec![2, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4],
                vec![2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2],
            ),
            (
                11,
                vec![1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 5],
                vec![4, 3, 2, 1, 4, 3, 2, 1, 4, 3, 2, 1, 4, 3, 2],
            ),
        ];

        for (tonic, right, left) in cases {
            let fingerings = ExercisePlan::generate(
                ExerciseSpec {
                    tonic,
                    tonality: ExerciseTonality::Minor,
                    minor_form: ExerciseMinorForm::Harmonic,
                    direction: ExerciseDirection::Ascending,
                    hands: ExerciseHands::Both,
                    octaves: 2,
                    ..Default::default()
                },
                &keyboard,
            )
            .unwrap()
            .fingerings()
            .unwrap();
            assert_eq!(fingerings.right, right, "right hand tonic {tonic}");
            assert_eq!(fingerings.left, left, "left hand tonic {tonic}");
        }
    }

    #[test]
    fn all_melodic_minor_keys_use_reviewed_two_octave_tables() {
        let keyboard = KeyboardRange::standard_88_keys();
        let common_right = vec![1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 5];
        let common_left = vec![5, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1];
        let cases = [
            (0, common_right.clone(), common_left.clone()),
            (
                1,
                vec![2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2],
                vec![3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3],
            ),
            (2, common_right.clone(), common_left.clone()),
            (
                3,
                vec![3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3],
                vec![2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2],
            ),
            (4, common_right.clone(), common_left.clone()),
            (
                5,
                vec![1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 4],
                common_left.clone(),
            ),
            (
                6,
                vec![2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2],
                vec![4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4],
            ),
            (
                7,
                vec![1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 4],
                common_left.clone(),
            ),
            (
                8,
                vec![3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3],
                vec![3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3],
            ),
            (9, common_right, common_left),
            (
                10,
                vec![4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4],
                vec![2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2],
            ),
            (
                11,
                vec![1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 5],
                vec![4, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1],
            ),
        ];

        for (tonic, right, left) in cases {
            let fingerings = ExercisePlan::generate(
                ExerciseSpec {
                    tonic,
                    tonality: ExerciseTonality::Minor,
                    minor_form: ExerciseMinorForm::Melodic,
                    direction: ExerciseDirection::Ascending,
                    hands: ExerciseHands::Both,
                    octaves: 2,
                    ..Default::default()
                },
                &keyboard,
            )
            .unwrap()
            .fingerings()
            .unwrap();
            assert_eq!(fingerings.right, right, "right hand tonic {tonic}");
            assert_eq!(fingerings.left, left, "left hand tonic {tonic}");
        }
    }

    #[test]
    fn melodic_minor_descends_with_natural_minor_fingering() {
        let fingerings = ExercisePlan::generate(
            ExerciseSpec {
                tonic: 10,
                tonality: ExerciseTonality::Minor,
                minor_form: ExerciseMinorForm::Melodic,
                hands: ExerciseHands::Both,
                ..Default::default()
            },
            &KeyboardRange::standard_88_keys(),
        )
        .unwrap()
        .fingerings()
        .unwrap();

        assert_eq!(
            fingerings.right,
            [4, 1, 2, 3, 1, 2, 3, 4, 3, 2, 1, 3, 2, 1, 2]
        );
        assert_eq!(
            fingerings.left,
            [2, 1, 4, 3, 2, 1, 3, 2, 3, 4, 1, 2, 3, 1, 2]
        );
    }

    #[test]
    fn c_minor_primary_chords_use_a_major_dominant() {
        let plan = ExercisePlan::generate(
            ExerciseSpec {
                tonality: ExerciseTonality::Minor,
                pattern: ExercisePattern::PrimaryChords,
                direction: ExerciseDirection::Ascending,
                hands: ExerciseHands::Right,
                ..Default::default()
            },
            &KeyboardRange::standard_88_keys(),
        )
        .unwrap();

        assert_eq!(
            pitches(&plan, PracticePart::RightHand),
            vec![
                vec![60, 63, 67],
                vec![65, 68, 72],
                vec![67, 71, 74],
                vec![72, 75, 79],
            ]
        );
        assert!(plan.moments.iter().all(|moment| moment.beats == 2));
    }

    #[test]
    fn three_octave_b_major_fits_standard_piano_but_not_a_small_range() {
        let spec = ExerciseSpec {
            tonic: 11,
            octaves: 3,
            direction: ExerciseDirection::Ascending,
            ..Default::default()
        };
        let plan = ExercisePlan::generate(spec, &KeyboardRange::standard_88_keys()).unwrap();
        assert_eq!(
            pitches(&plan, PracticePart::RightHand).last(),
            Some(&vec![107])
        );
        assert_eq!(
            ExercisePlan::generate(spec, &KeyboardRange::new(48..=84)),
            Err(ExerciseError::OutsideKeyboardRange)
        );
    }

    #[test]
    fn remaining_major_keys_use_reviewed_two_octave_tables() {
        let keyboard = KeyboardRange::standard_88_keys();
        let cases = [
            (
                1,
                vec![2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2],
                vec![3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3],
            ),
            (
                3,
                vec![3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3],
                vec![3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3],
            ),
            (
                6,
                vec![2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2],
                vec![4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4],
            ),
            (
                8,
                vec![3, 4, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3],
                vec![3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3],
            ),
            (
                10,
                vec![2, 1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4],
                vec![3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1, 3],
            ),
            (
                11,
                vec![1, 2, 3, 1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 4, 5],
                vec![4, 3, 2, 1, 4, 3, 2, 1, 3, 2, 1, 4, 3, 2, 1],
            ),
        ];

        for (tonic, right, left) in cases {
            let plan = ExercisePlan::generate(
                ExerciseSpec {
                    tonic,
                    direction: ExerciseDirection::Ascending,
                    hands: ExerciseHands::Both,
                    octaves: 2,
                    ..Default::default()
                },
                &keyboard,
            )
            .unwrap();
            let fingering = plan.fingerings().unwrap();
            assert_eq!(fingering.right, right, "right hand tonic {tonic}");
            assert_eq!(fingering.left, left, "left hand tonic {tonic}");
        }
    }

    #[test]
    fn display_names_prefer_readable_flat_major_keys() {
        let keyboard = KeyboardRange::standard_88_keys();
        let d_flat_major = ExercisePlan::generate(
            ExerciseSpec {
                tonic: 1,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap();
        let c_sharp_minor = ExercisePlan::generate(
            ExerciseSpec {
                tonic: 1,
                tonality: ExerciseTonality::Minor,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap();

        assert!(d_flat_major.display_name().starts_with("Db Major"));
        assert!(c_sharp_minor.display_name().starts_with("C# Natural Minor"));
        let g_sharp_minor = ExercisePlan::generate(
            ExerciseSpec {
                tonic: 8,
                tonality: ExerciseTonality::Minor,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap();
        assert!(g_sharp_minor.display_name().starts_with("G# Natural Minor"));
    }

    #[test]
    fn invalid_specifications_are_rejected_before_generation() {
        let keyboard = KeyboardRange::standard_88_keys();
        assert_eq!(
            ExercisePlan::generate(
                ExerciseSpec {
                    tonic: 12,
                    ..Default::default()
                },
                &keyboard
            ),
            Err(ExerciseError::InvalidTonic)
        );
        assert_eq!(
            ExercisePlan::generate(
                ExerciseSpec {
                    octaves: 4,
                    ..Default::default()
                },
                &keyboard
            ),
            Err(ExerciseError::InvalidOctaves)
        );
        assert_eq!(
            ExercisePlan::generate(
                ExerciseSpec {
                    tonality: ExerciseTonality::Major,
                    minor_form: ExerciseMinorForm::Harmonic,
                    ..Default::default()
                },
                &keyboard
            ),
            Err(ExerciseError::InvalidMinorForm)
        );
        assert_eq!(
            ExercisePlan::generate(
                ExerciseSpec {
                    tonality: ExerciseTonality::Minor,
                    minor_form: ExerciseMinorForm::Melodic,
                    pattern: ExercisePattern::Arpeggio,
                    ..Default::default()
                },
                &keyboard
            ),
            Err(ExerciseError::InvalidMinorForm)
        );
        assert_eq!(
            ExercisePlan::generate(
                ExerciseSpec {
                    repetitions: 0,
                    ..Default::default()
                },
                &keyboard
            ),
            Err(ExerciseError::InvalidRepetitions)
        );
        assert_eq!(
            ExercisePlan::generate(
                ExerciseSpec {
                    repetitions: 9,
                    ..Default::default()
                },
                &keyboard
            ),
            Err(ExerciseError::InvalidRepetitions)
        );
        assert_eq!(
            ExercisePlan::generate(
                ExerciseSpec {
                    tempo_bpm: 241,
                    ..Default::default()
                },
                &keyboard
            ),
            Err(ExerciseError::InvalidTempo)
        );
    }

    #[test]
    fn plan_converts_to_stable_type_one_midi_with_separate_hand_tracks() {
        let plan =
            ExercisePlan::generate(ExerciseSpec::default(), &KeyboardRange::standard_88_keys())
                .unwrap();
        let first = plan.to_midi_file().unwrap();
        let second = plan.to_midi_file().unwrap();

        assert_eq!(first.format, Format::Parallel);
        assert_eq!(first.source_path, None);
        assert_eq!(first.name, "C Major Scale · Both Hands · 60 BPM");
        assert_eq!(first.content_id, second.content_id);
        assert_eq!(first.tracks.len(), 3);
        assert!(first.tracks[0].notes.is_empty());
        assert_eq!(first.tracks[1].notes.len(), 15);
        assert_eq!(first.tracks[2].notes.len(), 15);
        assert_eq!(first.tracks[1].notes[0].note, 60);
        assert_eq!(first.tracks[2].notes[0].note, 36);
        assert_eq!(
            first.tracks[1].notes[0].duration,
            std::time::Duration::from_millis(800)
        );
        assert!(first.beats.len() >= 15);
        assert!(!first.measures.is_empty());
    }

    #[test]
    fn tempo_changes_midi_timing_and_content_identity() {
        let keyboard = KeyboardRange::standard_88_keys();
        let slow = ExercisePlan::generate(ExerciseSpec::default(), &keyboard)
            .unwrap()
            .to_midi_file()
            .unwrap();
        let fast = ExercisePlan::generate(
            ExerciseSpec {
                tempo_bpm: 120,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap()
        .to_midi_file()
        .unwrap();

        assert_ne!(slow.content_id, fast.content_id);
        assert_eq!(
            fast.tracks[1].notes[0].duration,
            std::time::Duration::from_millis(400)
        );
        assert!(fast.tracks[1].notes[1].start < slow.tracks[1].notes[1].start);
    }

    #[test]
    fn practice_identity_groups_tempo_and_hand_progression_but_not_musical_targets() {
        let keyboard = KeyboardRange::standard_88_keys();
        let base = ExercisePlan::generate(ExerciseSpec::default(), &keyboard).unwrap();
        let faster_left = ExercisePlan::generate(
            ExerciseSpec {
                tempo_bpm: 120,
                hands: ExerciseHands::Left,
                repetitions: 4,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap();
        let different_key = ExercisePlan::generate(
            ExerciseSpec {
                tonic: 2,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap();
        let harmonic_minor = ExercisePlan::generate(
            ExerciseSpec {
                tonality: ExerciseTonality::Minor,
                minor_form: ExerciseMinorForm::Harmonic,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap();

        assert_eq!(base.practice_id(), faster_left.practice_id());
        assert_ne!(base.practice_id(), different_key.practice_id());
        assert_ne!(base.practice_id(), harmonic_minor.practice_id());
        assert_eq!(base.practice_id().len(), 64);
    }

    #[test]
    fn repetitions_duplicate_complete_phrases_without_splitting_practice_identity() {
        let keyboard = KeyboardRange::standard_88_keys();
        let once = ExercisePlan::generate(
            ExerciseSpec {
                pattern: ExercisePattern::Arpeggio,
                direction: ExerciseDirection::Ascending,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap();
        let four = ExercisePlan::generate(
            ExerciseSpec {
                pattern: ExercisePattern::Arpeggio,
                direction: ExerciseDirection::Ascending,
                repetitions: 4,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap();

        assert_eq!(four.moments.len(), once.moments.len() * 4);
        assert_eq!(four.beats_per_repetition(), once.beats_per_repetition());
        assert_eq!(&four.moments[..once.moments.len()], once.moments);
        assert_eq!(four.practice_id(), once.practice_id());
        assert!(four.display_name().contains("· x4 ·"));
    }

    #[test]
    fn exercise_spec_saved_before_repetitions_defaults_to_one() {
        let legacy = r#"(
            tonic: 0,
            tonality: Major,
            pattern: Scale,
            direction: UpAndDown,
            hands: Both,
            octaves: 1,
            tempo_bpm: 60,
        )"#;

        let spec: ExerciseSpec = ron::from_str(legacy).unwrap();

        assert_eq!(spec.minor_form, ExerciseMinorForm::Natural);
        assert_eq!(spec.repetitions, 1);
    }
}
