//! Convert written notation to an owned, meter-aware practice performance.
use crate::{
    musicxml::*,
    practice::PracticePart,
    score_playback::{PlaybackLimits, build_playback_plan},
};
use midi_file::midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
    num::{u4, u7, u15, u24, u28},
};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone)]
pub struct PerformedNote {
    pub start: u64,
    pub end: u64,
    pub pitch: u8,
    pub velocity: u8,
    pub finger: Option<u8>,
}
pub struct StaffTrack {
    pub part: PracticePart,
    pub notes: Vec<PerformedNote>,
    pub pedals: Vec<(u64, u8)>,
}
pub struct Performance {
    pub smf: Smf<'static>,
    pub tracks: Vec<StaffTrack>,
    pub warnings: Vec<String>,
}
fn quarters(t: ScoreTime) -> f64 {
    t.numerator as f64 / f64::from(t.denominator)
}
fn pitch(p: &Pitch) -> Result<u8, String> {
    let step = match p.step {
        Step::C => 0,
        Step::D => 2,
        Step::E => 4,
        Step::F => 5,
        Step::G => 7,
        Step::A => 9,
        Step::B => 11,
    };
    let n = (i16::from(p.octave) + 1) * 12 + step + i16::from(p.alter);
    u8::try_from(n)
        .ok()
        .filter(|n| *n <= 127)
        .ok_or("谱面音高超出 MIDI 范围".into())
}
fn meter(t: &TimeSignature) -> Result<(u8, u16), String> {
    let n = t
        .beats
        .split('+')
        .try_fold(0u16, |total, v| {
            v.trim()
                .parse::<u16>()
                .ok()
                .and_then(|v| total.checked_add(v))
        })
        .ok_or("拍号无法识别")?;
    let d = t.beat_type.parse::<u16>().map_err(|_| "拍号无法识别")?;
    if n == 0 || n > 32 || ![1, 2, 4, 8, 16, 32, 64, 128].contains(&d) {
        return Err("拍号超出当前支持范围".into());
    }
    Ok((n as u8, d))
}

