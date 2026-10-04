use super::*;
use crate::routine_commands::{PracticeRoutine, RoutineGoal, RoutineItem, RoutineTarget};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Suggestion {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) measure: usize,
    pub(super) accuracy: f64,
    pub(super) kind: String,
    pub(super) timing: Option<RhythmMetric>,
    pub(super) attempts: usize,
    pub(super) judged: usize,
    pub(super) source_at: u64,
    pub(super) source_speed: f32,
    pub(super) source_mode: String,
    pub(super) item: RoutineItem,
    pub(super) expression: Option<crate::expression_review::ExpressionMetric>,
    pub(super) range: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RhythmMetric {
    on_time: f64,
    samples: usize,
    offset_ms: f64,
    deviation_ms: f64,
}
#[derive(Default)]
struct RhythmEvidence {
    matched: usize,
    on_time: usize,
    samples: usize,
    attempts: usize,
    offsets: Vec<f64>,
    deviations: Vec<f64>,
}
struct Candidate {
    measure: usize,
    accuracy: f64,
    judged: usize,
    attempts: usize,
    timing: Option<RhythmMetric>,
}
fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n % 2 == 0 {
        (values[n / 2 - 1] + values[n / 2]) / 2.
    } else {
        values[n / 2]
    }
}
impl Player {
    fn weak_practice_rows(&self) -> Vec<Suggestion> {
        let mut rows = vec![];
        for (cid, history) in self.history.snapshot() {
            let Some(last) = history.sessions.iter().rev().find(|s| {
                s.context
                    .as_ref()
                    .is_some_and(|c| c.target_notes > 0 && !c.grid.is_empty())
                    && s.scope.as_ref().is_some_and(|v| !v.is_empty())
                    && s.mode
                        .as_deref()
                        .is_some_and(|m| matches!(m, "wait" | "flow" | "recital" | "memory"))
            }) else {
                continue;
            };
            let latest_grid = last.context.as_ref().unwrap().grid.clone();
            let mut groups = std::collections::BTreeSet::new();
            let mut sources = vec![];
            for s in history.sessions.iter().rev() {
                let Some(c) = s.context.as_ref() else {
                    continue;
                };
                if c.grid != latest_grid
                    || c.target_notes == 0
                    || s.scope.as_ref().is_none_or(|v| v.is_empty())
                    || !matches!(
                        s.mode.as_deref(),
                        Some("wait" | "flow" | "recital" | "memory")
                    )
                {
                    continue;
                }
                let key = serde_json::to_string(&json!([
                    s.kind,
                    s.scope,
                    s.mode,
                    s.hands,
                    s.speed,
                    s.effective_tempo_bpm,
                    c.exercise_spec,
                    c.tracks,
                    c.parts,
                    c.latency,
                    c.pedal_latency_ms.unwrap_or(0),
                    c.metronome,
                    c.adaptive
                ]))
                .unwrap();
                if groups.insert(key) {
                    sources.push(s);
                    if sources.len() == 16 {
                        break;
                    }
                }
            }
            for last in sources {
                let c = last.context.as_ref().unwrap();
                let mut measures: BTreeMap<usize, (usize, usize, usize)> = BTreeMap::new();
                let mut rhythm = BTreeMap::<usize, RhythmEvidence>::new();
                let matching: Vec<_> = history
                    .sessions
                    .iter()
                    .rev()
                    .filter(|s| {
                        s.kind == last.kind
                            && s.scope == last.scope
                            && s.mode == last.mode
                            && s.hands == last.hands
                            && s.effective_tempo_bpm == last.effective_tempo_bpm
                            && (s.speed - last.speed).abs() < 0.0001
                            && s.context.as_ref().is_some_and(|x| {
                                x.grid == c.grid
                                    && x.exercise_spec == c.exercise_spec
                                    && x.tracks == c.tracks
                                    && x.parts == c.parts
                                    && x.latency == c.latency
                                    && x.metronome == c.metronome
                                    && x.adaptive == c.adaptive
                            })
                    })
                    .take(8)
                    .collect();
                rows.extend(self.expression_practice_rows(&cid, &history, last, &matching));
                for s in matching {
                    for m in &s.summary.measures {
                        let b = m.breakdown;
                        if m.measure == 0
                            || m.measure > 1_000_000
                            || b.target_notes == 0
                            || b.matched_notes + b.missed_notes < b.target_notes
                        {
                            continue;
                        }
                        if last.mode.as_deref() != Some("wait") && m.timing.matched_samples >= 4 {
                            if let (Some(offset), Some(deviation)) =
                                (m.timing.median_offset_ms, m.timing.median_deviation_ms)
                            {
                                let r = rhythm.entry(m.measure).or_default();
                                r.matched += b.matched_notes;
                                r.on_time += b.on_time_notes;
                                r.samples += m.timing.matched_samples;
                                r.attempts += 1;
                                r.offsets.push(f64::from(offset));
                                r.deviations.push(f64::from(deviation));
                            }
                        }
                        let total = b.matched_notes + b.missed_notes + b.wrong_notes;
                        let entry = measures.entry(m.measure).or_default();
                        entry.0 += b.matched_notes;
                        entry.1 += total;
                        entry.2 += 1;
                    }
                }
                let mut candidates: Vec<_> = measures
                    .iter()
                    .map(|(m, v)| (*m, *v))
                    .filter(|(_, (_, judged, attempts))| *judged >= 8 && *attempts >= 2)
                    .filter_map(|(m, (matched, judged, attempts))| {
                        let a = matched as f64 / judged as f64;
                        (a < 0.9).then_some(Candidate {
                            measure: m,
                            accuracy: a,
                            judged,
                            attempts,
                            timing: None,
                        })
                    })
                    .collect();
                for (measure, mut r) in rhythm {
                    if r.attempts < 2 || r.samples < 12 || r.matched == 0 {
                        continue;
                    }
                    let on_time = r.on_time as f64 / r.matched as f64;
                    if on_time >= 0.8 {
                        continue;
                    }
                    let Some((matched, judged, _)) = measures.get(&measure) else {
                        continue;
                    };
                    if *judged == 0 {
                        continue;
                    }
                    let timing = RhythmMetric {
                        on_time,
                        samples: r.samples,
                        offset_ms: median(&mut r.offsets),
                        deviation_ms: median(&mut r.deviations),
                    };
                    candidates.push(Candidate {
                        measure,
                        accuracy: *matched as f64 / *judged as f64,
                        judged: *judged,
                        attempts: r.attempts,
                        timing: Some(timing),
                    });
                }
                candidates.sort_by(|a, b| {
                    a.timing
                        .as_ref()
                        .map_or(a.accuracy, |t| t.on_time)
                        .total_cmp(&b.timing.as_ref().map_or(b.accuracy, |t| t.on_time))
                        .then_with(|| b.judged.cmp(&a.judged))
                        .then_with(|| a.measure.cmp(&b.measure))
                });
                candidates.truncate(3);
                for Candidate {
                    measure,
                    accuracy,
                    judged,
                    attempts,
                    timing,
                } in candidates
                {
                    let is_rhythm = timing.is_some();
                    let mode = if last.mode.as_deref() == Some("wait") {
                        "wait"
                    } else {
                        "flow"
                    };
                    let mut item = RoutineItem {
                        id: String::new(),
                        title: format!(
                            "{} · 第 {} 小节",
                            history.display_name.chars().take(80).collect::<String>(),
                            measure
                        ),
                        notes: format!(
                            "最近同条件 {} 次完整小节结果，共 {} 次音符判断，正确率 {:.0}%。先按当前建议慢练，连续两轮达到 90%。可调整要求。",
                            attempts,
                            judged,
                            accuracy * 100.
                        ),
                        content_id: cid.clone(),
                        song_title: history.display_name.clone(),
                        path: c
                            .source_path
                            .clone()
                            .or_else(|| history.library.source_path.clone()),
                        exercise: c.exercise_spec,
                        grid: c.grid.clone(),
                        scope: last.scope.clone().unwrap(),
                        settings: preferences::SessionSettings {
                            tracks: c.tracks.clone(),
                            parts: c.parts.clone(),
                            mode: Some(mode.into()),
                            hands: c.hands.clone(),
                            count_in: 1,
                            metronome: c.metronome,
                            latency: self.state.latency,
                            rounds: 0,
                            adaptive: false,
                        },
                        speed: (f64::from(last.speed) * 0.8).clamp(0.25, 2.),
                        target: RoutineTarget::Passage {
                            start: measure,
                            end: measure,
                        },
                        goal: RoutineGoal {
                            expression: None,
                            passes: 2,
                            accuracy: 90,
                            on_time: None,
                            consecutive: true,
                            attempt_limit: 20,
                        },
                    };
                    if let Some(t) = &timing {
                        item.title = format!(
                            "{} · 第 {} 小节 · 节奏",
                            history.display_name.chars().take(72).collect::<String>(),
                            measure
                        );
                        item.notes = format!(
                            "最近同条件 {} 次完整小节，{} 个节奏样本，准时率 {:.0}%；各轮中位偏移汇总 {:.0} ms，波动汇总 {:.0} ms。跟随节拍器慢练，连续两轮正确率至少 90%、准时率至少 85%。偏移为描述证据，不单独判定达标。",
                            attempts,
                            t.samples,
                            t.on_time * 100.,
                            t.offset_ms,
                            t.deviation_ms
                        );
                        item.settings.mode = Some("flow".into());
                        item.settings.metronome = true;
                        item.goal.on_time = Some(85);
                    }
                    let id = blake3::hash(&serde_json::to_vec(&item).unwrap())
                        .to_hex()
                        .to_string();
                    rows.push(Suggestion {
                        id,
                        title: history.display_name.clone(),
                        measure,
                        accuracy,
                        kind: if is_rhythm { "rhythm" } else { "accuracy" }.into(),
                        timing,
                        expression: None,
                        range: None,
                        attempts,
                        judged,
                        source_at: last.recorded_at_unix_ms,
                        source_speed: last.speed,
                        source_mode: last.mode.clone().unwrap(),
                        item,
                    });
                }
            }
        }
        rows.sort_by(|a, b| {
            a.expression
                .as_ref()
                .map_or_else(
                    || a.timing.as_ref().map_or(a.accuracy, |t| t.on_time),
                    |e| e.priority,
                )
                .total_cmp(&b.expression.as_ref().map_or_else(
                    || b.timing.as_ref().map_or(b.accuracy, |t| t.on_time),
                    |e| e.priority,
                ))
                .then_with(|| b.source_at.cmp(&a.source_at))
                .then_with(|| a.id.cmp(&b.id))
        });
        let mut counts = BTreeMap::<String, usize>::new();
        let mut accepted = std::collections::BTreeSet::new();
        rows.retain(|r| {
            let conditions = serde_json::to_string(&json!([
                r.item.content_id,
                r.item.target,
                r.item.settings,
                r.item.speed,
                r.item.scope,
                r.item.grid,
                r.item.goal
            ]))
            .unwrap();
            if !accepted.insert(conditions) {
                return false;
            }
            let n = counts.entry(r.item.content_id.clone()).or_default();
            *n += 1;
            *n <= 3
        });
        rows.truncate(60);
        rows
    }
    pub(super) fn weak_practice(&self) -> Result<Value, String> {
        Ok(
            json!({"suggestions":self.weak_practice_rows(),"note":"每曲最近最多 16 组条件，分别取最近最多 8 次结果，至少 2 次完整小节、8 次音符判断；正确率低于 90% 提出音符复习；连续演奏至少两次、12 个节奏样本，准时率低于 80% 提出节奏复习。力度/踏板建议使用至少两轮完整范围、正确率至少 90% 的记录；力度每轮至少 8 个样本和 6 段变化，踏板参考至少 4 次转换且有演奏输入。建议沿用原整曲或选段，踏板扣除原输入校准后判断。等音不评价节奏或踏板时机，不推断姿势问题。打开时仍需核对文件、小节和分手条件。"}),
        )
    }
    pub(super) fn accept_weak_practice(
        &mut self,
        ids: &[String],
        routine_id: Option<&str>,
        day: Option<&str>,
        name: &str,
    ) -> Result<Value, String> {
        if ids.is_empty()
            || ids.len() > 60
            || ids.iter().collect::<std::collections::BTreeSet<_>>().len() != ids.len()
        {
            return Err("请选择 1～60 个不同的复习项目".into());
        }
        if day.is_some_and(|d| !routine_commands::day_valid(d)) {
            return Err("日期无效".into());
        }
        let rows = self.weak_practice_rows();
        let mut selected = vec![];
        for id in ids {
            selected.push(
                rows.iter()
                    .find(|r| &r.id == id)
                    .ok_or("成绩或设备校准已改变，请重新生成建议")?
                    .item
                    .clone(),
            );
        }
        let old = self.preferences.clone();
        let id = if let Some(id) = routine_id {
            self.routine_editable(id, day)?;
            id.to_owned()
        } else {
            if name.trim().is_empty()
                || name.chars().count() > 80
                || self.preferences.routines.len() >= 100
            {
                return Err("新计划名称需 1～80 字，常用计划最多 100 个".into());
            }
            let id = format!(
                "routine-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            self.preferences.routines.push(PracticeRoutine {
                schedule: None,
                id: id.clone(),
                name: name.trim().into(),
                notes: "由历史练习证据生成；条件与要求可逐项调整。".into(),
                items: vec![],
            });
            id
        };
        let result = (|| {
            let target =
                self.routine_target_mut(&id, if routine_id.is_some() { day } else { None })?;
            let mut added = 0;
            let mut skipped = 0;
            for mut item in selected {
                if target.items.iter().any(|i| {
                    i.content_id == item.content_id
                        && i.grid == item.grid
                        && i.scope == item.scope
                        && serde_json::to_value(&i.settings).unwrap()
                            == serde_json::to_value(&item.settings).unwrap()
                        && i.goal == item.goal
                        && (i.speed - item.speed).abs() < 0.0001
                        && serde_json::to_value(&i.target).unwrap()
                            == serde_json::to_value(&item.target).unwrap()
                }) {
                    skipped += 1;
                    continue;
                }
                if target.items.len() >= 200 {
                    return Err("计划最多 200 个项目".into());
                }
                item.id = format!(
                    "weak-item-{}-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos(),
                    added
                );
                target.items.push(item);
                added += 1;
            }
            if routine_id.is_none() && day.is_some() {
                self.routine_target_mut(&id, day)?;
            }
            if self.preferences.routine_runs.len() > 10000 {
                return Err("日期安排超过容量上限".into());
            }
            self.persist()?;
            Ok(json!({"id":id,"added":added,"skipped":skipped}))
        })();
        if result.is_err() {
            self.preferences = old;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_flow_timing_and_wait_exclusion() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work/cycle132-unit")
            .join(format!(
                "{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut p = Player::new(data, PathBuf::new(), true);
        p.command(Command::Generate {
            spec: ExerciseSpec::default(),
        })
        .unwrap();
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let title = p.title.clone();
        let mut summary = AttemptSummary::default();
        summary
            .measures
            .push(neothesia_core::practice::MeasureSummary {
                measure: 1,
                breakdown: neothesia_core::practice::PracticeBreakdown {
                    target_notes: 8,
                    matched_notes: 8,
                    on_time_notes: 2,
                    late_notes: 6,
                    ..Default::default()
                },
                timing: neothesia_core::practice::TimingSummary {
                    matched_samples: 8,
                    median_offset_ms: Some(120),
                    median_deviation_ms: Some(15),
                },
            });
        let mut sample = PracticeSession::new(
            PracticeSessionKind::WholeSong,
            neothesia_core::practice::PracticeHands::Both,
            1.,
            summary,
        )
        .with_context(p.history_context())
        .with_scope(p.practice_scope())
        .with_mode("flow");
        p.history
            .record_session(&cid, &title, sample.clone())
            .unwrap();
        assert!(p.weak_practice_rows().is_empty());
        sample.id = Some("flow-2".into());
        p.history
            .record_session(&cid, &title, sample.clone())
            .unwrap();
        let rows = p.weak_practice_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "rhythm");
        assert_eq!(rows[0].accuracy, 1.);
        assert_eq!(rows[0].timing.as_ref().unwrap().on_time, 0.25);
        assert_eq!(rows[0].timing.as_ref().unwrap().samples, 16);
        assert_eq!(rows[0].item.goal.on_time, Some(85));
        assert!(rows[0].item.settings.metronome);
        assert_eq!(rows[0].item.settings.mode.as_deref(), Some("flow"));
        sample.mode = Some("wait".into());
        sample.id = Some("wait-1".into());
        p.history
            .record_session(&cid, &title, sample.clone())
            .unwrap();
        sample.id = Some("wait-2".into());
        p.history
            .record_session(&cid, &title, sample.clone())
            .unwrap();
        assert!(
            p.weak_practice_rows()
                .iter()
                .all(|r| r.source_mode != "wait")
        );
        let before = p.preferences.routines.len();
        let id = p
            .accept_weak_practice(&[rows[0].id.clone()], None, None, "节奏复习")
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(p.preferences.routines.len(), before + 1);
        assert_eq!(
            p.preferences
                .routines
                .iter()
                .find(|r| r.id == id)
                .unwrap()
                .items[0]
                .goal
                .on_time,
            Some(85)
        );
    }
    #[test]
    fn repeated_complete_evidence_conditions_preview_and_plan_snapshots() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work/cycle131-unit")
            .join(format!(
                "{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut p = Player::new(data.clone(), PathBuf::new(), true);
        p.command(Command::Generate {
            spec: ExerciseSpec::default(),
        })
        .unwrap();
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let title = p.title.clone();
        let mut summary = AttemptSummary::default();
        summary
            .measures
            .push(neothesia_core::practice::MeasureSummary {
                measure: 1,
                breakdown: neothesia_core::practice::PracticeBreakdown {
                    target_notes: 8,
                    matched_notes: 4,
                    missed_notes: 4,
                    ..Default::default()
                },
                timing: Default::default(),
            });
        let mut sample = PracticeSession::new(
            PracticeSessionKind::WholeSong,
            neothesia_core::practice::PracticeHands::Both,
            1.,
            summary,
        )
        .with_context(p.history_context())
        .with_scope(p.practice_scope())
        .with_mode("wait");
        p.history
            .record_session(&cid, &title, sample.clone())
            .unwrap();
        assert!(p.weak_practice_rows().is_empty());
        sample.id = Some("second".into());
        p.history
            .record_session(&cid, &title, sample.clone())
            .unwrap();
        let rows = p.weak_practice_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].accuracy, 0.5);
        assert_eq!(rows[0].attempts, 2);
        let before = p.title.clone();
        let result = p
            .accept_weak_practice(&[rows[0].id.clone()], None, Some("2026-10-03"), "自动复习")
            .unwrap();
        let id = result["id"].as_str().unwrap();
        assert_eq!(p.preferences.routines[0].items.len(), 1);
        assert_eq!(p.title, before);
        assert_eq!(
            p.routines("2026-10-03").unwrap()["runs"][0]["routine"]["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let duplicate = p
            .accept_weak_practice(&[rows[0].id.clone()], Some(id), Some("2026-10-03"), "")
            .unwrap();
        assert_eq!(duplicate["added"], 0);
        assert_eq!(duplicate["skipped"], 1);
        p.state.latency = 31;
        assert!(
            p.accept_weak_practice(&[rows[0].id.clone()], Some(id), None, "")
                .is_err()
        );
        p.state.latency = 0;
        let mut slow = sample.clone();
        slow.id = Some("slow-loop".into());
        slow.speed = 0.5;
        slow.kind = PracticeSessionKind::Loop {
            start_measure: 1,
            end_measure: 1,
        };
        slow.summary.measures[0].breakdown.matched_notes = 8;
        slow.summary.measures[0].breakdown.missed_notes = 0;
        p.history.record_session(&cid, &title, slow).unwrap();
        assert_eq!(p.weak_practice_rows().len(), 1);
        assert_eq!(p.weak_practice_rows()[0].source_speed, 1.);
        sample.id = Some("changed-grid".into());
        sample.context.as_mut().unwrap().grid = "changed".into();
        p.history.record_session(&cid, &title, sample).unwrap();
        assert!(p.weak_practice_rows().is_empty());
        let mut reopened = Player::new(data, PathBuf::new(), true);
        assert_eq!(
            reopened.routines("2026-10-03").unwrap()["runs"][0]["routine"]["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
}
