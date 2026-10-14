use midi_file::MidiFile;
use neothesia_core::{
    musicxml::{Score, import_musicxml_document},
    score_alignment::{ScoreMidiAlignment, align_score_to_midi, summarize_alignment},
    score_view::{
        RendererScoreNoteEvidence, ScoreNoteSemanticKey, correlate_score_notes,
        native_score_note_evidence,
    },
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct FingeringMark {
    pub slur_from: Option<(usize, usize)>,
    pub staccato: bool,
    pub staccato_from: Option<(usize, usize)>,
}
pub struct ScoreAsset {
    pub bytes: Vec<u8>,
    pub name: String,
    pub score: Score,
    pub alignment: ScoreMidiAlignment,
    pub content_id: String,
    pub source_fingerprint: String,
}
impl ScoreAsset {
    pub fn new(bytes: Vec<u8>, name: String, file: &MidiFile) -> Result<Self, String> {
        if bytes.len() > 4_000_000 {
            return Err("乐谱超过 4 MB".into());
        }
        let score = import_musicxml_document(&bytes).map_err(|e| e.to_string())?;
        let score = neothesia_core::score_performance::normalized(&score)?;
        let alignment = align_score_to_midi(&score, file);
        Ok(Self {
            content_id: file.content_id.clone(),
            source_fingerprint: blake3::hash(&bytes).to_hex().to_string(),
            bytes,
            name,
            score,
            alignment,
        })
    }
    /// Only exact, individually trusted occurrence matches may influence a MIDI fingering.
    /// Unmapped notes break a slur chain, so repeats and overlapping voices cannot invent links.
    pub fn fingering_marks(&self, file: &MidiFile) -> HashMap<(usize, usize), FingeringMark> {
        use neothesia_core::{
            musicxml::{ScoreEvent, SpanType},
            score_alignment::{AlignmentReadiness, summarize_alignment},
            score_playback::{PlaybackLimits, build_playback_plan, expand_event_occurrences},
        };
        if self.content_id != file.content_id
            || summarize_alignment(&self.alignment).readiness != AlignmentReadiness::Ready
        {
            return HashMap::new();
        }
        let mut matches = HashMap::new();
        let mut seen_scores = HashSet::new();
        let mut duplicate_scores = HashSet::new();
        let mut seen_midi = HashSet::new();
        let mut duplicate_midi = HashSet::new();
        for matched in &self.alignment.matches {
            if !matched.principal { continue; }
            let midi = (matched.midi_id.track_id, matched.midi_id.note_index);
            if !seen_scores.insert(matched.score_id.clone()) {
                duplicate_scores.insert(matched.score_id.clone());
            }
            if !seen_midi.insert(midi) { duplicate_midi.insert(midi); }
            if matched.confidence_percent < 85 || !matched.exact_score_projection { continue; }
            matches.insert(matched.score_id.clone(), midi);
        }
        for id in duplicate_scores { matches.remove(&id); }
        // A MIDI note that appears under more than one score occurrence is ambiguous.
        let mut counts = HashMap::new();
        for midi in matches.values() { *counts.entry(*midi).or_insert(0usize) += 1; }
        matches.retain(|_, midi| counts[midi] == 1 && !duplicate_midi.contains(midi));
        let mut marks = HashMap::new();
        for part in &self.score.parts {
            let plan = build_playback_plan(part, PlaybackLimits::default());
            if !plan.complete { continue; }
            let mut active: HashMap<(u8, String, u8), (usize, usize)> = HashMap::new();
            let mut previous_voice: HashMap<(u8, String), (usize, usize)> = HashMap::new();
            let mut occurrences = expand_event_occurrences(part, &plan);
            occurrences.sort_by(|a, b| a.onset.cmp(&b.onset).then_with(|| a.id.cmp(&b.id)));
            for occurrence in occurrences {
                let id = &occurrence.id.source_id;
                let Some(ScoreEvent::Note(note)) = part.measures
                    .get(id.measure_ordinal as usize)
                    .and_then(|measure| measure.events.iter().find(|event| match event {
                        ScoreEvent::Note(note) => note.id == *id,
                        ScoreEvent::Direction(_) => false,
                    })) else { continue; };
                let voice = note.voice.clone().unwrap_or_default();
                if note.pitch.is_none() || note.grace {
                    active.retain(|(staff, v, _), _| *staff != note.staff || *v != voice);
                    previous_voice.remove(&(note.staff, voice));
                    continue;
                }
                let midi = matches.get(&occurrence.id).copied();
                let keys: Vec<_> = active.keys().filter(|(staff, v, _)| *staff == note.staff && *v == voice).cloned().collect();
                // An untrusted score occurrence interrupts this voice's active chains.
                if midi.is_none() {
                    for key in keys { active.remove(&key); }
                    previous_voice.remove(&(note.staff, voice));
                    continue;
                }
                let midi = midi.unwrap();
                let prior_voice = previous_voice.insert((note.staff, voice.clone()), midi);
                let staccato_from = prior_voice.filter(|prior| marks.get(prior)
                    .is_some_and(|mark: &FingeringMark| mark.staccato));
                let mark = marks.entry(midi).or_insert(FingeringMark::default());
                mark.staccato = note.articulations.iter().any(|a| a == "staccato");
                mark.staccato_from = staccato_from;
                for key in keys {
                    if let Some(prior) = active.get(&key).copied() {
                        let prior_onset = file.tracks.iter().find(|t| t.track_id == prior.0)
                            .and_then(|t| t.notes.get(prior.1)).map(|n| n.start);
                        let current_onset = file.tracks.iter().find(|t| t.track_id == midi.0)
                            .and_then(|t| t.notes.get(midi.1)).map(|n| n.start);
                        if prior != midi && prior_onset < current_onset && mark.slur_from.is_none() {
                            mark.slur_from = Some(prior);
                        }
                        active.insert(key, midi);
                    }
                }
                for span in &note.slurs {
                    let key = (note.staff, voice.clone(), span.number);
                    match span.kind {
                        SpanType::Start => { active.insert(key, midi); }
                        SpanType::Stop => { active.remove(&key); }
                        SpanType::Continue => { active.insert(key, midi); }
                    }
                }
            }
        }
        marks
    }
    pub fn payload(&self, file: Option<&MidiFile>) -> serde_json::Value {
        let a = summarize_alignment(&self.alignment);
        let trill_count = self.score.parts.iter().flat_map(|part| &part.measures)
            .flat_map(|measure| &measure.events)
            .filter(|event| matches!(event, neothesia_core::musicxml::ScoreEvent::Note(note) if note.trill.is_some()))
            .count();
        let mordent_count = self.score.parts.iter().flat_map(|part| &part.measures)
            .flat_map(|measure| &measure.events)
            .filter(|event| matches!(event, neothesia_core::musicxml::ScoreEvent::Note(note) if note.mordent.is_some()))
            .count();
        let turn_count = self.score.parts.iter().flat_map(|part| &part.measures)
            .flat_map(|measure| &measure.events)
            .filter(|event| matches!(event, neothesia_core::musicxml::ScoreEvent::Note(note) if note.turn.as_ref().is_some_and(|mark| !mark.inverted)))
            .count();
        let inverted_turn_count = self.score.parts.iter().flat_map(|part| &part.measures)
            .flat_map(|measure| &measure.events)
            .filter(|event| matches!(event, neothesia_core::musicxml::ScoreEvent::Note(note) if note.turn.as_ref().is_some_and(|mark| mark.inverted)))
            .count();
        let route = file.and_then(|file| self.score.parts.first().map(|part| {
            use neothesia_core::score_playback::{build_playback_plan, PlaybackLimits};
            let plan = build_playback_plan(part, PlaybackLimits { max_visits: 20_000, max_repeat_passes: 16 });
            let seconds = |t: neothesia_core::musicxml::ScoreTime| file.tempo_track.pulses_to_duration(
                (t.numerator.max(0) as u128 * u128::from(file.musical_time.ppq) / u128::from(t.denominator)) as u64).as_secs_f64();
            let mut beat_types = vec![];
            let mut beat_type = 4f64;
            for m in &part.measures {
                for a in &m.attributes {
                    if let Some(time) = &a.time_signature {
                        if let Ok(value) = time.beat_type.parse::<f64>() { beat_type = value; }
                    }
                }
                beat_types.push(beat_type);
            }
            let visits: Vec<_> = plan.visits.iter().map(|v| {
                let index = v.source_measure_ordinal as usize;
                let m = &part.measures[index];
                let beat = |t: neothesia_core::musicxml::ScoreTime| t.numerator as f64 / f64::from(t.denominator) * beat_types[index] / 4. + 1.;
                serde_json::json!({"ordinal":v.occurrence_ordinal,"measure":index,"number":m.number,
                    "pass":v.repeat_pass,"start":seconds(v.performed_start),
                    "end":seconds(v.performed_end()),
                    "startBeat":beat(v.source_start),"endBeat":beat(v.source_end),
                    "partialStart":v.source_start > neothesia_core::musicxml::ScoreTime::default(),
                    "partialEnd":v.source_end < m.duration})
            }).collect();
            serde_json::json!({"complete":plan.complete && a.readiness == neothesia_core::score_alignment::AlignmentReadiness::Ready,"visits":visits})
        }));
        serde_json::json!({"route":route,"bytes":self.bytes,"name":self.name,"sourceFingerprint":self.source_fingerprint,"compressed":self.bytes.starts_with(b"PK"),"title":self.score.title,"coverage":a.coverage_percent,"confidence":a.mean_confidence_percent,"readiness":a.readiness,"diagnostics":a.navigation_diagnostics,"warnings":self.score.warnings.iter().map(score_warning_label).collect::<Vec<_>>(),"trillCount":trill_count,"mordentCount":mordent_count,"turnCount":turn_count,"invertedTurnCount":inverted_turn_count})
    }
    pub fn map(
        &self,
        file: &MidiFile,
        midi: &[u8],
        notes: Vec<RenderNote>,
    ) -> Result<serde_json::Value, String> {
        if midi.len() > 4_000_000 || notes.len() > 100_000 {
            return Err("乐谱映射数据过大".into());
        }
        let smf = midi_file::midly::Smf::parse(midi).map_err(|e| e.to_string())?;
        let renderer = MidiFile::from_smf("renderer", &smf)?;
        let evidence = notes
            .into_iter()
            .filter(|n| {
                n.onset_millis.is_finite()
                    && n.duration_millis.is_finite()
                    && n.onset_millis >= 0.
                    && n.duration_millis >= 0.
            })
            .map(|n| {
                let start = renderer
                    .tempo_track
                    .seconds_to_pulses(n.onset_millis / 1000.)
                    .round() as u64;
                let end = renderer
                    .tempo_track
                    .seconds_to_pulses((n.onset_millis + n.duration_millis) / 1000.)
                    .round() as u64;
                let scale = f64::from(file.musical_time.ppq) / f64::from(renderer.musical_time.ppq);
                let timestamp = file
                    .tempo_track
                    .pulses_to_duration((start as f64 * scale).round() as u64);
                let duration = file
                    .tempo_track
                    .pulses_to_duration((end as f64 * scale).round() as u64)
                    .saturating_sub(timestamp);
                RendererScoreNoteEvidence {
                    renderer_id: n.renderer_id,
                    page_index: n.page_index,
                    key: ScoreNoteSemanticKey {
                        onset_millis: timestamp.as_millis() as i64,
                        pitch: n.pitch,
                        duration_millis: duration.as_millis() as i64,
                    },
                }
            });
        let correlation = correlate_score_notes(
            native_score_note_evidence(&self.score, &file.tempo_track),
            evidence,
        );
        let mut render_by_source = std::collections::HashMap::new();
        for e in &correlation.elements {
            render_by_source.entry(e.source_id.clone()).or_insert(e);
        }
        let score_fingers: std::collections::HashMap<_, _> = self
            .score
            .parts
            .iter()
            .flat_map(|p| &p.measures)
            .flat_map(|m| &m.events)
            .filter_map(|event| match event {
                neothesia_core::musicxml::ScoreEvent::Note(note) => {
                    Some((note.id.clone(), note.fingering.as_deref()))
                }
                _ => None,
            })
            .collect();
        let mut mapping = Vec::new();
        if self.alignment.navigation_complete {
            for m in &self.alignment.matches {
                if m.confidence_percent < 85 {
                    continue;
                }
                if let Some(e) = render_by_source.get(&m.score_id.source_id) {
                    if let Some(n) = file
                        .tracks
                        .iter()
                        .find(|t| t.track_id == m.midi_id.track_id)
                        .and_then(|t| t.notes.get(m.midi_id.note_index))
                    {
                        let score_finger = if m.principal {
                            score_fingers.get(&m.score_id.source_id).copied().flatten()
                        } else { None };
                        mapping.push(serde_json::json!({"id":e.renderer_id,"page":e.page_index,"start":n.start.as_secs_f64(),"end":n.end.as_secs_f64(),"pitch":n.note,"track":m.midi_id.track_id,"index":m.midi_id.note_index,"scoreFinger":score_finger}));
                    }
                }
            }
        }
        Ok(
            serde_json::json!({"mapping":mapping,"ambiguities":correlation.ambiguities.len(),"unmapped":correlation.unmatched_native.len(),"following":self.alignment.navigation_complete && !correlation.elements.is_empty()}),
        )
    }
}

fn score_warning_label(warning: &neothesia_core::musicxml::ImportWarning) -> String {
    let location = if let Some((part, measure)) = warning.location.split_once(" measure ") {
        format!("{part} 第 {measure} 小节")
    } else if warning.location == "score" {
        "整份谱面".into()
    } else if warning.location == "MXL container" {
        "MXL 文件".into()
    } else {
        warning.location.clone()
    };
    let message = match warning.message.as_str() {
        "No playable parts were found" => "没有找到可演奏的声部",
        "incomplete or zero tuplet ratio was ignored" => "连音比例缺失或为零，已略过这一记号",
        "ornament notation and playback are not represented yet" => "装饰音尚未进入谱面和演奏解释",
        "written-to-sounding transposition is not represented yet" => "记谱音与实际发声音高的移调尚未解释",
        "crescendo and diminuendo wedge spans are not represented yet" => "渐强和渐弱线段尚未解释",
        "notation element is not represented yet" => "这个谱面记号尚未解释",
        text if text.starts_with("missing or invalid ") && text.ends_with(" mimetype marker") => "MXL 文件类型标记缺失或无效",
        text => text,
    };
    format!("{location}：{message}")
}
#[derive(Deserialize)]
pub struct RenderNote {
    pub renderer_id: String,
    pub page_index: usize,
    pub onset_millis: f64,
    pub pitch: u8,
    pub duration_millis: f64,
}

#[cfg(test)]
mod fingering_expression_tests {
    use super::*;
    #[test]
    fn trill_uses_only_its_written_pitch_attack_for_score_fingering_links() {
        let xml = br#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions><time><beats>2</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><voice>1</voice><notations><slur type="start" number="1"/><ornaments><trill-mark beats="3" start-note="upper" trill-step="whole"/></ornaments></notations></note><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration><voice>1</voice><notations><slur type="stop" number="1"/></notations></note></measure></part></score-partwise>"#;
        let score = neothesia_core::musicxml::import_musicxml_document(xml).unwrap();
        let generated = neothesia_core::score_performance::generate(&score, 120).unwrap();
        let file = MidiFile::from_smf("trill", &generated.smf).unwrap();
        let asset = ScoreAsset::new(xml.to_vec(), "trill.xml".into(), &file).unwrap();
        let notes: Vec<_> = file.tracks.iter().flat_map(|track| track.notes.iter().enumerate()
            .map(move |(index, note)| ((track.track_id, index), note.note))).collect();
        assert_eq!(notes.iter().map(|(_, pitch)| *pitch).collect::<Vec<_>>(), vec![62, 60, 62, 64]);
        assert_eq!(asset.alignment.matches.len(), 4);
        assert!(asset.alignment.unmatched_score.is_empty());
        let principal = asset.alignment.matches.iter().find(|matched| matched.score_id.source_id.ordinal == 0 && matched.principal).unwrap();
        assert_eq!(principal.strike_ordinal, 1);
        let marks = asset.fingering_marks(&file);
        assert_eq!(marks.get(&notes[3].0).unwrap().slur_from, Some(notes[1].0));
        assert!(!marks.contains_key(&notes[0].0));
        assert!(!marks.contains_key(&notes[2].0));
    }
    #[test]
    fn score_marks_use_exact_occurrences_and_reject_stale_pairing() {
        let xml = br#"<?xml version="1.0"?><score-partwise version="4.0">
          <part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
          <part id="P1"><measure number="1"><attributes><divisions>1</divisions></attributes>
          <note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
          <notations><slur type="start" number="1"/></notations></note>
          <note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
          <notations><slur type="stop" number="1"/></notations></note>
          <note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
          <notations><articulations><staccato/></articulations></notations></note>
          <note><pitch><step>F</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice></note>
          </measure></part></score-partwise>"#;
        let score = neothesia_core::musicxml::import_musicxml_document(xml).unwrap();
        let generated = neothesia_core::score_performance::generate(&score, 120).unwrap();
        let file = MidiFile::from_smf("expression", &generated.smf).unwrap();
        let mut asset = ScoreAsset::new(xml.to_vec(), "expression.xml".into(), &file).unwrap();
        let marks = asset.fingering_marks(&file);
        let id = |pitch| file.tracks.iter().flat_map(|t| t.notes.iter().enumerate()
            .filter_map(move |(index, note)| (note.note == pitch).then_some((t.track_id, index))))
            .next().unwrap();
        assert_eq!(marks.get(&id(62)).unwrap().slur_from, Some(id(60)));
        assert!(marks.get(&id(64)).unwrap().staccato);
        assert_eq!(marks.get(&id(65)).unwrap().staccato_from, Some(id(64)));
        let reopened = ScoreAsset::new(xml.to_vec(), "expression.xml".into(), &file).unwrap();
        assert_eq!(reopened.source_fingerprint, asset.source_fingerprint);
        assert_eq!(reopened.fingering_marks(&file).get(&id(62)).unwrap().slur_from,
            Some(id(60)));
        let assigned: HashMap<_, _> = file.tracks.iter().flat_map(|t| (0..t.notes.len())
            .map(move |index| ((t.track_id, index), neothesia_core::practice::PracticePart::RightHand)))
            .collect();
        let selected = id(60);
        let request = crate::fingerings::PlanRequest {
            content_id:file.content_id.clone(), track:selected.0, index:selected.1,
            profile:neothesia_core::fingering::HandSpanProfile::Standard,
            hand:neothesia_core::practice::PracticePart::RightHand,
            range:Some((1, 1)), keep_saved:false, practice_rate:1.0,
            pins:vec![], target_tracks:vec![],
        };
        let config = neothesia_core::song_config::SongConfig::new(&file.tracks);
        let prepare = |score: &ScoreAsset| crate::fingerings::prepare(&file, Some(score), &config,
            &HashMap::new(), &assigned, request.clone()).unwrap();
        let first = prepare(&asset);
        let restored = prepare(&reopened);
        assert_eq!(first.fingerprint, restored.fingerprint);
        assert!(first.expression.iter().any(|hint| hint.slur_from.is_some()));
        let changed_xml = String::from_utf8(xml.to_vec()).unwrap().replace("<staccato/>", "<tenuto/>");
        let changed = ScoreAsset::new(changed_xml.into_bytes(), "changed.xml".into(), &file).unwrap();
        assert_ne!(first.fingerprint, prepare(&changed).fingerprint);
        let mut ambiguous = ScoreAsset::new(xml.to_vec(), "ambiguous.xml".into(), &file).unwrap();
        let repeated_match = ambiguous.alignment.matches.iter()
            .find(|matched| (matched.midi_id.track_id, matched.midi_id.note_index) == id(62))
            .unwrap().clone();
        ambiguous.alignment.matches.push(repeated_match);
        assert!(!ambiguous.fingering_marks(&file).contains_key(&id(62)));
        asset.alignment.matches.iter_mut()
            .find(|matched| (matched.midi_id.track_id, matched.midi_id.note_index) == id(62))
            .unwrap().confidence_percent = 84;
        assert!(!asset.fingering_marks(&file).contains_key(&id(62)));
        asset.content_id = "different-song".into();
        assert!(asset.fingering_marks(&file).is_empty());
    }
    #[test]
    fn repeated_slur_links_each_performed_visit_to_its_own_notes() {
        let xml = br#"<?xml version="1.0"?><score-partwise version="4.0">
          <part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
          <part id="P1"><measure number="1"><attributes><divisions>1</divisions></attributes>
          <note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
          <notations><slur type="start" number="1"/></notations></note>
          <note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
          <notations><slur type="stop" number="1"/></notations></note>
          <barline location="right"><repeat direction="backward" times="2"/></barline>
          </measure></part></score-partwise>"#;
        let score = neothesia_core::musicxml::import_musicxml_document(xml).unwrap();
        let performance = neothesia_core::score_performance::generate(&score, 120).unwrap();
        let file = MidiFile::from_smf("repeat", &performance.smf).unwrap();
        let asset = ScoreAsset::new(xml.to_vec(), "repeat.xml".into(), &file).unwrap();
        let marks = asset.fingering_marks(&file);
        let mut c = Vec::new(); let mut d = Vec::new();
        for track in file.tracks.iter() { for (i, note) in track.notes.iter().enumerate() {
            if note.note == 60 { c.push((note.start, (track.track_id, i))); }
            if note.note == 62 { d.push((note.start, (track.track_id, i))); }
        }}
        c.sort(); d.sort();
        assert_eq!(c.len(), 2); assert_eq!(d.len(), 2);
        assert_eq!(marks.get(&d[0].1).unwrap().slur_from, Some(c[0].1));
        assert_eq!(marks.get(&d[1].1).unwrap().slur_from, Some(c[1].1));
    }
    #[test]
    fn rest_inside_a_slur_does_not_create_a_false_legato_transition() {
        let xml = br#"<score-partwise version="4.0">
          <part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
          <part id="P1"><measure number="1"><attributes><divisions>1</divisions></attributes>
          <note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
          <notations><slur type="start" number="1"/></notations></note>
          <note><rest/><duration>1</duration><voice>1</voice></note>
          <note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
          <notations><slur type="stop" number="1"/></notations></note>
          </measure></part></score-partwise>"#;
        let score = neothesia_core::musicxml::import_musicxml_document(xml).unwrap();
        let performance = neothesia_core::score_performance::generate(&score, 120).unwrap();
        let file = MidiFile::from_smf("rest", &performance.smf).unwrap();
        let asset = ScoreAsset::new(xml.to_vec(), "rest.xml".into(), &file).unwrap();
        let marks = asset.fingering_marks(&file);
        assert!(marks.values().all(|mark| mark.slur_from.is_none()));
    }
    #[test]
    fn simultaneous_voices_with_same_slur_number_keep_separate_chains() {
        let xml = br#"<score-partwise version="4.0">
          <part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
          <part id="P1"><measure number="1"><attributes><divisions>1</divisions></attributes>
          <note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
          <notations><slur type="start" number="1"/></notations></note>
          <note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
          <notations><slur type="stop" number="1"/></notations></note>
          <backup><duration>2</duration></backup>
          <note><pitch><step>G</step><octave>4</octave></pitch><duration>1</duration><voice>2</voice>
          <notations><slur type="start" number="1"/></notations></note>
          <note><pitch><step>A</step><octave>4</octave></pitch><duration>1</duration><voice>2</voice>
          <notations><slur type="stop" number="1"/></notations></note>
          </measure></part></score-partwise>"#;
        let score = neothesia_core::musicxml::import_musicxml_document(xml).unwrap();
        let performance = neothesia_core::score_performance::generate(&score, 120).unwrap();
        let file = MidiFile::from_smf("voices", &performance.smf).unwrap();
        let asset = ScoreAsset::new(xml.to_vec(), "voices.xml".into(), &file).unwrap();
        let marks = asset.fingering_marks(&file);
        let id = |pitch| file.tracks.iter().flat_map(|t| t.notes.iter().enumerate()
            .filter_map(move |(index, note)| (note.note == pitch).then_some((t.track_id, index))))
            .next().unwrap();
        assert_eq!(marks.get(&id(62)).unwrap().slur_from, Some(id(60)));
        assert_eq!(marks.get(&id(69)).unwrap().slur_from, Some(id(67)));
    }
}
