use super::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FingeringReviewKind {
    WideReach,
    RapidTurn,
    RapidShift,
    SameFingerMove,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FingeringReview {
    pub index: usize,
    pub from_index: usize,
    pub kind: FingeringReviewKind,
    pub semitones: u8,
    pub comfortable_semitones: i32,
    pub interval_ms: Option<u64>,
}

/// This reports key-range overlap between two assigned hands. It describes
/// score positions, not a measured physical hand or arm collision.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CrossHandReviewKind { SameKey, CrossedRange }

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossHandReview {
    pub index: usize,
    pub opposite_index: usize,
    pub kind: CrossHandReviewKind,
    pub pitch: u8,
    pub opposite_pitch: u8,
    pub finger: u8,
    pub opposite_finger: u8,
    pub overlap_ms: u64,
}

pub fn review_cross_hand(
    notes: &[FingeringNote], fingers: &[u8], hand: FingeringHand,
    opposite: &[FingeringNote], opposite_fingers: &[u8],
) -> Vec<CrossHandReview> {
    if notes.len() != fingers.len() || opposite.len() != opposite_fingers.len()
        || fingers.iter().chain(opposite_fingers).any(|f| !(1..=5).contains(f)) {
        return vec![];
    }
    let mut reviews = Vec::new();
    for (index, note) in notes.iter().enumerate() {
        let mut best: Option<(u8, CrossHandReview)> = None;
        for (opposite_index, other) in opposite.iter().enumerate() {
            if other.onset >= note.end || other.end <= note.onset { continue; }
            let kind = if note.pitch == other.pitch { CrossHandReviewKind::SameKey }
                else if (hand == FingeringHand::Left && note.pitch > other.pitch)
                    || (hand == FingeringHand::Right && other.pitch > note.pitch) {
                    CrossHandReviewKind::CrossedRange
                } else { continue; };
            let overlap = note.end.min(other.end).saturating_sub(note.onset.max(other.onset));
            let severity = if kind == CrossHandReviewKind::SameKey { 2 } else { 1 };
            let candidate = CrossHandReview { index, opposite_index, kind,
                pitch:note.pitch, opposite_pitch:other.pitch,
                finger:fingers[index], opposite_finger:opposite_fingers[opposite_index],
                overlap_ms:overlap.as_millis().min(u128::from(u64::MAX)) as u64 };
            if best.as_ref().is_none_or(|(score, old)| severity > *score
                || (severity == *score && candidate.overlap_ms > old.overlap_ms)) {
                best = Some((severity, candidate));
            }
        }
        if let Some((_, review)) = best { reviews.push(review); }
    }
    reviews
}

/// A bounded keyboard-space heuristic for ranking *pairs* of hand plans.
/// Two semitones per finger estimates a hand's key-range center. This is a
/// ranking aid only; it cannot infer wrist, arm, or body movement.
pub fn cross_hand_plan_cost(
    left: &[FingeringNote], left_fingers: &[u8],
    right: &[FingeringNote], right_fingers: &[u8],
) -> i64 {
    if left.len()!=left_fingers.len() || right.len()!=right_fingers.len()
        || left_fingers.iter().chain(right_fingers).any(|f|!(1..=5).contains(f)) {return 0;}
    let mut total=0_i64;
    for (a, left_note) in left.iter().enumerate() {
        let mut worst=0_i64;
        for (b, right_note) in right.iter().enumerate() {
            if left_note.onset>=right_note.end || right_note.onset>=left_note.end {continue;}
            let left_center=i32::from(left_note.pitch)+2*(i32::from(left_fingers[a])-3);
            let right_center=i32::from(right_note.pitch)-2*(i32::from(right_fingers[b])-3);
            let crowding=(left_center-right_center+2).max(0).min(16) as i64;
            let same_key=if left_note.pitch==right_note.pitch {20} else {0};
            worst=worst.max(crowding+same_key);
        }
        total=total.saturating_add(worst);
    }
    total
}

// These are transparent review thresholds, not measured limits of a pianist.
// Keep them shared by path scoring and review so a cost has a visible cause.
fn movement_kind(a: FingeringNote, af: u8, b: FingeringNote, bf: u8,
    hand: FingeringHand, profile: HandSpanProfile, rate: f64,
) -> Option<(FingeringReviewKind, u64)> {
    if a.pitch == b.pitch || b.onset <= a.onset { return None; }
    let ms = (b.onset.saturating_sub(a.onset).as_secs_f64() * 1000.0 / rate).round() as u64;
    let distance = i32::from(a.pitch.abs_diff(b.pitch));
    if af == bf && ms < 250 { return Some((FingeringReviewKind::SameFingerMove, ms)); }
    let reason = transition_reason(a.pitch, af, b.pitch, bf, hand);
    if matches!(reason, FingeringReason::ThumbUnder | FingeringReason::FingerOver) && ms < 180 {
        return Some((FingeringReviewKind::RapidTurn, ms));
    }
    let delta = i16::from(b.pitch) - i16::from(a.pitch);
    let outward = if hand == FingeringHand::Right { delta > 0 } else { delta < 0 };
    let follows = if outward { bf > af } else { bf < af };
    let reach = profile.comfortable_spans()[usize::from(af.abs_diff(bf))];
    if ms < 250 && (distance > 12 || (follows && distance > reach) || (!follows && !matches!(reason, FingeringReason::ThumbUnder | FingeringReason::FingerOver))) {
        return Some((FingeringReviewKind::RapidShift, ms));
    }
    None
}

pub(super) fn timed_movement_cost(a: FingeringNote, af: u8, b: FingeringNote, bf: u8,
    hand: FingeringHand, profile: HandSpanProfile, rate: f64,
) -> i32 {
    match movement_kind(a, af, b, bf, hand, profile, rate) {
        Some((FingeringReviewKind::RapidTurn, ms)) => 2 + (180 - ms as i32) / 20,
        Some((FingeringReviewKind::RapidShift, ms)) => 4 + (250 - ms as i32) / 20,
        Some((FingeringReviewKind::SameFingerMove, ms)) => 6 + (250 - ms as i32) / 15,
        _ => 0,
    }
}

/// Finds actual simultaneous reaches (including anchored and held notes) and
/// time-sensitive single-note movements. Source indices can locate every item.
pub fn review_fingering_plan(notes: &[FingeringNote], fingers: &[u8], hand: FingeringHand,
    profile: HandSpanProfile, rate: f64,
) -> Vec<FingeringReview> {
    if notes.len() != fingers.len() || !rate.is_finite() || !(0.25..=2.0).contains(&rate)
        || fingers.iter().any(|f| !(1..=5).contains(f)) { return vec![]; }
    let mut reviews = vec![];
    let mut held = Vec::<usize>::new();
    let mut previous_group = 0..0;
    let mut start = 0;
    while start < notes.len() {
        let mut end = start + 1;
        while end < notes.len() && notes[end].onset == notes[start].onset { end += 1; }
        held.retain(|&i| notes[i].end > notes[start].onset);
        for i in start..end {
            // One worst reach per newly attacked note avoids pairwise noise.
            let worst = held.iter().copied().chain(start..i).filter_map(|j| {
                let span = i32::from(notes[i].pitch.abs_diff(notes[j].pitch));
                let comfortable = profile.comfortable_spans()[usize::from(fingers[i].abs_diff(fingers[j]))];
                (span > comfortable).then_some((span - comfortable, j, comfortable))
            }).max_by_key(|v| v.0);
            if let Some((_, j, comfortable)) = worst {
                reviews.push(FingeringReview { index:i, from_index:j, kind:FingeringReviewKind::WideReach,
                    semitones:notes[i].pitch.abs_diff(notes[j].pitch), comfortable_semitones:comfortable,
                    interval_ms:None });
            }
        }
        if end - start == 1 && previous_group.len() == 1 {
            let j = previous_group.start;
            if let Some((kind, ms)) = movement_kind(notes[j], fingers[j], notes[start], fingers[start], hand, profile, rate) {
                reviews.push(FingeringReview { index:start, from_index:j, kind,
                    semitones:notes[start].pitch.abs_diff(notes[j].pitch),
                    comfortable_semitones:profile.comfortable_spans()[usize::from(fingers[start].abs_diff(fingers[j]))],
                    interval_ms:Some(ms) });
            }
        }
        held.extend(start..end);
        previous_group = start..end;
        start = end;
    }
    reviews
}

#[cfg(test)]
mod tests {
    use super::*;
    fn n(pitch:u8, ms:u64, end:u64) -> FingeringNote {
        FingeringNote {pitch,onset:Duration::from_millis(ms),end:Duration::from_millis(end),anchored_finger:None}
    }
    #[test]
    fn review_includes_pinned_and_held_reaches_and_rate_sensitive_turns() {
        let mut notes=vec![n(60,0,1000),n(72,500,900)];
        notes[0].anchored_finger=Some(1);
        let review=review_fingering_plan(&notes,&[1,5],FingeringHand::Right,HandSpanProfile::Compact,1.0);
        assert_eq!(review.len(),1);
        assert_eq!(review[0].kind,FingeringReviewKind::WideReach);
        assert_eq!(review[0].from_index,0);
        assert_eq!(review[0].comfortable_semitones,10);
        let turn=vec![n(64,0,100),n(65,240,300)];
        assert!(review_fingering_plan(&turn,&[3,1],FingeringHand::Right,HandSpanProfile::Standard,1.0).is_empty());
        let fast=review_fingering_plan(&turn,&[3,1],FingeringHand::Right,HandSpanProfile::Standard,2.0);
        assert_eq!(fast[0].kind,FingeringReviewKind::RapidTurn);
        assert_eq!(fast[0].interval_ms,Some(120));
        assert!(timed_movement_cost(turn[0],3,turn[1],1,FingeringHand::Right,HandSpanProfile::Standard,2.0)>0);
        assert_eq!(timed_movement_cost(turn[0],3,turn[1],1,FingeringHand::Right,HandSpanProfile::Standard,1.0),0);
    }
    #[test]
    fn cross_hand_uses_actual_held_intervals_and_fingers() {
        let left = [n(76, 0, 1000), n(60, 1000, 1200)];
        let right = [n(72, 500, 900), n(76, 900, 1100)];
        let reviews = review_cross_hand(&left, &[2, 5], FingeringHand::Left,
            &right, &[3, 1]);
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].kind, CrossHandReviewKind::SameKey);
        assert_eq!((reviews[0].finger, reviews[0].opposite_finger), (2, 1));
        assert_eq!(reviews[0].overlap_ms, 100);
        let crossing = review_cross_hand(&left[..1], &[2], FingeringHand::Left,
            &right[..1], &[3]);
        assert_eq!(crossing[0].kind, CrossHandReviewKind::CrossedRange);
        assert_eq!(crossing[0].overlap_ms, 400);
        assert!(review_cross_hand(&left[..1], &[2], FingeringHand::Left,
            &[n(72, 1000, 1300)], &[3]).is_empty());
    }
    #[test]
    fn joint_cost_changes_with_chosen_fingers_but_rest_removes_cost() {
        let left=[n(64,0,1000)];let right=[n(67,500,900)];
        let crowded=cross_hand_plan_cost(&left,&[5],&right,&[5]);
        let open=cross_hand_plan_cost(&left,&[1],&right,&[1]);
        assert!(crowded>open);
        assert_eq!(cross_hand_plan_cost(&left,&[5],&[n(67,1000,1200)],&[5]),0);
        assert!(cross_hand_plan_cost(&left,&[2],&[n(64,500,900)],&[3])>crowded);
    }
}
