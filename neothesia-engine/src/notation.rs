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
pub struct ScoreAsset {
    pub bytes: Vec<u8>,
    pub name: String,
    pub score: Score,
    pub alignment: ScoreMidiAlignment,
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
            bytes,
            name,
            score,
            alignment,
        })
    }
    pub fn payload(&self, file: Option<&MidiFile>) -> serde_json::Value {
        let a = summarize_alignment(&self.alignment);
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
        serde_json::json!({"route":route,"bytes":self.bytes,"name":self.name,"compressed":self.bytes.starts_with(b"PK"),"title":self.score.title,"coverage":a.coverage_percent,"confidence":a.mean_confidence_percent,"readiness":a.readiness,"diagnostics":a.navigation_diagnostics,"warnings":self.score.warnings.iter().map(|w|format!("{w:?}")).collect::<Vec<_>>()})
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
                        let score_finger =
                            score_fingers.get(&m.score_id.source_id).copied().flatten();
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
#[derive(Deserialize)]
pub struct RenderNote {
    pub renderer_id: String,
    pub page_index: usize,
    pub onset_millis: f64,
    pub pitch: u8,
    pub duration_millis: f64,
}
