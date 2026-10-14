use midi_file::MidiFile;
use neothesia_core::{
    fingering::{
        FingeringExpression, FingeringHand, FingeringNote, FingeringReason, HandSpanProfile,
        suggest_fingerings_with_profile,
    },
    practice::PracticePart,
    song_config::SongConfig,
};
use serde::Serialize;
use std::{collections::{HashMap, HashSet}, time::Duration};

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
    #[serde(default)]
    pub expression: Vec<FingeringExpression>,
    #[serde(default)]
    pub score_fingerprint: Option<String>,
    #[serde(default)]
    pub opposite_notes: Vec<FingeringNote>,
    #[serde(default)]
    pub opposite_positions: Vec<Position>,
}

fn cross_hand_reviews(p: &Prepared, fingers: &[Option<u8>], opposite_fingers: &[Option<u8>], only_selected: bool) -> Vec<serde_json::Value> {
    if fingers.len()!=p.notes.len() || p.opposite_notes.len()!=p.opposite_positions.len()
        || opposite_fingers.len()!=p.opposite_notes.len() {return vec![];}
    let hand=if p.request.hand==PracticePart::LeftHand {FingeringHand::Left} else {FingeringHand::Right};
    let indexed:Vec<_>=p.notes.iter().enumerate().filter(|(i,_)| fingers[*i].is_some()
        && (!only_selected || p.positions[*i].target)).collect();
    let notes:Vec<_>=indexed.iter().map(|(_,n)|**n).collect();
    let own_fingers:Vec<_>=indexed.iter().map(|(i,_)|fingers[*i].unwrap()).collect();
    let opposite:Vec<_>=p.opposite_notes.iter().enumerate().filter(|(i,_)|opposite_fingers[*i].is_some()).collect();
    let opposite_notes:Vec<_>=opposite.iter().map(|(_,n)|**n).collect();
    let opposite_assigned:Vec<_>=opposite.iter().map(|(i,_)|opposite_fingers[*i].unwrap()).collect();
    neothesia_core::fingering::review_cross_hand(&notes,&own_fingers,hand,&opposite_notes,&opposite_assigned)
        .into_iter().take(100).map(|r|{
            let n=&p.positions[indexed[r.index].0];let other=&p.opposite_positions[opposite[r.opposite_index].0];
            serde_json::json!({"contextIndex":indexed[r.index].0,"track":n.track,"index":n.index,
                "measure":n.measure,"pitch":r.pitch,"finger":r.finger,
                "oppositeTrack":other.track,"oppositeIndex":other.index,"oppositeMeasure":other.measure,
                "oppositePitch":r.opposite_pitch,"oppositeFinger":r.opposite_finger,"oppositeSaved":other.saved.is_some(),
                "kind":r.kind,"overlapMs":r.overlap_ms})
        }).collect()
}

fn joint_cost(p:&Prepared, primary:&[u8], opposite:&[Option<u8>])->i64 {
    let known:Vec<_>=p.opposite_notes.iter().zip(opposite).filter_map(|(n,f)|f.map(|f|(*n,f))).collect();
    let (other_notes,other_fingers):(Vec<_>,Vec<_>)=known.into_iter().unzip();
    if p.request.hand==PracticePart::LeftHand {
        neothesia_core::fingering::cross_hand_plan_cost(&p.notes,primary,&other_notes,&other_fingers)
    } else {
        neothesia_core::fingering::cross_hand_plan_cost(&other_notes,&other_fingers,&p.notes,primary)
    }
}

fn rank_joint_candidates(p:&Prepared, primary:&[(Vec<u8>,i64)], opposite:&[(Vec<Option<u8>>,i64)], limit:usize)
    ->Vec<(usize,usize,i64,i64)> {
    let mut pairs:Vec<_>=primary.iter().enumerate().flat_map(|(primary_index,(fingers,cost))|{
        opposite.iter().enumerate().map(move |(opposite_index,(other,other_cost))|{
            let cross_cost=joint_cost(p,fingers,other);
            (primary_index,opposite_index,*cost+*other_cost+cross_cost,cross_cost)
        })
    }).collect();
    pairs.sort_by_key(|(primary,opposite,total,_)|(*total,*primary,*opposite));
    pairs.truncate(limit);
    pairs
}

