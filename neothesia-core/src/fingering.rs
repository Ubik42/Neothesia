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
    pub end: Duration,
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
    ChordShape,
    ChordConnection,
    HeldChordPosition,
    WideChordShape,
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
            (Self::ChordShape, _) => "spreads unique fingers across the chord shape",
            (Self::ChordConnection, _) => "keeps a common chord tone under the same finger",
            (Self::HeldChordPosition, _) => {
                "uses fingers that remain free while earlier chord tones are held"
            }
            (Self::WideChordShape, _) => {
                "uses the outer fingers for a wide chord; do not force the reach"
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
            let mut groups = Vec::new();
            while index < notes.len() {
                let onset = notes[index].onset;
                let group_end = notes[index..].partition_point(|note| note.onset == onset) + index;
                if group_end - index <= 1 {
                    break;
                }
                groups.push((index, group_end));
                index = group_end;
            }
            suggest_chord_run(notes, &groups, hand, profile, &mut suggestions);
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

#[derive(Debug, Clone)]
struct ChordCandidate {
    shape_cost: i32,
    fingers: Vec<u8>,
}

fn suggest_chord_run(
    notes: &[FingeringNote],
    groups: &[(usize, usize)],
    hand: FingeringHand,
    profile: HandSpanProfile,
    output: &mut [Option<FingerSuggestion>],
) {
    let candidate_sets: Vec<_> = groups
        .iter()
        .map(|(start, end)| chord_candidates(&notes[*start..*end], hand, profile))
        .collect();
    let mut cursor = 0;
    while cursor < groups.len() {
        while cursor < groups.len() && candidate_sets[cursor].is_empty() {
            cursor += 1;
        }
        if cursor == groups.len() {
            break;
        }
        let start = cursor;
        cursor += 1;
        while cursor < groups.len()
            && !candidate_sets[cursor].is_empty()
            && chord_candidate_sets_connect(
                notes,
                groups[cursor - 1],
                &candidate_sets[cursor - 1],
                groups[cursor],
                &candidate_sets[cursor],
                hand,
            )
        {
            cursor += 1;
        }
        optimize_chord_subrun(
            notes,
            &groups[start..cursor],
            &candidate_sets[start..cursor],
            hand,
            profile,
            output,
        );
        if cursor < groups.len() && !candidate_sets[cursor].is_empty() {
            cursor += 1;
        }
    }
}

fn chord_candidates(
    notes: &[FingeringNote],
    hand: FingeringHand,
    profile: HandSpanProfile,
) -> Vec<ChordCandidate> {
    if !(2..=FINGER_COUNT).contains(&notes.len()) {
        return Vec::new();
    }
    let mut pitch_order: Vec<_> = (0..notes.len()).collect();
    pitch_order.sort_by_key(|index| (notes[*index].pitch, *index));
    if pitch_order
        .windows(2)
        .any(|pair| notes[pair[0]].pitch == notes[pair[1]].pitch)
    {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    for mask in 1_u8..(1_u8 << FINGER_COUNT) {
        if mask.count_ones() as usize != notes.len() {
            continue;
        }
        let mut ordered_fingers: Vec<_> = (1..=5)
            .filter(|finger| mask & (1 << (finger - 1)) != 0)
            .collect();
        if hand == FingeringHand::Left {
            ordered_fingers.reverse();
        }

        let mut assigned = vec![0_u8; notes.len()];
        for (index, finger) in pitch_order.iter().zip(ordered_fingers) {
            assigned[*index] = finger;
        }
        if notes.iter().zip(&assigned).any(|(note, finger)| {
            note.anchored_finger
                .filter(|anchor| (1..=5).contains(anchor))
                .is_some_and(|anchor| anchor != *finger)
        }) {
            continue;
        }

        let cost = chord_shape_cost(notes, &assigned, &pitch_order, hand, profile);
        candidates.push(ChordCandidate {
            shape_cost: cost,
            fingers: assigned,
        });
    }
    candidates
}

fn optimize_chord_subrun(
    notes: &[FingeringNote],
    groups: &[(usize, usize)],
    candidates: &[Vec<ChordCandidate>],
    hand: FingeringHand,
    profile: HandSpanProfile,
    output: &mut [Option<FingerSuggestion>],
) {
    let mut costs: Vec<Vec<i32>> = candidates
        .iter()
        .map(|group| vec![INFINITY; group.len()])
        .collect();
    let mut previous: Vec<Vec<usize>> = candidates
        .iter()
        .map(|group| vec![0; group.len()])
        .collect();
    for (index, candidate) in candidates[0].iter().enumerate() {
        costs[0][index] = candidate.shape_cost;
    }
    for group_index in 1..groups.len() {
        let (prior_start, prior_end) = groups[group_index - 1];
        let (next_start, next_end) = groups[group_index];
        for (next_index, next) in candidates[group_index].iter().enumerate() {
            for (prior_index, prior) in candidates[group_index - 1].iter().enumerate() {
                let cost = costs[group_index - 1][prior_index]
                    + next.shape_cost
                    + chord_transition_cost(
                        &notes[prior_start..prior_end],
                        &prior.fingers,
                        &notes[next_start..next_end],
                        &next.fingers,
                        hand,
                    );
                if cost < costs[group_index][next_index] {
                    costs[group_index][next_index] = cost;
                    previous[group_index][next_index] = prior_index;
                }
            }
        }
    }

    let mut selected = vec![0; groups.len()];
    selected[groups.len() - 1] = costs[groups.len() - 1]
        .iter()
        .enumerate()
        .min_by_key(|(_, cost)| *cost)
        .map_or(0, |(index, _)| index);
    for group_index in (1..groups.len()).rev() {
        selected[group_index - 1] = previous[group_index][selected[group_index]];
    }

    for group_index in 0..groups.len() {
        let (start, end) = groups[group_index];
        let chord = &notes[start..end];
        let candidate = &candidates[group_index][selected[group_index]];
        let chord_span = chord
            .iter()
            .map(|note| note.pitch)
            .max()
            .unwrap_or(0)
            .saturating_sub(chord.iter().map(|note| note.pitch).min().unwrap_or(0));
        let wide = i32::from(chord_span) > profile.comfortable_spans()[4];
        let prior = (group_index > 0).then(|| {
            let (prior_start, prior_end) = groups[group_index - 1];
            (
                &notes[prior_start..prior_end],
                &candidates[group_index - 1][selected[group_index - 1]].fingers,
            )
        });
        let has_held_context = prior.is_some_and(|(prior_notes, _)| {
            prior_notes.iter().any(|note| note.end > chord[0].onset)
        });
        for note_index in 0..chord.len() {
            let finger = candidate.fingers[note_index];
            let connected = prior.is_some_and(|(prior_notes, prior_fingers)| {
                prior_notes.iter().enumerate().any(|(index, prior_note)| {
                    prior_note.pitch == chord[note_index].pitch && prior_fingers[index] == finger
                })
            });
            let reason = if chord[note_index]
                .anchored_finger
                .is_some_and(|anchor| (1..=5).contains(&anchor))
            {
                FingeringReason::ManualAnchor
            } else if wide {
                FingeringReason::WideChordShape
            } else if connected {
                FingeringReason::ChordConnection
            } else if has_held_context {
                FingeringReason::HeldChordPosition
            } else {
                FingeringReason::ChordShape
            };
            output[start + note_index] = Some(FingerSuggestion {
                finger,
                confidence_percent: confidence(reason),
                reason,
            });
        }
    }
}

fn chord_transition_cost(
    prior_notes: &[FingeringNote],
    prior_fingers: &[u8],
    next_notes: &[FingeringNote],
    next_fingers: &[u8],
    hand: FingeringHand,
) -> i32 {
    if !held_transition_valid(prior_notes, prior_fingers, next_notes, next_fingers, hand) {
        return INFINITY;
    }
    let mut cost = 0;
    for (prior_index, prior_note) in prior_notes.iter().enumerate() {
        for (next_index, next_note) in next_notes.iter().enumerate() {
            if prior_note.pitch == next_note.pitch
                && prior_fingers[prior_index] != next_fingers[next_index]
            {
                cost += 3;
            }
        }
    }
    for finger in 1..=5 {
        let prior_pitch = prior_notes
            .iter()
            .zip(prior_fingers)
            .find_map(|(note, assigned)| (*assigned == finger).then_some(note.pitch));
        let next_pitch = next_notes
            .iter()
            .zip(next_fingers)
            .find_map(|(note, assigned)| (*assigned == finger).then_some(note.pitch));
        if let (Some(prior_pitch), Some(next_pitch)) = (prior_pitch, next_pitch) {
            cost += i32::from(prior_pitch.abs_diff(next_pitch));
        }
    }
    cost
}

fn chord_candidate_sets_connect(
    notes: &[FingeringNote],
    prior_group: (usize, usize),
    prior_candidates: &[ChordCandidate],
    next_group: (usize, usize),
    next_candidates: &[ChordCandidate],
    hand: FingeringHand,
) -> bool {
    prior_candidates.iter().any(|prior| {
        next_candidates.iter().any(|next| {
            held_transition_valid(
                &notes[prior_group.0..prior_group.1],
                &prior.fingers,
                &notes[next_group.0..next_group.1],
                &next.fingers,
                hand,
            )
        })
    })
}

fn held_transition_valid(
    prior_notes: &[FingeringNote],
    prior_fingers: &[u8],
    next_notes: &[FingeringNote],
    next_fingers: &[u8],
    hand: FingeringHand,
) -> bool {
    let Some(next_onset) = next_notes.first().map(|note| note.onset) else {
        return false;
    };
    for (prior_note, prior_finger) in prior_notes.iter().zip(prior_fingers) {
        if prior_note.end <= next_onset {
            continue;
        }
        for (next_note, next_finger) in next_notes.iter().zip(next_fingers) {
            if prior_finger == next_finger && prior_note.pitch != next_note.pitch {
                return false;
            }
            let ordered = match prior_note.pitch.cmp(&next_note.pitch) {
                std::cmp::Ordering::Less => match hand {
                    FingeringHand::Right => prior_finger < next_finger,
                    FingeringHand::Left => prior_finger > next_finger,
                },
                std::cmp::Ordering::Equal => prior_finger == next_finger,
                std::cmp::Ordering::Greater => match hand {
                    FingeringHand::Right => prior_finger > next_finger,
                    FingeringHand::Left => prior_finger < next_finger,
                },
            };
            if !ordered {
                return false;
            }
        }
    }
    true
}

fn chord_shape_cost(
    notes: &[FingeringNote],
    fingers: &[u8],
    pitch_order: &[usize],
    hand: FingeringHand,
    profile: HandSpanProfile,
) -> i32 {
    let low_pitch = notes[pitch_order[0]].pitch;
    let span = i32::from(
        notes[pitch_order[pitch_order.len() - 1]]
            .pitch
            .saturating_sub(low_pitch),
    )
    .max(1);
    let mut cost = 0;

    for index in pitch_order {
        let pitch_offset = i32::from(notes[*index].pitch.saturating_sub(low_pitch));
        let physical_position = match fingers[*index] {
            finger @ 1..=5 => i32::from(finger - 1),
            _ => return INFINITY,
        };
        let physical_position = match hand {
            FingeringHand::Right => physical_position,
            FingeringHand::Left => 4 - physical_position,
        };
        cost += (physical_position * span - pitch_offset * 4).abs();
        cost += static_key_cost(notes[*index].pitch, fingers[*index]);
    }

    for first in 0..pitch_order.len() {
        for second in (first + 1)..pitch_order.len() {
            let low = pitch_order[first];
            let high = pitch_order[second];
            let distance = i32::from(notes[high].pitch - notes[low].pitch);
            let finger_gap = usize::from(fingers[low].abs_diff(fingers[high]));
            let comfortable = profile.comfortable_spans()[finger_gap];
            cost += (distance - comfortable).max(0) * 8;
        }
    }
    cost
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
        FingeringReason::ChordConnection => 84,
        FingeringReason::HeldChordPosition => 82,
        FingeringReason::ChordShape => 78,
        FingeringReason::ThumbUnder | FingeringReason::FingerOver => 72,
        FingeringReason::PhraseStart => 65,
        FingeringReason::WideChordShape => 50,
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
            .map(|(index, pitch)| {
                let onset = Duration::from_millis(index as u64 * 500);
                FingeringNote {
                    pitch: *pitch,
                    onset,
                    end: onset + Duration::from_millis(400),
                    anchored_finger: None,
                }
            })
            .collect()
    }

    fn chord_note(pitch: u8, onset_ms: u64) -> FingeringNote {
        let onset = Duration::from_millis(onset_ms);
        FingeringNote {
            pitch,
            onset,
            end: onset + Duration::from_millis(400),
            anchored_finger: None,
        }
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
    fn root_position_triad_uses_an_ordered_five_finger_shape() {
        let source = vec![chord_note(60, 0), chord_note(64, 0), chord_note(67, 0)];
        let right = suggest_fingerings(&source, FingeringHand::Right);
        let left = suggest_fingerings(&source, FingeringHand::Left);
        assert_eq!(
            right
                .iter()
                .map(|suggestion| suggestion.unwrap().finger)
                .collect::<Vec<_>>(),
            [1, 3, 5]
        );
        assert_eq!(
            left.iter()
                .map(|suggestion| suggestion.unwrap().finger)
                .collect::<Vec<_>>(),
            [5, 3, 1]
        );
        assert!(
            right
                .iter()
                .all(|suggestion| suggestion.unwrap().reason == FingeringReason::ChordShape)
        );
    }

    #[test]
    fn chord_manual_anchors_are_hard_constraints() {
        let mut source = vec![chord_note(60, 0), chord_note(64, 0), chord_note(67, 0)];
        source[0].anchored_finger = Some(1);
        source[2].anchored_finger = Some(4);
        let suggestions = suggest_fingerings(&source, FingeringHand::Right);
        assert_eq!(suggestions[0].unwrap().finger, 1);
        assert_eq!(suggestions[2].unwrap().finger, 4);
        assert_eq!(
            suggestions[2].unwrap().reason,
            FingeringReason::ManualAnchor
        );

        source[0].anchored_finger = Some(5);
        assert_eq!(
            suggest_fingerings(&source, FingeringHand::Right),
            [None, None, None]
        );
    }

    #[test]
    fn oversized_or_duplicate_chords_are_left_without_false_precision() {
        let six_notes: Vec<_> = [60, 62, 64, 65, 67, 69]
            .into_iter()
            .map(|pitch| chord_note(pitch, 0))
            .collect();
        assert_eq!(
            suggest_fingerings(&six_notes, FingeringHand::Right),
            [None; 6]
        );

        let duplicate = vec![chord_note(60, 0), chord_note(60, 0)];
        assert_eq!(
            suggest_fingerings(&duplicate, FingeringHand::Right),
            [None, None]
        );
    }

    #[test]
    fn compact_profile_warns_on_an_octave_chord() {
        let octave = vec![chord_note(60, 0), chord_note(72, 0)];
        let suggestions = suggest_fingerings_with_profile(
            &octave,
            FingeringHand::Right,
            HandSpanProfile::Compact,
        );
        assert_eq!(suggestions[0].unwrap().finger, 1);
        assert_eq!(suggestions[1].unwrap().finger, 5);
        assert!(
            suggestions
                .iter()
                .all(|suggestion| suggestion.unwrap().reason == FingeringReason::WideChordShape)
        );
    }

    #[test]
    fn adjacent_chords_keep_common_tones_under_the_same_fingers() {
        let source: Vec<_> = [(60, 0), (64, 0), (67, 0), (60, 500), (64, 500), (69, 500)]
            .into_iter()
            .map(|(pitch, millis)| chord_note(pitch, millis))
            .collect();
        let suggestions = suggest_fingerings(&source, FingeringHand::Right);
        let fingers: Vec<_> = suggestions
            .iter()
            .map(|suggestion| suggestion.unwrap().finger)
            .collect();

        assert_eq!(fingers, [1, 3, 5, 1, 3, 5]);
        assert_eq!(
            suggestions[3].unwrap().reason,
            FingeringReason::ChordConnection
        );
        assert_eq!(
            suggestions[4].unwrap().reason,
            FingeringReason::ChordConnection
        );
        assert_eq!(suggestions[5].unwrap().reason, FingeringReason::ChordShape);
    }

    #[test]
    fn voice_leading_can_break_a_vertical_tie_without_forcing_an_awkward_shape() {
        let isolated_first = vec![chord_note(64, 0), chord_note(67, 0), chord_note(72, 0)];
        let isolated_second = vec![chord_note(62, 0), chord_note(67, 0), chord_note(71, 0)];
        let first = suggest_fingerings(&isolated_first, FingeringHand::Right);
        let second = suggest_fingerings(&isolated_second, FingeringHand::Right);
        assert_eq!(first[1].unwrap().finger, 2);
        assert_eq!(second[1].unwrap().finger, 3);

        let mut sequence = isolated_first;
        sequence.extend(isolated_second.into_iter().map(|note| FingeringNote {
            onset: Duration::from_millis(500),
            end: Duration::from_millis(900),
            ..note
        }));
        let connected = suggest_fingerings(&sequence, FingeringHand::Right);
        let fingers: Vec<_> = connected
            .iter()
            .map(|suggestion| suggestion.unwrap().finger)
            .collect();

        assert_eq!(fingers, [1, 3, 5, 1, 3, 5]);
        assert_eq!(
            connected[4].unwrap().reason,
            FingeringReason::ChordConnection
        );
    }

    #[test]
    fn held_chord_tone_keeps_its_finger_free_and_preserves_hand_order() {
        let mut source = vec![
            chord_note(60, 0),
            chord_note(64, 0),
            chord_note(67, 0),
            chord_note(69, 500),
            chord_note(71, 500),
        ];
        source[0].end = Duration::from_millis(900);
        let suggestions = suggest_fingerings(&source, FingeringHand::Right);
        let held_finger = suggestions[0].unwrap().finger;
        let next_fingers = [
            suggestions[3].unwrap().finger,
            suggestions[4].unwrap().finger,
        ];

        assert!(next_fingers.iter().all(|finger| *finger != held_finger));
        assert!(next_fingers.iter().all(|finger| *finger > held_finger));
        assert!(
            suggestions[3..]
                .iter()
                .all(|suggestion| suggestion.unwrap().reason == FingeringReason::HeldChordPosition)
        );

        let held_high = [FingeringNote {
            pitch: 67,
            onset: Duration::ZERO,
            end: Duration::from_millis(900),
            anchored_finger: None,
        }];
        let lower_next = [chord_note(60, 500), chord_note(64, 500)];
        assert!(held_transition_valid(
            &held_high,
            &[1],
            &lower_next,
            &[5, 3],
            FingeringHand::Left,
        ));
        assert!(!held_transition_valid(
            &held_high,
            &[1],
            &lower_next,
            &[1, 3],
            FingeringHand::Left,
        ));
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
