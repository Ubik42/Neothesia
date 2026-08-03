//! Renderer-neutral synchronization between aligned performance notes and an
//! engraved score.
//!
//! Renderers own layout and their private element identifiers. The Rust score
//! model remains authoritative: an adapter must return a [`ScoreRenderIndex`]
//! that maps those private identifiers back to stable score-event identities.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    time::Duration,
};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    musicxml::ScoreEventId,
    score_alignment::{MidiNoteId, PerformanceNote, ScoreMidiAlignment},
    score_playback::ScoreEventOccurrenceId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedScoreElement {
    pub source_id: ScoreEventId,
    pub renderer_id: String,
    pub page_index: usize,
}

/// Renderer-exported evidence used to correlate a generated SVG element back
/// to a native score event. Values are nominal score MIDI values, not human
/// performance timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScoreNoteSemanticKey {
    pub onset_millis: i64,
    pub pitch: u8,
    pub duration_millis: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeScoreNoteEvidence {
    pub source_id: ScoreEventId,
    pub key: ScoreNoteSemanticKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendererScoreNoteEvidence {
    pub renderer_id: String,
    pub page_index: usize,
    pub key: ScoreNoteSemanticKey,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerovioManifest {
    pub schema_version: u32,
    pub renderer_name: String,
    pub renderer_version: String,
    pub source_sha256: String,
    pub source_bytes: u64,
    pub page_count: usize,
    pub pages: Vec<VerovioManifestPage>,
    pub notes: Vec<VerovioManifestNote>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerovioManifestPage {
    pub page_index: usize,
    pub svg_file: String,
    pub svg_bytes: u64,
    #[serde(default)]
    pub svg_sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerovioManifestNote {
    pub renderer_id: String,
    pub page_index: usize,
    pub onset_millis: i64,
    pub pitch: u16,
    pub duration_millis: i64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VerovioManifestError {
    #[error("invalid Verovio manifest JSON: {0}")]
    InvalidJson(String),
    #[error("unsupported Verovio manifest schema {0}")]
    UnsupportedSchema(u32),
    #[error("manifest renderer must be verovio with a non-empty version")]
    InvalidRenderer,
    #[error("manifest source fingerprint does not match the requested score")]
    SourceMismatch,
    #[error("manifest page table must contain every page exactly once")]
    InvalidPages,
    #[error("manifest SVG path is unsafe: {0}")]
    UnsafeSvgPath(String),
    #[error("manifest note `{0}` is invalid or duplicated")]
    InvalidNote(String),
    #[error("manifest page `{0}` has no valid content hash")]
    MissingPageHash(String),
    #[error("could not read manifest page `{path}`: {message}")]
    PageIo { path: String, message: String },
    #[error("manifest page `{0}` size does not match")]
    PageSizeMismatch(String),
    #[error("manifest page `{0}` fingerprint does not match")]
    PageHashMismatch(String),
    #[error("manifest has no page {0}")]
    PageNotFound(usize),
}

impl VerovioManifest {
    pub fn parse_and_validate(
        json: &str,
        expected_source_sha256: &str,
    ) -> Result<Self, VerovioManifestError> {
        let manifest: Self = serde_json::from_str(json)
            .map_err(|error| VerovioManifestError::InvalidJson(error.to_string()))?;
        manifest.validate(expected_source_sha256)?;
        Ok(manifest)
    }

    pub fn validate(&self, expected_source_sha256: &str) -> Result<(), VerovioManifestError> {
        if !matches!(self.schema_version, 1 | 2) {
            return Err(VerovioManifestError::UnsupportedSchema(self.schema_version));
        }
        if self.renderer_name != "verovio" || self.renderer_version.trim().is_empty() {
            return Err(VerovioManifestError::InvalidRenderer);
        }
        let valid_hash = self.source_sha256.len() == 64
            && self
                .source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            && self.source_sha256 == self.source_sha256.to_ascii_lowercase();
        if !valid_hash || self.source_sha256 != expected_source_sha256 {
            return Err(VerovioManifestError::SourceMismatch);
        }
        if self.page_count == 0 || self.pages.len() != self.page_count {
            return Err(VerovioManifestError::InvalidPages);
        }
        let mut page_indices: Vec<_> = self.pages.iter().map(|page| page.page_index).collect();
        page_indices.sort_unstable();
        if page_indices != (0..self.page_count).collect::<Vec<_>>() {
            return Err(VerovioManifestError::InvalidPages);
        }
        for page in &self.pages {
            let path = std::path::Path::new(&page.svg_file);
            let safe = !page.svg_file.is_empty()
                && path.is_relative()
                && path
                    .components()
                    .all(|component| matches!(component, std::path::Component::Normal(_)));
            if !safe {
                return Err(VerovioManifestError::UnsafeSvgPath(page.svg_file.clone()));
            }
            if self.schema_version >= 2 && !valid_sha256(page.svg_sha256.as_deref()) {
                return Err(VerovioManifestError::MissingPageHash(page.svg_file.clone()));
            }
        }
        let mut renderer_ids = std::collections::HashSet::new();
        for note in &self.notes {
            let valid = !note.renderer_id.trim().is_empty()
                && renderer_ids.insert(note.renderer_id.clone())
                && note.page_index < self.page_count
                && note.onset_millis >= 0
                && note.duration_millis >= 0
                && note.pitch <= 127;
            if !valid {
                return Err(VerovioManifestError::InvalidNote(note.renderer_id.clone()));
            }
        }
        Ok(())
    }

    pub fn validate_page_files(
        &self,
        root: impl AsRef<std::path::Path>,
    ) -> Result<(), VerovioManifestError> {
        for page in &self.pages {
            let Some(expected_hash) = page
                .svg_sha256
                .as_deref()
                .filter(|hash| valid_sha256(Some(hash)))
            else {
                return Err(VerovioManifestError::MissingPageHash(page.svg_file.clone()));
            };
            let page_path = root.as_ref().join(&page.svg_file);
            let bytes =
                std::fs::read(&page_path).map_err(|error| VerovioManifestError::PageIo {
                    path: page.svg_file.clone(),
                    message: error.to_string(),
                })?;
            verify_page_bytes(page, expected_hash, &bytes)?;
        }
        Ok(())
    }

    pub fn read_verified_page(
        &self,
        root: impl AsRef<std::path::Path>,
        page_index: usize,
    ) -> Result<Vec<u8>, VerovioManifestError> {
        let page = self
            .pages
            .iter()
            .find(|page| page.page_index == page_index)
            .ok_or(VerovioManifestError::PageNotFound(page_index))?;
        let Some(expected_hash) = page
            .svg_sha256
            .as_deref()
            .filter(|hash| valid_sha256(Some(hash)))
        else {
            return Err(VerovioManifestError::MissingPageHash(page.svg_file.clone()));
        };
        let bytes = std::fs::read(root.as_ref().join(&page.svg_file)).map_err(|error| {
            VerovioManifestError::PageIo {
                path: page.svg_file.clone(),
                message: error.to_string(),
            }
        })?;
        verify_page_bytes(page, expected_hash, &bytes)?;
        Ok(bytes)
    }

    pub fn renderer_evidence(&self) -> Vec<RendererScoreNoteEvidence> {
        self.notes
            .iter()
            .map(|note| RendererScoreNoteEvidence {
                renderer_id: note.renderer_id.clone(),
                page_index: note.page_index,
                key: ScoreNoteSemanticKey {
                    onset_millis: note.onset_millis,
                    pitch: note.pitch as u8,
                    duration_millis: note.duration_millis,
                },
            })
            .collect()
    }
}

fn valid_sha256(value: Option<&str>) -> bool {
    value.is_some_and(|hash| {
        hash.len() == 64
            && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            && hash == hash.to_ascii_lowercase()
    })
}

fn verify_page_bytes(
    page: &VerovioManifestPage,
    expected_hash: &str,
    bytes: &[u8],
) -> Result<(), VerovioManifestError> {
    if bytes.len() as u64 != page.svg_bytes {
        return Err(VerovioManifestError::PageSizeMismatch(
            page.svg_file.clone(),
        ));
    }
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected_hash {
        return Err(VerovioManifestError::PageHashMismatch(
            page.svg_file.clone(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScoreDocumentGeneration(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScorePageRequest {
    pub generation: ScoreDocumentGeneration,
    pub page_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLoadDisposition {
    Accepted,
    StaleGeneration,
    OutsideWindow,
    Unexpected,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ScorePageCacheError {
    #[error("an engraved score must contain at least one page")]
    EmptyDocument,
    #[error("page {page_index} is outside the {page_count}-page document")]
    PageOutOfRange {
        page_index: usize,
        page_count: usize,
    },
}

/// A bounded cache for the focused score page and its immediate neighbours.
/// Worker results carry a document generation, so output from a previously
/// opened song cannot enter the current cache.
#[derive(Debug, Clone)]
pub struct ScorePageCache<T> {
    generation: ScoreDocumentGeneration,
    next_generation: u64,
    page_count: usize,
    focus: Option<usize>,
    desired: BTreeSet<usize>,
    pending: BTreeSet<usize>,
    pages: BTreeMap<usize, T>,
}

impl<T> Default for ScorePageCache<T> {
    fn default() -> Self {
        Self {
            generation: ScoreDocumentGeneration(0),
            next_generation: 1,
            page_count: 0,
            focus: None,
            desired: BTreeSet::new(),
            pending: BTreeSet::new(),
            pages: BTreeMap::new(),
        }
    }
}

impl<T> ScorePageCache<T> {
    pub fn begin_document(
        &mut self,
        page_count: usize,
    ) -> Result<ScoreDocumentGeneration, ScorePageCacheError> {
        if page_count == 0 {
            return Err(ScorePageCacheError::EmptyDocument);
        }
        self.generation = ScoreDocumentGeneration(self.next_generation);
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        self.page_count = page_count;
        self.focus = None;
        self.desired.clear();
        self.pending.clear();
        self.pages.clear();
        Ok(self.generation)
    }

    /// Changes focus and returns new load requests in current/previous/next
    /// priority order. Pages outside that three-page window are evicted.
    pub fn focus_page(
        &mut self,
        page_index: usize,
    ) -> Result<Vec<ScorePageRequest>, ScorePageCacheError> {
        if page_index >= self.page_count {
            return Err(ScorePageCacheError::PageOutOfRange {
                page_index,
                page_count: self.page_count,
            });
        }
        self.focus = Some(page_index);
        self.desired.clear();
        self.desired.insert(page_index);
        if page_index > 0 {
            self.desired.insert(page_index - 1);
        }
        if page_index + 1 < self.page_count {
            self.desired.insert(page_index + 1);
        }
        self.pages.retain(|index, _| self.desired.contains(index));
        self.pending.retain(|index| self.desired.contains(index));

        let priorities = [
            Some(page_index),
            page_index.checked_sub(1),
            page_index.checked_add(1),
        ];
        let mut requests = Vec::new();
        for candidate in priorities.into_iter().flatten() {
            if candidate < self.page_count
                && !self.pages.contains_key(&candidate)
                && self.pending.insert(candidate)
            {
                requests.push(ScorePageRequest {
                    generation: self.generation,
                    page_index: candidate,
                });
            }
        }
        Ok(requests)
    }

    pub fn complete(&mut self, request: ScorePageRequest, page: T) -> PageLoadDisposition {
        if request.generation != self.generation {
            return PageLoadDisposition::StaleGeneration;
        }
        if !self.desired.contains(&request.page_index) {
            self.pending.remove(&request.page_index);
            return PageLoadDisposition::OutsideWindow;
        }
        if !self.pending.remove(&request.page_index) {
            return PageLoadDisposition::Unexpected;
        }
        self.pages.insert(request.page_index, page);
        PageLoadDisposition::Accepted
    }

    pub fn fail(&mut self, request: ScorePageRequest) -> PageLoadDisposition {
        if request.generation != self.generation {
            return PageLoadDisposition::StaleGeneration;
        }
        if !self.desired.contains(&request.page_index) {
            self.pending.remove(&request.page_index);
            return PageLoadDisposition::OutsideWindow;
        }
        if !self.pending.remove(&request.page_index) {
            return PageLoadDisposition::Unexpected;
        }
        PageLoadDisposition::Accepted
    }

    pub fn page(&self, page_index: usize) -> Option<&T> {
        self.pages.get(&page_index)
    }

    pub fn cached_pages(&self) -> impl Iterator<Item = usize> + '_ {
        self.pages.keys().copied()
    }

    pub fn focus(&self) -> Option<usize> {
        self.focus
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreCorrelationAmbiguity {
    pub key: ScoreNoteSemanticKey,
    pub native_count: usize,
    pub renderer_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScoreCorrelation {
    pub elements: Vec<RenderedScoreElement>,
    pub unmatched_native: Vec<ScoreEventId>,
    pub unmatched_renderer: Vec<String>,
    pub ambiguities: Vec<ScoreCorrelationAmbiguity>,
}

/// Correlates only unique semantic groups. Unisons in separate voices or any
/// other duplicate evidence remain explicit ambiguities; source order is never
/// used as a hidden tie-breaker.
pub fn correlate_score_notes(
    native: impl IntoIterator<Item = NativeScoreNoteEvidence>,
    rendered: impl IntoIterator<Item = RendererScoreNoteEvidence>,
) -> ScoreCorrelation {
    let mut native_groups: BTreeMap<_, Vec<_>> = BTreeMap::new();
    let mut renderer_groups: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for note in native {
        native_groups.entry(note.key).or_default().push(note);
    }
    for note in rendered {
        renderer_groups.entry(note.key).or_default().push(note);
    }

    let mut keys: Vec<_> = native_groups
        .keys()
        .chain(renderer_groups.keys())
        .copied()
        .collect();
    keys.sort();
    keys.dedup();
    let mut result = ScoreCorrelation::default();
    for key in keys {
        let native_notes = native_groups.remove(&key).unwrap_or_default();
        let renderer_notes = renderer_groups.remove(&key).unwrap_or_default();
        if native_notes.len() == 1 && renderer_notes.len() == 1 {
            let native_note = native_notes.into_iter().next().unwrap();
            let renderer_note = renderer_notes.into_iter().next().unwrap();
            result.elements.push(RenderedScoreElement {
                source_id: native_note.source_id,
                renderer_id: renderer_note.renderer_id,
                page_index: renderer_note.page_index,
            });
            continue;
        }
        if native_notes.len() > 1 || renderer_notes.len() > 1 {
            result.ambiguities.push(ScoreCorrelationAmbiguity {
                key,
                native_count: native_notes.len(),
                renderer_count: renderer_notes.len(),
            });
        }
        result
            .unmatched_native
            .extend(native_notes.into_iter().map(|note| note.source_id));
        result
            .unmatched_renderer
            .extend(renderer_notes.into_iter().map(|note| note.renderer_id));
    }
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreRenderIndex {
    page_count: usize,
    by_source: HashMap<ScoreEventId, RenderedScoreElement>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RenderIndexError {
    #[error("an engraved score must contain at least one page")]
    EmptyDocument,
    #[error("renderer element identifiers must not be empty")]
    EmptyRendererId,
    #[error("renderer element `{renderer_id}` points outside the {page_count}-page document")]
    PageOutOfRange {
        renderer_id: String,
        page_count: usize,
    },
    #[error("score event {0:?} has more than one rendered element")]
    DuplicateSource(ScoreEventId),
    #[error("renderer element identifier `{0}` is not unique")]
    DuplicateRendererId(String),
}

impl ScoreRenderIndex {
    pub fn new(
        page_count: usize,
        elements: impl IntoIterator<Item = RenderedScoreElement>,
    ) -> Result<Self, RenderIndexError> {
        if page_count == 0 {
            return Err(RenderIndexError::EmptyDocument);
        }
        let mut by_source = HashMap::new();
        let mut renderer_ids = std::collections::HashSet::new();
        for element in elements {
            if element.renderer_id.trim().is_empty() {
                return Err(RenderIndexError::EmptyRendererId);
            }
            if element.page_index >= page_count {
                return Err(RenderIndexError::PageOutOfRange {
                    renderer_id: element.renderer_id,
                    page_count,
                });
            }
            if !renderer_ids.insert(element.renderer_id.clone()) {
                return Err(RenderIndexError::DuplicateRendererId(element.renderer_id));
            }
            let source_id = element.source_id.clone();
            if by_source.insert(source_id.clone(), element).is_some() {
                return Err(RenderIndexError::DuplicateSource(source_id));
            }
        }
        Ok(Self {
            page_count,
            by_source,
        })
    }

    pub fn page_count(&self) -> usize {
        self.page_count
    }

    pub fn element(&self, source_id: &ScoreEventId) -> Option<&RenderedScoreElement> {
        self.by_source.get(source_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreHighlightCue {
    pub score_id: ScoreEventOccurrenceId,
    pub midi_id: MidiNoteId,
    pub start: Duration,
    pub end: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScoreHighlightTimeline {
    pub cues: Vec<ScoreHighlightCue>,
    pub missing_performance_notes: Vec<MidiNoteId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedHighlight {
    pub score_id: ScoreEventOccurrenceId,
    pub renderer_id: String,
    pub page_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScoreHighlightFrame {
    pub focus: Option<RenderedHighlight>,
    pub active: Vec<RenderedHighlight>,
    pub missing_renderer_elements: Vec<ScoreEventOccurrenceId>,
}

impl ScoreHighlightTimeline {
    pub fn from_alignment(
        alignment: &ScoreMidiAlignment,
        performance_notes: &[PerformanceNote],
    ) -> Self {
        let performance_by_id: HashMap<_, _> = performance_notes
            .iter()
            .map(|note| (note.id, note))
            .collect();
        let mut timeline = Self::default();
        for matched in &alignment.matches {
            let Some(note) = performance_by_id.get(&matched.midi_id) else {
                timeline.missing_performance_notes.push(matched.midi_id);
                continue;
            };
            timeline.cues.push(ScoreHighlightCue {
                score_id: matched.score_id.clone(),
                midi_id: matched.midi_id,
                start: note.timestamp,
                end: note.timestamp.saturating_add(note.duration),
            });
        }
        timeline.cues.sort_by(|left, right| {
            left.start
                .cmp(&right.start)
                .then_with(|| left.midi_id.cmp(&right.midi_id))
        });
        timeline.missing_performance_notes.sort();
        timeline
    }

    /// Produces the active engraved notes plus a stable focus used for page
    /// following. During a rest, focus remains on the latest started note
    /// instead of jumping ahead to the next page.
    pub fn frame_at(&self, timestamp: Duration, index: &ScoreRenderIndex) -> ScoreHighlightFrame {
        let focus_cue = self.cues.iter().rev().find(|cue| cue.start <= timestamp);
        let active_cues = self
            .cues
            .iter()
            .filter(|cue| cue.start <= timestamp && timestamp < cue.end);
        let mut frame = ScoreHighlightFrame {
            focus: focus_cue.and_then(|cue| rendered_highlight(cue, index)),
            ..Default::default()
        };
        for cue in active_cues {
            if let Some(highlight) = rendered_highlight(cue, index) {
                frame.active.push(highlight);
            } else if !frame.missing_renderer_elements.contains(&cue.score_id) {
                frame.missing_renderer_elements.push(cue.score_id.clone());
            }
        }
        frame
    }
}

fn rendered_highlight(
    cue: &ScoreHighlightCue,
    index: &ScoreRenderIndex,
) -> Option<RenderedHighlight> {
    index
        .element(&cue.score_id.source_id)
        .map(|rendered| RenderedHighlight {
            score_id: cue.score_id.clone(),
            renderer_id: rendered.renderer_id.clone(),
            page_index: rendered.page_index,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{musicxml::ScoreEventKind, score_alignment::AlignedNote};

    fn score_id(ordinal: u32, occurrence: u32) -> ScoreEventOccurrenceId {
        ScoreEventOccurrenceId {
            source_id: ScoreEventId {
                part_id: "P1".into(),
                measure_ordinal: 2,
                kind: ScoreEventKind::Note,
                ordinal,
            },
            measure_occurrence_ordinal: occurrence,
        }
    }

    fn aligned(score_id: ScoreEventOccurrenceId, note_index: usize) -> AlignedNote {
        AlignedNote {
            score_id,
            midi_id: MidiNoteId {
                track_id: 0,
                note_index,
            },
            onset_delta_micros: 0,
            duration_delta_micros: 0,
            confidence_percent: 100,
            exact_score_projection: true,
        }
    }

    fn performed(note_index: usize, start_ms: u64, duration_ms: u64) -> PerformanceNote {
        PerformanceNote {
            id: MidiNoteId {
                track_id: 0,
                note_index,
            },
            pitch: 60,
            timestamp: Duration::from_millis(start_ms),
            duration: Duration::from_millis(duration_ms),
        }
    }

    fn semantic(onset_millis: i64, pitch: u8) -> ScoreNoteSemanticKey {
        ScoreNoteSemanticKey {
            onset_millis,
            pitch,
            duration_millis: 250,
        }
    }

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn manifest_json(svg_file: &str, schema_version: u32) -> String {
        format!(
            r#"{{
  "schemaVersion": {schema_version},
  "rendererName": "verovio",
  "rendererVersion": "6.1.0",
  "sourceSha256": "{HASH}",
  "sourceBytes": 42,
  "pageCount": 1,
  "pages": [{{"pageIndex": 0, "svgFile": "{svg_file}", "svgBytes": 100}}],
  "notes": [{{
    "rendererId": "note-a",
    "pageIndex": 0,
    "onsetMillis": 250,
    "pitch": 64,
    "durationMillis": 500
  }}]
}}"#
        )
    }

    fn manifest_v2_json(svg_file: &str, svg: &[u8]) -> String {
        let hash = format!("{:x}", Sha256::digest(svg));
        format!(
            r#"{{
  "schemaVersion": 2,
  "rendererName": "verovio",
  "rendererVersion": "6.1.0",
  "sourceSha256": "{HASH}",
  "sourceBytes": 42,
  "pageCount": 1,
  "pages": [{{
    "pageIndex": 0,
    "svgFile": "{svg_file}",
    "svgBytes": {},
    "svgSha256": "{hash}"
  }}],
  "notes": []
}}"#,
            svg.len()
        )
    }

    #[test]
    fn validates_and_converts_a_versioned_verovio_manifest() {
        let manifest =
            VerovioManifest::parse_and_validate(&manifest_json("score-page-1.svg", 1), HASH)
                .unwrap();
        assert_eq!(manifest.page_count, 1);
        assert_eq!(
            manifest.renderer_evidence(),
            vec![RendererScoreNoteEvidence {
                renderer_id: "note-a".into(),
                page_index: 0,
                key: ScoreNoteSemanticKey {
                    onset_millis: 250,
                    pitch: 64,
                    duration_millis: 500,
                },
            }]
        );
    }

    #[test]
    fn rejects_stale_unsupported_and_unsafe_manifests() {
        assert_eq!(
            VerovioManifest::parse_and_validate(&manifest_json("score.svg", 3), HASH),
            Err(VerovioManifestError::UnsupportedSchema(3))
        );
        assert_eq!(
            VerovioManifest::parse_and_validate(
                &manifest_json("score.svg", 1),
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            ),
            Err(VerovioManifestError::SourceMismatch)
        );
        assert_eq!(
            VerovioManifest::parse_and_validate(&manifest_json("../score.svg", 1), HASH),
            Err(VerovioManifestError::UnsafeSvgPath("../score.svg".into()))
        );
    }

    #[test]
    fn verifies_page_size_and_fingerprint_before_loading() {
        let root = std::env::temp_dir().join(format!(
            "neothesia-verovio-page-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let svg = b"<svg/>";
        std::fs::write(root.join("score.svg"), svg).unwrap();
        let manifest =
            VerovioManifest::parse_and_validate(&manifest_v2_json("score.svg", svg), HASH).unwrap();
        manifest.validate_page_files(&root).unwrap();
        assert_eq!(manifest.read_verified_page(&root, 0).unwrap(), svg);
        assert_eq!(
            manifest.read_verified_page(&root, 1),
            Err(VerovioManifestError::PageNotFound(1))
        );

        std::fs::write(root.join("score.svg"), b"<bad/>").unwrap();
        assert_eq!(
            manifest.validate_page_files(&root),
            Err(VerovioManifestError::PageHashMismatch("score.svg".into()))
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn page_cache_prioritizes_focus_and_neighbours_then_evicts() {
        let mut cache = ScorePageCache::default();
        cache.begin_document(5).unwrap();
        let initial = cache.focus_page(2).unwrap();
        assert_eq!(
            initial
                .iter()
                .map(|request| request.page_index)
                .collect::<Vec<_>>(),
            vec![2, 1, 3]
        );
        for request in initial {
            assert_eq!(
                cache.complete(request, format!("page-{}", request.page_index)),
                PageLoadDisposition::Accepted
            );
        }
        assert_eq!(cache.cached_pages().collect::<Vec<_>>(), vec![1, 2, 3]);

        let shifted = cache.focus_page(3).unwrap();
        assert_eq!(
            shifted
                .iter()
                .map(|request| request.page_index)
                .collect::<Vec<_>>(),
            vec![4]
        );
        assert_eq!(cache.cached_pages().collect::<Vec<_>>(), vec![2, 3]);
        assert_eq!(
            cache.complete(shifted[0], "page-4".to_owned()),
            PageLoadDisposition::Accepted
        );
        assert_eq!(cache.cached_pages().collect::<Vec<_>>(), vec![2, 3, 4]);
        assert_eq!(cache.focus(), Some(3));
    }

    #[test]
    fn page_cache_rejects_old_document_and_old_window_results() {
        let mut cache = ScorePageCache::default();
        let first_generation = cache.begin_document(4).unwrap();
        let old_window = cache.focus_page(0).unwrap();
        assert_eq!(old_window.len(), 2);

        cache.focus_page(3).unwrap();
        assert_eq!(
            cache.complete(old_window[0], "late-page"),
            PageLoadDisposition::OutsideWindow
        );

        let old_document = ScorePageRequest {
            generation: first_generation,
            page_index: 3,
        };
        let second_generation = cache.begin_document(2).unwrap();
        assert_ne!(first_generation, second_generation);
        cache.focus_page(0).unwrap();
        assert_eq!(
            cache.complete(old_document, "wrong-song"),
            PageLoadDisposition::StaleGeneration
        );
        assert!(cache.cached_pages().next().is_none());
    }

    #[test]
    fn page_cache_rejects_duplicate_and_invalid_requests() {
        let mut cache = ScorePageCache::default();
        assert_eq!(
            cache.begin_document(0),
            Err(ScorePageCacheError::EmptyDocument)
        );
        cache.begin_document(2).unwrap();
        assert_eq!(
            cache.focus_page(2),
            Err(ScorePageCacheError::PageOutOfRange {
                page_index: 2,
                page_count: 2,
            })
        );
        let requests = cache.focus_page(0).unwrap();
        let request = requests[0];
        let neighbour = requests[1];
        assert_eq!(
            cache.complete(request, "page"),
            PageLoadDisposition::Accepted
        );
        assert_eq!(
            cache.complete(request, "duplicate"),
            PageLoadDisposition::Unexpected
        );
        assert_eq!(cache.page(0), Some(&"page"));

        assert_eq!(neighbour.page_index, 1);
        assert_eq!(cache.fail(neighbour), PageLoadDisposition::Accepted);
        assert_eq!(
            cache.focus_page(0).unwrap(),
            vec![neighbour],
            "a failed current-window page must be retryable"
        );
    }

    #[test]
    fn correlates_only_unique_semantic_note_groups() {
        let unique = score_id(0, 0).source_id;
        let unison_a = score_id(1, 0).source_id;
        let unison_b = score_id(2, 0).source_id;
        let correlation = correlate_score_notes(
            [
                NativeScoreNoteEvidence {
                    source_id: unique.clone(),
                    key: semantic(0, 60),
                },
                NativeScoreNoteEvidence {
                    source_id: unison_a.clone(),
                    key: semantic(250, 64),
                },
                NativeScoreNoteEvidence {
                    source_id: unison_b.clone(),
                    key: semantic(250, 64),
                },
            ],
            [
                RendererScoreNoteEvidence {
                    renderer_id: "unique".into(),
                    page_index: 0,
                    key: semantic(0, 60),
                },
                RendererScoreNoteEvidence {
                    renderer_id: "unison-1".into(),
                    page_index: 0,
                    key: semantic(250, 64),
                },
                RendererScoreNoteEvidence {
                    renderer_id: "unison-2".into(),
                    page_index: 0,
                    key: semantic(250, 64),
                },
            ],
        );
        assert_eq!(correlation.elements.len(), 1);
        assert_eq!(correlation.elements[0].source_id, unique);
        assert_eq!(correlation.unmatched_native, vec![unison_a, unison_b]);
        assert_eq!(
            correlation.unmatched_renderer,
            vec!["unison-1".to_owned(), "unison-2".to_owned()]
        );
        assert_eq!(
            correlation.ambiguities,
            vec![ScoreCorrelationAmbiguity {
                key: semantic(250, 64),
                native_count: 2,
                renderer_count: 2,
            }]
        );
    }

    #[test]
    fn reports_one_sided_correlation_gaps() {
        let native_only = score_id(0, 0).source_id;
        let correlation = correlate_score_notes(
            [NativeScoreNoteEvidence {
                source_id: native_only.clone(),
                key: semantic(0, 60),
            }],
            [RendererScoreNoteEvidence {
                renderer_id: "renderer-only".into(),
                page_index: 0,
                key: semantic(500, 72),
            }],
        );
        assert_eq!(correlation.unmatched_native, vec![native_only]);
        assert_eq!(correlation.unmatched_renderer, vec!["renderer-only"]);
        assert!(correlation.ambiguities.is_empty());
    }

    #[test]
    fn rejects_ambiguous_or_out_of_range_renderer_mappings() {
        let source = score_id(0, 0).source_id;
        let duplicate = vec![
            RenderedScoreElement {
                source_id: source.clone(),
                renderer_id: "n1".into(),
                page_index: 0,
            },
            RenderedScoreElement {
                source_id: source.clone(),
                renderer_id: "n2".into(),
                page_index: 0,
            },
        ];
        assert_eq!(
            ScoreRenderIndex::new(1, duplicate),
            Err(RenderIndexError::DuplicateSource(source))
        );
        assert!(matches!(
            ScoreRenderIndex::new(
                1,
                [RenderedScoreElement {
                    source_id: score_id(1, 0).source_id,
                    renderer_id: "n3".into(),
                    page_index: 1,
                }]
            ),
            Err(RenderIndexError::PageOutOfRange { .. })
        ));
    }

    #[test]
    fn highlights_overlapping_chord_notes_and_keeps_focus_through_a_rest() {
        let first = score_id(0, 0);
        let second = score_id(1, 0);
        let alignment = ScoreMidiAlignment {
            matches: vec![aligned(first.clone(), 0), aligned(second.clone(), 1)],
            ..Default::default()
        };
        let timeline = ScoreHighlightTimeline::from_alignment(
            &alignment,
            &[performed(1, 100, 500), performed(0, 100, 250)],
        );
        let index = ScoreRenderIndex::new(
            2,
            [
                RenderedScoreElement {
                    source_id: first.source_id.clone(),
                    renderer_id: "vrv-first".into(),
                    page_index: 0,
                },
                RenderedScoreElement {
                    source_id: second.source_id.clone(),
                    renderer_id: "vrv-second".into(),
                    page_index: 1,
                },
            ],
        )
        .unwrap();

        let chord = timeline.frame_at(Duration::from_millis(200), &index);
        assert_eq!(chord.active.len(), 2);
        assert_eq!(chord.focus.unwrap().renderer_id, "vrv-second");

        let rest = timeline.frame_at(Duration::from_millis(700), &index);
        assert!(rest.active.is_empty());
        assert_eq!(rest.focus.unwrap().renderer_id, "vrv-second");
    }

    #[test]
    fn repeated_occurrences_reuse_written_element_without_losing_visit_identity() {
        let first_visit = score_id(0, 0);
        let repeat_visit = score_id(0, 3);
        let alignment = ScoreMidiAlignment {
            matches: vec![
                aligned(first_visit.clone(), 0),
                aligned(repeat_visit.clone(), 1),
            ],
            ..Default::default()
        };
        let timeline = ScoreHighlightTimeline::from_alignment(
            &alignment,
            &[performed(0, 0, 100), performed(1, 1_000, 100)],
        );
        let index = ScoreRenderIndex::new(
            1,
            [RenderedScoreElement {
                source_id: first_visit.source_id.clone(),
                renderer_id: "same-written-note".into(),
                page_index: 0,
            }],
        )
        .unwrap();

        let repeated = timeline.frame_at(Duration::from_millis(1_050), &index);
        assert_eq!(repeated.active[0].renderer_id, "same-written-note");
        assert_eq!(repeated.active[0].score_id, repeat_visit);
    }

    #[test]
    fn reports_missing_midi_and_renderer_elements_without_panicking() {
        let visible = score_id(0, 0);
        let missing = score_id(1, 0);
        let alignment = ScoreMidiAlignment {
            matches: vec![aligned(visible.clone(), 0), aligned(missing.clone(), 9)],
            ..Default::default()
        };
        let timeline = ScoreHighlightTimeline::from_alignment(
            &alignment,
            &[performed(0, 0, 500), performed(9, 0, 500)],
        );
        let index = ScoreRenderIndex::new(
            1,
            [RenderedScoreElement {
                source_id: visible.source_id,
                renderer_id: "visible".into(),
                page_index: 0,
            }],
        )
        .unwrap();
        let frame = timeline.frame_at(Duration::from_millis(100), &index);
        assert_eq!(frame.active.len(), 1);
        assert_eq!(frame.missing_renderer_elements, vec![missing]);

        let alignment_with_missing_midi = ScoreMidiAlignment {
            matches: vec![aligned(score_id(2, 0), 10)],
            ..Default::default()
        };
        let missing_midi =
            ScoreHighlightTimeline::from_alignment(&alignment_with_missing_midi, &[]);
        assert_eq!(
            missing_midi.missing_performance_notes,
            vec![MidiNoteId {
                track_id: 0,
                note_index: 10
            }]
        );
    }
}
