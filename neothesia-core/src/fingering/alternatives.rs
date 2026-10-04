use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};
#[derive(Debug,Clone,Eq,PartialEq,Ord,PartialOrd)]
pub struct FingerSubstitution { pub note:usize, pub before_note:usize, pub at:Duration, pub from:u8, pub to:u8 }
#[derive(Debug)]
pub struct FingeringPlan {
    pub substitutions: Vec<FingerSubstitution>,
    pub suggestions: Vec<FingerSuggestion>,
    pub cost: i64,
}
#[derive(Debug)]
pub struct FingeringSearchFailure {
    pub index: usize,
    pub kind: &'static str,
}
struct Node {
    parent: Option<usize>,
    start: usize,
    fingers: Vec<u8>,
    substitutions: Vec<FingerSubstitution>,
}
#[derive(Clone)]
struct State {
    node: Option<usize>,
    cost: i64,
    held: Vec<(usize, u8)>,
    last_change:BTreeMap<usize,Duration>,
    focus: u64,
}
struct Pending {
    parent: Option<usize>,
    cost: i64,
    held: Vec<(usize, u8)>,
    last_change:BTreeMap<usize,Duration>,
    focus: u64,
    fingers: Vec<u8>,
    substitutions: Vec<FingerSubstitution>,
}
/// Bounded k-best path search. Adjacent chords and all still-held notes share
/// the same state; target-prefix diversity survives context-only differences.
pub fn suggest_fingering_plans(
    notes: &[FingeringNote],
    hand: FingeringHand,
    profile: HandSpanProfile,
    focus: Range<usize>,
    limit: usize,
) -> Result<Vec<FingeringPlan>, FingeringSearchFailure> {
    suggest_fingering_plans_at_rate(notes, hand, profile, focus, limit, 1.0)
}

/// Rate affects physical transition time, while source note identities stay intact.
pub fn suggest_fingering_plans_at_rate(
    notes: &[FingeringNote],
    hand: FingeringHand,
    profile: HandSpanProfile,
    focus: Range<usize>,
    limit: usize,
    rate: f64,
) -> Result<Vec<FingeringPlan>, FingeringSearchFailure> {
    if focus.is_empty() || focus.end > notes.len() {
        return Err(FingeringSearchFailure { index:0,kind:"range" });
    }
    suggest_fingering_plans_for_indices(notes,hand,profile,&focus.collect::<Vec<_>>(),limit,rate)
}

/// Target notes may be interleaved with read-only notes from other tracks.
pub fn suggest_fingering_plans_for_indices(
    notes: &[FingeringNote], hand: FingeringHand, profile: HandSpanProfile,
    focus: &[usize], limit: usize, rate: f64,
) -> Result<Vec<FingeringPlan>, FingeringSearchFailure> {
    suggest_fingering_plans_with_substitutions(notes,hand,profile,focus,limit,rate,false)
}

