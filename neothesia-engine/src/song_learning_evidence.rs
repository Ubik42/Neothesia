use crate::{library_workspace::Workspace, preferences::Preferences, routine_commands::RoutineTarget, song_learning::Draft};
use chrono::{Datelike, Duration, Local, TimeZone, Utc};
use neothesia_core::practice_history::{PracticeHistoryStore, PracticeSessionKind};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path, time::{SystemTime, UNIX_EPOCH}};

const DAY_MS: u64 = 86_400_000;
const RECENT_MS: u64 = 90 * DAY_MS;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekEvidence {
    pub from: u64,
    pub to: u64,
    pub running_ms: u64,
    pub target_minutes: Option<u16>,
    pub met: bool,
    pub reasons: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WholeAttempt {
    pub session_id: String,
    pub recorded_at: u64,
    pub scope: String,
    pub accuracy_percent: f32,
    pub effective_bpm: Option<u16>,
    pub meets_target_bpm: bool,
    pub readable: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WholeEvidence {
    pub recent: Vec<WholeAttempt>,
    pub comparable_count: usize,
    pub target_bpm_met: bool,
    pub reasons: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutineItemEvidence {
    pub routine_id: String,
    pub day: String,
    pub item_id: String,
    pub title: String,
    pub target_kind: String,
    pub completed: bool,
    pub skipped: bool,
    pub session_ids: Vec<String>,
    pub readable_session_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutineEvidence {
    pub items: Vec<RoutineItemEvidence>,
    pub completed_count: usize,
    pub reasons: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalRef { pub session_id: String, pub readable: bool }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub content_id: String,
    pub generated_at: u64,
    pub week: WeekEvidence,
    pub whole_song: WholeEvidence,
    pub routines: RoutineEvidence,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub historical_refs: Vec<HistoricalRef>,
    pub reasons: Vec<String>,
}

pub fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().min(u128::from(u64::MAX)) as u64 }

fn local_week(now:u64)->(u64,u64) {
    let instant=Utc.timestamp_millis_opt(now.min(i64::MAX as u64) as i64).single().unwrap_or_else(Utc::now).with_timezone(&Local);
    let start_day=instant.date_naive()-Duration::days(i64::from(instant.weekday().num_days_from_monday()));
    let local_midnight=|day:chrono::NaiveDate| {let naive=day.and_hms_opt(0,0,0).unwrap();Local.from_local_datetime(&naive).earliest().or_else(||Local.from_local_datetime(&naive).latest()).map(|at|at.timestamp_millis().max(0) as u64)};
    let from=local_midnight(start_day).unwrap_or_else(||now/DAY_MS*DAY_MS);
    let to=local_midnight(start_day+Duration::days(7)).unwrap_or_else(||from.saturating_add(7*DAY_MS));
    (from,to)
}

/// Every value comes from persisted files. An edited draft is supplied only while saving its transition.
pub fn read(data: &Path, content_id: &str) -> Result<Evidence, String> {
    let workspace = Workspace::load(data)?;
    let draft = workspace.learning.get(content_id).map(|entry| &entry.plan);
    let mut evidence = build(data, content_id, draft, now_ms())?;
    if let Some(learning) = workspace.learning.get(content_id) {
        let history = PracticeHistoryStore::load(data.join("web-practice-history.ron"));
        let readable: BTreeSet<_> = history.song(content_id).into_iter().flat_map(|song| &song.sessions).map(|session|session.stable_id()).collect();
        let ids: BTreeSet<_> = learning.transitions.iter().filter_map(|t|t.evidence.as_ref()).flat_map(|snapshot| {
            snapshot.whole_song.recent.iter().map(|row|row.session_id.clone()).chain(snapshot.routines.items.iter().flat_map(|item|item.session_ids.clone()))
        }).collect();
        evidence.historical_refs = ids.into_iter().map(|session_id| HistoricalRef { readable: readable.contains(&session_id), session_id }).collect();
    }
    Ok(evidence)
}

pub fn build(data: &Path, content_id: &str, draft: Option<&Draft>, now: u64) -> Result<Evidence, String> {
    if content_id.len()!=64 || !content_id.bytes().all(|byte|byte.is_ascii_hexdigit()) {return Err("曲目内容身份无效".into());}
    let history = PracticeHistoryStore::load(data.join("web-practice-history.ron"));
    let preferences = Preferences::load(&data.join("web-preferences.json"))?;
    Ok(collect(&history,&preferences,content_id,draft,now))
}

fn collect(history: &PracticeHistoryStore, preferences: &Preferences, content_id: &str, draft: Option<&Draft>, now: u64) -> Evidence {
    // Match the browser's local Monday-to-Monday window, including DST transitions.
    let (monday,week_end)=local_week(now);
    let mut running_ms = 0u64;
    if let Some(entries)=history.time_ledger().get(content_id) {
        for entry in entries.values() { for slice in &entry.slices { running_ms=running_ms.saturating_add(slice.intersection_ms(monday,week_end.min(now))); } }
    }
    let target_minutes=draft.and_then(|value|value.weekly_minutes);
    let week_met=target_minutes.is_some_and(|minutes|running_ms >= u64::from(minutes)*60_000);
    let mut week_reasons=Vec::new();
    if target_minutes.is_none(){week_reasons.push("尚未设置每周练习分钟目标".into());}
    else if !week_met {week_reasons.push("本周已记录的实际运行时间未达到目标".into());}
    if running_ms==0 {week_reasons.push("本周没有已保存的实际运行时间".into());}
    let week=WeekEvidence{from:monday,to:week_end,running_ms,target_minutes,met:week_met,reasons:week_reasons};

    let sessions=history.song(content_id).map(|song|song.sessions.as_slice()).unwrap_or(&[]);
    let readable: BTreeSet<_>=sessions.iter().map(|s|s.stable_id()).collect();
    let mut candidates:Vec<_>=sessions.iter().filter(|session| {
        matches!(session.kind,PracticeSessionKind::WholeSong)
        && session.recorded_at_unix_ms<=now && now-session.recorded_at_unix_ms<=RECENT_MS
        && matches!(session.mode.as_deref(),Some("flow"|"recital"|"memory"))
        && session.scope.as_ref().is_some_and(|scope|!scope.is_empty())
        && session.context.as_ref().is_some_and(|ctx|ctx.target_notes>0 && ctx.judged_notes>=ctx.target_notes && ctx.exercise_spec.is_none())
        && session.summary.overall.accuracy().is_some()
    }).collect();
    candidates.sort_by_key(|session|std::cmp::Reverse(session.recorded_at_unix_ms));
    let anchor=candidates.first().copied();
    let comparable:Vec<_>=candidates.into_iter().filter(|session|anchor.is_some_and(|current|crate::practice_conditions::same_attempt_conditions(current,session))).collect();
    let comparable_count=comparable.len();
    let target_bpm=draft.and_then(|value|value.target_bpm);
    let target_bpm_met=target_bpm.is_some_and(|bpm|comparable.iter().any(|session|session.effective_tempo_bpm.is_some_and(|value|value>=bpm)));
    let recent=comparable.into_iter().take(8).map(|session|WholeAttempt{session_id:session.stable_id(),recorded_at:session.recorded_at_unix_ms,scope:session.scope.clone().unwrap_or_default(),accuracy_percent:session.summary.overall.accuracy().unwrap_or_default()*100.,effective_bpm:session.effective_tempo_bpm,meets_target_bpm:target_bpm.is_some_and(|bpm|session.effective_tempo_bpm.is_some_and(|value|value>=bpm)),readable:true}).collect();
    let mut whole_reasons=Vec::new();
    if comparable_count==0 {whole_reasons.push("近 90 天没有条件相同且完整弹完的整曲成绩；选段和中途保存不计入".into());}
    if target_bpm.is_none(){whole_reasons.push("尚未设置整曲目标速度".into());}
    else if !target_bpm_met{whole_reasons.push("近期完整整曲成绩尚未达到目标速度，或旧记录缺少有效 BPM".into());}
    let whole_song=WholeEvidence{recent,comparable_count,target_bpm_met,reasons:whole_reasons};

    let mut routine_items=Vec::new();
    for run in preferences.routine_runs.values(){
        for item in &run.routine.items {if item.content_id!=content_id{continue;}
            let progress=run.progress.get(&item.id);
            let session_ids:Vec<String>=progress.map(|row|row.session_ids.iter().rev().take(8).cloned().collect::<Vec<_>>().into_iter().rev().collect()).unwrap_or_default();
            let readable_session_ids=session_ids.iter().filter(|id|{
                readable.contains(*id) && sessions.iter().any(|session|session.stable_id()==**id && session.context.as_ref().and_then(|ctx|ctx.routine.as_ref()).is_some_and(|condition|condition.routine_id==run.routine.id && condition.day==run.day && condition.item_id==item.id))
            }).cloned().collect::<Vec<_>>();
            routine_items.push(RoutineItemEvidence{routine_id:run.routine.id.clone(),day:run.day.clone(),item_id:item.id.clone(),title:item.title.clone(),target_kind:match &item.target{RoutineTarget::Whole=>"whole",RoutineTarget::Ladder{..}=>"ladder",_=>"passage"}.into(),completed:progress.is_some_and(|p|p.completed),skipped:progress.is_some_and(|p|p.skipped),session_ids,readable_session_ids});
        }
    }
    routine_items.sort_by(|a,b|b.day.cmp(&a.day).then_with(||a.routine_id.cmp(&b.routine_id)).then_with(||a.item_id.cmp(&b.item_id)));
    let completed_count=routine_items.iter().filter(|item|item.completed&&!item.skipped).count();
    let mut routine_reasons=Vec::new();
    if routine_items.is_empty(){routine_reasons.push("日期计划里还没有这首曲目的项目".into());}
    else if completed_count==0{routine_reasons.push("日期计划里的同曲项目尚未达标".into());}
    if routine_items.iter().any(|item|item.completed&&item.readable_session_ids.is_empty()) {routine_reasons.push("部分已达标项目的历史成绩已经清理；计划结果仍保留，但当前无法打开原成绩".into());}
    routine_items.truncate(32);
    let routines=RoutineEvidence{items:routine_items,completed_count,reasons:routine_reasons};
    let reasons=week.reasons.iter().chain(whole_song.reasons.iter()).chain(routines.reasons.iter()).cloned().collect();
    Evidence{content_id:content_id.into(),generated_at:now,week,whole_song,routines,historical_refs:Vec::new(),reasons}
}

#[cfg(test)]
mod tests {
    use super::*;
    use neothesia_core::{practice::{AttemptSummary, PracticeHands, PracticeSnapshot}, practice_history::{PracticeSession, PracticeSessionContext, PracticeTimeEntry, PracticeTimeSlice}};
    use std::collections::BTreeMap;

    fn session(kind:PracticeSessionKind,id:&str,at:u64,judged:usize,scope:&str,bpm:Option<u16>)->PracticeSession {
        let summary=AttemptSummary{overall:PracticeSnapshot{matched_notes:judged,required_notes:10,..Default::default()},..Default::default()};
        let mut s=PracticeSession::new(kind,PracticeHands::Both,1.,summary);
        s.id=Some(id.into());s.recorded_at_unix_ms=at;s.mode=Some("flow".into());s.scope=Some(scope.into());s.effective_tempo_bpm=bpm;
        s.context=Some(PracticeSessionContext{routine:None,ladder:None,target_notes:10,judged_notes:judged,tracks:vec![],parts:BTreeMap::new(),hands:"both".into(),count_in:0,metronome:false,latency:0,pedal_latency_ms:None,rounds:1,adaptive:false,grid:"grid".into(),source_path:None,exercise_spec:None});
        s
    }
    #[test]
    fn weekly_time_whole_comparability_and_partial_exclusion_are_identity_scoped() {
        let root=std::env::temp_dir().join(format!("learning-evidence-{}",now_ms()));std::fs::create_dir_all(&root).unwrap();
        let cid="a".repeat(64);let other="b".repeat(64);let now=20_000*DAY_MS+12*3_600_000;
        let mut history=PracticeHistoryStore::load(root.join("web-practice-history.ron"));
        let monday=local_week(now).0+8*3_600_000;
        let record=|h:&mut PracticeHistoryStore,s:PracticeSession,from:u64,to:u64|{h.record_session_with_time(&cid,"一首曲",s,PracticeTimeEntry{saved_at_unix_ms:to,slices:vec![PracticeTimeSlice{from,to,running_ms:to-from}],untracked_ms:0}).unwrap();};
        record(&mut history,session(PracticeSessionKind::Loop{start_measure:1,end_measure:2},"partial",now-8000,10,"same",Some(150)),monday+1000,monday+61_000);
        record(&mut history,session(PracticeSessionKind::WholeSong,"incomplete",now-7000,5,"same",Some(140)),monday+61_000,monday+121_000);
        record(&mut history,session(PracticeSessionKind::WholeSong,"whole-a",now-6000,10,"same",Some(100)),monday+121_000,monday+181_000);
        record(&mut history,session(PracticeSessionKind::WholeSong,"whole-b",now-5000,10,"same",Some(120)),monday+181_000,monday+241_000);
        record(&mut history,session(PracticeSessionKind::WholeSong,"different",now-4000,10,"different",Some(180)),monday+241_000,monday+301_000);
        history.record_session_with_time(&other,"另一个曲目",session(PracticeSessionKind::WholeSong,"other",now-3000,10,"same",Some(200)),PracticeTimeEntry{saved_at_unix_ms:now,slices:vec![PracticeTimeSlice{from:monday+301_000,to:monday+361_000,running_ms:60_000}],untracked_ms:0}).unwrap();
        let draft=Draft{stage:"learning".into(),goal:String::new(),due:None,weekly_minutes:Some(5),target_bpm:Some(110)};
        let result=collect(&history,&Preferences::default(),&cid,Some(&draft),now);
        assert_eq!(result.week.running_ms,300_000);assert!(result.week.met);
        assert_eq!(result.whole_song.comparable_count,1);assert_eq!(result.whole_song.recent[0].session_id,"different");assert!(result.whole_song.target_bpm_met);
        let earlier=collect(&history,&Preferences::default(),&cid,Some(&draft),now-4500);
        assert_eq!(earlier.whole_song.comparable_count,1);
        assert_eq!(earlier.whole_song.recent[0].session_id,"whole-b");
        assert!(earlier.whole_song.target_bpm_met);
        let other_scope=collect(&history,&Preferences::default(),&other,Some(&draft),now);
        assert_eq!(other_scope.week.running_ms,60_000);assert!(!other_scope.week.met);
        assert_eq!(other_scope.whole_song.comparable_count,1);
        let past=collect(&history,&Preferences::default(),&cid,Some(&draft),now+RECENT_MS+1);
        assert_eq!(past.whole_song.comparable_count,0);
    }

    #[test]
    fn dated_routine_and_saved_stage_keep_evidence_when_old_result_is_trimmed() {
        use crate::{library_workspace::Workspace, song_learning::{Learning, Transition}};
        let root=std::env::temp_dir().join(format!("learning-routine-{}",now_ms()));std::fs::create_dir_all(&root).unwrap();
        let cid="c".repeat(64);let now=now_ms();let day="2026-10-12";
        let mut history=PracticeHistoryStore::load(root.join("web-practice-history.ron"));
        let mut first=session(PracticeSessionKind::WholeSong,"attested",now-1000,10,"both",Some(120));
        first.context.as_mut().unwrap().routine=Some(neothesia_core::practice_history::PracticeRoutineCondition{expression:None,item_notes:String::new(),consecutive:false,routine_id:"r".into(),routine_name:"日课".into(),day:day.into(),item_id:"i".into(),item_title:"整曲".into(),passes_required:1,accuracy_percent:90,on_time_percent:None});
        history.record_session(&cid,"测试",first).unwrap();
        let mut prefs:Preferences=serde_json::from_value(serde_json::json!({"routineRuns":{format!("r:{day}"):{"day":day,"routine":{"id":"r","name":"日课","notes":"","items":[{"id":"i","title":"整曲","notes":"","contentId":cid,"songTitle":"测试","path":null,"exercise":null,"grid":"g","scope":"both","settings":{},"speed":1.0,"target":{"kind":"whole"},"goal":{"passes":1,"accuracy":90,"onTime":null,"consecutive":false,"attemptLimit":5}}]},"progress":{},"lastItem":null}}})).unwrap();
        prefs.routine_runs.get_mut(&format!("r:{day}")).unwrap().progress.insert("i".into(),crate::routine_commands::RoutineProgress{completed:true,session_ids:vec!["attested".into()],..Default::default()});
        let draft=Draft{stage:"polishing".into(),goal:"稳定整曲".into(),due:None,weekly_minutes:Some(30),target_bpm:Some(110)};
        let before=collect(&history,&prefs,&cid,Some(&draft),now);
        assert_eq!(before.routines.completed_count,1);
        assert_eq!(before.routines.items[0].readable_session_ids,["attested"]);
        assert_eq!(before.whole_song.recent[0].session_id,"attested");
        let transition=Transition{stage:"polishing".into(),at:now/1000,plan:Some(draft.clone()),evidence:Some(before)};
        let mut workspace=Workspace::default();workspace.learning.insert(cid.clone(),Learning{plan:draft,updated_at:now/1000,title:"测试".into(),transitions:vec![transition]});workspace.save(&root).unwrap();
        for i in 0..201{history.record_session(&cid,"测试",session(PracticeSessionKind::Loop{start_measure:1,end_measure:1},&format!("filler-{i}"),now+i,10,"both",None)).unwrap();}
        assert!(!history.song(&cid).unwrap().sessions.iter().any(|session|session.stable_id()=="attested"));
        prefs.save(&root.join("web-preferences.json")).unwrap();
        let current=read(&root,&cid).unwrap();
        assert_eq!(current.routines.completed_count,1);
        assert!(current.routines.items[0].readable_session_ids.is_empty());
        assert_eq!(current.historical_refs.iter().find(|entry|entry.session_id=="attested").unwrap().readable,false);
        let reopened=Workspace::load(&root).unwrap();
        assert_eq!(reopened.learning[&cid].transitions[0].evidence.as_ref().unwrap().whole_song.recent[0].session_id,"attested");
    }
}
