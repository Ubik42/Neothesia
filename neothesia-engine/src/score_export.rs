//! Export saved personal note annotations into a copy of the original score.
use crate::notation::ScoreAsset;
use midi_file::MidiFile;
use neothesia_core::{
    musicxml::{ScoreEventId, ScoreEventKind, musicxml_source},
    practice::PracticePart,
};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    time::Duration,
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub fingers: bool,
    pub hands: bool,
    #[serde(default)]
    pub substitutions: bool,
    pub repeat_policy: RepeatPolicy,
}
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RepeatPolicy {
    Consistent,
    First,
}
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Mark {
    finger: Option<u8>,
    hand: Option<PracticePart>,
    changes: Option<Vec<Change>>,
}
#[derive(Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
struct Change {
    offset_tick: u64,
    from: u8,
    to: u8,
    hand: PracticePart,
}

pub(crate) fn export(
    asset: &ScoreAsset,
    file: &MidiFile,
    fingers: &HashMap<(usize, Duration, u8), u8>,
    hands: &HashMap<(usize, usize), PracticePart>,
    actions: &[(neothesia_core::library::FingerAction, bool)],
    options: &Options,
    download: bool,
    expected: Option<&str>,
) -> Result<serde_json::Value, String> {
    if !options.fingers && !options.hands && !options.substitutions {
        return Err("请至少选择指法、分手或持音换指标记".into());
    }
    if !asset.alignment.navigation_complete {
        return Err("谱面反复导航尚未通过，不能可靠导出个人标记".into());
    }
    let mut occurrences: BTreeMap<ScoreEventId, Vec<(Duration, usize, usize, Mark)>> =
        BTreeMap::new();
    let mut covered = HashSet::new();
    let mut saved: HashMap<(usize, usize), Vec<_>> = HashMap::new();
    if options.substitutions {
        for (a, valid) in actions {
            saved
                .entry((a.track_id, a.note_index))
                .or_default()
                .push((a, *valid));
        }
    }
    let invalid_actions = if options.substitutions {
        actions.iter().filter(|(_, valid)| !*valid).count()
    } else {
        0
    };
    for m in &asset.alignment.matches {
        if m.confidence_percent < 85 {
            continue;
        }
        let Some(note) = file
            .tracks
            .iter()
            .find(|t| t.track_id == m.midi_id.track_id)
            .and_then(|t| t.notes.get(m.midi_id.note_index))
        else {
            continue;
        };
        let source_actions = saved.get(&(m.midi_id.track_id, m.midi_id.note_index));
        let onset_tick = file
            .tempo_track
            .seconds_to_pulses(note.start.as_secs_f64())
            .round() as u64;
        let changes = if options.substitutions {
            if source_actions.is_some_and(|v| v.iter().any(|(_, valid)| !*valid)) {
                None
            } else {
                Some(
                    source_actions
                        .map(|v| {
                            v.iter()
                                .map(|(a, _)| Change {
                                    offset_tick: a.at_tick.saturating_sub(onset_tick),
                                    from: a.from,
                                    to: a.to,
                                    hand: a.hand,
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                )
            }
        } else {
            None
        };
        let mark = Mark {
            changes,
            finger: options
                .fingers
                .then(|| {
                    fingers
                        .get(&(m.midi_id.track_id, note.start, note.note))
                        .copied()
                })
                .flatten(),
            hand: options
                .hands
                .then(|| {
                    hands
                        .get(&(m.midi_id.track_id, m.midi_id.note_index))
                        .copied()
                })
                .flatten()
                .filter(|h| *h != PracticePart::Other),
        };
        occurrences
            .entry(m.score_id.source_id.clone())
            .or_default()
            .push((note.start, m.midi_id.track_id, m.midi_id.note_index, mark));
        covered.insert((m.midi_id.track_id, m.midi_id.note_index));
    }
    let mut unmatched = 0usize;
    for track in file.tracks.iter() {
        for (i, n) in track.notes.iter().enumerate() {
            if ((options.fingers && fingers.contains_key(&(track.track_id, n.start, n.note)))
                || (options.substitutions && saved.contains_key(&(track.track_id, i)))
                || (options.hands
                    && hands
                        .get(&(track.track_id, i))
                        .is_some_and(|h| *h != PracticePart::Other)))
                && !covered.contains(&(track.track_id, i))
            {
                unmatched += 1;
            }
        }
    }
    let mut edits = BTreeMap::new();
    let mut conflicts = Vec::new();
    let mut finger_count = 0;
    let mut hand_count = 0;
    let mut change_count = 0;
    let mut change_notes = 0;
    for (id, mut positions) in occurrences {
        positions.sort_by_key(|p| (p.0, p.1, p.2));
        positions.dedup_by_key(|p| (p.1, p.2));
        let first = positions[0].3.clone();
        let finger_conflict = positions.iter().any(|p| p.3.finger != first.finger);
        let hand_conflict = positions.iter().any(|p| p.3.hand != first.hand);
        let change_conflict = positions.iter().any(|p| p.3.changes != first.changes);
        if finger_conflict || hand_conflict || change_conflict {
            conflicts.push(serde_json::json!({"part":id.part_id,"measure":id.measure_ordinal+1,"note":id.ordinal+1,"fingers":finger_conflict,"hands":hand_conflict,"substitutions":change_conflict,"positions":positions.iter().map(|p|serde_json::json!({"track":p.1,"index":p.2,"seconds":p.0.as_secs_f64(),"finger":p.3.finger,"changes":p.3.changes,"hand":p.3.hand.map(|h|if h==PracticePart::LeftHand{"左手"}else{"右手"})})).collect::<Vec<_>>()}));
        }
        let mut chosen = Mark {
            changes: if change_conflict && options.repeat_policy == RepeatPolicy::Consistent {
                None
            } else {
                first.changes.clone()
            },
            finger: if finger_conflict && options.repeat_policy == RepeatPolicy::Consistent {
                None
            } else {
                first.finger
            },
            hand: if hand_conflict && options.repeat_policy == RepeatPolicy::Consistent {
                None
            } else {
                first.hand
            },
        };
        if chosen.changes.as_ref().is_some_and(|v| !v.is_empty()) {
            // A substitution chain always includes its matching attack finger.
            // Never attach a selected chain to an unrelated preserved score digit.
            chosen.finger = chosen
                .changes
                .as_ref()
                .and_then(|v| v.first())
                .map(|a| a.from);
            change_count += chosen.changes.as_ref().unwrap().len();
            change_notes += 1;
        } else {
            chosen.changes = None;
        }
        if chosen.finger.is_some() {
            finger_count += 1;
        }
        if chosen.hand.is_some() {
            hand_count += 1;
        }
        if chosen != Mark::default() {
            edits.insert(id, chosen);
        }
    }
    let source = musicxml_source(&asset.bytes).map_err(|e| e.to_string())?;
    if source.len() > 16_000_000 {
        return Err("展开后的原谱超过 16 MB，暂不支持改写导出".into());
    }
    let bytes = rewrite(&source, &edits, file.tempo_track.ppq())?;
    // Includes options as well as current data: a preview cannot silently change scope.
    let mut hash = blake3::Hasher::new();
    hash.update(&bytes);
    hash.update(
        &serde_json::to_vec(
            &edits
                .iter()
                .map(|(id, mark)| {
                    serde_json::json!([
                        id.part_id,
                        id.measure_ordinal,
                        id.ordinal,
                        mark.finger,
                        mark.hand,
                        mark.changes
                    ])
                })
                .collect::<Vec<_>>(),
        )
        .map_err(|e| e.to_string())?,
    );
    hash.update(&serde_json::to_vec(options).map_err(|e| e.to_string())?);
    hash.update(&serde_json::to_vec(&conflicts).map_err(|e| e.to_string())?);
    hash.update(&unmatched.to_le_bytes());
    if options.substitutions {
        hash.update(&serde_json::to_vec(actions).map_err(|e| e.to_string())?);
    }
    let fingerprint = hash.finalize().to_hex().to_string();
    if download && expected != Some(fingerprint.as_str()) {
        return Err("指法、换指、分手或谱面已经改变，请刷新预览后导出".into());
    }
    let base = std::path::Path::new(&asset.name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let base = base
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect::<String>();
    let mut value = serde_json::json!({"name":format!("{}-个人标记.musicxml",if base.is_empty(){"乐谱"}else{&base}),"fingerNotes":finger_count,"handNotes":hand_count,"substitutionNotes":change_notes,"substitutionActions":change_count,"invalidActions":invalid_actions,"ppq":file.tempo_track.ppq(),"writtenNotes":edits.len(),"unmappedNotes":unmatched,"conflicts":conflicts,"fingerprint":fingerprint});
    if download {
        value["bytes"] = serde_json::json!(bytes);
    }
    Ok(value)
}

// Preserve every original XML event outside edited note nodes, including layout,
// lyrics, articulations, comments, entity references and processing instructions.
enum Node {
    Element(BytesStart<'static>, Vec<Node>, BytesEnd<'static>),
    Raw(Event<'static>),
}
impl Node {
    fn named(&self, name: &[u8]) -> bool {
        match self {
            Self::Element(s, _, _) => s.name().as_ref() == name,
            _ => false,
        }
    }
    fn children(&mut self) -> &mut Vec<Node> {
        match self {
            Self::Element(_, c, _) => c,
            _ => unreachable!(),
        }
    }
    fn write(&self, w: &mut Writer<Vec<u8>>) -> Result<(), String> {
        match self {
            Self::Raw(e) => w.write_event(e.borrow()).map_err(|e| e.to_string()),
            Self::Element(s, c, e) => {
                w.write_event(Event::Start(s.borrow()))
                    .map_err(|e| e.to_string())?;
                for n in c {
                    n.write(w)?;
                }
                w.write_event(Event::End(e.borrow()))
                    .map_err(|e| e.to_string())
            }
        }
    }
}
fn element(name: &str, children: Vec<Node>) -> Node {
    Node::Element(
        BytesStart::new(name).into_owned(),
        children,
        BytesEnd::new(name).into_owned(),
    )
}
fn parse_node(
    reader: &mut Reader<&[u8]>,
    start: BytesStart<'static>,
    depth: usize,
) -> Result<Node, String> {
    if depth > 64 {
        return Err("乐谱节点嵌套过深".into());
    }
    let mut children = Vec::new();
    loop {
        match reader.read_event().map_err(|e| e.to_string())?.into_owned() {
            Event::Start(s) => children.push(parse_node(reader, s, depth + 1)?),
            Event::Empty(s) => {
                let end = s.to_end().into_owned();
                children.push(Node::Element(s, vec![], end));
            }
            Event::End(e) => return Ok(Node::Element(start, children, e)),
            Event::Eof => return Err("乐谱音符没有结束标签".into()),
            e => children.push(Node::Raw(e)),
        }
    }
}
fn technical_add(mark: &Mark, ppq: u16) -> Vec<Node> {
    let mut children = Vec::new();
    if let Some(f) = mark.finger {
        let mut start = BytesStart::new("fingering");
        start.push_attribute((
            "placement",
            if mark.hand.or_else(|| {
                mark.changes
                    .as_ref()
                    .and_then(|v| v.first())
                    .map(|a| a.hand)
            }) == Some(PracticePart::LeftHand)
            {
                "below"
            } else {
                "above"
            },
        ));
        children.push(Node::Element(
            start,
            vec![Node::Raw(Event::Text(
                BytesText::new(&f.to_string()).into_owned(),
            ))],
            BytesEnd::new("fingering"),
        ));
    }
    if let Some(changes) = &mark.changes {
        for a in changes {
            children.push(Node::Raw(Event::Comment(
                BytesText::new("neothesia:substitution").into_owned(),
            )));
            let mut start = BytesStart::new("fingering");
            start.push_attribute(("substitution", "yes"));
            start.push_attribute((
                "placement",
                if a.hand == PracticePart::LeftHand {
                    "below"
                } else {
                    "above"
                },
            ));
            children.push(Node::Element(
                start,
                vec![Node::Raw(Event::Text(
                    BytesText::new(&a.to.to_string()).into_owned(),
                ))],
                BytesEnd::new("fingering"),
            ));
            children.push(Node::Raw(Event::Comment(
                BytesText::new("neothesia:substitution-time").into_owned(),
            )));
            let text = format!(
                "持音换指 {}→{}：起音后 {:.3} 四分音符拍",
                a.from,
                a.to,
                a.offset_tick as f64 / ppq as f64
            );
            children.push(element(
                "other-technical",
                vec![Node::Raw(Event::Text(BytesText::new(&text).into_owned()))],
            ));
        }
    }
    if let Some(h) = mark.hand {
        children.push(Node::Raw(Event::Comment(
            BytesText::new("neothesia:hand").into_owned(),
        )));
        let mut start = BytesStart::new("other-technical");
        start.push_attribute((
            "placement",
            if h == PracticePart::LeftHand {
                "below"
            } else {
                "above"
            },
        ));
        children.push(Node::Element(
            start,
            vec![Node::Raw(Event::Text(
                BytesText::new(if h == PracticePart::LeftHand {
                    "左手"
                } else {
                    "右手"
                })
                .into_owned(),
            ))],
            BytesEnd::new("other-technical"),
        ));
    }
    children
}
fn edit_note(note: &mut Node, mut mark: Mark, ppq: u16) {
    // Imported score digits may also be stored as practice hints. If the digit
    // already agrees, retain its original typography/substitution metadata.
    let originals = note
        .children()
        .iter_mut()
        .filter(|n| n.named(b"notations"))
        .flat_map(|n| n.children())
        .filter(|n| n.named(b"technical"))
        .flat_map(|n| n.children())
        .filter(|n| n.named(b"fingering"))
        .map(|n| {
            n.children()
                .iter()
                .filter_map(|c| match c {
                    Node::Raw(Event::Text(t)) => t.decode().ok().map(|v| v.into_owned()),
                    _ => None,
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    if mark.changes.is_none()
        && mark
            .finger
            .is_some_and(|f| originals.len() == 1 && originals[0].trim() == f.to_string())
    {
        mark.finger = None;
    }
    if mark == Mark::default() {
        return;
    }

    let children = note.children();
    for notation in children.iter_mut().filter(|n| n.named(b"notations")) {
        for technical in notation
            .children()
            .iter_mut()
            .filter(|n| n.named(b"technical"))
        {
            let mut skip_hand = false;
            technical.children().retain(|n| {
                if (mark.finger.is_some() || mark.changes.is_some()) && n.named(b"fingering") {
                    return false;
                }
                if mark.changes.is_some() || mark.finger.is_some() {
                    if matches!(n,Node::Raw(Event::Comment(c)) if &**c==b"neothesia:substitution"||&**c==b"neothesia:substitution-time") {skip_hand=true;return false;}
                    if skip_hand && n.named(b"other-technical") {skip_hand=false;return false;}
                    if !matches!(n,Node::Raw(Event::Text(_))) {skip_hand=false;}
                }
                if mark.hand.is_some() {
                    if matches!(n,Node::Raw(Event::Comment(c)) if &**c==b"neothesia:hand") {
                        skip_hand = true;
                        return false;
                    }
                    if skip_hand && n.named(b"other-technical") {
                        skip_hand = false;
                        return false;
                    }
                    if !matches!(n, Node::Raw(Event::Text(_))) {
                        skip_hand = false;
                    }
                }
                true
            });
        }
    }
    if let Some(notation) = children.iter_mut().find(|n| n.named(b"notations")) {
        let c = notation.children();
        if let Some(technical) = c.iter_mut().find(|n| n.named(b"technical")) {
            technical.children().extend(technical_add(&mark, ppq));
        } else {
            c.push(element("technical", technical_add(&mark, ppq)));
        }
    } else {
        // Notations precede lyric/play/listen in the MusicXML note content model.
        let index = children
            .iter()
            .position(|n| n.named(b"lyric") || n.named(b"play") || n.named(b"listen"))
            .unwrap_or(children.len());
        children.insert(
            index,
            element(
                "notations",
                vec![element("technical", technical_add(&mark, ppq))],
            ),
        );
    }
}
fn rewrite(
    source: &[u8],
    edits: &BTreeMap<ScoreEventId, Mark>,
    ppq: u16,
) -> Result<Vec<u8>, String> {
    let mut reader = Reader::from_reader(source);
    let mut writer = Writer::new(Vec::new());
    let mut part = String::new();
    let mut measure = 0u32;
    let mut ordinal = 0u32;
    let mut applied = 0;
    loop {
        let e = reader.read_event().map_err(|e| e.to_string())?.into_owned();
        match e {
            Event::Start(ref s) if s.name().as_ref() == b"part" => {
                part = s
                    .attributes()
                    .filter_map(Result::ok)
                    .find(|a| a.key.as_ref() == b"id")
                    .map(|a| {
                        a.decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map(|s| s.to_string())
                    })
                    .transpose()
                    .map_err(|e| e.to_string())?
                    .unwrap_or_default();
                measure = 0;
            }
            Event::Start(ref s) if s.name().as_ref() == b"measure" => {
                ordinal = 0;
            }
            Event::End(ref s) if s.name().as_ref() == b"measure" => {
                measure += 1;
            }
            Event::Start(s) if s.name().as_ref() == b"note" => {
                let id = ScoreEventId {
                    part_id: part.clone(),
                    measure_ordinal: measure,
                    kind: ScoreEventKind::Note,
                    ordinal,
                };
                ordinal += 1;
                let mut node = parse_node(&mut reader, s, 0)?;
                if let Some(mark) = edits.get(&id) {
                    edit_note(&mut node, mark.clone(), ppq);
                    applied += 1;
                }
                node.write(&mut writer)?;
                continue;
            }
            Event::Eof => break,
            _ => {}
        }
        writer.write_event(e).map_err(|e| e.to_string())?;
    }
    if applied != edits.len() {
        return Err("原谱音符身份与练习谱面不一致，导出已取消".into());
    }
    let bytes = writer.into_inner();
    neothesia_core::musicxml::import_musicxml(&bytes)
        .map_err(|e| format!("导出谱校验失败：{e}"))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserve_original_notation_and_replace_only_personal_marks() {
        let source=br#"<?xml version="1.0"?><score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions></attributes><note default-x="42"><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><notations><slur type="start" number="1"/><technical><fingering substitution="yes">3</fingering><hammer-on type="start">H</hammer-on></technical></notations><lyric><text>keep &amp; sing</text></lyric></note><note><rest/><duration>1</duration></note></measure></part></score-partwise>"#;
        let id = ScoreEventId {
            part_id: "P1".into(),
            measure_ordinal: 0,
            kind: ScoreEventKind::Note,
            ordinal: 0,
        };
        let edits = BTreeMap::from([(
            id,
            Mark {
                finger: Some(2),
                hand: Some(PracticePart::LeftHand),
                changes: None,
            },
        )]);
        let output = rewrite(source, &edits, 480).unwrap();
        let text = String::from_utf8(output.clone()).unwrap();
        assert!(text.contains("default-x=\"42\""));
        assert!(text.contains("<slur type=\"start\" number=\"1\"></slur>"));
        assert!(text.contains("<hammer-on type=\"start\">H</hammer-on>"));
        assert!(text.contains("keep &amp; sing"));
        assert!(text.contains("<fingering placement=\"below\">2</fingering>"));
        assert!(!text.contains("substitution="));
        assert_eq!(text.matches("<other-technical").count(), 1);
        let second = String::from_utf8(rewrite(&output, &edits, 480).unwrap()).unwrap();
        assert_eq!(second.matches("<other-technical").count(), 1);
        let original_id = ScoreEventId {
            part_id: "P1".into(),
            measure_ordinal: 0,
            kind: ScoreEventKind::Note,
            ordinal: 0,
        };
        let same = BTreeMap::from([(
            original_id,
            Mark {
                finger: Some(3),
                hand: None,
                changes: None,
            },
        )]);
        assert!(
            String::from_utf8(rewrite(source, &same, 480).unwrap())
                .unwrap()
                .contains("substitution=\"yes\">3")
        );
        let no_edits = String::from_utf8(rewrite(source, &BTreeMap::new(), 480).unwrap()).unwrap();
        assert!(no_edits.contains("substitution=\"yes\">3"));
    }
    #[test]
    fn substitution_export_keeps_attack_and_timing_without_duplicate_text() {
        let source=br#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><notations><technical><fingering>5</fingering><other-technical>keep</other-technical></technical></notations></note></measure></part></score-partwise>"#;
        let id = ScoreEventId {
            part_id: "P1".into(),
            measure_ordinal: 0,
            kind: ScoreEventKind::Note,
            ordinal: 0,
        };
        let edits = BTreeMap::from([(
            id,
            Mark {
                finger: Some(3),
                hand: Some(PracticePart::LeftHand),
                changes: Some(vec![
                    Change {
                        offset_tick: 840,
                        from: 3,
                        to: 2,
                        hand: PracticePart::LeftHand,
                    },
                    Change {
                        offset_tick: 1200,
                        from: 2,
                        to: 1,
                        hand: PracticePart::LeftHand,
                    },
                ]),
            },
        )]);
        let first = rewrite(source, &edits, 480).unwrap();
        let twice = rewrite(&first, &edits, 480).unwrap();
        let text = String::from_utf8(twice.clone()).unwrap();
        assert_eq!(text.matches("substitution=\"yes\"").count(), 2);
        assert_eq!(text.matches("neothesia:substitution-time").count(), 2);
        assert_eq!(text.matches("neothesia:hand").count(), 1);
        assert!(text.contains("1.750"));
        assert!(text.contains("2.500"));
        assert!(text.contains(">keep<"));
        let score = neothesia_core::musicxml::import_musicxml(&twice).unwrap();
        let neothesia_core::musicxml::ScoreEvent::Note(note) =
            &score.parts[0].measures[0].events[0]
        else {
            panic!("note")
        };
        assert_eq!(note.fingering.as_deref(), Some("3"));
        let changed: BTreeMap<_, _> = edits
            .into_iter()
            .map(|(id, _)| {
                (
                    id,
                    Mark {
                        finger: Some(4),
                        hand: None,
                        changes: None,
                    },
                )
            })
            .collect();
        let replaced = String::from_utf8(rewrite(&twice, &changed, 480).unwrap()).unwrap();
        assert!(!replaced.contains("neothesia:substitution"));
        assert!(replaced.contains("neothesia:hand"));
        assert!(replaced.contains(">keep<"));
    }
}
