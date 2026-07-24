use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    time::Duration,
};

use midi_file::{MidiFile, tempo_track::TempoTrack};

use crate::musicxml::{Pitch, Score, ScoreEvent, ScoreEventId, ScoreTime, Step};
use crate::score_playback::{
    PlaybackLimits, PlaybackPlan, ScoreEventOccurrenceId, build_playback_plan,
    expand_event_occurrences,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedScoreEvent {
    pub id: ScoreEventId,
    pub timestamp: Duration,
    pub duration: Option<Duration>,
    pub exact_to_midi_pulse: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScoreTimelineProjection {
    pub events: Vec<ProjectedScoreEvent>,
    pub unprojected: Vec<ScoreEventId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedScoreOccurrence {
    pub id: ScoreEventOccurrenceId,
    pub timestamp: Duration,
    pub duration: Option<Duration>,
    pub exact_to_midi_pulse: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PerformedTimelineProjection {
    pub events: Vec<ProjectedScoreOccurrence>,
    pub unprojected: Vec<ScoreEventOccurrenceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MidiNoteId {
    pub track_id: usize,
    pub note_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceNote {
    pub id: MidiNoteId,
    pub pitch: u8,
    pub timestamp: Duration,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignedNote {
    pub score_id: ScoreEventOccurrenceId,
    pub midi_id: MidiNoteId,
    pub onset_delta_micros: i64,
    pub duration_delta_micros: i64,
    pub confidence_percent: u8,
    pub exact_score_projection: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreMidiAlignment {
    pub matches: Vec<AlignedNote>,
    pub unmatched_score: Vec<ScoreEventOccurrenceId>,
    pub unmatched_midi: Vec<MidiNoteId>,
    pub coverage_percent: u8,
    pub mean_confidence_percent: u8,
    pub navigation_complete: bool,
    pub navigation_diagnostics: Vec<String>,
}

impl Default for ScoreMidiAlignment {
    fn default() -> Self {
        Self {
            matches: Vec::new(),
            unmatched_score: Vec::new(),
            unmatched_midi: Vec::new(),
            coverage_percent: 0,
            mean_confidence_percent: 0,
            navigation_complete: true,
            navigation_diagnostics: Vec::new(),
        }
    }
}

#[derive(Clone)]
struct ScoreCandidate {
    id: ScoreEventOccurrenceId,
    timestamp: Duration,
    duration: Duration,
    exact: bool,
}

const MAX_ONSET_DELTA: Duration = Duration::from_millis(250);
const SKIP_COST: u64 = 300_000;
const MAX_DP_CELLS_PER_PITCH: usize = 1_000_000;

#[derive(Clone, Copy)]
enum AlignmentAction {
    Match,
    SkipScore,
    SkipMidi,
}

#[derive(Clone, Copy)]
struct AlignmentCell {
    cost: u64,
    action: AlignmentAction,
}

/// Projects semantic score events through the paired MIDI tempo map.
///
/// This does not claim that the score and MIDI are aligned. It only establishes
/// comparable timestamps and records pulse quantization before matching begins.
pub fn project_score_timeline(score: &Score, tempo: &TempoTrack) -> ScoreTimelineProjection {
    let mut projection = ScoreTimelineProjection::default();
    for event in score
        .parts
        .iter()
        .flat_map(|part| &part.measures)
        .flat_map(|measure| &measure.events)
    {
        let (id, onset, duration) = match event {
            ScoreEvent::Note(note) => (&note.id, note.onset, Some(note.duration)),
            ScoreEvent::Direction(direction) => (&direction.id, direction.onset, None),
        };
        let Some(start) = project_time(tempo, onset) else {
            projection.unprojected.push(id.clone());
            continue;
        };
        let (duration, end_exact) = if let Some(duration) = duration {
            let end = onset.add(duration);
            match project_time(tempo, end) {
                Some(end) => (
                    Some(end.timestamp.saturating_sub(start.timestamp)),
                    end.exact_to_pulse,
                ),
                None => {
                    projection.unprojected.push(id.clone());
                    continue;
                }
            }
        } else {
            (None, true)
        };
        projection.events.push(ProjectedScoreEvent {
            id: id.clone(),
            timestamp: start.timestamp,
            duration,
            exact_to_midi_pulse: start.exact_to_pulse && end_exact,
        });
    }
    projection.events.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.id.cmp(&right.id))
    });
    projection
}

/// Projects repeated event occurrences through the paired MIDI tempo map.
pub fn project_performed_part_timeline(
    part: &crate::musicxml::Part,
    plan: &PlaybackPlan,
    tempo: &TempoTrack,
) -> PerformedTimelineProjection {
    let mut projection = PerformedTimelineProjection::default();
    for event in expand_event_occurrences(part, plan) {
        let Some(start) = project_time(tempo, event.onset) else {
            projection.unprojected.push(event.id);
            continue;
        };
        let (duration, exact_end) = match event.duration {
            Some(duration) => {
                let Some(end) = project_time(tempo, event.onset.add(duration)) else {
                    projection.unprojected.push(event.id);
                    continue;
                };
                (
                    Some(end.timestamp.saturating_sub(start.timestamp)),
                    end.exact_to_pulse,
                )
            }
            None => (None, true),
        };
        projection.events.push(ProjectedScoreOccurrence {
            id: event.id,
            timestamp: start.timestamp,
            duration,
            exact_to_midi_pulse: start.exact_to_pulse && exact_end,
        });
    }
    projection.events.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.id.cmp(&right.id))
    });
    projection
}

/// Aligns a semantic score with the concrete MIDI notes already used by the
/// player. Drum-channel notes are excluded from the performance candidates.
pub fn align_score_to_midi(score: &Score, midi: &MidiFile) -> ScoreMidiAlignment {
    let performance: Vec<_> = midi
        .tracks
        .iter()
        .flat_map(|track| {
            track
                .notes
                .iter()
                .enumerate()
                .filter(|(_, note)| note.channel != 9)
                .map(|(note_index, note)| PerformanceNote {
                    id: MidiNoteId {
                        track_id: track.track_id,
                        note_index,
                    },
                    pitch: note.note,
                    timestamp: note.start,
                    duration: note.duration,
                })
        })
        .collect();
    align_score_occurrences(
        score,
        &midi.tempo_track,
        &performance,
        PlaybackLimits::default(),
    )
}

/// Aligns the performed occurrence order, including common repeats and
/// numbered endings, with concrete MIDI notes.
pub fn align_score_occurrences(
    score: &Score,
    tempo: &TempoTrack,
    performance: &[PerformanceNote],
    limits: PlaybackLimits,
) -> ScoreMidiAlignment {
    let mut score_by_pitch: BTreeMap<u8, Vec<ScoreCandidate>> = BTreeMap::new();
    let mut result = ScoreMidiAlignment::default();
    let mut reference_navigation: Option<Vec<(u32, u16)>> = None;

    for part in &score.parts {
        let plan = build_playback_plan(part, limits);
        if !plan.complete {
            result.navigation_complete = false;
        }
        result
            .navigation_diagnostics
            .extend(plan.diagnostics.iter().map(|diagnostic| {
                let location = diagnostic
                    .measure_ordinal
                    .map(|measure| format!(" measure {}", measure + 1))
                    .unwrap_or_default();
                format!("part {}{location}: {}", part.id, diagnostic.message)
            }));

        let signature: Vec<_> = plan
            .visits
            .iter()
            .map(|visit| (visit.source_measure_ordinal, visit.repeat_pass))
            .collect();
        if let Some(reference) = &reference_navigation {
            if *reference != signature {
                result.navigation_complete = false;
                result.navigation_diagnostics.push(format!(
                    "part {} has a different performed measure order",
                    part.id
                ));
            }
        } else {
            reference_navigation = Some(signature);
        }

        let pitch_by_source: HashMap<_, _> = part
            .measures
            .iter()
            .flat_map(|measure| &measure.events)
            .filter_map(|event| match event {
                ScoreEvent::Note(note) => note
                    .pitch
                    .as_ref()
                    .map(|pitch| (note.id.clone(), midi_pitch(pitch))),
                ScoreEvent::Direction(_) => None,
            })
            .collect();
        let projection = project_performed_part_timeline(part, &plan, tempo);
        for id in projection.unprojected {
            if pitch_by_source.contains_key(&id.source_id) {
                result.unmatched_score.push(id);
            }
        }
        for event in projection.events {
            let Some(pitch) = pitch_by_source.get(&event.id.source_id) else {
                continue;
            };
            let Some(pitch) = *pitch else {
                result.unmatched_score.push(event.id);
                continue;
            };
            score_by_pitch
                .entry(pitch)
                .or_default()
                .push(ScoreCandidate {
                    id: event.id,
                    timestamp: event.timestamp,
                    duration: event.duration.unwrap_or_default(),
                    exact: event.exact_to_midi_pulse,
                });
        }
    }
    finish_alignment(score_by_pitch, performance, result)
}

/// Aligns pitched score notes with ordered performance notes. Matching runs
/// independently per pitch so chords can cross tracks without arbitrary XML
/// ordering, while repeated pitches retain sequence order.
pub fn align_score_notes(
    score: &Score,
    tempo: &TempoTrack,
    performance: &[PerformanceNote],
) -> ScoreMidiAlignment {
    let projection = project_score_timeline(score, tempo);
    let projected: HashMap<_, _> = projection
        .events
        .into_iter()
        .map(|event| (event.id.clone(), event))
        .collect();
    let mut score_by_pitch: BTreeMap<u8, Vec<ScoreCandidate>> = BTreeMap::new();
    let mut result = ScoreMidiAlignment::default();

    for note in score
        .parts
        .iter()
        .flat_map(|part| &part.measures)
        .flat_map(|measure| &measure.events)
        .filter_map(|event| match event {
            ScoreEvent::Note(note) => Some(note),
            _ => None,
        })
    {
        let occurrence_id = ScoreEventOccurrenceId {
            source_id: note.id.clone(),
            measure_occurrence_ordinal: note.id.measure_ordinal,
        };
        let Some(written_pitch) = note.pitch.as_ref() else {
            continue;
        };
        let Some(pitch) = midi_pitch(written_pitch) else {
            result.unmatched_score.push(occurrence_id);
            continue;
        };
        let Some(projected) = projected.get(&note.id) else {
            result.unmatched_score.push(occurrence_id);
            continue;
        };
        score_by_pitch
            .entry(pitch)
            .or_default()
            .push(ScoreCandidate {
                id: occurrence_id,
                timestamp: projected.timestamp,
                duration: projected.duration.unwrap_or_default(),
                exact: projected.exact_to_midi_pulse,
            });
    }
    finish_alignment(score_by_pitch, performance, result)
}

fn finish_alignment(
    mut score_by_pitch: BTreeMap<u8, Vec<ScoreCandidate>>,
    performance: &[PerformanceNote],
    mut result: ScoreMidiAlignment,
) -> ScoreMidiAlignment {
    for notes in score_by_pitch.values_mut() {
        notes.sort_by(|left, right| {
            left.timestamp
                .cmp(&right.timestamp)
                .then_with(|| left.id.cmp(&right.id))
        });
    }

    let mut midi_by_pitch: BTreeMap<u8, Vec<PerformanceNote>> = BTreeMap::new();
    for note in performance {
        midi_by_pitch
            .entry(note.pitch)
            .or_default()
            .push(note.clone());
    }
    for notes in midi_by_pitch.values_mut() {
        notes.sort_by(|left, right| {
            left.timestamp
                .cmp(&right.timestamp)
                .then_with(|| left.id.cmp(&right.id))
        });
    }

    let pitches: BTreeSet<_> = score_by_pitch
        .keys()
        .chain(midi_by_pitch.keys())
        .copied()
        .collect();
    for pitch in pitches {
        let score_notes = score_by_pitch.remove(&pitch).unwrap_or_default();
        let midi_notes = midi_by_pitch.remove(&pitch).unwrap_or_default();
        if score_notes.len().saturating_mul(midi_notes.len()) <= MAX_DP_CELLS_PER_PITCH {
            align_pitch_dynamic(&score_notes, &midi_notes, &mut result);
        } else {
            align_pitch_linear(&score_notes, &midi_notes, &mut result);
        }
    }

    result.matches.sort_by(|left, right| {
        left.score_id
            .cmp(&right.score_id)
            .then_with(|| left.midi_id.cmp(&right.midi_id))
    });
    result.unmatched_score.sort();
    result.unmatched_midi.sort();
    let denominator = result.matches.len()
        + result
            .unmatched_score
            .len()
            .max(result.unmatched_midi.len());
    result.coverage_percent = percent(result.matches.len(), denominator);
    result.mean_confidence_percent = if result.matches.is_empty() {
        0
    } else {
        let total: usize = result
            .matches
            .iter()
            .map(|item| usize::from(item.confidence_percent))
            .sum();
        (total / result.matches.len()) as u8
    };
    result
}

fn align_pitch_dynamic(
    score: &[ScoreCandidate],
    midi: &[PerformanceNote],
    result: &mut ScoreMidiAlignment,
) {
    let width = midi.len() + 1;
    let mut cells = vec![
        AlignmentCell {
            cost: 0,
            action: AlignmentAction::Match,
        };
        (score.len() + 1) * width
    ];
    for index in 1..=score.len() {
        cells[index * width] = AlignmentCell {
            cost: index as u64 * SKIP_COST,
            action: AlignmentAction::SkipScore,
        };
    }
    for (index, cell) in cells.iter_mut().take(midi.len() + 1).enumerate().skip(1) {
        *cell = AlignmentCell {
            cost: index as u64 * SKIP_COST,
            action: AlignmentAction::SkipMidi,
        };
    }

    for score_index in 1..=score.len() {
        for midi_index in 1..=midi.len() {
            let prior = cells[(score_index - 1) * width + midi_index - 1].cost;
            let match_cost = candidate_cost(&score[score_index - 1], &midi[midi_index - 1]);
            let mut best = AlignmentCell {
                cost: prior.saturating_add(match_cost),
                action: AlignmentAction::Match,
            };
            let skip_score = cells[(score_index - 1) * width + midi_index]
                .cost
                .saturating_add(SKIP_COST);
            if skip_score < best.cost {
                best = AlignmentCell {
                    cost: skip_score,
                    action: AlignmentAction::SkipScore,
                };
            }
            let skip_midi = cells[score_index * width + midi_index - 1]
                .cost
                .saturating_add(SKIP_COST);
            if skip_midi < best.cost {
                best = AlignmentCell {
                    cost: skip_midi,
                    action: AlignmentAction::SkipMidi,
                };
            }
            cells[score_index * width + midi_index] = best;
        }
    }

    let (mut score_index, mut midi_index) = (score.len(), midi.len());
    while score_index > 0 || midi_index > 0 {
        match cells[score_index * width + midi_index].action {
            AlignmentAction::Match if score_index > 0 && midi_index > 0 => {
                push_match(&score[score_index - 1], &midi[midi_index - 1], result);
                score_index -= 1;
                midi_index -= 1;
            }
            AlignmentAction::SkipScore if score_index > 0 => {
                result
                    .unmatched_score
                    .push(score[score_index - 1].id.clone());
                score_index -= 1;
            }
            AlignmentAction::SkipMidi if midi_index > 0 => {
                result.unmatched_midi.push(midi[midi_index - 1].id);
                midi_index -= 1;
            }
            _ if score_index > 0 => {
                result
                    .unmatched_score
                    .push(score[score_index - 1].id.clone());
                score_index -= 1;
            }
            _ => {
                result.unmatched_midi.push(midi[midi_index - 1].id);
                midi_index -= 1;
            }
        }
    }
}

fn align_pitch_linear(
    score: &[ScoreCandidate],
    midi: &[PerformanceNote],
    result: &mut ScoreMidiAlignment,
) {
    let (mut score_index, mut midi_index) = (0, 0);
    while score_index < score.len() && midi_index < midi.len() {
        let delta =
            absolute_duration_delta(score[score_index].timestamp, midi[midi_index].timestamp);
        if delta <= MAX_ONSET_DELTA {
            push_match(&score[score_index], &midi[midi_index], result);
            score_index += 1;
            midi_index += 1;
        } else if score[score_index].timestamp < midi[midi_index].timestamp {
            result.unmatched_score.push(score[score_index].id.clone());
            score_index += 1;
        } else {
            result.unmatched_midi.push(midi[midi_index].id);
            midi_index += 1;
        }
    }
    result
        .unmatched_score
        .extend(score[score_index..].iter().map(|note| note.id.clone()));
    result
        .unmatched_midi
        .extend(midi[midi_index..].iter().map(|note| note.id));
}

fn candidate_cost(score: &ScoreCandidate, midi: &PerformanceNote) -> u64 {
    let onset = absolute_duration_delta(score.timestamp, midi.timestamp);
    if onset > MAX_ONSET_DELTA {
        return SKIP_COST * 2 + 1;
    }
    let duration =
        absolute_duration_delta(score.duration, midi.duration).min(Duration::from_millis(100));
    micros(onset).saturating_add(micros(duration) / 4)
}

fn push_match(score: &ScoreCandidate, midi: &PerformanceNote, result: &mut ScoreMidiAlignment) {
    let onset_delta = signed_micros_delta(midi.timestamp, score.timestamp);
    let duration_delta = signed_micros_delta(midi.duration, score.duration);
    let absolute_onset = onset_delta.unsigned_abs();
    let mut confidence = match absolute_onset {
        0..=5_000 => 100,
        5_001..=20_000 => 95,
        20_001..=50_000 => 85,
        50_001..=100_000 => 70,
        _ => 50,
    };
    if !score.exact {
        confidence = confidence.min(90);
    }
    confidence = confidence.min(match duration_delta.unsigned_abs() {
        0..=20_000 => 100,
        20_001..=100_000 => 95,
        100_001..=250_000 => 85,
        _ => 70,
    });
    result.matches.push(AlignedNote {
        score_id: score.id.clone(),
        midi_id: midi.id,
        onset_delta_micros: onset_delta,
        duration_delta_micros: duration_delta,
        confidence_percent: confidence,
        exact_score_projection: score.exact,
    });
}

fn midi_pitch(pitch: &Pitch) -> Option<u8> {
    let step = match pitch.step {
        Step::C => 0,
        Step::D => 2,
        Step::E => 4,
        Step::F => 5,
        Step::G => 7,
        Step::A => 9,
        Step::B => 11,
    };
    let value = (i16::from(pitch.octave) + 1) * 12 + step + i16::from(pitch.alter);
    u8::try_from(value).ok().filter(|value| *value <= 127)
}

fn absolute_duration_delta(left: Duration, right: Duration) -> Duration {
    left.max(right).saturating_sub(left.min(right))
}

fn signed_micros_delta(left: Duration, right: Duration) -> i64 {
    if left >= right {
        i64::try_from((left - right).as_micros()).unwrap_or(i64::MAX)
    } else {
        -i64::try_from((right - left).as_micros()).unwrap_or(i64::MAX)
    }
}

fn micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn percent(numerator: usize, denominator: usize) -> u8 {
    if denominator == 0 {
        100
    } else {
        ((numerator.saturating_mul(100) / denominator).min(100)) as u8
    }
}

fn project_time(
    tempo: &TempoTrack,
    time: ScoreTime,
) -> Option<midi_file::tempo_track::TempoProjection> {
    tempo.project_quarter_fraction(time.numerator, time.denominator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::musicxml::{ScoreEventKind, import_musicxml};
    use crate::score_playback::{PlaybackLimits, build_playback_plan};

    #[test]
    fn projects_notes_and_directions_with_duration_and_identity() {
        let score = import_musicxml(
            br#"<score-partwise>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1"><measure number="1"><attributes><divisions>3</divisions></attributes>
<direction><offset>1</offset><direction-type><words>after</words></direction-type></direction>
<note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note>
</measure></part></score-partwise>"#,
        )
        .unwrap();
        let tempo = TempoTrack::build(&[], 480);
        let projection = project_score_timeline(&score, &tempo);
        assert!(projection.unprojected.is_empty());
        assert_eq!(projection.events.len(), 2);
        assert_eq!(projection.events[0].timestamp, Duration::ZERO);
        assert_eq!(projection.events[0].id.kind, ScoreEventKind::Note);
        assert_eq!(
            projection.events[0].duration,
            Some(Duration::from_micros(166_666))
        );
        assert_eq!(
            projection.events[1].timestamp,
            Duration::from_micros(166_666)
        );
        assert_eq!(projection.events[1].id.kind, ScoreEventKind::Direction);
        assert!(
            projection
                .events
                .iter()
                .all(|event| event.exact_to_midi_pulse)
        );
    }

    #[test]
    fn reports_negative_direction_offsets_as_unprojected() {
        let score = import_musicxml(
            br#"<score-partwise>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1"><measure number="1"><direction><offset>-1</offset>
<direction-type><words>before</words></direction-type></direction></measure></part>
</score-partwise>"#,
        )
        .unwrap();
        let projection = project_score_timeline(&score, &TempoTrack::build(&[], 480));
        assert!(projection.events.is_empty());
        assert_eq!(projection.unprojected.len(), 1);
    }

    #[test]
    fn flags_fractions_that_need_midi_pulse_rounding() {
        let score = import_musicxml(
            br#"<score-partwise>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1"><measure number="1"><attributes><divisions>7</divisions></attributes>
<note><rest/><duration>1</duration></note></measure></part></score-partwise>"#,
        )
        .unwrap();
        let projection = project_score_timeline(&score, &TempoTrack::build(&[], 480));
        assert_eq!(projection.events[0].timestamp, Duration::ZERO);
        assert!(!projection.events[0].exact_to_midi_pulse);
    }

    #[test]
    fn projects_repeated_occurrences_on_flattened_midi_time() {
        let score = import_musicxml(
            br#"<score-partwise>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1"><measure number="1">
<barline location="left"><repeat direction="forward"/></barline>
<note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note>
<barline><repeat direction="backward"/></barline>
</measure></part></score-partwise>"#,
        )
        .unwrap();
        let plan = build_playback_plan(&score.parts[0], PlaybackLimits::default());
        let projection =
            project_performed_part_timeline(&score.parts[0], &plan, &TempoTrack::build(&[], 480));

        assert!(projection.unprojected.is_empty());
        assert_eq!(projection.events.len(), 2);
        assert_eq!(
            projection.events[0].id.source_id,
            projection.events[1].id.source_id
        );
        assert_eq!(projection.events[0].id.measure_occurrence_ordinal, 0);
        assert_eq!(projection.events[1].id.measure_occurrence_ordinal, 1);
        assert_eq!(projection.events[0].timestamp, Duration::ZERO);
        assert_eq!(projection.events[1].timestamp, Duration::from_millis(500));
        assert_eq!(
            projection.events[1].duration,
            Some(Duration::from_millis(500))
        );
    }

    #[test]
    fn aligns_both_occurrences_of_a_repeated_source_note() {
        let score = import_musicxml(
            br#"<score-partwise>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1"><measure number="1">
<barline location="left"><repeat direction="forward"/></barline>
<note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note>
<barline><repeat direction="backward"/></barline>
</measure></part></score-partwise>"#,
        )
        .unwrap();
        let performance = [performed(0, 60, 0), performed(1, 60, 500)];
        let alignment = align_score_occurrences(
            &score,
            &TempoTrack::build(&[], 480),
            &performance,
            PlaybackLimits::default(),
        );

        assert_eq!(alignment.matches.len(), 2);
        assert_eq!(
            alignment
                .matches
                .iter()
                .map(|item| item.score_id.measure_occurrence_ordinal)
                .collect::<Vec<_>>(),
            [0, 1]
        );
        assert_eq!(
            alignment.matches[0].score_id.source_id,
            alignment.matches[1].score_id.source_id
        );
        assert!(alignment.navigation_complete);
        assert!(alignment.navigation_diagnostics.is_empty());
    }

    #[test]
    fn diagnoses_conflicting_part_navigation() {
        let score = import_musicxml(
            br#"<score-partwise>
<part-list>
<score-part id="P1"><part-name>Right</part-name></score-part>
<score-part id="P2"><part-name>Left</part-name></score-part>
</part-list>
<part id="P1"><measure number="1">
<note><rest/><duration>1</duration></note>
<barline><repeat direction="backward"/></barline>
</measure></part>
<part id="P2"><measure number="1"><note><rest/><duration>1</duration></note></measure></part>
</score-partwise>"#,
        )
        .unwrap();
        let alignment = align_score_occurrences(
            &score,
            &TempoTrack::build(&[], 480),
            &[],
            PlaybackLimits::default(),
        );

        assert!(!alignment.navigation_complete);
        assert_eq!(alignment.navigation_diagnostics.len(), 1);
        assert!(alignment.navigation_diagnostics[0].contains("part P2"));
    }

    fn three_note_score(pitches: [&str; 3]) -> Score {
        let notes = pitches
            .into_iter()
            .map(|pitch| {
                format!(
                    "<note><pitch><step>{pitch}</step><octave>4</octave></pitch><duration>1</duration></note>"
                )
            })
            .collect::<String>();
        import_musicxml(
            format!(
                r#"<score-partwise>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1"><measure number="1"><attributes><divisions>1</divisions></attributes>
{notes}</measure></part></score-partwise>"#
            )
            .as_bytes(),
        )
        .unwrap()
    }

    fn performed(note_index: usize, pitch: u8, timestamp_millis: u64) -> PerformanceNote {
        PerformanceNote {
            id: MidiNoteId {
                track_id: 2,
                note_index,
            },
            pitch,
            timestamp: Duration::from_millis(timestamp_millis),
            duration: Duration::from_millis(500),
        }
    }

    #[test]
    fn aligns_chronological_pitches_and_reports_extra_midi_notes() {
        let score = three_note_score(["C", "D", "C"]);
        let performance = [
            performed(0, 60, 0),
            performed(1, 65, 250),
            performed(2, 62, 500),
            performed(3, 60, 1_008),
        ];
        let alignment = align_score_notes(&score, &TempoTrack::build(&[], 480), &performance);
        assert_eq!(alignment.matches.len(), 3);
        assert!(alignment.unmatched_score.is_empty());
        assert_eq!(
            alignment.unmatched_midi,
            [MidiNoteId {
                track_id: 2,
                note_index: 1
            }]
        );
        assert_eq!(
            alignment
                .matches
                .iter()
                .map(|item| item.midi_id.note_index)
                .collect::<Vec<_>>(),
            [0, 2, 3]
        );
        assert_eq!(alignment.matches[2].onset_delta_micros, 8_000);
        assert_eq!(alignment.matches[2].confidence_percent, 95);
        assert_eq!(alignment.coverage_percent, 75);
        assert_eq!(alignment.mean_confidence_percent, 98);
    }

    #[test]
    fn repeated_pitch_gap_does_not_shift_the_later_match() {
        let score = three_note_score(["C", "C", "C"]);
        let performance = [performed(0, 60, 0), performed(1, 60, 1_000)];
        let alignment = align_score_notes(&score, &TempoTrack::build(&[], 480), &performance);
        assert_eq!(alignment.matches.len(), 2);
        assert_eq!(
            alignment
                .matches
                .iter()
                .map(|item| item.score_id.source_id.ordinal)
                .collect::<Vec<_>>(),
            [0, 2]
        );
        assert_eq!(alignment.unmatched_score[0].source_id.ordinal, 1);
        assert!(alignment.unmatched_midi.is_empty());
    }

    #[test]
    fn refuses_same_pitch_match_outside_timing_window() {
        let score = three_note_score(["C", "D", "E"]);
        let performance = [performed(0, 60, 400)];
        let alignment = align_score_notes(&score, &TempoTrack::build(&[], 480), &performance);
        assert!(alignment.matches.is_empty());
        assert_eq!(alignment.unmatched_score.len(), 3);
        assert_eq!(alignment.unmatched_midi.len(), 1);
        assert_eq!(alignment.coverage_percent, 0);
    }
}
