use crate::{
    Command, Player,
    preferences::{PassagePreset, SessionSettings},
};
use neothesia_core::{
    musicxml::ScoreTime,
    score_playback::{PlaybackLimits, build_playback_plan},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// A reference to one continuous performed path, bounded by written visits.
/// The paired MIDI, score bytes, and complete route must still agree on reopen.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScoreRange {
    #[serde(default,skip_serializing_if="is_zero")]
    pub start_trim:u64,
    #[serde(default,skip_serializing_if="is_zero")]
    pub end_trim:u64,
    pub score_identity: String,
    pub route_identity: String,
    pub first_visit: u32,
    pub last_visit: u32,
    pub start_tick: u64,
    pub end_tick: u64,
    pub description: String,
}

fn is_zero(n:&u64)->bool{*n==0}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::practice_backup::{Backup, Selection};
    use crate::routine_commands::{RoutineGoal, RoutineTarget};
    use std::path::PathBuf;

    const XML: &str = r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>2</beats><beat-type>4</beat-type></time></attributes><direction><sound tempo="60"/></direction><note><pitch><step>C</step><octave>4</octave></pitch><duration>2</duration></note><sound segno="S"><offset>-1</offset></sound><sound fine="yes"/></measure><measure number="2"><direction><sound tempo="120"/></direction><note><pitch><step>E</step><octave>4</octave></pitch><duration>2</duration></note><sound dalsegno="S"/></measure></part></score-partwise>"#;
    fn player() -> Player {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work/cycle154-unit")
            .join(format!(
                "{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut p = Player::new(path, PathBuf::new(), true);
        p.command(Command::ImportScore {
            bytes: XML.as_bytes().to_vec(),
            name: "partial.musicxml".into(),
            default_bpm: 120,
        })
        .unwrap();
        p
    }
    #[test]
    fn written_beat_trim_repeat_partial_visit_and_portable_daily() {
        let mut p=player();let ppq=u64::from(p.file.as_ref().unwrap().musical_time.ppq);
        let v=p.preview_score_range_trim(0,2,ppq/2,ppq/4).unwrap();assert_eq!(v["start"],0.5);assert_eq!(v["end"],3.75);assert_eq!(v["crossings"],1);assert_eq!(v["rows"][0]["startBeat"],1.5);assert_eq!(v["rows"][2]["endBeat"],2.75);
        let r:ScoreRange=serde_json::from_value(v["range"].clone()).unwrap();p.apply_score_range(&r).unwrap();assert_eq!(p.score_range_for_current_loop(),Some(r.clone()));
        let saved=p.save_score_passage(None,"反复拍内".into(),"保持连接".into(),r.clone()).unwrap();let id=saved["passage"]["id"].as_str().unwrap().to_string();p.command(Command::OpenPassage{id:id.clone()}).unwrap();assert_eq!(p.state.passage.as_ref().unwrap().end,3.75);
        let plan=p.save_routine(None,None,"短句日课".into(),"".into(),None).unwrap();let rid=plan["id"].as_str().unwrap();let goal=RoutineGoal{expression:None,passes:1,accuracy:90,on_time:None,consecutive:false,attempt_limit:4};let item=p.save_routine_item(rid,None,None,Some("passage"),Some(&id),"反复连接".into(),"".into(),goal.clone()).unwrap();p.save_routine_item(rid,None,None,Some("current"),None,"当前原谱".into(),"".into(),goal).unwrap();assert!(matches!(&p.preferences.routines[0].items[1].target,RoutineTarget::ScorePassage{range}if range==&r));
        let b:Backup=serde_json::from_value(p.practice_backup_snapshot(&Selection::default()).unwrap()).unwrap();assert_eq!(b.version(),11);let b=Backup::decode(&b.encode().unwrap()).unwrap();assert_eq!(b.passages.values().next().unwrap()[0].score_range,Some(r.clone()));let bytes:Vec<u8>=serde_json::from_value(p.command(Command::ExportPackage).unwrap()["bytes"].clone()).unwrap();let package=crate::piece_package::Package::decode(&bytes).unwrap();assert_eq!(package.manifest.version,8);let mut target=player();target.command(Command::ImportPackage{bytes,policy:"replace".into()}).unwrap();target.command(Command::OpenPassage{id:id.clone()}).unwrap();assert_eq!(target.state.passage.as_ref().unwrap().start,0.5);
        let data=p.data.clone();let path=p.file.as_ref().unwrap().source_path.clone().unwrap();drop(p);let mut p=Player::new(data,PathBuf::new(),true);p.command(Command::Load{path,title:"reopen".into()}).unwrap();p.command(Command::OpenPassage{id}).unwrap();p.open_routine_item(rid,"2026-10-02",item["id"].as_str().unwrap(),true).unwrap();p.command(Command::Play).unwrap();for _ in 0..600{p.tick(std::time::Duration::from_millis(10));for pitch in p.snapshot().required{p.command(Command::Note{pitch,velocity:90,active:true}).unwrap();p.command(Command::Note{pitch,velocity:0,active:false}).unwrap();}if p.routine_status().unwrap()["progress"]["completed"]==true{break}}assert_eq!(p.routine_status().unwrap()["progress"]["completed"],true);assert_eq!(p.matcher.snapshot().matched_notes,2);
    }
    #[test]
    fn written_trim_cannot_leave_visit_or_overwrite_route_identity() {
        let p=player();let ppq=u64::from(p.file.as_ref().unwrap().musical_time.ppq);let v=p.preview_score_range_trim(2,2,ppq/4,ppq/4).unwrap();assert_eq!(v["firstLimits"]["minBeat"],2.);assert_eq!(v["lastLimits"]["maxBeat"],3.);assert_eq!(v["rows"][0]["startBeat"],2.25);assert_eq!(v["rows"][0]["endBeat"],2.75);
        for (a,b) in [(ppq,0),(0,ppq),(ppq/2,ppq/2),(u64::MAX,0)]{assert!(p.preview_score_range_trim(2,2,a,b).is_err())}
        let mut r:ScoreRange=serde_json::from_value(v["range"].clone()).unwrap();r.start_tick-=1;assert!(p.validate_score_range(&r).is_err());r.start_tick+=1;r.start_trim+=1;assert!(p.validate_score_range(&r).is_err());assert!(p.score_range_context(&p.file.as_ref().unwrap().content_id,p.score_revision+1).is_err());
    }

    #[test]
    fn score_range_exact_save_reopen_context_and_path_guards() {
        let mut p = player();
        let whole = p.preview_score_range(0, 2).unwrap();
        assert_eq!(whole["crossings"], 1);
        assert_eq!(whole["rows"].as_array().unwrap().len(), 3);
        let range: ScoreRange =
            serde_json::from_value(p.preview_score_range(2, 2).unwrap()["range"].clone()).unwrap();
        assert!(range.description.contains("第 2 次"));
        p.state.speed = 0.7;
        let saved = p
            .save_score_passage(None, "中途返回".into(), "保持音".into(), range.clone())
            .unwrap();
        let id = saved["passage"]["id"].as_str().unwrap().to_string();
        p.state.speed = 1.;
        p.command(Command::OpenPassage { id: id.clone() }).unwrap();
        assert_eq!(
            (
                p.state.passage.as_ref().unwrap().start,
                p.state.passage.as_ref().unwrap().end
            ),
            (3., 4.)
        );
        assert_eq!(p.state.speed, 0.7);
        assert_eq!(p.score_range_for_current_loop(), Some(range.clone()));
        let context = p.file.as_ref().unwrap().content_id.clone();
        assert!(
            p.score_range_context(&context, p.score_revision + 1)
                .is_err()
        );
        let mut tampered = range.clone();
        tampered.start_tick -= 1;
        assert!(
            p.apply_score_range(&tampered)
                .unwrap_err()
                .contains("路径已改变")
        );
        assert_eq!(p.state.passage.as_ref().unwrap().start, 3.);
        p.command(Command::UpdatePassageDetails {
            content_id: context.clone(),
            id: id.clone(),
            name: "改名".into(),
            notes: "教师备注".into(),
        })
        .unwrap();
        assert_eq!(
            p.preferences.passages[&context][0].score_range,
            Some(range.clone())
        );
        let data = p.data.clone();
        let path = p.file.as_ref().unwrap().source_path.clone().unwrap();
        drop(p);
        let mut restarted = Player::new(data, PathBuf::new(), true);
        restarted
            .command(Command::Load {
                path,
                title: "reopened".into(),
            })
            .unwrap();
        restarted
            .command(Command::OpenPassage { id: id.clone() })
            .unwrap();
        assert_eq!(restarted.state.passage.as_ref().unwrap().start, 3.);
        restarted.notation.as_mut().unwrap().bytes.push(b' ');
        assert!(
            restarted
                .command(Command::OpenPassage { id })
                .unwrap_err()
                .contains("谱面版本")
        );
        assert_eq!(restarted.state.passage.as_ref().unwrap().start, 3.);
    }

    #[test]
    fn score_range_plan_snapshot_backup_and_piece_package_roundtrip() {
        let mut p = player();
        let range: ScoreRange =
            serde_json::from_value(p.preview_score_range(2, 2).unwrap()["range"].clone()).unwrap();
        let saved = p
            .save_score_passage(None, "返回练习".into(), "".into(), range.clone())
            .unwrap();
        let id = saved["passage"]["id"].as_str().unwrap();
        let plan = p
            .save_routine(None, None, "日课".into(), "".into(), None)
            .unwrap();
        let routine_id = plan["id"].as_str().unwrap();
        let item = p
            .save_routine_item(
                routine_id,
                None,
                None,
                Some("passage"),
                Some(id),
                "原谱片段".into(),
                "".into(),
                RoutineGoal {
                    expression: None,
                    passes: 1,
                    accuracy: 80,
                    on_time: None,
                    consecutive: true,
                    attempt_limit: 3,
                },
            )
            .unwrap();
        assert!(
            matches!(&p.preferences.routines[0].items[0].target,RoutineTarget::ScorePassage{range:r} if *r==range)
        );
        let snapshot: Backup =
            serde_json::from_value(p.practice_backup_snapshot(&Selection::default()).unwrap())
                .unwrap();
        assert_eq!(snapshot.version(), 5);
        let decoded = Backup::decode(&snapshot.encode().unwrap()).unwrap();
        assert_eq!(
            decoded.passages.values().next().unwrap()[0].score_range,
            Some(range.clone())
        );
        let bytes: Vec<u8> =
            serde_json::from_value(p.command(Command::ExportPackage).unwrap()["bytes"].clone())
                .unwrap();
        let package = crate::piece_package::Package::decode(&bytes).unwrap();
        assert_eq!(package.manifest.version, 5);
        assert_eq!(
            package.manifest.passages[0].score_range,
            Some(range.clone())
        );
        let mut target = player();
        target
            .command(Command::ImportPackage {
                bytes,
                policy: "replace".into(),
            })
            .unwrap();
        let restored_id = target.preferences.passages.values().next().unwrap()[0]
            .id
            .clone();
        target
            .command(Command::OpenPassage { id: restored_id })
            .unwrap();
        assert_eq!(
            (
                target.state.passage.as_ref().unwrap().start,
                target.state.passage.as_ref().unwrap().end
            ),
            (3., 4.)
        );
        let item_id = item["id"].as_str().unwrap();
        p.open_routine_item(routine_id, "2026-10-02", item_id, false)
            .unwrap();
        assert_eq!(
            (
                p.state.passage.as_ref().unwrap().start,
                p.state.passage.as_ref().unwrap().end
            ),
            (3., 4.)
        );
    }
}
impl ScoreRange {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if [&self.score_identity, &self.route_identity]
            .iter()
            .any(|v| v.len() != 64 || !v.bytes().all(|b| b.is_ascii_hexdigit()))
            || self.first_visit > self.last_visit
            || self.last_visit >= 20_000
            || self.start_trim>0x0fff_ffff || self.end_trim>0x0fff_ffff
            || self.end_tick <= self.start_tick
            || self.end_tick > 0x0fff_ffff
            || self.description.is_empty()
            || self.description.len() > 1024
        {
            return Err("原谱练习段的身份或演奏范围无效".into());
        }
        Ok(())
    }
}

