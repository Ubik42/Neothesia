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

#[derive(Debug, Clone, Eq, Error, PartialEq)]
pub enum ExerciseError {
    #[error("tonic must be a pitch class from 0 to 11")]
    InvalidTonic,
    #[error("exercise span must be between one and three octaves")]
    InvalidOctaves,
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
                melodic_moments(scale_intervals(spec.tonality), spec.octaves, spec.direction)
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
        let tonic = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ][self.spec.tonic as usize];
        let tonality = match self.spec.tonality {
            ExerciseTonality::Major => "Major",
            ExerciseTonality::Minor => "Minor",
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
        let tonality = match self.spec.tonality {
            ExerciseTonality::Major => "major",
            ExerciseTonality::Minor => "minor",
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

fn validate_spec(spec: ExerciseSpec) -> Result<(), ExerciseError> {
    if spec.tonic > 11 {
        return Err(ExerciseError::InvalidTonic);
    }
    if !(1..=3).contains(&spec.octaves) {
        return Err(ExerciseError::InvalidOctaves);
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

fn scale_intervals(tonality: ExerciseTonality) -> &'static [u8] {
    match tonality {
        ExerciseTonality::Major => &[0, 2, 4, 5, 7, 9, 11],
        ExerciseTonality::Minor => &[0, 2, 3, 5, 7, 8, 10],
    }
}

fn arpeggio_intervals(tonality: ExerciseTonality) -> &'static [u8] {
    match tonality {
        ExerciseTonality::Major => &[0, 4, 7],
        ExerciseTonality::Minor => &[0, 3, 7],
    }
}

fn melodic_moments(intervals: &[u8], octaves: u8, direction: ExerciseDirection) -> Vec<Vec<u8>> {
    let mut ascending = Vec::new();
    for octave in 0..octaves {
        ascending.extend(intervals.iter().map(|interval| octave * 12 + interval));
    }
    ascending.push(octaves * 12);
    apply_direction(
        ascending.into_iter().map(|note| vec![note]).collect(),
        direction,
    )
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

        assert_eq!(base.practice_id(), faster_left.practice_id());
        assert_ne!(base.practice_id(), different_key.practice_id());
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

        assert_eq!(spec.repetitions, 1);
    }
}