/// Explicitly enabled held-note substitution search. Read-only context may never be changed.
pub fn suggest_fingering_plans_with_substitutions(
    notes:&[FingeringNote],hand:FingeringHand,profile:HandSpanProfile,focus:&[usize],limit:usize,rate:f64,allow:bool,
)->Result<Vec<FingeringPlan>,FingeringSearchFailure>{
    let error = |index, kind| FingeringSearchFailure { index, kind };
    if !rate.is_finite() || !(0.25..=2.0).contains(&rate) {
        return Err(error(0, "rate"));
    }
    if notes.is_empty() || notes.len() > 4608 || focus.is_empty()
        || focus.last().is_some_and(|i| *i>=notes.len()) || focus.windows(2).any(|w|w[0]>=w[1]) {
        return Err(error(0, "range"));
    }
    if notes.windows(2).any(|n| n[0].onset > n[1].onset) {
        return Err(error(0, "order"));
    }
    if let Some(i) = notes
        .iter()
        .position(|n| n.anchored_finger.is_some_and(|f| !(1..=5).contains(&f)) || n.end < n.onset)
    {
        return Err(error(i, "anchor"));
    }
    let count = limit.clamp(1, 5);
    let mut arena: Vec<Node> = vec![];
    let mut beam = vec![State {
        node: None,
        cost: 0,
        held: vec![],
        last_change:BTreeMap::new(),
        focus: 0xcbf29ce484222325,
    }];
    let mut groups = vec![];
    let mut cursor = 0;
    while cursor < notes.len() {
        let start = cursor;
        while cursor < notes.len() && notes[cursor].onset == notes[start].onset {
            cursor += 1;
        }
        groups.push((start, cursor));
    }
    for (g, &(start, end)) in groups.iter().enumerate() {
        let current = &notes[start..end];
        let candidates = if current.len() == 1 {
            (1..=5)
                .filter(|f| finger_allowed(current[0], *f))
                .map(|f| ChordCandidate {
                    shape_cost: static_key_cost(current[0].pitch, f as u8),
                    fingers: vec![f as u8],
                })
                .collect::<Vec<_>>()
        } else {
            joint_chord_candidates(current, hand, profile)
        };
        if candidates.is_empty() {
            return Err(error(start, "shape"));
        }
        let mut buckets: BTreeMap<(Vec<u8>, Vec<(usize, u8)>,Vec<(usize,Duration)>), Vec<Pending>> = BTreeMap::new();
        for prior in &beam {
            let original_held: Vec<_> = prior
                .held
                .iter()
                .copied()
                .filter(|(i, _)| notes[*i].end > current[0].onset)
                .collect();
            for candidate in &candidates {
                let variants=held_variants(notes,&original_held,&prior.held,&prior.last_change,current,&candidate.fingers,hand,focus,rate,allow,start);
                for (held,substitutions) in variants {
                let held_notes:Vec<_>=held.iter().map(|(i,_)|notes[*i]).collect();
                let held_fingers:Vec<_>=held.iter().map(|(_,f)|*f).collect();
                let mut addition = i64::from(candidate.shape_cost)+substitutions.iter().map(|s|48+i64::from(static_key_cost(notes[s.note].pitch,s.to))).sum::<i64>();
                if g == 0
                    || (held.is_empty()
                        && groups[g - 1].0 < start
                        && notes[groups[g - 1].0..groups[g - 1].1]
                            .iter()
                            .map(|n| n.end)
                            .max()
                            .unwrap()
                            + Duration::from_millis(600)
                            < current[0].onset)
                {
                    if current.len() == 1 {
                        addition =
                            i64::from(start_cost(&notes[start..], hand, candidate.fingers[0]));
                    }
                } else if let Some(node) = prior.node {
                    let old = &arena[node];
                    let previous = &notes[old.start..old.start + old.fingers.len()];
                    let previous_fingers:Vec<_>=old.fingers.iter().enumerate().map(|(i,f)|held.iter().find(|(j,_)|*j==old.start+i).map_or(*f,|(_,f)|*f)).collect();
                    if previous.len() == 1 && current.len() == 1 {
                        addition += i64::from(if rapid_repeated_transition_at_rate(notes, start, rate) {
                            rapid_repeat_cost(previous_fingers[0], candidate.fingers[0])
                        } else {
                            transition_cost(
                                previous[0].pitch,
                                previous_fingers[0],
                                current[0].pitch,
                                candidate.fingers[0],
                                hand,
                                profile,
                            )
                        });
                        addition += i64::from(timed_movement_cost(
                            previous[0], previous_fingers[0], current[0], candidate.fingers[0],
                            hand, profile, rate,
                        ));
                    } else {
                        let connection = chord_transition_cost(
                            previous,
                            &previous_fingers,
                            current,
                            &candidate.fingers,
                            hand,
                        );
                        if connection == INFINITY {
                            continue;
                        }
                        addition += i64::from(connection);
                        for (n, f) in current.iter().zip(&candidate.fingers) {
                            addition += i64::from(
                                previous
                                    .iter()
                                    .zip(&previous_fingers)
                                    .map(|(p, pf)| {
                                        transition_cost(p.pitch, *pf, n.pitch, *f, hand, profile)
                                            + timed_movement_cost(*p, *pf, *n, *f, hand, profile, rate)
                                    })
                                    .min()
                                    .unwrap_or(0),
                            ) / current.len() as i64;
                        }
                    }
                }
                // Held intervals also contribute reach cost; holding a bass and adding a
                // melodic note must not evade the selected hand-span model.
                for (prior_note, pf) in held_notes.iter().zip(&held_fingers) {
                    for (n, f) in current.iter().zip(&candidate.fingers) {
                        let gap = usize::from(pf.abs_diff(*f));
                        addition += i64::from(
                            (i32::from(prior_note.pitch.abs_diff(n.pitch))
                                - profile.comfortable_spans()[gap])
                                .max(0)
                                * 8,
                        );
                    }
                }
                let mut next_held = held.clone();
                next_held.extend((start..end).zip(candidate.fingers.iter().copied()));
                let mut signature = prior.focus;
                for (i, f) in (start..end).zip(&candidate.fingers) {
                    if focus.binary_search(&i).is_ok() {
                        signature ^= u64::from(*f);
                        signature = signature.wrapping_mul(0x100000001b3);
                    }
                }
                for change in &substitutions {
                    for v in [change.note as u64,change.from as u64,change.to as u64,change.before_note as u64] { signature^=v;signature=signature.wrapping_mul(0x100000001b3); }
                }
                let mut last_change=prior.last_change.clone();last_change.retain(|i,_|next_held.iter().any(|(j,_)|j==i));
                for change in &substitutions{last_change.insert(change.note,change.at);}
                buckets
                    .entry((candidate.fingers.clone(), next_held.clone(),last_change.iter().map(|(i,t)|(*i,*t)).collect()))
                    .or_default()
                    .push(Pending {
                        parent: prior.node,
                        cost: prior.cost + addition,
                        held: next_held,
                        last_change,
                        focus: signature,
                        fingers: candidate.fingers.clone(),
                        substitutions,
                    });
                }
            }
        }
        let mut retained = vec![];
        for mut bucket in buckets.into_values() {
            bucket.sort_by_key(|p| p.cost);
            let mut seen = BTreeSet::new();
            retained.extend(
                bucket
                    .into_iter()
                    .filter(|p| seen.insert(p.focus))
                    .take(count),
            );
        }
        retained.sort_by_key(|p| p.cost);
        retained.truncate(128);
        if retained.is_empty() {
            return Err(error(start, "held"));
        }
        beam = retained
            .into_iter()
            .map(|p| {
                let node = arena.len();
                arena.push(Node {
                    parent: p.parent,
                    start,
                    fingers: p.fingers,
                    substitutions:p.substitutions,
                });
                State {
                    node: Some(node),
                    cost: p.cost,
                    held: p.held,
                    last_change:p.last_change,
                    focus: p.focus,
                }
            })
            .collect();
    }
    beam.sort_by_key(|p| p.cost);
    let mut plans = vec![];
    let mut seen = BTreeSet::new();
    for state in beam {
        let mut fingers = vec![0; notes.len()];
        let mut substitutions=Vec::new();
        let mut node = state.node;
        while let Some(i) = node {
            let n = &arena[i];
            fingers[n.start..n.start + n.fingers.len()].copy_from_slice(&n.fingers);
            substitutions.extend(n.substitutions.clone());
            node = n.parent;
        }
        substitutions.sort_by_key(|s|(s.at,s.note));
        if !seen.insert((focus.iter().map(|i|fingers[*i]).collect::<Vec<_>>(),substitutions.clone())) {
            continue;
        }
        let mut suggestions = Vec::with_capacity(notes.len());
        for (g, &(start, end)) in groups.iter().enumerate() {
            let held = (0..start).any(|i| notes[i].end > notes[start].onset);
            let wide = (start..end).any(|i| {
                (start..end).any(|j| {
                    i32::from(notes[i].pitch.abs_diff(notes[j].pitch))
                        > profile.comfortable_spans()[usize::from(fingers[i].abs_diff(fingers[j]))]
                })
            });
            for i in start..end {
                let reason = if notes[i].anchored_finger.is_some() {
                    FingeringReason::ManualAnchor
                } else if wide {
                    FingeringReason::WideChordShape
                } else if held {
                    FingeringReason::HeldChordPosition
                } else if end - start > 1 {
                    if g > 0
                        && (groups[g - 1].0..groups[g - 1].1)
                            .any(|j| notes[j].pitch == notes[i].pitch && fingers[j] == fingers[i])
                    {
                        FingeringReason::ChordConnection
                    } else {
                        FingeringReason::ChordShape
                    }
                } else if g == 0
                    || notes[groups[g - 1].0..groups[g - 1].1]
                        .iter()
                        .map(|n| n.end)
                        .max()
                        .unwrap()
                        + Duration::from_millis(600)
                        < notes[i].onset
                {
                    FingeringReason::PhraseStart
                } else if g > 0
                    && groups[g - 1].1 - groups[g - 1].0 == 1
                    && rapid_repeated_transition_at_rate(notes, i, rate)
                {
                    FingeringReason::RapidRepeatedNote
                } else {
                    let previous = groups[g - 1].0;
                    transition_reason(
                        notes[previous].pitch,
                        fingers[previous],
                        notes[i].pitch,
                        fingers[i],
                        hand,
                    )
                };
                suggestions.push(FingerSuggestion {
                    finger: fingers[i],
                    confidence_percent: confidence(reason),
                    reason,
                });
            }
        }
        if allow && validate_substitution_plan(notes,&fingers,&substitutions,hand,focus,rate).is_err(){continue;}
        plans.push(FingeringPlan {
            suggestions,
            substitutions,
            cost: state.cost,
        });
        if plans.len() == count {
            break;
        }
    }
    if plans.is_empty(){return Err(error(0,"substitution"));}
    Ok(plans)
}

