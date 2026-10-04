use super::*;
use crate::preferences::{ExercisePreset, LadderPreset, LadderRun, PassagePreset};
use crate::routine_commands::{PracticeRoutine, RoutineRun, RoutineTarget};
use neothesia_core::practice_history::SongPracticeHistory;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use std::path::Path;
const MAX_FILE: usize = 256_000_000;
const MAX_DATA: usize = 512_000_000;
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub plans: bool,
    pub records: bool,
    pub presets: bool,
    #[serde(default)]
    pub days: u32,
}
impl Default for Selection {
    fn default() -> Self {
        Self {
            plans: true,
            records: true,
            presets: true,
            days: 0,
        }
    }
}
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata_templates: BTreeMap<String, crate::metadata_templates::Rules>,
    pub routines: Vec<PracticeRoutine>,
    pub runs: BTreeMap<String, RoutineRun>,
    pub passages: BTreeMap<String, Vec<PassagePreset>>,
    pub ladders: BTreeMap<String, Vec<LadderPreset>>,
    pub ladder_runs: BTreeMap<String, LadderRun>,
    pub exercises: Vec<ExercisePreset>,
    #[serde(default)]
    pub track_appearances:
        BTreeMap<String, BTreeMap<usize, crate::track_appearance::TrackAppearance>>,
    #[serde(default)]
    pub track_sounds: BTreeMap<String, BTreeMap<usize, crate::track_sound::TrackSound>>,
    pub history: BTreeMap<String, SongPracticeHistory>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub performances: BTreeMap<String, BTreeMap<String, crate::performance_archive::Archive>>,
    #[serde(default)]
    pub learning: BTreeMap<String, crate::song_learning::Learning>,
}
fn identity(s: &str) -> Result<(), String> {
    if s.is_empty() || s.len() > 160 || s.chars().any(char::is_control) {
        Err("备份中存在无效身份编号".into())
    } else {
        Ok(())
    }
}
fn text(s: &str, max: usize) -> Result<(), String> {
    if s.len() > max {
        Err("备份文本超过容量限制".into())
    } else {
        Ok(())
    }
}
fn settings(s: &preferences::SessionSettings) -> Result<(), String> {
    if s.tracks.len() > 1000
        || s.parts.len() > 1000
        || s.count_in > 4
        || s.latency.unsigned_abs() > 1000
        || s.rounds > 10000
        || !matches!(s.hands.as_str(), "both" | "left" | "right" | "custom")
        || s.mode
            .as_deref()
            .is_some_and(|m| !matches!(m, "wait" | "flow" | "listen" | "recital" | "memory"))
    {
        return Err("备份的练习条件无效".into());
    }
    let mut ids = BTreeSet::new();
    if s.tracks.iter().any(|t| !ids.insert(t.track_id)) {
        return Err("备份音轨配置重复".into());
    }
    Ok(())
}
fn speed(v: f64) -> Result<(), String> {
    if !v.is_finite() || !(0.25..=2.).contains(&v) {
        Err("备份练习速度无效".into())
    } else {
        Ok(())
    }
}
fn range(a: usize, b: usize) -> Result<(), String> {
    if a == 0 || a > b || b > 1_000_000 {
        Err("备份小节范围无效".into())
    } else {
        Ok(())
    }
}
fn ladder(p: &LadderPreset) -> Result<(), String> {
    identity(&p.id)?;
    text(&p.name, 320)?;
    text(&p.grid, 2048)?;
    text(&p.scope, 2048)?;
    range(p.start, p.end)?;
    settings(&p.settings)?;
    p.plan.validate()?;
    if p.settings.mode.as_deref() == Some("wait") && p.plan.on_time_percent.is_some() {
        return Err("等音阶梯不能要求准时率".into());
    }
    Ok(())
}
fn routine(r: &PracticeRoutine) -> Result<(), String> {
    if let Some(s) = &r.schedule {
        s.validate()?;
    }
    identity(&r.id)?;
    text(&r.name, 320)?;
    text(&r.notes, 8192)?;
    if r.items.len() > 500 {
        return Err("备份计划项目过多".into());
    }
    let mut ids = BTreeSet::new();
    for i in &r.items {
        identity(&i.id)?;
        identity(&i.content_id)?;
        if !ids.insert(&i.id) {
            return Err("备份计划项目编号重复".into());
        }
        text(&i.title, 320)?;
        text(&i.song_title, 2048)?;
        text(&i.notes, 8192)?;
        text(&i.grid, 2048)?;
        text(&i.scope, 2048)?;
        speed(i.speed)?;
        settings(&i.settings)?;
        i.goal
            .validate(i.settings.mode.as_deref().unwrap_or("wait"))?;
        match &i.target {
            RoutineTarget::Whole => {}
            RoutineTarget::PrecisePassage {range} => {range.validate()?;if range.content_id!=i.content_id || range.grid!=i.grid{return Err("拍内计划归属无效".into())}},
            RoutineTarget::ScorePassage { range } => range.validate()?,
            RoutineTarget::Passage { start, end } => range(*start, *end)?,
            RoutineTarget::Ladder { preset } => {
                if i.goal.expression.is_some() {
                    return Err("阶梯项目不支持力度/踏板要求".into());
                }
                ladder(preset)?;
            }
        };
        if let Some(spec) = i.exercise {
            let p = ExercisePlan::generate(spec, &KeyboardRange::new(21..=108))
                .map_err(|e| e.to_string())?;
            if p.practice_id() != i.content_id {
                return Err("备份生成练习身份不匹配".into());
            }
        }
    }
    Ok(())
}
fn finite_numbers(v: &Value) -> Result<(), String> {
    fn check(v: &Value, key: &str) -> Result<(), String> {
        match v {
            Value::Array(a) => {
                if a.len() > 1_000_000 {
                    return Err("备份数组过大".into());
                }
                for v in a {
                    check(v, key)?;
                }
            }
            Value::Object(m) => {
                for (k, v) in m {
                    check(v, k)?;
                }
            }
            Value::Number(n) => {
                let max = if matches!(
                    key,
                    "createdAt"
                        | "recorded_at_unix_ms"
                        | "last_used_unix_ms"
                        | "updated_at_unix_ms"
                ) {
                    10_000_000_000_000_000
                } else if matches!(key, "updatedAt" | "at") {
                    10_000_000_000
                } else if matches!(key, "startTick" | "endTick") {
                    0x0fff_ffff
                } else {
                    100_000_000
                };
                if n.as_u64().is_some_and(|x| x > max) || n.as_f64().is_some_and(|x| !x.is_finite())
                {
                    return Err("备份数值无效".into());
                }
            }
            _ => {}
        }
        Ok(())
    }
    check(v, "")
}