/// Align measure extents across parts and fill regular bars to their declared
/// meter. An explicitly implicit pickup keeps its shorter extent.
pub fn normalized(score: &Score) -> Result<Score, String> {
    if score.parts.is_empty() {
        return Err("谱面没有声部".into());
    }
    let mut out = score.clone();
    let count = out
        .parts
        .iter()
        .map(|p| p.measures.len())
        .max()
        .unwrap_or(0);
    if count > 20_000 {
        return Err("谱面小节过多".into());
    }
    let mut position = ScoreTime::default();
    let mut signature = (4, 4u16);
    for index in 0..count {
        for part in &out.parts {
            if let Some(m) = part.measures.get(index) {
                for a in &m.attributes {
                    if quarters(a.onset.subtract(m.start)).abs() < 0.00001 {
                        if let Some(t) = &a.time_signature {
                            signature = meter(t)?;
                        }
                    }
                }
            }
        }
        let extent = out
            .parts
            .iter()
            .filter_map(|p| p.measures.get(index))
            .map(|m| m.duration)
            .max()
            .unwrap_or_default();
        let implicit = out
            .parts
            .iter()
            .filter_map(|p| p.measures.get(index))
            .any(|m| m.implicit);
        let duration = if implicit {
            extent
        } else {
            extent.max(ScoreTime::new(
                i64::from(signature.0) * 4,
                u32::from(signature.1),
            ))
        };
        if duration.numerator <= 0 {
            return Err(format!("第 {} 小节没有有效时长", index + 1));
        }
        for part in &mut out.parts {
            if let Some(m) = part.measures.get_mut(index) {
                let offset = position.subtract(m.start);
                for a in &mut m.attributes {
                    a.onset = a.onset.add(offset);
                }
                for event in &mut m.events {
                    match event {
                        ScoreEvent::Note(n) => n.onset = n.onset.add(offset),
                        ScoreEvent::Direction(d) => {
                            d.onset = d.onset.add(offset);
                            for n in &mut d.navigation {
                                n.onset = n.onset.add(offset);
                            }
                        }
                    }
                }
                m.start = position;
                m.duration = duration;
            }
        }
        position = position.add(duration);
    }
    let source = out
        .parts
        .iter()
        .max_by_key(|p| {
            p.measures
                .iter()
                .flat_map(|m| &m.barlines)
                .filter(|b| b.repeat.is_some() || !b.endings.is_empty())
                .count()
        })
        .unwrap()
        .clone();
    for p in &mut out.parts {
        if !p
            .measures
            .iter()
            .flat_map(|m| &m.barlines)
            .any(|b| b.repeat.is_some() || !b.endings.is_empty())
        {
            for (i, m) in p.measures.iter_mut().enumerate() {
                if let Some(master) = source.measures.get(i) {
                    m.barlines.extend(
                        master
                            .barlines
                            .iter()
                            .filter(|b| b.repeat.is_some() || !b.endings.is_empty())
                            .cloned(),
                    );
                }
            }
        }
    }
    // Navigation is commonly written only above the first part. Merge exact
    // sound instructions by written position and give every part the same route.
    // Conflicting labels/commands remain present for the planner to diagnose.
    for index in 0..source.measures.len() {
        let mut navigation = vec![];
        for p in &out.parts {
            if let Some(m) = p.measures.get(index) {
                for e in &m.events {
                    if let ScoreEvent::Direction(d) = e {
                        for n in &d.navigation {
                            if !navigation.contains(n) {
                                navigation.push(n.clone());
                            }
                        }
                    }
                }
            }
        }
        for p in &mut out.parts {
            if let Some(m) = p.measures.get_mut(index) {
                let missing: Vec<_> = navigation
                    .iter()
                    .filter(|n| {
                        !m.events.iter().any(
                            |e| matches!(e, ScoreEvent::Direction(d) if d.navigation.contains(n)),
                        )
                    })
                    .cloned()
                    .collect();
                if !missing.is_empty() {
                    let ordinal = m
                        .events
                        .iter()
                        .filter_map(|e| {
                            if let ScoreEvent::Direction(d) = e {
                                Some(d.id.ordinal)
                            } else {
                                None
                            }
                        })
                        .max()
                        .map_or(0, |o| o.saturating_add(1));
                    m.events.push(ScoreEvent::Direction(Direction {
                        id: ScoreEventId {
                            part_id: p.id.clone(),
                            measure_ordinal: index as u32,
                            kind: ScoreEventKind::Direction,
                            ordinal,
                        },
                        onset: m.start,
                        staff: None,
                        dynamics: vec![],
                        words: vec![],
                        tempo_bpm: None,
                        navigation: missing,
                        pedals: vec![],
                    }));
                }
            }
        }
    }
    Ok(out)
}