fn held_variants(notes:&[FingeringNote],held:&[(usize,u8)],occupied:&[(usize,u8)],last_change:&BTreeMap<usize,Duration>,current:&[FingeringNote],fingers:&[u8],hand:FingeringHand,focus:&[usize],rate:f64,allow:bool,before:usize)->Vec<(Vec<(usize,u8)>,Vec<FingerSubstitution>)>{
    let old:Vec<_>=held.iter().map(|(i,_)|notes[*i]).collect();let old_fingers:Vec<_>=held.iter().map(|(_,f)|*f).collect();
    if held.is_empty()||held_transition_valid(&old,&old_fingers,current,fingers,hand){return vec![(held.to_vec(),vec![])];}
    if !allow{return vec![];}
    let Some(at)=current[0].onset.checked_sub(Duration::from_secs_f64(0.12*rate)) else{return vec![];};
    let mut result=vec![];
    for (slot,&(index,from)) in held.iter().enumerate(){
        // A conservative minimum action interval; duplicated physical keys need a joint action and remain unsupported.
        if focus.binary_search(&index).is_err()||last_change.get(&index).copied().unwrap_or(notes[index].onset)+Duration::from_secs_f64(0.18*rate)>at
            ||held.iter().filter(|(i,_)|notes[*i].pitch==notes[index].pitch).count()!=1 {continue;}
        for to in 1..=5 {
            if to==from||occupied.iter().any(|(i,f)|*f==to&&notes[*i].pitch!=notes[index].pitch&&notes[*i].onset<=at&&notes[*i].end>at){continue;}
            let mut changed=old_fingers.clone();changed[slot]=to;
            if !held_transition_valid(&old,&changed,&old,&changed,hand)||!held_transition_valid(&old,&changed,current,fingers,hand){continue;}
            let occupied_notes:Vec<_>=occupied.iter().filter(|(i,_)|notes[*i].onset<=at&&notes[*i].end>at).map(|(i,_)|notes[*i]).collect();
            let occupied_fingers:Vec<_>=occupied.iter().filter(|(i,_)|notes[*i].onset<=at&&notes[*i].end>at).map(|(i,f)|if *i==index{to}else{*f}).collect();
            if !held_transition_valid(&occupied_notes,&occupied_fingers,&occupied_notes,&occupied_fingers,hand){continue;}
            let mut next=held.to_vec();next[slot].1=to;
            result.push((next,vec![FingerSubstitution{note:index,before_note:before,at,from,to}]));
        }
    }result
}

