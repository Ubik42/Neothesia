use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FingeringHand {
    Right,
    Left,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum HandSpanProfile {
    Compact,
    #[default]
    Standard,
    Large,
}

impl HandSpanProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact · up to a 7th",
            Self::Standard => "Standard · up to an octave",
            Self::Large => "Large · up to a 9th",
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Compact => Self::Compact,
            Self::Standard => Self::Compact,
            Self::Large => Self::Standard,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Compact => Self::Standard,
            Self::Standard => Self::Large,
            Self::Large => Self::Large,
        }
    }

    fn comfortable_spans(self) -> [i32; 5] {
        match self {
            Self::Compact => [0, 2, 4, 7, 10],
            Self::Standard => [0, 3, 5, 8, 12],
            Self::Large => [0, 4, 7, 10, 14],
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FingeringNote {
    pub pitch: u8,
    pub onset: Duration,
    pub anchored_finger: Option<u8>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FingeringReason {
    ManualAnchor,
    PhraseStart,
    RepeatedNote,
    InPosition,
    ThumbUnder,
    FingerOver,
    PositionShift,
}

impl FingeringReason {
    pub fn explanation(self, hand: FingeringHand) -> &'static str {
        match (self, hand) {
            (Self::ManualAnchor, _) => "keeps your saved finger as a fixed anchor",
            (Self::PhraseStart, _) => "starts from a balanced hand position",
            (Self::RepeatedNote, _) => "repeats the pitch without changing finger",
            (Self::InPosition, _) => "follows the melodic direction inside one hand position",
            (Self::ThumbUnder, FingeringHand::Right) => {
                "uses a right-hand thumb-under turn to continue upward"
            }
            (Self::ThumbUnder, FingeringHand::Left) => {
                "uses a left-hand thumb-under turn to continue downward"
            }
            (Self::FingerOver, FingeringHand::Right) => {
                "uses a right-hand finger-over turn to continue downward"
            }
            (Self::FingerOver, FingeringHand::Left) => {
                "uses a left-hand finger-over turn to continue upward"
            }
            (Self::PositionShift, _) => "resets the hand after a leap or awkward reach",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FingerSuggestion {
    pub finger: u8,
    pub confidence_percent: u8,
    pub reason: FingeringReason,
}

const FINGER_COUNT: usize = 5;
const INFINITY: i32 = i32::MAX / 8;

pub fn suggest_fingerings(
    notes: &[FingeringNote],
    hand: FingeringHand,
) -> Vec<Option<FingerSuggestion>> {
    suggest_fingerings_with_profile(notes, hand, HandSpanProfile::Standard)
}

pub fn suggest_fingerings_with_profile(
    notes: &[FingeringNote],
    hand: FingeringHand,
    profile: HandSpanProfile,
) -> Vec<Option<FingerSuggestion>> {
    let mut suggestions = vec![None; notes.len()];
    let mut index = 0;
    while index < notes.len() {
        let onset = notes[index].onset;
        let end = notes[index..].partition_point(|note| note.onset == onset) + index;
        if end - index > 1 {
            for chord_note in &mut suggestions[index..end] {
                *chord_note = None;
            }
            index = end;
            continue;
        }

        let run_start = index;
        index += 1;
        while index < notes.len() {
            let next_onset = notes[index].onset;
            let next_end = notes[index..].partition_point(|note| note.onset == next_onset) + index;
            if next_end - index > 1 {
                break;
            }
            index += 1;
        }
        suggest_run(
            &notes[run_start..index],
            hand,
            profile,
            &mut suggestions[run_start..index],
        );
    }
    suggestions
}

fn suggest_run(
    notes: &[FingeringNote],
    hand: FingeringHand,
    profile: HandSpanProfile,
    output: &mut [Option<FingerSuggestion>],
) {
    if notes.is_empty() {
        return;
    }
    let mut costs = vec![[INFINITY; FINGER_COUNT]; notes.len()];
    let mut previous = vec![[0_u8; FINGER_COUNT]; notes.len()];

    for finger in 1..=5 {
        if finger_allowed(notes[0], finger) {
            costs[0][finger - 1] = start_cost(notes, hand, finger as u8);
        }
    }

    for note_index in 1..notes.len() {
        for next_finger in 1..=5 {
            if !finger_allowed(notes[note_index], next_finger) {
                continue;
            }
            for prior_finger in 1..=5 {
                let prior_cost = costs[note_index - 1][prior_finger - 1];
                if prior_cost == INFINITY {
                    continue;
                }
                let candidate = prior_cost
                    + transition_cost(
                        notes[note_index - 1].pitch,
                        prior_finger as u8,
                        notes[note_index].pitch,
                        next_finger as u8,
                        hand,
                        profile,
                    )
                    + static_key_cost(notes[note_index].pitch, next_finger as u8);
                if candidate < costs[note_index][next_finger - 1] {
                    costs[note_index][next_finger - 1] = candidate;
                    previous[note_index][next_finger - 1] = prior_finger as u8;
                }
            }
        }
    }

    let mut fingers = vec![1_u8; notes.len()];
    fingers[notes.len() - 1] = best_finger(&costs[notes.len() - 1]);
    for note_index in (1..notes.len()).rev() {
        fingers[note_index - 1] = previous[note_index][usize::from(fingers[note_index] - 1)];
    }

    for note_index in 0..notes.len() {
        let reason = if notes[note_index]
            .anchored_finger
            .is_some_and(|finger| (1..=5).contains(&finger))
        {
            FingeringReason::ManualAnchor
        } else if note_index == 0 {
            FingeringReason::PhraseStart
        } else {
            transition_reason(
                notes[note_index - 1].pitch,
                fingers[note_index - 1],
                notes[note_index].pitch,
                fingers[note_index],
                hand,
            )
        };
        output[note_index] = Some(FingerSuggestion {
            finger: fingers[note_index],
            confidence_percent: confidence(reason),
            reason,
        });
    }
}

fn finger_allowed(note: FingeringNote, finger: usize) -> bool {
    note.anchored_finger
        .filter(|anchor| (1..=5).contains(anchor))
        .is_none_or(|anchor| usize::from(anchor) == finger)
}

fn best_finger(costs: &[i32; FINGER_COUNT]) -> u8 {
    costs
        .iter()
        .enumerate()
        .min_by_key(|(_, cost)| *cost)
        .map(|(index, _)| index as u8 + 1)
        .unwrap_or(1)
}

fn start_cost(notes: &[FingeringNote], hand: FingeringHand, finger: u8) -> i32 {
    let direction = notes
        .iter()
        .skip(1)
        .find_map(|note| (note.pitch != notes[0].pitch).then_some(note.pitch > notes[0].pitch));
    let preferred = direction.map_or(3, |ascending| {
        let outward = match hand {
            FingeringHand::Right => ascending,
            FingeringHand::Left => !ascending,
        };
        if outward { 1 } else { 5 }
    });
    static_key_cost(notes[0].pitch, finger) + i32::from(finger.abs_diff(preferred)) * 5
}

fn static_key_cost(pitch: u8, finger: u8) -> i32 {
    if is_black_key(pitch) {
        match finger {
            1 => 7,
            5 => 3,
            _ => 0,
        }
    } else {
        0
    }
}

fn transition_cost(
    prior_pitch: u8,
    prior_finger: u8,
    next_pitch: u8,
    next_finger: u8,
    hand: FingeringHand,
    profile: HandSpanProfile,
) -> i32 {
    let pitch_delta = i16::from(next_pitch) - i16::from(prior_pitch);
    let distance = pitch_delta.unsigned_abs() as i32;
    if pitch_delta == 0 {
        return if prior_finger == next_finger { 0 } else { 10 };
    }
    if distance > 12 {
        return 5 + i32::from(next_finger.abs_diff(3));
    }
    if prior_finger == next_finger {
        return 12 + distance;
    }

    let outward = match hand {
        FingeringHand::Right => pitch_delta > 0,
        FingeringHand::Left => pitch_delta < 0,
    };
    let finger_delta = i16::from(next_finger) - i16::from(prior_finger);
    let follows_position = (outward && finger_delta > 0) || (!outward && finger_delta < 0);
    let is_thumb_under = outward && next_finger == 1 && prior_finger >= 2;
    let is_finger_over = !outward && prior_finger == 1 && next_finger >= 2;

    let movement = if follows_position {
        let expected = i32::from(finger_delta.unsigned_abs()) * 2;
        1 + (distance - expected).unsigned_abs() as i32
    } else if is_thumb_under || is_finger_over {
        4 + if distance > 5 { (distance - 5) * 2 } else { 0 }
    } else {
        18 + distance
    };

    let finger_gap = i32::from(prior_finger.abs_diff(next_finger));
    let comfortable_span = profile.comfortable_spans()[finger_gap as usize];
    movement + (distance - comfortable_span).max(0) * 4
}

fn transition_reason(
    prior_pitch: u8,
    prior_finger: u8,
    next_pitch: u8,
    next_finger: u8,
    hand: FingeringHand,
) -> FingeringReason {
    if prior_pitch == next_pitch {
        return FingeringReason::RepeatedNote;
    }
    let pitch_delta = i16::from(next_pitch) - i16::from(prior_pitch);
    if pitch_delta.unsigned_abs() > 12 {
        return FingeringReason::PositionShift;
    }
    let outward = match hand {
        FingeringHand::Right => pitch_delta > 0,
        FingeringHand::Left => pitch_delta < 0,
    };
    if outward && next_finger == 1 && prior_finger >= 2 {
        FingeringReason::ThumbUnder
    } else if !outward && prior_finger == 1 && next_finger >= 2 {
        FingeringReason::FingerOver
    } else {
        FingeringReason::InPosition
    }
}

fn confidence(reason: FingeringReason) -> u8 {
    match reason {
        FingeringReason::ManualAnchor => 100,
        FingeringReason::RepeatedNote => 92,
        FingeringReason::InPosition => 85,
        FingeringReason::ThumbUnder | FingeringReason::FingerOver => 72,
        FingeringReason::PhraseStart => 65,
        FingeringReason::PositionShift => 55,
    }
}

fn is_black_key(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes(pitches: &[u8]) -> Vec<FingeringNote> {
        pitches
            .iter()
            .enumerate()
            .map(|(index, pitch)| FingeringNote {
                pitch: *pitch,
                onset: Duration::from_millis(index as u64 * 500),
                anchored_finger: None,
            })
            .collect()
    }

    fn fingers(notes: &[FingeringNote], hand: FingeringHand) -> Vec<u8> {
        suggest_fingerings(notes, hand)
            .into_iter()
            .map(|suggestion| suggestion.unwrap().finger)
            .collect()
    }

    #[test]
    fn five_note_position_follows_each_hands_natural_direction() {
        assert_eq!(
            fingers(&notes(&[60, 62, 64, 65, 67]), FingeringHand::Right),
            [1, 2, 3, 4, 5]
        );
        assert_eq!(
            fingers(&notes(&[60, 62, 64, 65, 67]), FingeringHand::Left),
            [5, 4, 3, 2, 1]
        );
    }

    #[test]
    fn c_major_octave_uses_an_explainable_thumb_turn() {
        let source = notes(&[60, 62, 64, 65, 67, 69, 71, 72]);
        let suggestions = suggest_fingerings(&source, FingeringHand::Right);
        let fingers: Vec<_> = suggestions
            .iter()
            .map(|suggestion| suggestion.unwrap().finger)
            .collect();
        assert_eq!(fingers, [1, 2, 3, 1, 2, 3, 4, 5]);
        assert_eq!(suggestions[3].unwrap().reason, FingeringReason::ThumbUnder);
    }

    #[test]
    fn repeats_keep_a_finger_and_manual_anchors_are_hard_constraints() {
        let mut source = notes(&[60, 60, 62]);
        source[0].anchored_finger = Some(3);
        let suggestions = suggest_fingerings(&source, FingeringHand::Right);
        assert_eq!(suggestions[0].unwrap().finger, 3);
        assert_eq!(
            suggestions[0].unwrap().reason,
            FingeringReason::ManualAnchor
        );
        assert_eq!(suggestions[1].unwrap().finger, 3);
        assert_eq!(
            suggestions[1].unwrap().reason,
            FingeringReason::RepeatedNote
        );
    }

    #[test]
    fn unmodeled_chords_are_left_without_false_precision() {
        let source = vec![
            FingeringNote {
                pitch: 60,
                onset: Duration::ZERO,
                anchored_finger: None,
            },
            FingeringNote {
                pitch: 64,
                onset: Duration::ZERO,
                anchored_finger: None,
            },
            FingeringNote {
                pitch: 67,
                onset: Duration::ZERO,
                anchored_finger: None,
            },
        ];
        assert_eq!(
            suggest_fingerings(&source, FingeringHand::Right),
            [None, None, None]
        );
    }

    #[test]
    fn black_key_penalty_avoids_thumb_when_an_in_position_choice_exists() {
        let suggestions = suggest_fingerings(&notes(&[60, 61, 63]), FingeringHand::Right);
        assert_ne!(suggestions[1].unwrap().finger, 1);
        assert_ne!(suggestions[2].unwrap().finger, 1);
    }

    #[test]
    fn reasons_are_plain_language_and_confidence_is_bounded() {
        let suggestion = suggest_fingerings(&notes(&[60]), FingeringHand::Right)[0].unwrap();
        assert!(
            suggestion
                .reason
                .explanation(FingeringHand::Right)
                .contains("balanced")
        );
        assert!((1..=100).contains(&suggestion.confidence_percent));
    }

    #[test]
    fn hand_span_profiles_change_stretch_planning() {
        let candidates = [
            [48, 52, 55, 60, 64],
            [48, 53, 57, 60, 65],
            [60, 64, 67, 72, 76],
            [60, 65, 69, 72, 77],
        ];

        let changes_plan = candidates.into_iter().any(|pitches| {
            let source = notes(&pitches);
            let compact: Vec<_> = suggest_fingerings_with_profile(
                &source,
                FingeringHand::Right,
                HandSpanProfile::Compact,
            )
            .into_iter()
            .map(|suggestion| suggestion.unwrap().finger)
            .collect();
            let large: Vec<_> = suggest_fingerings_with_profile(
                &source,
                FingeringHand::Right,
                HandSpanProfile::Large,
            )
            .into_iter()
            .map(|suggestion| suggestion.unwrap().finger)
            .collect();
            compact != large
        });

        assert!(
            changes_plan,
            "compact and large profiles should not plan every wide phrase identically"
        );
    }
}
