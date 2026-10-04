use midi_file::MidiFile;
use neothesia_core::{
    fingering::{
        FingeringHand, FingeringNote, FingeringReason, HandSpanProfile,
        suggest_fingerings_with_profile,
    },
    practice::PracticePart,
    song_config::SongConfig,
};
use serde::Serialize;
use std::{collections::HashMap, time::Duration};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub track: usize,
    pub index: usize,
    pub pitch: u8,
    pub measure: usize,
    pub finger: u8,
    pub saved: Option<u8>,
    pub confidence: u8,
    pub reason: &'static str,
}

fn explanation(reason: FingeringReason) -> &'static str {
    match reason {
        FingeringReason::ManualAnchor => "保留已确认指法作为固定位置",
        FingeringReason::PhraseStart => "以自然手型开始乐句",
        FingeringReason::RepeatedNote => "慢速同音重复保持同一手指",
        FingeringReason::RapidRepeatedNote => "快速重复音交替手指，便于放松与离键",
        FingeringReason::InPosition => "在同一手位内顺着旋律移动",
        FingeringReason::ThumbUnder => "用拇指穿指衔接后续音符",
        FingeringReason::FingerOver => "用跨指衔接后续音符",
        FingeringReason::ChordShape => "为和弦各音分配不同手指",
        FingeringReason::ChordConnection => "相邻和弦的共同音尽量保留手指",
        FingeringReason::HeldChordPosition => "避开仍在按住其他音符的手指",
        FingeringReason::WideChordShape => "宽和弦使用外侧手指；跨度不舒适时应改用分解",
        FingeringReason::PositionShift => "跳进或不自然伸展后重新安排手位",
    }
}

pub fn propose(
    file: &MidiFile,
    config: &SongConfig,
    saved: &HashMap<(usize, Duration, u8), u8>,
    assigned: &HashMap<(usize, usize), PracticePart>,
    requested_hand: Option<PracticePart>,
    track: usize,
    index: usize,
    profile: HandSpanProfile,
    range: Option<(usize, usize)>,
) -> Result<Vec<Proposal>, String> {
    let t = file
        .tracks
        .iter()
        .find(|t| t.track_id == track)
        .ok_or("音轨不存在")?;
    let part = requested_hand.unwrap_or_else(|| crate::hands::part(config, assigned, track, index));
    let hand = match part {
        PracticePart::LeftHand => FingeringHand::Left,
        PracticePart::RightHand => FingeringHand::Right,
        _ => return Err("请先在逐音分手或音轨设置中指定左右手".into()),
    };
    let selected = t.notes.get(index).ok_or("音符不存在")?;
    // MIDI notes are stored in note-off order. Sort a view while preserving the
    // original indices used by content-bound finger sidecars.
    let mut order: Vec<_> = (0..t.notes.len())
        .filter(|&i| crate::hands::part(config, assigned, track, i) == part)
        .collect();
    order.sort_by_key(|&i| (t.notes[i].start, t.notes[i].note, i));
    let notes: Vec<_> = order.iter().map(|&i| &t.notes[i]).collect();
    let bounds = if let Some((a, b)) = range {
        if a == 0 || a > b || b > file.musical_time.measures.len() {
            return Err("请选择有效的小节范围".into());
        }
        let start = file
            .tempo_track
            .pulses_to_duration(file.musical_time.measures[a - 1].start_tick);
        let end = file
            .tempo_track
            .pulses_to_duration(file.musical_time.measures[b - 1].end_tick);
        (
            notes.partition_point(|n| n.start < start),
            notes.partition_point(|n| n.start < end),
        )
    } else {
        (
            notes.partition_point(|n| n.start < selected.start),
            notes.partition_point(|n| n.start <= selected.start),
        )
    };
    if bounds.0 == bounds.1 {
        return Err("选段内没有音符".into());
    }
    if bounds.1 - bounds.0 > 4096 {
        return Err("一次最多建议 4096 个音符，请缩小选段".into());
    }
    // Keep complete boundary chords and a bounded musical context, including held notes.
    let mut begin = bounds.0.saturating_sub(128);
    while begin > 0 && notes[begin - 1].start == notes[begin].start {
        begin -= 1;
    }
    let mut end = (bounds.1 + 128).min(notes.len());
    while end < notes.len() && notes[end - 1].start == notes[end].start {
        end += 1;
    }
    let inputs: Vec<_> = notes[begin..end]
        .iter()
        .enumerate()
        .map(|(i, n)| FingeringNote {
            pitch: n.note,
            onset: n.start,
            end: n.end,
            anchored_finger: if range.is_none() && (bounds.0..bounds.1).contains(&(begin + i)) {
                None
            } else {
                saved.get(&(track, n.start, n.note)).copied()
            },
        })
        .collect();
    let suggestions = suggest_fingerings_with_profile(&inputs, hand, profile);
    Ok((bounds.0..bounds.1)
        .filter_map(|i| {
            let n = notes[i];
            let s = suggestions[i - begin]?;
            let tick = file
                .tempo_track
                .seconds_to_pulses(n.start.as_secs_f64())
                .round() as u64;
            let measure = file
                .musical_time
                .measures
                .partition_point(|m| m.start_tick <= tick)
                .max(1);
            Some(Proposal {
                track,
                index: order[i],
                pitch: n.note,
                measure,
                finger: s.finger,
                saved: saved.get(&(track, n.start, n.note)).copied(),
                confidence: s.confidence_percent,
                reason: explanation(s.reason),
            })
        })
        .collect())
}

