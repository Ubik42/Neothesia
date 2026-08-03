//! Renderer-neutral synchronization between aligned performance notes and an
//! engraved score.
//!
//! Renderers own layout and their private element identifiers. The Rust score
//! model remains authoritative: an adapter must return a [`ScoreRenderIndex`]
//! that maps those private identifiers back to stable score-event identities.

use std::{collections::HashMap, time::Duration};

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
