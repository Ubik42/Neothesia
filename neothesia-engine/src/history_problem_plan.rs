use super::*;
use crate::preferences::SessionSettings;
use crate::routine_commands::{PracticeRoutine, RoutineGoal, RoutineItem, RoutineTarget};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProblemPlanSpec {
    pub content_id: String,
    pub id: String,
    pub score_revision: u64,
    pub references: Vec<usize>,
    pub before: u8,
    pub after: u8,
    pub mode: String,
    pub speed: f64,
    pub routine_id: Option<String>,
    pub day: Option<String>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProblemPlanEdit {
    #[serde(default)]
    pub cut:Option<crate::history_problem_cuts::ProblemCut>,
    pub key: String,
    pub title: String,
    pub notes: String,
}
pub(super) struct Built {
    pub(super) cuts:BTreeMap<String,crate::history_problem_cuts::CutContext>,
    pub(super) rows: Vec<Value>,
    pub(super) items: Vec<RoutineItem>,
    pub(super) revision: String,
}
impl Player {
    pub(super) fn build_history_problem_plan(&self, spec: &ProblemPlanSpec) -> Result<Built, String> {
        let file = self.file.as_ref().ok_or("请先从历史打开原曲并批阅")?;
        if file.content_id != spec.content_id || self.score_revision != spec.score_revision {
            return Err("批阅曲目或谱面已改变，请重新打开这次批阅".into());
        }
        self.build_history_problem_source(
            spec,
            file,
            &self.notes,
            &self.grid_signature(),
            &self.practice_scope(),
            json!(spec.score_revision),
        )
    }
    pub(super) fn build_history_problem_source(
        &self,
        spec: &ProblemPlanSpec,
        file: &MidiFile,
        notes: &[PracticeNote],
        grid: &str,
        scope: &str,
        source_proof: Value,
    ) -> Result<Built, String> {
        if spec.references.is_empty()
            || spec.references.len() > 60
            || spec.references.iter().collect::<BTreeSet<_>>().len() != spec.references.len()
        {
            return Err("请选择 1～60 个不同的问题音".into());
        }
        if spec.before > 8
            || spec.after > 8
            || !matches!(spec.mode.as_str(), "wait" | "flow")
            || !spec.speed.is_finite()
            || !(0.25..=2.).contains(&spec.speed)
        {
            return Err("练习条件无效：前后最多 8 小节，速度 25%～200%，使用等音或连续模式".into());
        }
        if spec
            .day
            .as_deref()
            .is_some_and(|d| !routine_commands::day_valid(d))
        {
            return Err("日期无效".into());
        }
        if file.content_id != spec.content_id {
            return Err("历史曲目内容不对应".into());
        }
        let history = self
            .history
            .song(&spec.content_id)
            .ok_or("历史曲目不存在")?;
        let session = history
            .sessions
            .iter()
            .find(|s| s.stable_id() == spec.id)
            .ok_or("演奏记录已不存在")?;
        let ctx = session
            .context
            .as_ref()
            .ok_or("旧记录缺少原练习条件，不能生成问题计划")?;
        if grid != ctx.grid || session.scope.as_deref() != Some(scope) {
            return Err("小节、声部或逐音分手已改变，请重新校对原条件")?;
        }
        let archive = crate::performance_archive::read(&self.data, &spec.content_id, &spec.id)?;
        let decisions = archive
            .judgments
            .as_ref()
            .ok_or("旧演奏没有逐音判定，不能自动当作问题音")?;
        let settings = SessionSettings {
            tracks: ctx.tracks.clone(),
            parts: ctx.parts.clone(),
            mode: Some(spec.mode.clone()),
            hands: ctx.hands.clone(),
            count_in: ctx.count_in,
            metronome: spec.mode == "flow" || ctx.metronome,
            latency: self.state.latency,
            rounds: 0,
            adaptive: false,
        };
        let mut rows: Vec<Value> = vec![];
        let mut items: Vec<RoutineItem> = vec![];
        let mut ranges = BTreeMap::new();
        for reference in &spec.references {
            let n = archive
                .references
                .get(*reference)
                .ok_or("所选参考音已不存在，请重新读取")?;
            let grade = decisions
                .iter()
                .find(|d| d.reference == Some(*reference))
                .filter(|d| matches!(d.kind.as_str(), "early" | "late" | "missed"))
                .ok_or("所选音没有偏早、偏晚或漏音判定")?;
            let matching = notes
                .iter()
                .filter(|note| {
                    note.track_id == n.track
                        && note.note == n.pitch
                        && (note.start.as_secs_f64() - n.song_time).abs() < 0.001
                })
                .count();
            let measure = file
                .measures
                .partition_point(|m| m.as_secs_f64() <= n.song_time);
            if matching != 1
                || measure == 0
                || measure != n.measure
                || measure > file.musical_time.measures.len()
            {
                return Err("问题音或小节没有唯一对应，不能按旧记录编排")?;
            }
            let start = measure.saturating_sub(spec.before as usize).max(1);
            let end = (measure + spec.after as usize).min(file.musical_time.measures.len());
            if let Some(&index) = ranges.get(&(start, end)) {
                let row: &mut Value = &mut rows[index];
                row["references"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!(reference));
                row["problems"].as_array_mut().unwrap().push(json!({"reference":reference,"measure":measure,"pitch":n.pitch,"kind":grade.kind}));
                continue;
            }
            ranges.insert((start, end), rows.len());
            let key = format!("{start}-{end}");
            let title = if start == end {
                format!("第 {start} 小节重练")
            } else {
                format!("第 {start}–{end} 小节重练")
            };
            rows.push(json!({"key":key,"title":title,"notes":"","start":start,"end":end,"references":[reference],"problems":[{"reference":reference,"measure":measure,"pitch":n.pitch,"kind":grade.kind}]}));
            items.push(RoutineItem {
                id: key,
                title,
                notes: format!(
                    "来自 {} 的演奏记录 {}，问题小节 {}。",
                    history.display_name, spec.id, measure
                ),
                content_id: spec.content_id.clone(),
                song_title: history.display_name.clone(),
                path: file.source_path.clone(),
                exercise: ctx.exercise_spec,
                grid: ctx.grid.clone(),
                scope: session.scope.clone().unwrap(),
                settings: settings.clone(),
                speed: spec.speed,
                target: RoutineTarget::Passage { start, end },
                goal: RoutineGoal {
                    expression: None,
                    passes: 2,
                    accuracy: 90,
                    on_time: None,
                    consecutive: false,
                    attempt_limit: 20,
                },
            });
        }
        for (index, item) in items.iter_mut().enumerate() {
            let issues = rows[index]["problems"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    format!(
                        "第 {} 小节{}",
                        p["measure"],
                        match p["kind"].as_str().unwrap() {
                            "early" => "偏早",
                            "late" => "偏晚",
                            _ => "漏音",
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("、");
            item.notes = format!("来自演奏记录 {}。所选问题：{}。", spec.id, issues);
        }
        let mut cuts=BTreeMap::new();
        for row in &mut rows {
            let anchors=row["references"].as_array().unwrap().iter().map(|r|archive.references[r.as_u64().unwrap()as usize].song_time).collect();
            let ctx=crate::history_problem_cuts::CutContext::new(file,grid,row["start"].as_u64().unwrap()as usize,row["end"].as_u64().unwrap()as usize,anchors)?;
            row["cutBounds"]=ctx.bounds()?;row["cut"]=Value::Null;
            cuts.insert(row["key"].as_str().unwrap().to_string(),ctx);
        }
        let target = if let Some(id) = &spec.routine_id {
            self.routine_editable(id, spec.day.as_deref())?;
            if let Some(day) = &spec.day {
                self.preferences
                    .routine_runs
                    .get(&format!("{id}:{day}"))
                    .map(|r| json!(r))
                    .or_else(|| {
                        self.preferences
                            .routines
                            .iter()
                            .find(|r| &r.id == id)
                            .map(|r| json!({"newDay":r}))
                    })
                    .ok_or("目标计划不存在")?
            } else {
                json!(
                    self.preferences
                        .routines
                        .iter()
                        .find(|r| &r.id == id)
                        .ok_or("目标计划不存在")?
                )
            }
        } else {
            Value::Null
        };
        let proof = json!([
            spec,
            source_proof,
            rows,
            ctx,
            self.state.latency,
            target,
            blake3::hash(&archive.bytes()?).to_hex().to_string()
        ]);
        let revision = blake3::hash(&serde_json::to_vec(&proof).unwrap())
            .to_hex()
            .to_string();
        Ok(Built {
            cuts,
            rows,
            items,
            revision,
        })
    }
    pub(super) fn preview_history_problem_plan(
        &self,
        spec: &ProblemPlanSpec,
    ) -> Result<Value, String> {
        let built = self.build_history_problem_plan(spec)?;
        Ok(
            json!({"rows":built.rows,"revision":built.revision,"selected":spec.references.len(),"mode":spec.mode,"speed":spec.speed}),
        )
    }
    pub(super) fn save_history_problem_plan(
        &mut self,
        spec: &ProblemPlanSpec,
        expected: &str,
        name: &str,
        notes: &str,
        goal: &RoutineGoal,
        edits: &[ProblemPlanEdit],
    ) -> Result<Value, String> {
        if self.state.recording
            || self.routine.is_some()
            || self.ladder.as_ref().is_some_and(|l| l.active)
            || matches!(self.state.status.as_str(), "playing" | "countIn")
            || !self.state.pressed.is_empty()
        {
            return Err("请先停止演奏、录音和活动计划，再保存编排".into());
        }
        let built = self.build_history_problem_plan(spec)?;
        let built = self.prepare_problem_built(built, expected, goal, &spec.mode, edits)?;
        self.save_problem_items(spec, built, expected, name, notes, goal, edits)
    }
    pub(super) fn prepare_problem_built(
        &self,
        mut built: Built,
        expected: &str,
        goal: &RoutineGoal,
        mode: &str,
        edits: &[ProblemPlanEdit],
    ) -> Result<Built, String> {
        if built.revision != expected {
            return Err("演奏证据、条件或目标计划已改变，请重新预览后保存".into());
        }
        built=self.apply_problem_cuts(built,expected,edits)?;
        goal.validate(mode)?;
        if goal.expression.is_some() {
            return Err("批阅编排先保存音符或节奏要求；力度/踏板要求可在计划中逐项校对".into());
        }
        if edits.len() != built.items.len()
            || edits.iter().map(|e| &e.key).collect::<BTreeSet<_>>().len() != edits.len()
        {
            return Err("编排项目已改变，请重新预览".into());
        }
        for item in &mut built.items {
            let edit = edits
                .iter()
                .find(|e| e.key == item.id)
                .ok_or("项目范围已改变")?;
            if edit.title.trim().is_empty()
                || edit.title.chars().count() > 100
                || edit.notes.len() > 7000
            {
                return Err("项目名称需 1～100 字，要求最长 7000 字节".into());
            }
            item.title = edit.title.trim().into();
            item.notes = format!("{}\n{}", item.notes, edit.notes).trim().into();
            item.goal = goal.clone();
            if item.notes.len() > 8192 {
                return Err("项目要求和来源说明合计超过 8192 字节，请缩短要求".into());
            }
        }
        Ok(built)
    }
    pub(super) fn save_problem_items(
        &mut self,
        spec: &ProblemPlanSpec,
        built: Built,
        expected: &str,
        name: &str,
        notes: &str,
        goal: &RoutineGoal,
        edits: &[ProblemPlanEdit],
    ) -> Result<Value, String> {
        if self.state.recording
            || self.routine.is_some()
            || self.ladder.as_ref().is_some_and(|l| l.active)
            || matches!(self.state.status.as_str(), "playing" | "countIn")
            || !self.state.pressed.is_empty()
        {
            return Err("请先停止演奏、录音和活动计划，再保存编排".into());
        }
        if spec.routine_id.is_none()
            && (name.trim().is_empty() || name.chars().count() > 80 || notes.len() > 8192)
        {
            return Err("计划名称需 1～80 字，备注最多 8192 字节，常用计划最多 100 个".into());
        }
        let previous = self.preferences.clone();
        let result = (|| {
            let id = spec.routine_id.clone().unwrap_or_else(|| {
                format!(
                    "history-plan-{}",
                    blake3::hash(
                        &serde_json::to_vec(&json!([expected, name, notes, goal, edits])).unwrap()
                    )
                    .to_hex()
                )
            });
            if spec.routine_id.is_none() && !self.preferences.routines.iter().any(|r| r.id == id) {
                if self.preferences.routines.len() >= 100 {
                    return Err("常用计划最多 100 个".into());
                }
                self.preferences.routines.push(PracticeRoutine {
                    schedule: None,
                    id: id.clone(),
                    name: name.trim().into(),
                    notes: notes.into(),
                    items: vec![],
                });
            }
            let target = self.routine_target_mut(
                &id,
                if spec.routine_id.is_some() {
                    spec.day.as_deref()
                } else {
                    None
                },
            )?;
            let mut added = 0;
            let mut skipped = 0;
            let mut ids = vec![];
            for mut item in built.items {
                if target.items.iter().any(|old| {
                    old.content_id == item.content_id
                        && old.grid == item.grid
                        && old.scope == item.scope
                        && old.speed == item.speed
                        && old.goal == item.goal
                        && old.title == item.title
                        && old.notes == item.notes
                        && serde_json::to_value(&old.settings).unwrap()
                            == serde_json::to_value(&item.settings).unwrap()
                        && serde_json::to_value(&old.target).unwrap()
                            == serde_json::to_value(&item.target).unwrap()
                }) {
                    skipped += 1;
                    continue;
                }
                if target.items.len() >= 200 {
                    return Err("每个计划最多 200 个项目，未保存任何新增项目".into());
                }
                item.id = format!(
                    "{}-{}-{}",
                    id,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos(),
                    added
                );
                ids.push(item.id.clone());
                target.items.push(item);
                added += 1;
            }
            if spec.routine_id.is_none() && spec.day.is_some() {
                self.routine_target_mut(&id, spec.day.as_deref())?;
            }
            if self.preferences.routine_runs.len() > 10000 {
                return Err("日期安排超过容量上限".into());
            }
            self.preferences.save(&self.preferences_path)?;
            Ok(json!({"id":id,"day":spec.day,"added":added,"skipped":skipped,"itemIds":ids}))
        })();
        if result.is_err() {
            self.preferences = previous;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> (Player, ProblemPlanSpec) {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work")
            .join(format!(
                "cycle179-unit-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut p = Player::new(data, PathBuf::new(), true);
        let measures=(0..5).map(|i|format!("<measure number=\"{}\">{}<note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note>{}</measure>",i+1,if i==0{"<attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes>"}else{""},if i==0{"<note><chord/><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration></note>"}else{""})).collect::<String>();
        let xml = format!(
            "<score-partwise><part-list><score-part id=\"P1\"><part-name>Piano</part-name></score-part></part-list><part id=\"P1\">{measures}</part></score-partwise>"
        );
        p.command(Command::ImportScore {
            bytes: xml.into_bytes(),
            name: "多问题.musicxml".into(),
            default_bpm: 120,
        })
        .unwrap();
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let track = p.notes[0].track_id;
        p.command(Command::Track {
            id: track,
            mode: "human".into(),
            part: "right".into(),
            visible: true,
        })
        .unwrap();
        p.command(Command::Mode {
            value: "recital".into(),
        })
        .unwrap();
        p.command(Command::CountIn { bars: 0 }).unwrap();
        p.command(Command::Play).unwrap();
        for _ in 0..70 {
            p.tick(Duration::from_millis(50));
        }
        assert_eq!(p.state.status, "finished");
        let id = p.history.song(&cid).unwrap().sessions[0].stable_id();
        p.history_score_review(&cid, &id).unwrap();
        let spec = ProblemPlanSpec {
            content_id: cid,
            id,
            score_revision: p.score_revision,
            references: vec![5, 0, 1],
            before: 0,
            after: 0,
            mode: "wait".into(),
            speed: 0.7,
            routine_id: None,
            day: Some("2026-10-02".into()),
        };
        (p, spec)
    }
    fn edits(b: &Built) -> Vec<ProblemPlanEdit> {
        b.rows
            .iter()
            .map(|r| ProblemPlanEdit {
                cut:None,
                key: r["key"].as_str().unwrap().into(),
                title: r["title"].as_str().unwrap().into(),
                notes: "放松换指".into(),
            })
            .collect()
    }
    fn goal() -> RoutineGoal {
        RoutineGoal {
            expression: None,
            passes: 1,
            accuracy: 90,
            on_time: None,
            consecutive: false,
            attempt_limit: 3,
        }
    }
    #[test]
    fn ordered_problem_ranges_save_retry_reopen_backup_and_real_daily_progress() {
        let (mut p, spec) = source();
        let count = p.history.song(&spec.content_id).unwrap().sessions.len();
        let position = p.state.position;
        let score = serde_json::to_value(p.matcher.snapshot()).unwrap();
        let b = p.build_history_problem_plan(&spec).unwrap();
        assert_eq!(b.rows.len(), 2);
        assert_eq!(b.rows[0]["start"], 5);
        assert_eq!(b.rows[1]["references"].as_array().unwrap().len(), 2);
        assert_eq!(p.state.position, position);
        let saved = p
            .save_history_problem_plan(
                &spec,
                &b.revision,
                "批阅重练",
                "保留顺序",
                &goal(),
                &edits(&b),
            )
            .unwrap();
        let id = saved["id"].as_str().unwrap().to_string();
        assert_eq!(saved["added"], 2);
        let retry = p
            .save_history_problem_plan(
                &spec,
                &b.revision,
                "批阅重练",
                "保留顺序",
                &goal(),
                &edits(&b),
            )
            .unwrap();
        assert_eq!(retry["id"], saved["id"]);
        assert_eq!(retry["added"], 0);
        assert_eq!(p.preferences.routines.len(), 1);
        assert_eq!(
            p.history.song(&spec.content_id).unwrap().sessions.len(),
            count
        );
        assert_eq!(serde_json::to_value(p.matcher.snapshot()).unwrap(), score);
        assert_eq!(p.state.position, position);
        let backup: crate::practice_backup::Backup = serde_json::from_value(
            p.practice_backup_snapshot(&crate::practice_backup::Selection::default())
                .unwrap(),
        )
        .unwrap();
        let decoded = crate::practice_backup::Backup::decode(&backup.encode().unwrap()).unwrap();
        assert_eq!(decoded.routines[0].items.len(), 2);
        let mut target = Player::new(p.data.join("restored"), PathBuf::new(), true);
        target
            .apply_practice_backup(
                decoded,
                crate::practice_backup::Selection::default(),
                "keep".into(),
            )
            .unwrap();
        assert_eq!(
            target.preferences.routines[0].items[0].title,
            b.items[0].title
        );
        let mut reopen = Player::new(p.data.clone(), PathBuf::new(), true);
        reopen.restore();
        let item = reopen.preferences.routines[0].items[0].id.clone();
        reopen
            .command(Command::OpenRoutineItem {
                routine_id: id.clone(),
                day: "2026-10-02".into(),
                item_id: item,
                resume: true,
            })
            .unwrap();
        assert_eq!(reopen.state.passage.as_ref().unwrap().start, 2.);
        assert_eq!(reopen.state.speed, 0.7);
        reopen.command(Command::Play).unwrap();
        for _ in 0..400 {
            reopen.tick(Duration::from_millis(10));
            for pitch in reopen.snapshot().required {
                reopen
                    .command(Command::Note {
                        pitch,
                        velocity: 90,
                        active: true,
                    })
                    .unwrap();
                reopen
                    .command(Command::Note {
                        pitch,
                        velocity: 0,
                        active: false,
                    })
                    .unwrap();
            }
            if reopen.routine_status().unwrap()["progress"]["completed"] == true {
                break;
            }
        }
        assert_eq!(
            reopen.routine_status().unwrap()["progress"]["completed"],
            true
        );
        assert_eq!(reopen.routine_status().unwrap()["index"], 1);
        assert_eq!(reopen.routine_status().unwrap()["total"], 2);
        let next = reopen.routine_status().unwrap()["nextItemId"]
            .as_str()
            .unwrap()
            .to_string();
        reopen
            .command(Command::OpenRoutineItem {
                routine_id: id,
                day: "2026-10-02".into(),
                item_id: next,
                resume: true,
            })
            .unwrap();
        assert_eq!(reopen.state.passage.as_ref().unwrap().start, 0.);
        assert_eq!(reopen.routine_status().unwrap()["progress"]["attempts"], 0);
        let run_key = format!("{}:2026-10-02", saved["id"].as_str().unwrap());
        let previous_progress =
            serde_json::to_value(&reopen.preferences.routine_runs[&run_key].progress).unwrap();
        reopen.command(Command::StopRoutine).unwrap();
        let mut append = spec.clone();
        append.routine_id = Some(saved["id"].as_str().unwrap().into());
        append.score_revision = reopen.score_revision;
        append.references = vec![2];
        let b = reopen.build_history_problem_plan(&append).unwrap();
        let before_count = reopen
            .history
            .song(&append.content_id)
            .unwrap()
            .sessions
            .len();
        reopen
            .save_history_problem_plan(&append, &b.revision, "", "", &goal(), &edits(&b))
            .unwrap();
        assert_eq!(
            reopen.preferences.routine_runs[&run_key]
                .routine
                .items
                .len(),
            3
        );
        assert_eq!(reopen.preferences.routines[0].items.len(), 2);
        assert_eq!(
            serde_json::to_value(&reopen.preferences.routine_runs[&run_key].progress).unwrap(),
            previous_progress
        );
        assert_eq!(
            reopen
                .history
                .song(&append.content_id)
                .unwrap()
                .sessions
                .len(),
            before_count
        );
    }
    #[test]
    fn problem_plan_guards_stale_capacity_and_write_failure_are_atomic() {
        let (mut p, mut spec) = source();
        let b = p.build_history_problem_plan(&spec).unwrap();
        let e = edits(&b);
        p.state.latency += 1;
        assert!(
            p.save_history_problem_plan(&spec, &b.revision, "bad", "", &goal(), &e)
                .is_err()
        );
        assert!(p.preferences.routines.is_empty());
        p.state.latency -= 1;
        let original = p.preferences_path.clone();
        p.preferences_path = p.data.clone();
        assert!(
            p.save_history_problem_plan(&spec, &b.revision, "bad", "", &goal(), &e)
                .is_err()
        );
        assert!(p.preferences.routines.is_empty());
        p.preferences_path = original;
        let saved = p
            .save_history_problem_plan(&spec, &b.revision, "existing", "", &goal(), &e)
            .unwrap();
        let id = saved["id"].as_str().unwrap().to_string();
        spec.routine_id = Some(id.clone());
        spec.day = None;
        let b = p.build_history_problem_plan(&spec).unwrap();
        p.preferences.routines[0].name = "其他入口改名".into();
        assert!(
            p.save_history_problem_plan(&spec, &b.revision, "", "", &goal(), &e)
                .is_err()
        );
        let first = p.preferences.routines[0].items[0].clone();
        p.preferences.routines[0].items = vec![first; 199];
        let b = p.build_history_problem_plan(&spec).unwrap();
        let mut changed = edits(&b);
        for e in &mut changed {
            e.title.push_str("新增");
        }
        assert!(
            p.save_history_problem_plan(&spec, &b.revision, "", "", &goal(), &changed)
                .is_err()
        );
        assert_eq!(p.preferences.routines[0].items.len(), 199);
        spec.references = vec![0, 0];
        assert!(p.build_history_problem_plan(&spec).is_err());
        spec.references = vec![999];
        assert!(p.build_history_problem_plan(&spec).is_err());
        spec.references = vec![0];
        spec.score_revision += 1;
        assert!(p.build_history_problem_plan(&spec).is_err());
    }
}
