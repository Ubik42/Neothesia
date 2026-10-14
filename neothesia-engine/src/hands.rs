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

#[derive(Clone, Copy, PartialEq, Eq)]
enum LayoutKind {
    Ordinary,
    Crossed,
    Interleaved,
}

struct HandLayout {
    hands: Vec<PracticePart>,
    kind: LayoutKind,
}

fn hand_layouts(anchors: &[Option<PracticePart>]) -> Vec<HandLayout> {
    let mut layouts = Vec::new();
    let mut add = |hands: Vec<PracticePart>, kind| {
        let left = hands
            .iter()
            .filter(|hand| **hand == PracticePart::LeftHand)
            .count();
        if left > 5 || hands.len() - left > 5 {
            return;
        }
        if anchors
            .iter()
            .zip(&hands)
            .any(|(anchor, hand)| anchor.is_some_and(|anchor| anchor != *hand))
        {
            return;
        }
        layouts.push(HandLayout { hands, kind });
    };
    for split in 0..=anchors.len() {
        add(
            (0..anchors.len())
                .map(|position| {
                    if position < split {
                        PracticePart::LeftHand
                    } else {
                        PracticePart::RightHand
                    }
                })
                .collect(),
            LayoutKind::Ordinary,
        );
        if split > 0 && split < anchors.len() {
            add(
                (0..anchors.len())
                    .map(|position| {
                        if position < split {
                            PracticePart::RightHand
                        } else {
                            PracticePart::LeftHand
                        }
                    })
                    .collect(),
                LayoutKind::Crossed,
            );
        }
    }
    let confirmed: Vec<_> = anchors.iter().flatten().copied().collect();
    let needs_interleaving = confirmed
        .windows(3)
        .any(|hands| hands[0] == hands[2] && hands[0] != hands[1]);
    if needs_interleaving {
        for first in 1..anchors.len().saturating_sub(1) {
            for second in first + 1..anchors.len() {
                for outside in [PracticePart::LeftHand, PracticePart::RightHand] {
                    let inside = if outside == PracticePart::LeftHand {
                        PracticePart::RightHand
                    } else {
                        PracticePart::LeftHand
                    };
                    add(
                        (0..anchors.len())
                            .map(|position| {
                                if position < first || position >= second {
                                    outside
                                } else {
                                    inside
                                }
                            })
                            .collect(),
                        LayoutKind::Interleaved,
                    );
                }
            }
        }
    }
    layouts
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
    let initial_held: Vec<_> = t
        .notes
        .iter()
        .enumerate()
        .filter(|(_, note)| note.start < start && note.end > start)
        .map(|(index, note)| (note.note, note.end, part(config, assigned, track, index)))
        .collect();
    let mut initial_centers = [f64::from(reference) - 12., f64::from(reference) + 7.];
    for (side, hand) in [PracticePart::LeftHand, PracticePart::RightHand]
        .into_iter()
        .enumerate()
    {
        let pitches: Vec<_> = initial_held
            .iter()
            .filter(|(_, _, held_hand)| *held_hand == hand)
            .map(|(pitch, _, _)| f64::from(*pitch))
            .collect();
        if !pitches.is_empty() {
            initial_centers[side] = pitches.iter().sum::<f64>() / pitches.len() as f64;
        }
    }
    let mut beams = vec![Beam {
        cost: 0.,
        left: initial_centers[0],
        right: initial_centers[1],
        held: initial_held,
        chain: None,
    }];
    for group in groups {
        let onset = t.notes[group[0]].start;
        let anchors: Vec<_> = group
            .iter()
            .map(|&index| {
                assigned
                    .get(&(track, index))
                    .copied()
                    .filter(|hand| *hand != PracticePart::Other)
            })
            .collect();
        let layouts = hand_layouts(&anchors);
        let mut next = vec![];
        for previous in &beams {
            for layout in &layouts {
                let mut assignments = vec![];
                let mut valid = true;
                let mut cost = previous.cost
                    + match layout.kind {
                        LayoutKind::Ordinary => 0.,
                        LayoutKind::Crossed => 8.,
                        LayoutKind::Interleaved => 14.,
                    };
                let mut held: Vec<_> = previous
                    .held
                    .iter()
                    .copied()
                    .filter(|(_, end, _)| *end > onset)
                    .collect();
                let mut centers = [previous.left, previous.right];
                for (side, hand) in [PracticePart::LeftHand, PracticePart::RightHand]
                    .into_iter()
                    .enumerate()
                {
                    let indices: Vec<_> = group
                        .iter()
                        .zip(&layout.hands)
                        .filter_map(|(&index, &suggested)| (suggested == hand).then_some(index))
                        .collect();
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
                    for &i in &indices {
                        let n = &t.notes[i];
                        if held
                            .iter()
                            .any(|(pitch, end, _)| *pitch == n.note && *end > onset)
                        {
                            valid = false;
                            break;
                        }
                        let anchor = assigned
                            .get(&(track, i))
                            .is_some_and(|p| *p != PracticePart::Other);
                        let confidence = if anchor {
                            100
                        } else if layout.kind == LayoutKind::Interleaved {
                            45
                        } else if layout.kind == LayoutKind::Crossed {
                            60
                        } else if group.len() >= 2 && reach <= 12. {
                            80
                        } else {
                            55
                        };
                        let reason = if anchor {
                            "保留已确认的逐音分手"
                        } else if layout.kind == LayoutKind::Interleaved {
                            "根据已确认手别补足交错声部；请逐音检查手型与动作"
                        } else if layout.kind == LayoutKind::Crossed {
                            "根据音域与持音尝试双手交叉；请检查手臂动作"
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
                let fallback: Vec<_> = group
                    .iter()
                    .map(|&index| {
                        (
                            index,
                            part(config, assigned, track, index),
                            35,
                            "同键重叠、交错声部或复杂和弦，请逐音指定手",
                        )
                    })
                    .collect();
                previous.chain = Some(Arc::new(Chain {
                    previous: previous.chain.clone(),
                    assignments: fallback.clone(),
                }));
                previous.held.retain(|(_, end, _)| *end > onset);
                let mut centers = [Vec::<f64>::new(), Vec::<f64>::new()];
                for (index, hand, _, _) in fallback {
                    let note = &t.notes[index];
                    previous.held.push((note.note, note.end, hand));
                    if hand == PracticePart::LeftHand {
                        centers[0].push(f64::from(note.note));
                    } else if hand == PracticePart::RightHand {
                        centers[1].push(f64::from(note.note));
                    }
                }
                if !centers[0].is_empty() {
                    previous.left = centers[0].iter().sum::<f64>() / centers[0].len() as f64;
                }
                if !centers[1].is_empty() {
                    previous.right = centers[1].iter().sum::<f64>() / centers[1].len() as f64;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Player};

    #[test]
    fn interleaved_layout_requires_confirmed_alternating_hands() {
        let left = PracticePart::LeftHand;
        let right = PracticePart::RightHand;
        let layouts = hand_layouts(&[Some(left), Some(right), None, Some(left)]);
        assert!(layouts.iter().any(|layout| {
            layout.kind == LayoutKind::Interleaved
                && layout.hands.as_slice() == &[left, right, right, left]
        }));
        assert!(hand_layouts(&[None, None, None, None])
            .iter()
            .all(|layout| layout.kind != LayoutKind::Interleaved));
        assert!(hand_layouts(&[Some(left), Some(right), Some(left), Some(right)]).is_empty());
    }

    #[test]
    fn confirmed_crossed_chord_still_recommends_the_unmarked_note() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work")
            .join(format!(
                "crossed-hands-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut player = Player::new(root, std::path::PathBuf::new(), true);
        let xml = r#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note><note><chord/><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration></note><note><chord/><pitch><step>C</step><octave>5</octave></pitch><duration>1</duration></note></measure></part></score-partwise>"#;
        player
            .command(Command::ImportScore {
                bytes: xml.as_bytes().to_vec(),
                name: "crossed.musicxml".into(),
                default_bpm: 100,
            })
            .unwrap();
        let file = player.file.as_ref().unwrap();
        let track = file
            .tracks
            .iter()
            .find(|track| track.notes.len() == 3)
            .unwrap();
        let mut anchors = HashMap::new();
        for (index, note) in track.notes.iter().enumerate() {
            if note.note == 60 {
                anchors.insert((track.track_id, index), PracticePart::RightHand);
            }
            if note.note == 72 {
                anchors.insert((track.track_id, index), PracticePart::LeftHand);
            }
        }
        let suggestions = propose(
            file,
            &SongConfig::new(&file.tracks),
            &anchors,
            track.track_id,
            (1, 1),
            66,
        )
        .unwrap();
        assert_eq!(suggestions.len(), 3);
        assert_eq!(
            suggestions
                .iter()
                .find(|item| item.pitch == 60)
                .unwrap()
                .part,
            PracticePart::RightHand
        );
        assert_eq!(
            suggestions
                .iter()
                .find(|item| item.pitch == 72)
                .unwrap()
                .part,
            PracticePart::LeftHand
        );
        let middle = suggestions.iter().find(|item| item.pitch == 64).unwrap();
        assert_ne!(middle.part, PracticePart::Other);
        assert_eq!(middle.confidence, 60);
    }
}
