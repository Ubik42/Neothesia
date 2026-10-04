use super::*;
use crate::fingerings::{PlanRequest,Edit};
#[derive(Deserialize,Serialize)]
pub struct PairPart {pub request:PlanRequest,pub fingerprint:String,pub edits:Vec<Edit>}
impl Player {
 pub(super) fn finger_pair(&mut self,parts:&[PairPart],proof:Option<&str>,acknowledged:bool)->Result<serde_json::Value,String>{
  if parts.len()!=2 || parts[0].request.hand==parts[1].request.hand || parts.iter().any(|p|!matches!(p.request.hand,PracticePart::LeftHand|PracticePart::RightHand)) || parts[0].request.range.is_none() || parts[0].request.range!=parts[1].request.range || parts[0].request.content_id!=parts[1].request.content_id {return Err("双手方案需要同曲、相同小节范围和各一份左右手方案".into());}
  let file=self.file.as_ref().ok_or("请先打开曲目")?;
  let mut reviews=vec![];let mut prepared=vec![];let mut identities=std::collections::HashSet::new();let mut edits=vec![];
  for part in parts {
   let context=crate::fingerings::prepare(file,&self.config,&self.fingers,&self.note_hands,part.request.clone())?;
   if context.fingerprint!=part.fingerprint{return Err("音符、分手或已有指法已变化，请重新生成双手方案".into());}
   let review=crate::fingerings::review_acceptance(&context,&part.edits)?;
   for e in &part.edits {if !identities.insert((e.track_id,e.note_index)){return Err("双手方案包含重复音符，未保存".into());}edits.push(e.clone());}
   reviews.push(review);prepared.push(context);
  }
  let (first,last)=parts[0].request.range.unwrap();let start=file.tempo_track.pulses_to_duration(file.musical_time.measures[first-1].start_tick);let end=file.tempo_track.pulses_to_duration(file.musical_time.measures[last-1].end_tick);
  if parts[0].request.practice_rate!=parts[1].request.practice_rate{return Err("双手方案需要使用相同的推荐速度".into());}
  let left_request=&parts.iter().find(|p|p.request.hand==PracticePart::LeftHand).unwrap().request;
  let right_request=&parts.iter().find(|p|p.request.hand==PracticePart::RightHand).unwrap().request;
  let left=crate::finger_pair_interactions::collect(file,&self.config,&self.note_hands,left_request,start.as_secs_f64(),end.as_secs_f64())?;
  let right=crate::finger_pair_interactions::collect(file,&self.config,&self.note_hands,right_request,start.as_secs_f64(),end.as_secs_f64())?;
  let interactions=crate::finger_pair_interactions::analyze(&left,&right,start.as_secs_f64(),end.as_secs_f64(),parts[0].request.practice_rate);
  let unassigned=file.tracks.iter().flat_map(|t|t.notes.iter().enumerate().map(move |(i,n)|(t.track_id,i,n))).filter(|(t,i,n)|n.start>=start&&n.start<end&&!matches!(hands::part(&self.config,&self.note_hands,*t,*i),PracticePart::LeftHand|PracticePart::RightHand)).count();
  let mut shared=vec![];
  for a in prepared[0].positions.iter().filter(|p|p.target){for b in prepared[1].positions.iter().filter(|p|p.target){if a.pitch==b.pitch&&(a.onset-b.onset).abs()<0.000001{if shared.len()<100{shared.push(serde_json::json!({"measure":a.measure,"pitch":a.pitch,"leftTrack":a.track,"rightTrack":b.track}));}}}}
  let valid=reviews.iter().all(|r|r["valid"]==true)&&edits.len()<=4096;
  let needs_ack=interactions.same_key>0||interactions.crossed_range>0||interactions.truncated||unassigned>0||!shared.is_empty()||reviews.iter().any(|r|r["selected"]!=r["total"]||r["contextUnmarked"].as_u64().unwrap_or(0)>0);
  let practice_settings=self.finger_pair_settings(parts);
  let bytes=serde_json::to_vec(&serde_json::json!([parts,reviews,unassigned,shared,interactions,self.saved_hints()?,self.saved_actions()?,practice_settings])).map_err(|e|e.to_string())?;let signature=blake3::hash(&bytes).to_hex().to_string();
  if let Some(proof)=proof {
   if self.state.recording||self.routine.is_some()||self.ladder.as_ref().is_some_and(|l|l.active)||matches!(self.state.status.as_str(),"playing"|"countIn")||!self.state.pressed.is_empty(){return Err("请先停止演奏、录音和活动计划，并松开按键，再保存双手指法".into());}
   if proof!=signature{return Err("双手审阅已过时，请重新核对".into());}
   if !valid{return Err("一只手的指法存在冲突或超过数量上限，双手均未保存".into());}
   if needs_ack&&!acknowledged{return Err("请确认未选建议、未分手音符和双手同音提示后保存".into());}
   self.command(Command::EditFingersFor{content_id:parts[0].request.content_id.clone(),edits})?;
   return Ok(serde_json::json!({"saved":true,"hands":2}));
  }
  Ok(serde_json::json!({"proof":signature,"reviews":reviews,"valid":valid,"needsAck":needs_ack,"unassigned":unassigned,"shared":shared,"selected":edits.len(),"interactions":interactions}))
 }
 pub(super) fn start_pair_demo(&mut self,parts:&[PairPart],proof:&str,acknowledged:bool,rate:f64)->Result<serde_json::Value,String>{
  if !rate.is_finite()||!(0.25..=1.).contains(&rate){return Err("示范速度应为原速的 25% 至 100%".into());}
  if self.recorder.is_recording()||self.routine.is_some()||self.ladder.as_ref().is_some_and(|l|l.active)||matches!(self.state.status.as_str(),"playing"|"countIn")||!self.state.pressed.is_empty(){return Err("请先停止练习和录音，并松开琴键，再开始双手示范".into());}
  let review=self.finger_pair(parts,None,false)?;
  if review["proof"].as_str()!=Some(proof){return Err("双手审阅已过时，请重新核对".into());}
  if review["valid"]!=true||review["needsAck"]==true&&!acknowledged{return Err("请先完成双手结果审阅与确认，再开始示范".into());}
  let current=self.command(Command::CurrentSong)?;let stored=self.saved_actions()?;let validity=self.check_action_groups(&current,&stored,rate);
  let file=self.file.as_ref().ok_or("请先打开曲目")?;let (first,last)=parts[0].request.range.unwrap();
  let start=file.tempo_track.pulses_to_duration(file.musical_time.measures[first-1].start_tick).as_secs_f64();
  let end=file.tempo_track.pulses_to_duration(file.musical_time.measures[last-1].end_tick).as_secs_f64();
  let mut notes=vec![];let mut events=vec![];let mut ids=std::collections::HashSet::new();
  for part in parts {
   let prepared=crate::fingerings::prepare(file,&self.config,&self.fingers,&self.note_hands,part.request.clone())?;
   for p in prepared.positions.iter().filter(|p|p.onset<end&&p.end>start){
    if !ids.insert((p.track,p.index)){continue;}
    let finger=part.edits.iter().find(|e|e.track_id==p.track&&e.note_index==p.index).map(|e|e.finger).unwrap_or(p.saved);
    let n=file.tracks.iter().find(|t|t.track_id==p.track).and_then(|t|t.notes.get(p.index)).ok_or("示范音符已变化")?;
    notes.push(crate::held_demo::HeldNote{track:p.track,index:p.index,pitch:p.pitch,part:crate::hands::label(part.request.hand).into(),onset:p.onset,end:p.end,finger});
    events.push((p.onset.max(start),p.track,[144|n.channel,n.note,n.velocity.max(1)]));events.push((p.end.min(end),p.track,[128|n.channel,n.note,0]));
   }
  }
  if notes.is_empty()||notes.len()>9216{return Err("双手示范音符为空或超过 9216 个，请缩小范围".into());}
  let mut actions=vec![];
  for (i,a) in stored.iter().enumerate(){
   let Some(n)=notes.iter().find(|n|n.track==a.track_id&&n.index==a.note_index) else{continue;};
   let at=file.tempo_track.pulses_to_duration(a.at_tick).as_secs_f64();if at>=end{continue;}
   let original=current["notes"].as_array().unwrap().iter().find(|n|n["track"].as_u64()==Some(a.track_id as u64)&&n["index"].as_u64()==Some(a.note_index as u64)).and_then(|n|n["finger"].as_u64()).map(|f|f as u8);
   if !validity[i]||n.finger!=original{return Err("选段的持音换指与当前指法或示范速度不匹配，请先保存指法并重新审阅动作".into());}
   actions.push(crate::held_demo::HeldChange{track:a.track_id,index:a.note_index,at,at_tick:a.at_tick,from:a.from,to:a.to});
  }
  let context=crate::held_demo::HeldContext{baseline:proof.into(),content_id:parts[0].request.content_id.clone(),source_track:parts[0].request.track,source_index:parts[0].request.index,start,end,notes,actions};
  self.begin_finger_demo(start,end,rate,events,Some(context))
 }
 fn finger_pair_settings(&self,parts:&[PairPart])->preferences::SessionSettings{
  use neothesia_core::practice_history::PracticeTrackMode;
  let targets=parts.iter().flat_map(|p|crate::fingerings::target_tracks(&p.request)).collect::<std::collections::HashSet<_>>();
  let mut tracks=self.config.practice_track_setup();for t in &mut tracks {t.mode=if targets.contains(&t.track_id){PracticeTrackMode::Human}else if t.mode==PracticeTrackMode::Mute{PracticeTrackMode::Mute}else{PracticeTrackMode::Auto};}
  preferences::SessionSettings{tracks,parts:self.config.tracks.iter().map(|t|(t.track_id,t.practice_part)).collect(),mode:Some("wait".into()),count_in:self.state.count_in.max(1),metronome:self.state.metronome,latency:self.state.latency,rounds:1,hands:"both".into(),adaptive:false}
 }
 pub(super) fn save_pair_practice(&mut self,parts:&[PairPart],proof:&str,acknowledged:bool,ranges:Vec<crate::finger_practice::PracticeRange>)->Result<serde_json::Value,String>{
  if self.state.recording||self.routine.is_some()||self.ladder.as_ref().is_some_and(|l|l.active)||matches!(self.state.status.as_str(),"playing"|"countIn")||!self.state.pressed.is_empty(){return Err("请先停止演奏、录音和活动计划，再保存双手练习段".into());}
  let review=self.finger_pair(parts,None,false)?;
  if review["proof"].as_str()!=Some(proof){return Err("双手条件或指法已改变，请重新核对保存结果".into());}
  if review["valid"]!=true||review["needsAck"]==true&&!acknowledged {return Err("请先完成双手结果审阅与确认，再建立练习段".into());}
  if ranges.is_empty()||ranges.len()>32{return Err("每次请选择 1～32 个双手练习段".into());}
  let file=self.file.as_ref().ok_or("请先打开曲目")?;let cid=file.content_id.clone();let targets=parts.iter().flat_map(|p|crate::fingerings::target_tracks(&p.request)).collect::<std::collections::HashSet<_>>();
  for r in &ranges {
   if r.name.trim().is_empty()||r.name.chars().count()>80||r.notes.len()>8192||r.start==0||r.start>r.end||r.end>file.musical_time.measures.len()||!r.speed.is_finite()||!(0.25..=2.).contains(&r.speed)||!(1..=99).contains(&r.rounds){return Err("双手练习段的名称、小节、速度或重复次数无效，整批未保存".into());}
   let start=file.tempo_track.pulses_to_duration(file.musical_time.measures[r.start-1].start_tick);let end=file.tempo_track.pulses_to_duration(file.musical_time.measures[r.end-1].end_tick);
   let (a,b)=parts[0].request.range.unwrap();if r.end<a||r.start>b{return Err("双手练习段需要包含本次共同审阅范围内的目标音".into());}
   let mut seen=std::collections::HashSet::new();let mut has_review_target=false;
   for t in file.tracks.iter().filter(|t|targets.contains(&t.track_id)){for (i,n) in t.notes.iter().enumerate().filter(|(_,n)|n.start>=start&&n.start<end){let hand=hands::part(&self.config,&self.note_hands,t.track_id,i);if hand==PracticePart::Other{return Err(format!("第 {}～{} 小节有未分手目标音，请先完成分手再生成双手练习段",r.start,r.end));}let measure=file.measures.partition_point(|time|*time<=n.start).max(1);if measure>=a&&measure<=b&&parts.iter().any(|p|p.request.hand==hand&&crate::fingerings::target_tracks(&p.request).contains(&t.track_id)){has_review_target=true;}seen.insert(hand);}}
   if seen.is_empty()||!has_review_target{return Err("练习段没有本次双手声部的审阅目标音".into());}
  }
  let settings=self.finger_pair_settings(parts);let grid=self.grid_signature();let mut prefs=self.preferences.clone();let rows=prefs.passages.entry(cid).or_default();if rows.len()>200{return Err("每首曲目最多保存 200 个练习段，整批未保存".into());}
  let stamp=blake3::hash(&serde_json::to_vec(&serde_json::json!([proof,&ranges])).map_err(|e|e.to_string())?).to_hex().to_string();let mut ids=vec![];let mut existing=0;
  for (i,r) in ranges.into_iter().enumerate(){let id=format!("finger-pair-practice-{stamp}-{i}");let mut settings=settings.clone();settings.rounds=r.rounds;let preset=preferences::PassagePreset{id:id.clone(),name:r.name.trim().into(),start:r.start,end:r.end,notes:r.notes,speed:r.speed,settings,grid:Some(grid.clone()),score_range:None,precise_range:None};if let Some(old)=rows.iter().find(|p|p.id==id){if serde_json::to_value(old).unwrap()!=serde_json::to_value(&preset).unwrap(){return Err("对应练习段后来已修改，请重新命名后保存".into());}existing+=1;}else{if rows.len()>=200{return Err("每首曲目最多保存 200 个练习段，整批未保存".into());}rows.push(preset);ids.push(id);}}
  prefs.save(&self.preferences_path)?;self.preferences=prefs;Ok(serde_json::json!({"saved":ids.len(),"ids":ids,"existing":existing}))
 }

}

