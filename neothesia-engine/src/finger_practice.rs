use crate::{fingerings, preferences, Player};
use neothesia_core::practice_history::PracticeTrackMode;
use serde::Deserialize;

pub(crate) fn same_passage_content(saved: &preferences::PassagePreset, candidate: &preferences::PassagePreset) -> Result<bool,String> {
    let mut candidate=candidate.clone();
    candidate.id=saved.id.clone();
    Ok(serde_json::to_value(saved).map_err(|e|e.to_string())? == serde_json::to_value(candidate).map_err(|e|e.to_string())?)
}

#[derive(Deserialize,serde::Serialize)]
#[serde(rename_all="camelCase")]
pub struct PracticeRange {
    pub(super) name: String, pub(super) start: usize, pub(super) end: usize, pub(super) notes: String, pub(super) speed: f64, pub(super) rounds: usize,
}

impl Player {
    pub(crate) fn save_finger_practice(&mut self, request: fingerings::PlanRequest, fingerprint: String, ranges: Vec<PracticeRange>) -> Result<serde_json::Value,String> {
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        let prepared = fingerings::prepare(file,self.notation.as_ref(),&self.config,&self.fingers,&self.note_hands,request.clone())?;
        if prepared.fingerprint != fingerprint {return Err("音符、分手或指法已变化，请重新生成方案后建立练习段".into());}
        if ranges.is_empty() || ranges.len()>32 {return Err("每次请选择 1～32 个练习段".into());}
        for range in &ranges {
            if range.name.trim().is_empty() || range.name.chars().count()>80 {return Err("练习段名称需要 1～80 个字".into());}
            if range.start==0 || range.start>range.end || range.end>file.musical_time.measures.len() {return Err("练习段小节范围无效".into());}
            if !range.speed.is_finite() || !(0.25..=2.).contains(&range.speed) {return Err("练习速度需要在 25%～200% 之间".into());}
            if !(1..=99).contains(&range.rounds) || range.notes.len()>8192 {return Err("重复次数需要 1～99 次，备注不得超过 8192 字节".into());}
            if !prepared.positions.iter().any(|n|n.target && n.measure>=range.start && n.measure<=range.end) {return Err("练习段必须包含本次推荐范围内的目标音符".into());}
        }
        let targets=fingerings::target_tracks(&request);
        let mut tracks=self.config.practice_track_setup();
        for track in &mut tracks {
            track.mode=if targets.contains(&track.track_id) {PracticeTrackMode::Human} else if track.mode==PracticeTrackMode::Mute {PracticeTrackMode::Mute} else {PracticeTrackMode::Auto};
        }
        let settings=preferences::SessionSettings {
            tracks,parts:self.config.tracks.iter().map(|t|(t.track_id,t.practice_part)).collect(),
            mode:Some("wait".into()),count_in:self.state.count_in.max(1),metronome:self.state.metronome,
            latency:self.state.latency,rounds:1,hands:crate::hands::label(request.hand).into(),adaptive:false,
        };
        let mut preferences=self.preferences.clone();
        let content_id=request.content_id;
        let passages=preferences.passages.entry(content_id.clone()).or_default();
        let grid=self.grid_signature();
        let mut ids=Vec::new();
        let mut existing=0;
        for range in ranges {
            let identity=serde_json::to_vec(&serde_json::json!([&content_id,&fingerprint,&range])).map_err(|e|e.to_string())?;
            let id=format!("finger-practice-{}",blake3::hash(&identity).to_hex());
            let mut settings=settings.clone();settings.rounds=range.rounds;
            let preset=preferences::PassagePreset {id:id.clone(),name:range.name.trim().into(),start:range.start,end:range.end,
                notes:range.notes,speed:range.speed,settings,grid:Some(grid.clone()),score_range:None,precise_range:None};
            if let Some(saved)=passages.iter().find(|saved|saved.id==id) {
                if !same_passage_content(saved,&preset)? {
                    return Err("练习条件或已保存练习段发生变化，请重新核对后保存".into());
                }
                existing+=1;
            } else {
                let mut already_saved=false;
                for saved in passages.iter() {
                    if same_passage_content(saved,&preset)? {already_saved=true;break;}
                }
                if already_saved {existing+=1;continue;}
                if passages.len()>=200 {return Err("每首曲目最多保存 200 个练习段，整批未保存".into());}
                passages.push(preset);
                ids.push(id);
            }
        }
        if !ids.is_empty() {
            preferences.save(&self.preferences_path)?;
            self.preferences=preferences;
        }
        Ok(serde_json::json!({"saved":ids.len(),"ids":ids,"existing":existing}))
    }
}
