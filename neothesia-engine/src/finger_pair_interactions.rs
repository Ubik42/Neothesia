use crate::fingerings::Position;
use serde::Serialize;
pub fn collect(file:&midi_file::MidiFile,config:&neothesia_core::song_config::SongConfig,assigned:&std::collections::HashMap<(usize,usize),neothesia_core::practice::PracticePart>,request:&crate::fingerings::PlanRequest,start:f64,end:f64)->Result<Vec<Position>,String>{
    let targets=crate::fingerings::target_tracks(request);
    let mut positions=Vec::new();
    for track in file.tracks.iter() {for (index,note) in track.notes.iter().enumerate(){
        let onset=note.start.as_secs_f64();let release=note.end.as_secs_f64();
        if onset>=end || release<=start || crate::hands::part(config,assigned,track.track_id,index)!=request.hand {continue;}
        if positions.len()>=16_384{return Err("同手选段上下文超过 16384 个音符，请缩小范围后检查两手交接".into());}
        let tick=file.tempo_track.seconds_to_pulses(onset).round() as u64;
        positions.push(Position{track:track.track_id,index,pitch:note.note,measure:file.musical_time.measures.partition_point(|m|m.start_tick<=tick).max(1),saved:None,target:targets.contains(&track.track_id)&&onset>=start,onset,end:release});
    }}
    Ok(positions)
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Interaction {
    pub kind: &'static str, pub measure:usize, pub from_measure:usize,
    pub left_track:usize,pub left_index:usize,pub right_track:usize,pub right_index:usize,
    pub left_pitch:u8,pub right_pitch:u8,pub at:f64,pub end:f64,pub overlap_ms:u64,
    pub left_started_before_range:bool,pub right_started_before_range:bool,
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Analysis {pub items:Vec<Interaction>,pub same_key:usize,pub crossed_range:usize,pub truncated:bool}
/// Half-open MIDI key-down intervals: a hand may release a key at the exact
/// moment the other hand takes it. Pedal sustain is not key occupancy.
pub fn analyze(left:&[Position],right:&[Position],start:f64,end:f64,rate:f64)->Analysis{
    let mut events:Vec<_>=left.iter().filter(|n|n.onset<end&&n.end>start&&n.end>n.onset).map(|n|(n.onset.max(start),false,n)).chain(right.iter().filter(|n|n.onset<end&&n.end>start&&n.end>n.onset).map(|n|(n.onset.max(start),true,n))).collect();
    events.sort_by(|a,b|a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.track.cmp(&b.2.track)).then(a.2.index.cmp(&b.2.index)));
    let mut active:[Vec<&Position>;2]=[vec![],vec![]];
    let mut result=Analysis{items:vec![],same_key:0,crossed_range:0,truncated:false};
    let mut checked=0usize;
    for (at,is_right,note) in events {
        active[0].retain(|p|p.end>at+1e-9);active[1].retain(|p|p.end>at+1e-9);
        for other in &active[usize::from(!is_right)] {
            checked+=1;if checked>100_000 {result.truncated=true;return result;}
            let (l,r)=if is_right {(*other,note)}else{(note,*other)};
            if !l.target&&!r.target{continue;}
            let until=l.end.min(r.end).min(end);if until<=at+1e-9{continue;}
            let kind=if l.pitch==r.pitch {result.same_key+=1;"sameKeyHandoff"}else if l.pitch>r.pitch{result.crossed_range+=1;"crossedRange"}else{continue;};
            if result.items.len()<200 {
                result.items.push(Interaction{kind,measure:if l.onset>=r.onset{l.measure}else{r.measure},from_measure:if l.onset<r.onset{l.measure}else{r.measure},left_track:l.track,left_index:l.index,right_track:r.track,right_index:r.index,left_pitch:l.pitch,right_pitch:r.pitch,at,end:until,overlap_ms:((until-at)/rate*1000.).round() as u64,left_started_before_range:l.onset<start,right_started_before_range:r.onset<start});
            }else{result.truncated=true;}
        }
        active[usize::from(is_right)].push(note);
    }
    result
}
#[cfg(test)]
mod tests {
 use super::*;
 fn p(index:usize,pitch:u8,onset:f64,end:f64,target:bool)->Position{Position{track:index,index,pitch,measure:index+1,saved:None,target,onset,end}}
 #[test]fn held_handoff_range_boundary_rate_and_crossed_notes(){
  let left=vec![p(0,60,0.,2.,false),p(1,72,2.,3.,true)];
  let right=vec![p(2,60,1.,2.,true),p(3,65,2.,3.,true),p(4,72,3.,4.,true)];
  let a=analyze(&left,&right,1.,4.,0.5);
  assert_eq!(a.same_key,1);assert_eq!(a.crossed_range,1);assert_eq!(a.items[0].overlap_ms,2000);assert!(a.items[0].left_started_before_range);
  assert_eq!(a.items[1].at,2.);assert_eq!(a.items[1].end,3.);assert!(!a.truncated);
  assert!(analyze(&[p(0,60,0.,1.,true)],&[p(1,60,1.,2.,true)],0.,2.,1.).items.is_empty());
  assert!(analyze(&[p(0,60,0.,2.,false)],&[p(1,60,1.,2.,false)],0.,2.,1.).items.is_empty());
 }
}