impl Player {
    pub(super) fn preview_score_range(&self, first: u32, last: u32) -> Result<Value, String> {
        self.preview_score_range_trim(first,last,0,0)
    }
    pub(super) fn preview_score_range_trim(&self,first:u32,last:u32,start_trim:u64,end_trim:u64)->Result<Value,String>{
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        let asset = self.notation.as_ref().ok_or("请先配对 MusicXML 乐谱")?;
        let payload = asset.payload(Some(file));
        if payload["route"]["complete"] != true {
            return Err("谱面演奏顺序或音符对应需要校对，暂不能保存原谱选段".into());
        }
        let part = asset.score.parts.first().ok_or("乐谱没有可演奏声部")?;
        let plan = build_playback_plan(
            part,
            PlaybackLimits {
                max_visits: 20_000,
                max_repeat_passes: 16,
            },
        );
        if !plan.complete || first > last || last as usize >= plan.visits.len() {
            return Err("请选择起点之后的演奏终点".into());
        }
        let rows = payload["route"]["visits"]
            .as_array()
            .ok_or("演奏顺序不可用")?;
        let begin = &plan.visits[first as usize];
        let finish = &plan.visits[last as usize];
        let tick = |t: ScoreTime| {
            (t.numerator.max(0) as u128 * u128::from(file.musical_time.ppq)
                / u128::from(t.denominator)) as u64
        };
        let description = |i: usize| {
            let visit = &plan.visits[i];
            let number: String = part.measures[visit.source_measure_ordinal as usize]
                .number
                .chars()
                .take(64)
                .collect();
            let count = plan.visits[..=i]
                .iter()
                .filter(|v| v.source_measure_ordinal == visit.source_measure_ordinal)
                .count();
            format!(
                "原谱第 {} 小节（第 {} 次）",
                if number.is_empty() {
                    (visit.source_measure_ordinal + 1).to_string()
                } else {
                    number
                },
                count
            )
        };
        let begin_tick=tick(begin.performed_start);let finish_tick=tick(finish.performed_end());
        let begin_end=tick(begin.performed_end());let finish_start=tick(finish.performed_start);
        if start_trim>=begin_end-begin_tick || end_trim>=finish_tick-finish_start || begin_tick+start_trim>=finish_tick-end_trim {return Err("拍内端点超出所选演奏次数，或起终点顺序无效".into())}
        let limits=|i:usize,a:u64,b:u64|{let row=&rows[i];let min=row["startBeat"].as_f64().unwrap();let max=row["endBeat"].as_f64().unwrap();json!({"minBeat":min,"maxBeat":max,"ticksPerBeat":(b-a)as f64/(max-min)})};
        let first_limits=limits(first as usize,begin_tick,begin_end);let last_limits=limits(last as usize,finish_start,finish_tick);
        let first_beat=first_limits["minBeat"].as_f64().unwrap()+start_trim as f64/first_limits["ticksPerBeat"].as_f64().unwrap();let last_beat=last_limits["maxBeat"].as_f64().unwrap()-end_trim as f64/last_limits["ticksPerBeat"].as_f64().unwrap();
        let mut summary = if first == last {
            description(first as usize)
        } else {
            format!(
                "{} 至 {}",
                description(first as usize),
                description(last as usize)
            )
        };
        if rows[first as usize]["partialStart"] == true || start_trim>0 {
            summary.push_str(&format!(
                " · 第 {} 拍开始",
                first_beat
            ));
        }
        if rows[last as usize]["partialEnd"] == true || end_trim>0 {
            summary.push_str(&format!(
                " · 第 {} 拍前结束",
                last_beat
            ));
        }
        let range = ScoreRange {
            start_trim,end_trim,
            score_identity: blake3::hash(&asset.bytes).to_hex().to_string(),
            route_identity: blake3::hash(
                &serde_json::to_vec(&payload["route"]).map_err(|e| e.to_string())?,
            )
            .to_hex()
            .to_string(),
            first_visit: first,
            last_visit: last,
            start_tick: begin_tick+start_trim,
            end_tick: finish_tick-end_trim,
            description: summary,
        };
        range.validate()?;
        let start = file
            .tempo_track
            .pulses_to_duration(range.start_tick)
            .as_secs_f64();
        let end = file
            .tempo_track
            .pulses_to_duration(range.end_tick)
            .as_secs_f64();
        if end - start < 0.1 || end > self.state.duration + 0.000001 {
            return Err("选段时长过短或超出实际曲目".into());
        }
        let crossed = plan.visits[first as usize..=last as usize]
            .windows(2)
            .filter(|w| w[1].source_measure_ordinal != w[0].source_measure_ordinal + 1)
            .count();
        let notes = file
            .tracks
            .iter()
            .flat_map(|t| t.notes.iter())
            .filter(|n| n.start.as_secs_f64() >= start && n.start.as_secs_f64() < end)
            .count();
        let start_bar = file
            .musical_time
            .measures
            .partition_point(|m| m.start_tick <= range.start_tick)
            .max(1);
        let end_bar = file
            .musical_time
            .measures
            .partition_point(|m| m.start_tick < range.end_tick)
            .max(1);
        let mut selected_rows=rows[first as usize..=last as usize].to_vec();
        let last_index=selected_rows.len()-1;
        selected_rows[0]["start"]=json!(start);selected_rows[0]["startBeat"]=json!(first_beat);if start_trim>0{selected_rows[0]["partialStart"]=json!(true)}
        selected_rows[last_index]["end"]=json!(end);selected_rows[last_index]["endBeat"]=json!(last_beat);if end_trim>0{selected_rows[last_index]["partialEnd"]=json!(true)}
        Ok(
            json!({"firstLimits":first_limits,"lastLimits":last_limits,"range":range,"start":start,"end":end,"startBar":start_bar,"endBar":end_bar,
            "crossings":crossed,"notes":notes,"rows":selected_rows}),
        )
    }
    pub(super) fn validate_score_range(&self, range: &ScoreRange) -> Result<Value, String> {
        range.validate()?;
        let preview = self.preview_score_range_trim(range.first_visit, range.last_visit,range.start_trim,range.end_trim)?;
        let current: ScoreRange =
            serde_json::from_value(preview["range"].clone()).map_err(|e| e.to_string())?;
        if current != *range {
            return Err("谱面版本或演奏路径已改变，请重新审阅原谱选段并保存".into());
        }
        Ok(preview)
    }
    pub(super) fn score_range_context(&self, content: &str, revision: u64) -> Result<(), String> {
        if self.file.as_ref().is_none_or(|f| f.content_id != content)
            || revision != self.score_revision
        {
            return Err("曲目或谱面已切换，请重新打开原谱选段".into());
        }
        Ok(())
    }
    pub(super) fn score_range_for_current_loop(&self) -> Option<ScoreRange> {
        let passage = self.state.passage.as_ref()?;
        let file = self.file.as_ref()?;
        let payload = self.notation.as_ref()?.payload(Some(file));
        if payload["route"]["complete"] != true {
            return None;
        }
        let rows = payload["route"]["visits"].as_array()?;
        let first=rows.iter().find(|v|v["start"].as_f64().is_some_and(|s|s<=passage.start+0.000001)&&v["end"].as_f64().is_some_and(|e|e>passage.start+0.000001))?;
        let last=rows.iter().find(|v|v["start"].as_f64().is_some_and(|s|s<passage.end-0.000001)&&v["end"].as_f64().is_some_and(|e|e>=passage.end-0.000001))?;
        let ticks=|sec:f64|file.tempo_track.seconds_to_pulses(sec).round()as u64;
        let a=ticks(passage.start).checked_sub(ticks(first["start"].as_f64()?))?;
        let b=ticks(last["end"].as_f64()?).checked_sub(ticks(passage.end))?;
        let preview=self.preview_score_range_trim(first["ordinal"].as_u64()?as u32,last["ordinal"].as_u64()?as u32,a,b).ok()?;
        if (preview["start"].as_f64()?-passage.start).abs()>0.000001 || (preview["end"].as_f64()?-passage.end).abs()>0.000001{return None}
        serde_json::from_value(preview["range"].clone()).ok()
    }