#[cfg(test)]
mod tests {
 use super::*;
 #[test]fn both_hands_review_save_undo_and_stale_guard(){
  let data=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../work/cycle186-unit").join(format!("{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));let mut p=Player::new(data.clone(),PathBuf::new(),true);p.command(Command::Generate{spec:ExerciseSpec::default()}).unwrap();
  let cid=p.file.as_ref().unwrap().content_id.clone();let file=p.file.as_ref().unwrap();let mut parts=vec![];
  for hand in [PracticePart::LeftHand,PracticePart::RightHand] {
   let note=p.notes.iter().find(|n|hands::part(&p.config,&p.note_hands,n.track_id,n.index)==hand).unwrap();
   let request=PlanRequest{content_id:cid.clone(),track:note.track_id,index:note.index,profile:neothesia_core::fingering::HandSpanProfile::Standard,hand,range:Some((1,1)),keep_saved:true,practice_rate:1.,pins:vec![],target_tracks:vec![]};
   let prepared=crate::fingerings::prepare(file,&p.config,&p.fingers,&p.note_hands,request.clone()).unwrap();let fingerprint=prepared.fingerprint.clone();let generated=crate::fingerings::plans(prepared).unwrap();
   let edits=generated["plans"][0]["proposals"].as_array().unwrap().iter().map(|n|Edit{track_id:n["track"].as_u64().unwrap() as usize,note_index:n["index"].as_u64().unwrap() as usize,finger:Some(n["finger"].as_u64().unwrap() as u8)}).collect();parts.push(PairPart{request,fingerprint,edits});
  }
  let before=p.saved_hints().unwrap();let review=p.finger_pair(&parts,None,false).unwrap();assert_eq!(review["valid"],true);assert_eq!(p.saved_hints().unwrap(),before);let proof=review["proof"].as_str().unwrap().to_owned();
  assert!(p.start_pair_demo(&parts,"stale",true,0.5).is_err());
  let state_before=p.snapshot();let undo_before=p.finger_undo.len();
  let demo=p.start_pair_demo(&parts,&proof,true,0.5).unwrap();assert_eq!(demo["active"],true);
  let live=demo["fingerNotes"].as_array().unwrap();assert!(live.iter().any(|n|n["part"]=="left"));assert!(live.iter().any(|n|n["part"]=="right"));
  for n in live {let accepted=parts.iter().flat_map(|p|&p.edits).find(|e|e.track_id==n["track"].as_u64().unwrap() as usize&&e.note_index==n["index"].as_u64().unwrap() as usize).unwrap();assert_eq!(n["finger"].as_u64(),accepted.finger.map(|f|f as u64));}
  assert_eq!(p.saved_hints().unwrap(),before);assert_eq!(p.finger_undo.len(),undo_before);assert_eq!(p.snapshot().position,state_before.position);
  p.tick_finger_demo(Duration::from_secs(1));assert!(p.finger_demo_state()["position"].as_f64().unwrap()>demo["start"].as_f64().unwrap());
  let id=demo["id"].as_str().unwrap();let paused=p.finger_demo_control(id,"pause",None).unwrap();assert_eq!(paused["paused"],true);let pos=paused["position"].as_f64().unwrap();p.tick_finger_demo(Duration::from_millis(500));assert_eq!(p.finger_demo_state()["position"].as_f64().unwrap(),pos);
  assert!(p.finger_demo_control("other","resume",None).is_err());assert!(p.finger_demo_control(id,"seek",Some(f64::NAN)).is_err());
  let located=p.finger_demo_control(id,"seek",Some(demo["start"].as_f64().unwrap())).unwrap();assert_eq!(located["paused"],true);assert_eq!(located["fingerNotes"],demo["fingerNotes"]);
  assert_eq!(p.finger_demo_control(id,"resume",None).unwrap()["active"],true);
  p.finger_demo_control(id,"seek",Some(demo["end"].as_f64().unwrap())).unwrap();assert!(p.finger_demo_control(id,"resume",None).is_err());
  p.finger_demo_control(id,"seek",Some(demo["start"].as_f64().unwrap())).unwrap();assert_eq!(p.finger_demo_control(id,"resume",None).unwrap()["active"],true);
  p.command(Command::StopFingerDemo{id:Some(demo["id"].as_str().unwrap().into())}).unwrap();assert_eq!(p.finger_demo_state()["active"],false);
  let range=||crate::finger_practice::PracticeRange{name:"双手慢练".into(),start:1,end:1,notes:"同时放松两手".into(),speed:0.6,rounds:3};
  let mut invalid=range();invalid.end=999;assert!(p.save_pair_practice(&parts,&proof,true,vec![range(),invalid]).is_err());assert!(!p.preferences.passages.contains_key(&cid));
  p.state.metronome=true;assert!(p.save_pair_practice(&parts,&proof,true,vec![range()]).is_err());p.state.metronome=false;
  let original_path=p.preferences_path.clone();p.preferences_path=data.clone();assert!(p.save_pair_practice(&parts,&proof,true,vec![range()]).is_err());assert!(!p.preferences.passages.contains_key(&cid));p.preferences_path=original_path;
  let passage=p.save_pair_practice(&parts,&proof,true,vec![range()]).unwrap();assert_eq!(passage["saved"],1);assert_eq!(p.saved_hints().unwrap(),before);assert_eq!(p.save_pair_practice(&parts,&proof,true,vec![range()]).unwrap()["existing"],1);
  let id=passage["ids"][0].as_str().unwrap().to_owned();let preset=&p.preferences.passages[&cid][0];assert_eq!(preset.settings.hands,"both");assert_eq!(preset.settings.rounds,3);assert_eq!(preset.speed,0.6);
  let rid=p.save_routine(None,None,"双手慢练日课".into(),"".into(),None).unwrap()["id"].as_str().unwrap().to_owned();
  let item=p.save_routine_item(&rid,None,None,Some("passage"),Some(&id),"双手达标".into(),"".into(),crate::routine_commands::RoutineGoal{expression:None,passes:1,accuracy:90,on_time:None,consecutive:false,attempt_limit:3}).unwrap()["id"].as_str().unwrap().to_owned();
  p.open_routine_item(&rid,"2026-10-02",&item,true).unwrap();assert_eq!(p.state.hands,"both");assert_eq!(p.state.speed,0.6);p.command(Command::Play).unwrap();
  for _ in 0..1400 {p.tick(Duration::from_millis(10));for pitch in p.snapshot().required {p.command(Command::Note{pitch,velocity:90,active:true}).unwrap();p.command(Command::Note{pitch,velocity:0,active:false}).unwrap();}if p.routine_status().unwrap()["progress"]["completed"]==true {break;}}
  assert_eq!(p.routine_status().unwrap()["progress"]["completed"],true);p.stop_routine().unwrap();p.command(Command::Restart).unwrap();
  let backup:crate::practice_backup::Backup=serde_json::from_value(p.practice_backup_snapshot(&crate::practice_backup::Selection::default()).unwrap()).unwrap();let decoded=crate::practice_backup::Backup::decode(&backup.encode().unwrap()).unwrap();assert_eq!(decoded.passages[&cid][0].settings.hands,"both");
  p.finger_pair(&parts,Some(&proof),true).unwrap();let saved=p.saved_hints().unwrap();assert!(saved.len()>before.len());assert_eq!(p.finger_undo.len(),1);
  assert!(p.finger_pair(&parts,Some(&proof),true).is_err());assert_eq!(p.saved_hints().unwrap(),saved);
  p.command(Command::UndoFingersFor{content_id:cid.clone()}).unwrap();assert_eq!(p.saved_hints().unwrap(),before);
  let original=parts[1].request.range;parts[1].request.range=Some((1,2));assert!(p.finger_pair(&parts,None,false).is_err());parts[1].request.range=original;
  let actual=parts[1].edits[0].finger;parts[1].edits[0].finger=Some(9);assert!(p.finger_pair(&parts,Some(&proof),true).is_err());assert_eq!(p.saved_hints().unwrap(),before);parts[1].edits[0].finger=actual;
  p.finger_pair(&parts,Some(&proof),true).unwrap();drop(p);let mut p=Player::new(data,PathBuf::new(),true);p.restore();assert_eq!(p.saved_hints().unwrap(),saved);
 }
}
