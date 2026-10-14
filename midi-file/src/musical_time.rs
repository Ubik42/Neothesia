//! Meter-aware musical positions shared by native and web practice surfaces.
use crate::tempo_track::TempoTrack;
use midly::{MetaMessage, TrackEvent, TrackEventKind};
use std::{collections::BTreeMap, time::Duration};

#[derive(Clone, Debug)]
pub struct Measure {
    pub number: usize,
    pub display_number: i32,
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
    /// Segments use resulting performance-bar ordinals, never source-file bar numbers.
    pub fn segmented(
        ppq: u16,
        last_tick: u64,
        pickup_ticks: u64,
        segments: &[(usize, u8, u16, Option<i32>)],
    ) -> Result<Self, String> {
        Self::segmented_with_lengths(ppq, last_tick, pickup_ticks, segments, &[])
    }

    /// Length overrides use resulting 1-based measure ordinals. The following
    /// measure starts at the end of the overridden measure.
    pub fn segmented_with_lengths(
        ppq: u16,
        last_tick: u64,
        pickup_ticks: u64,
        segments: &[(usize, u8, u16, Option<i32>)],
        measure_lengths: &[(usize, u64)],
    ) -> Result<Self, String> {
        if segments.is_empty()
            || segments.len() > 1024
            || segments[0].0 != 1
            || segments.windows(2).any(|w| w[0].0 >= w[1].0)
        {
            return Err("拍号变化点必须从第 1 小节开始，并按小节递增".into());
        }
        for &(_, n, d, label) in segments {
            if ppq == 0
                || n == 0
                || n > 32
                || ![1, 2, 4, 8, 16, 32, 64, 128].contains(&d)
                || u64::from(ppq) * 4 % u64::from(d) != 0
                || label.is_some_and(|x| !(-9999..=999999).contains(&x))
            {
                return Err("拍号或书面小节编号无效，当前 MIDI 精度必须能表达每一拍".into());
            }
        }
        if measure_lengths.len() > 1024 {
            return Err("不完整小节数量过多".into());
        }
        let mut lengths = BTreeMap::new();
        for &(measure, ticks) in measure_lengths {
            if measure < 2 || measure > 1_000_000 || ticks == 0 {
                return Err("不完整小节必须从第 2 小节起设置，且长度必须为正整数 MIDI tick".into());
            }
            if lengths.insert(measure, ticks).is_some() {
                return Err("同一小节不能重复指定长度".into());
            }
        }
        let mut measures = Vec::new();
        let mut tick = 0u64;
        let mut segment = 0;
        let mut display = segments[0].3.unwrap_or(1);
        while tick < last_tick.max(1) {
            let number = measures.len() + 1;
            if number > 1_000_000 {
                return Err("小节数量过大".into());
            }
            if segment + 1 < segments.len() && segments[segment + 1].0 == number {
                segment += 1;
                if let Some(label) = segments[segment].3 {
                    display = label;
                }
            }
            let (_, n, d, _) = segments[segment];
            let length = u64::from(ppq) * 4 * u64::from(n) / u64::from(d);
            if number == 1 && pickup_ticks >= length {
                return Err("弱起必须短于第一个完整小节".into());
            }
            let size = if number == 1 && pickup_ticks > 0 {
                pickup_ticks
            } else if let Some(&short) = lengths.get(&number) {
                if short >= length {
                    return Err("不完整小节长度必须短于该处拍号规定的完整小节".into());
                }
                short
            } else {
                length
            };
            let end = tick.checked_add(size).ok_or("小节时间超出范围")?;
            measures.push(Measure {
                number,
                display_number: display,
                start_tick: tick,
                end_tick: end,
                numerator: n,
                denominator: d,
                explicit_meter: true,
                partial: size < length,
            });
            display = display.checked_add(1).ok_or("书面小节编号超出范围")?;
            tick = end;
        }
        if segment + 1 != segments.len() {
            return Err("拍号变化点超出曲目结尾，请先调整前面的拍号".into());
        }
        if lengths.last_key_value().is_some_and(|(&measure, _)| measure > measures.len()) {
            return Err("不完整小节位置超出曲目结尾，请先调整前面的小节".into());
        }
        Ok(Self { ppq, measures })
    }
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
                display_number: (measures.len() + 1) as i32,
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
                    display_number: (measures.len() + 1) as i32,
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
    #[test]
    fn fourth_measure_shortens_and_fifth_resumes_normal_meter() {
        let grid = MusicalTime::segmented_with_lengths(
            480, 9000, 0,
            &[(1, 4, 4, Some(1)), (6, 3, 4, Some(10))],
            &[(4, 480)],
        ).unwrap();
        assert_eq!(grid.measures.iter().take(7).map(|m| (m.start_tick, m.end_tick)).collect::<Vec<_>>(),
            vec![(0, 1920), (1920, 3840), (3840, 5760), (5760, 6240),
                 (6240, 8160), (8160, 9600)]);
        assert!(grid.measures[3].partial);
        assert!(!grid.measures[4].partial);
        assert_eq!(grid.measures[5].display_number, 10);
        assert_eq!(grid.measures[5].numerator, 3);
        assert_eq!(grid.measure_at(6239.).unwrap().number, 4);
        assert_eq!(grid.measure_at(6240.).unwrap().number, 5);
        assert_eq!(grid.measure_at(8160.).unwrap().number, 6);
        assert_eq!(grid.beat_at(6240.), 1.);
        assert_eq!(grid.beat_at(6720.), 2.);
    }
    #[test]
    fn length_overrides_reject_conflicts_and_out_of_range_measures() {
        let segments = [(1, 4, 4, None)];
        let build = |lengths: &[(usize, u64)]| MusicalTime::segmented_with_lengths(
            480, 7000, 240, &segments, lengths,
        );
        assert!(build(&[(1, 120)]).is_err());
        assert!(build(&[(2, 0)]).is_err());
        assert!(build(&[(2, 1920)]).is_err());
        assert!(build(&[(2, 1921)]).is_err());
        assert!(build(&[(2, 480), (2, 240)]).is_err());
        assert!(build(&[(99, 480)]).is_err());
        let grid = build(&[(4, 480)]).unwrap();
        assert_eq!(grid.measures[0].end_tick, 240);
        assert_eq!(grid.measures[3].end_tick, 4560);
        assert_eq!(grid.measures[4].start_tick, 4560);
    }
}