/// Validate onset assignments and every silent substitution as one physical schedule.
pub fn validate_substitution_plan(notes:&[FingeringNote],fingers:&[u8],changes:&[FingerSubstitution],hand:FingeringHand,focus:&[usize],rate:f64)->Result<(),FingeringSearchFailure>{
    let err=|index,kind|FingeringSearchFailure{index,kind};
    if notes.is_empty()||notes.len()!=fingers.len()||notes.len()>4608||changes.len()>4096||!rate.is_finite()||!(0.25..=2.).contains(&rate)
        ||focus.windows(2).any(|w|w[0]>=w[1])||focus.iter().any(|i|*i>=notes.len())||notes.windows(2).any(|w|w[0].onset>w[1].onset){return Err(err(0,"range"));}
    for (i,(n,f)) in notes.iter().zip(fingers).enumerate(){if !(1..=5).contains(f)||n.end<n.onset||n.anchored_finger.is_some_and(|a|a!=*f){return Err(err(i,"anchor"));}}
    if changes.windows(2).any(|w|(w[0].at,w[0].note)>=(w[1].at,w[1].note)){return Err(err(0,"substitution-order"));}
    let mut events:Vec<_>=notes.iter().enumerate().map(|(i,n)|(n.onset,1,i)).chain(changes.iter().enumerate().map(|(i,s)|(s.at,0,i))).collect();events.sort();
    let mut effective=fingers.to_vec();let mut last=BTreeMap::new();
    for (time,kind,index) in events {
        if kind==0 {
            let c=&changes[index];
            if c.note>=notes.len()||c.before_note>=notes.len()||focus.binary_search(&c.note).is_err()||!(1..=5).contains(&c.to)||c.from==c.to||effective[c.note]!=c.from
                ||notes[c.note].onset>=time||notes[c.note].end<=notes[c.before_note].onset
                ||time+Duration::from_secs_f64(0.12*rate)>notes[c.before_note].onset
                ||last.get(&c.note).copied().unwrap_or(notes[c.note].onset)+Duration::from_secs_f64(0.18*rate)>time {return Err(err(c.note,"substitution"));}
            // The recipient must be free before the finger releases the key.
            if notes.iter().enumerate().any(|(i,n)|i!=c.note&&n.onset<=time&&n.end>time&&effective[i]==c.to){return Err(err(c.note,"occupied"));}
            effective[c.note]=c.to;last.insert(c.note,time);
        }
        let active:Vec<_>=notes.iter().enumerate().filter(|(_,n)|n.onset<=time&&n.end>time).collect();
        let active_notes:Vec<_>=active.iter().map(|(_,n)|**n).collect();let active_fingers:Vec<_>=active.iter().map(|(i,_)|effective[*i]).collect();
        if !active_notes.is_empty()&&!held_transition_valid(&active_notes,&active_fingers,&active_notes,&active_fingers,hand){return Err(err(if kind==0{changes[index].note}else{index},"held"));}
    }Ok(())
}

