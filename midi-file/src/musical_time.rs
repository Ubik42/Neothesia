//! Meter-aware musical positions shared by native and web practice surfaces.
use crate::tempo_track::TempoTrack;
use midly::{MetaMessage, TrackEvent, TrackEventKind};
use std::{collections::BTreeMap, time::Duration};

#[derive(Clone, Debug)]
pub struct Measure {
    pub number: usize,
    pub start_tick: u64,
    pub end_tick: u64,
    pub numerator: u8,
    pub denominator: u16,
    pub explicit_meter: bool,
    pub partial: bool,
}
#[derive(Clone, Debug)]
pub struct MusicalTime {
    pub ppq: u16,
    pub measures: Vec<Measure>,
}
impl MusicalTime {
    pub fn corrected(
        ppq: u16,
        last_tick: u64,
        numerator: u8,
        denominator: u16,
        pickup_ticks: u64,
    ) -> Result<Self, String> {
        if numerator == 0 || numerator > 32 || ![1, 2, 4, 8, 16, 32, 64, 128].contains(&denominator)
        {
            return Err("拍号无效".into());
        }
        let length = u64::from(ppq) * 4 * u64::from(numerator) / u64::from(denominator);
        if length == 0 || pickup_ticks >= length {
            return Err("弱起必须短于一个完整小节".into());
        }
        let mut measures = vec![];
        let mut tick = 0;
        while tick < last_tick.max(1) {
            if measures.len() >= 1_000_000 {
                return Err("小节数量过大".into());
            }
            let size = if tick == 0 && pickup_ticks > 0 {
                pickup_ticks
            } else {
                length
            };
            let end = tick.checked_add(size).ok_or("小节时间超出范围")?;
            measures.push(Measure {
                number: measures.len() + 1,
                start_tick: tick,
                end_tick: end,
                numerator,
                denominator,
                explicit_meter: true,
                partial: size < length,
            });
            tick = end;
        }
        Ok(Self { ppq, measures })
    }
    pub fn build(tracks: &[Vec<TrackEvent>], ppq: u16, last_tick: u64) -> Result<Self, String> {
        let mut signatures = BTreeMap::new();
        signatures.insert(0, (4, 4u16, false));
        for track in tracks {
            let mut tick = 0u64;
            for event in track {
                tick = tick
                    .checked_add(u64::from(event.delta.as_int()))
                    .ok_or("MIDI 时间超出范围")?;
                if let TrackEventKind::Meta(MetaMessage::TimeSignature(n, power, _, _)) = event.kind
                {
                    if n == 0 || power > 7 {
                        return Err("MIDI 拍号无效或不受支持".into());
                    }
                    let value = (n, 1u16 << power, true);
                    if signatures
                        .get(&tick)
                        .is_some_and(|previous| previous.2 && *previous != value)
                    {
                        return Err("不同音轨在同一位置给出冲突拍号".into());
                    }
                    signatures.insert(tick, value);
                }
            }
        }
        let signatures: Vec<_> = signatures.into_iter().collect();
        let mut measures = Vec::new();
        for (index, &(start, (n, d, explicit))) in signatures.iter().enumerate() {
            if start >= last_tick.max(1) {
                break;
            }
            let limit = signatures.get(index + 1).map(|s| s.0).unwrap_or(u64::MAX);
            let length = u64::from(ppq) * 4 * u64::from(n) / u64::from(d);
            if length == 0 {
                return Err("MIDI 分辨率不能表达该拍号".into());
            }
            let mut tick = start;
            while tick < last_tick.max(1) && tick < limit {
                if measures.len() >= 1_000_000 {
                    return Err("MIDI 小节数量过大".into());
                }
                let end = tick.saturating_add(length).min(limit);
                measures.push(Measure {
                    number: measures.len() + 1,
                    start_tick: tick,
                    end_tick: end,
                    numerator: n,
                    denominator: d,
                    explicit_meter: explicit,
                    partial: end - tick < length,
                });
                tick = end;
            }
        }
        Ok(Self { ppq, measures })
    }
    pub fn measure_at(&self, tick: f64) -> Option<&Measure> {
        let index = self
            .measures
            .partition_point(|m| m.start_tick as f64 <= tick)
            .saturating_sub(1);
        self.measures.get(index)
    }
    pub fn beat_at(&self, tick: f64) -> f64 {
        self.measure_at(tick).map_or(1., |m| {
            1. + (tick - m.start_tick as f64)
                / (f64::from(self.ppq) * 4. / f64::from(m.denominator))
        })
    }
    pub fn bar_times(&self, tempo: &TempoTrack) -> Vec<Duration> {
        self.measures
            .iter()
            .map(|m| tempo.pulses_to_duration(m.start_tick))
            .collect()
    }
    pub fn beat_times(&self, tempo: &TempoTrack) -> Vec<Duration> {
        self.measures
            .iter()
            .flat_map(|m| {
                (0..m.numerator)
                    .map(move |b| {
                        m.start_tick
                            + u64::from(b) * u64::from(self.ppq) * 4 / u64::from(m.denominator)
                    })
                    .take_while(move |tick| *tick < m.end_tick)
            })
            .map(|tick| tempo.pulses_to_duration(tick))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn meter(delta: u32, n: u8, d: u8) -> TrackEvent<'static> {
        TrackEvent {
            delta: delta.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(n, d, 24, 8)),
        }
    }
    #[test]
    fn meter_changes_and_compound_beats() {
        let grid =
            MusicalTime::build(&[vec![meter(0, 3, 2), meter(2880, 6, 3)]], 480, 4320).unwrap();
        assert_eq!(
            grid.measures
                .iter()
                .map(|m| m.start_tick)
                .collect::<Vec<_>>(),
            vec![0, 1440, 2880]
        );
        assert_eq!(grid.measure_at(3000.).unwrap().denominator, 8);
        assert_eq!(grid.beat_at(3120.), 2.);
    }
    #[test]
    fn missing_meter_is_marked_and_mid_bar_change_is_partial() {
        let grid = MusicalTime::build(&[vec![meter(720, 3, 2)]], 480, 2000).unwrap();
        assert!(!grid.measures[0].explicit_meter);
        assert!(grid.measures[0].partial);
        assert_eq!(grid.measures[1].start_tick, 720);
    }
}