#[derive(Clone,serde::Serialize,serde::Deserialize)]
pub struct Edit {
    pub track_id: usize,
    pub note_index: usize,
    pub finger: Option<u8>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanRequest {
    pub content_id: String,
    pub track: usize,
    pub index: usize,
    pub profile: HandSpanProfile,
    pub hand: PracticePart,
    pub range: Option<(usize, usize)>,
    pub keep_saved: bool,
    #[serde(default = "default_practice_rate")]
    pub practice_rate: f64,
    #[serde(default)]
    pub pins: Vec<Pin>,
    #[serde(default)]
    pub target_tracks: Vec<usize>,
}
fn default_practice_rate() -> f64 { 1.0 }
pub fn target_tracks(request: &PlanRequest) -> Vec<usize> {
    let mut tracks=request.target_tracks.clone();tracks.push(request.track);
    tracks.sort_unstable();tracks.dedup();tracks
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Pin {
    #[serde(default)]
    pub track: Option<usize>,
    pub index: usize,
    pub finger: u8,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub track: usize,
    pub index: usize,
    pub pitch: u8,
    pub measure: usize,
    pub saved: Option<u8>,
    pub target: bool,
    pub onset: f64,
    pub end: f64,
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Prepared {
    pub request: PlanRequest,
    pub notes: Vec<FingeringNote>,
    pub positions: Vec<Position>,
    pub focus: (usize, usize),
    pub fingerprint: String,
}
pub fn prepare(
    file: &MidiFile,
    config: &SongConfig,
    saved: &HashMap<(usize, Duration, u8), u8>,
    assigned: &HashMap<(usize, usize), PracticePart>,
    request: PlanRequest,
) -> Result<Prepared, String> {
    if !request.practice_rate.is_finite() || !(0.25..=2.0).contains(&request.practice_rate) {
        return Err("指法推荐速度应为原速的 25% 至 200%".into());
    }
    if file.content_id != request.content_id {
        return Err("曲目已切换，请重新生成方案".into());
    }
    let part = request.hand;
    if !matches!(part, PracticePart::LeftHand | PracticePart::RightHand) {
        return Err("请指定左右手".into());
    }
    let t = file
        .tracks
        .iter()
        .find(|t| t.track_id == request.track)
        .ok_or("音轨不存在")?;
    let selected = t.notes.get(request.index).ok_or("音符不存在")?;
    let mut target_tracks=vec![request.track];
    if request.target_tracks.len()>32 {return Err("一次最多共同编辑 32 条音轨".into());}
    let mut seen_tracks=std::collections::HashSet::new();
    for &track in &request.target_tracks {
        if !seen_tracks.insert(track) {return Err("共同编辑音轨重复".into());}
        if !target_tracks.contains(&track) {target_tracks.push(track);}
    }
    if target_tracks.len()>32 {return Err("一次最多共同编辑 32 条音轨".into());}
    for &track in &target_tracks {
        let source=file.tracks.iter().find(|t|t.track_id==track).ok_or("共同编辑音轨不存在")?;
        if !source.notes.iter().enumerate().any(|(i,_)|crate::hands::part(config,assigned,track,i)==part) {
            return Err(format!("音轨 {} 没有指定为所选手的音符，请先调整分手",track+1));
        }
    }
    let window=if let Some((first,last))=request.range {
        if first==0 || first>last || last>file.musical_time.measures.len() {return Err("请选择有效的小节范围".into());}
        Some((file.tempo_track.pulses_to_duration(file.musical_time.measures[first-1].start_tick),
            file.tempo_track.pulses_to_duration(file.musical_time.measures[last-1].end_tick)))
    } else {None};
    let mut domain=Vec::new();
    for track in file.tracks.iter() {
        for (i,n) in track.notes.iter().enumerate() {
            if crate::hands::part(config,assigned,track.track_id,i)==part {domain.push((track.track_id,i,n));}
        }
    }
    domain.sort_by_key(|(track,i,n)|(n.start,n.note,*track,*i));
    let target=|(track,_,n):&(usize,usize,&midi_file::MidiNote)| {
        target_tracks.contains(track) && if let Some((start,end))=window {n.start>=start && n.start<end} else {n.start==selected.start}
    };
    let targets:Vec<_>=domain.iter().enumerate().filter_map(|(i,n)|target(n).then_some(i)).collect();
    if targets.is_empty() || targets.len()>4096 {return Err("所选声部和范围应包含 1 至 4096 个该手音符".into());}
    let a=*targets.first().unwrap(); let b=targets.last().unwrap()+1;
    let mut begin=a.saturating_sub(128);
    while begin>0 && domain[begin-1].2.start==domain[begin].2.start {begin-=1;}
    let mut end=(b+128).min(domain.len());
    while end<domain.len() && domain[end-1].2.start==domain[end].2.start {end+=1;}
    let onset=domain[begin].2.start;
    let indices:Vec<_>=(0..begin).filter(|i|domain[*i].2.end>onset).chain(begin..end).collect();
    if indices.len()>4608 {return Err("同手联合上下文超过 4608 个音符，请缩小选段".into());}
    let mut pins=HashMap::new();
    for pin in &request.pins {
        let track=pin.track.unwrap_or(request.track);
        if !(1..=5).contains(&pin.finger) || !targets.iter().any(|i|domain[*i].0==track && domain[*i].1==pin.index)
            || pins.insert((track,pin.index),pin.finger).is_some() {return Err("固定指法包含无效音符或重复位置".into());}
    }
    let mut notes=vec![];let mut positions=vec![];
    for j in indices {
        let (track,i,n)=domain[j];let is_target=target(&domain[j]);
        let existing=saved.get(&(track,n.start,n.note)).copied();
        notes.push(FingeringNote{pitch:n.note,onset:n.start,end:n.end,anchored_finger:if is_target {
            pins.get(&(track,i)).copied().or(if request.keep_saved {existing} else {None})
        } else {existing}});
        let tick=file.tempo_track.seconds_to_pulses(n.start.as_secs_f64()).round() as u64;
        positions.push(Position{track,index:i,pitch:n.note,measure:file.musical_time.measures.partition_point(|m|m.start_tick<=tick).max(1),
            saved:existing,target:is_target,onset:n.start.as_secs_f64(),end:n.end.as_secs_f64()});
    }
    let focus=(positions.iter().position(|p|p.target).unwrap(),positions.iter().rposition(|p|p.target).unwrap()+1);
    let baseline:Vec<_>=domain.iter().filter(|(track,_,_)|target_tracks.contains(track)).map(|(track,i,n)|
        (*track,*i,n.note,n.start,n.end,saved.get(&(*track,n.start,n.note)).copied())).collect();
    let fingerprint = blake3::hash(
        &serde_json::to_vec(&(&request, &notes, &positions, &baseline))
            .map_err(|e| e.to_string())?,
    )
    .to_hex()
    .to_string();
    Ok(Prepared {
        request,
        notes,
        positions,
        focus,
        fingerprint,
    })
}
pub fn plans(p: Prepared) -> Result<serde_json::Value, String> {
    let hand = if p.request.hand == PracticePart::LeftHand {
        FingeringHand::Left
    } else {
        FingeringHand::Right
    };
    let targets:Vec<_>=p.positions.iter().enumerate().filter_map(|(i,n)|n.target.then_some(i)).collect();
    let solutions = neothesia_core::fingering::suggest_fingering_plans_for_indices(
        &p.notes,
        hand,
        p.request.profile,
        &targets,
        3,
        p.request.practice_rate,
    )
    .map_err(|e| {
        let n = &p.positions[e.index.min(p.positions.len() - 1)];
        format!(
            "音轨 {}，第 {} 小节，MIDI 音高 {} 无法安排连贯指法（{}）。请调整固定指法、分手或选段。",
            n.track + 1,
            n.measure,
            n.pitch,
            if e.kind == "held" {
                "持音与指法冲突"
            } else {
                "手型或锚点冲突"
            }
        )
    })?;
    let result: Vec<_> = solutions.iter().enumerate().map(|(rank, s)| {
        let proposals: Vec<_> = p.positions.iter().zip(&s.suggestions).filter(|(n, _)| n.target)
            .map(|(n, f)| Proposal { track:n.track,index:n.index,pitch:n.pitch,measure:n.measure,
                finger:f.finger,saved:n.saved,confidence:f.confidence_percent,reason:explanation(f.reason) }).collect();
        let count = |r| p.positions.iter().zip(&s.suggestions).filter(|(n,f)| n.target && f.reason==r).count();
        let fingers: Vec<_> = s.suggestions.iter().map(|s| s.finger).collect();
        let review: Vec<_> = neothesia_core::fingering::review_fingering_plan(
            &p.notes, &fingers, hand, p.request.profile, p.request.practice_rate,
        ).into_iter().filter(|r| p.positions[r.index].target).map(|r| {
            let n=&p.positions[r.index]; let from=&p.positions[r.from_index];
            serde_json::json!({"contextIndex":r.index,"index":n.index,"fromIndex":from.index,"track":n.track,"fromTrack":from.track,
                "measure":n.measure,"fromMeasure":from.measure,"pitch":n.pitch,"fromPitch":from.pitch,"kind":r.kind,
                "semitones":r.semitones,"comfortableSemitones":r.comfortable_semitones,
                "intervalMs":r.interval_ms})
        }).collect();
        serde_json::json!({"rank":rank+1,"cost":s.cost,"proposals":proposals,
            "contextFingers":fingers,"review":review,"practiceRate":p.request.practice_rate,
            "crossings":count(FingeringReason::ThumbUnder)+count(FingeringReason::FingerOver),
            "shifts":count(FingeringReason::PositionShift),"wide":count(FingeringReason::WideChordShape),
            "changed":proposals.iter().filter(|n|n.saved.is_some_and(|f|f!=n.finger)).count()})
    }).collect();
    let target_tracks=target_tracks(&p.request);
    let mut context_tracks:Vec<_>=p.positions.iter().filter(|n|!target_tracks.contains(&n.track)).map(|n|n.track).collect();
    context_tracks.sort_unstable();context_tracks.dedup();
    Ok(
        serde_json::json!({"targetTracks":target_tracks,"contextTracks":context_tracks,"contentId":p.request.content_id,"fingerprint":p.fingerprint,"request":p.request,"plans":result,"context":p.positions}),
    )
}

/// Audit the marks that will actually remain after a selective acceptance.
/// Never substitute unaccepted suggestions for unsaved or retained positions.
/// Read-only action plans, distinct from onset-only acceptance until schedule persistence is connected.
pub fn held_plans(p:Prepared)->Result<serde_json::Value,String>{
    let hand=if p.request.hand==PracticePart::LeftHand{FingeringHand::Left}else{FingeringHand::Right};
    let targets:Vec<_>=p.positions.iter().enumerate().filter_map(|(i,n)|n.target.then_some(i)).collect();
    let plans=neothesia_core::fingering::suggest_fingering_plans_with_substitutions(&p.notes,hand,p.request.profile,&targets,3,p.request.practice_rate,true)
        .map_err(|e|format!("第 {} 小节不能安排安全的持音换指动作（{}），请调整固定指法、分手或速度",p.positions[e.index.min(p.positions.len()-1)].measure,e.kind))?;
    let result:Vec<_>=plans.iter().enumerate().map(|(rank,plan)|{
        let changes:Vec<_>=plan.substitutions.iter().map(|s|{let n=&p.positions[s.note];let before=&p.positions[s.before_note];
            serde_json::json!({"track":n.track,"index":n.index,"pitch":n.pitch,"measure":n.measure,"beforeTrack":before.track,"beforeIndex":before.index,
                "beforeMeasure":before.measure,"at":s.at.as_secs_f64(),"from":s.from,"to":s.to})}).collect();
        let proposals:Vec<_>=p.positions.iter().zip(&plan.suggestions).filter(|(n,_)|n.target).map(|(n,f)|Proposal{track:n.track,index:n.index,pitch:n.pitch,measure:n.measure,finger:f.finger,saved:n.saved,confidence:f.confidence_percent,reason:explanation(f.reason)}).collect();
        serde_json::json!({"rank":rank+1,"cost":plan.cost,"proposals":proposals,"substitutions":changes,"contextFingers":plan.suggestions.iter().map(|s|s.finger).collect::<Vec<_>>()})
    }).collect();
    Ok(serde_json::json!({"contentId":p.request.content_id,"request":p.request,"fingerprint":p.fingerprint,"context":p.positions,"plans":result,"readOnly":true}))
}
pub fn review_acceptance(p: &Prepared, edits: &[Edit]) -> Result<serde_json::Value, String> {
    if edits.is_empty() || edits.len() > 4096 { return Err("请选择 1 至 4096 个音符".into()); }
    let mut selected=HashMap::new();
    for edit in edits {
        let f=edit.finger.filter(|f| (1..=5).contains(f)).ok_or("建议手指应为 1 至 5")?;
        if !p.positions.iter().any(|n| n.target && n.track==edit.track_id && n.index==edit.note_index)
            || selected.insert((edit.track_id,edit.note_index),f).is_some() {
            return Err("保存选择包含范围外或重复音符".into());
        }
    }
    let mut known=vec![]; let mut fingers=vec![]; let mut source=vec![];let mut context_fingers=vec![];
    let mut unknown=0; let mut retained=0;
    for (i,(n,position)) in p.notes.iter().zip(&p.positions).enumerate() {
        let accepted=selected.get(&(position.track,position.index)).copied();
        let value=accepted.or(position.saved);
        context_fingers.push(value);
        if position.target && accepted.is_none() {
            if value.is_some() {retained+=1;} else {unknown+=1;}
        }
        if let Some(f)=value {known.push(*n);fingers.push(f);source.push(i);}
    }
    let hand=if p.request.hand==PracticePart::LeftHand {FingeringHand::Left} else {FingeringHand::Right};
    let mut active=Vec::<usize>::new(); let mut conflicts=vec![]; let mut truncated=false;
    'scan: for i in 0..known.len() {
        active.retain(|&j| known[j].end>known[i].onset || known[j].onset==known[i].onset);
        for &j in &active {
            let a=&p.positions[source[j]];let b=&p.positions[source[i]];
            if !selected.contains_key(&(a.track,a.index)) && !selected.contains_key(&(b.track,b.index)) {continue;}
            let same=fingers[j]==fingers[i] && known[j].pitch!=known[i].pitch;
            let order=match known[j].pitch.cmp(&known[i].pitch) {
                std::cmp::Ordering::Less=> if hand==FingeringHand::Right {fingers[j]<fingers[i]} else {fingers[j]>fingers[i]},
                std::cmp::Ordering::Greater=> if hand==FingeringHand::Right {fingers[j]>fingers[i]} else {fingers[j]<fingers[i]},
                std::cmp::Ordering::Equal=>fingers[j]==fingers[i],
            };
            if same || !order {
                conflicts.push(serde_json::json!({"contextIndex":source[i],"fromContextIndex":source[j],
                    "index":b.index,"fromIndex":a.index,"track":b.track,"fromTrack":a.track,"measure":b.measure,"fromMeasure":a.measure,
                    "pitch":b.pitch,"fromPitch":a.pitch,"finger":fingers[i],"fromFinger":fingers[j],
                    "reason":if same {"同一手指同时分配给不同音符"} else {"同时按住的音符与所选手的指序冲突"}}));
                if conflicts.len()==100 {truncated=true;break 'scan;}
            }
        }
        active.push(i);
    }
    let mut onset_counts=HashMap::new();
    for note in &p.notes {*onset_counts.entry(note.onset).or_insert(0usize)+=1;}
    let mut review:Vec<_>=neothesia_core::fingering::review_fingering_plan(&known,&fingers,hand,p.request.profile,p.request.practice_rate)
        .into_iter().filter(|r|{
            let a=&p.positions[source[r.from_index]];let b=&p.positions[source[r.index]];
            (selected.contains_key(&(a.track,a.index))||selected.contains_key(&(b.track,b.index))) &&
            (r.kind==neothesia_core::fingering::FingeringReviewKind::WideReach ||
             (source[r.from_index]+1==source[r.index] && onset_counts[&known[r.index].onset]==1
              && onset_counts[&known[r.from_index].onset]==1))
        }).map(|r|{
            let b=&p.positions[source[r.index]];let a=&p.positions[source[r.from_index]];
            serde_json::json!({"contextIndex":source[r.index],"track":b.track,"fromTrack":a.track,"measure":b.measure,"fromMeasure":a.measure,"pitch":b.pitch,"fromPitch":a.pitch,
                "kind":r.kind,"semitones":r.semitones,"comfortableSemitones":r.comfortable_semitones,"intervalMs":r.interval_ms})
        }).collect();
    let review_truncated=review.len()>100;review.truncate(100);
    let first=p.positions.iter().filter(|n|n.target).map(|n|n.onset).fold(f64::INFINITY,f64::min);
    let last=p.positions.iter().filter(|n|n.target).map(|n|n.end).fold(0.0,f64::max);
    let context_unmarked=p.positions.iter().filter(|n|!n.target && n.track!=p.request.track && n.saved.is_none() && n.onset<last && (n.end>first || n.onset==first)).count();
    let tracks:Vec<_>=target_tracks(&p.request).into_iter().map(|track|{
        let positions:Vec<_>=p.positions.iter().filter(|n|n.track==track && n.target).collect();
        let selected_count=positions.iter().filter(|n|selected.contains_key(&(n.track,n.index))).count();
        let retained_count=positions.iter().filter(|n|!selected.contains_key(&(n.track,n.index)) && n.saved.is_some()).count();
        let changed=positions.iter().filter(|n|n.saved.is_some() && selected.get(&(n.track,n.index)).is_some_and(|f|Some(*f)!=n.saved)).count();
        let new=positions.iter().filter(|n|n.saved.is_none() && selected.contains_key(&(n.track,n.index))).count();
        serde_json::json!({"track":track,"total":positions.len(),"selected":selected_count,"retained":retained_count,
            "unmarked":positions.len()-selected_count-retained_count,"changed":changed,"new":new})
    }).collect();
    Ok(serde_json::json!({"contextUnmarked":context_unmarked,"valid":conflicts.is_empty(),"conflicts":conflicts,"truncated":truncated,
        "review":review,"tracks":tracks,"contextFingers":context_fingers,"reviewTruncated":review_truncated,"selected":selected.len(),"retained":retained,"unmarked":unknown,
        "total":p.positions.iter().filter(|n|n.target).count()}))
}

#[cfg(test)]
mod acceptance_tests {
    use super::*;
    fn fixture(hand:PracticePart,saved:&[Option<u8>]) -> Prepared {
        let notes:Vec<_>=[(60,0,1000),(64,0,400),(65,500,900)].into_iter().map(|(pitch,start,end)|FingeringNote{
            pitch,onset:Duration::from_millis(start),end:Duration::from_millis(end),anchored_finger:None}).collect();
        let positions=notes.iter().enumerate().map(|(i,n)|Position{track:2,index:i,pitch:n.pitch,measure:1,
            saved:saved[i],target:i>0,onset:n.onset.as_secs_f64(),end:n.end.as_secs_f64()}).collect();
        Prepared{request:PlanRequest{content_id:"test".into(),track:2,index:1,hand,profile:HandSpanProfile::Standard,
            range:Some((1,1)),keep_saved:false,practice_rate:1.0,pins:vec![],target_tracks:vec![]},notes,positions,focus:(1,3),fingerprint:"test".into()}
    }
    #[test]
    fn selective_acceptance_uses_retained_marks_and_held_context_not_unaccepted_proposals() {
        let p=fixture(PracticePart::RightHand,&[Some(1),Some(3),None]);
        let bad=review_acceptance(&p,&[Edit{track_id:2,note_index:2,finger:Some(1)}]).unwrap();
        assert_eq!(bad["valid"],false);assert_eq!(bad["retained"],1);assert_eq!(bad["unmarked"],0);
        assert_eq!(bad["conflicts"][0]["fromIndex"],0);
        let good=review_acceptance(&p,&[Edit{track_id:2,note_index:2,finger:Some(4)}]).unwrap();
        assert_eq!(good["valid"],true);assert_eq!(good["selected"],1);
        let unknown=fixture(PracticePart::RightHand,&[Some(1),None,None]);
        let r=review_acceptance(&unknown,&[Edit{track_id:2,note_index:1,finger:Some(3)}]).unwrap();
        assert_eq!(r["valid"],true);assert_eq!(r["unmarked"],1);assert_eq!(r["retained"],0);
        assert!(review_acceptance(&p,&[Edit{track_id:2,note_index:0,finger:Some(1)}]).is_err());
        assert!(review_acceptance(&p,&[Edit{track_id:2,note_index:1,finger:Some(3)},Edit{track_id:2,note_index:1,finger:Some(3)}]).is_err());
        let left=fixture(PracticePart::LeftHand,&[Some(5),Some(3),None]);
        assert_eq!(review_acceptance(&left,&[Edit{track_id:2,note_index:2,finger:Some(2)}]).unwrap()["valid"],true);
        assert_eq!(review_acceptance(&left,&[Edit{track_id:2,note_index:2,finger:Some(5)}]).unwrap()["valid"],false);
    }
    #[test]
    fn shared_note_indices_across_tracks_do_not_override_readonly_context() {
        let mut p=fixture(PracticePart::RightHand,&[Some(1),Some(3),None]);
        p.positions[0].track=7;p.positions[0].index=1;
        let r=review_acceptance(&p,&[Edit{track_id:2,note_index:1,finger:Some(3)}]).unwrap();
        assert_eq!(r["valid"],true);assert_eq!(r["contextFingers"][0],1);
        p.positions[0].saved=None;
        let r=review_acceptance(&p,&[Edit{track_id:2,note_index:1,finger:Some(3)}]).unwrap();
        assert_eq!(r["contextUnmarked"],1);assert!(r["contextFingers"][0].is_null());
        assert!(review_acceptance(&p,&[Edit{track_id:7,note_index:1,finger:Some(1)}]).is_err());
    }
    #[test]
    fn batch_acceptance_keeps_track_qualified_edits_and_individual_summaries() {
        let mut p=fixture(PracticePart::RightHand,&[Some(1),Some(3),None]);
        p.request.target_tracks=vec![7];p.positions[0].track=7;p.positions[0].index=1;p.positions[0].target=true;
        let edits=vec![Edit{track_id:7,note_index:1,finger:Some(1)},Edit{track_id:2,note_index:1,finger:Some(3)}];
        let r=review_acceptance(&p,&edits).unwrap();
        assert_eq!(r["valid"],true);assert_eq!(r["selected"],2);assert_eq!(r["unmarked"],1);
        assert_eq!(r["tracks"][0]["track"],2);assert_eq!(r["tracks"][1]["track"],7);
        assert_eq!(r["tracks"][0]["selected"],1);assert_eq!(r["tracks"][1]["selected"],1);
        assert_eq!(r["contextFingers"][0],1);assert_eq!(r["contextFingers"][1],3);
    }
}
