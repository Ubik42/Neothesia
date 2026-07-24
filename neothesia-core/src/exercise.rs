use piano_layout::KeyboardRange;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::practice::PracticePart;

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
            tempo_bpm: 60,
        }
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
        validate_spec(spec)?;

        let relative_moments = match spec.pattern {
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
}

fn validate_spec(spec: ExerciseSpec) -> Result<(), ExerciseError> {
    if spec.tonic > 11 {
        return Err(ExerciseError::InvalidTonic);
    }
    if !(1..=3).contains(&spec.octaves) {
        return Err(ExerciseError::InvalidOctaves);
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
                    tempo_bpm: 241,
                    ..Default::default()
                },
                &keyboard
            ),
            Err(ExerciseError::InvalidTempo)
        );
    }
}
