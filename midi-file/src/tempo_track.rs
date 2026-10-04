use midly::{MetaMessage, TrackEvent, TrackEventKind};
use std::{collections::HashMap, sync::Arc, time::Duration};

#[derive(Debug, Clone)]
pub struct TempoEvent {
    pub absolute_pulses: u64,
    pub timestamp: Duration,
    /// Tempo in microseconds per quarter note.
    pub tempo: u32,
}

#[derive(Debug, Clone)]
pub struct TempoTrack {
    pulses_per_quarter_note: u16,
    events: Arc<[TempoEvent]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TempoProjection {
    pub timestamp: Duration,
    pub pulses: u64,
    pub exact_to_pulse: bool,
}

impl TempoTrack {
    pub fn events(&self) -> &[TempoEvent] {
        &self.events
    }
    pub fn ppq(&self) -> u16 {
        self.pulses_per_quarter_note
    }
    pub fn seconds_to_pulses(&self, seconds: f64) -> f64 {
        let index = self
            .events
            .partition_point(|e| e.timestamp.as_secs_f64() <= seconds);
        let (start, tick, tempo) = index
            .checked_sub(1)
            .and_then(|i| self.events.get(i))
            .map_or((0., 0., 500_000), |e| {
                (e.timestamp.as_secs_f64(), e.absolute_pulses as f64, e.tempo)
            });
        tick + (seconds - start).max(0.) * 1_000_000. / f64::from(tempo)
            * f64::from(self.pulses_per_quarter_note)
    }
    pub fn bpm_at_seconds(&self, seconds: f64) -> f64 {
        let index = self
            .events
            .partition_point(|e| e.timestamp.as_secs_f64() <= seconds);
        let tempo = index
            .checked_sub(1)
            .and_then(|i| self.events.get(i))
            .map_or(500_000, |e| e.tempo);
        60_000_000. / f64::from(tempo)
    }
    pub fn build(track_events: &[Vec<TrackEvent>], pulses_per_quarter_note: u16) -> TempoTrack {
        // This map will help us get rid of duplicate events if
        // the tempo is specified in every track (as is common).
        let mut tempo_events: HashMap<u64, TempoEvent> = HashMap::new();

        for events in track_events.iter() {
            let mut pulses: u64 = 0;
            for event in events.iter() {
                pulses += event.delta.as_int() as u64;

                if let TrackEventKind::Meta(MetaMessage::Tempo(t)) = &event.kind {
                    tempo_events.insert(
                        pulses,
                        TempoEvent {
                            absolute_pulses: pulses,
                            timestamp: Duration::ZERO,
                            tempo: t.as_int(),
                        },
                    );
                };
            }
        }

        let mut tempo_events: Vec<_> = tempo_events.into_values().collect();
        tempo_events.sort_by_key(|e| e.absolute_pulses);

        let mut previous_absolute_pulses = 0u64;
        let mut running_tempo = 500_000;
        let mut res = Duration::ZERO;

        for tempo_event in tempo_events.iter_mut() {
            let tempo_event_pulses = tempo_event.absolute_pulses;

            let relative_pulses = tempo_event_pulses - previous_absolute_pulses;

            res += pulse_to_duration(relative_pulses, running_tempo, pulses_per_quarter_note);

            tempo_event.timestamp = res;

            running_tempo = tempo_event.tempo;
            previous_absolute_pulses = tempo_event_pulses;
        }

        TempoTrack {
            pulses_per_quarter_note,
            events: tempo_events.into(),
        }
    }

    pub fn tempo_event_for_pulses(&self, pulses: u64) -> Option<&TempoEvent> {
        let res = self
            .events
            .binary_search_by_key(&pulses, |e| e.absolute_pulses);

        let id = match res {
            Ok(id) => Some(id),
            Err(id) => id.checked_sub(1),
        };

        id.and_then(|id| self.events.get(id))
    }

    pub fn pulses_to_duration(&self, event_pulses: u64) -> Duration {
        let tempo_event = self.tempo_event_for_pulses(event_pulses);

        let (res, previous_absolute_pulses, tempo) = if let Some(event) = tempo_event {
            (event.timestamp, event.absolute_pulses, event.tempo)
        } else {
            // 120 BPM
            let default_tempo = 500_000;
            (Duration::ZERO, 0, default_tempo)
        };

        let delta_pulses = event_pulses - previous_absolute_pulses;
        res + pulse_to_duration(delta_pulses, tempo, self.pulses_per_quarter_note)
    }

    /// Projects an exact quarter-note fraction onto this MIDI file's pulse and
    /// tempo timeline. Fractions between pulses are rounded to the nearest
    /// pulse and reported through `exact_to_pulse`.
    pub fn project_quarter_fraction(
        &self,
        numerator: i64,
        denominator: u32,
    ) -> Option<TempoProjection> {
        if numerator < 0 || denominator == 0 {
            return None;
        }
        let scaled = i128::from(numerator) * i128::from(self.pulses_per_quarter_note);
        let denominator = i128::from(denominator);
        let rounded = (scaled + denominator / 2) / denominator;
        let pulses = u64::try_from(rounded).ok()?;
        Some(TempoProjection {
            timestamp: self.pulses_to_duration(pulses),
            pulses,
            exact_to_pulse: scaled % denominator == 0,
        })
    }
}

fn pulse_to_duration(pulses: u64, tempo: u32, pulses_per_quarter_note: u16) -> Duration {
    let u_time = pulses as f64 / pulses_per_quarter_note as f64;
    // We floor only because Synthesia floors,
    // so if we want to test for timing regresions we have to do the same
    let time = (u_time * tempo as f64).floor() as u64;
    Duration::from_micros(time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_exact_and_inexact_quarter_fractions() {
        let tempo = TempoTrack::build(&[], 480);
        let triplet = tempo.project_quarter_fraction(1, 3).unwrap();
        assert_eq!(triplet.pulses, 160);
        assert_eq!(triplet.timestamp, Duration::from_micros(166_666));
        assert!(triplet.exact_to_pulse);

        let septuplet = tempo.project_quarter_fraction(1, 7).unwrap();
        assert_eq!(septuplet.pulses, 69);
        assert!(!septuplet.exact_to_pulse);
        assert_eq!(tempo.project_quarter_fraction(-1, 1), None);
        assert_eq!(tempo.project_quarter_fraction(1, 0), None);
    }

    #[test]
    fn projection_uses_tempo_changes_from_the_paired_midi() {
        let tracks = vec![vec![TrackEvent {
            delta: midly::num::u28::new(480),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(midly::num::u24::new(1_000_000))),
        }]];
        let tempo = TempoTrack::build(&tracks, 480);
        let after_change = tempo.project_quarter_fraction(2, 1).unwrap();
        assert_eq!(after_change.pulses, 960);
        assert_eq!(after_change.timestamp, Duration::from_millis(1_500));
    }
}
