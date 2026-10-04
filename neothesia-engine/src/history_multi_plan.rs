use super::*;
use crate::history_problem_plan::{Built, ProblemPlanEdit, ProblemPlanSpec};
use crate::routine_commands::RoutineGoal;
use serde_json::{Value, json};
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryProblemSelection {
    pub content_id: String,
    pub id: String,
    pub references: Vec<usize>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MultiProblemSpec {
    pub sources: Vec<HistoryProblemSelection>,
    #[serde(default)]
    pub order: Vec<HistoryProblemSelection>,
    pub before: u8,
    pub after: u8,
    pub mode: String,
    pub speed: f64,
    pub routine_id: Option<String>,
    pub day: Option<String>,
}
impl Player {
    pub(super) fn history_problem_notes(
        &self,
        cid: &str,
        id: &str,
        offset: usize,
    ) -> Result<Value, String> {
        let history = self.history.song(cid).ok_or("历史曲目不存在")?;
        let session = history
            .sessions
            .iter()
            .find(|s| s.stable_id() == id)
            .ok_or("演奏记录已不存在")?;
        if session.context.is_none() {
            return Err("旧记录缺少原条件，不能编排".into());
        }
        let a = crate::performance_archive::read(&self.data, cid, id)?;
        let judgments = a
            .judgments
            .as_ref()
            .ok_or("旧演奏没有逐音判定，不能自动当作问题音")?;
        let problems=judgments.iter().filter_map(|d|d.reference.filter(|_|matches!(d.kind.as_str(),"early"|"late"|"missed")).and_then(|reference|a.references.get(reference).map(|n|json!({"reference":reference,"measure":n.measure,"pitch":n.pitch,"kind":d.kind})))).collect::<Vec<_>>();
        Ok(
            json!({"contentId":cid,"id":id,"title":history.display_name,"recordedAt":session.recorded_at_unix_ms,"total":problems.len(),"offset":offset,"notes":problems.into_iter().skip(offset).take(200).collect::<Vec<_>>()}),
        )
    }
    fn readonly_problem_file(
        &self,
        cid: &str,
        id: &str,
    ) -> Result<(MidiFile, Vec<PracticeNote>, String, String, Value), String> {
        let h = self.history.song(cid).ok_or("历史曲目不存在")?;
        let s = h
            .sessions
            .iter()
            .find(|s| s.stable_id() == id)
            .ok_or("演奏记录已不存在")?;
        let ctx = s.context.as_ref().ok_or("旧记录缺少练习条件")?;
        let (mut file, annotation) = if let Some(spec) = ctx.exercise_spec {
            let plan = ExercisePlan::generate(spec, &KeyboardRange::new(21..=108))
                .map_err(|e| e.to_string())?;
            let mut f = plan.to_midi_file()?;
            f.content_id = plan.practice_id();
            if f.content_id != cid {
                return Err("原生成练习已改变".into());
            }
            (f, self.data.join("song-assets").join(format!("{cid}.mid")))
        } else {
            let f = crate::library::verified_file(
                &self.data,
                cid,
                [
                    ctx.source_path.clone(),
                    h.setup.as_ref().and_then(|s| s.source_path.clone()),
                    h.library.source_path.clone(),
                ]
                .into_iter()
                .flatten(),
            )?;
            let p = f.source_path.clone().ok_or("曲目文件位置缺失")?;
            (f, p)
        };
        let sidecar = match neothesia_core::library::load_song_sidecar(&annotation, cid) {
            Ok(s) => s,
            Err(neothesia_core::library::MetadataError::Read(e))
                if e.kind() == std::io::ErrorKind::NotFound =>
            {
                Default::default()
            }
            Err(e) => return Err(e.to_string()),
        };
        if let Some(m) = sidecar.meter {
            let grid = Self::corrected_grid(&file, m)?;
            file.measures = grid.bar_times(&file.tempo_track).into();
            file.beats = grid.beat_times(&file.tempo_track).into();
            file.musical_time = grid;
        }
        let mut config = SongConfig::new(&file.tracks);
        config.apply_practice_setup(&SongPracticeSetup {
            tracks: ctx.tracks.clone(),
            ..Default::default()
        });
        for t in &mut config.tracks {
            if let Some(p) = ctx.parts.get(&t.track_id) {
                t.practice_part = *p;
            }
        }
        let mut assigned = HashMap::new();
        for hint in &sidecar.hands {
            if file
                .tracks
                .iter()
                .find(|t| t.track_id == hint.track_id)
                .and_then(|t| t.notes.get(hint.note_index))
                .is_none()
            {
                return Err("逐音分手资料没有原曲对应，请重新校对".into());
            }
            assigned.insert((hint.track_id, hint.note_index), hint.part);
        }
        let mut notes: Vec<_> = file
            .tracks
            .iter()
            .filter(|t| !t.has_drums || t.has_other_than_drums)
            .flat_map(|t| {
                t.notes.iter().enumerate().map(|(index, n)| PracticeNote {
                    inner: n.clone(),
                    index,
                })
            })
            .collect();
        notes.sort_by_key(|n| (n.start, n.track_id, n.note));
        let grid = crate::practice_conditions::grid(&file);
        let scope =
            crate::practice_conditions::scope(Some(&file), &notes, &config, &assigned, &ctx.hands);
        let proof = json!([file.content_id, sidecar.hands, sidecar.meter, sidecar.score]);
        Ok((file, notes, grid, scope, proof))
    }
    pub(super) fn build_multi_problem_plan(&self, spec: &MultiProblemSpec) -> Result<Built, String> {
        let count: usize = spec.sources.iter().map(|s| s.references.len()).sum();
        if spec.sources.is_empty()
            || spec.sources.len() > 20
            || count == 0
            || count > 60
            || spec
                .sources
                .iter()
                .map(|s| (&s.content_id, &s.id))
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != spec.sources.len()
        {
            return Err("请选择最多 20 条不同演奏记录，共 1～60 个问题音".into());
        }
        let mut rows = vec![];
        let mut items = vec![];
        let mut proofs = vec![];
        let mut cuts=std::collections::BTreeMap::new();
        for source in &spec.sources {
            let history = self
                .history
                .song(&source.content_id)
                .ok_or("历史曲目不存在")?;
            let (file, notes, grid, scope, proof) = self
                .readonly_problem_file(&source.content_id, &source.id)
                .map_err(|e| format!("{}：{}", history.display_name, e))?;
            let single = ProblemPlanSpec {
                content_id: source.content_id.clone(),
                id: source.id.clone(),
                score_revision: 0,
                references: source.references.clone(),
                before: spec.before,
                after: spec.after,
                mode: spec.mode.clone(),
                speed: spec.speed,
                routine_id: spec.routine_id.clone(),
                day: spec.day.clone(),
            };
            let mut built = self
                .build_history_problem_source(&single, &file, &notes, &grid, &scope, proof)
                .map_err(|e| format!("{}：{}", history.display_name, e))?;
            proofs.push(built.revision);
            for (mut row, mut item) in built.rows.into_iter().zip(built.items) {
                let key = format!("{}:{}:{}", source.content_id, source.id, item.id);
                cuts.insert(key.clone(),built.cuts.remove(&item.id).ok_or("拍位资料缺失")?);
                item.id = key.clone();
                row["key"] = json!(key);
                row["contentId"] = json!(source.content_id);
                row["historyId"] = json!(source.id);
                row["songTitle"] = json!(history.display_name);
                rows.push(row);
                items.push(item);
            }
        }
        if !spec.order.is_empty() {
            let expected = spec
                .sources
                .iter()
                .flat_map(|s| s.references.iter().map(move |r| (&s.content_id, &s.id, *r)))
                .collect::<std::collections::BTreeSet<_>>();
            let actual = spec
                .order
                .iter()
                .flat_map(|s| s.references.iter().map(move |r| (&s.content_id, &s.id, *r)))
                .collect::<Vec<_>>();
            if actual.len() != count
                || actual
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
                    != expected
            {
                return Err("问题音顺序已改变，请重新选择".into());
            }
            let mut paired = rows.into_iter().zip(items).collect::<Vec<_>>();
            paired.sort_by_key(|(row, _)| {
                actual
                    .iter()
                    .position(|(cid, id, r)| {
                        row["contentId"].as_str() == Some(cid.as_str())
                            && row["historyId"].as_str() == Some(id.as_str())
                            && row["references"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|v| v.as_u64() == Some(*r as u64))
                    })
                    .unwrap()
            });
            (rows, items) = paired.into_iter().unzip();
        }
        let revision = blake3::hash(&serde_json::to_vec(&json!([spec, proofs])).unwrap())
            .to_hex()
            .to_string();
        Ok(Built {
            cuts,
            rows,
            items,
            revision,
        })
    }
    pub(super) fn preview_multi_problem_plan(
        &self,
        spec: &MultiProblemSpec,
    ) -> Result<Value, String> {
        let b = self.build_multi_problem_plan(spec)?;
        Ok(
            json!({"rows":b.rows,"revision":b.revision,"selected":spec.sources.iter().map(|s|s.references.len()).sum::<usize>()}),
        )
    }
    pub(super) fn save_multi_problem_plan(
        &mut self,
        spec: &MultiProblemSpec,
        expected: &str,
        name: &str,
        notes: &str,
        goal: &RoutineGoal,
        edits: &[ProblemPlanEdit],
    ) -> Result<Value, String> {
        let b = self.build_multi_problem_plan(spec)?;
        let b = self.prepare_problem_built(b, expected, goal, &spec.mode, edits)?;
        let first = &spec.sources[0];
        let target = ProblemPlanSpec {
            content_id: first.content_id.clone(),
            id: first.id.clone(),
            score_revision: 0,
            references: first.references.clone(),
            before: spec.before,
            after: spec.after,
            mode: spec.mode.clone(),
            speed: spec.speed,
            routine_id: spec.routine_id.clone(),
            day: spec.day.clone(),
        };
        self.save_problem_items(&target, b, expected, name, notes, goal, edits)
    }
}

#[cfg(test)]
mod tests {
 use super::*;
 use crate::history_problem_plan::ProblemPlanEdit;
 fn record(p:&mut Player,step:&str,hand:&str)->HistoryProblemSelection {
  if p.file.is_some(){p.command(Command::Restart).unwrap();}
  let bars=(0..3).map(|i|format!("<measure number=\"{}\">{}<note><pitch><step>{step}</step><octave>4</octave></pitch><duration>1</duration></note></measure>",i+1,if i==0{"<attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes>"}else{""})).collect::<String>();let xml=format!("<score-partwise><part-list><score-part id=\"P1\"><part-name>Piano</part-name></score-part></part-list><part id=\"P1\">{bars}</part></score-partwise>");
  p.command(Command::ImportScore{bytes:xml.into_bytes(),name:format!("{step}.musicxml"),default_bpm:120}).unwrap();p.command(Command::Restart).unwrap();let cid=p.file.as_ref().unwrap().content_id.clone();let track=p.notes[0].track_id;p.command(Command::Track{id:track,mode:"human".into(),part:hand.into(),visible:true}).unwrap();p.command(Command::Mode{value:"recital".into()}).unwrap();p.command(Command::CountIn{bars:0}).unwrap();p.command(Command::Play).unwrap();for _ in 0..50{p.tick(Duration::from_millis(50));}assert_eq!(p.state.status,"finished");let id=p.history.song(&cid).unwrap().sessions.last().unwrap().stable_id();HistoryProblemSelection{content_id:cid,id,references:vec![0]}
 }
 fn goal()->RoutineGoal{RoutineGoal{expression:None,passes:1,accuracy:90,on_time:None,consecutive:false,attempt_limit:3}}
 fn edits(b:&Built)->Vec<ProblemPlanEdit>{b.rows.iter().map(|r|ProblemPlanEdit{cut:None,key:r["key"].as_str().unwrap().into(),title:r["title"].as_str().unwrap().into(),notes:"各曲重点".into()}).collect()}
 fn player()->Player {Player::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../work").join(format!("cycle180-unit-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())),PathBuf::new(),true)}
 #[test]
 fn cross_song_preview_is_readonly_ordered_and_daily_plan_restores_each_scope(){
  let mut p=player();let mut a=record(&mut p,"C","right");let b=record(&mut p,"D","left");a.references=vec![0,2];let spec=MultiProblemSpec{sources:vec![a.clone(),b.clone()],order:vec![HistoryProblemSelection{references:vec![0],..a.clone()},b.clone(),HistoryProblemSelection{references:vec![2],..a.clone()}],before:0,after:0,mode:"wait".into(),speed:0.7,routine_id:None,day:Some("2026-10-02".into())};
  let position=p.state.position;let cid=p.file.as_ref().unwrap().content_id.clone();let score=serde_json::to_value(p.matcher.snapshot()).unwrap();let prefs=std::fs::read(&p.preferences_path).unwrap();let hist=std::fs::read(p.data.join("web-practice-history.ron")).unwrap();let built=p.build_multi_problem_plan(&spec).unwrap();assert_eq!(built.rows.len(),3);assert_eq!(built.rows[0]["contentId"],a.content_id);assert_eq!(built.rows[1]["contentId"],b.content_id);assert_eq!(built.rows[2]["start"],3);assert_eq!(p.state.position,position);assert_eq!(p.file.as_ref().unwrap().content_id,cid);assert_eq!(std::fs::read(&p.preferences_path).unwrap(),prefs);assert_eq!(std::fs::read(p.data.join("web-practice-history.ron")).unwrap(),hist);
  let saved=p.save_multi_problem_plan(&spec,&built.revision,"两曲问题","交错重练",&goal(),&edits(&built)).unwrap();assert_eq!(saved["added"],3);assert_eq!(serde_json::to_value(p.matcher.snapshot()).unwrap(),score);assert_eq!(p.state.position,position);
  let retry=p.save_multi_problem_plan(&spec,&built.revision,"两曲问题","交错重练",&goal(),&edits(&built)).unwrap();assert_eq!(retry["id"],saved["id"]);assert_eq!(retry["added"],0);
  let backup:crate::practice_backup::Backup=serde_json::from_value(p.practice_backup_snapshot(&crate::practice_backup::Selection::default()).unwrap()).unwrap();let backup=crate::practice_backup::Backup::decode(&backup.encode().unwrap()).unwrap();assert_eq!(backup.routines[0].items.len(),3);
  let id=saved["id"].as_str().unwrap().to_string();let mut restored=Player::new(p.data.clone(),PathBuf::new(),true);restored.restore();let first=restored.preferences.routines[0].items[0].id.clone();restored.command(Command::OpenRoutineItem{routine_id:id.clone(),day:"2026-10-02".into(),item_id:first,resume:true}).unwrap();assert_eq!(restored.file.as_ref().unwrap().content_id,a.content_id);assert!(restored.config.tracks.iter().any(|t|t.practice_part==PracticePart::RightHand&&t.player==PlayerConfig::Human));restored.command(Command::Play).unwrap();for _ in 0..250{restored.tick(Duration::from_millis(10));for pitch in restored.snapshot().required{restored.command(Command::Note{pitch,velocity:90,active:true}).unwrap();restored.command(Command::Note{pitch,velocity:0,active:false}).unwrap();}if restored.routine_status().unwrap()["progress"]["completed"]==true{break;}}assert_eq!(restored.routine_status().unwrap()["progress"]["completed"],true);let next=restored.routine_status().unwrap()["nextItemId"].as_str().unwrap().to_string();restored.command(Command::OpenRoutineItem{routine_id:id,day:"2026-10-02".into(),item_id:next,resume:true}).unwrap();assert_eq!(restored.file.as_ref().unwrap().content_id,b.content_id);assert!(restored.config.tracks.iter().any(|t|t.practice_part==PracticePart::LeftHand&&t.player==PlayerConfig::Human));
 }
 #[test]
 fn multi_records_same_song_stale_grid_and_missing_source_reject_whole_save(){
  let mut p=player();let a=record(&mut p,"C","right");let b=record(&mut p,"C","right");assert_eq!(a.content_id,b.content_id);assert_ne!(a.id,b.id);let mut spec=MultiProblemSpec{sources:vec![a,b],order:vec![],before:0,after:0,mode:"wait".into(),speed:0.7,routine_id:None,day:None};let built=p.build_multi_problem_plan(&spec).unwrap();assert_eq!(built.rows.len(),2);assert_ne!(built.rows[0]["key"],built.rows[1]["key"]);
  p.command(Command::Restart).unwrap();p.command(Command::Meter{value:Some(neothesia_core::library::MeterCorrection{numerator:3,denominator:4,pickup_ticks:0})}).unwrap();assert!(p.save_multi_problem_plan(&spec,&built.revision,"bad","",&goal(),&edits(&built)).is_err());assert!(p.preferences.routines.is_empty());p.command(Command::Meter{value:None}).unwrap();let built=p.build_multi_problem_plan(&spec).unwrap();let path=built.items[0].path.clone().unwrap();std::fs::rename(&path,path.with_extension("missing")).unwrap();assert!(p.save_multi_problem_plan(&spec,&built.revision,"bad","",&goal(),&edits(&built)).is_err());assert!(p.preferences.routines.is_empty());spec.sources.push(spec.sources[0].clone());assert!(p.build_multi_problem_plan(&spec).is_err());
 }
}
