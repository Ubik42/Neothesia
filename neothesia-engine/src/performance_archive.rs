use super::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::{
    collections::BTreeSet,
    io::{Read, Write},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub at: f64,
    pub judged_at: f64,
    pub song_time: f64,
    pub bytes: [u8; 3],
    pub synthetic: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub at: f64,
    pub end: f64,
    pub pitch: u8,
    pub velocity: u8,
    pub measure: usize,
    pub song_time: f64,
    pub track: usize,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct MeasureSpan {
    pub measure:usize,pub occurrence:usize,pub at:f64,pub end:f64,
    pub song_start:f64,pub song_end:f64,pub measure_start:f64,pub measure_end:f64,
}
#[derive(Default)]
pub struct Capture {
    inputs: Vec<Input>,
    references: Vec<Reference>,
    pedals: Vec<Input>,
    measures:Vec<MeasureSpan>,
    measure_counts:std::collections::BTreeMap<usize,usize>,
    pressed: BTreeSet<u8>,
    pedal: bool,
    truncated: bool,
}
impl Capture {
    pub(crate) fn note_velocity_evidence(&self, results: &[neothesia_core::practice::PracticeResult], expected_notes: usize) -> Option<crate::expression_goal::NoteVelocityEvidence> {
        if self.truncated { return None; }
        Some(note_velocity_evidence(&self.references,&self.inputs,expected_notes,results.iter().filter_map(|result|{
            matches!(result.judgement,neothesia_core::practice::PracticeJudgement::Matched(_))
                .then_some(result.reference.zip(result.input)).flatten()
        })))
    }
    pub fn advance(&mut self,at:f64,end:f64,from:f64,to:f64,speed:f64,grid:&[Duration],duration:f64){
        if grid.is_empty() || from>=duration-0.000001 || to<from || end<at{return;}
        let mut source=from;let mut clock=at;
        loop {
            let measure=grid.partition_point(|t|t.as_secs_f64()<=source+0.000001).max(1);
            let left=grid[measure-1].as_secs_f64();let right=grid.get(measure).map_or(duration,Duration::as_secs_f64).min(duration);
            let source_end=to.min(right);let clock_end=(end-(to-source_end)/speed).max(clock).min(end);
            if right<=left || source_end<source{return;}
            let can_merge=self.measures.last().is_some_and(|last|last.measure==measure && (last.measure_start-left).abs()<0.000001 && (last.measure_end-right).abs()<0.000001 && (last.end-clock).abs()<0.000001 && (last.song_end-source).abs()<0.000001);
            if can_merge{let last=self.measures.last_mut().unwrap();last.end=clock_end;last.song_end=source_end;}
            else if self.measures.len()<100000{
                let count=self.measure_counts.entry(measure).or_default();*count+=1;let occurrence=*count;
                self.measures.push(MeasureSpan{measure,occurrence,at:clock,end:clock_end,song_start:source,song_end:source_end,measure_start:left,measure_end:right});
            }else{self.truncated=true;return;}
            if source_end>=to-0.000001{return;}
            source=right;clock=clock_end;
        }
    }

    pub fn input(
        &mut self,
        at: f64,
        judged_at: f64,
        song_time: f64,
        bytes: [u8; 3],
    ) -> Option<usize> {
        if self.inputs.len() >= 100000 {
            self.truncated = true;
            return None;
        }
        let active = bytes[0] == 144 && bytes[2] > 0;
        if bytes[0] == 144 || bytes[0] == 128 {
            if active {
                self.pressed.insert(bytes[1]);
            } else {
                self.pressed.remove(&bytes[1]);
            }
        } else {
            self.pedal = bytes[2] >= 64;
        }
        let index = self.inputs.len();
        self.inputs.push(Input {
            at,
            judged_at,
            song_time,
            bytes,
            synthetic: false,
        });
        Some(index)
    }
    pub fn target(&mut self, mut n: Reference) -> Option<usize> {
        n.at = n.at.max(0.);
        n.end = n.end.max(n.at);
        if self.references.len() < 50000 {
            let index = self.references.len();
            self.references.push(n);
            Some(index)
        } else {
            self.truncated = true;
            None
        }
    }
    pub fn pedal(&mut self, at: f64, song_time: f64, value: u8) {
        if self.pedals.len() < 100000 {
            self.pedals.push(Input {
                at,
                judged_at: at,
                song_time,
                bytes: [176, 64, value],
                synthetic: false,
            });
        } else {
            self.truncated = true;
        }
    }
    pub fn close(&mut self, at: f64, song_time: f64) {
        for pitch in std::mem::take(&mut self.pressed) {
            self.inputs.push(Input {
                at,
                judged_at: at,
                song_time,
                bytes: [128, pitch, 0],
                synthetic: true,
            });
        }
        if self.pedal {
            self.inputs.push(Input {
                at,
                judged_at: at,
                song_time,
                bytes: [176, 64, 0],
                synthetic: true,
            });
            self.pedal = false;
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub reference: Option<usize>,
    pub input: Option<usize>,
    pub kind: String,
    pub offset_ms: Option<i32>,
    pub measure: Option<usize>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Archive {
    version: u8,
    content_id: String,
    id: String,
    pub duration: f64,
    pub inputs: Vec<Input>,
    pub references: Vec<Reference>,
    #[serde(default)]
    pub reference_pedals: Vec<Input>,
    pub truncated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judgments: Option<Vec<Decision>>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub measures:Option<Vec<MeasureSpan>>,
}
pub(crate) fn path(data: &Path, cid: &str, id: &str) -> PathBuf {
    data.join("performance-history")
        .join(
            blake3::hash(format!("{cid}\n{id}").as_bytes())
                .to_hex()
                .as_str(),
        )
        .with_extension("json")
}
pub fn available(data: &Path, cid: &str, id: &str) -> bool {
    path(data, cid, id).is_file()
}
pub fn save(
    data: &Path,
    cid: &str,
    id: &str,
    c: &Capture,
    duration: f64,
    results: &[neothesia_core::practice::PracticeResult],
) -> Result<(), String> {
    if c.references.is_empty() {
        return Ok(());
    }
    let mut inputs = c.inputs.clone();
    for pitch in &c.pressed {
        inputs.push(Input {
            at: duration,
            judged_at: duration,
            song_time: 0.,
            bytes: [128, *pitch, 0],
            synthetic: true,
        });
    }
    if c.pedal {
        inputs.push(Input {
            at: duration,
            judged_at: duration,
            song_time: 0.,
            bytes: [176, 64, 0],
            synthetic: true,
        });
    }
    let judgments: Vec<_> = results
        .iter()
        .filter(|r| r.reference.is_some() || r.input.is_some())
        .take(150000)
        .map(|r| Decision {
            reference: r.reference,
            input: r.input,
            kind: match r.judgement {
                neothesia_core::practice::PracticeJudgement::Matched(
                    neothesia_core::practice::TimingGrade::Early(_),
                ) => "early",
                neothesia_core::practice::PracticeJudgement::Matched(
                    neothesia_core::practice::TimingGrade::Late(_),
                ) => "late",
                neothesia_core::practice::PracticeJudgement::Matched(_) => "onTime",
                neothesia_core::practice::PracticeJudgement::Missed => "missed",
                neothesia_core::practice::PracticeJudgement::Wrong { .. } => "wrong",
            }
            .into(),
            offset_ms: r.timing_offset_ms,
            measure: r.target.map(|t| t.measure).filter(|m| *m > 0),
        })
        .collect();
    let truncated = c.truncated || judgments.len() != results.len();
    let a = Archive {
        version: 3,
        content_id: cid.into(),
        id: id.into(),
        duration,
        inputs,
        references: c.references.clone(),
        reference_pedals: c.pedals.clone(),
        truncated,
        judgments: Some(judgments),
        measures:Some(c.measures.clone()),
    };
    a.validate(cid, id)?;
    let bytes = serde_json::to_vec(&a).map_err(|e| e.to_string())?;
    if bytes.len() > 32000000 {
        return Err("本轮演奏回放超过容量上限，保存未完成".into());
    }
    let p = path(data, cid, id);
    std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    let temp = p.with_extension("json.tmp");
    let mut f = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
    f.write_all(&bytes)
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())?;
    drop(f);
    std::fs::rename(temp, p).map_err(|e| e.to_string())
}
pub(crate) fn read(data: &Path, cid: &str, id: &str) -> Result<Archive, String> {
    let p = path(data, cid, id);
    let mut bytes = Vec::new();
    std::fs::File::open(p)
        .map_err(|e| format!("本次没有可读取的演奏回放：{e}"))?
        .take(32000001)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 32000000 {
        return Err("演奏回放资料过大".into());
    }
    let a: Archive = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    a.validate(cid, id)?;
    Ok(a)
}
pub(crate) fn frozen_note_velocity_limit(session:&neothesia_core::practice_history::PracticeSession)->Option<u8>{
    session.context.as_ref()
        .and_then(|ctx|ctx.expression.as_ref().and_then(|goal|goal.note_velocity_difference)
            .or_else(||ctx.routine.as_ref().and_then(|routine|routine.expression.as_ref()).and_then(|goal|goal.note_velocity_difference)))
}
pub(crate) fn exact_velocity_difference(archive:&Archive,decision:&Decision,reference:usize)->Option<u8>{
    if decision.reference!=Some(reference)||!matches!(decision.kind.as_str(),"onTime"|"early"|"late"){return None}
    let target=archive.references.get(reference)?;
    let input=archive.inputs.get(decision.input?)?;
    if target.velocity==0||input.synthetic||input.bytes[0]!=144||input.bytes[2]==0||input.bytes[1]!=target.pitch{return None}
    Some(target.velocity.abs_diff(input.bytes[2]))
}
pub(crate) fn historical_problem(archive:&Archive,decision:&Decision,reference:usize,limit:Option<u8>)->Option<Value>{
    let target=archive.references.get(reference)?;
    let difference=limit.and_then(|_|exact_velocity_difference(archive,decision,reference));
    if let (Some(limit),Some(difference))=(limit,difference){
        if difference>limit {
            let input=&archive.inputs[decision.input?];
            return Some(serde_json::json!({"reference":reference,"measure":target.measure,"pitch":target.pitch,
                "kind":"velocityHigh","timingKind":decision.kind,"velocityDifference":difference,"velocityLimit":limit,
                "referenceVelocity":target.velocity,"actualVelocity":input.bytes[2]}));
        }
    }
    matches!(decision.kind.as_str(),"early"|"late"|"missed")
        .then(||serde_json::json!({"reference":reference,"measure":target.measure,"pitch":target.pitch,"kind":decision.kind}))
}
fn pedal_events(inputs: &[Input]) -> Vec<(f64, bool)> {
    let mut events: Vec<_> = inputs.iter()
        .filter(|input| !input.synthetic && input.bytes[0] == 176 && input.bytes[1] == 64)
        .map(|input| (input.at, input.bytes[2] >= 64))
        .collect();
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    events
}
fn pedal_at(events: &[(f64, bool)], at: f64) -> Option<bool> {
    let previous = events.partition_point(|event| event.0 <= at);
    previous.checked_sub(1).map(|index| events[index].1)
}
fn note_velocity_evidence(
    references: &[Reference], inputs: &[Input], expected_notes: usize,
    pairs: impl IntoIterator<Item=(usize,usize)>,
) -> crate::expression_goal::NoteVelocityEvidence {
    let mut matched = BTreeSet::new();
    let mut largest = None::<u8>;
    for (reference_index,input_index) in pairs {
        let (Some(reference),Some(input))=(references.get(reference_index),inputs.get(input_index)) else {continue};
        if reference.velocity == 0 || input.synthetic || input.bytes[0] != 144 || input.bytes[2] == 0
            || input.bytes[1] != reference.pitch || !matched.insert(reference_index) {continue;}
        let difference=reference.velocity.abs_diff(input.bytes[2]);
        largest=Some(largest.map_or(difference,|old|old.max(difference)));
    }
    let reference_notes=expected_notes.max(references.len());
    crate::expression_goal::NoteVelocityEvidence{reference_notes,matched_notes:matched.len(),
        missing_notes:reference_notes.saturating_sub(matched.len()),max_abs_difference:largest}
}
impl Archive {
    pub(crate) fn note_velocity_evidence(&self, expected_notes: usize) -> Option<crate::expression_goal::NoteVelocityEvidence> {
        let judgments=self.judgments.as_ref()?;
        if self.truncated { return None; }
        Some(note_velocity_evidence(&self.references,&self.inputs,expected_notes,judgments.iter().filter_map(|decision|{
            matches!(decision.kind.as_str(),"onTime"|"early"|"late")
                .then_some(decision.reference.zip(decision.input)).flatten()
        })))
    }
    fn expression_review(&self) -> Vec<Value> {
        let actual_pedals = pedal_events(&self.inputs);
        let reference_pedals = pedal_events(&self.reference_pedals);
        let mut result = vec![Value::Null; self.references.len()];
        let Some(decisions) = &self.judgments else { return result; };
        for decision in decisions {
            if !matches!(decision.kind.as_str(), "onTime" | "early" | "late") { continue; }
            let (Some(reference_index), Some(input_index)) = (decision.reference, decision.input) else { continue; };
            let (Some(reference), Some(input)) = (self.references.get(reference_index), self.inputs.get(input_index)) else { continue; };
            if reference.velocity == 0 || input.synthetic || input.bytes[0] != 144 || input.bytes[2] == 0 || input.bytes[1] != reference.pitch { continue; }
            let reference_pedal_down = pedal_at(&reference_pedals, reference.at);
            let actual_pedal_down = pedal_at(&actual_pedals, input.at);
            let pedal_evidence = if reference_pedals.is_empty() { "noReferenceEvents" }
                else if actual_pedals.is_empty() { "noActualEvents" }
                else if reference_pedal_down.is_none() { "beforeReferenceEvent" }
                else if actual_pedal_down.is_none() { "beforeActualEvent" }
                else { "paired" };
            result[reference_index] = serde_json::json!({
                "velocityDelta": i16::from(input.bytes[2]) - i16::from(reference.velocity),
                "referencePedalDown": reference_pedal_down,
                "actualPedalDown": actual_pedal_down,
                "pedalSame": reference_pedal_down.zip(actual_pedal_down).map(|(r,a)|r==a),
                "pedalEvidence": pedal_evidence,
            });
        }
        result
    }
    pub(crate) fn has_measures(&self)->bool{self.measures.is_some()}
    pub(crate) fn has_judgments(&self) -> bool {
        self.judgments.is_some()
    }
    pub(crate) fn validate_summary(
        &self,
        score: neothesia_core::practice::PracticeSnapshot,
    ) -> Result<(), String> {
        let Some(rows) = &self.judgments else {
            return Ok(());
        };
        let mut found = [0usize; 6];
        for r in rows {
            match r.kind.as_str() {
                "onTime" => {
                    found[0] += 1;
                    found[1] += 1
                }
                "early" => {
                    found[0] += 1;
                    found[2] += 1
                }
                "late" => {
                    found[0] += 1;
                    found[3] += 1
                }
                "wrong" => found[4] += 1,
                "missed" => found[5] += 1,
                _ => {}
            }
        }
        let expected = [
            score.matched_notes,
            score.on_time_notes,
            score.early_notes,
            score.late_notes,
            score.wrong_notes,
            score.missed_notes,
        ];
        if found
            .iter()
            .zip(expected)
            .any(|(a, b)| *a > b || (!self.truncated && *a != b))
        {
            return Err("逐音判定与原成绩不一致".into());
        }
        Ok(())
    }
    pub(crate) fn reidentify(&mut self, id: &str) {
        self.id = id.into();
    }
    pub(crate) fn bytes(&self) -> Result<Vec<u8>, String> {
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() > 32000000 {
            return Err("演奏回放资料过大".into());
        }
        Ok(bytes)
    }
    pub(crate) fn validate(&self, cid: &str, id: &str) -> Result<(), String> {
        if ![1, 2,3].contains(&self.version)
            || self.content_id != cid
            || self.id != id
            || !self.duration.is_finite()
            || self.duration < 0.
            || self.inputs.len() > 100256
            || self.references.len() > 50000
            || self.reference_pedals.len() > 100000
            || self.reference_pedals.iter().any(|n| {
                !n.at.is_finite()
                    || n.at < 0.
                    || !n.judged_at.is_finite()
                    || !n.song_time.is_finite()
                    || n.bytes[0] != 176
                    || n.bytes[1] != 64
                    || n.bytes[2] > 127
            })
            || self.inputs.iter().any(|n| {
                !n.at.is_finite()
                    || n.at < 0.
                    || !n.judged_at.is_finite()
                    || !n.song_time.is_finite()
                    || n.bytes[1] > 127
                    || n.bytes[2] > 127
                    || ![128, 144, 176].contains(&n.bytes[0])
                    || (n.bytes[0] == 176 && n.bytes[1] != 64)
            })
            || self.references.iter().any(|n| {
                !n.at.is_finite()
                    || !n.end.is_finite()
                    || n.at < 0.
                    || n.end < n.at
                    || !n.song_time.is_finite()
                    || n.pitch > 127
                    || n.velocity > 127
            })
        {
            return Err("演奏回放内容或记录身份无效".into());
        }
        if (self.version == 1 && self.judgments.is_some())
            || (self.version >= 2 && self.judgments.is_none())
            || (self.version==3 && self.measures.is_none())
            || (self.version<3 && self.measures.is_some())
        {
            return Err("演奏判定版本无效".into());
        }
        if let Some(measures)=&self.measures{
            if measures.len()>100000{return Err("历史小节边界超过容量".into());}
            let mut previous=0.;let mut counts=std::collections::BTreeMap::<usize,usize>::new();
            for m in measures{
                let occurrence=counts.entry(m.measure).or_default();*occurrence+=1;
                if [m.at,m.end,m.song_start,m.song_end,m.measure_start,m.measure_end].iter().any(|v|!v.is_finite() || *v<0.) || m.measure==0 || m.occurrence!=*occurrence || m.at<previous-0.000001 || m.end<m.at || m.end>self.duration+0.000001 || m.measure_end<=m.measure_start || m.song_start<m.measure_start-0.000001 || m.song_end>m.measure_end+0.000001 || m.song_end<m.song_start{
                    return Err("历史小节边界或演奏次数无效".into());
                }
                previous=m.end;
            }
        }
        if let Some(rows) = &self.judgments {
            if rows.len() > 150000 {
                return Err("演奏判定数量过多".into());
            }
            let mut targets = BTreeSet::new();
            let mut inputs = BTreeSet::new();
            for r in rows {
                if r.reference.is_none() && r.input.is_none() {
                    return Err("演奏判定缺少参考音和按键".into());
                }
                if r.measure.is_some_and(|m| m == 0 || m > 1000000) {
                    return Err("演奏判定小节无效".into());
                }
                let target = match r.reference {
                    Some(index) => {
                        if !targets.insert(index) {
                            return Err("演奏参考音重复判定".into());
                        }
                        Some(self.references.get(index).ok_or("演奏判定参考音不存在")?)
                    }
                    None => None,
                };
                let input = match r.input {
                    Some(index) => {
                        if !inputs.insert(index) {
                            return Err("演奏按键重复判定".into());
                        }
                        let input = self.inputs.get(index).ok_or("演奏判定按键不存在")?;
                        if input.synthetic || input.bytes[0] != 144 || input.bytes[2] == 0 {
                            return Err("演奏判定不是实际起音".into());
                        }
                        Some(input)
                    }
                    None => None,
                };
                match r.kind.as_str() {
                    "missed" if target.is_some() && input.is_none() && r.offset_ms.is_none() => {}
                    "wrong" if target.is_none() && input.is_some() && r.offset_ms.is_none() => {}
                    "early" | "late" | "onTime"
                        if (target.is_some() || self.truncated)
                            && (input.is_some() || self.truncated)
                            && r.offset_ms.is_some() =>
                    {
                        let offset = r.offset_ms.unwrap();
                        if (r.kind == "early" && offset > -80)
                            || (r.kind == "late" && offset < 80)
                            || (r.kind == "onTime" && offset.unsigned_abs() > 80)
                        {
                            return Err("演奏判定早晚等级无效".into());
                        }
                        if let (Some(target), Some(input)) = (target, input) {
                            if input.bytes[1] != target.pitch
                                || (((input.judged_at - target.at) * 1000.)
                                    .clamp(f64::from(i32::MIN), f64::from(i32::MAX))
                                    - f64::from(offset))
                                .abs()
                                    > 2.
                            {
                                return Err("演奏判定按键与参考音不对应".into());
                            }
                        }
                    }
                    _ => return Err("演奏判定内容无效".into()),
                }
                if let Some(target) = target {
                    if r.measure != Some(target.measure) {
                        return Err("演奏判定小节不对应".into());
                    }
                }
            }
        }
        self.bytes()?;
        Ok(())
    }
}

impl Player {
    pub fn history_score_review(&mut self, cid: &str, id: &str) -> Result<Value, String> {
        if self.state.recording
            || self.routine.is_some()
            || self.ladder.as_ref().is_some_and(|l| l.active)
            || ["playing", "countIn"].contains(&self.state.status.as_str())
            || !self.state.pressed.is_empty()
        {
            return Err("请先暂停演奏、停止录音并结束计划或阶梯，再批阅历史谱面".into());
        }
        let detail = self.history_detail(cid, id)?;
        let a = read(&self.data, cid, id)?;
        let result = self.history_open(cid, id, true)?;
        if result["restoreWarning"].as_str().is_some() {
            return Ok(result);
        }
        if self.notation.is_none() {
            return Err("这首曲目没有关联 MusicXML 乐谱，请先关联并校对谱面".into());
        }
        if self.state.mode == "memory" {
            self.command(Command::Mode {
                value: "flow".into(),
            })?;
        }
        let mut identities = std::collections::HashMap::<(usize, u8, u64), Vec<usize>>::new();
        for n in &self.notes {
            identities
                .entry((
                    n.track_id,
                    n.note,
                    (n.start.as_secs_f64() * 1000000.).round() as u64,
                ))
                .or_default()
                .push(n.index);
        }
        let decisions: std::collections::HashMap<_, _> = a
            .judgments
            .as_ref()
            .into_iter()
            .flatten()
            .filter_map(|d| d.reference.map(|r| (r, d)))
            .collect();
        let velocity_limit=self.history.song(cid).and_then(|h|h.sessions.iter().find(|s|s.stable_id()==id)).and_then(frozen_note_velocity_limit);
        let notes:Vec<_>=a.references.iter().enumerate().map(|(reference,n)|{
            let candidates=identities.get(&(n.track,n.pitch,(n.song_time*1000000.).round() as u64));
            let index=candidates.filter(|v|v.len()==1).map(|v|v[0]);let decision=decisions.get(&reference);
            let problem=decision.and_then(|d|historical_problem(&a,d,reference,velocity_limit));
            serde_json::json!({"reference":reference,"track":n.track,"index":index,"pitch":n.pitch,"measure":n.measure,"songTime":n.song_time,"at":n.at,"end":n.end,"kind":if a.judgments.is_some(){problem.as_ref().and_then(|p|p["kind"].as_str()).or_else(||decision.map(|d|d.kind.as_str())).unwrap_or("pending")}else{"approximate"},"timingKind":problem.as_ref().and_then(|p|p["timingKind"].as_str()),"velocityDifference":problem.as_ref().and_then(|p|p["velocityDifference"].as_u64()),"velocityLimit":problem.as_ref().and_then(|p|p["velocityLimit"].as_u64()),"offsetMs":decision.and_then(|d|d.offset_ms),"referenceVelocity":n.velocity,"actualVelocity":decision.and_then(|d|d.input).and_then(|i|a.inputs.get(i)).map(|i|i.bytes[2])})
        }).collect();
        let mut result = self.command(Command::CurrentSong)?;
        result["scoreReview"] = serde_json::json!({"contentId":cid,"historyId":id,"scoreRevision":self.score_revision,"recordedAt":detail["recordedAt"],"title":self.title,"truncated":a.truncated,"exact":a.judgments.is_some(),"wrongNotes":a.judgments.as_ref().map(|rows|rows.iter().filter(|r|r.kind=="wrong").count()),"notes":notes,"measures":a.measures});
        Ok(result)
    }
    pub fn history_practice_range(
        &mut self,
        cid: &str,
        id: &str,
        reference: usize,
        before: u8,
        after: u8,
    ) -> Result<Value, String> {
        if before > 8 || after > 8 {
            return Err("重练前后最多各选 8 小节".into());
        }
        if self.state.recording
            || self.routine.is_some()
            || self.ladder.as_ref().is_some_and(|l| l.active)
            || ["playing", "countIn"].contains(&self.state.status.as_str())
            || !self.state.pressed.is_empty()
        {
            return Err("请先暂停演奏、停止录音并结束计划或阶梯，再从历史选择重练".into());
        }
        self.history_detail(cid, id)?;
        let a = read(&self.data, cid, id)?;
        let target = a
            .references
            .get(reference)
            .ok_or("这次演奏没有所选参考音，请重新读取记录")?;
        let result = self.history_open(cid, id, true)?;
        if result["restoreWarning"].as_str().is_some() {
            return Ok(result);
        }
        let file = self.file.as_ref().ok_or("曲目打开失败")?;
        if file.content_id != cid
            || !self.notes.iter().any(|n| {
                n.track_id == target.track
                    && n.note == target.pitch
                    && (n.start.as_secs_f64() - target.song_time).abs() < 0.001
            })
        {
            return Err("原曲参考音位置已改变，请重新检查曲目，不能按旧记录定位".into());
        }
        let total = file.musical_time.measures.len();
        let measure = file
            .measures
            .partition_point(|t| t.as_secs_f64() <= target.song_time);
        if measure == 0 || measure > total || measure != target.measure {
            return Err("原曲小节位置已改变，不能按旧记录定位".into());
        }
        let start = measure.saturating_sub(usize::from(before)).max(1);
        let end = measure.saturating_add(usize::from(after)).min(total);
        if ["recital", "memory", "listen"].contains(&self.state.mode.as_str()) {
            self.command(Command::Mode {
                value: "flow".into(),
            })?;
        }
        self.command(Command::MeasureLoop {
            start,
            end,
            enabled: true,
        })?;
        self.state.adaptive = false;
        self.persist()?;
        let mut result = self.command(Command::CurrentSong)?;
        result["practiceRange"] = serde_json::json!({"start":start,"end":end,"measure":measure,"reference":reference,"songTime":target.song_time});
        let candidates: Vec<_> = self
            .notes
            .iter()
            .filter(|n| {
                n.track_id == target.track
                    && n.note == target.pitch
                    && (n.start.as_secs_f64() - target.song_time).abs() < 0.001
            })
            .collect();
        result["paperFocus"] = serde_json::json!({
            "contentId":cid,"measure":measure,"historyId":id,"reference":reference,
            "pitch":target.pitch,"track":target.track,"songTime":target.song_time,
            "startMeasure":start,"endMeasure":end,
        });
        if candidates.len()==1 {
            result["paperFocus"]["index"]=serde_json::json!(candidates[0].index);
            if let Ok(papers)=crate::score_attachments::list(&self.data,cid) {
                let active=papers["active"].as_str();
                if let Some(anchor)=papers["attachments"].as_array().into_iter().flatten()
                    .find(|a|a["id"].as_str()==active)
                    .and_then(|a|a["noteAnchors"].as_array())
                    .and_then(|rows|rows.iter().find(|n|n["track"].as_u64()==Some(target.track as u64)&&n["index"].as_u64()==Some(candidates[0].index as u64)&&n["pitch"].as_u64()==Some(target.pitch as u64)&&n["measure"].as_u64()==Some(measure as u64)&&n["grid"].as_str()==Some(self.grid_signature().as_str()))) {
                    result["paperFocus"]["noteAnchor"]=anchor.clone();
                }
            }
        }
        result["scoreFocus"] = if candidates.len() == 1 && self.notation.is_some() {
            serde_json::json!({"contentId":cid,"scoreRevision":self.score_revision,"historyId":id,"reference":reference,"track":target.track,"index":candidates[0].index,"pitch":target.pitch,"measure":measure,"kind":a.judgments.as_ref().map(|rows|rows.iter().find(|r|r.reference==Some(reference)).map(|r|r.kind.as_str()).unwrap_or("pending")).unwrap_or("approximate")})
        } else {
            serde_json::Value::Null
        };
        Ok(result)
    }
    pub fn history_performance(&self, cid: &str, id: &str) -> Result<Value, String> {
        self.history_detail(cid, id)?;
        let a = read(&self.data, cid, id)?;
        let review = a.expression_review();
        let mut value = serde_json::to_value(a).map_err(|e| e.to_string())?;
        value["expressionReview"] = serde_json::json!(review);
        Ok(value)
    }
    pub fn start_history_replay(
        &mut self,
        cid: &str,
        id: &str,
        source: &str,
        rate: f64,
    ) -> Result<Value, String> {
        if !rate.is_finite()
            || !(0.25..=1.).contains(&rate)
            || !["actual", "reference"].contains(&source)
        {
            return Err("请选择有效的回放来源和速度".into());
        }
        if ["playing", "countIn"].contains(&self.state.status.as_str())
            || self.recorder.is_recording()
            || self.routine.is_some()
            || self.ladder.as_ref().is_some_and(|r| r.active)
            || !self.state.pressed.is_empty()
        {
            return Err("请先停止练习、录音和计划，并松开琴键，再回放记录".into());
        }
        self.history_detail(cid, id)?;
        let a = read(&self.data, cid, id)?;
        let mut events = if source == "actual" {
            a.inputs
                .iter()
                .map(|n| (n.at, usize::MAX, n.bytes))
                .collect::<Vec<_>>()
        } else {
            a.references
                .iter()
                .flat_map(|n| {
                    [
                        (n.at, usize::MAX, [144, n.pitch, n.velocity.max(1)]),
                        (n.end, usize::MAX, [128, n.pitch, 0]),
                    ]
                })
                .collect::<Vec<_>>()
        };
        if source == "reference" {
            events.extend(
                a.reference_pedals
                    .iter()
                    .map(|n| (n.at, usize::MAX, n.bytes)),
            );
        }
        if events.is_empty() {
            return Err("该来源没有声音事件".into());
        }
        let end = events
            .iter()
            .map(|e| e.0)
            .fold(a.duration, f64::max)
            .max(0.01);
        events.push((end, usize::MAX, [176, 64, 0]));
        self.begin_finger_demo(0., end, rate, events, None)?;
        self.finger_demo.as_mut().unwrap().history=true;
        Ok(self.finger_demo_state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measure_capture_preserves_rests_waits_partial_spans_and_boundaries(){
        let grid=[0.,0.5,1.,1.5].map(Duration::from_secs_f64);
        let mut c=Capture::default();c.advance(0.,0.75,0.,0.75,1.,&grid,2.);
        assert_eq!(c.measures.len(),2);assert_eq!(c.measures[0].end,0.5);
        c.advance(0.75,1.75,0.75,0.75,1.,&grid,2.);assert_eq!(c.measures[1].end,1.75);
        c.advance(1.75,3.,0.75,2.,1.,&grid,2.);assert_eq!(c.measures.len(),4);
        assert_eq!(c.measures[1].end,2.);assert_eq!(c.measures[2].at,2.);assert_eq!(c.measures[2].end,2.5);
        c.advance(3.,3.5,2.,2.,1.,&grid,2.);assert_eq!(c.measures[3].end,3.);
        let mut partial=Capture::default();partial.advance(0.,0.25,0.75,1.25,2.,&grid,2.);
        assert_eq!(partial.measures.len(),2);assert_eq!(partial.measures[0].song_start,0.75);assert_eq!(partial.measures[1].song_end,1.25);assert_eq!(partial.measures[0].end,0.125);
    }
    #[test]
    fn exact_wait_late_input_wrong_identity_and_v8_backup_roundtrip() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work")
            .join(format!(
                "cycle171-unit-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut p = Player::new(data.clone(), PathBuf::from("unused.sf2"), true);
        let xml=br#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>2</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration></note></measure></part></score-partwise>"#;
        p.command(Command::ImportScore {
            bytes: xml.to_vec(),
            name: "逐音判定.musicxml".into(),
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
            value: "wait".into(),
        })
        .unwrap();
        p.command(Command::CountIn { bars: 0 }).unwrap();
        p.command(Command::Play).unwrap();
        p.tick(Duration::from_millis(30));
        p.tick(Duration::from_millis(1200));
        p.midi(&[144, 60, 83]);
        p.midi(&[128, 60, 0]);
        p.midi(&[144, 65, 99]);
        p.midi(&[128, 65, 0]);
        for _ in 0..30 {
            p.tick(Duration::from_millis(50));
        }
        p.midi(&[144, 62, 91]);
        p.midi(&[128, 62, 0]);
        for _ in 0..30 {
            p.tick(Duration::from_millis(50));
        }
        assert_eq!(p.state.status, "finished");
        let session = p
            .history
            .song(&cid)
            .unwrap()
            .sessions
            .last()
            .unwrap()
            .clone();
        let id = session.stable_id();
        let a = read(&data, &cid, &id).unwrap();
        assert_eq!(a.version, 3);
        let spans=a.measures.as_ref().unwrap();assert_eq!(spans.len(),1);assert!(spans[0].end>2.);assert_eq!(spans[0].song_start,0.);
        let mut old=a.clone();old.version=2;old.measures=None;old.validate(&cid,&id).unwrap();
        let mut invalid_span=a.clone();invalid_span.measures.as_mut().unwrap()[0].end=a.duration+1.;assert!(invalid_span.validate(&cid,&id).is_err());
        let mut missing=a.clone();missing.measures=None;assert!(missing.validate(&cid,&id).is_err());
        let rows = a.judgments.as_ref().unwrap();
        assert_eq!(rows.len(), 3);
        a.validate_summary(session.summary.overall).unwrap();
        let matched = rows.iter().find(|r| r.reference == Some(0)).unwrap();
        assert_eq!(matched.kind, "late");
        assert!(matched.offset_ms.unwrap() > 500);
        assert_eq!(a.inputs[matched.input.unwrap()].bytes, [144, 60, 83]);
        assert_eq!(a.expression_review()[0]["velocityDelta"], 83_i16 - i16::from(a.references[0].velocity));
        let note_velocity=a.note_velocity_evidence(2).unwrap();
        assert_eq!(note_velocity.reference_notes,2);
        assert_eq!(note_velocity.matched_notes,2);
        assert_eq!(note_velocity.missing_notes,0);
        assert!(note_velocity.max_abs_difference.unwrap()>=a.references[0].velocity.abs_diff(83));
        let incomplete=note_velocity_evidence(&a.references,&a.inputs,2,[(0,matched.input.unwrap())]);
        assert_eq!((incomplete.matched_notes,incomplete.missing_notes),(1,1));
        let mut legacy=a.clone();legacy.version=1;legacy.judgments=None;legacy.measures=None;
        assert!(legacy.note_velocity_evidence(2).is_none());
        let mut expression = a.clone();
        expression.reference_pedals.clear();
        assert!(expression.expression_review()[0]["pedalSame"].is_null());
        expression.reference_pedals.push(Input { at: 0., judged_at: 0., song_time: 0., bytes: [176, 64, 127], synthetic: false });
        assert_eq!(expression.expression_review()[0]["pedalEvidence"], "noActualEvents");
        assert!(expression.expression_review()[0]["pedalSame"].is_null());
        expression.inputs.push(Input { at: expression.inputs[matched.input.unwrap()].at + 0.1, judged_at: 0., song_time: 0., bytes: [176, 64, 0], synthetic: false });
        assert_eq!(expression.expression_review()[0]["pedalEvidence"], "beforeActualEvent");
        assert!(expression.expression_review()[0]["pedalSame"].is_null());
        expression.inputs.pop();
        expression.inputs.push(Input { at: expression.inputs[matched.input.unwrap()].at, judged_at: 0., song_time: 0., bytes: [176, 64, 0], synthetic: false });
        let review = expression.expression_review();
        assert_eq!(review[0]["referencePedalDown"], true);
        assert_eq!(review[0]["actualPedalDown"], false);
        assert_eq!(review[0]["pedalSame"], false);
        assert_eq!(review[0]["pedalEvidence"], "paired");
        let original_velocity = expression.references[0].velocity;
        expression.references[0].velocity = 0;
        assert!(expression.expression_review()[0].is_null());
        expression.references[0].velocity = original_velocity;
        expression.judgments = None;
        assert!(expression.expression_review().iter().all(Value::is_null));
        let wrong = rows.iter().find(|r| r.kind == "wrong").unwrap();
        assert_eq!(wrong.reference, None);
        assert_eq!(a.inputs[wrong.input.unwrap()].bytes, [144, 65, 99]);
        let backup: crate::practice_backup::Backup = serde_json::from_value(
            p.practice_backup_snapshot(&crate::practice_backup::Selection::default())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(backup.version(), 12);
        assert_eq!(serde_json::to_value(backup.performances[&cid][&id].measures.as_ref()).unwrap(),serde_json::to_value(a.measures.as_ref()).unwrap());
        let decoded = crate::practice_backup::Backup::decode(&backup.encode().unwrap()).unwrap();
        let target_data = data.join("restored");
        let mut target = Player::new(target_data, PathBuf::from("unused.sf2"), true);
        target
            .apply_practice_backup(
                decoded,
                crate::practice_backup::Selection::default(),
                "keep".into(),
            )
            .unwrap();
        assert_eq!(
            target.history_performance(&cid, &id).unwrap()["judgments"],
            serde_json::to_value(rows).unwrap()
        );
        assert_eq!(target.history_performance(&cid,&id).unwrap()["measures"],serde_json::to_value(a.measures.as_ref()).unwrap());
        let mut invalid = a.clone();
        let row = invalid
            .judgments
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r.reference == Some(0))
            .unwrap();
        row.input = wrong.input;
        assert!(invalid.validate(&cid, &id).is_err());
        let mut duplicate = a.clone();
        duplicate.judgments.as_mut().unwrap().push(rows[0].clone());
        assert!(duplicate.validate(&cid, &id).is_err());
        let mut changed = backup;
        changed.history.get_mut(&cid).unwrap().sessions[0]
            .summary
            .overall
            .late_notes = 0;
        assert!(changed.encode().is_err());
    }
    #[test]
    fn history_reference_repractice_restores_conditions_and_expanded_repeat_range() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work")
            .join(format!(
                "cycle170-unit-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut p = Player::new(data.clone(), PathBuf::from("unused.sf2"), true);
        let xml=br#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><barline location="left"><repeat direction="forward"/></barline><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure><measure number="2"><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration></note><barline location="right"><repeat direction="backward"/></barline></measure><measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration></note></measure></part></score-partwise>"#;
        p.command(Command::ImportScore {
            bytes: xml.to_vec(),
            name: "历史反复重练.musicxml".into(),
            default_bpm: 120,
        })
        .unwrap();
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let track = p.notes[0].track_id;
        p.command(Command::Track {
            id: track,
            mode: "human".into(),
            visible: true,
            part: "right".into(),
        })
        .unwrap();
        p.command(Command::Mode {
            value: "recital".into(),
        })
        .unwrap();
        p.command(Command::Speed { value: 0.75 }).unwrap();
        p.command(Command::Latency { milliseconds: 24 }).unwrap();
        p.command(Command::CountIn { bars: 0 }).unwrap();
        p.command(Command::Play).unwrap();
        for _ in 0..100 {
            p.tick(Duration::from_millis(50));
        }
        assert_eq!(p.state.status, "finished");
        let session = p
            .history
            .song(&cid)
            .unwrap()
            .sessions
            .last()
            .unwrap()
            .clone();
        let id = session.stable_id();
        let count = p.history.song(&cid).unwrap().sessions.len();
        let archive = read(&data, &cid, &id).unwrap();
        assert_eq!(archive.references.len(), 5);
        let index = archive
            .references
            .iter()
            .position(|r| r.measure == 3)
            .unwrap();
        assert_eq!(archive.references[index].pitch, 60);
        let review = p
            .command(Command::HistoryScoreReview {
                content_id: cid.clone(),
                id: id.clone(),
            })
            .unwrap();
        assert_eq!(review["scoreReview"]["notes"].as_array().unwrap().len(), 5);
        assert_eq!(review["scoreReview"]["notes"][2]["measure"], 3);
        assert_eq!(review["scoreReview"]["notes"][2]["kind"], "missed");
        assert_eq!(review["scoreReview"]["notes"][2]["at"], archive.references[2].at);
        assert_eq!(review["scoreReview"]["notes"][2]["end"], archive.references[2].end);
        assert_ne!(
            review["scoreReview"]["notes"][0]["index"],
            review["scoreReview"]["notes"][2]["index"]
        );
        assert_eq!(p.history.song(&cid).unwrap().sessions.len(), count);

        p.command(Command::Mode {
            value: "wait".into(),
        })
        .unwrap();
        p.command(Command::Speed { value: 1.25 }).unwrap();
        assert!(p.history_practice_range(&cid, &id, 9999, 0, 0).is_err());
        assert_eq!(p.state.speed, 1.25);
        let result = p
            .command(Command::HistoryPracticeRange {
                content_id: cid.clone(),
                id: id.clone(),
                reference: index,
                before: 0,
                after: 1,
            })
            .unwrap();
        assert_eq!(result["scoreFocus"]["contentId"], cid);
        assert_eq!(result["paperFocus"]["contentId"], cid);
        assert_eq!(result["paperFocus"]["measure"], 3);
        assert_eq!(result["paperFocus"]["pitch"], archive.references[index].pitch);
        assert_eq!(result["paperFocus"]["track"], archive.references[index].track);
        assert_eq!(result["paperFocus"]["startMeasure"], 3);
        assert_eq!(result["paperFocus"]["endMeasure"], 4);
        assert_eq!(result["paperFocus"]["index"],result["scoreFocus"]["index"]);
        let paper=crate::score_attachments::add(&data,&cid,"反复练习谱",b"\x89PNG\r\n\x1a\nrepeat".to_vec(),None).unwrap();
        let book=paper["active"].as_str().unwrap().to_string();
        let asset=paper["attachments"][0]["pages"][0]["id"].as_str().unwrap().to_string();
        let note_index=result["paperFocus"]["index"].as_u64().unwrap() as usize;
        p.command(Command::EditPaperNoteAnchor{content_id:cid.clone(),book,track:archive.references[index].track,index:note_index,draft:Some(crate::score_attachments::NoteAnchorDraft{pitch:60,measure:3,asset_id:asset,page:1,region:crate::score_attachments::PageRegion{x:0.2,y:0.3,width:0.08,height:0.04}}),expected:None}).unwrap();
        let exact=p.history_practice_range(&cid,&id,index,0,1).unwrap();
        assert_eq!(exact["paperFocus"]["noteAnchor"]["index"],note_index);
        let repeated=p.history_practice_range(&cid,&id,0,0,0).unwrap();
        assert_ne!(repeated["paperFocus"]["index"],note_index);
        assert!(repeated["paperFocus"]["noteAnchor"].is_null());
        p.history_practice_range(&cid,&id,index,0,1).unwrap();
        assert_eq!(result["scoreFocus"]["measure"], 3);
        assert_eq!(result["scoreFocus"]["pitch"], 60);
        assert_eq!(result["scoreFocus"]["kind"], "missed");
        assert_eq!(result["practiceRange"]["start"], 3);
        assert_eq!(result["practiceRange"]["end"], 4);
        assert_eq!(p.state.mode, "flow");
        assert_eq!(p.state.speed, 0.75);
        assert_eq!(p.state.latency, 24);
        assert_eq!(p.state.status, "ready");
        assert!(!p.state.adaptive);
        assert!((p.state.position - archive.references[index].song_time).abs() < 0.001);
        assert_eq!(p.history.song(&cid).unwrap().sessions.len(), count);
        p.command(Command::Play).unwrap();
        assert!(p.history_practice_range(&cid, &id, index, 0, 0).is_err());
        p.command(Command::Pause).unwrap();
        let result = p.history_practice_range(&cid, &id, index, 8, 8).unwrap();
        assert_eq!(result["practiceRange"]["start"], 1);
        assert_eq!(result["practiceRange"]["end"], 5);
        let routine = p
            .save_routine(None, None, "保持活动计划".into(), "".into(), None)
            .unwrap();
        let routine_id = routine["id"].as_str().unwrap();
        let item = p
            .save_routine_item(
                routine_id,
                None,
                None,
                Some("current"),
                None,
                "选段".into(),
                "".into(),
                crate::routine_commands::RoutineGoal {
                    expression: None,
                    passes: 1,
                    accuracy: 90,
                    on_time: None,
                    consecutive: false,
                    attempt_limit: 20,
                },
            )
            .unwrap();
        p.open_routine_item(routine_id, "2026-10-02", item["id"].as_str().unwrap(), true)
            .unwrap();
        assert!(
            p.command(Command::HistoryPracticeRange {
                content_id: cid.clone(),
                id: id.clone(),
                reference: index,
                before: 0,
                after: 0
            })
            .is_err()
        );
        assert!(
            p.command(Command::HistoryScoreReview {
                content_id: cid.clone(),
                id: id.clone()
            })
            .is_err()
        );
        assert!(p.routine.is_some());
        p.stop_routine().unwrap();
        let mut invalid = read(&data, &cid, &id).unwrap();
        invalid.references[index].measure = 1;
        std::fs::write(path(&data, &cid, &id), invalid.bytes().unwrap()).unwrap();
        assert!(p.history_practice_range(&cid, &id, index, 0, 0).is_err());
    }
    #[test]
    fn saved_history_keeps_real_inputs_targets_and_scoped_replay_without_scoring() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work")
            .join(format!(
                "cycle168-unit-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let sf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../default.sf2");
        let mut p = Player::new(data.clone(), sf.clone(), true);
        let xml=br#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>2</beats><beat-type>4</beat-type></time></attributes><direction><direction-type><pedal type="start"/></direction-type></direction><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration></note><direction><direction-type><pedal type="stop"/></direction-type></direction></measure></part></score-partwise>"#;
        p.command(Command::ImportScore {
            bytes: xml.to_vec(),
            name: "演奏回放单元.musicxml".into(),
            default_bpm: 120,
        })
        .unwrap();
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let track = p
            .file
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| !t.notes.is_empty())
            .unwrap()
            .track_id;
        p.command(Command::Track {
            id: track,
            mode: "human".into(),
            visible: true,
            part: "right".into(),
        })
        .unwrap();
        p.command(Command::Mode {
            value: "flow".into(),
        })
        .unwrap();
        p.command(Command::CountIn { bars: 0 }).unwrap();
        p.command(Command::Play).unwrap();
        p.tick(Duration::from_millis(30));
        p.midi(&[144, 60, 83]);
        p.midi(&[176, 64, 100]);
        p.tick(Duration::from_millis(220));
        p.midi(&[128, 60, 0]);
        p.tick(Duration::from_millis(350));
        p.midi(&[176, 64, 0]);
        p.tick(Duration::from_millis(600));
        let session = p
            .history
            .song(&cid)
            .unwrap()
            .sessions
            .last()
            .unwrap()
            .clone();
        let id = session.stable_id();
        let a = read(&data, &cid, &id).unwrap();
        assert_eq!(a.references.len(), 2);
        assert!(!a.reference_pedals.is_empty());
        assert_eq!(
            a.inputs.iter().filter(|i| i.bytes == [144, 60, 83]).count(),
            1
        );
        assert_eq!(a.inputs.iter().filter(|i| i.bytes[0] == 176).count(), 2);
        assert!(a.inputs.iter().any(|i| i.bytes == [128, 60, 0]));
        let position = p.state.position;
        let score = p.matcher.snapshot();
        let count = p.history.song(&cid).unwrap().sessions.len();
        p.start_history_replay(&cid, &id, "actual", 0.5).unwrap();
        p.tick(Duration::from_millis(200));
        assert_eq!(p.state.position, position);
        assert_eq!(p.matcher.snapshot().matched_notes, score.matched_notes);
        assert_eq!(p.history.song(&cid).unwrap().sessions.len(), count);
        p.command(Command::Note {
            pitch: 64,
            active: true,
            velocity: 90,
        })
        .unwrap();
        assert!(p.finger_demo.is_none());
        p.command(Command::Note {
            pitch: 64,
            active: false,
            velocity: 0,
        })
        .unwrap();
        let reloaded = Player::new(data, sf, true);
        assert!(reloaded.history_performance(&cid, &id).is_ok());
        assert!(reloaded.history_performance("different", &id).is_err());
    }
}
