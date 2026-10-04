use midi_file::MidiFile;
use neothesia_core::{library::NoteHandHint, practice::PracticePart, song_config::SongConfig};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc, time::Duration};

pub fn part(
    config: &SongConfig,
    assigned: &HashMap<(usize, usize), PracticePart>,
    track: usize,
    index: usize,
) -> PracticePart {
    assigned.get(&(track, index)).copied().unwrap_or_else(|| {
        config
            .tracks
            .iter()
            .find(|t| t.track_id == track)
            .map_or(PracticePart::Other, |t| t.practice_part)
    })
}
pub fn label(part: PracticePart) -> &'static str {
    match part {
        PracticePart::LeftHand => "left",
        PracticePart::RightHand => "right",
        _ => "other",
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub track: usize,
    pub index: usize,
    pub pitch: u8,
    pub start: f64,
    pub measure: usize,
    pub part: PracticePart,
    pub saved: PracticePart,
    pub confidence: u8,
    pub reason: &'static str,
}
struct Chain {
    previous: Option<Arc<Chain>>,
    assignments: Vec<(usize, PracticePart, u8, &'static str)>,
}
#[derive(Clone)]
struct Beam {
    cost: f64,
    left: f64,
    right: f64,
    held: Vec<(u8, Duration, PracticePart)>,
    chain: Option<Arc<Chain>>,
}
pub fn propose(
    file: &MidiFile,
    config: &SongConfig,
    assigned: &HashMap<(usize, usize), PracticePart>,
    track: usize,
    range: (usize, usize),
    reference: u8,
) -> Result<Vec<Proposal>, String> {
    let t = file
        .tracks
        .iter()
        .find(|t| t.track_id == track)
        .ok_or("音轨不存在")?;
    let (a, b) = range;
    if a == 0 || a > b || b > file.musical_time.measures.len() || reference > 127 {
        return Err("分手参考位置或小节范围无效".into());
    }
    let start = file
        .tempo_track
        .pulses_to_duration(file.musical_time.measures[a - 1].start_tick);
    let end = file
        .tempo_track
        .pulses_to_duration(file.musical_time.measures[b - 1].end_tick);
    let mut order: Vec<_> = t
        .notes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.start >= start && n.start < end)
        .map(|(i, _)| i)
        .collect();
    order.sort_by_key(|&i| (t.notes[i].start, t.notes[i].note, i));
    if order.is_empty() {
        return Err("这个选段没有音符".into());
    }
    if order.len() > 4096 {
        return Err("一次最多建议 4096 个音符，请缩小选段".into());
    }
    let mut groups: Vec<Vec<usize>> = vec![];
    for i in order {
        if groups
            .last()
            .is_none_or(|g| t.notes[g[0]].start != t.notes[i].start)
        {
            groups.push(vec![]);
        }
        groups.last_mut().unwrap().push(i);
    }
    let mut beams = vec![Beam {
        cost: 0.,
        left: f64::from(reference) - 12.,
        right: f64::from(reference) + 7.,
        held: vec![],
        chain: None,
    }];
    for group in groups {
        let onset = t.notes[group[0]].start;
        let mut next = vec![];
        for previous in &beams {
            for split in 0..=group.len() {
                if split > 5 || group.len() - split > 5 {
                    continue;
                }
                let mut assignments = vec![];
                let mut valid = true;
                let mut cost = previous.cost;
                let mut held: Vec<_> = previous
                    .held
                    .iter()
                    .copied()
                    .filter(|(_, end, _)| *end > onset)
                    .collect();
                let mut centers = [previous.left, previous.right];
                for (side, slice) in [
                    (&group[..split], PracticePart::LeftHand),
                    (&group[split..], PracticePart::RightHand),
                ]
                .into_iter()
                .enumerate()
                {
                    let (indices, hand) = slice;
                    if indices.is_empty() {
                        continue;
                    }
                    let pitches: Vec<_> = indices
                        .iter()
                        .map(|&i| f64::from(t.notes[i].note))
                        .collect();
                    let center = pitches.iter().sum::<f64>() / pitches.len() as f64;
                    cost += (center - centers[side]).abs() * 0.12;
                    centers[side] = center;
                    let mut all = pitches.clone();
                    all.extend(
                        held.iter()
                            .filter(|(_, _, h)| *h == hand)
                            .map(|(p, _, _)| f64::from(*p)),
                    );
                    let reach = all.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                        - all.iter().copied().fold(f64::INFINITY, f64::min);
                    cost += (reach - 12.).max(0.).powi(2) * 0.8;
                    cost += if hand == PracticePart::LeftHand {
                        (center - f64::from(reference)).max(0.) * 0.04
                    } else {
                        (f64::from(reference) - center).max(0.) * 0.04
                    };
                    for &i in indices {
                        let n = &t.notes[i];
                        if assigned
                            .get(&(track, i))
                            .is_some_and(|h| *h != PracticePart::Other && *h != hand)
                        {
                            valid = false;
                            break;
                        }
                        let anchor = assigned
                            .get(&(track, i))
                            .is_some_and(|p| *p != PracticePart::Other);
                        let confidence = if anchor {
                            100
                        } else if group.len() >= 2 && reach <= 12. {
                            80
                        } else {
                            55
                        };
                        let reason = if anchor {
                            "保留已确认的逐音分手"
                        } else if group.len() >= 2 {
                            "结合同时发声的手型跨度与前后声部走向"
                        } else {
                            "结合前后旋律连续性与参考手位；单声部需要确认"
                        };
                        assignments.push((i, hand, confidence, reason));
                        held.push((n.note, n.end, hand));
                    }
                }
                if valid {
                    next.push(Beam {
                        cost,
                        left: centers[0],
                        right: centers[1],
                        held,
                        chain: Some(Arc::new(Chain {
                            previous: previous.chain.clone(),
                            assignments,
                        })),
                    });
                }
            }
        }
        if next.is_empty() {
            for previous in &mut beams {
                previous.chain = Some(Arc::new(Chain {
                    previous: previous.chain.clone(),
                    assignments: group
                        .iter()
                        .map(|&i| {
                            (
                                i,
                                part(config, assigned, track, i),
                                35,
                                "交叉声部或复杂和弦，请逐音指定手",
                            )
                        })
                        .collect(),
                }));
                previous.held.retain(|(_, end, _)| *end > onset);
            }
        } else {
            next.sort_by(|a, b| a.cost.total_cmp(&b.cost));
            next.truncate(12);
            beams = next;
        }
    }
    let mut output = vec![];
    let mut chain = beams[0].chain.clone();
    while let Some(node) = chain {
        for &(i, hand, confidence, reason) in &node.assignments {
            let n = &t.notes[i];
            let tick = file
                .tempo_track
                .seconds_to_pulses(n.start.as_secs_f64())
                .round() as u64;
            output.push(Proposal {
                track,
                index: i,
                pitch: n.note,
                start: n.start.as_secs_f64(),
                measure: file
                    .musical_time
                    .measures
                    .partition_point(|m| m.start_tick <= tick)
                    .max(1),
                part: hand,
                saved: part(config, assigned, track, i),
                confidence,
                reason,
            });
        }
        chain = node.previous.clone();
    }
    output.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
    Ok(output)
}
pub fn hints(assigned: &HashMap<(usize, usize), PracticePart>) -> Vec<NoteHandHint> {
    let mut hints: Vec<_> = assigned
        .iter()
        .map(|(&(track_id, note_index), &part)| NoteHandHint {
            track_id,
            note_index,
            part,
        })
        .collect();
    hints.sort_by_key(|h| (h.track_id, h.note_index));
    hints
}
