use super::*;
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub(super) struct HeldContext{
 pub baseline:String,pub content_id:String,pub source_track:usize,pub source_index:usize,pub start:f64,pub end:f64,pub notes:Vec<HeldNote>,pub actions:Vec<HeldChange>,
}
#[derive(Clone,Serialize)]
pub(super) struct HeldNote{pub track:usize,pub index:usize,pub pitch:u8,pub part:String,pub onset:f64,pub end:f64,pub finger:Option<u8>}
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub(super) struct HeldChange{pub track:usize,pub index:usize,pub at:f64,pub at_tick:u64,pub from:u8,pub to:u8}
impl Player{
 pub(super) fn prepare_held_demo(&mut self,content_id:String,track:usize,index:usize,rate:f64)->Result<HeldContext,String>{
  if !rate.is_finite()||!(0.25..=1.).contains(&rate){return Err("示范速度应为原速的 25% 至 100%".into());}
  if self.file.as_ref().is_none_or(|f|f.content_id!=content_id){return Err("曲目已切换，请重新打开持音换指示范".into());}
  let current=self.command(Command::CurrentSong)?;let stored=self.saved_actions()?;
  let selected:Vec<_>=stored.iter().filter(|a|a.track_id==track&&a.note_index==index).collect();
  if selected.is_empty(){return Err("此演奏位置尚未保存持音换指".into());}
  let hand=selected[0].hand;let all=current["notes"].as_array().ok_or("音符资料不可用")?;
  let source=all.iter().find(|n|n["track"].as_u64()==Some(track as u64)&&n["index"].as_u64()==Some(index as u64)).ok_or("持音音符已变化")?;
  let start=source["start"].as_f64().unwrap();let mut end=start+source["duration"].as_f64().unwrap();
  for a in &selected{let before=all.iter().find(|n|n["track"].as_u64()==Some(a.before_track as u64)&&n["index"].as_u64()==Some(a.before_index as u64)).ok_or("衔接音已变化")?;end=end.max(before["start"].as_f64().unwrap()+before["duration"].as_f64().unwrap());}
  let notes:Vec<_>=all.iter().filter(|n|n["part"].as_str()==Some(crate::hands::label(hand))&&n["start"].as_f64().unwrap()<end&&n["start"].as_f64().unwrap()+n["duration"].as_f64().unwrap()>start)
   .map(|n|HeldNote{track:n["track"].as_u64().unwrap() as usize,index:n["index"].as_u64().unwrap() as usize,pitch:n["pitch"].as_u64().unwrap() as u8,part:n["part"].as_str().unwrap().to_string(),onset:n["start"].as_f64().unwrap(),end:n["start"].as_f64().unwrap()+n["duration"].as_f64().unwrap(),finger:n["finger"].as_u64().map(|f|f as u8)}).collect();
  if notes.len()>4608{return Err("示范相关音符超过 4608 个，请缩小持音段".into());}
  let ids:std::collections::HashSet<_>=notes.iter().map(|n|(n.track,n.index)).collect();
  let validity=self.check_action_groups(&current,&stored,rate);
  if stored.iter().enumerate().any(|(i,a)|a.track_id==track&&a.note_index==index&&!validity[i]){return Err("此持音音符的分手、起音指法或动作已不适用，请先重新审阅".into());}
  let file=self.file.as_ref().unwrap();let mut actions=Vec::new();
  for (i,a) in stored.iter().enumerate(){let at=file.tempo_track.pulses_to_duration(a.at_tick).as_secs_f64();
   if !ids.contains(&(a.track_id,a.note_index))||at>=end{continue;}
   if !validity[i]{return Err("这段持音换指在所选示范速度下不能衔接，请先审阅起音指法、分手与动作".into());}
   actions.push(HeldChange{track:a.track_id,index:a.note_index,at,at_tick:a.at_tick,from:a.from,to:a.to});
  }
  Ok(HeldContext{baseline:current["fingerActionRevision"].as_str().ok_or("换指资料已变化")?.to_string(),content_id,source_track:track,source_index:index,start,end,notes,actions})
 }
 pub(super) fn start_held_demo(&mut self,content_id:String,track:usize,index:usize,baseline:String,rate:f64)->Result<serde_json::Value,String>{
  if matches!(self.state.status.as_str(),"playing"|"countIn")||self.recorder.is_recording()||self.routine.is_some()||self.ladder.as_ref().is_some_and(|r|r.active)||!self.state.pressed.is_empty(){return Err("请先停止练习和录音，并松开琴键，再开始换指示范".into());}
  let context=self.prepare_held_demo(content_id,track,index,rate)?;
  if context.baseline!=baseline{return Err("音符、分手、指法或动作已变化，请重新打开示范".into());}
  let file=self.file.as_ref().unwrap();let mut events=Vec::new();
  for p in &context.notes{
   let n=file.tracks.iter().find(|t|t.track_id==p.track).and_then(|t|t.notes.get(p.index)).ok_or("示范音符已变化")?;
   events.push((p.onset.max(context.start),p.track,[144|n.channel,n.note,n.velocity.max(1)]));
   events.push((p.end.min(context.end),p.track,[128|n.channel,n.note,0]));
  }
  self.begin_finger_demo(context.start,context.end,rate,events,Some(context))
 }
}