fn joint_chord_candidates(notes:&[FingeringNote], hand:FingeringHand, profile:HandSpanProfile) -> Vec<ChordCandidate> {
    let mut unique=Vec::<FingeringNote>::new(); let mut indices=vec![];
    for n in notes {
        if let Some(i)=unique.iter().position(|p|p.pitch==n.pitch) {
            if unique[i].anchored_finger.zip(n.anchored_finger).is_some_and(|(a,b)|a!=b) {return vec![];}
            unique[i].anchored_finger=unique[i].anchored_finger.or(n.anchored_finger);
            unique[i].end=unique[i].end.max(n.end);indices.push(i);
        } else {indices.push(unique.len());unique.push(*n);}
    }
    let candidates=if unique.len()==1 {
        (1..=5).filter(|f|finger_allowed(unique[0],*f)).map(|f|ChordCandidate{
            shape_cost:static_key_cost(unique[0].pitch,f as u8),fingers:vec![f as u8]}).collect()
    } else {chord_candidates(&unique,hand,profile)};
    candidates.into_iter().map(|c|ChordCandidate{
        shape_cost:c.shape_cost,fingers:indices.iter().map(|i|c.fingers[*i]).collect()}).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn n(pitch: u8, start: u64, end: u64, anchor: Option<u8>) -> FingeringNote {
        FingeringNote {
            pitch,
            onset: Duration::from_millis(start),
            end: Duration::from_millis(end),
            anchored_finger: anchor,
        }
    }
    #[test]
    fn interleaved_targets_are_distinct_and_shared_key_anchors_agree() {
        let notes=vec![n(60,0,100,Some(1)),n(64,200,300,None),n(67,400,500,Some(5)),n(65,600,700,None)];
        let plans=suggest_fingering_plans_for_indices(&notes,FingeringHand::Right,HandSpanProfile::Standard,&[1,3],3,1.0).unwrap();
        assert_eq!(plans.len(),3);
        let signatures:BTreeSet<_>=plans.iter().map(|p|vec![p.suggestions[1].finger,p.suggestions[3].finger]).collect();
        assert_eq!(signatures.len(),3);
        assert!(plans.iter().all(|p|p.suggestions[0].finger==1 && p.suggestions[2].finger==5));
        let shared=vec![n(60,0,500,Some(1)),n(60,0,800,None),n(64,0,500,None)];
        let plans=suggest_fingering_plans_for_indices(&shared,FingeringHand::Right,HandSpanProfile::Standard,&[1,2],3,1.0).unwrap();
        assert!(plans.iter().all(|p|p.suggestions[0].finger==1 && p.suggestions[1].finger==1 && p.suggestions[2].finger>1));
        let mut conflict=shared;conflict[1].anchored_finger=Some(2);
        assert_eq!(suggest_fingering_plans_for_indices(&conflict,FingeringHand::Right,HandSpanProfile::Standard,&[1,2],3,1.0).unwrap_err().kind,"shape");
    }
    #[test]
    fn tempo_changes_transition_cost_without_changing_anchors_and_repeats_follow_rate() {
        for (hand, pitches) in [(FingeringHand::Right, [64,65]), (FingeringHand::Left,[65,64])] {
            let notes = vec![n(pitches[0],0,100,Some(3)),n(pitches[1],240,350,Some(1))];
            let slow = suggest_fingering_plans_at_rate(&notes,hand,HandSpanProfile::Standard,0..2,3,1.0).unwrap();
            let fast = suggest_fingering_plans_at_rate(&notes,hand,HandSpanProfile::Standard,0..2,3,2.0).unwrap();
            assert!(fast[0].cost > slow[0].cost);
            assert_eq!(fast[0].suggestions.iter().map(|f| f.finger).collect::<Vec<_>>(),vec![3,1]);
        }
        let repeated=vec![n(60,0,100,None),n(60,400,500,None),n(60,800,900,None)];
        let slow=suggest_fingering_plans_at_rate(&repeated,FingeringHand::Right,HandSpanProfile::Standard,0..3,1,0.5).unwrap();
        let fast=suggest_fingering_plans_at_rate(&repeated,FingeringHand::Right,HandSpanProfile::Standard,0..3,1,2.0).unwrap();
        assert_eq!(slow[0].suggestions[0].finger,slow[0].suggestions[1].finger);
        assert_ne!(fast[0].suggestions[0].finger,fast[0].suggestions[1].finger);
        assert_eq!(fast[0].suggestions[1].reason,FingeringReason::RapidRepeatedNote);
        assert!(suggest_fingering_plans_at_rate(&repeated,FingeringHand::Right,HandSpanProfile::Standard,0..3,1,f64::NAN).is_err());
    }
    #[test]
    fn alternatives_are_distinct_and_respect_all_held_notes() {
        let notes = vec![
            n(60, 0, 3000, Some(1)),
            n(64, 500, 900, None),
            n(67, 1000, 1400, None),
            n(65, 1500, 1900, None),
        ];
        let plans = suggest_fingering_plans(
            &notes,
            FingeringHand::Right,
            HandSpanProfile::Standard,
            1..4,
            3,
        )
        .unwrap();
        assert_eq!(plans.len(), 3);
        let mut seen = BTreeSet::new();
        for p in &plans {
            assert_eq!(p.suggestions[0].finger, 1);
            assert!(p.suggestions[1..].iter().all(|f| f.finger > 1));
            assert!(
                seen.insert(
                    p.suggestions[1..]
                        .iter()
                        .map(|f| f.finger)
                        .collect::<Vec<_>>()
                )
            );
        }
        assert!(plans.windows(2).all(|p| p[0].cost <= p[1].cost));
    }
    #[test]
    fn incompatible_pinned_held_finger_is_reported() {
        let notes = vec![n(60, 0, 2000, Some(1)), n(64, 500, 1000, Some(1))];
        let error = suggest_fingering_plans(
            &notes,
            FingeringHand::Right,
            HandSpanProfile::Standard,
            0..2,
            3,
        )
        .unwrap_err();
        assert_eq!(error.index, 1);
        assert_eq!(error.kind, "held");
    }
    #[test]
    fn pinned_chord_has_only_one_distinct_target() {
        let notes = vec![
            n(60, 0, 500, Some(1)),
            n(64, 0, 500, Some(3)),
            n(67, 0, 500, Some(5)),
        ];
        let p = suggest_fingering_plans(
            &notes,
            FingeringHand::Right,
            HandSpanProfile::Standard,
            0..3,
            3,
        )
        .unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(
            p[0].suggestions
                .iter()
                .map(|f| f.finger)
                .collect::<Vec<_>>(),
            vec![1, 3, 5]
        );
    }
}

