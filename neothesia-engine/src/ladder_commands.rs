use super::*;
use neothesia_core::speed_ladder::{SpeedLadderPlan, SpeedLadderProgress};
use preferences::{LadderPreset, LadderRun, SessionSettings};
use serde_json::{Value, json};
fn key(content_id: &str, id: &str) -> String {
    format!("{content_id}:{id}")
}
impl Player {
    pub(super) fn ladder_presets(&self) -> Result<Value, String> {
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        let presets = self
            .preferences
            .ladder_presets
            .get(&file.content_id)
            .cloned()
            .unwrap_or_default();
        let runs = presets
            .iter()
            .filter_map(|p| {
                self.preferences
                    .ladder_runs
                    .get(&key(&file.content_id, &p.id))
            })
            .collect::<Vec<_>>();
        Ok(json!({"presets":presets,"runs":runs,"current":self.ladder}))
    }
    pub(super) fn save_ladder(
        &mut self,
        id: Option<String>,
        name: &str,
        start: usize,
        end: usize,
        plan: SpeedLadderPlan,
    ) -> Result<Value, String> {
        if self.recorder.is_recording() || self.ladder.as_ref().is_some_and(|r| r.active) {
            return Err("请先停止录音或结束速度阶梯，再保存方案".into());
        }
        plan.validate()?;
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        if name.trim().is_empty() || name.chars().count() > 80 {
            return Err("方案名称需 1～80 个字".into());
        }
        if start == 0 || start > end || end > file.musical_time.measures.len() {
            return Err("小节范围无效".into());
        }
        if !["wait", "flow"].contains(&self.state.mode.as_str()) {
            return Err("速度阶梯请使用等音或连续模式".into());
        }
        if self.state.mode == "wait" && plan.on_time_percent.is_some() {
            return Err("等音不评价节奏，请关闭准时率门槛或改用连续模式".into());
        }
        let begin = file
            .tempo_track
            .pulses_to_duration(file.musical_time.measures[start - 1].start_tick)
            .as_secs_f64();
        let finish = file
            .tempo_track
            .pulses_to_duration(file.musical_time.measures[end - 1].end_tick)
            .as_secs_f64();
        if !self.notes.iter().any(|n| {
            self.note_player(n.track_id, n.index) == PlayerConfig::Human
                && n.start.as_secs_f64() >= begin
                && n.start.as_secs_f64() < finish
        }) {
            return Err("选段没有自己弹的目标音符，请确认声部和小节范围".into());
        }
        let content_id = file.content_id.clone();
        let id = id.unwrap_or_else(|| {
            format!(
                "ladder-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )
        });
        let preset = LadderPreset {
            id: id.clone(),
            name: name.trim().into(),
            start,
            end,
            grid: self.grid_signature(),
            scope: self.practice_scope(),
            exercise_spec: self.exercise,
            settings: SessionSettings {
                tracks: self.config.practice_track_setup(),
                parts: self
                    .config
                    .tracks
                    .iter()
                    .map(|t| (t.track_id, t.practice_part))
                    .collect(),
                mode: Some(self.state.mode.clone()),
                hands: self.state.hands.clone(),
                count_in: self.state.count_in,
                metronome: self.state.metronome,
                latency: self.state.latency,
                rounds: 0,
                adaptive: false,
            },
            plan,
        };
        let rows = self
            .preferences
            .ladder_presets
            .entry(content_id.clone())
            .or_default();
        let mut preserve = false;
        if let Some(old) = rows.iter_mut().find(|p| p.id == id) {
            preserve = old.start == preset.start
                && old.end == preset.end
                && old.grid == preset.grid
                && old.scope == preset.scope
                && old.exercise_spec == preset.exercise_spec
                && old.plan == preset.plan
                && serde_json::to_value(&old.settings).ok()
                    == serde_json::to_value(&preset.settings).ok();
            *old = preset.clone();
        } else {
            if rows.len() >= 200 {
                return Err("每首曲目最多保存 200 个速度阶梯方案".into());
            }
            rows.push(preset.clone());
        }
        if preserve {
            if let Some(run) = self.preferences.ladder_runs.get_mut(&key(&content_id, &id)) {
                run.preset = preset.clone();
            }
            if let Some(run) = &mut self.ladder {
                if run.preset.id == id {
                    run.preset = preset;
                }
            }
        } else {
            // Changed conditions cannot inherit successes earned under the previous rules.
            self.preferences.ladder_runs.remove(&key(&content_id, &id));
            if self.ladder.as_ref().is_some_and(|r| r.preset.id == id) {
                self.ladder = None;
            }
        }
        self.persist()?;
        Ok(json!({"id":id}))
    }
    pub(super) fn remove_ladder(&mut self, id: &str) -> Result<Value, String> {
        if self
            .ladder
            .as_ref()
            .is_some_and(|r| r.active && r.preset.id == id)
        {
            return Err("请先结束这个阶梯再移除方案".into());
        }
        let content_id = self.file.as_ref().ok_or("请先选择曲目")?.content_id.clone();
        if let Some(rows) = self.preferences.ladder_presets.get_mut(&content_id) {
            rows.retain(|p| p.id != id);
        }
        self.preferences.ladder_runs.remove(&key(&content_id, id));
        if self.ladder.as_ref().is_some_and(|r| r.preset.id == id) {
            self.ladder = None;
        }
        self.persist()?;
        Ok(json!({"ok":true}))
    }
    pub(super) fn start_ladder(&mut self, id: &str, resume: bool) -> Result<Value, String> {
        if self.recorder.is_recording() {
            return Err("请先停止录音".into());
        }
        if self.ladder.as_ref().is_some_and(|r| r.active) {
            return Err("请先结束正在进行的速度阶梯".into());
        }
        if is_recital_mode(&self.state.mode)
            && ["playing", "countIn", "paused"].contains(&self.state.status.as_str())
        {
            return Err("请先重新开始或完成演奏，再打开速度阶梯".into());
        }
        let content_id = self.file.as_ref().ok_or("请先选择曲目")?.content_id.clone();
        let preset = self
            .preferences
            .ladder_presets
            .get(&content_id)
            .and_then(|p| p.iter().find(|p| p.id == id))
            .cloned()
            .ok_or("方案不属于当前曲目")?;
        preset.plan.validate()?;
        let checkpoint = if resume {
            Some(
                self.preferences
                    .ladder_runs
                    .get(&key(&content_id, id))
                    .cloned()
                    .ok_or("此方案没有保存进度")?,
            )
        } else {
            None
        };
        if checkpoint.as_ref().is_some_and(|r| r.progress.completed) {
            return Err("此方案已经完成，请选择从起始速度重练".into());
        }
        if let Some(spec) = preset.exercise_spec {
            if self.exercise != Some(spec) {
                self.command(Command::Generate { spec })?;
            }
        }
        if preset.grid != self.grid_signature() {
            return Err("小节网格已改变，请核对范围并重新保存方案".into());
        }
        if !["wait", "flow"].contains(&preset.settings.mode.as_deref().unwrap_or("wait")) {
            return Err("方案模式无效".into());
        }
        let previous = self.config.clone();
        let previous_hands = self.state.hands.clone();
        self.config.apply_practice_setup(&SongPracticeSetup {
            tracks: preset.settings.tracks.clone(),
            ..Default::default()
        });
        for track in &mut self.config.tracks {
            if let Some(part) = preset.settings.parts.get(&track.track_id) {
                track.practice_part = *part;
            }
        }
        self.state.hands = preset.settings.hands.clone();
        if self.practice_scope() != preset.scope {
            self.config = previous;
            self.state.hands = previous_hands;
            return Err("评分范围或逐音分手已改变，请确认声部并重新保存方案".into());
        }
        self.state.mode = preset
            .settings
            .mode
            .clone()
            .unwrap_or_else(|| "wait".into());
        self.state.wait = self.state.mode == "wait";
        self.state.count_in = preset.settings.count_in;
        self.state.metronome = preset.settings.metronome;
        self.state.latency = preset.settings.latency;
        self.state.rounds = 0;
        self.state.adaptive = false;
        self.state.status = "ready".into();
        let mut progress = checkpoint.map_or_else(SpeedLadderProgress::default, |r| r.progress);
        if resume {
            progress.resume();
        }
        self.state.speed = f64::from(progress.speed(&preset.plan)) / 100.;
        self.command(Command::MeasureLoop {
            start: preset.start,
            end: preset.end,
            enabled: true,
        })?;
        self.ladder = Some(LadderRun {
            preset,
            progress,
            active: true,
        });
        self.persist()?;
        self.ladder_presets()
    }
    pub(super) fn stop_ladder(&mut self) -> Result<Value, String> {
        if let Some(run) = &mut self.ladder {
            run.active = false;
            run.progress.last_result = "已结束，可继续保存进度".into();
        }
        self.panic();
        self.state.status = "paused".into();
        self.persist()?;
        self.ladder_presets()
    }
}
