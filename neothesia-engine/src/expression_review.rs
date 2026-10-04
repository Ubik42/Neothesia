use crate::{
    Player, PracticeSession, PracticeSessionKind,
    expression_goal::ExpressionGoal,
    preferences::SessionSettings,
    routine_commands::{RoutineGoal, RoutineItem, RoutineTarget},
    weak_practice::Suggestion,
};
use serde::Serialize;
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExpressionMetric {
    pub contour_percent: Option<f64>,
    pub contour_steps: usize,
    pub samples: usize,
    pub mean_difference: Option<f64>,
    pub pedal_offset_ms: Option<f64>,
    pub pedal_absolute_offset_ms: Option<f64>,
    pub pedal_deviation_ms: Option<f64>,
    pub reference_transitions: usize,
    pub correspondence_failures: usize,
    pub calibrated_legacy: usize,
    #[serde(skip)]
    pub priority: f64,
}
fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    Some(if n % 2 == 0 {
        (values[n / 2 - 1] + values[n / 2]) / 2.
    } else {
        values[n / 2]
    })
}
impl Player {
    pub(super) fn expression_practice_rows(
        &self,
        cid: &str,
        history: &neothesia_core::practice_history::SongPracticeHistory,
        last: &PracticeSession,
        sessions: &[&PracticeSession],
    ) -> Vec<Suggestion> {
        let Some(c) = &last.context else {
            return vec![];
        };
        let (target, range, measure) = match last.kind {
            PracticeSessionKind::WholeSong => (RoutineTarget::Whole, "全曲".to_string(), 0),
            PracticeSessionKind::Loop {
                start_measure: a,
                end_measure: b,
            } if a > 0 && b >= a && b <= 1000000 => (
                RoutineTarget::Passage { start: a, end: b },
                format!("第 {a}–{b} 小节"),
                a,
            ),
            _ => return vec![],
        };
        let mut contour = vec![];
        let mut pedal = vec![];
        for s in sessions {
            let Some(ctx) = &s.context else { continue };
            let o = s.summary.overall;
            let judged = o.matched_notes + o.missed_notes + o.wrong_notes;
            if judged == 0
                || ctx.target_notes == 0
                || o.matched_notes + o.missed_notes < ctx.target_notes
                || o.matched_notes as f64 / (judged as f64) < 0.9
            {
                continue;
            }
            let e = s.summary.expression;
            let v = e.velocity;
            if v.matched_samples >= 8
                && v.contour_steps >= 6
                && v.contour_aligned <= v.contour_steps
            {
                contour.push((*s, v, judged));
            }
            let p = e.pedal;
            if last.mode.as_deref() != Some("wait")
                && p.target_present
                && p.target_transitions >= 4
                && p.user_used
                && p.user_transitions > 0
            {
                pedal.push((*s, p, judged));
            }
        }
        let mut results = vec![];
        if contour.len() >= 2 {
            let steps: usize = contour.iter().map(|(_, v, _)| v.contour_steps).sum();
            let aligned: usize = contour.iter().map(|(_, v, _)| v.contour_aligned).sum();
            let ratio = aligned as f64 / steps as f64;
            let bad = contour
                .iter()
                .filter(|(_, v, _)| v.contour_aligned as f64 / (v.contour_steps as f64) < 0.8)
                .count();
            if bad >= 2 && ratio < 0.8 {
                let mut dif: Vec<_> = contour
                    .iter()
                    .filter_map(|(_, v, _)| v.mean_abs_difference.map(f64::from))
                    .collect();
                let samples = contour.iter().map(|(_, v, _)| v.matched_samples).sum();
                let judged = contour.iter().map(|(_, _, j)| *j).sum();
                let matched: usize = contour
                    .iter()
                    .map(|(s, _, _)| s.summary.overall.matched_notes)
                    .sum();
                let metric = ExpressionMetric {
                    contour_percent: Some(ratio * 100.),
                    contour_steps: steps,
                    samples,
                    mean_difference: median(&mut dif),
                    pedal_offset_ms: None,
                    pedal_absolute_offset_ms: None,
                    pedal_deviation_ms: None,
                    reference_transitions: 0,
                    correspondence_failures: 0,
                    calibrated_legacy: 0,
                    priority: ratio,
                };
                results.push((
                    "dynamics",
                    contour.len(),
                    judged,
                    matched as f64 / judged as f64,
                    metric,
                    ExpressionGoal {
                        contour_percent: Some(85),
                        ..Default::default()
                    },
                ));
            }
        }
        if pedal.len() >= 2 {
            let mut offsets = vec![];
            let mut deviations = vec![];
            let mut mismatch = 0;
            let mut bad = 0;
            let mut legacy = 0;
            let mut samples = 0;
            let mut transitions = 0;
            let mut judged = 0;
            let mut matched = 0;
            for (s, p, j) in &pedal {
                let ctx = s.context.as_ref().unwrap();
                let offset = p.median_offset_ms.map(|v| {
                    f64::from(v) - f64::from(ctx.latency)
                        + f64::from(ctx.pedal_latency_ms.unwrap_or(0))
                });
                let correspondence = p.user_transitions == p.target_transitions
                    && p.timing_samples == p.target_transitions;
                if !correspondence {
                    mismatch += 1;
                }
                if !correspondence
                    || offset.is_some_and(|v| v.abs() > 80.)
                    || p.median_deviation_ms.is_some_and(|v| v > 120)
                {
                    bad += 1;
                }
                if let Some(v) = offset {
                    offsets.push(v)
                }
                if let Some(v) = p.median_deviation_ms {
                    deviations.push(f64::from(v))
                }
                if ctx.pedal_latency_ms.is_none() && ctx.latency != 0 {
                    legacy += 1;
                }
                samples += p.timing_samples;
                transitions += p.target_transitions;
                judged += *j;
                matched += s.summary.overall.matched_notes;
            }
            let mut absolute_offsets: Vec<_> = offsets.iter().map(|v| v.abs()).collect();
            let absolute_offset = median(&mut absolute_offsets);
            let offset = median(&mut offsets);
            let deviation = median(&mut deviations);
            if bad >= 2
                && (mismatch >= 2
                    || absolute_offset.is_some_and(|v| v > 80.)
                    || deviation.is_some_and(|v| v > 120.))
            {
                let priority = if mismatch >= 2 {
                    0.2
                } else {
                    (80. / absolute_offset.unwrap_or(0.).max(80.))
                        .min(120. / deviation.unwrap_or(0.).max(120.))
                };
                let metric = ExpressionMetric {
                    contour_percent: None,
                    contour_steps: 0,
                    samples,
                    mean_difference: None,
                    pedal_offset_ms: offset,
                    pedal_absolute_offset_ms: absolute_offset,
                    pedal_deviation_ms: deviation,
                    reference_transitions: transitions,
                    correspondence_failures: mismatch,
                    calibrated_legacy: legacy,
                    priority,
                };
                results.push((
                    "pedal",
                    pedal.len(),
                    judged,
                    matched as f64 / judged as f64,
                    metric,
                    ExpressionGoal {
                        pedal_offset_ms: Some(80),
                        ..Default::default()
                    },
                ));
            }
        }
        results.into_iter().map(|(kind, attempts, judged, accuracy, metric, goal)| {
            let label = if kind == "dynamics" { "力度起伏" } else { "踏板时机" };
            let number = |v: Option<f64>| v.map_or("缺少".to_string(), |n| format!("{n:.0}"));
            let evidence = if kind == "dynamics" {
                format!("{} 段力度变化，方向一致 {:.0}%", metric.contour_steps, metric.contour_percent.unwrap())
            } else {
                format!("参考 {} 次踏板转换，{} 轮未完整对应；校准后的绝对偏移汇总 {} ms，有向偏移汇总 {} ms，波动汇总 {} ms", metric.reference_transitions, metric.correspondence_failures, number(metric.pedal_absolute_offset_ms), number(metric.pedal_offset_ms), number(metric.pedal_deviation_ms))
            };
            let item = RoutineItem {
                id: String::new(),
                title: format!("{} · {range} · {label}", history.display_name.chars().take(64).collect::<String>()),
                notes: format!("最近同条件 {attempts} 次完整范围、正确率至少 90%；{evidence}。按原整曲/选段慢练，连续两轮准确性与专项要求同时达标。偏移/波动汇总为各轮统计量的中位数，不是逐样本中位数。踏板先确认输入有效；不推断姿势问题。"),
                content_id: cid.to_string(),
                song_title: history.display_name.clone(),
                path: c.source_path.clone().or_else(|| history.library.source_path.clone()),
                exercise: c.exercise_spec,
                grid: c.grid.clone(),
                scope: last.scope.clone().unwrap_or_default(),
                settings: SessionSettings {
                    tracks: c.tracks.clone(),
                    parts: c.parts.clone(),
                    mode: Some(if kind == "dynamics" && last.mode.as_deref() == Some("wait") { "wait" } else { "flow" }.into()),
                    hands: c.hands.clone(),
                    count_in: 1,
                    metronome: kind == "pedal" || c.metronome,
                    latency: self.state.latency,
                    rounds: 0,
                    adaptive: false,
                },
                speed: (f64::from(last.speed) * 0.8).clamp(0.25, 2.),
                target: target.clone(),
                goal: RoutineGoal { expression: Some(goal), passes: 2, accuracy: 90, on_time: None, consecutive: true, attempt_limit: 20 },
            };
            let id = blake3::hash(&serde_json::to_vec(&item).unwrap()).to_hex().to_string();
            Suggestion {
                id, title: history.display_name.clone(), measure, range: Some(range.clone()), accuracy,
                kind: kind.into(), timing: None, expression: Some(metric), attempts, judged,
                source_at: last.recorded_at_unix_ms, source_speed: last.speed,
                source_mode: last.mode.clone().unwrap_or_default(), item,
            }
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, ExerciseSpec};
    use neothesia_core::practice::{AttemptSummary, PracticeHands};
    fn player() -> Player {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work/cycle136-unit")
            .join(format!(
                "{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut p = Player::new(root, std::path::PathBuf::new(), true);
        p.command(Command::Generate {
            spec: ExerciseSpec::default(),
        })
        .unwrap();
        p
    }
    fn sample(p: &Player) -> PracticeSession {
        let mut summary = AttemptSummary::default();
        summary.overall.matched_notes = 8;
        summary.expression.velocity.matched_samples = 8;
        summary.expression.velocity.contour_steps = 7;
        summary.expression.velocity.contour_flat = 7;
        let mut ctx = p.history_context();
        ctx.target_notes = 8;
        ctx.judged_notes = 8;
        PracticeSession::new(
            PracticeSessionKind::WholeSong,
            PracticeHands::Both,
            1.,
            summary,
        )
        .with_context(ctx)
        .with_scope(p.practice_scope())
        .with_mode("flow")
    }
    fn record(p: &mut Player, s: &PracticeSession, index: usize) {
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let mut s = s.clone();
        s.id = Some(format!("attempt-{index}"));
        p.history.record_session(&cid, &p.title.clone(), s).unwrap();
    }
    #[test]
    fn scope_evidence_selection_and_whole_song_dedup() {
        let mut p = player();
        let s = sample(&p);
        record(&mut p, &s, 1);
        assert!(
            p.weak_practice().unwrap()["suggestions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        record(&mut p, &s, 2);
        let data = p.weak_practice().unwrap();
        let row = &data["suggestions"][0];
        assert_eq!(row["kind"], "dynamics");
        assert_eq!(row["range"], "全曲");
        assert_eq!(row["expression"]["contourSteps"], 14);
        assert_eq!(row["item"]["goal"]["expression"]["contourPercent"], 85);
        let id = row["id"].as_str().unwrap().to_owned();
        let before = p.state.position;
        let result = p
            .accept_weak_practice(&[id.clone()], None, Some("2026-10-07"), "专项复习")
            .unwrap();
        assert_eq!(result["added"], 1);
        assert_eq!(p.state.position, before);
        let rid = result["id"].as_str().unwrap();
        let duplicate = p
            .accept_weak_practice(&[id.clone()], Some(rid), Some("2026-10-07"), "")
            .unwrap();
        assert_eq!(duplicate["skipped"], 1);
        let backup: crate::practice_backup::Backup =
            serde_json::from_value(p.practice_backup_snapshot(&Default::default()).unwrap())
                .unwrap();
        backup.validate().unwrap();
        assert_eq!(backup.version(), 2);
        p.state.latency = 20;
        assert!(
            p.accept_weak_practice(&[id], None, None, "陈旧建议")
                .is_err()
        );
        let mut looped = s.clone();
        looped.kind = PracticeSessionKind::Loop {
            start_measure: 2,
            end_measure: 3,
        };
        record(&mut p, &looped, 3);
        record(&mut p, &looped, 4);
        assert!(
            p.weak_practice().unwrap()["suggestions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["range"] == "第 2–3 小节"
                    && r["item"]["target"]["start"] == 2
                    && r["item"]["target"]["end"] == 3)
        );
    }
    #[test]
    fn incomplete_inaccurate_and_wait_pedal_are_excluded() {
        let mut p = player();
        let mut s = sample(&p);
        s.summary.overall.matched_notes = 7;
        record(&mut p, &s, 1);
        record(&mut p, &s, 2);
        assert!(
            p.weak_practice().unwrap()["suggestions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        s.summary.overall.missed_notes = 1;
        s.summary.overall.wrong_notes = 1;
        record(&mut p, &s, 3);
        record(&mut p, &s, 4);
        assert!(
            p.weak_practice().unwrap()["suggestions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        s.summary.overall.matched_notes = 8;
        s.summary.overall.missed_notes = 0;
        s.summary.overall.wrong_notes = 0;
        s.summary.expression.velocity.contour_aligned = 7;
        let ped = &mut s.summary.expression.pedal;
        ped.target_present = true;
        ped.target_transitions = 4;
        ped.user_used = true;
        ped.user_transitions = 4;
        ped.timing_samples = 4;
        ped.median_offset_ms = Some(200);
        ped.median_deviation_ms = Some(10);
        s.mode = Some("wait".into());
        record(&mut p, &s, 5);
        record(&mut p, &s, 6);
        assert!(
            p.weak_practice().unwrap()["suggestions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn opposite_round_biases_do_not_cancel_pedal_recommendation() {
        let mut p = player();
        let mut s = sample(&p);
        s.summary.expression.velocity.contour_aligned = 7;
        let ped = &mut s.summary.expression.pedal;
        ped.target_present = true;
        ped.target_transitions = 4;
        ped.user_used = true;
        ped.user_transitions = 4;
        ped.timing_samples = 4;
        ped.median_offset_ms = Some(200);
        ped.median_deviation_ms = Some(10);
        record(&mut p, &s, 1);
        s.summary.expression.pedal.median_offset_ms = Some(-200);
        record(&mut p, &s, 2);
        let data = p.weak_practice().unwrap();
        assert_eq!(data["suggestions"][0]["expression"]["pedalOffsetMs"], 0.);
        assert_eq!(
            data["suggestions"][0]["expression"]["pedalAbsoluteOffsetMs"],
            200.
        );
    }
    #[test]
    fn pedal_calibration_legacy_conversion_missing_input_and_correspondence() {
        let mut p = player();
        let mut s = sample(&p);
        s.summary.expression.velocity.contour_aligned = 7;
        s.context.as_mut().unwrap().latency = 200;
        s.context.as_mut().unwrap().pedal_latency_ms = None;
        let ped = &mut s.summary.expression.pedal;
        ped.target_present = true;
        ped.target_transitions = 4;
        ped.user_used = true;
        ped.user_transitions = 4;
        ped.timing_samples = 4;
        ped.median_offset_ms = Some(220);
        ped.median_deviation_ms = Some(10);
        record(&mut p, &s, 1);
        record(&mut p, &s, 2);
        assert!(
            p.weak_practice().unwrap()["suggestions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        s.summary.expression.pedal.median_offset_ms = Some(400);
        record(&mut p, &s, 3);
        record(&mut p, &s, 4);
        let data = p.weak_practice().unwrap();
        let r = &data["suggestions"][0];
        assert_eq!(r["kind"], "pedal");
        assert_eq!(r["expression"]["pedalOffsetMs"], 110.);
        assert_eq!(r["expression"]["calibratedLegacy"], 4);
        assert_eq!(r["item"]["goal"]["expression"]["pedalOffsetMs"], 80);
        assert_eq!(r["item"]["settings"]["metronome"], true);
        let mut fresh = player();
        s.context.as_mut().unwrap().pedal_latency_ms = Some(200);
        s.summary.expression.pedal.user_used = false;
        s.summary.expression.pedal.user_transitions = 0;
        record(&mut fresh, &s, 5);
        record(&mut fresh, &s, 6);
        assert!(
            fresh.weak_practice().unwrap()["suggestions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        s.summary.expression.pedal.user_used = true;
        s.summary.expression.pedal.user_transitions = 3;
        s.summary.expression.pedal.timing_samples = 3;
        record(&mut fresh, &s, 7);
        record(&mut fresh, &s, 8);
        assert_eq!(
            fresh.weak_practice().unwrap()["suggestions"][0]["expression"]["correspondenceFailures"],
            2
        );
    }
}