fn expression_reason(
    expression: &[FingeringExpression],
    suggestions: &[neothesia_core::fingering::FingerSuggestion],
    index: usize,
) -> Option<&'static str> {
    let mark = expression.get(index)?;
    if let Some(from) = mark.slur_from {
        if suggestions.get(from)?.finger == suggestions.get(index)?.finger {
            return Some("谱面连线：同指换音会中断衔接，建议检查此处");
        }
        if suggestions[index].reason == FingeringReason::PositionShift {
            return Some("谱面连线：此处换手位可能中断衔接，建议放慢检查");
        }
        return Some("谱面连线：换指连接前后两音");
    }
    if mark.staccato_from.is_some() && suggestions[index].reason == FingeringReason::PositionShift {
        return Some("前音为谱面断奏：离键后可以重新安排手位");
    }
    if mark.staccato {
        return Some("谱面断奏：此音短促离键，为后续手位留空间");
    }
    None
}
pub fn prepare(
    file: &MidiFile,
    score: Option<&crate::notation::ScoreAsset>,
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
    let target_start=positions.iter().filter(|p|p.target).map(|p|p.onset).fold(f64::INFINITY,f64::min);
    let target_end=positions.iter().filter(|p|p.target).map(|p|p.end).fold(0.0,f64::max);
    let opposite_part=if part==PracticePart::LeftHand {PracticePart::RightHand} else {PracticePart::LeftHand};
    let mut opposite_notes=Vec::new();let mut opposite_positions=Vec::new();
    for track in file.tracks.iter() {for (index,n) in track.notes.iter().enumerate() {
        if crate::hands::part(config,assigned,track.track_id,index)!=opposite_part
            || n.start.as_secs_f64()>=target_end || n.end.as_secs_f64()<=target_start {continue;}
        let finger=saved.get(&(track.track_id,n.start,n.note)).copied().filter(|f|(1..=5).contains(f));
        let tick=file.tempo_track.seconds_to_pulses(n.start.as_secs_f64()).round() as u64;
        opposite_notes.push(FingeringNote{pitch:n.note,onset:n.start,end:n.end,anchored_finger:finger});
        opposite_positions.push(Position{track:track.track_id,index,pitch:n.note,
            measure:file.musical_time.measures.partition_point(|m|m.start_tick<=tick).max(1),saved:finger,
            target:false,onset:n.start.as_secs_f64(),end:n.end.as_secs_f64()});
    }}
    if opposite_notes.len()>4608 {return Err("对侧手在选段中超过 4608 个音符，请缩小小节范围".into());}
    let mut opposite_paired:Vec<_>=opposite_notes.into_iter().zip(opposite_positions).collect();
    opposite_paired.sort_by_key(|(n,p)|(n.onset,n.pitch,p.track,p.index));
    let (opposite_notes,opposite_positions):(Vec<_>,Vec<_>)=opposite_paired.into_iter().unzip();
    let trusted_score = score.filter(|score| score.content_id == file.content_id);
    let marks = trusted_score.map(|score| score.fingering_marks(file)).unwrap_or_default();
    let by_midi: HashMap<_, _> = positions.iter().enumerate()
        .map(|(i, position)| ((position.track, position.index), i)).collect();
    let expression: Vec<_> = positions.iter().enumerate().map(|(i, position)| {
        let mark = marks.get(&(position.track, position.index)).copied().unwrap_or_default();
        let linked = |id: Option<(usize, usize)>| id.and_then(|id| by_midi.get(&id).copied())
            .filter(|from| *from < i && positions[*from].onset < position.onset);
        FingeringExpression {
            slur_from: linked(mark.slur_from),
            staccato: mark.staccato,
            staccato_from: linked(mark.staccato_from),
        }
    }).collect();
    let score_fingerprint = trusted_score.map(|score| score.source_fingerprint.clone());
    let baseline:Vec<_>=domain.iter().filter(|(track,_,_)|target_tracks.contains(track)).map(|(track,i,n)|
        (*track,*i,n.note,n.start,n.end,saved.get(&(*track,n.start,n.note)).copied())).collect();
    let fingerprint = blake3::hash(
        &serde_json::to_vec(&(&request, &notes, &positions, &baseline, &expression, &score_fingerprint, &opposite_notes, &opposite_positions))
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
        expression,
        score_fingerprint,
        opposite_notes,
        opposite_positions,
    })
}
pub fn plans(p: Prepared) -> Result<serde_json::Value, String> {
    let hand = if p.request.hand == PracticePart::LeftHand {
        FingeringHand::Left
    } else {
        FingeringHand::Right
    };
    let targets:Vec<_>=p.positions.iter().enumerate().filter_map(|(i,n)|n.target.then_some(i)).collect();
    let solutions = neothesia_core::fingering::suggest_fingering_plans_with_expression(
        &p.notes,
        hand,
        p.request.profile,
        &targets,
        3,
        p.request.practice_rate,
        false,
        &p.expression,
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
    let opposite_focus:Vec<_>=p.opposite_notes.iter().enumerate().filter_map(|(i,n)|n.anchored_finger.is_none().then_some(i)).collect();
    let saved_opposite:Vec<_>=p.opposite_notes.iter().map(|n|n.anchored_finger).collect();
    let opposite_hand=if hand==FingeringHand::Left {FingeringHand::Right} else {FingeringHand::Left};
    let (opposite_plans,joint_status):(Vec<(Vec<Option<u8>>,i64)>,&'static str)=if opposite_focus.is_empty() {
        (vec![(saved_opposite,0)],"对侧指法均已保存，按固定位置参与联合比较")
    } else {
        match neothesia_core::fingering::suggest_fingering_plans_for_indices(
            &p.opposite_notes,opposite_hand,HandSpanProfile::Standard,&opposite_focus,3,p.request.practice_rate) {
            Ok(plans)=>(plans.into_iter().map(|s|(s.suggestions.into_iter().map(|f|Some(f.finger)).collect(),s.cost)).collect(),
                "对侧未保存音符使用标准跨度临时推荐；只有所选手的指法会保存"),
            Err(_)=>(vec![(saved_opposite,0)],"对侧固定指法与持音位置无法组成完整候选；联合比较只使用已保存位置"),
        }
    };
    let primary_candidates:Vec<_>=solutions.iter().map(|s|(s.suggestions.iter().map(|f|f.finger).collect(),s.cost)).collect();
    let pairs=rank_joint_candidates(&p,&primary_candidates,&opposite_plans,3);
    let result: Vec<_> = pairs.into_iter().enumerate().map(|(rank, (primary_index,opposite_index,total_cost,cross_cost))| {
        let s=&solutions[primary_index];
        let opposite_fingers=&opposite_plans[opposite_index].0;
        let proposals: Vec<_> = p.positions.iter().zip(&s.suggestions).enumerate().filter(|(_, (n, _))| n.target)
            .map(|(i, (n, f))| Proposal { track:n.track,index:n.index,pitch:n.pitch,measure:n.measure,
                finger:f.finger,saved:n.saved,confidence:f.confidence_percent,
                reason:expression_reason(&p.expression, &s.suggestions, i).unwrap_or_else(|| explanation(f.reason)) }).collect();
        let count = |r| p.positions.iter().zip(&s.suggestions).filter(|(n,f)| n.target && f.reason==r).count();
        let fingers: Vec<_> = s.suggestions.iter().map(|s| s.finger).collect();
        let cross_hand_review=cross_hand_reviews(&p,&fingers.iter().copied().map(Some).collect::<Vec<_>>(),opposite_fingers,true);
        let review: Vec<_> = neothesia_core::fingering::review_fingering_plan(
            &p.notes, &fingers, hand, p.request.profile, p.request.practice_rate,
        ).into_iter().filter(|r| p.positions[r.index].target).map(|r| {
            let n=&p.positions[r.index]; let from=&p.positions[r.from_index];
            serde_json::json!({"contextIndex":r.index,"index":n.index,"fromIndex":from.index,"track":n.track,"fromTrack":from.track,
                "measure":n.measure,"fromMeasure":from.measure,"pitch":n.pitch,"fromPitch":from.pitch,"kind":r.kind,
                "semitones":r.semitones,"comfortableSemitones":r.comfortable_semitones,
                "intervalMs":r.interval_ms})
        }).collect();
        serde_json::json!({"rank":rank+1,"cost":total_cost,"primaryCost":s.cost,
            "oppositeCost":opposite_plans[opposite_index].1,"jointCost":cross_cost,"proposals":proposals,
            "contextFingers":fingers,"oppositeContextFingers":opposite_fingers,"review":review,"crossHandReview":cross_hand_review,"practiceRate":p.request.practice_rate,
            "crossings":count(FingeringReason::ThumbUnder)+count(FingeringReason::FingerOver),
            "shifts":count(FingeringReason::PositionShift),"wide":count(FingeringReason::WideChordShape),
            "changed":proposals.iter().filter(|n|n.saved.is_some_and(|f|f!=n.finger)).count()})
    }).collect();
    let target_tracks=target_tracks(&p.request);
    let mut context_tracks:Vec<_>=p.positions.iter().filter(|n|!target_tracks.contains(&n.track)).map(|n|n.track).collect();
    context_tracks.sort_unstable();context_tracks.dedup();
    Ok(
        serde_json::json!({"targetTracks":target_tracks,"contextTracks":context_tracks,"contentId":p.request.content_id,"fingerprint":p.fingerprint,"request":p.request,"plans":result,"context":p.positions,"oppositeContext":p.opposite_positions,"jointStatus":joint_status,
            "scoreArticulations":{"paired":p.score_fingerprint.is_some(),"slurLinks":p.expression.iter().filter(|mark|mark.slur_from.is_some()).count(),
                "staccatoNotes":p.expression.iter().filter(|mark|mark.staccato).count()}}),
    )
}

/// Audit the marks that will actually remain after a selective acceptance.
/// Never substitute unaccepted suggestions for unsaved or retained positions.
/// Read-only action plans, distinct from onset-only acceptance until schedule persistence is connected.
pub fn held_plans(p:Prepared)->Result<serde_json::Value,String>{
    let hand=if p.request.hand==PracticePart::LeftHand{FingeringHand::Left}else{FingeringHand::Right};
    let targets:Vec<_>=p.positions.iter().enumerate().filter_map(|(i,n)|n.target.then_some(i)).collect();
    let plans=neothesia_core::fingering::suggest_fingering_plans_with_expression(&p.notes,hand,p.request.profile,&targets,3,p.request.practice_rate,true,&p.expression)
        .map_err(|e|format!("第 {} 小节不能安排安全的持音换指动作（{}），请调整固定指法、分手或速度",p.positions[e.index.min(p.positions.len()-1)].measure,e.kind))?;
    let result:Vec<_>=plans.iter().enumerate().map(|(rank,plan)|{
        let changes:Vec<_>=plan.substitutions.iter().map(|s|{let n=&p.positions[s.note];let before=&p.positions[s.before_note];
            serde_json::json!({"track":n.track,"index":n.index,"pitch":n.pitch,"measure":n.measure,"beforeTrack":before.track,"beforeIndex":before.index,
                "beforeMeasure":before.measure,"at":s.at.as_secs_f64(),"from":s.from,"to":s.to})}).collect();
        let proposals:Vec<_>=p.positions.iter().zip(&plan.suggestions).enumerate().filter(|(_, (n,_))|n.target).map(|(i,(n,f))|{
            let source_substituted=p.expression.get(i).and_then(|mark|mark.slur_from)
                .is_some_and(|from|plan.substitutions.iter().any(|change|change.note==from && change.before_note==i));
            let reason=if source_substituted {
                "谱面连线：前音保持发声，先在该音换指，再衔接此音"
            } else {
                expression_reason(&p.expression,&plan.suggestions,i).unwrap_or_else(||explanation(f.reason))
            };
            Proposal{track:n.track,index:n.index,pitch:n.pitch,measure:n.measure,finger:f.finger,saved:n.saved,confidence:f.confidence_percent,reason}
        }).collect();
        serde_json::json!({"rank":rank+1,"cost":plan.cost,"proposals":proposals,"substitutions":changes,"contextFingers":plan.suggestions.iter().map(|s|s.finger).collect::<Vec<_>>()})
    }).collect();
    Ok(serde_json::json!({"contentId":p.request.content_id,"request":p.request,"fingerprint":p.fingerprint,"context":p.positions,"plans":result,"readOnly":true,
        "scoreArticulations":{"paired":p.score_fingerprint.is_some(),"slurLinks":p.expression.iter().filter(|mark|mark.slur_from.is_some()).count(),
            "staccatoNotes":p.expression.iter().filter(|mark|mark.staccato).count()}}))
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
    let mut expression_review=Vec::new();
    let mut linked_unmarked=HashSet::new();
    let trusted_expression=if p.score_fingerprint.is_some() {p.expression.as_slice()} else {&[]};
    for (i,mark) in trusted_expression.iter().enumerate() {
        let Some(from)=mark.slur_from else {continue};
        if i>=p.positions.len() || from>=i {continue;}
        let a=&p.positions[from];let b=&p.positions[i];
        if !selected.contains_key(&(a.track,a.index)) && !selected.contains_key(&(b.track,b.index)) {continue;}
        let (Some(af),Some(bf))=(context_fingers[from],context_fingers[i]) else {
            if context_fingers[from].is_none() {linked_unmarked.insert(from);}
            if context_fingers[i].is_none() {linked_unmarked.insert(i);}
            continue;
        };
        let delta=i16::from(b.pitch)-i16::from(a.pitch);
        let outward=if hand==FingeringHand::Right {delta>0} else {delta<0};
        let crossing=if outward {bf==1 && af>=2} else {af==1 && bf>=2};
        let position_shift=delta.unsigned_abs()>12 || (!crossing && if outward {bf<af} else {bf>af});
        let reason=if a.pitch!=b.pitch && af==bf {
            "连线两音使用同一手指，换音时可能断开"
        } else if a.pitch!=b.pitch && position_shift {
            "连线中需要移动手位，请放慢检查衔接"
        } else {continue};
        expression_review.push(serde_json::json!({"contextIndex":i,"fromContextIndex":from,
            "track":b.track,"index":b.index,"measure":b.measure,"pitch":b.pitch,"finger":bf,
            "fromTrack":a.track,"fromIndex":a.index,"fromMeasure":a.measure,"fromPitch":a.pitch,"fromFinger":af,
            "reason":reason}));
    }
    let expression_review_truncated=expression_review.len()>100;expression_review.truncate(100);
    let first=p.positions.iter().filter(|n|n.target).map(|n|n.onset).fold(f64::INFINITY,f64::min);
    let last=p.positions.iter().filter(|n|n.target).map(|n|n.end).fold(0.0,f64::max);
    let context_unmarked=p.positions.iter().enumerate().filter(|(i,n)|!n.target && n.saved.is_none() &&
        (linked_unmarked.contains(i) || (n.track!=p.request.track && n.onset<last && (n.end>first || n.onset==first)))).count();
    // Keep identities from the source notes. A recommendation is not a saved
    // mark until its exact (track, index) edit is selected.
    let marked=|i:usize|selected.get(&(p.positions[i].track,p.positions[i].index)).copied().or(p.positions[i].saved);
    let mut coverage=Vec::new();
    let mut coverage_total=0usize;
    for (i,n) in p.positions.iter().enumerate() {
        let relevant_context=linked_unmarked.contains(&i) ||
            (!n.target && n.track!=p.request.track && n.onset<last && (n.end>first || n.onset==first));
        if !(n.target || relevant_context) || marked(i).is_some() {continue;}
        coverage_total+=1;
        if coverage.len()>=256 {continue;}
        let neighbor=|j:usize|{
            let pos=&p.positions[j];
            serde_json::json!({"track":pos.track,"index":pos.index,"measure":pos.measure,
                "pitch":pos.pitch,"finger":marked(j),"contextIndex":j})
        };
        let before=(0..i).rev().find(|&j|marked(j).is_some()).map(&neighbor);
        let after=(i+1..p.positions.len()).find(|&j|marked(j).is_some()).map(&neighbor);
        coverage.push(serde_json::json!({"track":n.track,"index":n.index,"measure":n.measure,
            "pitch":n.pitch,"contextIndex":i,"target":n.target,"before":before,"after":after}));
    }
    let tracks:Vec<_>=target_tracks(&p.request).into_iter().map(|track|{
        let positions:Vec<_>=p.positions.iter().filter(|n|n.track==track && n.target).collect();
        let selected_count=positions.iter().filter(|n|selected.contains_key(&(n.track,n.index))).count();
        let retained_count=positions.iter().filter(|n|!selected.contains_key(&(n.track,n.index)) && n.saved.is_some()).count();
        let changed=positions.iter().filter(|n|n.saved.is_some() && selected.get(&(n.track,n.index)).is_some_and(|f|Some(*f)!=n.saved)).count();
        let new=positions.iter().filter(|n|n.saved.is_none() && selected.contains_key(&(n.track,n.index))).count();
        serde_json::json!({"track":track,"total":positions.len(),"selected":selected_count,"retained":retained_count,
            "unmarked":positions.len()-selected_count-retained_count,"changed":changed,"new":new})
    }).collect();
    let marked_subset_valid=conflicts.is_empty();
    let saved_opposite:Vec<_>=p.opposite_notes.iter().map(|n|n.anchored_finger).collect();
    let cross_hand_review=cross_hand_reviews(p,&context_fingers,&saved_opposite,true);
    let full_passage_validated=marked_subset_valid && coverage_total==0 && review.is_empty()
        && expression_review.is_empty() && cross_hand_review.is_empty()
        && !review_truncated && !expression_review_truncated && !truncated;
    Ok(serde_json::json!({"contextUnmarked":context_unmarked,"valid":marked_subset_valid,"markedSubsetValid":marked_subset_valid,
        "fullPassageValidated":full_passage_validated,"coverage":coverage,"coverageTotal":coverage_total,"coverageTruncated":coverage_total>256,
        "conflicts":conflicts,"truncated":truncated,
        "review":review,"expressionReview":expression_review,"expressionReviewTruncated":expression_review_truncated,
        "crossHandReview":cross_hand_review,"tracks":tracks,"contextFingers":context_fingers,"reviewTruncated":review_truncated,"selected":selected.len(),"retained":retained,"unmarked":unknown,
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
            range:Some((1,1)),keep_saved:false,practice_rate:1.0,pins:vec![],target_tracks:vec![]},notes,positions,focus:(1,3),fingerprint:"test".into(),
            expression:vec![],score_fingerprint:None,opposite_notes:vec![],opposite_positions:vec![]}
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
        assert_eq!(r["markedSubsetValid"],true);assert_eq!(r["fullPassageValidated"],false);
        assert_eq!(r["coverageTotal"],1);assert_eq!(r["coverage"][0]["track"],2);
        assert_eq!(r["coverage"][0]["index"],2);assert_eq!(r["coverage"][0]["measure"],1);
        assert_eq!(r["coverage"][0]["target"],true);
        assert_eq!(r["coverage"][0]["before"]["index"],1);
        assert_eq!(r["coverage"][0]["before"]["finger"],3);
        assert!(r["coverage"][0]["after"].is_null());
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
        assert_eq!(r["coverageTotal"],2);assert_eq!(r["coverage"][0]["track"],7);
        assert_eq!(r["coverage"][0]["index"],1);assert_eq!(r["coverage"][0]["target"],false);
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
    #[test]
    fn acceptance_reviews_trusted_slur_after_selecting_or_retaining_fingers() {
        let mut p=fixture(PracticePart::RightHand,&[Some(1),Some(3),None]);
        p.score_fingerprint=Some("paired-score".into());
        p.expression=vec![FingeringExpression::default(),FingeringExpression::default(),
            FingeringExpression{slur_from:Some(1),..Default::default()}];
        let same=review_acceptance(&p,&[Edit{track_id:2,note_index:2,finger:Some(3)}]).unwrap();
        assert_eq!(same["valid"],true);
        assert_eq!(same["fullPassageValidated"],false);
        assert_eq!(same["expressionReview"][0]["fromFinger"],3);
        assert_eq!(same["expressionReview"][0]["finger"],3);
        assert_eq!(same["expressionReview"][0]["contextIndex"],2);
        let shift=review_acceptance(&p,&[Edit{track_id:2,note_index:2,finger:Some(2)}]).unwrap();
        assert_eq!(shift["expressionReview"][0]["finger"],2);
        let smooth=review_acceptance(&p,&[Edit{track_id:2,note_index:2,finger:Some(4)}]).unwrap();
        assert!(smooth["expressionReview"].as_array().unwrap().is_empty());
        p.positions[1].target=false;
        p.positions[1].saved=None;
        let missing=review_acceptance(&p,&[Edit{track_id:2,note_index:2,finger:Some(4)}]).unwrap();
        assert_eq!(missing["coverageTotal"],1);
        assert_eq!(missing["coverage"][0]["index"],1);
        assert_eq!(missing["contextUnmarked"],1);
        assert_eq!(missing["fullPassageValidated"],false);
        p.score_fingerprint=None;
        let unpaired=review_acceptance(&p,&[Edit{track_id:2,note_index:2,finger:Some(4)}]).unwrap();
        assert!(unpaired["expressionReview"].as_array().unwrap().is_empty());
    }
    #[test]
    fn held_plan_reports_only_trusted_score_expression_and_keeps_empty_evidence_neutral() {
        let mut p=fixture(PracticePart::RightHand,&[None,None,None]);
        p.notes.truncate(2);p.positions.truncate(2);
        p.notes[0]=FingeringNote{pitch:60,onset:Duration::ZERO,end:Duration::from_secs(3),anchored_finger:Some(3)};
        p.notes[1]=FingeringNote{pitch:64,onset:Duration::from_secs(1),end:Duration::from_millis(1500),anchored_finger:Some(3)};
        p.positions[0].target=true;p.positions[0].end=3.;
        p.positions[1].onset=1.;p.positions[1].end=1.5;
        let unmarked=held_plans(Prepared{expression:vec![],score_fingerprint:None,..fixture_for_held(&p)}).unwrap();
        assert_eq!(unmarked["scoreArticulations"]["paired"],false);
        assert_eq!(unmarked["scoreArticulations"]["slurLinks"],0);
        assert!(!unmarked["plans"][0]["proposals"][1]["reason"].as_str().unwrap().contains("谱面连线"));
        p.expression=vec![FingeringExpression::default(),FingeringExpression{slur_from:Some(0),..Default::default()}];
        p.score_fingerprint=Some("trusted-source".into());
        let marked=held_plans(p).unwrap();
        assert_eq!(marked["scoreArticulations"]["paired"],true);
        assert_eq!(marked["scoreArticulations"]["slurLinks"],1);
        assert!(marked["plans"][0]["cost"].as_i64().unwrap()>unmarked["plans"][0]["cost"].as_i64().unwrap());
        assert!(marked["plans"][0]["proposals"][1]["reason"].as_str().unwrap().contains("谱面连线"));
    }
    fn fixture_for_held(p:&Prepared)->Prepared {
        Prepared{request:p.request.clone(),notes:p.notes.clone(),positions:p.positions.iter().map(|n|Position{
            track:n.track,index:n.index,pitch:n.pitch,measure:n.measure,saved:n.saved,target:n.target,onset:n.onset,end:n.end
        }).collect(),focus:p.focus,fingerprint:p.fingerprint.clone(),expression:vec![],score_fingerprint:None,
            opposite_notes:vec![],opposite_positions:vec![]}
    }
    #[test]
    fn cross_hand_review_keeps_saved_other_hand_and_uses_accepted_finger() {
        let mut p=fixture(PracticePart::RightHand,&[Some(1),None,None]);
        p.opposite_notes.push(FingeringNote{pitch:66,onset:Duration::from_millis(500),
            end:Duration::from_millis(800),anchored_finger:Some(2)});
        p.opposite_positions.push(Position{track:7,index:4,pitch:66,measure:1,saved:Some(2),target:false,
            onset:0.5,end:0.8});
        let result=review_acceptance(&p,&[Edit{track_id:2,note_index:2,finger:Some(4)}]).unwrap();
        let review=&result["crossHandReview"][0];
        assert_eq!(review["kind"],"crossedRange");
        assert_eq!(review["finger"],4);
        assert_eq!(review["oppositeFinger"],2);
        assert_eq!(review["oppositeTrack"],7);
        assert_eq!(review["overlapMs"],300);
        assert_eq!(result["valid"],true);
        assert_eq!(result["fullPassageValidated"],false);
        let preserved=review_acceptance(&p,&[Edit{track_id:2,note_index:1,finger:Some(3)}]).unwrap();
        assert!(preserved["crossHandReview"].as_array().unwrap().is_empty());
    }
    #[test]
    fn joint_ranking_can_choose_higher_single_hand_cost_to_reduce_overlap() {
        let mut p=fixture(PracticePart::RightHand,&[Some(1),None,None]);
        p.opposite_notes.push(FingeringNote{pitch:66,onset:Duration::from_millis(500),
            end:Duration::from_millis(800),anchored_finger:Some(5)});
        p.opposite_positions.push(Position{track:7,index:4,pitch:66,measure:1,saved:Some(5),target:false,
            onset:0.5,end:0.8});
        let ranked=rank_joint_candidates(&p,&[(vec![1,3,5],0),(vec![1,3,1],2)],
            &[(vec![Some(5)],0)],2);
        assert_eq!(ranked[0].0,1);
        assert!(ranked[0].2<ranked[1].2);
        assert!(ranked[0].3<ranked[1].3);
    }
    #[test]
    fn unsaved_opposite_hand_has_visible_temporary_plan() {
        let mut p=fixture(PracticePart::RightHand,&[Some(1),None,None]);
        p.opposite_notes.push(FingeringNote{pitch:66,onset:Duration::from_millis(500),
            end:Duration::from_millis(800),anchored_finger:None});
        p.opposite_positions.push(Position{track:7,index:4,pitch:66,measure:1,saved:None,target:false,
            onset:0.5,end:0.8});
        let result=plans(p).unwrap();
        let candidate=&result["plans"][0];
        assert!(candidate["oppositeContextFingers"][0].as_u64().is_some());
        assert_eq!(candidate["crossHandReview"][0]["oppositeSaved"],false);
        assert_eq!(candidate["cost"].as_i64().unwrap(),
            candidate["primaryCost"].as_i64().unwrap()+candidate["oppositeCost"].as_i64().unwrap()
                +candidate["jointCost"].as_i64().unwrap());
    }
}