    pub(super) fn apply_score_range(&mut self, range: &ScoreRange) -> Result<Value, String> {
        let preview = self.validate_score_range(range)?;
        self.command(Command::Loop {
            start: preview["start"].as_f64().unwrap(),
            end: preview["end"].as_f64().unwrap(),
            enabled: true,
        })
    }
    pub(super) fn save_score_passage(
        &mut self,
        id: Option<String>,
        name: String,
        notes: String,
        range: ScoreRange,
    ) -> Result<Value, String> {
        let preview = self.validate_score_range(&range)?;
        if name.trim().is_empty() || name.chars().count() > 80 || notes.len() > 8192 {
            return Err("请输入 1～80 字的名称，备注最多 8192 字节".into());
        }
        let content = self.file.as_ref().unwrap().content_id.clone();
        let updating = id.is_some();
        let preset = PassagePreset {
            id: id.unwrap_or_else(|| {
                format!(
                    "passage-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos()
                )
            }),
            name: name.trim().into(),
            notes,
            grid: Some(self.grid_signature()),
            start: preview["startBar"].as_u64().unwrap() as usize,
            end: preview["endBar"].as_u64().unwrap() as usize,
            score_range: Some(range),
            precise_range: None,
            speed: self.state.speed,
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
                rounds: self.state.rounds,
                adaptive: self.state.adaptive,
            },
        };
        if updating
            && !self
                .preferences
                .passages
                .get(&content)
                .is_some_and(|rows| rows.iter().any(|p| p.id == preset.id))
        {
            return Err("原练习段已不存在，请刷新后重新保存".into());
        }
        let mut prefs = self.preferences.clone();
        let rows = prefs.passages.entry(content).or_default();
        if let Some(existing) = rows.iter_mut().find(|p| p.id == preset.id) {
            *existing = preset.clone();
        } else {
            if rows.len() >= 200 {
                return Err("每首曲目最多保存 200 个练习段".into());
            }
            rows.push(preset.clone());
        }
        prefs.save(&self.preferences_path)?;
        self.preferences = prefs;
        Ok(json!({"saved":true,"passage":preset}))
    }
}
