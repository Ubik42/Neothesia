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
}
