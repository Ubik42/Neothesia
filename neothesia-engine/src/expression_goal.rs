use neothesia_core::practice::ExpressionSummary;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExpressionGoal {
    pub velocity_difference: Option<u8>,
    pub contour_percent: Option<u8>,
    pub pedal_offset_ms: Option<u16>,
}
impl ExpressionGoal {
    pub(crate) fn validate(&self, mode: &str) -> Result<(), String> {
        if self.velocity_difference.is_none()
            && self.contour_percent.is_none()
            && self.pedal_offset_ms.is_none()
        {
            return Err("请选择力度或踏板要求，或关闭专项要求".into());
        }
        if self
            .velocity_difference
            .is_some_and(|v| !(1..=64).contains(&v))
            || self
                .contour_percent
                .is_some_and(|v| !(50..=100).contains(&v))
            || self
                .pedal_offset_ms
                .is_some_and(|v| !(10..=500).contains(&v))
        {
            return Err("力度平均差需 1–64，起伏一致率需 50–100%，踏板偏移需 10–500 ms".into());
        }
        if mode == "wait" && self.pedal_offset_ms.is_some() {
            return Err("等音不评价踏板时机，请使用连续或完整演奏".into());
        }
        Ok(())
    }
    pub(crate) fn assess(&self, e: ExpressionSummary) -> Result<(), String> {
        if let Some(limit) = self.velocity_difference {
            if !e.has_velocity_evidence() || e.velocity.mean_abs_difference.is_none() {
                return Err(format!(
                    "力度样本不足：{} / 4 个",
                    e.velocity.matched_samples
                ));
            }
            let value = e.velocity.mean_abs_difference.unwrap();
            if value > limit {
                return Err(format!("力度平均差未达标：{value}，要求 ≤ {limit}"));
            }
        }
        if let Some(limit) = self.contour_percent {
            let v = e.velocity;
            if !e.has_velocity_contour_evidence() {
                return Err(format!("力度起伏样本不足：{} / 6 段", v.contour_steps));
            }
            if (v.contour_aligned as u128) * 100 < u128::from(limit) * (v.contour_steps as u128) {
                return Err(format!(
                    "力度起伏未达标：{} / {} 段一致，要求 {limit}%",
                    v.contour_aligned, v.contour_steps
                ));
            }
        }
        if let Some(limit) = self.pedal_offset_ms {
            let p = e.pedal;
            if !p.target_present || p.target_transitions < 4 {
                return Err("参考曲目的踏板转换不足 4 次".into());
            }
            if p.user_transitions != p.target_transitions
                || p.timing_samples != p.target_transitions
            {
                return Err(format!(
                    "踏板转换未完整对应：演奏 {} / 参考 {} 次",
                    p.user_transitions, p.target_transitions
                ));
            }
            if !e.has_pedal_timing_evidence() {
                return Err("踏板时机样本不足或波动超过 120 ms".into());
            }
            let offset = p.median_offset_ms.ok_or("踏板时机样本不足")?.unsigned_abs();
            if offset > u32::from(limit) {
                return Err(format!("踏板偏移未达标：{offset} ms，要求 ≤ {limit} ms"));
            }
        }
        Ok(())
    }
}
impl crate::Player {
    pub(crate) fn validate_expression_source(
        &mut self,
        item: &crate::routine_commands::RoutineItem,
    ) -> Result<(), String> {
        let Some(goal) = &item.goal.expression else {
            return Ok(());
        };
        if matches!(
            item.target,
            crate::routine_commands::RoutineTarget::Ladder { .. }
        ) {
            return Err("阶梯项目暂不支持力度或踏板达标要求".into());
        }
        let Some(file) = self
            .file
            .as_ref()
            .filter(|f| f.content_id == item.content_id)
        else {
            return Ok(());
        };
        let (start, end) = match &item.target {
            crate::routine_commands::RoutineTarget::Passage { start, end } => {
                let a = file
                    .musical_time
                    .measures
                    .get(start.saturating_sub(1))
                    .ok_or("项目小节范围无效")?;
                let b = file
                    .musical_time
                    .measures
                    .get(end.saturating_sub(1))
                    .ok_or("项目小节范围无效")?;
                (
                    file.tempo_track.pulses_to_duration(a.start_tick),
                    file.tempo_track.pulses_to_duration(b.end_tick),
                )
            }
            crate::routine_commands::RoutineTarget::PrecisePassage {range} => (file.tempo_track.pulses_to_duration(range.start_tick),file.tempo_track.pulses_to_duration(range.end_tick)),
            crate::routine_commands::RoutineTarget::ScorePassage {range} => (file.tempo_track.pulses_to_duration(range.start_tick),file.tempo_track.pulses_to_duration(range.end_tick)),
            _ => (
                std::time::Duration::ZERO,
                std::time::Duration::from_secs_f64(self.state.duration),
            ),
        };
        let old = self.config.clone();
        let hands = self.state.hands.clone();
        self.config
            .apply_practice_setup(&neothesia_core::practice_history::SongPracticeSetup {
                tracks: item.settings.tracks.clone(),
                ..Default::default()
            });
        for t in &mut self.config.tracks {
            if let Some(p) = item.settings.parts.get(&t.track_id) {
                t.practice_part = *p
            }
        }
        self.state.hands = item.settings.hands.clone();
        let mut points = std::collections::BTreeMap::new();
        let mut count = 0;
        for n in &self.notes {
            if n.start >= start
                && n.start < end
                && n.velocity > 0
                && self.note_player(n.track_id, n.index)
                    == neothesia_core::song_config::PlayerConfig::Human
            {
                count += 1;
                let v = points.entry(n.start).or_insert((0u32, 0u32));
                v.0 += u32::from(n.velocity);
                v.1 += 1;
            }
        }
        let values: Vec<_> = points
            .values()
            .map(|(sum, count)| (sum / count) as i32)
            .collect();
        let steps = values
            .windows(2)
            .filter(|v| (v[1] - v[0]).abs() >= 6)
            .count();
        let mut last = None;
        let mut transitions = 0;
        for e in &self.events {
            if e.timestamp >= start && e.timestamp < end && self.selected(e.track_id) {
                if let midi_file::midly::MidiMessage::Controller { controller, value } = e.message {
                    if controller.as_int() == 64 {
                        let down = value.as_int() >= 64;
                        if last != Some(down) && (last.is_some() || down) {
                            transitions += 1;
                        }
                        last = Some(down);
                    }
                }
            }
        }
        self.config = old;
        self.state.hands = hands;
        if goal.velocity_difference.is_some() && count < 4 {
            return Err("所选练习范围的参考力度不足 4 个音符".into());
        }
        if goal.contour_percent.is_some() && steps < 6 {
            return Err(format!(
                "所选范围仅有 {steps} 段明显力度变化，起伏目标至少需要 6 段"
            ));
        }
        if goal.pedal_offset_ms.is_some() && transitions < 4 {
            return Err(format!(
                "所选范围仅有 {transitions} 次踏板转换，时机目标至少需要 4 次"
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thresholds_samples_and_pedal_correspondence() {
        let mut e = ExpressionSummary::default();
        let g = ExpressionGoal {
            velocity_difference: Some(12),
            contour_percent: Some(80),
            pedal_offset_ms: Some(80),
        };
        assert!(g.validate("wait").is_err());
        assert!(g.validate("flow").is_ok());
        assert!(g.assess(e).unwrap_err().contains("样本不足"));
        e.velocity.matched_samples = 8;
        e.velocity.mean_abs_difference = Some(20);
        assert!(g.assess(e).unwrap_err().contains("平均差未达标"));
        e.velocity.mean_abs_difference = Some(12);
        e.velocity.contour_steps = 6;
        e.velocity.contour_aligned = 4;
        assert!(g.assess(e).unwrap_err().contains("起伏未达标"));
        e.velocity.contour_aligned = 5;
        assert!(g.assess(e).unwrap_err().contains("参考曲目"));
        e.pedal.target_present = true;
        e.pedal.target_transitions = 4;
        e.pedal.user_transitions = 3;
        assert!(g.assess(e).unwrap_err().contains("完整对应"));
        e.pedal.user_transitions = 4;
        e.pedal.timing_samples = 4;
        e.pedal.median_offset_ms = Some(-90);
        e.pedal.median_deviation_ms = Some(20);
        assert!(g.assess(e).unwrap_err().contains("偏移未达标"));
        e.pedal.median_offset_ms = Some(-80);
        assert!(g.assess(e).is_ok());
        e.pedal.median_deviation_ms = Some(121);
        assert!(g.assess(e).is_err());
    }
}

#[cfg(test)]
mod flow_tests {
    use super::*;
    use crate::{Command, Player, routine_commands::RoutineGoal};
    use std::{path::PathBuf, time::Duration};
    #[test]
    fn actual_pedal_input_rounds_fail_then_pass_and_keep_history_goal() {
        use midi_file::midly::{
            Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
        };
        let mut events = vec![];
        for i in 0..8 {
            events.push((
                i * 240,
                TrackEventKind::Midi {
                    channel: 0.into(),
                    message: MidiMessage::NoteOn {
                        key: 60.into(),
                        vel: 80.into(),
                    },
                },
            ));
            events.push((
                i * 240 + 200,
                TrackEventKind::Midi {
                    channel: 0.into(),
                    message: MidiMessage::NoteOff {
                        key: 60.into(),
                        vel: 0.into(),
                    },
                },
            ));
        }
        for (tick, value) in [(120, 127), (600, 0), (1080, 127), (1560, 0)] {
            events.push((
                tick,
                TrackEventKind::Midi {
                    channel: 0.into(),
                    message: MidiMessage::Controller {
                        controller: 64.into(),
                        value: value.into(),
                    },
                },
            ))
        }
        events.push((1920, TrackEventKind::Meta(MetaMessage::EndOfTrack)));
        events.sort_by_key(|(t, _)| *t);
        let mut last = 0;
        let track = events
            .into_iter()
            .map(|(at, kind)| {
                let delta = at - last;
                last = at;
                TrackEvent {
                    delta: delta.into(),
                    kind,
                }
            })
            .collect();
        let smf = Smf {
            header: Header {
                format: Format::SingleTrack,
                timing: Timing::Metrical(480.into()),
            },
            tracks: vec![track],
        };
        let mut bytes = vec![];
        smf.write_std(&mut bytes).unwrap();
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work/cycle135-unit")
            .join(format!(
                "pedal-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut p = Player::new(data.clone(), PathBuf::new(), true);
        p.command(Command::ImportMidi {
            bytes,
            name: "踏板课堂.mid".into(),
        })
        .unwrap();
        p.command(Command::Mode {
            value: "flow".into(),
        })
        .unwrap();
        p.command(Command::CountIn { bars: 0 }).unwrap();
        p.command(Command::Latency { milliseconds: 100 }).unwrap();
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let rid = p
            .save_routine(None, None, "踏板时机".into(), "".into(), None)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let goal = RoutineGoal {
            passes: 1,
            accuracy: 90,
            on_time: None,
            consecutive: false,
            attempt_limit: 10,
            expression: Some(ExpressionGoal {
                pedal_offset_ms: Some(80),
                ..Default::default()
            }),
        };
        let item = p
            .save_routine_item(
                &rid,
                None,
                None,
                Some("current"),
                None,
                "踏板对应".into(),
                "".into(),
                goal,
            )
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        p.open_routine_item(&rid, "2026-10-06", &item, false)
            .unwrap();
        for (round, delay) in [(1, 0.3), (2, 0.12)] {
            let mut note = 0;
            let mut pedal = 0;
            p.command(Command::Play).unwrap();
            for _ in 0..1500 {
                p.tick(Duration::from_millis(10));
                let position = p.state.position;
                while note < 8 && position >= note as f64 * 0.25 + 0.01 {
                    p.midi(&[144, 60, 80]);
                    p.midi(&[128, 60, 0]);
                    note += 1;
                }
                let times = [0.125, 0.625, 1.125, 1.625];
                while pedal < 4 && position >= times[pedal] + delay {
                    p.midi(&[176, 64, if pedal % 2 == 0 { 127 } else { 0 }]);
                    pedal += 1;
                }
                if p.routine_status().unwrap()["progress"]["attempts"] == round {
                    break;
                }
            }
            let status = p.routine_status().unwrap();
            assert_eq!(status["progress"]["attempts"], round, "state={} position={} error={:?} routine={status}",p.state.status,p.state.position,p.state.error);
            if round == 1 {
                assert_eq!(status["progress"]["passed"], 0);
                assert!(
                    status["progress"]["lastResult"]
                        .as_str()
                        .unwrap()
                        .contains("踏板偏移未达标")
                );
            } else {
                assert_eq!(status["progress"]["completed"], true);
            }
        }
        let sessions = &p.history.song(&cid).unwrap().sessions;
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[1].summary.expression.pedal.timing_samples, 4);
        assert_eq!(
            sessions[1].context.as_ref().unwrap().pedal_latency_ms,
            Some(100)
        );
        assert!(
            sessions[1]
                .summary
                .expression
                .pedal
                .median_offset_ms
                .unwrap()
                .abs()
                <= 30
        );
        assert_eq!(
            sessions[1]
                .context
                .as_ref()
                .unwrap()
                .routine
                .as_ref()
                .unwrap()
                .expression
                .as_ref()
                .unwrap()
                .pedal_offset_ms,
            Some(80)
        );
        let backup = p.practice_backup_snapshot(&Default::default()).unwrap();
        let backup: crate::practice_backup::Backup = serde_json::from_value(backup).unwrap();
        backup.validate().unwrap();
        assert_eq!(backup.version(), 8);
        let restored = crate::practice_backup::Backup::decode(&backup.encode().unwrap()).unwrap();
        assert_eq!(
            restored.routines[0].items[0]
                .goal
                .expression
                .as_ref()
                .unwrap()
                .pedal_offset_ms,
            Some(80)
        );
        let mut reopened = Player::new(data, PathBuf::new(), true);
        reopened.restore();
        assert!(
            reopened.preferences.routines[0].items[0]
                .goal
                .expression
                .is_some()
        );
    }
}

#[cfg(test)]
mod source_tests {
    use super::*;
    use crate::{Command, Player, routine_commands::RoutineGoal};
    #[test]
    fn reject_flat_reference_without_modifying_plan_or_scope() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work/cycle135-unit")
            .join(format!(
                "flat-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut p = Player::new(data, std::path::PathBuf::new(), true);
        p.command(Command::Generate {
            spec: Default::default(),
        })
        .unwrap();
        let id = p
            .save_routine(None, None, "参考检查".into(), "".into(), None)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let scope = p.practice_scope();
        let goal = RoutineGoal {
            expression: Some(ExpressionGoal {
                contour_percent: Some(80),
                ..Default::default()
            }),
            passes: 1,
            accuracy: 90,
            on_time: None,
            consecutive: false,
            attempt_limit: 20,
        };
        let e = p
            .save_routine_item(
                &id,
                None,
                None,
                Some("current"),
                None,
                "起伏".into(),
                "".into(),
                goal,
            )
            .unwrap_err();
        assert!(e.contains("明显力度变化"));
        assert!(p.preferences.routines[0].items.is_empty());
        assert_eq!(p.practice_scope(), scope);
    }
}
