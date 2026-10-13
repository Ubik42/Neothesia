use super::*;
use neothesia_core::practice_history::PracticeSession;

/// A score trend is evidence only when both attempts describe the same played task.
/// Older sessions without an immutable context remain readable, but cannot prove this.
pub(super) fn same_attempt_conditions(a: &PracticeSession, b: &PracticeSession) -> bool {
    if a.scope.as_deref().is_none_or(str::is_empty)
        || a.mode.as_deref().is_none_or(str::is_empty)
        || a.scope != b.scope
        || a.mode != b.mode
        || a.hands != b.hands
        || a.kind != b.kind
        || (a.speed - b.speed).abs() >= 0.0001
        || !a.speed.is_finite()
        || !b.speed.is_finite()
        || a.effective_tempo_bpm != b.effective_tempo_bpm
    {
        return false;
    }
    let (Some(a), Some(b)) = (&a.context, &b.context) else {
        return false;
    };
    !a.grid.is_empty()
        && !b.grid.is_empty()
        && !a.hands.is_empty()
        && !b.hands.is_empty()
        && a.target_notes > 0
        && b.target_notes > 0
        && a.judged_notes > 0
        && b.judged_notes > 0
        && a.grid == b.grid
        && a.tracks == b.tracks
        && a.parts == b.parts
        && a.hands == b.hands
        && a.exercise_spec == b.exercise_spec
        && a.target_notes == b.target_notes
        && a.judged_notes == b.judged_notes
        && a.count_in == b.count_in
        && a.metronome == b.metronome
        && a.latency == b.latency
        // Before separate pedal calibration existed, pedal timing used input latency.
        && a.pedal_latency_ms.unwrap_or(a.latency)
            == b.pedal_latency_ms.unwrap_or(b.latency)
        && a.rounds == b.rounds
        && a.adaptive == b.adaptive
        && a.routine.as_ref().and_then(|r| r.expression.as_ref())
            == b.routine.as_ref().and_then(|r| r.expression.as_ref())
}
pub(super) fn grid(file: &MidiFile) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for m in &file.musical_time.measures {
        for value in [
            m.start_tick,
            m.end_tick,
            u64::from(m.numerator),
            u64::from(m.denominator),
        ] {
            for b in value.to_le_bytes() {
                hash ^= u64::from(b);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
    }
    format!("grid-v1-{hash:016x}")
}
pub(super) fn scope(
    file: Option<&MidiFile>,
    notes: &[PracticeNote],
    config: &SongConfig,
    assigned: &HashMap<(usize, usize), PracticePart>,
    wanted: &str,
) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    let mut feed = |value: u64| {
        for b in value.to_le_bytes() {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    };
    if let Some(file) = file {
        for m in &file.musical_time.measures {
            feed(m.start_tick);
            feed(m.end_tick);
            feed(u64::from(m.numerator));
            feed(u64::from(m.denominator));
        }
    }
    for n in notes {
        let player = config
            .tracks
            .iter()
            .find(|t| t.track_id == n.track_id)
            .map_or(PlayerConfig::Mute, |t| t.player);
        let part = hands::part(config, assigned, n.track_id, n.index);
        if player == PlayerConfig::Human
            && match wanted {
                "left" => part == PracticePart::LeftHand,
                "right" => part == PracticePart::RightHand,
                _ => true,
            }
        {
            feed(n.track_id as u64);
            feed(n.index as u64);
            feed(match part {
                PracticePart::LeftHand => 1,
                PracticePart::RightHand => 2,
                _ => 0,
            });
        }
    }
    format!("practice-v1-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use neothesia_core::practice_history::PracticeSessionContext;
    use std::collections::BTreeMap;

    fn attempt() -> PracticeSession {
        PracticeSession {
            replay_clips: vec![], playing_ms: None, annotation: None, id: None,
            context: Some(PracticeSessionContext {
                routine: None, ladder: None, target_notes: 12, judged_notes: 12,
                tracks: vec![], parts: BTreeMap::new(), hands: "both".into(),
                count_in: 1, metronome: true, latency: 10,
                pedal_latency_ms: Some(10), rounds: 0, adaptive: false,
                grid: "grid-1".into(), source_path: None, exercise_spec: None,
            }),
            recorded_at_unix_ms: 1,
            kind: PracticeSessionKind::Loop { start_measure: 2, end_measure: 4 },
            hands: PracticeHands::Both, speed: 0.8, mode: Some("flow".into()),
            scope: Some("scope-1".into()), effective_tempo_bpm: Some(80),
            summary: AttemptSummary::default(),
        }
    }

    #[test]
    fn same_condition_requires_matching_note_coverage_and_pedal_calibration() {
        let original = attempt();
        assert!(same_attempt_conditions(&original, &original));
        let mut altered = original.clone();
        altered.context.as_mut().unwrap().target_notes = 10;
        assert!(!same_attempt_conditions(&original, &altered));
        altered = original.clone();
        altered.context.as_mut().unwrap().judged_notes = 10;
        assert!(!same_attempt_conditions(&original, &altered));
        altered = original.clone();
        altered.context.as_mut().unwrap().pedal_latency_ms = Some(18);
        assert!(!same_attempt_conditions(&original, &altered));
        altered = original.clone();
        altered.context.as_mut().unwrap().pedal_latency_ms = None;
        assert!(same_attempt_conditions(&original, &altered));
    }

    #[test]
    fn same_condition_rejects_changed_range_grid_settings_and_missing_legacy_context() {
        let original = attempt();
        let mut altered = original.clone();
        altered.kind = PracticeSessionKind::Loop { start_measure: 3, end_measure: 4 };
        assert!(!same_attempt_conditions(&original, &altered));
        altered = original.clone();
        altered.context.as_mut().unwrap().grid = "grid-2".into();
        assert!(!same_attempt_conditions(&original, &altered));
        altered = original.clone();
        altered.context.as_mut().unwrap().count_in = 2;
        assert!(!same_attempt_conditions(&original, &altered));
        altered = original.clone();
        altered.context.as_mut().unwrap().adaptive = true;
        assert!(!same_attempt_conditions(&original, &altered));
        altered = original.clone();
        altered.context = None;
        assert!(!same_attempt_conditions(&altered, &altered));
    }
}