pub fn generate(source: &Score, default_bpm: u16) -> Result<Performance, String> {
    if !(20..=400).contains(&default_bpm) {
        return Err("默认速度须为 20 至 400 BPM".into());
    }
    if source
        .warnings
        .iter()
        .any(|w| w.message.contains("transposition"))
    {
        return Err("谱面含未支持的移调，请提供实际音高谱面".into());
    }
    let score = normalized(source)?;
    let master = score
        .parts
        .iter()
        .max_by_key(|p| {
            p.measures
                .iter()
                .flat_map(|m| &m.barlines)
                .filter(|b| b.repeat.is_some() || !b.endings.is_empty())
                .count()
        })
        .unwrap();
    let plan = build_playback_plan(
        master,
        PlaybackLimits {
            max_visits: 20_000,
            max_repeat_passes: 16,
        },
    );
    if !plan.complete {
        return Err(format!(
            "谱面演奏顺序无法可靠展开：{}",
            plan.diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
                .join("；")
        ));
    }
    for part in &score.parts {
        if part.measures.len() != master.measures.len() {
            return Err("各声部的小节数量不一致，请先校对谱面".into());
        }
        {
            let own = build_playback_plan(
                part,
                PlaybackLimits {
                    max_visits: 20_000,
                    max_repeat_passes: 16,
                },
            );
            if !own.complete
                || own
                    .visits
                    .iter()
                    .map(|v| (v.source_measure_ordinal, v.source_start, v.source_end))
                    .collect::<Vec<_>>()
                    != plan
                        .visits
                        .iter()
                        .map(|v| (v.source_measure_ordinal, v.source_start, v.source_end))
                        .collect::<Vec<_>>()
            {
                return Err("各声部的演奏顺序不一致，请先校对谱面".into());
            }
        }
    }
    let mut ppq = 960u64;
    let gcd = |mut a: u64, mut b: u64| {
        while b > 0 {
            let r = a % b;
            a = b;
            b = r;
        }
        a
    };
    let mut include = |t: ScoreTime| -> Result<(), String> {
        let d = u64::from(t.denominator);
        ppq = ppq
            .checked_div(gcd(ppq, d))
            .and_then(|v| v.checked_mul(d))
            .ok_or("乐谱时间精度超出范围")?;
        if ppq > 32767 {
            return Err("谱面连音精度超出 MIDI 分辨率，请调整连音".into());
        }
        Ok(())
    };
    for p in &score.parts {
        for m in &p.measures {
            include(m.start)?;
            include(m.duration)?;
            for e in &m.events {
                match e {
                    ScoreEvent::Note(n) => {
                        include(n.onset)?;
                        include(n.duration)?;
                    }
                    ScoreEvent::Direction(d) => {
                        include(d.onset)?;
                        for n in &d.navigation {
                            include(n.onset)?;
                        }
                    }
                }
            }
        }
    }
    let tick = |t: ScoreTime| -> Result<u64, String> {
        if t.numerator < 0 {
            return Err("谱面存在负时间音符或标记".into());
        }
        let v = (t.numerator as u128) * (ppq as u128) / u128::from(t.denominator);
        if v > 0x0fff_ffff {
            return Err("谱面演奏时间过长".into());
        }
        Ok(v as u64)
    };
    let mut warnings: Vec<_> = score.warnings.iter().map(|w| w.message.clone()).collect();
    if !score
        .parts
        .iter()
        .flat_map(|p| &p.measures)
        .flat_map(|m| &m.attributes)
        .any(|a| a.time_signature.is_some())
    {
        warnings.push("原谱缺少拍号，当前按 4/4 生成，请在小节网格中校对".into());
    }
    if score.parts.iter().any(|p| {
        let staffs: std::collections::BTreeSet<_> = p
            .measures
            .iter()
            .flat_map(|m| &m.events)
            .filter_map(|e| {
                if let ScoreEvent::Note(n) = e {
                    Some(n.staff)
                } else {
                    None
                }
            })
            .collect();
        staffs.contains(&1) && staffs.contains(&2)
    }) {
        warnings.push("双谱表暂按上谱表右手、下谱表左手设置，交叉声部请逐音校对".into());
    }
    let mut tempos = BTreeMap::from([(0, 60_000_000 / u32::from(default_bpm))]);
    let mut meters = BTreeMap::new();
    let mut written_tempos = BTreeMap::new();
    for p in &score.parts {
        for m in &p.measures {
            for e in &m.events {
                if let ScoreEvent::Direction(d) = e {
                    if let Some(value) = &d.tempo_bpm {
                        let bpm = value.parse::<f64>().map_err(|_| "谱面速度标记无效")?;
                        if !bpm.is_finite() || !(4.0..=1000.0).contains(&bpm) {
                            return Err("谱面速度超出范围".into());
                        }
                        let value = (60_000_000.0 / bpm).round() as u32;
                        let at = tick(d.onset)?;
                        if written_tempos
                            .insert(at, value)
                            .is_some_and(|old| old != value)
                        {
                            return Err("各声部给出冲突的速度标记".into());
                        }
                    }
                }
            }
        }
    }
    let mut inherited = Vec::new();
    let mut current = (4, 4u16);
    for index in 0..master.measures.len() {
        for p in &score.parts {
            for a in &p.measures[index].attributes {
                if a.onset == p.measures[index].start {
                    if let Some(t) = &a.time_signature {
                        current = meter(t)?;
                    }
                }
            }
        }
        inherited.push(current);
        for p in &score.parts {
            for a in &p.measures[index].attributes {
                if let Some(t) = &a.time_signature {
                    current = meter(t)?;
                }
            }
        }
    }
    for visit in &plan.visits {
        let m = &master.measures[visit.source_measure_ordinal as usize];
        let at = tick(visit.performed_start)?;
        let source = tick(m.start.add(visit.source_start))?;
        tempos.insert(
            at,
            written_tempos
                .range(..=source)
                .next_back()
                .map_or(60_000_000 / u32::from(default_bpm), |(_, v)| *v),
        );
        for (&written, &value) in written_tempos.range(source..tick(m.start.add(visit.source_end))?)
        {
            tempos.insert(at + written - source, value);
        }
    }
    let mut tracks = vec![];
    let mut total = 0;
    let mut generated_count = 0;
    for part in &score.parts {
        let staffs: std::collections::BTreeSet<_> = part
            .measures
            .iter()
            .flat_map(|m| &m.events)
            .filter_map(|e| {
                if let ScoreEvent::Note(n) = e {
                    n.pitch.as_ref().map(|_| n.staff)
                } else {
                    None
                }
            })
            .collect();
        for staff in staffs.iter().copied() {
            if tracks.len() >= 15 {
                return Err("目前最多支持 15 条可演奏谱表".into());
            }
            let role = if staffs.len() == 2 && staffs.contains(&1) && staffs.contains(&2) {
                if staff == 1 {
                    PracticePart::RightHand
                } else {
                    PracticePart::LeftHand
                }
            } else {
                PracticePart::Other
            };
            let mut notes: Vec<PerformedNote> = vec![];
            let mut ties: HashMap<(u8, Option<String>), usize> = HashMap::new();
            let mut pedals = vec![];
            let dynamic = |marks: &Vec<String>, mut velocity: u8| {
                for mark in marks {
                    velocity = match mark.as_str() {
                        "ppp" => 35,
                        "pp" => 45,
                        "p" => 55,
                        "mp" => 65,
                        "mf" => 80,
                        "f" => 95,
                        "ff" => 110,
                        "fff" => 120,
                        _ => velocity,
                    };
                }
                velocity
            };
            let mut bases = vec![];
            let mut previous_velocity = 80;
            let mut previous_pedal = 0;
            for m in &part.measures {
                bases.push((previous_velocity, previous_pedal));
                let mut dirs: Vec<_> = m
                    .events
                    .iter()
                    .filter_map(|e| {
                        if let ScoreEvent::Direction(d) = e {
                            Some(d)
                        } else {
                            None
                        }
                    })
                    .collect();
                dirs.sort_by_key(|d| d.onset);
                for d in dirs {
                    if d.staff.is_none_or(|s| s == staff) {
                        previous_velocity = dynamic(&d.dynamics, previous_velocity);
                        for p in &d.pedals {
                            previous_pedal = match p.kind {
                                PedalType::Start | PedalType::Resume | PedalType::Change => 127,
                                PedalType::Stop | PedalType::Discontinue => 0,
                                _ => previous_pedal,
                            };
                        }
                    }
                }
            }
            let mut previous_end = None;
            for visit in &plan.visits {
                let index = visit.source_measure_ordinal as usize;
                let m = &part.measures[index];
                let start = tick(visit.performed_start)?;
                let extent = tick(visit.source_end.subtract(visit.source_start))?;
                let source = m.start.add(visit.source_start);
                let source_end = m.start.add(visit.source_end);
                total = total.max(start + extent);
                let mut velocity = bases[index].0;
                if previous_end != Some(source) {
                    ties.clear();
                    pedals.push((start, bases[index].1));
                }
                previous_end = Some(source_end);
                // A meter event at each written/performed boundary preserves pickups.
                meters.insert(start, inherited[index]);
                for a in &m.attributes {
                    if let Some(t) = &a.time_signature {
                        if a.onset <= source {
                            meters.insert(start, meter(t)?);
                        } else if a.onset < source_end {
                            meters.insert(start + tick(a.onset.subtract(source))?, meter(t)?);
                        }
                    }
                }
                let mut events: Vec<_> = m.events.iter().collect();
                events.sort_by_key(|e| match e {
                    ScoreEvent::Note(n) => (n.onset, 1),
                    ScoreEvent::Direction(d) => (d.onset, 0),
                });
                for e in events {
                    match e {
                        ScoreEvent::Direction(d) => {
                            if d.onset > source_end {
                                continue;
                            }
                            if d.onset < source {
                                if d.staff.is_none_or(|s| s == staff) {
                                    velocity = dynamic(&d.dynamics, velocity);
                                    for p in &d.pedals {
                                        match p.kind {
                                            PedalType::Start
                                            | PedalType::Resume
                                            | PedalType::Change => {
                                                pedals.push((start, 127));
                                            }
                                            PedalType::Stop | PedalType::Discontinue => {
                                                pedals.push((start, 0));
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                continue;
                            }
                            let at = start + tick(d.onset.subtract(source))?;
                            if d.staff.is_none_or(|s| s == staff) {
                                velocity = dynamic(&d.dynamics, velocity);
                                for p in &d.pedals {
                                    match p.kind {
                                        PedalType::Start | PedalType::Resume => {
                                            pedals.push((at, 127))
                                        }
                                        PedalType::Stop | PedalType::Discontinue => {
                                            pedals.push((at, 0))
                                        }
                                        PedalType::Change => {
                                            pedals.push((at, 0));
                                            pedals.push((at, 127));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        ScoreEvent::Note(n) => {
                            if n.staff != staff
                                || n.onset >= source_end
                                || n.onset.add(n.duration) <= source
                            {
                                continue;
                            }
                            let Some(written) = &n.pitch else {
                                continue;
                            };
                            if n.grace || n.duration.numerator <= 0 {
                                if !warnings
                                    .iter()
                                    .any(|w| w == "装饰音保留在原谱中，当前练习演奏跳过其自由时值")
                                {
                                    warnings.push(
                                        "装饰音保留在原谱中，当前练习演奏跳过其自由时值".into(),
                                    );
                                }
                                continue;
                            }
                            generated_count += 1;
                            if generated_count > 200_000 {
                                return Err("演奏音符超过 20 万，请分段导入".into());
                            }
                            let pitch = pitch(written)?;
                            let onset = start + tick(n.onset.max(source).subtract(source))?;
                            let end = start
                                + tick(n.onset.add(n.duration).min(source_end).subtract(source))?;
                            let key = (pitch, n.voice.clone());
                            let stop = n.ties.contains(&SpanType::Stop);
                            let begin = n.ties.contains(&SpanType::Start);
                            let tied = if stop {
                                ties.remove(&key).filter(|&i| notes[i].end == onset)
                            } else {
                                None
                            };
                            let i = if let Some(i) = tied {
                                notes[i].end = end;
                                i
                            } else {
                                let i = notes.len();
                                notes.push(PerformedNote {
                                    start: onset,
                                    end,
                                    pitch,
                                    velocity,
                                    finger: n
                                        .fingering
                                        .as_ref()
                                        .and_then(|s| s.trim().parse::<u8>().ok())
                                        .filter(|f| (1..=5).contains(f)),
                                });
                                i
                            };
                            if begin {
                                ties.insert(key, i);
                            }
                        }
                    }
                }
            }
            notes.sort_by_key(|n| (n.start, n.pitch, n.end));
            // MIDI has one key state per pitch/channel; overlapping unisons in
            // separate voices are a shared sustained key rather than two presses.
            let mut merged: Vec<PerformedNote> = vec![];
            let mut last = HashMap::new();
            for n in notes {
                if let Some(&i) = last.get(&n.pitch) {
                    let old: &mut PerformedNote = &mut merged[i];
                    if n.start < old.end {
                        old.end = old.end.max(n.end);
                        continue;
                    }
                }
                last.insert(n.pitch, merged.len());
                merged.push(n);
            }
            if !merged.is_empty() {
                tracks.push(StaffTrack {
                    part: role,
                    notes: merged,
                    pedals,
                });
            }
        }
    }
    if tracks.is_empty() {
        return Err("谱面中没有可练习的有时值音符".into());
    }
    let mut conductor: Vec<(u64, u8, TrackEventKind<'static>)> = vec![];
    for (at, bpm) in tempos {
        conductor.push((
            at,
            0,
            TrackEventKind::Meta(MetaMessage::Tempo(u24::new(bpm))),
        ));
    }
    for (at, (n, d)) in meters {
        conductor.push((
            at,
            1,
            TrackEventKind::Meta(MetaMessage::TimeSignature(
                n,
                d.trailing_zeros() as u8,
                24,
                8,
            )),
        ));
    }
    fn delta_track(
        mut events: Vec<(u64, u8, TrackEventKind<'static>)>,
        end: u64,
    ) -> Result<Vec<TrackEvent<'static>>, String> {
        events.sort_by_key(|e| (e.0, e.1));
        let mut previous = 0;
        let mut out = vec![];
        for (t, _, kind) in events {
            let delta = u32::try_from(t - previous).map_err(|_| "MIDI 时间过大")?;
            if delta > 0x0fff_ffff {
                return Err("MIDI 时间过大".into());
            }
            out.push(TrackEvent {
                delta: u28::new(delta),
                kind,
            });
            previous = t;
        }
        out.push(TrackEvent {
            delta: u28::new(end.saturating_sub(previous) as u32),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        Ok(out)
    }
    let mut midi_tracks = vec![delta_track(conductor, total)?];
    for (i, track) in tracks.iter().enumerate() {
        let channel = u4::new(if i >= 9 { i as u8 + 1 } else { i as u8 });
        let mut events = vec![(
            0,
            0,
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::ProgramChange {
                    program: u7::new(0),
                },
            },
        )];
        for &(at, value) in &track.pedals {
            events.push((
                at,
                0,
                TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::Controller {
                        controller: u7::new(64),
                        value: u7::new(value),
                    },
                },
            ));
        }
        for n in &track.notes {
            events.push((
                n.start,
                2,
                TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::NoteOn {
                        key: u7::new(n.pitch),
                        vel: u7::new(n.velocity),
                    },
                },
            ));
            events.push((
                n.end,
                1,
                TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::NoteOff {
                        key: u7::new(n.pitch),
                        vel: u7::new(0),
                    },
                },
            ));
        }
        midi_tracks.push(delta_track(events, total)?);
    }
    Ok(Performance {
        smf: Smf {
            header: Header {
                format: Format::Parallel,
                timing: Timing::Metrical(u15::new(ppq as u16)),
            },
            tracks: midi_tracks,
        },
        tracks,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pickup_repeat_ties_tempo_staffs_and_pedals_have_performed_semantics() {
        let score =
            import_musicxml_document(include_bytes!("score_performance_test.musicxml")).unwrap();
        let p = generate(&score, 100).unwrap();
        let file = midi_file::MidiFile::from_smf("test", &p.smf).unwrap();
        assert_eq!(p.tracks.len(), 2);
        assert_eq!(p.tracks[0].part, PracticePart::RightHand);
        assert_eq!(p.tracks[1].part, PracticePart::LeftHand);
        assert_eq!(p.tracks[0].notes.len(), 2);
        assert_eq!(p.tracks[1].notes.len(), 4);
        assert_eq!(p.tracks[0].notes[0].finger, Some(3));
        assert_eq!(p.tracks[0].notes[0].end, 2880);
        assert_eq!(p.tracks[0].notes[1].start, 3840);
        assert!((file.duration.as_secs_f64() - 5.0).abs() < 0.00001);
        assert!((file.tracks[1].notes[0].duration.as_secs_f64() - 2.0).abs() < 0.00001);
        assert!(file.musical_time.measures[0].partial);
        assert!(file.musical_time.measures[2].partial);
        assert_eq!(file.musical_time.measures[1].numerator, 3);
        assert!(p.tracks[0].pedals.contains(&(0, 127)));
        assert!(p.tracks[0].pedals.contains(&(960, 0)));
        assert_eq!(p.tracks[1].notes[2].velocity, 80);
    }
    #[test]
    fn unsupported_jump_is_rejected_instead_of_playing_wrong_order() {
        let xml = String::from_utf8(include_bytes!("score_performance_test.musicxml").to_vec())
            .unwrap()
            .replace(
                "<sound tempo=\"60\"/>",
                "<sound tempo=\"60\" dacapo=\"yes\"/>",
            );
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        assert!(generate(&score, 120).err().unwrap().contains("D.C."));
    }
    #[test]
    fn partial_bar_return_clips_sound_and_alignment_and_propagates_to_other_parts() {
        let xml = r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part><score-part id="P2"><part-name>Lower</part-name></score-part></part-list>
        <part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>2</beats><beat-type>4</beat-type></time></attributes>
        <direction><sound tempo="60"/></direction><note><pitch><step>C</step><octave>4</octave></pitch><duration>2</duration></note>
        <direction><offset>-2</offset><sound segno="S"><offset>-1</offset></sound></direction><sound fine="yes"/></measure>
        <measure number="2"><direction><sound tempo="120"/></direction><note><pitch><step>E</step><octave>4</octave></pitch><duration>2</duration></note><sound dalsegno="S"/></measure></part>
        <part id="P2"><measure number="1"><attributes><divisions>1</divisions><time><beats>2</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>3</octave></pitch><duration>2</duration></note></measure>
        <measure number="2"><note><pitch><step>E</step><octave>3</octave></pitch><duration>2</duration></note></measure></part></score-partwise>"#;
        let shifted_tempo = xml.replace(
            r#"<direction><sound tempo="60"/></direction>"#,
            r#"<direction><sound tempo="60"><offset>1</offset></sound></direction>"#,
        );
        let shifted = generate(
            &import_musicxml_document(shifted_tempo.as_bytes()).unwrap(),
            120,
        )
        .unwrap();
        let shifted_file = midi_file::MidiFile::from_smf("shifted", &shifted.smf).unwrap();
        assert_eq!(
            shifted_file
                .tempo_track
                .pulses_to_duration(4800)
                .as_secs_f64(),
            3.5
        );
        let mid_fine = xml
            .replace(
                r#"<sound segno="S"><offset>-1</offset></sound>"#,
                r#"<sound segno="S"><offset>-2</offset></sound>"#,
            )
            .replace(
                r#"<sound fine="yes"/>"#,
                r#"<direction><offset>-1</offset><sound fine="yes"/></direction>"#,
            );
        let cut = import_musicxml_document(mid_fine.as_bytes()).unwrap();
        let cut_score = normalized(&cut).unwrap();
        let cut_plan = build_playback_plan(&cut_score.parts[0], PlaybackLimits::default());
        assert!(cut_plan.complete, "{:?}", cut_plan.diagnostics);
        assert_eq!(
            (
                cut_plan.visits[2].source_start,
                cut_plan.visits[2].source_end
            ),
            (ScoreTime::new(0, 1), ScoreTime::new(1, 1))
        );
        let cut_performance = generate(&cut, 120).unwrap();
        assert_eq!(
            (
                cut_performance.tracks[0].notes[2].start,
                cut_performance.tracks[0].notes[2].end
            ),
            (3840, 4800)
        );
        let source = import_musicxml_document(xml.as_bytes()).unwrap();
        let score = normalized(&source).unwrap();
        let plan = build_playback_plan(&score.parts[0], PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(plan.visits.len(), 3);
        assert_eq!(plan.visits[2].source_start, ScoreTime::new(1, 1));
        assert_eq!(plan.visits[2].source_end, ScoreTime::new(2, 1));
        let performance = generate(&source, 120).unwrap();
        assert_eq!(performance.tracks.len(), 2);
        for track in &performance.tracks {
            assert_eq!(track.notes.len(), 3);
            assert_eq!((track.notes[2].start, track.notes[2].end), (3840, 4800));
        }
        let file = midi_file::MidiFile::from_smf("partial", &performance.smf).unwrap();
        let alignment = crate::score_alignment::align_score_to_midi(&score, &file);
        assert!(
            alignment.navigation_complete,
            "{:?}",
            alignment.navigation_diagnostics
        );
        assert_eq!(
            crate::score_alignment::summarize_alignment(&alignment).coverage_percent,
            100
        );
        assert_eq!(file.tempo_track.pulses_to_duration(4800).as_secs_f64(), 4.0);
    }
}
