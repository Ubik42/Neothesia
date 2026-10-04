use super::*;
use serde_json::{Value,json};
use neothesia_core::practice_history::{ReplayClip,validate_replay_clips};
fn revision(clips:&[ReplayClip])->String{blake3::hash(&serde_json::to_vec(clips).unwrap()).to_hex().to_string()}
impl Player{
    fn replay_clip_rows(&self,cid:&str,id:&str)->Result<Vec<ReplayClip>,String>{
        let song=self.history.song(cid).ok_or("曲目记录不存在")?;
        Ok(song.sessions.iter().find(|s|s.stable_id()==id).ok_or("这条记录已不在历史中")?.replay_clips.clone())
    }
    pub(super) fn history_replay_clips(&self,cid:&str,id:&str)->Result<Value,String>{
        let clips=self.replay_clip_rows(cid,id)?;
        Ok(json!({"clips":clips,"revision":revision(&clips)}))
    }
    pub(super) fn save_history_replay_clip(&mut self,cid:&str,id:&str,clip_id:Option<&str>,name:&str,start:f64,end:f64,notes:&str,expected:&str)->Result<Value,String>{
        let old=self.replay_clip_rows(cid,id)?;
        if revision(&old)!=expected{return Err("回放片段已更新，请重新读取后再保存".into());}
        let archive=performance_archive::read(&self.data,cid,id)?;
        let time=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let row=ReplayClip{id:clip_id.map(str::to_owned).unwrap_or_else(||format!("clip-{}",time.as_nanos())),name:name.trim().into(),start,end,notes:notes.into(),updated_at_unix_ms:time.as_millis() as u64};
        let mut clips=old.clone();
        if let Some(key)=clip_id{let slot=clips.iter_mut().find(|c|c.id==key).ok_or("这个回放片段已不存在，请重新读取")?;*slot=row;}else{clips.push(row);}
        validate_replay_clips(&clips,Some(archive.duration))?;
        self.history.set_replay_clips(cid,id,old,clips)?;
        self.history_replay_clips(cid,id)
    }
    pub(super) fn delete_history_replay_clip(&mut self,cid:&str,id:&str,clip_id:&str,expected:&str)->Result<Value,String>{
        let old=self.replay_clip_rows(cid,id)?;
        if revision(&old)!=expected{return Err("回放片段已更新，请重新读取后再删除".into());}
        let clips:Vec<_>=old.iter().filter(|c|c.id!=clip_id).cloned().collect();
        if clips.len()==old.len(){return Err("这个回放片段已不存在，请重新读取".into());}
        self.history.set_replay_clips(cid,id,old,clips)?;
        self.history_replay_clips(cid,id)
    }
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn clips_crud_stale_range_reopen_and_missing_archive(){
   let data=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../work").join(format!("cycle178-unit-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
   let mut p=Player::new(data.clone(),PathBuf::from("unused.sf2"),true);
   let xml=br#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>2</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>2</duration></note></measure></part></score-partwise>"#;
   p.command(Command::ImportScore{bytes:xml.to_vec(),name:"clips.musicxml".into(),default_bpm:120}).unwrap();
   let cid=p.file.as_ref().unwrap().content_id.clone();let track=p.notes[0].track_id;
   p.command(Command::Track{id:track,mode:"human".into(),part:"right".into(),visible:true}).unwrap();p.command(Command::Mode{value:"recital".into()}).unwrap();p.command(Command::CountIn{bars:0}).unwrap();p.command(Command::Play).unwrap();for _ in 0..30 {p.tick(Duration::from_millis(50));}
   assert_eq!(p.state.status,"finished");let before=p.history.song(&cid).unwrap().sessions[0].clone();let id=before.stable_id();let old=p.history_replay_clips(&cid,&id).unwrap();let expected=old["revision"].as_str().unwrap();
   assert!(p.save_history_replay_clip(&cid,&id,None,"bad",0.,10.,"",expected).is_err());
   let saved=p.save_history_replay_clip(&cid,&id,None,"第一小节",0.,0.5,"右手放松",expected).unwrap();let clip_id=saved["clips"][0]["id"].as_str().unwrap();
   assert!(p.save_history_replay_clip(&cid,&id,Some(clip_id),"stale",0.,0.5,"",expected).is_err());assert!(p.delete_history_replay_clip(&cid,&id,clip_id,expected).is_err());
   assert_eq!(p.history.song(&cid).unwrap().sessions.len(),1);assert!(p.history.song(&cid).unwrap().sessions[0].same_performance(&before));
   let updated=p.save_history_replay_clip(&cid,&id,Some(clip_id),"重新听",0.2,0.8,"留意节奏",saved["revision"].as_str().unwrap()).unwrap();
   let mut reload=Player::new(data.clone(),PathBuf::from("unused.sf2"),true);assert_eq!(reload.history_replay_clips(&cid,&id).unwrap(),updated);
   let folder=data.join("performance-history");for file in std::fs::read_dir(folder).unwrap(){let file=file.unwrap().path();if file.extension().is_some_and(|s|s=="json"){std::fs::rename(&file,file.with_extension("unavailable")).unwrap();}}
   assert_eq!(reload.history_replay_clips(&cid,&id).unwrap(),updated);assert!(reload.save_history_replay_clip(&cid,&id,Some(clip_id),"missing",0.,0.5,"",updated["revision"].as_str().unwrap()).is_err());
   let deleted=reload.delete_history_replay_clip(&cid,&id,clip_id,updated["revision"].as_str().unwrap()).unwrap();assert_eq!(deleted["clips"].as_array().unwrap().len(),0);
 }
}
