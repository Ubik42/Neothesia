use std::time::Duration;

use midi_file::tempo_track::TempoTrack;

use crate::musicxml::{Score, ScoreEvent, ScoreEventId, ScoreTime};

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
}