#[cfg(test)]
mod substitution_tests {
 use super::*;
 fn note(pitch:u8,onset:f64,end:f64,finger:u8)->FingeringNote{FingeringNote{pitch,onset:Duration::from_secs_f64(onset),end:Duration::from_secs_f64(end),anchored_finger:Some(finger)}}
 #[test]
 fn opt_in_preserves_attack_and_frees_held_finger_for_both_hands(){
  for (hand,pitches) in [(FingeringHand::Right,[60,64]),(FingeringHand::Left,[64,60])]{
   let notes=[note(pitches[0],0.,3.,3),note(pitches[1],1.,1.5,3)];
   assert!(suggest_fingering_plans_for_indices(&notes,hand,HandSpanProfile::Standard,&[0,1],3,1.).is_err());
   let plans=suggest_fingering_plans_with_substitutions(&notes,hand,HandSpanProfile::Standard,&[0,1],3,1.,true).unwrap();
   assert!(!plans.is_empty());
   for p in plans{validate_substitution_plan(&notes,&p.suggestions.iter().map(|s|s.finger).collect::<Vec<_>>(),&p.substitutions,hand,&[0,1],1.).unwrap();assert_eq!(p.suggestions.iter().map(|s|s.finger).collect::<Vec<_>>(),vec![3,3]);assert_eq!(p.substitutions.len(),1);let s=&p.substitutions[0];assert_eq!((s.note,s.before_note,s.from),(0,1,3));assert!([1,2].contains(&s.to));assert_eq!(s.at,Duration::from_millis(880));}
   assert!(suggest_fingering_plans_with_substitutions(&notes,hand,HandSpanProfile::Standard,&[1],3,1.,true).is_err());
  }
 }
 #[test]
 fn occupied_before_next_onset_and_rate_window_are_respected(){
  let notes=[note(60,0.,0.95,1),note(62,0.,3.,3),note(64,1.,1.5,3)];
  let plans=suggest_fingering_plans_with_substitutions(&notes,FingeringHand::Right,HandSpanProfile::Standard,&[0,1,2],3,1.,true).unwrap();
  assert!(plans.iter().all(|p|p.substitutions[0].note==1&&p.substitutions[0].to==2));
  let notes=[note(60,0.,3.,3),note(64,0.45,1.,3)];
  assert!(suggest_fingering_plans_with_substitutions(&notes,FingeringHand::Right,HandSpanProfile::Standard,&[0,1],3,1.,true).is_ok());
  assert!(suggest_fingering_plans_with_substitutions(&notes,FingeringHand::Right,HandSpanProfile::Standard,&[0,1],3,2.,true).is_err());
 }
 #[test]
 fn schedule_validation_rejects_wrong_from_read_only_and_back_to_back_actions(){
  let notes=[note(60,0.,3.,3),note(64,1.,1.05,3),note(65,1.1,1.5,2)];
  let first=FingerSubstitution{note:0,before_note:1,at:Duration::from_millis(880),from:3,to:1};
  validate_substitution_plan(&notes,&[3,3,2],&[first.clone()],FingeringHand::Right,&[0,1,2],1.).unwrap();
  assert!(validate_substitution_plan(&notes,&[3,3,2],&[first.clone()],FingeringHand::Right,&[1,2],1.).is_err());
  let wrong=FingerSubstitution{from:4,..first.clone()};
  assert!(validate_substitution_plan(&notes,&[3,3,2],&[wrong],FingeringHand::Right,&[0,1,2],1.).is_err());
  let first=FingerSubstitution{to:2,..first};
  let second=FingerSubstitution{note:0,before_note:2,at:Duration::from_millis(980),from:2,to:1};
  assert!(validate_substitution_plan(&notes,&[3,3,2],&[first,second],FingeringHand::Right,&[0,1,2],1.).is_err());
 }
}
