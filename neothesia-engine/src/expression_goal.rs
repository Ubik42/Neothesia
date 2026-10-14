use neothesia_core::practice::ExpressionSummary;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionGoalResult {
    pub passed: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_velocity: Option<NoteVelocityEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_pedal: Option<NotePedalEvidence>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteVelocityEvidence {
    pub reference_notes: usize,
    pub matched_notes: usize,
    pub missing_notes: usize,
    pub max_abs_difference: Option<u8>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotePedalEvidence {
    pub reference_notes: usize,
    pub matched_notes: usize,
    pub paired_state_notes: usize,
    pub missing_notes: usize,
    pub unknown_state_notes: usize,
    pub mismatch_count: usize,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExpressionGoal {
    pub velocity_difference: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_velocity_difference: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_pedal_mismatch_count: Option<u8>,
    pub contour_percent: Option<u8>,
    pub pedal_offset_ms: Option<u16>,
}
impl ExpressionGoal {
    #[cfg(test)]
    pub(crate) fn result_with_note_velocity(&self, e: ExpressionSummary, note_velocity: Option<NoteVelocityEvidence>) -> ExpressionGoalResult {
        self.result_with_note_evidence(e,note_velocity,None)
    }
    pub(crate) fn result_with_note_evidence(&self,e:ExpressionSummary,note_velocity:Option<NoteVelocityEvidence>,note_pedal:Option<NotePedalEvidence>)->ExpressionGoalResult{
        let evidence = self.note_velocity_difference.and(note_velocity);
        let pedal_evidence=self.note_pedal_mismatch_count.and(note_pedal);
        match self.assess_with_note_evidence(e,note_velocity,note_pedal) {
            Ok(()) => ExpressionGoalResult { passed: true,
                message: if self.note_velocity_difference.is_some() && self.velocity_difference.is_none()
                    && self.contour_percent.is_none() && self.pedal_offset_ms.is_none() && self.note_pedal_mismatch_count.is_none() {
                    "本轮逐音力度要求达标"
                } else if self.note_pedal_mismatch_count.is_some() && self.velocity_difference.is_none()
                    && self.note_velocity_difference.is_none() && self.contour_percent.is_none() && self.pedal_offset_ms.is_none() {
                    "本轮逐音踏板状态要求达标"
                } else { "本轮力度/踏板要求达标" }.into(), note_velocity: evidence, note_pedal:pedal_evidence },
            Err(message) => ExpressionGoalResult { passed: false, message, note_velocity: evidence, note_pedal:pedal_evidence },
        }
    }
    pub(crate) fn validate(&self, mode: &str) -> Result<(), String> {
        if self.velocity_difference.is_none()
            && self.note_velocity_difference.is_none()
            && self.note_pedal_mismatch_count.is_none()
            && self.contour_percent.is_none()
            && self.pedal_offset_ms.is_none()
        {
            return Err("请选择力度或踏板要求，或关闭专项要求".into());
        }
        if self
            .velocity_difference
            .is_some_and(|v| !(1..=64).contains(&v))
            || self.note_velocity_difference.is_some_and(|v| !(1..=64).contains(&v))
            || self.note_pedal_mismatch_count.is_some_and(|v|v>8)
            || self
                .contour_percent
                .is_some_and(|v| !(50..=100).contains(&v))
            || self
                .pedal_offset_ms
                .is_some_and(|v| !(10..=500).contains(&v))
        {
            return Err("力度平均差与逐音力度差需 1–64，逐音踏板错配数需 0–8，起伏一致率需 50–100%，踏板偏移需 10–500 ms".into());
        }
        if mode == "wait" && self.pedal_offset_ms.is_some() {
            return Err("等音不评价踏板时机，请使用连续或完整演奏".into());
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn assess_with_note_velocity(&self, e: ExpressionSummary, note_velocity: Option<NoteVelocityEvidence>) -> Result<(), String> {
        self.assess_with_note_evidence(e,note_velocity,None)
    }
    pub(crate) fn assess_with_note_evidence(&self,e:ExpressionSummary,note_velocity:Option<NoteVelocityEvidence>,note_pedal:Option<NotePedalEvidence>)->Result<(),String>{
        if let Some(limit)=self.note_pedal_mismatch_count {
            let evidence=note_pedal.ok_or("没有可核对的逐音踏板记录，逐音踏板目标未达标")?;
            if evidence.reference_notes<4||evidence.matched_notes<4{return Err(format!("逐音踏板需要至少 4 个确切配对音：当前 {} / 4 个",evidence.matched_notes))}
            if evidence.missing_notes>0||evidence.matched_notes!=evidence.reference_notes{return Err(format!("逐音踏板有 {} 个参考音未确切配对，漏音不能达标",evidence.missing_notes))}
            if evidence.unknown_state_notes>0||evidence.paired_state_notes!=evidence.reference_notes{return Err(format!("逐音踏板有 {} 个音缺少起音前的参考或实弹 CC64 状态，不能推断为松开",evidence.unknown_state_notes))}
            if evidence.mismatch_count>usize::from(limit){return Err(format!("逐音踏板状态错配 {} 个，要求 ≤ {limit} 个",evidence.mismatch_count))}
        }
        if let Some(limit) = self.note_velocity_difference {
            let evidence = note_velocity.ok_or("没有可核对的逐音演奏记录，逐音力度目标未达标")?;
            if evidence.reference_notes < 4 || evidence.matched_notes < 4 {
                return Err(format!("逐音力度需要至少 4 个确切配对音：当前 {} / 4 个", evidence.matched_notes));
            }
            if evidence.missing_notes > 0 || evidence.matched_notes != evidence.reference_notes {
                return Err(format!("逐音力度有 {} 个参考音未确切配对，漏音不能达标", evidence.missing_notes));
            }
            let largest = evidence.max_abs_difference.ok_or("没有可核对的逐音力度差，逐音力度目标未达标")?;
            if largest > limit {
                return Err(format!("逐音力度差未达标：最大差 {largest}，要求每音 ≤ {limit}"));
            }
        }
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
    pub(crate) fn current_song_expression_goal(&self) -> Option<ExpressionGoal> {
        self.file.as_ref().and_then(|file| self.preferences.song_expression_goals.get(&file.content_id)).cloned()
    }
    pub(crate) fn effective_expression_goal(&self) -> Option<ExpressionGoal> {
        if let Some(active) = &self.routine { return active.item.goal.expression.clone(); }
        if self.ladder.as_ref().is_some_and(|ladder| ladder.active) { return None; }
        self.current_song_expression_goal()
    }
    pub(crate) fn set_song_expression_goal(&mut self, goal: Option<ExpressionGoal>) -> Result<serde_json::Value, String> {
        let content_id = self.file.as_ref().ok_or("请先选择曲目")?.content_id.clone();
        if self.routine.is_some() || self.ladder.as_ref().is_some_and(|l| l.active) {
            return Err("请先结束练习计划或速度阶梯，再修改曲目专项要求".into());
        }
        let empty_attempt = self.session_clock == std::time::Duration::ZERO
            && self.matcher.snapshot().matched_notes + self.matcher.snapshot().wrong_notes + self.matcher.snapshot().missed_notes == 0;
        let editable = matches!(self.state.status.as_str(), "ready" | "idle")
            || self.state.status == "finished" && self.state.saved
            || self.state.status == "paused" && empty_attempt && !self.expression_attempt_started;
        if self.recorder.is_recording() || !editable {
            return Err("请先完成或重新开始当前练习，再修改曲目专项要求".into());
        }
        if let Some(g) = &goal { self.validate_current_expression_source(g)?; }
        let previous = self.preferences.song_expression_goals.get(&content_id).cloned();
        if let Some(g) = &goal { self.preferences.song_expression_goals.insert(content_id.clone(), g.clone()); }
        else { self.preferences.song_expression_goals.remove(&content_id); }
        if let Err(e) = self.preferences.save(&self.preferences_path) {
            if let Some(old) = previous { self.preferences.song_expression_goals.insert(content_id, old); }
            else { self.preferences.song_expression_goals.remove(&content_id); }
            return Err(e);
        }
        // The finished summary belongs to the previous frozen target; its history retains that target.
        self.active_expression_goal = None;
        Ok(serde_json::json!({"contentId": content_id, "goal": goal}))
    }
    pub(crate) fn validate_current_expression_source(&mut self, goal: &ExpressionGoal) -> Result<(), String> {
        goal.validate(&self.state.mode)?;
        if self.state.mode == "listen" { return Err("听曲模式不保存演奏成绩，请选择等音、连续或完整演奏".into()); }
        let range = self.state.passage.as_ref().map_or((0., self.state.duration), |p| (p.start,p.end));
        if range.0 < 0. || range.1 <= range.0 || !range.1.is_finite() { return Err("当前练习范围无效".into()); }
        self.inspect_expression_reference(goal, std::time::Duration::from_secs_f64(range.0), std::time::Duration::from_secs_f64(range.1))
    }
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
        let result = self.inspect_expression_reference(goal, start, end);
        self.config = old;
        self.state.hands = hands;
        result
    }
    fn inspect_expression_reference(&self, goal: &ExpressionGoal, start: std::time::Duration, end: std::time::Duration) -> Result<(), String> {
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
        if (goal.velocity_difference.is_some() || goal.note_velocity_difference.is_some()) && count < 4 {
            return Err("所选练习范围的参考力度不足 4 个音符".into());
        }
        if goal.note_pedal_mismatch_count.is_some(){
            if count<4{return Err("逐音踏板目标至少需要 4 个参考音".into())}
            let pedal_events=self.events.iter().filter(|event|self.selected(event.track_id))
                .filter(|event|matches!(event.message,midi_file::midly::MidiMessage::Controller{controller,..} if controller.as_int()==64))
                .map(|event|event.timestamp).collect::<Vec<_>>();
            let unknown=self.notes.iter().filter(|n|n.start>=start&&n.start<end&&n.velocity>0
                &&self.note_player(n.track_id,n.index)==neothesia_core::song_config::PlayerConfig::Human)
                .filter(|n|!pedal_events.iter().any(|event|*event<n.start)).count();
            if unknown>0{return Err(format!("{unknown} 个参考音起音前没有明确 CC64 状态，请选择包含踏板记录的曲目或范围"))}
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
    fn per_note_pedal_requires_complete_explicit_states_and_mismatch_limit(){
        let goal=ExpressionGoal{note_pedal_mismatch_count:Some(0),..Default::default()};
        assert!(goal.validate("wait").is_ok());
        assert!(ExpressionGoal{note_pedal_mismatch_count:Some(9),..Default::default()}.validate("flow").is_err());
        assert!(goal.assess_with_note_evidence(ExpressionSummary::default(),None,None).unwrap_err().contains("逐音踏板记录"));
        let complete=NotePedalEvidence{reference_notes:4,matched_notes:4,paired_state_notes:4,missing_notes:0,unknown_state_notes:0,mismatch_count:0};
        assert!(goal.assess_with_note_evidence(ExpressionSummary::default(),None,Some(complete)).is_ok());
        let missing=NotePedalEvidence{reference_notes:5,missing_notes:1,..complete};
        assert!(goal.assess_with_note_evidence(ExpressionSummary::default(),None,Some(missing)).unwrap_err().contains("漏音"));
        let unknown=NotePedalEvidence{paired_state_notes:3,unknown_state_notes:1,..complete};
        assert!(goal.assess_with_note_evidence(ExpressionSummary::default(),None,Some(unknown)).unwrap_err().contains("缺少起音前"));
        let wrong=NotePedalEvidence{mismatch_count:1,..complete};
        assert!(goal.assess_with_note_evidence(ExpressionSummary::default(),None,Some(wrong)).unwrap_err().contains("错配"));
        let result=goal.result_with_note_evidence(ExpressionSummary::default(),None,Some(complete));
        assert!(result.passed);assert_eq!(result.note_pedal,Some(complete));
    }
    #[test]
    fn per_note_velocity_requires_four_exact_pairs_no_misses_and_every_note_within_limit() {
        let goal=ExpressionGoal{note_velocity_difference:Some(10),..Default::default()};
        assert!(goal.validate("wait").is_ok());
        assert!(goal.assess_with_note_velocity(ExpressionSummary::default(),None).unwrap_err().contains("逐音演奏记录"));
        let evidence=NoteVelocityEvidence{reference_notes:3,matched_notes:3,missing_notes:0,max_abs_difference:Some(1)};
        assert!(goal.assess_with_note_velocity(ExpressionSummary::default(),Some(evidence)).unwrap_err().contains("至少 4"));
        let evidence=NoteVelocityEvidence{reference_notes:5,matched_notes:4,missing_notes:1,max_abs_difference:Some(2)};
        assert!(goal.assess_with_note_velocity(ExpressionSummary::default(),Some(evidence)).unwrap_err().contains("漏音"));
        let evidence=NoteVelocityEvidence{reference_notes:4,matched_notes:4,missing_notes:0,max_abs_difference:Some(11)};
        assert!(goal.assess_with_note_velocity(ExpressionSummary::default(),Some(evidence)).unwrap_err().contains("最大差 11"));
        let evidence=NoteVelocityEvidence{max_abs_difference:Some(10),..evidence};
        let result=goal.result_with_note_velocity(ExpressionSummary::default(),Some(evidence));
        assert!(result.passed);
        assert_eq!(result.note_velocity,Some(evidence));
        assert!(serde_json::from_str::<ExpressionGoal>(r#"{"velocityDifference":12}"#).unwrap().note_velocity_difference.is_none());
    }
    #[test]
    fn thresholds_samples_and_pedal_correspondence() {
        let mut e = ExpressionSummary::default();
        let g = ExpressionGoal {
            velocity_difference: Some(12),
            note_velocity_difference: None,
            note_pedal_mismatch_count: None,
            contour_percent: Some(80),
            pedal_offset_ms: Some(80),
        };
        assert!(g.validate("wait").is_err());
        assert!(g.validate("flow").is_ok());
        assert!(g.assess_with_note_velocity(e,None).unwrap_err().contains("样本不足"));
        e.velocity.matched_samples = 8;
        e.velocity.mean_abs_difference = Some(20);
        assert!(g.assess_with_note_velocity(e,None).unwrap_err().contains("平均差未达标"));
        e.velocity.mean_abs_difference = Some(12);
        e.velocity.contour_steps = 6;
        e.velocity.contour_aligned = 4;
        assert!(g.assess_with_note_velocity(e,None).unwrap_err().contains("起伏未达标"));
        e.velocity.contour_aligned = 5;
        assert!(g.assess_with_note_velocity(e,None).unwrap_err().contains("参考曲目"));
        e.pedal.target_present = true;
        e.pedal.target_transitions = 4;
        e.pedal.user_transitions = 3;
        assert!(g.assess_with_note_velocity(e,None).unwrap_err().contains("完整对应"));
        e.pedal.user_transitions = 4;
        e.pedal.timing_samples = 4;
        e.pedal.median_offset_ms = Some(-90);
        e.pedal.median_deviation_ms = Some(20);
        assert!(g.assess_with_note_velocity(e,None).unwrap_err().contains("偏移未达标"));
        e.pedal.median_offset_ms = Some(-80);
        assert!(g.assess_with_note_velocity(e,None).is_ok());
        e.pedal.median_deviation_ms = Some(121);
        assert!(g.assess_with_note_velocity(e,None).is_err());
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
        assert_eq!(backup.version(), 12);
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
    fn note_pedal_source_requires_prior_reference_cc64_for_every_target(){
        use midi_file::midly::{Format,Header,MidiMessage,Smf,Timing,TrackEvent,TrackEventKind,MetaMessage};
        let data=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../work")
            .join(format!("note-pedal-source-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut p=Player::new(data,std::path::PathBuf::new(),true);
        p.command(Command::Generate{spec:Default::default()}).unwrap();
        let goal=ExpressionGoal{note_pedal_mismatch_count:Some(0),..Default::default()};
        assert!(p.command(Command::SetSongExpressionGoal{goal:Some(goal.clone())}).unwrap_err().contains("CC64"));
        let mut events=vec![(0u32,TrackEventKind::Midi{channel:0.into(),message:MidiMessage::Controller{controller:64.into(),value:127.into()}})];
        for i in 1..=5u32{
            events.push((i*240,TrackEventKind::Midi{channel:0.into(),message:MidiMessage::NoteOn{key:60.into(),vel:80.into()}}));
            events.push((i*240+120,TrackEventKind::Midi{channel:0.into(),message:MidiMessage::NoteOff{key:60.into(),vel:0.into()}}));
        }
        events.push((1500,TrackEventKind::Meta(MetaMessage::EndOfTrack)));
        events.sort_by_key(|event|event.0);
        let mut previous=0;
        let track=events.into_iter().map(|(at,kind)|{let delta=at-previous;previous=at;TrackEvent{delta:delta.into(),kind}}).collect();
        let smf=Smf{header:Header{format:Format::SingleTrack,timing:Timing::Metrical(480.into())},tracks:vec![track]};
        let mut bytes=vec![];smf.write_std(&mut bytes).unwrap();
        p.command(Command::ImportMidi{bytes,name:"逐音踏板.mid".into()}).unwrap();
        let track=p.notes[0].track_id;
        p.command(Command::Track{id:track,mode:"human".into(),part:"right".into(),visible:true}).unwrap();
        p.command(Command::Mode{value:"flow".into()}).unwrap();
        p.command(Command::SetSongExpressionGoal{goal:Some(goal.clone())}).unwrap();
        assert_eq!(p.current_song_expression_goal(),Some(goal));
    }
    #[test]
    fn song_goal_persists_isolated_and_frozen_for_saved_attempt() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work/song-expression-unit")
            .join(format!("{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut p = Player::new(data.clone(), std::path::PathBuf::new(), true);
        p.command(Command::Generate { spec: Default::default() }).unwrap();
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let goal = ExpressionGoal { velocity_difference: Some(20), note_velocity_difference:Some(10), ..Default::default() };
        p.command(Command::SetSongExpressionGoal { goal: Some(goal.clone()) }).unwrap();
        assert_eq!(p.snapshot().song_expression_goal, Some(goal.clone()));
        assert_eq!(p.preferences.song_expression_goals.get(&cid), Some(&goal));
        p.command(Command::Generate { spec: neothesia_core::exercise::ExerciseSpec { tonic: 2, ..Default::default() } }).unwrap();
        assert_ne!(p.file.as_ref().unwrap().content_id, cid);
        assert_eq!(p.snapshot().song_expression_goal, None);
        p.command(Command::Generate { spec: Default::default() }).unwrap();
        assert_eq!(p.snapshot().song_expression_goal, Some(goal.clone()));
        let backup = p.practice_backup_snapshot(&Default::default()).unwrap();
        let backup: crate::practice_backup::Backup = serde_json::from_value(backup).unwrap();
        assert_eq!(backup.version(), 14);
        assert_eq!(crate::practice_backup::Backup::decode(&backup.encode().unwrap()).unwrap().song_expression_goals.get(&cid), Some(&goal));
        let mut reopened = Player::new(data, std::path::PathBuf::new(), true);
        reopened.restore();
        assert_eq!(reopened.current_song_expression_goal(), Some(goal.clone()));
        reopened.command(Command::CountIn { bars: 0 }).unwrap();
        reopened.command(Command::Play).unwrap();
        assert_eq!(reopened.active_expression_goal, Some(goal.clone()));
        assert!(reopened.command(Command::SetSongExpressionGoal { goal: None }).is_err());
        let context = reopened.history_context();
        let frozen=context.expression.unwrap();
        assert_eq!(frozen.velocity_difference, Some(20));
        assert_eq!(frozen.note_velocity_difference, Some(10));
        reopened.command(Command::Pause).unwrap();
        assert!(reopened.command(Command::SetSongExpressionGoal { goal: None }).is_err());
        reopened.command(Command::Restart).unwrap();
        reopened.command(Command::SetSongExpressionGoal { goal: None }).unwrap();
        assert_eq!(reopened.snapshot().song_expression_goal, None);
        assert_eq!(reopened.preferences.song_expression_goals.get(&cid), None);
    }
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
