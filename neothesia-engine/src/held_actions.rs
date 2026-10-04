use crate::{Player,Command,fingerings,hands};
use neothesia_core::{library::{FingerAction,FingerHint},practice::PracticePart,fingering::{FingerSubstitution,FingeringHand,validate_substitution_plan}};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ActionInput{track:usize,index:usize,before_track:usize,before_index:usize,at:f64,from:u8,to:u8}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ActionEdit {pub content_id:String,pub track:usize,pub index:usize,pub original_tick:u64,pub at_tick:u64,pub to:u8,pub baseline:String,pub rate:f64}
impl Player {
 pub(crate) fn saved_actions(&self)->Result<Vec<FingerAction>,String>{
  let f=self.file.as_ref().ok_or("请先选择曲目")?;let path=self.annotation_path(f)?;
  match neothesia_core::library::load_song_sidecar(&path,&f.content_id){Ok(s)=>Ok(s.finger_actions),Err(neothesia_core::library::MetadataError::Read(e))if e.kind()==std::io::ErrorKind::NotFound=>Ok(vec![]),Err(e)=>Err(e.to_string())}
 }
 pub(crate) fn accept_held_plan(&mut self,request:fingerings::PlanRequest,fingerprint:String,edits:Vec<fingerings::Edit>,actions:Vec<ActionInput>)->Result<serde_json::Value,String>{
  let f=self.file.as_ref().ok_or("请先选择曲目")?;
  let p=fingerings::prepare(f,&self.config,&self.fingers,&self.note_hands,request.clone())?;
  if p.fingerprint!=fingerprint{return Err("音符、分手或已有指法已变化，请重新生成持音换指方案".into());}
  if actions.is_empty()||actions.len()>4096{return Err("请选择含持音换指的完整方案".into());}
  let mut map=std::collections::BTreeMap::new();
  for e in &edits{let finger=e.finger.filter(|f|(1..=5).contains(f)).ok_or("起音手指无效")?;if !p.positions.iter().any(|n|n.target&&n.track==e.track_id&&n.index==e.note_index)||map.insert((e.track_id,e.note_index),finger).is_some(){return Err("方案修改越界或重复".into());}}
  let focus:Vec<_>=p.positions.iter().enumerate().filter_map(|(i,n)|n.target.then_some(i)).collect();
  if map.len()!=focus.len(){return Err("持音换指需要接受完整选段，不能单独保存部分动作".into());}
  let mut initial=Vec::new();for n in &p.positions{initial.push(if n.target{map[&(n.track,n.index)]}else{n.saved.ok_or("参考位置尚未标记，请先确认参考音轨的指法后重新推荐")?});}
  let mut schedule=Vec::new();let mut saved=Vec::new();
  for a in actions{
   if !a.at.is_finite()||a.at<0.{return Err("换指时刻无效".into());}
   let index=p.positions.iter().position(|n|n.target&&n.track==a.track&&n.index==a.index).ok_or("换指源音符不在选段")?;
   let before=p.positions.iter().position(|n|n.track==a.before_track&&n.index==a.before_index).ok_or("换指后音符已改变")?;
   let tick=f.tempo_track.seconds_to_pulses(a.at).floor() as u64;
   schedule.push(FingerSubstitution{note:index,before_note:before,at:f.tempo_track.pulses_to_duration(tick),from:a.from,to:a.to});
   saved.push(FingerAction{track_id:a.track,note_index:a.index,before_track:a.before_track,before_index:a.before_index,at_tick:tick,from:a.from,to:a.to,hand:request.hand,rate_milli:(request.practice_rate*1000.).round() as u16});
  }
  schedule.sort_by_key(|s|(s.at,s.note));
  let notes=p.notes.clone();
  validate_substitution_plan(&notes,&initial,&schedule,if request.hand==PracticePart::LeftHand{FingeringHand::Left}else{FingeringHand::Right},&focus,request.practice_rate)
   .map_err(|e|format!("当前保存组合不能完成持音换指（{}），请重新审阅指法和速度",e.kind))?;
  let mut hints=self.saved_hints()?;hints.retain(|h|!map.contains_key(&(h.track_id,h.note_index)));hints.extend(map.iter().map(|((track,index),finger)|FingerHint{track_id:*track,note_index:*index,finger:*finger}));
  let mut previous=self.saved_actions()?;previous.retain(|a|!map.contains_key(&(a.track_id,a.note_index))&&!map.contains_key(&(a.before_track,a.before_index)));let added=saved.len();let first_new=previous.len();previous.extend(saved);
  let mut actual=self.command(Command::CurrentSong)?;
  if let Some(notes)=actual["notes"].as_array_mut(){for n in notes{let key=(n["track"].as_u64().unwrap() as usize,n["index"].as_u64().unwrap() as usize);if let Some(finger)=map.get(&key){n["finger"]=serde_json::json!(finger);}}}
  if self.check_action_groups(&actual,&previous,request.practice_rate)[first_new..].iter().any(|valid|!*valid){return Err("持音到实际离键之间仍有未标记或冲突的音符，请扩大选段并确认相关声部指法后重新推荐；未保存修改".into());}
  self.write_schedule(hints,previous,true)?;Ok(serde_json::json!({"saved":true,"actions":added}))
 }
 pub(crate) fn decorate_finger_actions(&self,v:&mut serde_json::Value){
  let Some(file)=self.file.as_ref()else{return;};let Ok(actions)=self.saved_actions()else{return;};
  if actions.is_empty(){v["fingerActions"]=serde_json::json!([]);return;}
  let validity=self.check_action_groups(v,&actions,self.state.speed);
  v["fingerActionRevision"]=serde_json::json!(blake3::hash(&serde_json::to_vec(&(&file.content_id,&actions,&v["notes"],self.score_revision,self.saved_hints().ok())).unwrap()).to_hex().to_string());
  v["fingerActions"]=serde_json::json!(actions.iter().enumerate().map(|(i,a)|serde_json::json!({"track":a.track_id,"index":a.note_index,"beforeTrack":a.before_track,"beforeIndex":a.before_index,"atTick":a.at_tick,"at":file.tempo_track.pulses_to_duration(a.at_tick).as_secs_f64(),"from":a.from,"to":a.to,"valid":validity[i],"validationRate":self.state.speed,"practiceRate":a.rate_milli as f64/1000.})).collect::<Vec<_>>());
 }
 pub(super) fn check_action_groups(&self,v:&serde_json::Value,actions:&[FingerAction],rate:f64)->Vec<bool>{
  let file=self.file.as_ref().unwrap();let all=v["notes"].as_array().unwrap();
  let key=|n:&serde_json::Value|(n["track"].as_u64().unwrap() as usize,n["index"].as_u64().unwrap() as usize);
  let by_id:std::collections::HashMap<_,_>=all.iter().map(|n|(key(n),n)).collect();
  let start=|n:&serde_json::Value|n["start"].as_f64().unwrap();
  let end=|n:&serde_json::Value|start(n)+n["duration"].as_f64().unwrap();
  let mut validity=vec![false;actions.len()];
  for hand in [PracticePart::LeftHand,PracticePart::RightHand]{
   let mut intervals=Vec::new();
   for (i,a) in actions.iter().enumerate().filter(|(_,a)|a.hand==hand){
    let Some(source)=by_id.get(&(a.track_id,a.note_index))else{continue;};
    let Some(before)=by_id.get(&(a.before_track,a.before_index))else{continue;};
    if source["part"].as_str()!=Some(hands::label(hand))||before["part"].as_str()!=Some(hands::label(hand)){continue;}
    intervals.push((start(source),end(source),vec![i]));
   }
   intervals.sort_by(|a,b|a.0.total_cmp(&b.0));
   let mut regions:Vec<(f64,f64,Vec<usize>)>=Vec::new();
   for (lo,hi,ids) in intervals{
    if let Some(last)=regions.last_mut().filter(|r|lo<=r.1){last.1=last.1.max(hi);last.2.extend(ids);}else{regions.push((lo,hi,ids));}
   }
   let mut hand_notes:Vec<_>=all.iter().filter(|n|n["part"].as_str()==Some(hands::label(hand))).collect();
   hand_notes.sort_by(|a,b|start(a).total_cmp(&start(b)));
   let mut cursor=0;let mut active:Vec<&serde_json::Value>=Vec::new();
   for (lo,hi,ids) in regions{
    active.retain(|n|end(n)>lo);
    while cursor<hand_notes.len()&&start(hand_notes[cursor])<hi{if end(hand_notes[cursor])>lo{active.push(hand_notes[cursor]);}cursor+=1;}
    let mut context=active.clone();
    context.sort_by(|a,b|start(a).total_cmp(&start(b)).then(a["pitch"].as_u64().cmp(&b["pitch"].as_u64())));
    let index:std::collections::HashMap<_,_>=context.iter().enumerate().map(|(i,n)|(key(n),i)).collect();
    let notes:Vec<_>=context.iter().map(|n|neothesia_core::fingering::FingeringNote{pitch:n["pitch"].as_u64().unwrap() as u8,onset:std::time::Duration::from_secs_f64(start(n)),end:std::time::Duration::from_secs_f64(end(n)),anchored_finger:None}).collect();
    let fingers:Vec<_>=context.iter().map(|n|n["finger"].as_u64().unwrap_or(0) as u8).collect();
    let mut schedule=Vec::new();
    for i in &ids{let a=&actions[*i];if let (Some(note),Some(before_note))=(index.get(&(a.track_id,a.note_index)),index.get(&(a.before_track,a.before_index))){schedule.push(FingerSubstitution{note:*note,before_note:*before_note,at:file.tempo_track.pulses_to_duration(a.at_tick),from:a.from,to:a.to});}}
    schedule.sort_by_key(|s|(s.at,s.note));
    let valid=schedule.len()==ids.len()&&validate_substitution_plan(&notes,&fingers,&schedule,if hand==PracticePart::LeftHand{FingeringHand::Left}else{FingeringHand::Right},&(0..notes.len()).collect::<Vec<_>>(),rate).is_ok();
    for i in ids{validity[i]=valid;}
   }
  }
  validity
 }
 fn prepare_action_edit(&mut self,r:&ActionEdit)->Result<(Vec<FingerAction>,String,serde_json::Value),String>{
  if self.file.as_ref().is_none_or(|f|f.content_id!=r.content_id){return Err("曲目已切换，换指调整未保存".into());}
  if !r.rate.is_finite() || (r.rate-self.state.speed).abs()>0.000001{return Err("练习速度已变化，请重新检查换指调整".into());}
  if !(1..=5).contains(&r.to){return Err("接替手指应为 1 至 5".into());}
  let current=self.command(Command::CurrentSong)?;
  if current["fingerActionRevision"].as_str()!=Some(&r.baseline){return Err("音符、分手、已有指法或换指动作已变化，请重新打开并检查调整".into());}
  let mut actions=self.saved_actions()?;
  let a=actions.iter_mut().find(|a|a.track_id==r.track&&a.note_index==r.index&&a.at_tick==r.original_tick).ok_or("原换指动作已变化，请重新审阅")?;
  a.at_tick=r.at_tick;a.to=r.to;a.rate_milli=(r.rate*1000.).round() as u16;let hand=a.hand;let from=a.from;
  actions=neothesia_core::library::normalize_finger_actions(actions).map_err(|_|"换指时刻或接替手指无效，接替手指须不同于原指".to_string())?;
  let validity=self.check_action_groups(&current,&actions,r.rate);
  let edited=actions.iter().position(|a|a.track_id==r.track&&a.note_index==r.index&&a.at_tick==r.at_tick).ok_or("换指动作身份无效")?;
  if !validity[edited]{return Err("调整后相关持音段的换指不能衔接：请检查原指、接替指的占用、指序和换指前后的时间间隔".into());}
  let unrelated_invalid=actions.iter().zip(&validity).filter(|(a,valid)|a.hand==hand&&!**valid).count();
  let review=blake3::hash(&serde_json::to_vec(&(&r.baseline,&actions,r.rate)).unwrap()).to_hex().to_string();
  let at=self.file.as_ref().unwrap().tempo_track.pulses_to_duration(r.at_tick).as_secs_f64();
  let summary=serde_json::json!({"review":review,"valid":true,"at":at,"from":from,"to":r.to,"rate":r.rate,"scope":"related","unrelatedInvalid":unrelated_invalid,"actions":actions.iter().filter(|a|a.hand==hand).count()});
  Ok((actions,review,summary))
 }
 pub(crate) fn review_action_edit(&mut self,r:ActionEdit)->Result<serde_json::Value,String>{let (_,_,summary)=self.prepare_action_edit(&r)?;Ok(summary)}
 pub(crate) fn save_action_edit(&mut self,r:ActionEdit,expected:String)->Result<serde_json::Value,String>{
  let (actions,review,summary)=self.prepare_action_edit(&r)?;if review!=expected{return Err("调整内容已变化，请重新检查后保存".into());}
  if actions!=self.saved_actions()?{self.write_schedule(self.saved_hints()?,actions,true)?;}
  Ok(summary)
 }

}