impl Backup {
    pub fn version(&self) -> u8 {
        let trimmed=|r:&crate::score_ranges::ScoreRange|r.start_trim>0||r.end_trim>0;
        if self.passages.values().flatten().any(|p|p.score_range.as_ref().is_some_and(trimmed)) || self.routines.iter().chain(self.runs.values().map(|r|&r.routine)).any(|r|r.items.iter().any(|i|matches!(&i.target,RoutineTarget::ScorePassage{range} if trimmed(range)))){return 11;}
        if self.passages.values().flatten().any(|p|p.precise_range.is_some()) || self.routines.iter().chain(self.runs.values().map(|r|&r.routine)).any(|r|r.items.iter().any(|i|matches!(i.target,RoutineTarget::PrecisePassage{..}))){return 10;}
        if self.history.values().any(|h|h.sessions.iter().any(|s|!s.replay_clips.is_empty())) {return 9;}
        if self.performances.values().flat_map(|rows|rows.values()).any(|a|a.has_measures()){return 8;}
        if self
            .performances
            .values()
            .flat_map(|rows| rows.values())
            .any(|a| a.has_judgments())
        {
            return 7;
        }
        if !self.performances.is_empty() {
            return 6;
        }
        if self
            .passages
            .values()
            .flatten()
            .any(|p| p.score_range.is_some())
            || self
                .routines
                .iter()
                .chain(self.runs.values().map(|r| &r.routine))
                .any(|r| {
                    r.items
                        .iter()
                        .any(|i| matches!(i.target, RoutineTarget::ScorePassage { .. }))
                })
        {
            return 5;
        }
        if !self.metadata_templates.is_empty() {
            return 4;
        }
        if !self.learning.is_empty() {
            return 3;
        }
        if self
            .routines
            .iter()
            .chain(self.runs.values().map(|r| &r.routine))
            .any(|r| r.items.iter().any(|i| i.goal.expression.is_some()))
            || self.history.values().any(|h| {
                h.sessions.iter().any(|s| {
                    s.context.as_ref().is_some_and(|c| {
                        c.pedal_latency_ms.is_some_and(|v| v != 0)
                            || c.routine.as_ref().is_some_and(|r| r.expression.is_some())
                    })
                })
            })
        {
            2
        } else {
            1
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        for (cid,h) in &self.history {for s in &h.sessions {
            let duration=self.performances.get(cid).and_then(|rows|rows.get(&session_key(s))).map(|a|a.duration);
            neothesia_core::practice_history::validate_replay_clips(&s.replay_clips,duration)?;
        }}
        if self.performances.len() > 10000 {
            return Err("备份回放曲目过多".into());
        }
        for (cid, rows) in &self.performances {
            if rows.len() > 200 {
                return Err("备份每曲回放超过 200 次".into());
            }
            let history = self.history.get(cid).ok_or("备份回放缺少对应成绩")?;
            for (id, a) in rows {
                if !history.sessions.iter().any(|s| session_key(s) == *id) {
                    return Err("备份回放缺少对应成绩".into());
                }
                a.validate(cid, id)?;
                a.validate_summary(
                    history
                        .sessions
                        .iter()
                        .find(|s| session_key(s) == *id)
                        .unwrap()
                        .summary
                        .overall,
                )?;
            }
        }

        if self.metadata_templates.len() > 50 {
            return Err("备份资料模板超过 50 套".into());
        }
        for (name, rules) in &self.metadata_templates {
            if name != name.trim()
                || name.trim().is_empty()
                || name.chars().count() > 80
                || name.chars().any(char::is_control)
            {
                return Err("备份资料模板名称无效".into());
            }
            crate::metadata_templates::validate(rules)?;
        }
        if self.learning.len() > 10000 {
            return Err("备份学习曲目数量超过上限".into());
        }
        for (cid, plan) in &self.learning {
            if cid.len() != 64 || !cid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err("备份学习曲目身份无效".into());
            }
            plan.validate()?;
        }

        if self.track_appearances.len() > 10000 {
            return Err("备份音轨名称与颜色过多".into());
        }
        for (cid, rows) in &self.track_appearances {
            identity(cid)?;
            if rows.len() > 1000 {
                return Err("备份音轨名称与颜色过多".into());
            }
            for (id, a) in rows {
                if *id > 100000 {
                    return Err("备份音轨编号无效".into());
                }
                a.validate()?;
            }
        }
        if self.track_sounds.len() > 10000 {
            return Err("备份音轨设置过多".into());
        }
        for (cid, rows) in &self.track_sounds {
            identity(cid)?;
            if rows.len() > 1000
                || rows.iter().any(|(id, s)| {
                    *id > 100000
                        || s.volume > 100
                        || s.pan.is_some_and(|p| p > 127)
                        || s.program.is_some_and(|p| p > 127)
                })
            {
                return Err("备份音轨声音设置无效".into());
            }
        }
        if self.routines.len() > 100
            || self.runs.len() > 10000
            || self.history.len() > 10000
            || self.exercises.len() > 1000
            || self.passages.len() > 10000
            || self.ladders.len() > 10000
            || self.ladder_runs.len() > 10000
        {
            return Err("备份条目超过容量限制".into());
        }
        let mut ids = BTreeSet::new();
        for r in &self.routines {
            routine(r)?;
            if !ids.insert(&r.id) {
                return Err("备份计划编号重复".into());
            }
        }
        for (k, r) in &self.runs {
            routine(&r.routine)?;
            if !routine_commands::day_valid(&r.day) || *k != format!("{}:{}", r.routine.id, r.day) {
                return Err("备份日期安排编号无效".into());
            }
            if r.progress.len() > 1000 {
                return Err("备份日期进度过多".into());
            }
            for p in r.progress.values() {
                if p.session_ids.len() > 500
                    || p.best_accuracy
                        .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
                    || p.completed && p.skipped
                {
                    return Err("备份日期进度无效".into());
                }
                text(&p.last_result, 8192)?;
            }
        }
        for (cid, rows) in &self.passages {
            identity(cid)?;
            if rows.len() > 200 {
                return Err("备份片段过多".into());
            }
            let mut ids = BTreeSet::new();
            for p in rows {
                identity(&p.id)?;
                if !ids.insert(&p.id) {
                    return Err("备份片段编号重复".into());
                }
                text(&p.name, 320)?;
                text(&p.notes, 8192)?;
                range(p.start, p.end)?;
                if let Some(r)=&p.precise_range {r.validate()?;if &r.content_id!=cid || p.score_range.is_some(){return Err("拍内段落归属无效".into())}}
                if let Some(range) = &p.score_range {
                    range.validate()?;
                }
                speed(p.speed)?;
                settings(&p.settings)?;
            }
        }
        for (cid, rows) in &self.ladders {
            identity(cid)?;
            if rows.len() > 200 {
                return Err("备份阶梯过多".into());
            }
            let mut ids = BTreeSet::new();
            for p in rows {
                ladder(p)?;
                if !ids.insert(&p.id) {
                    return Err("备份阶梯编号重复".into());
                }
            }
        }
        for r in self.ladder_runs.values() {
            ladder(&r.preset)?;
            if r.progress.stage >= r.preset.plan.speeds().len()
                || r.progress.success_streak > r.preset.plan.passes_required
                || r.progress.batch_rounds > r.preset.plan.attempt_limit
            {
                return Err("备份阶梯阶段无效".into());
            }
        }
        let mut exercise_ids = BTreeSet::new();
        for p in &self.exercises {
            identity(&p.id)?;
            if !exercise_ids.insert(&p.id) {
                return Err("备份技术方案编号重复".into());
            }
            text(&p.name, 320)?;
            ExercisePlan::generate(p.spec, &KeyboardRange::new(21..=108))
                .map_err(|e| e.to_string())?;
        }
        for (cid, h) in &self.history {
            identity(cid)?;
            text(&h.display_name, 2048)?;
            if h.sessions.len() > 200 {
                return Err("每曲备份最多 200 条保留记录".into());
            }
            let mut ids = BTreeSet::new();
            for s in &h.sessions {
                if let Some(a) = &s.annotation {
                    text(&a.note, 8192)?;
                    text(&a.teacher, 8192)?;
                    text(&a.next, 8192)?;
                }
                speed(f64::from(s.speed))?;
                let id = session_key(s);
                if !ids.insert(id) {
                    return Err("备份成绩编号重复".into());
                }
                if let Some(id) = &s.id {
                    identity(id)?;
                }
                if let PracticeSessionKind::Loop {
                    start_measure,
                    end_measure,
                } = s.kind
                {
                    range(start_measure, end_measure)?;
                }
                if s.mode.as_deref().is_some_and(|m| {
                    !matches!(m, "wait" | "flow" | "listen" | "recital" | "memory")
                }) {
                    return Err("备份成绩模式无效".into());
                }
                if s.playing_ms.is_some_and(|ms| ms > 9_000_000_000_000_000) {
                    return Err("备份练习用时超出有效范围".into());
                }
                if let Some(c) = &s.context {
                    if let Some(e) = c.routine.as_ref().and_then(|r| r.expression.as_ref()) {
                        crate::expression_goal::ExpressionGoal {
                            velocity_difference: e.velocity_difference,
                            contour_percent: e.contour_percent,
                            pedal_offset_ms: e.pedal_offset_ms,
                        }
                        .validate(s.mode.as_deref().unwrap_or("flow"))?;
                    }
                    if c.tracks.len() > 1000
                        || c.parts.len() > 1000
                        || c.latency.unsigned_abs() > 1000
                        || c.pedal_latency_ms.is_some_and(|v| v.unsigned_abs() > 1000)
                        || c.count_in > 4
                        || c.target_notes > 100_000_000
                        || c.judged_notes > 100_000_000
                    {
                        return Err("备份成绩条件无效".into());
                    }
                }
            }
        }
        finite_numbers(&serde_json::to_value(self).map_err(|e| e.to_string())?)
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let data = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if data.len() > MAX_DATA {
            return Err("备份解压内容超过 512 MB，请按日期减少成绩范围".into());
        }
        let header = json!({"format":"neothesia-practice","version":self.version(),"hash":blake3::hash(&data).to_hex().to_string(),"size":data.len()});
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, b) in [
            ("manifest.json", serde_json::to_vec(&header).unwrap()),
            ("practice.json", data),
        ] {
            zip.start_file(name, opts).map_err(|e| e.to_string())?;
            zip.write_all(&b).map_err(|e| e.to_string())?;
        }
        let bytes = zip.finish().map_err(|e| e.to_string())?.into_inner();
        if bytes.len() > MAX_FILE {
            return Err("备份超过 256 MB".into());
        }
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_FILE {
            return Err("练习备份为空或超过 256 MB".into());
        }
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| format!("练习备份格式无效：{e}"))?;
        if zip.len() != 2 {
            return Err("练习备份目录无效".into());
        }
        let mut files = BTreeMap::new();
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
            let name = f.name().to_owned();
            let max = if name == "manifest.json" {
                4096
            } else if name == "practice.json" {
                MAX_DATA
            } else {
                return Err("备份包含未知文件".into());
            };
            if f.is_dir() || f.size() > max as u64 || files.contains_key(&name) {
                return Err("备份文件重复或超过限制".into());
            }
            let mut b = vec![];
            f.by_ref()
                .take(max as u64 + 1)
                .read_to_end(&mut b)
                .map_err(|e| e.to_string())?;
            if b.len() > max {
                return Err("备份解压内容超过限制".into());
            }
            files.insert(name, b);
        }
        let header: Value =
            serde_json::from_slice(&files["manifest.json"]).map_err(|e| e.to_string())?;
        let data = &files["practice.json"];
        if header["format"] != "neothesia-practice"
            || !matches!(header["version"].as_u64(), Some(1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11))
            || header["size"].as_u64() != Some(data.len() as u64)
            || header["hash"].as_str() != Some(blake3::hash(data).to_hex().as_str())
        {
            return Err("练习备份校验失败或版本不支持".into());
        }
        let backup: Self = serde_json::from_slice(data).map_err(|e| e.to_string())?;
        backup.validate()?;
        if u64::from(backup.version()) > header["version"].as_u64().unwrap_or(0) {
            return Err("备份包含新的练习资料字段，请使用新版备份格式".into());
        }
        Ok(backup)
    }
    pub fn summary(&self) -> Value {
        json!({"replayClips":self.history.values().flat_map(|h|h.sessions.iter()).map(|s|s.replay_clips.len()).sum::<usize>(),"performances":self.performances.values().map(BTreeMap::len).sum::<usize>(),"performanceBytes":self.performances.values().flat_map(|rows|rows.values()).filter_map(|a|a.bytes().ok()).map(|b|b.len()).sum::<usize>(),"metadataTemplates":self.metadata_templates.len(),"metadataTemplateRows":self.metadata_templates.iter().map(|(name,rules)|json!({"name":name,"rules":rules})).collect::<Vec<_>>(),"learning":self.learning.len(),"createdAt":self.created_at,"plans":self.routines.iter().map(|r|json!({"id":r.id,"name":r.name,"notes":r.notes,"schedule":r.schedule,"items":r.items.len()})).collect::<Vec<_>>(),"days":self.runs.len(),"songs":self.history.len(),"records":self.history.values().map(|h|h.sessions.len()).sum::<usize>(),"passages":self.passages.values().map(Vec::len).sum::<usize>(),"ladders":self.ladders.values().map(Vec::len).sum::<usize>(),"exercises":self.exercises.len(),"trackSounds":self.track_sounds.values().map(BTreeMap::len).sum::<usize>(),"trackAppearances":self.track_appearances.values().map(BTreeMap::len).sum::<usize>()})
    }
}
fn session_key(s: &PracticeSession) -> String {
    s.stable_id()
}
fn collision_key(session:&PracticeSession)->String {
    let mut evidence=session.clone();evidence.annotation=None;evidence.replay_clips.clear();
    format!("import-{}",blake3::hash(&serde_json::to_vec(&evidence).unwrap()).to_hex())
}
fn merge_rows<T: Clone + Serialize>(
    local: &mut Vec<T>,
    incoming: &[T],
    id: impl Fn(&T) -> &str,
    replace: bool,
    max: usize,
) -> Result<usize, String> {
    let mut changed = 0;
    for r in incoming {
        if let Some(i) = local.iter().position(|x| id(x) == id(r)) {
            if replace
                && serde_json::to_value(&local[i]).unwrap() != serde_json::to_value(r).unwrap()
            {
                local[i] = r.clone();
                changed += 1;
            }
        } else {
            local.push(r.clone());
            changed += 1;
        }
    }
    if local.len() > max {
        return Err("合并后的条目超过容量限制，请先整理本地资料".into());
    }
    Ok(changed)
}
fn selected(mut b: Backup, s: &Selection) -> Backup {
    if !s.plans {
        b.routines.clear();
        b.runs.clear();
        b.learning.clear();
    }
    if !s.presets {
        b.metadata_templates.clear();
        b.passages.clear();
        b.ladders.clear();
        b.ladder_runs.clear();
        b.exercises.clear();
        b.track_sounds.clear();
        b.track_appearances.clear();
    }
    if !s.records {
        b.history.clear();
    } else if s.days > 0 {
        let cutoff = b.created_at.saturating_sub(u64::from(s.days) * 86_400_000);
        for h in b.history.values_mut() {
            h.sessions.retain(|v| v.recorded_at_unix_ms >= cutoff);
        }
        b.history.retain(|_, h| !h.sessions.is_empty());
    }
    b.performances.retain(|cid, rows| {
        let Some(h) = b.history.get(cid) else {
            return false;
        };
        rows.retain(|id, _| h.sessions.iter().any(|session| session_key(session) == *id));
        !rows.is_empty()
    });
    b
}
#[derive(Deserialize, Serialize)]
struct Journal {
    #[serde(default)]
    performances: BTreeMap<String, bool>,
    folder: String,
    prefs: bool,
    history: bool,
    #[serde(default)]
    workspace: Option<bool>,
    #[serde(default)]
    templates: Option<bool>,
    committed: bool,
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("practice-tmp");
    let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    f.write_all(bytes).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    drop(f);
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}
pub fn recover(data: &Path) -> Result<(), String> {
    let journal = data.join("practice-import-journal.json");
    if !journal.exists() {
        return Ok(());
    }
    let j: Journal = serde_json::from_slice(&std::fs::read(&journal).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if !j.folder.starts_with("restore-")
        || j.folder.len() > 120
        || !j
            .folder
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err("练习导入恢复目录无效".into());
    }
    if j.performances.keys().any(|name| {
        name.len() != 69
            || !name.ends_with(".json")
            || !name.as_bytes()[..64].iter().all(|b| b.is_ascii_hexdigit())
    }) {
        return Err("练习导入回放恢复路径无效".into());
    }
    if !j.committed {
        let old = data.join("practice-restore-points").join(&j.folder);
        for (name, exists) in [
            ("web-preferences.json", j.prefs),
            ("web-practice-history.ron", j.history),
        ]
        .into_iter()
        .chain(j.workspace.map(|exists| ("library-workspace.json", exists)))
        .chain(
            j.templates
                .map(|exists| ("metadata-templates.json", exists)),
        ) {
            if exists {
                atomic(
                    &data.join(name),
                    &std::fs::read(old.join(name)).map_err(|e| e.to_string())?,
                )?;
            } else if data.join(name).exists() {
                std::fs::remove_file(data.join(name)).map_err(|e| e.to_string())?;
            }
        }
        for (name, exists) in &j.performances {
            let path = data.join("performance-history").join(name);
            if *exists {
                std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
                atomic(
                    &path,
                    &std::fs::read(old.join("performance-history").join(name))
                        .map_err(|e| e.to_string())?,
                )?;
            } else if path.exists() {
                std::fs::remove_file(path).map_err(|e| e.to_string())?;
            }
        }
    }
    std::fs::remove_file(journal).map_err(|e| e.to_string())
}
fn commit(
    data: &Path,
    prefs: &Preferences,
    history: &PracticeHistoryStore,
    workspace: &library_workspace::Workspace,
    templates: Option<&[u8]>,
    performances: &BTreeMap<String, Vec<u8>>,
) -> Result<String, String> {
    recover(data)?;
    std::fs::create_dir_all(data).map_err(|e| e.to_string())?;
    let folder = format!(
        "restore-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let root = data.join("practice-restore-points").join(&folder);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let mut j = Journal {
        performances: BTreeMap::new(),
        folder: folder.clone(),
        prefs: false,
        history: false,
        workspace: Some(false),
        templates: templates.map(|_| false),
        committed: false,
    };
    for (name, exists) in [
        ("web-preferences.json", &mut j.prefs),
        ("web-practice-history.ron", &mut j.history),
        ("library-workspace.json", j.workspace.as_mut().unwrap()),
    ]
    .into_iter()
    .chain(
        j.templates
            .as_mut()
            .map(|exists| ("metadata-templates.json", exists)),
    ) {
        let p = data.join(name);
        *exists = p.exists();
        if *exists {
            std::fs::copy(&p, root.join(name)).map_err(|e| e.to_string())?;
        }
    }
    if !performances.is_empty() {
        std::fs::create_dir_all(root.join("performance-history")).map_err(|e| e.to_string())?;
    }
    for name in performances.keys() {
        let path = data.join("performance-history").join(name);
        let exists = path.exists();
        if exists {
            std::fs::copy(&path, root.join("performance-history").join(name))
                .map_err(|e| e.to_string())?;
        }
        j.performances.insert(name.clone(), exists);
    }
    let journal = data.join("practice-import-journal.json");
    atomic(&journal, &serde_json::to_vec(&j).unwrap())?;
    let result = (|| {
        atomic(
            &data.join("web-preferences.json"),
            &serde_json::to_vec_pretty(prefs).map_err(|e| e.to_string())?,
        )?;
        atomic(
            &data.join("web-practice-history.ron"),
            &history.snapshot_bytes().map_err(|e| e.to_string())?,
        )?;
        atomic(
            &data.join("library-workspace.json"),
            &serde_json::to_vec(workspace).map_err(|e| e.to_string())?,
        )?;
        if let Some(bytes) = templates {
            atomic(&data.join("metadata-templates.json"), bytes)?;
        }
        if !performances.is_empty() {
            std::fs::create_dir_all(data.join("performance-history")).map_err(|e| e.to_string())?;
        }
        for (name, bytes) in performances {
            atomic(&data.join("performance-history").join(name), bytes)?;
        }
        j.committed = true;
        atomic(&journal, &serde_json::to_vec(&j).unwrap())?;
        Ok::<_, String>(())
    })();
    if let Err(e) = result {
        recover(data).map_err(|r| format!("导入失败：{e}；恢复失败：{r}"))?;
        return Err(e);
    }
    let _ = recover(data);
    Ok(folder)
}
impl Player {
    pub(super) fn practice_backup_snapshot(&self, s: &Selection) -> Result<Value, String> {
        if !s.plans && !s.records && !s.presets {
            return Err("请选择要备份的练习资料".into());
        }
        let lock = library_workspace::lock_for(&self.data)?;
        let _guard = lock.lock().map_err(|_| "曲库资料正忙")?;
        let b = Backup {
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            metadata_templates: if s.presets {
                crate::metadata_templates::snapshot_unlocked(&self.data)?
            } else {
                BTreeMap::new()
            },
            routines: self.preferences.routines.clone(),
            runs: self.preferences.routine_runs.clone(),
            passages: self.preferences.passages.clone(),
            ladders: self.preferences.ladder_presets.clone(),
            ladder_runs: self.preferences.ladder_runs.clone(),
            exercises: self.preferences.exercise_presets.clone(),
            track_sounds: self.preferences.track_sounds.clone(),
            track_appearances: self.preferences.track_appearances.clone(),
            history: self.history.snapshot(),
            performances: BTreeMap::new(),
            learning: library_workspace::Workspace::load(&self.data)?.learning,
        };
        let mut b = selected(b, s);
        let mut bytes = 0usize;
        for (cid, h) in &b.history {
            for session in &h.sessions {
                let id = session_key(session);
                if crate::performance_archive::available(&self.data, cid, &id) {
                    let a = crate::performance_archive::read(&self.data, cid, &id)?;
                    bytes = bytes.saturating_add(a.bytes()?.len());
                    if bytes > MAX_DATA {
                        return Err("回放资料超过备份容量，请缩小成绩日期范围".into());
                    }
                    b.performances.entry(cid.clone()).or_default().insert(id, a);
                }
            }
        }
        serde_json::to_value(b).map_err(|e| e.to_string())
    }
    pub(super) fn inspect_practice_backup(&self, b: &Backup) -> Value {
        let local = self.history.snapshot();
        let conflicts = b
            .routines
            .iter()
            .filter(|r| {
                self.preferences.routines.iter().any(|l| {
                    l.id == r.id
                        && serde_json::to_value(l).unwrap() != serde_json::to_value(r).unwrap()
                })
            })
            .count();
        let duplicate = b
            .history
            .iter()
            .map(|(cid, h)| {
                h.sessions
                    .iter()
                    .filter(|s| {
                        local.get(cid).is_some_and(|l| {
                            l.sessions
                                .iter()
                                .any(|x| session_key(x) == session_key(s) && x.same_performance(s))
                        })
                    })
                    .count()
            })
            .sum::<usize>();
        let known = library_workspace::Workspace::load(&self.data).ok();
        let mut refs = BTreeMap::new();
        for r in b.routines.iter().chain(b.runs.values().map(|r| &r.routine)) {
            for i in &r.items {
                refs.entry(i.content_id.clone()).or_insert(json!({"contentId":i.content_id,"title":i.song_title,"generated":i.exercise.is_some(),"indexed":known.as_ref().is_some_and(|w|w.entries.values().any(|e|e.content_id.as_ref()==Some(&i.content_id)))}));
            }
        }
        let mut conflict_rows = vec![];
        for (cid, plan) in &b.learning {
            let entry = known.as_ref().and_then(|w| {
                w.entries
                    .values()
                    .find(|entry| entry.content_id.as_ref() == Some(cid))
            });
            let title = entry
                .and_then(|entry| entry.metadata.title.clone())
                .unwrap_or_else(|| {
                    if plan.title.is_empty() {
                        format!("未定位曲目 · {}", &cid[..12])
                    } else {
                        plan.title.clone()
                    }
                });
            refs.entry(cid.clone()).or_insert(
                json!({"contentId":cid,"title":title,"generated":false,"indexed":entry.is_some()}),
            );
            if known
                .as_ref()
                .and_then(|w| w.learning.get(cid))
                .is_some_and(|local| local != plan)
            {
                conflict_rows.push(json!({"kind":"曲目学习","local":title,"incoming":title}));
            }
        }

        for r in &b.routines {
            if let Some(local) = self.preferences.routines.iter().find(|v| v.id == r.id) {
                if serde_json::to_value(local).unwrap() != serde_json::to_value(r).unwrap() {
                    conflict_rows.push(json!({"kind":"计划","local":local.name,"incoming":r.name}));
                }
            }
        }
        let day_conflicts = b
            .runs
            .iter()
            .filter(|(key, r)| {
                self.preferences
                    .routine_runs
                    .get(*key)
                    .is_some_and(|local| {
                        serde_json::to_value(local).unwrap() != serde_json::to_value(r).unwrap()
                    })
            })
            .count();
        let mut preset_conflicts = 0;
        for (cid, rows) in &b.passages {
            for r in rows {
                if self.preferences.passages.get(cid).is_some_and(|local| {
                    local.iter().any(|v| {
                        v.id == r.id
                            && serde_json::to_value(v).unwrap() != serde_json::to_value(r).unwrap()
                    })
                }) {
                    preset_conflicts += 1;
                }
            }
        }
        for (cid, rows) in &b.ladders {
            for r in rows {
                if self
                    .preferences
                    .ladder_presets
                    .get(cid)
                    .is_some_and(|local| {
                        local.iter().any(|v| {
                            v.id == r.id
                                && serde_json::to_value(v).unwrap()
                                    != serde_json::to_value(r).unwrap()
                        })
                    })
                {
                    preset_conflicts += 1;
                }
            }
        }
        preset_conflicts += b
            .exercises
            .iter()
            .filter(|r| {
                self.preferences.exercise_presets.iter().any(|v| {
                    v.id == r.id
                        && serde_json::to_value(v).unwrap() != serde_json::to_value(r).unwrap()
                })
            })
            .count();
        preset_conflicts += b
            .ladder_runs
            .iter()
            .filter(|(key, r)| {
                self.preferences.ladder_runs.get(*key).is_some_and(|local| {
                    serde_json::to_value(local).unwrap() != serde_json::to_value(r).unwrap()
                })
            })
            .count();
        preset_conflicts += b
            .track_sounds
            .iter()
            .map(|(cid, rows)| {
                rows.iter()
                    .filter(|(id, s)| {
                        self.preferences
                            .track_sounds
                            .get(cid)
                            .and_then(|m| m.get(id))
                            .is_some_and(|old| old != *s)
                    })
                    .count()
            })
            .sum::<usize>();
        preset_conflicts += b
            .track_appearances
            .iter()
            .map(|(cid, rows)| {
                rows.iter()
                    .filter(|(id, a)| {
                        self.preferences
                            .track_appearances
                            .get(cid)
                            .and_then(|m| m.get(id))
                            .is_some_and(|old| old != *a)
                    })
                    .count()
            })
            .sum::<usize>();
        let templates = crate::metadata_templates::snapshot_unlocked(&self.data);
        if let Ok(local) = &templates {
            for (name, rules) in &b.metadata_templates {
                if local.get(name).is_some_and(|r| r != rules) {
                    preset_conflicts += 1;
                    conflict_rows.push(json!({"kind":"资料模板","local":name,"incoming":name}));
                }
            }
        }
        let mut performance_conflicts = 0;
        let mut performance_duplicates = 0;
        let mut performance_errors = Vec::new();
        for (cid, rows) in &b.performances {
            for (id, a) in rows {
                // Different performances with the same record ID get a new ID on import.
                if !local.get(cid).is_some_and(|h| {
                    h.sessions.iter().any(|s| {
                        session_key(s) == *id
                            && b.history[cid]
                                .sessions
                                .iter()
                                .any(|other| session_key(other) == *id && s.same_performance(other))
                    })
                }) {
                    continue;
                }
                if crate::performance_archive::available(&self.data, cid, id) {
                    match crate::performance_archive::read(&self.data, cid, id) {
                        Ok(old) if old.bytes().ok() == a.bytes().ok() => {
                            performance_duplicates += 1
                        }
                        Ok(old) => {
                            performance_conflicts += 1;
                            if performance_conflicts <= 100 {
                                conflict_rows.push(json!({"kind":"演奏回放","local":format!("{} · {} · {} 个按键/踏板事件",b.history[cid].display_name,id,old.inputs.len()),"incoming":format!("{} · {} · {} 个按键/踏板事件",b.history[cid].display_name,id,a.inputs.len())}));
                            }
                        }
                        Err(error) => {
                            performance_errors.push(json!({"contentId":cid,"id":id,"error":error}))
                        }
                    }
                }
            }
        }
        let mut v = b.summary();
        v["performanceConflicts"] = json!(performance_conflicts);
        v["duplicatePerformances"] = json!(performance_duplicates);
        v["performanceErrors"] = json!(performance_errors);
        v["templateError"] = json!(if b.metadata_templates.is_empty() {
            None
        } else {
            templates.err()
        });
        v["learningConflicts"] = json!(
            b.learning
                .iter()
                .filter(|(cid, plan)| known
                    .as_ref()
                    .and_then(|w| w.learning.get(*cid))
                    .is_some_and(|local| local != *plan))
                .count()
        );
        v["conflicts"] = json!(conflicts);
        v["dayConflicts"] = json!(day_conflicts);
        v["presetConflicts"] = json!(preset_conflicts);
        v["conflictRows"] = json!(conflict_rows);
        v["duplicateRecords"] = json!(duplicate);
        v["annotationConflicts"] = json!(
            b.history
                .iter()
                .map(|(cid, h)| h
                    .sessions
                    .iter()
                    .filter(|s| s.annotation.is_some()
                        && local
                            .get(cid)
                            .is_some_and(|l| l.sessions.iter().any(|x| session_key(x)
                                == session_key(s)
                                && x.same_performance(s)
                                && x.annotation.is_some()
                                && x.annotation != s.annotation)))
                    .count())
                .sum::<usize>()
        );
        v["replayClipConflicts"] = json!(b.history.iter().map(|(cid,h)|h.sessions.iter().map(|s|local.get(cid).and_then(|l|l.sessions.iter().find(|x|(session_key(x)==session_key(s)||session_key(x)==collision_key(s))&&x.same_performance(s))).map(|x|s.replay_clips.iter().filter(|c|x.replay_clips.iter().any(|old|old.id==c.id&&old!=*c)).count()).unwrap_or(0)).sum::<usize>()).sum::<usize>());
        v["references"] = json!(refs.into_values().collect::<Vec<_>>());
        v
    }
    pub(super) fn apply_practice_backup(
        &mut self,
        b: Backup,
        s: Selection,
        policy: String,
    ) -> Result<Value, String> {
        b.validate()?;
        if !matches!(policy.as_str(), "keep" | "backup") {
            return Err("备份冲突策略无效".into());
        }
        if self.state.recording
            || matches!(self.state.status.as_str(), "playing" | "countIn")
            || self.routine.is_some()
            || self.ladder.as_ref().is_some_and(|l| l.active)
        {
            return Err("请先暂停演奏、停止录音并结束活动计划/阶梯，再导入练习资料".into());
        }
        if !s.plans && !s.records && !s.presets {
            return Err("请选择要导入的练习资料".into());
        }
        let lock = library_workspace::lock_for(&self.data)?;
        let _guard = lock.lock().map_err(|_| "曲库资料正忙")?;
        let mut workspace = library_workspace::Workspace::load(&self.data)?;
        let mut b = selected(b, &s);
        let mut learning_changed = 0;
        for (cid, plan) in &b.learning {
            if workspace.learning.get(cid).is_none()
                || (policy == "backup" && workspace.learning.get(cid) != Some(plan))
            {
                workspace.learning.insert(cid.clone(), plan.clone());
                learning_changed += 1;
            }
        }
        if workspace.learning.len() > 10000 {
            return Err("合并后的学习曲目超过上限".into());
        }
        if learning_changed > 0 {
            workspace.revision = workspace.revision.saturating_add(1);
        }

        let replace = policy == "backup";
        let templates = if s.presets && !b.metadata_templates.is_empty() {
            crate::metadata_templates::merge_bytes_unlocked(
                &self.data,
                &b.metadata_templates,
                replace,
            )?
        } else {
            None
        };
        let mut prefs = self.preferences.clone();
        let mut songs = self.history.snapshot();
        let mut added = 0;
        let mut duplicates = 0;
        let mut collisions = 0;
        let mut trimmed = 0;
        let mut remap = BTreeMap::new();
        for (cid, h) in b.history {
            let local = songs.entry(cid.clone()).or_insert_with(|| {
                let mut copy = h.clone();
                copy.sessions.clear();
                copy
            });
            for mut session in h.sessions {
                let key = session_key(&session);
                if let Some(x) = local
                    .sessions
                    .iter_mut()
                    .find(|x| session_key(x) == key && x.same_performance(&session))
                {
                    if x.playing_ms.is_none() {
                        x.playing_ms = session.playing_ms;
                    }
                    if session.annotation.is_some() && (replace || x.annotation.is_none()) {
                        x.annotation = session.annotation.clone();
                    }
                    merge_rows(&mut x.replay_clips,&session.replay_clips,|c|&c.id,replace,100)?;
                    duplicates += 1;
                    continue;
                }
                if local.sessions.iter().any(|x| session_key(x) == key) {
                    let new = collision_key(&session);
                    remap.insert((cid.clone(), key), new.clone());
                    session.id = Some(new);
                    if let Some(x) = local.sessions.iter_mut().find(|x| {
                        session_key(x) == session_key(&session) && x.same_performance(&session)
                    }) {
                        if x.playing_ms.is_none() {
                            x.playing_ms = session.playing_ms;
                        }
                        if session.annotation.is_some() && (replace || x.annotation.is_none()) {
                            x.annotation = session.annotation.clone();
                        }
                        merge_rows(&mut x.replay_clips,&session.replay_clips,|c|&c.id,replace,100)?;
                    duplicates += 1;
                        continue;
                    }
                    collisions += 1;
                }
                local.sessions.push(session);
                added += 1;
            }
            local.sessions.sort_by_key(|v| v.recorded_at_unix_ms);
            if local.sessions.len() > 200 {
                trimmed += local.sessions.len() - 200;
                local.sessions.drain(..local.sessions.len() - 200);
            }
        }
        if s.plans {
            for r in &mut b.routines {
                for i in &mut r.items {
                    i.settings.latency = self.state.latency;
                    if let RoutineTarget::Ladder { preset } = &mut i.target {
                        preset.settings.latency = self.state.latency;
                    }
                }
            }
            for r in b.runs.values_mut() {
                for i in &mut r.routine.items {
                    i.settings.latency = self.state.latency;
                    if let RoutineTarget::Ladder { preset } = &mut i.target {
                        preset.settings.latency = self.state.latency;
                    }
                    if let Some(p) = r.progress.get_mut(&i.id) {
                        for id in &mut p.session_ids {
                            if let Some(new) = remap.get(&(i.content_id.clone(), id.clone())) {
                                *id = new.clone();
                            }
                        }
                    }
                }
            }
            merge_rows(&mut prefs.routines, &b.routines, |r| &r.id, replace, 100)?;
            for (k, r) in b.runs {
                if replace || !prefs.routine_runs.contains_key(&k) {
                    prefs.routine_runs.insert(k, r);
                }
            }
        }
        if s.presets {
            for (cid, rows) in b.track_appearances {
                for (id, a) in rows {
                    let local = prefs.track_appearances.entry(cid.clone()).or_default();
                    if replace || !local.contains_key(&id) {
                        local.insert(id, a);
                    }
                }
            }
            for (cid, rows) in b.track_sounds {
                for (id, sound) in rows {
                    let local = prefs.track_sounds.entry(cid.clone()).or_default();
                    if replace || !local.contains_key(&id) {
                        local.insert(id, sound);
                    }
                }
            }
            for (cid, mut rows) in b.passages {
                for p in &mut rows {
                    p.settings.latency = self.state.latency;
                }
                merge_rows(
                    prefs.passages.entry(cid).or_default(),
                    &rows,
                    |r| &r.id,
                    replace,
                    200,
                )?;
            }
            for (cid, mut rows) in b.ladders {
                for p in &mut rows {
                    p.settings.latency = self.state.latency;
                }
                merge_rows(
                    prefs.ladder_presets.entry(cid).or_default(),
                    &rows,
                    |r| &r.id,
                    replace,
                    200,
                )?;
            }
            for (k, mut r) in b.ladder_runs {
                r.active = false;
                r.preset.settings.latency = self.state.latency;
                if replace || !prefs.ladder_runs.contains_key(&k) {
                    prefs.ladder_runs.insert(k, r);
                }
            }
            merge_rows(
                &mut prefs.exercise_presets,
                &b.exercises,
                |r| &r.id,
                replace,
                1000,
            )?;
        }
        if prefs.track_appearances.len() > 10000
            || prefs.track_sounds.len() > 10000
            || prefs.routine_runs.len() > 10000
            || prefs.passages.len() > 10000
            || prefs.ladder_presets.len() > 10000
            || prefs.ladder_runs.len() > 10000
            || songs.len() > 10000
        {
            return Err("合并后的练习资料超过容量上限".into());
        }
        let mut performance_files = BTreeMap::new();
        let mut imported_performances = 0;
        let mut duplicate_performances = 0;
        let mut kept_performances = 0;
        let mut replaced_performances = 0;
        let mut trimmed_performances = 0;
        for (cid, rows) in b.performances {
            for (original_id, mut a) in rows {
                let id = remap
                    .get(&(cid.clone(), original_id.clone()))
                    .unwrap_or(&original_id);
                if !songs
                    .get(&cid)
                    .is_some_and(|h| h.sessions.iter().any(|s| session_key(s) == *id))
                {
                    trimmed_performances += 1;
                    continue;
                }
                a.reidentify(id);
                a.validate(&cid, id)?;
                let bytes = a.bytes()?;
                let path = crate::performance_archive::path(&self.data, &cid, id);
                let name = path.file_name().unwrap().to_str().unwrap().to_owned();
                if path.exists() {
                    match crate::performance_archive::read(&self.data, &cid, id) {
                        Ok(old) if old.bytes()? == bytes => {
                            duplicate_performances += 1;
                            continue;
                        }
                        Ok(_) if !replace => {
                            kept_performances += 1;
                            continue;
                        }
                        Err(error) if !replace => {
                            return Err(format!(
                                "本地回放损坏：{error}。请选择使用备份版本恢复，或取消成绩导入。"
                            ));
                        }
                        _ => replaced_performances += 1,
                    }
                } else {
                    imported_performances += 1;
                }
                performance_files.insert(name, bytes);
            }
        }
        let mut history = self.history.clone();
        history.replace_snapshot(songs);
        let restore = commit(
            &self.data,
            &prefs,
            &history,
            &workspace,
            templates.as_deref(),
            &performance_files,
        )?;
        let sound_changed = self.preferences.track_sounds != prefs.track_sounds;
        self.preferences = prefs;
        self.history = history;
        if sound_changed {
            self.reset(self.state.position);
        }
        Ok(
            json!({"importedPerformances":imported_performances,"duplicatePerformances":duplicate_performances,"keptPerformances":kept_performances,"replacedPerformances":replaced_performances,"trimmedPerformances":trimmed_performances,"learningChanged":learning_changed,"addedRecords":added,"duplicateRecords":duplicates,"renamedRecords":collisions,"trimmedRecords":trimmed,"restorePoint":restore}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn player() -> Player {
        static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work/cycle128-unit")
            .join(format!(
                "{}-{}",
                SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        Player::new(data, PathBuf::from("unused.sf2"), true)
    }
    fn fixture(p: &mut Player) -> Backup {
        p.command(Command::Generate {
            spec: ExerciseSpec::default(),
        })
        .unwrap();
        p.command(Command::Mode {
            value: "wait".into(),
        })
        .unwrap();
        let result = p
            .save_routine(None, None, "迁移日课".into(), "教师要求".into(), None)
            .unwrap();
        let id = result["id"].as_str().unwrap();
        p.save_routine_item(
            id,
            None,
            None,
            Some("current"),
            None,
            "热身".into(),
            "保持连奏".into(),
            routine_commands::RoutineGoal {
                expression: None,
                passes: 1,
                accuracy: 90,
                on_time: None,
                consecutive: false,
                attempt_limit: 20,
            },
        )
        .unwrap();
        let item = p.preferences.routines[0].items[0].clone();
        let run = RoutineRun {
            day: "2026-10-02".into(),
            routine: p.preferences.routines[0].clone(),
            progress: BTreeMap::from([(
                item.id.clone(),
                routine_commands::RoutineProgress {
                    completed: true,
                    passed: 1,
                    attempts: 1,
                    session_ids: vec!["same-id".into()],
                    ..Default::default()
                },
            )]),
            last_item: Some(item.id.clone()),
        };
        p.preferences
            .routine_runs
            .insert(format!("{}:{}", id, run.day), run);
        let mut session = PracticeSession::new(
            PracticeSessionKind::WholeSong,
            neothesia_core::practice::PracticeHands::Both,
            1.0,
            AttemptSummary::default(),
        );
        session.id = Some("same-id".into());
        session.summary.overall.matched_notes = 9;
        p.history
            .record_session(&item.content_id, &item.song_title, session)
            .unwrap();
        serde_json::from_value(p.practice_backup_snapshot(&Selection::default()).unwrap()).unwrap()
    }
    #[test]
    fn replay_clip_v9_merges_metadata_without_duplicate_records_and_preserves_legacy() {
        use neothesia_core::practice_history::ReplayClip;
        let mut p=player();let mut b=fixture(&mut p);let cid=b.history.keys().next().unwrap().clone();
        let before=p.history.song(&cid).unwrap().sessions[0].clone();
        let clip=ReplayClip{id:"clip-one".into(),name:"左手慢练".into(),start:0.,end:1.,notes:"留意转指".into(),updated_at_unix_ms:1};
        p.history.set_replay_clips(&cid,"same-id",vec![],vec![clip.clone()]).unwrap();
        assert!(p.history.song(&cid).unwrap().sessions[0].same_performance(&before));
        let mut legacy=before.clone();legacy.id=None;let old_id=legacy.stable_id();legacy.replay_clips.push(clip.clone());assert_eq!(legacy.stable_id(),old_id);
        b.history.get_mut(&cid).unwrap().sessions[0].replay_clips=vec![ReplayClip{name:"备份名称".into(),..clip.clone()},ReplayClip{id:"clip-two".into(),name:"第二段".into(),..clip.clone()}];
        assert_eq!(b.version(),9);let b=Backup::decode(&b.encode().unwrap()).unwrap();assert_eq!(b.summary()["replayClips"],2);assert_eq!(p.inspect_practice_backup(&b)["replayClipConflicts"],1);
        p.apply_practice_backup(b.clone(),Selection::default(),"keep".into()).unwrap();
        assert_eq!(p.history.song(&cid).unwrap().sessions.len(),1);assert_eq!(p.history.song(&cid).unwrap().sessions[0].replay_clips[0].name,"左手慢练");assert_eq!(p.history.song(&cid).unwrap().sessions[0].replay_clips.len(),2);
        p.apply_practice_backup(b.clone(),Selection::default(),"backup".into()).unwrap();assert_eq!(p.history.song(&cid).unwrap().sessions[0].replay_clips[0].name,"备份名称");
        let mut old=b.clone();old.history.get_mut(&cid).unwrap().sessions[0].replay_clips.clear();p.apply_practice_backup(old,Selection::default(),"backup".into()).unwrap();assert_eq!(p.history.song(&cid).unwrap().sessions[0].replay_clips.len(),2);
        let mut invalid=b.clone();invalid.history.get_mut(&cid).unwrap().sessions[0].replay_clips[0].end=0.;assert!(invalid.encode().is_err());
        let mut collision=b.clone();collision.history.get_mut(&cid).unwrap().sessions[0].speed=0.5;
        p.apply_practice_backup(collision.clone(),Selection::default(),"keep".into()).unwrap();assert_eq!(p.history.song(&cid).unwrap().sessions.len(),2);
        collision.history.get_mut(&cid).unwrap().sessions[0].replay_clips[0].name="碰撞后的新名字".into();assert_eq!(p.inspect_practice_backup(&collision)["replayClipConflicts"],1);p.apply_practice_backup(collision,Selection::default(),"backup".into()).unwrap();assert_eq!(p.history.song(&cid).unwrap().sessions.len(),2);assert!(p.history.song(&cid).unwrap().sessions.iter().any(|s|s.replay_clips.iter().any(|c|c.name=="碰撞后的新名字")));
        let reload=Player::new(p.data.clone(),PathBuf::from("unused.sf2"),true);assert_eq!(reload.history.song(&cid).unwrap().sessions.len(),2);
    }
    #[test]
    fn measured_time_backup_roundtrip_dedup_and_legacy_unknown() {
        let mut p = player();
        let mut backup = fixture(&mut p);
        let cid = backup.history.keys().next().unwrap().clone();
        backup.history.get_mut(&cid).unwrap().sessions[0].playing_ms = Some(75000);
        let encoded = backup.encode().unwrap();
        let decoded = Backup::decode(&encoded).unwrap();
        assert_eq!(decoded.history[&cid].sessions[0].playing_ms, Some(75000));
        let mut target = player();
        target
            .apply_practice_backup(decoded.clone(), Selection::default(), "keep".into())
            .unwrap();
        target
            .apply_practice_backup(decoded, Selection::default(), "keep".into())
            .unwrap();
        assert_eq!(
            target
                .practice_time(&cid, 0, 9_000_000_000_000_000)
                .unwrap()["totalMs"],
            75000
        );
        assert_eq!(target.history.song(&cid).unwrap().sessions.len(), 1);
        backup.history.get_mut(&cid).unwrap().sessions[0].playing_ms = None;
        let old = Backup::decode(&backup.encode().unwrap()).unwrap();
        assert_eq!(old.history[&cid].sessions[0].playing_ms, None);
        target
            .apply_practice_backup(old.clone(), Selection::default(), "backup".into())
            .unwrap();
        assert_eq!(target.history.song(&cid).unwrap().sessions.len(), 1);
        assert_eq!(
            target
                .practice_time(&cid, 0, 9_000_000_000_000_000)
                .unwrap()["totalMs"],
            75000
        );
        let mut unknown = player();
        unknown
            .apply_practice_backup(old, Selection::default(), "keep".into())
            .unwrap();
        backup.history.get_mut(&cid).unwrap().sessions[0].playing_ms = Some(75000);
        unknown
            .apply_practice_backup(backup.clone(), Selection::default(), "keep".into())
            .unwrap();
        assert_eq!(unknown.history.song(&cid).unwrap().sessions.len(), 1);
        assert_eq!(
            unknown
                .practice_time(&cid, 0, 9_000_000_000_000_000)
                .unwrap()["totalMs"],
            75000
        );
        backup.history.get_mut(&cid).unwrap().sessions[0].playing_ms = Some(u64::MAX);
        assert!(backup.encode().is_err());
    }
    #[test]
    fn notes_legacy_identity_conflicts_restart_and_backup_without_duplicate_grades() {
        use neothesia_core::practice_history::PracticeAnnotation;
        let mut p = player();
        let b = fixture(&mut p);
        let cid = b.history.keys().next().unwrap().clone();
        let annotation = PracticeAnnotation {
            note: "练习感受".into(),
            teacher: "手腕放松".into(),
            next: "左手第 3 小节".into(),
            updated_at_unix_ms: 1790960000000,
        };
        let before = p.history.song(&cid).unwrap().sessions[0].clone();
        p.history
            .annotate_session(&cid, "same-id", None, annotation.clone())
            .unwrap();
        assert!(p.history.song(&cid).unwrap().sessions[0].same_performance(&before));
        assert!(
            p.history
                .annotate_session(&cid, "same-id", None, annotation.clone())
                .is_err()
        );
        assert_eq!(
            p.history_query("手腕", "", "", "", 0, 0, 50).unwrap()["total"],
            1
        );
        let mut annotated: Backup =
            serde_json::from_value(p.practice_backup_snapshot(&Selection::default()).unwrap())
                .unwrap();
        annotated.history.get_mut(&cid).unwrap().sessions[0]
            .annotation
            .as_mut()
            .unwrap()
            .teacher = "备份评语".into();
        assert_eq!(
            p.inspect_practice_backup(&annotated)["annotationConflicts"],
            1
        );
        p.apply_practice_backup(annotated.clone(), Selection::default(), "keep".into())
            .unwrap();
        assert_eq!(p.history.song(&cid).unwrap().sessions.len(), 1);
        assert_eq!(
            p.history.song(&cid).unwrap().sessions[0]
                .annotation
                .as_ref()
                .unwrap()
                .teacher,
            "手腕放松"
        );
        p.apply_practice_backup(annotated, Selection::default(), "backup".into())
            .unwrap();
        assert_eq!(p.history.song(&cid).unwrap().sessions.len(), 1);
        assert_eq!(
            p.history.song(&cid).unwrap().sessions[0]
                .annotation
                .as_ref()
                .unwrap()
                .teacher,
            "备份评语"
        );
        let mut collision = b.clone();
        let extra = &mut collision.history.get_mut(&cid).unwrap().sessions[0];
        extra.summary.overall.wrong_notes = 1;
        extra.annotation = Some(annotation.clone());
        p.apply_practice_backup(collision.clone(), Selection::default(), "keep".into())
            .unwrap();
        collision.history.get_mut(&cid).unwrap().sessions[0]
            .annotation
            .as_mut()
            .unwrap()
            .teacher = "碰撞记录改评语".into();
        p.apply_practice_backup(collision.clone(), Selection::default(), "backup".into())
            .unwrap();
        p.apply_practice_backup(collision, Selection::default(), "backup".into())
            .unwrap();
        assert_eq!(p.history.song(&cid).unwrap().sessions.len(), 2);
        assert_eq!(
            p.history.song(&cid).unwrap().sessions[1]
                .annotation
                .as_ref()
                .unwrap()
                .teacher,
            "碰撞记录改评语"
        );
        let mut legacy = before.clone();
        legacy.id = None;
        let id = legacy.stable_id();
        p.history
            .record_session("legacy-song", "旧记录", legacy)
            .unwrap();
        p.history
            .annotate_session("legacy-song", &id, None, annotation)
            .unwrap();
        assert_eq!(
            p.history.song("legacy-song").unwrap().sessions[0].stable_id(),
            id
        );
        let loaded = PracticeHistoryStore::load(p.history.path().to_path_buf());
        assert_eq!(
            loaded.song(&cid).unwrap().sessions[0]
                .annotation
                .as_ref()
                .unwrap()
                .teacher,
            "备份评语"
        );
        assert_eq!(
            loaded.song("legacy-song").unwrap().sessions[0].stable_id(),
            id
        );
    }
    #[test]
    fn archive_merge_conflicts_and_plan_calibration() {
        let mut source = player();
        let b = fixture(&mut source);
        b.validate().unwrap();
        let bytes = b.encode().unwrap();
        let decoded = Backup::decode(&bytes).unwrap();
        assert_eq!(decoded.summary(), b.summary());
        let mut target = player();
        target.state.latency = 32;
        target.preferences.input = Some("local keyboard".into());
        target
            .apply_practice_backup(decoded.clone(), Selection::default(), "keep".into())
            .unwrap();
        assert_eq!(target.preferences.routines[0].items[0].settings.latency, 32);
        assert_eq!(target.preferences.input.as_deref(), Some("local keyboard"));
        assert_eq!(
            target
                .history
                .snapshot()
                .values()
                .next()
                .unwrap()
                .sessions
                .len(),
            1
        );
        target.preferences.routines[0].name = "本地安排".into();
        let result = target
            .apply_practice_backup(decoded.clone(), Selection::default(), "keep".into())
            .unwrap();
        assert_eq!(target.preferences.routines[0].name, "本地安排");
        assert_eq!(result["duplicateRecords"], 1);
        target
            .apply_practice_backup(decoded.clone(), Selection::default(), "backup".into())
            .unwrap();
        assert_eq!(target.preferences.routines[0].name, "迁移日课");
        assert_eq!(
            target
                .preferences
                .routine_runs
                .values()
                .next()
                .unwrap()
                .progress
                .values()
                .next()
                .unwrap()
                .completed,
            true
        );
        let mut changed = decoded;
        changed.history.values_mut().next().unwrap().sessions[0]
            .summary
            .overall
            .matched_notes = 8;
        let result = target
            .apply_practice_backup(changed, Selection::default(), "backup".into())
            .unwrap();
        assert_eq!(result["renamedRecords"], 1);
        assert_eq!(
            target
                .history
                .snapshot()
                .values()
                .next()
                .unwrap()
                .sessions
                .len(),
            2
        );
        let run = target.preferences.routine_runs.values().next().unwrap();
        assert!(run.progress.values().next().unwrap().session_ids[0].starts_with("import-"));
        let disk = PracticeHistoryStore::load(target.data.join("web-practice-history.ron"));
        assert_eq!(disk.snapshot(), target.history.snapshot());
    }
    #[test]
    fn malformed_archives_and_active_import_do_not_change_data() {
        let mut p = player();
        let b = fixture(&mut p);
        let before = p.preferences.routines[0].name.clone();
        p.state.status = "playing".into();
        assert!(
            p.apply_practice_backup(b.clone(), Selection::default(), "backup".into())
                .is_err()
        );
        assert_eq!(p.preferences.routines[0].name, before);
        let mut invalid = b.clone();
        invalid.routines[0].items[0].settings.latency = i32::MIN;
        assert!(invalid.validate().is_err());
        let mut corrupt = b.encode().unwrap();
        corrupt[0] = 0;
        assert!(Backup::decode(&corrupt).is_err());
        let mut invalid = b;
        invalid.runs.values_mut().next().unwrap().day = "2026-02-31".into();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn metadata_templates_back_up_merge_and_roll_back_with_old_versions() {
        let source = player();
        let rules: crate::metadata_templates::Rules =
            serde_json::from_value(json!({"notes":{"mode":"append","value":"备份提示"}})).unwrap();
        crate::metadata_templates::update(&source.data, "课堂".into(), Some(rules.clone()), 0)
            .unwrap();
        let selection = Selection {
            plans: false,
            records: false,
            presets: true,
            days: 0,
        };
        let backup: Backup =
            serde_json::from_value(source.practice_backup_snapshot(&selection).unwrap()).unwrap();
        assert_eq!(backup.version(), 4);
        assert_eq!(backup.metadata_templates.len(), 1);
        let decoded = Backup::decode(&backup.encode().unwrap()).unwrap();
        assert_eq!(decoded.metadata_templates, backup.metadata_templates);
        let excluded = selected(
            backup.clone(),
            &Selection {
                presets: false,
                ..Selection::default()
            },
        );
        assert!(excluded.metadata_templates.is_empty());
        let mut target = player();
        let local_rules: crate::metadata_templates::Rules =
            serde_json::from_value(json!({"notes":{"mode":"append","value":"本地提示"}})).unwrap();
        crate::metadata_templates::update(
            &target.data,
            "课堂".into(),
            Some(local_rules.clone()),
            0,
        )
        .unwrap();
        assert_eq!(
            target.inspect_practice_backup(&decoded)["presetConflicts"],
            1
        );
        target
            .apply_practice_backup(decoded.clone(), selection.clone(), "keep".into())
            .unwrap();
        assert_eq!(
            crate::metadata_templates::snapshot_unlocked(&target.data).unwrap()["课堂"],
            local_rules
        );
        target
            .apply_practice_backup(decoded.clone(), selection.clone(), "backup".into())
            .unwrap();
        assert_eq!(
            crate::metadata_templates::snapshot_unlocked(&target.data).unwrap()["课堂"],
            rules
        );
        let path = target.data.join("metadata-templates.json");
        let original = std::fs::read(&path).unwrap();
        let folder = "restore-template-test";
        let restore = target.data.join("practice-restore-points").join(folder);
        std::fs::create_dir_all(&restore).unwrap();
        std::fs::write(restore.join("metadata-templates.json"), &original).unwrap();
        std::fs::write(&path, b"partial new templates").unwrap();
        let journal = Journal {
            performances: BTreeMap::new(),
            folder: folder.into(),
            prefs: false,
            history: false,
            workspace: None,
            templates: Some(true),
            committed: false,
        };
        std::fs::write(
            target.data.join("practice-import-journal.json"),
            serde_json::to_vec(&journal).unwrap(),
        )
        .unwrap();
        recover(&target.data).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let old = Backup::default();
        assert_eq!(Backup::decode(&old.encode().unwrap()).unwrap().version(), 1);
        target
            .apply_practice_backup(old, selection.clone(), "backup".into())
            .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let mut invalid = backup;
        invalid
            .metadata_templates
            .get_mut("课堂")
            .unwrap()
            .get_mut("notes")
            .unwrap()
            .mode = "invalid".into();
        assert!(invalid.encode().is_err());
        std::fs::write(&path, b"broken").unwrap();
        assert!(
            target
                .apply_practice_backup(decoded, selection, "backup".into())
                .is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), b"broken");
    }

    fn add_performance(p: &Player, b: &mut Backup, velocity: u8) {
        let cid = b.history.keys().next().unwrap().clone();
        let id = session_key(&b.history[&cid].sessions[0]);
        let a:crate::performance_archive::Archive=serde_json::from_value(json!({
            "version":1,"contentId":cid,"id":id,"duration":1.,"truncated":false,
            "inputs":[{"at":0.1,"judgedAt":0.08,"songTime":0.1,"bytes":[144,60,velocity],"synthetic":false},{"at":0.8,"judgedAt":0.78,"songTime":0.8,"bytes":[128,60,0],"synthetic":false}],
            "references":[{"at":0.,"end":1.,"pitch":60,"velocity":90,"measure":1,"songTime":0.,"track":0}],
            "referencePedals":[{"at":0.2,"judgedAt":0.2,"songTime":0.2,"bytes":[176,64,100],"synthetic":false}]
        })).unwrap();
        let path = crate::performance_archive::path(&p.data, &cid, &id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, a.bytes().unwrap()).unwrap();
        b.performances.entry(cid).or_default().insert(id, a);
    }
    #[test]
    fn performances_roundtrip_conflicts_remap_date_scope_and_restart() {
        let mut source = player();
        let mut b = fixture(&mut source);
        add_performance(&source, &mut b, 83);
        let cid = b.history.keys().next().unwrap().clone();
        let snapshot: Backup = serde_json::from_value(
            source
                .practice_backup_snapshot(&Selection::default())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(snapshot.version(), 6);
        assert_eq!(snapshot.summary()["performances"], 1);
        let decoded = Backup::decode(&snapshot.encode().unwrap()).unwrap();
        let mut target = player();
        let result = target
            .apply_practice_backup(decoded.clone(), Selection::default(), "keep".into())
            .unwrap();
        assert_eq!(result["importedPerformances"], 1);
        assert_eq!(
            target.history_performance(&cid, "same-id").unwrap()["inputs"][0]["bytes"][2],
            83
        );
        assert_eq!(
            target
                .apply_practice_backup(decoded.clone(), Selection::default(), "keep".into())
                .unwrap()["duplicatePerformances"],
            1
        );
        let mut changed = decoded.clone();
        changed
            .performances
            .get_mut(&cid)
            .unwrap()
            .get_mut("same-id")
            .unwrap()
            .inputs[0]
            .bytes[2] = 99;
        assert_eq!(
            target.inspect_practice_backup(&changed)["performanceConflicts"],
            1
        );
        assert_eq!(
            target
                .apply_practice_backup(changed.clone(), Selection::default(), "keep".into())
                .unwrap()["keptPerformances"],
            1
        );
        assert_eq!(
            target.history_performance(&cid, "same-id").unwrap()["inputs"][0]["bytes"][2],
            83
        );
        assert_eq!(
            target
                .apply_practice_backup(changed.clone(), Selection::default(), "backup".into())
                .unwrap()["replacedPerformances"],
            1
        );
        let mut collision = decoded.clone();
        collision.history.get_mut(&cid).unwrap().sessions[0]
            .summary
            .overall
            .wrong_notes = 3;
        assert_eq!(
            target
                .apply_practice_backup(collision.clone(), Selection::default(), "keep".into())
                .unwrap()["renamedRecords"],
            1
        );
        let renamed = target
            .history
            .song(&cid)
            .unwrap()
            .sessions
            .iter()
            .find(|s| s.stable_id() != "same-id")
            .unwrap()
            .stable_id();
        assert_eq!(
            target.history_performance(&cid, &renamed).unwrap()["id"],
            renamed
        );
        assert_eq!(
            target.history_performance(&cid, &renamed).unwrap()["inputs"][0]["bytes"][2],
            83
        );
        assert_eq!(
            target
                .apply_practice_backup(collision, Selection::default(), "keep".into())
                .unwrap()["duplicatePerformances"],
            1
        );
        let restarted = Player::new(target.data.clone(), PathBuf::from("unused.sf2"), true);
        assert_eq!(
            restarted.history_performance(&cid, "same-id").unwrap()["inputs"][0]["bytes"][2],
            99
        );
        assert!(
            selected(
                decoded.clone(),
                &Selection {
                    records: false,
                    ..Selection::default()
                }
            )
            .performances
            .is_empty()
        );
        let mut dated = decoded.clone();
        dated.created_at = dated.history[&cid].sessions[0].recorded_at_unix_ms + 90 * 86400000;
        assert!(
            selected(
                dated,
                &Selection {
                    days: 30,
                    ..Selection::default()
                }
            )
            .performances
            .is_empty()
        );
        let mut orphan = decoded.clone();
        orphan.history.clear();
        assert!(orphan.encode().is_err());
        let mut wrong = decoded;
        wrong.performances.get_mut(&cid).unwrap().insert(
            "wrong-id".into(),
            changed.performances[&cid]["same-id"].clone(),
        );
        assert!(wrong.encode().is_err());
        let path = crate::performance_archive::path(&target.data, &cid, "same-id");
        std::fs::write(&path, b"broken").unwrap();
        assert_eq!(
            target.inspect_practice_backup(&changed)["performanceErrors"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(
            target
                .apply_practice_backup(changed.clone(), Selection::default(), "keep".into())
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"broken");
        assert_eq!(
            target
                .apply_practice_backup(changed, Selection::default(), "backup".into())
                .unwrap()["replacedPerformances"],
            1
        );
    }
    #[test]
    fn interrupted_import_restores_existing_and_removes_new_performances() {
        let mut p = player();
        let mut b = fixture(&mut p);
        add_performance(&p, &mut b, 83);
        let cid = b.history.keys().next().unwrap();
        let path = crate::performance_archive::path(&p.data, cid, "same-id");
        let original = std::fs::read(&path).unwrap();
        let new_path = crate::performance_archive::path(&p.data, cid, "new-id");
        let restore = p
            .data
            .join("practice-restore-points/restore-performance-test/performance-history");
        std::fs::create_dir_all(&restore).unwrap();
        std::fs::write(restore.join(path.file_name().unwrap()), &original).unwrap();
        std::fs::write(&path, b"partial").unwrap();
        std::fs::write(&new_path, b"new").unwrap();
        let j = Journal {
            folder: "restore-performance-test".into(),
            prefs: false,
            history: false,
            workspace: None,
            templates: None,
            committed: false,
            performances: BTreeMap::from([
                (path.file_name().unwrap().to_str().unwrap().into(), true),
                (
                    new_path.file_name().unwrap().to_str().unwrap().into(),
                    false,
                ),
            ]),
        };
        std::fs::write(
            p.data.join("practice-import-journal.json"),
            serde_json::to_vec(&j).unwrap(),
        )
        .unwrap();
        recover(&p.data).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), original);
        assert!(!new_path.exists());
    }
    #[test]
    fn interrupted_transaction_restores_both_files() {
        let mut p = player();
        fixture(&mut p);
        p.preferences.save(&p.preferences_path).unwrap();
        p.history.save().unwrap();
        let old_prefs = std::fs::read(&p.preferences_path).unwrap();
        let old_history = std::fs::read(p.history.path()).unwrap();
        let folder = "restore-test";
        let restore = p.data.join("practice-restore-points").join(folder);
        std::fs::create_dir_all(&restore).unwrap();
        std::fs::write(restore.join("web-preferences.json"), &old_prefs).unwrap();
        std::fs::write(restore.join("web-practice-history.ron"), &old_history).unwrap();
        std::fs::write(&p.preferences_path, b"partial new preferences").unwrap();
        let journal = Journal {
            performances: BTreeMap::new(),
            folder: folder.into(),
            prefs: true,
            history: true,
            workspace: None,
            templates: None,
            committed: false,
        };
        std::fs::write(
            p.data.join("practice-import-journal.json"),
            serde_json::to_vec(&journal).unwrap(),
        )
        .unwrap();
        recover(&p.data).unwrap();
        assert_eq!(std::fs::read(&p.preferences_path).unwrap(), old_prefs);
        assert_eq!(std::fs::read(p.history.path()).unwrap(), old_history);
        assert!(!p.data.join("practice-import-journal.json").exists());
    }
}

// An internally tagged command buffers map keys as strings. Re-enter the JSON
// deserializer here so nested numeric track/part maps keep their normal semantics.
pub(crate) fn deserialize_command_backup<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Backup, D::Error> {
    let value = serde_json::Value::deserialize(d)?;
    serde_json::from_value(value).map_err(serde::de::Error::custom)
}
