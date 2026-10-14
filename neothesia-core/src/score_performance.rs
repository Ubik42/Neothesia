//! Convert written notation to an owned, meter-aware practice performance.
use crate::{
    musicxml::*,
    practice::PracticePart,
    score_ornament::{TrillStrike, expand_trill},
    score_playback::{PlaybackLimits, build_playback_plan, grace_note_timing},
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
                        segno: false,
                        coda: false,
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
    if score.parts.len() > 1
        && score
            .parts
            .iter()
            .flat_map(|part| &part.measures)
            .flat_map(|measure| &measure.events)
            .any(|event| {
                matches!(event, ScoreEvent::Direction(direction)
                if direction.navigation.iter().any(|mark|
                    mark.attributes.get("fine").is_some_and(|value| value != "yes")))
            })
    {
        return Err(
            "数值 Fine 在多个分谱中可能对应不同末音或休止符，请先校对并拆成唯一演奏声部".into(),
        );
    }
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
    let grace_timings: Vec<_> = score
        .parts
        .iter()
        .map(grace_note_timing)
        .collect::<Result<_, _>>()?;
    let mut trill_strikes: HashMap<ScoreEventId, Vec<TrillStrike>> = HashMap::new();
    for (part, timings) in score.parts.iter().zip(&grace_timings) {
        for measure in &part.measures {
            for event in &measure.events {
                let ScoreEvent::Note(note) = event else {
                    continue;
                };
                let Some(mark) = &note.trill else { continue };
                if !note.ties.is_empty() {
                    return Err(format!(
                        "第 {} 小节的颤音与连音线相交，暂不能可靠展开，请校对原谱",
                        measure.number
                    ));
                }
                if note.grace || timings.contains_key(&note.id) {
                    return Err(format!(
                        "第 {} 小节的颤音与倚音借时值相交，暂不能可靠展开，请校对原谱",
                        measure.number
                    ));
                }
                let Some(written) = &note.pitch else {
                    return Err(format!("第 {} 小节的休止符不能标颤音", measure.number));
                };
                let strikes = expand_trill(mark, pitch(written)?, note.onset, note.duration)
                    .map_err(|error| format!("第 {} 小节：{error}", measure.number))?;
                trill_strikes.insert(note.id.clone(), strikes);
            }
        }
    }
    for (part, timings) in score.parts.iter().zip(&grace_timings) {
        for visit in &plan.visits {
            let Some(tail) = visit.fine_tail else {
                continue;
            };
            let measure = &part.measures[visit.source_measure_ordinal as usize];
            if measure.events.iter().any(|event| {
                matches!(event,
                ScoreEvent::Note(note) if !note.grace && timings.contains_key(&note.id)
                    && note.onset.subtract(measure.start) == tail.onset)
            }) {
                return Err(format!(
                    "第 {} 小节的数值 Fine 与装饰音借时值相交，请校对实际终止时值",
                    visit.source_measure_ordinal + 1
                ));
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
            return Err("谱面时间精度超出 MIDI 分辨率，请调整装饰音或连音时值".into());
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
    for spans in &grace_timings {
        for span in spans.values() {
            include(span.onset)?;
            include(span.duration)?;
        }
    }
    for strikes in trill_strikes.values() {
        for strike in strikes {
            include(strike.onset)?;
            include(strike.duration)?;
        }
    }
    for visit in &plan.visits {
        if let Some(tail) = visit.fine_tail {
            include(tail.actual_duration)?;
            include(visit.performed_end())?;
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
        let performed_end = tick(visit.performed_end())?;
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
            let position = at + written - source;
            if position <= performed_end {
                tempos.insert(position, value);
            }
        }
    }
    let mut tracks = vec![];
    let mut total = 0;
    let mut generated_count = 0;
    for (part, grace_spans) in score.parts.iter().zip(&grace_timings) {
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
                let visit_end = tick(visit.performed_end())?;
                let source = m.start.add(visit.source_start);
                let source_end = m.start.add(visit.source_end);
                for event in &m.events {
                    let ScoreEvent::Note(note) = event else {
                        continue;
                    };
                    let Some(span) = grace_spans.get(&note.id) else {
                        continue;
                    };
                    let split = if note.grace && span.onset < note.onset {
                        source > span.onset && source <= note.onset
                            || source_end > span.onset && source_end <= note.onset
                    } else if !note.grace && span.onset > note.onset {
                        source > note.onset && source <= span.onset
                            || source_end > note.onset && source_end <= span.onset
                    } else {
                        false
                    };
                    if split {
                        return Err(format!(
                            "第 {} 小节的跳转切开了装饰音和主音，请调整跳转位置",
                            index + 1
                        ));
                    }
                }
                for event in &m.events {
                    let ScoreEvent::Note(note) = event else {
                        continue;
                    };
                    let Some(strikes) = trill_strikes.get(&note.id) else {
                        continue;
                    };
                    let first = strikes.first().unwrap().onset;
                    let last = strikes.last().unwrap();
                    let end = last.onset.add(last.duration);
                    if (first < source && source < end)
                        || (first < source_end && source_end < end)
                        || visit
                            .fine_tail
                            .is_some_and(|tail| note.onset.subtract(m.start) == tail.onset)
                    {
                        return Err(format!(
                            "第 {} 小节的跳转或 Fine 切开颤音，请校对演奏范围",
                            index + 1
                        ));
                    }
                }
                total = total.max(visit_end);
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
                            let at = start + tick(a.onset.subtract(source))?;
                            if at <= visit_end {
                                meters.insert(at, meter(t)?);
                            }
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
                            if at > visit_end {
                                continue;
                            }
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
                            let grace_span = grace_spans.get(&n.id);
                            let effective_onset = grace_span.map_or(n.onset, |span| span.onset);
                            let effective_duration =
                                grace_span.map_or(n.duration, |span| span.duration);
                            if n.staff != staff
                                || effective_onset >= source_end
                                || effective_onset.add(effective_duration) <= source
                            {
                                continue;
                            }
                            let Some(written) = &n.pitch else {
                                continue;
                            };
                            if effective_duration.numerator <= 0 {
                                continue;
                            }
                            let plain_pitch = pitch(written)?;
                            let plain = TrillStrike {
                                onset: effective_onset,
                                duration: effective_duration,
                                pitch: plain_pitch,
                                ordinal: 0,
                            };
                            let strikes = trill_strikes.get(&n.id).map_or_else(
                                || std::slice::from_ref(&plain),
                                |strikes| strikes.as_slice(),
                            );
                            let first_main = strikes
                                .iter()
                                .find(|strike| strike.pitch == plain_pitch)
                                .map(|strike| strike.ordinal);
                            for strike in strikes {
                                generated_count += 1;
                                if generated_count > 200_000 {
                                    return Err("演奏音符超过 20 万，请分段导入".into());
                                }
                                let pitch = strike.pitch;
                                let onset =
                                    start + tick(strike.onset.max(source).subtract(source))?;
                                let end = if n.trill.is_none() {
                                    if let Some(tail) = visit.fine_tail.filter(|tail| {
                                        effective_onset.subtract(m.start) == tail.onset
                                            && n.duration == tail.written_duration
                                    }) {
                                        onset + tick(tail.actual_duration)?
                                    } else {
                                        start
                                            + tick(
                                                strike
                                                    .onset
                                                    .add(strike.duration)
                                                    .min(source_end)
                                                    .subtract(source),
                                            )?
                                    }
                                } else {
                                    start
                                        + tick(strike.onset.add(strike.duration).subtract(source))?
                                };
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
                                        finger: (Some(strike.ordinal) == first_main)
                                            .then(|| n.fingering.as_ref())
                                            .flatten()
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
    fn nested_repeat_route_is_shared_by_parts_without_duplicate_barlines() {
        let mut xml = String::from(
            r#"<score-partwise><part-list><score-part id="P1"><part-name>Upper</part-name></score-part><score-part id="P2"><part-name>Lower</part-name></score-part></part-list>"#,
        );
        for (part_id, step, octave) in [("P1", "C", 4), ("P2", "G", 3)] {
            xml.push_str(&format!(r#"<part id="{part_id}">"#));
            for number in 1..=5 {
                let left = if part_id == "P1" && (number == 1 || number == 2) {
                    r#"<barline location="left"><repeat direction="forward"/></barline>"#
                } else {
                    ""
                };
                let right = if part_id == "P1" && (number == 3 || number == 4) {
                    r#"<barline><repeat direction="backward"/></barline>"#
                } else {
                    ""
                };
                xml.push_str(&format!(r#"<measure number="{number}"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes>{left}<note><pitch><step>{step}</step><octave>{octave}</octave></pitch><duration>1</duration></note>{right}</measure>"#));
            }
            xml.push_str("</part>");
        }
        xml.push_str("</score-partwise>");
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        assert_eq!(performance.tracks.len(), 2);
        assert_eq!(performance.tracks[0].notes.len(), 13);
        assert_eq!(performance.tracks[1].notes.len(), 13);
        assert_eq!(
            performance.tracks[0]
                .notes
                .iter()
                .map(|note| note.start)
                .collect::<Vec<_>>(),
            performance.tracks[1]
                .notes
                .iter()
                .map(|note| note.start)
                .collect::<Vec<_>>()
        );
    }
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
    fn printed_fine_stops_the_generated_practice_file_after_the_return() {
        let xml = r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">
        <measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure>
        <measure number="2"><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration></note><direction><direction-type><words>Fine</words></direction-type></direction></measure>
        <measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration></note><direction><direction-type><words>D.C. al Fine</words></direction-type><sound dacapo="yes"/></direction></measure>
        </part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        let notes = &performance.tracks[0].notes;
        assert_eq!(
            notes.iter().map(|note| note.pitch).collect::<Vec<_>>(),
            [60, 62, 64, 60, 62]
        );
        assert_eq!(
            notes.iter().map(|note| note.start).collect::<Vec<_>>(),
            [0, 960, 1920, 2880, 3840]
        );
        assert_eq!(notes.last().unwrap().end, 4800);
        let file = midi_file::MidiFile::from_smf("printed-fine", &performance.smf).unwrap();
        assert_eq!(
            file.tracks
                .iter()
                .flat_map(|track| track.notes.iter())
                .last()
                .unwrap()
                .note,
            62
        );
    }

    #[test]
    fn printed_dal_segno_fine_generates_the_return_passage_and_midi_end() {
        let xml = r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">
        <measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure>
        <measure number="2"><direction><direction-type><segno/></direction-type></direction><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration></note></measure>
        <measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration></note><direction><direction-type><words>Fine</words></direction-type></direction></measure>
        <measure number="4"><note><pitch><step>F</step><octave>4</octave></pitch><duration>1</duration></note><direction><direction-type><words>D.S. al Fine</words></direction-type></direction></measure>
        </part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        let notes = &performance.tracks[0].notes;
        assert_eq!(
            notes.iter().map(|note| note.pitch).collect::<Vec<_>>(),
            [60, 62, 64, 65, 62, 64]
        );
        assert_eq!(
            notes.iter().map(|note| note.start).collect::<Vec<_>>(),
            [0, 960, 1920, 2880, 3840, 4800]
        );
        assert_eq!(notes.last().unwrap().end, 5760);
        let file = midi_file::MidiFile::from_smf("printed-ds-fine", &performance.smf).unwrap();
        assert_eq!(
            file.tracks
                .iter()
                .flat_map(|track| track.notes.iter())
                .count(),
            6
        );
        assert!((file.duration.as_secs_f64() - 3.0).abs() < 0.00001);
    }
    #[test]
    fn printed_plain_dal_segno_generates_both_passages_through_the_last_measure() {
        let xml = r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">
        <measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure>
        <measure number="2"><direction><direction-type><segno/></direction-type></direction><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration></note></measure>
        <measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration></note></measure>
        <measure number="4"><note><pitch><step>F</step><octave>4</octave></pitch><duration>1</duration></note><direction><direction-type><words>D.S.</words></direction-type></direction></measure>
        </part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        let pitches: Vec<_> = performance.tracks[0]
            .notes
            .iter()
            .map(|note| note.pitch)
            .collect();
        assert_eq!(pitches, [60, 62, 64, 65, 62, 64, 65]);
        assert_eq!(performance.tracks[0].notes.last().unwrap().end, 6720);
        let file = midi_file::MidiFile::from_smf("printed-ds", &performance.smf).unwrap();
        assert_eq!(
            file.tracks
                .iter()
                .flat_map(|track| track.notes.iter())
                .count(),
            7
        );
        assert!((file.duration.as_secs_f64() - 3.5).abs() < 0.00001);
    }
    #[test]
    fn printed_dal_segno_coda_preserves_notes_source_occurrences_and_midi_end() {
        let xml = r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">
        <measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure>
        <measure number="2"><direction><direction-type><segno/></direction-type></direction><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration></note></measure>
        <measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration></note><direction><direction-type><words>To Coda</words></direction-type></direction></measure>
        <measure number="4"><note><pitch><step>F</step><octave>4</octave></pitch><duration>1</duration></note><direction><direction-type><words>D.S. al Coda</words></direction-type></direction></measure>
        <measure number="5"><note><pitch><step>G</step><octave>4</octave></pitch><duration>1</duration></note></measure>
        <measure number="6"><direction><direction-type><coda/></direction-type></direction><note><pitch><step>A</step><octave>4</octave></pitch><duration>1</duration></note></measure>
        </part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        let notes = &performance.tracks[0].notes;
        assert_eq!(
            notes.iter().map(|note| note.pitch).collect::<Vec<_>>(),
            [60, 62, 64, 65, 62, 64, 69]
        );
        assert_eq!(
            notes.iter().map(|note| note.start).collect::<Vec<_>>(),
            [0, 960, 1920, 2880, 3840, 4800, 5760]
        );
        assert_eq!(notes.last().unwrap().end, 6720);
        let part = &score.parts[0];
        let plan = crate::score_playback::build_playback_plan(part, Default::default());
        let events = crate::score_playback::expand_event_occurrences(part, &plan);
        let d_occurrences: Vec<_> = events
            .iter()
            .filter(|event| event.id.source_id.measure_ordinal == 1 && event.duration.is_some())
            .collect();
        assert_eq!(d_occurrences.len(), 2);
        assert_eq!(d_occurrences[0].id.source_id, d_occurrences[1].id.source_id);
        assert_ne!(d_occurrences[0].id, d_occurrences[1].id);
        let file = midi_file::MidiFile::from_smf("printed-ds-coda", &performance.smf).unwrap();
        assert_eq!(
            file.tracks
                .iter()
                .flat_map(|track| track.notes.iter())
                .count(),
            7
        );
        assert!((file.duration.as_secs_f64() - 3.5).abs() < 0.00001);
    }
    #[test]
    fn trill_repeats_as_separate_key_strikes_and_preserves_midi_precision() {
        let xml = r#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><barline location="left"><repeat direction="forward"/></barline><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><notations><ornaments><trill-mark beats="4" start-note="upper" trill-step="half" second-beat="25" last-beat="75"/></ornaments><technical><fingering>3</fingering></technical></notations></note><barline location="right"><repeat direction="backward"/></barline></measure></part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        let notes = &performance.tracks[0].notes;
        assert_eq!(notes.len(), 8);
        assert_eq!(
            notes.iter().map(|n| n.pitch).collect::<Vec<_>>(),
            [61, 60, 61, 60, 61, 60, 61, 60]
        );
        assert_eq!(
            notes.iter().map(|n| n.finger).collect::<Vec<_>>(),
            [None, Some(3), None, None, None, Some(3), None, None]
        );
        assert_eq!(
            notes.iter().map(|n| n.start).collect::<Vec<_>>(),
            [0, 240, 480, 720, 960, 1200, 1440, 1680]
        );
        assert_eq!(notes.last().unwrap().end, 1920);
        let midi = midi_file::MidiFile::from_smf("trill-repeat", &performance.smf).unwrap();
        assert_eq!(
            midi.tracks
                .iter()
                .flat_map(|track| track.notes.iter())
                .count(),
            8
        );
    }

    #[test]
    fn unsupported_trill_hint_reports_its_written_measure() {
        let xml = r#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="7"><attributes><divisions>4</divisions></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><notations><ornaments><trill-mark accelerate="yes"/></ornaments></notations></note></measure></part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let error = generate(&score, 120).err().unwrap();
        assert!(error.contains("第 7 小节"), "{error}");
        assert!(error.contains("accelerate"), "{error}");
    }

    #[test]
    fn timed_grace_and_trill_cannot_create_conflicting_alignment() {
        let xml = r#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><grace slash="yes" steal-time-following="25"/><pitch><step>B</step><octave>3</octave></pitch></note><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><notations><ornaments><trill-mark/></ornaments></notations></note></measure></part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let error = generate(&score, 120).err().unwrap();
        assert!(error.contains("第 1 小节"), "{error}");
        assert!(error.contains("倚音借时值"), "{error}");
    }

    #[test]
    fn timed_grace_notes_borrow_from_following_notes_on_every_repeat() {
        for (slash, share, grace_ticks) in [("yes", "25", 240_u64), ("no", "50", 480_u64)] {
            let xml = format!(
                r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">
            <measure number="1"><attributes><divisions>4</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><barline location="left"><repeat direction="forward"/></barline><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note></measure>
            <measure number="2"><note><grace slash="{slash}" steal-time-following="{share}"/><pitch><step>D</step><octave>4</octave></pitch></note><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note><barline location="right"><repeat direction="backward"/></barline></measure>
            </part></score-partwise>"#
            );
            let score = import_musicxml_document(xml.as_bytes()).unwrap();
            let performance = generate(&score, 120).unwrap();
            let notes = &performance.tracks[0].notes;
            assert_eq!(
                notes.iter().map(|note| note.pitch).collect::<Vec<_>>(),
                [60, 62, 64, 60, 62, 64]
            );
            assert_eq!(
                notes.iter().map(|note| note.start).collect::<Vec<_>>(),
                [0, 960, 960 + grace_ticks, 1920, 2880, 2880 + grace_ticks]
            );
            assert_eq!(notes[1].end, 960 + grace_ticks);
            assert_eq!(notes[4].end, 2880 + grace_ticks);
            assert_eq!(notes.last().unwrap().end, 3840);
            let plan =
                crate::score_playback::build_playback_plan(&score.parts[0], Default::default());
            let events = crate::score_playback::expand_event_occurrences(&score.parts[0], &plan);
            let grace: Vec<_> = events
                .iter()
                .filter(|event| {
                    event.id.source_id.measure_ordinal == 1 && event.id.source_id.ordinal == 0
                })
                .collect();
            assert_eq!(grace.len(), 2);
            assert_eq!(
                grace[0].duration,
                Some(ScoreTime::new(share.parse::<i64>().unwrap(), 100))
            );
            assert_eq!(grace[0].id.source_id, grace[1].id.source_id);
            assert_ne!(grace[0].id, grace[1].id);
            let file = midi_file::MidiFile::from_smf("grace", &performance.smf).unwrap();
            assert_eq!(
                file.tracks
                    .iter()
                    .flat_map(|track| track.notes.iter())
                    .count(),
                6
            );
            assert!((file.duration.as_secs_f64() - 2.0).abs() < 0.00001);
        }
    }

    #[test]
    fn previous_note_grace_keeps_repeat_occurrences_and_midi_length() {
        let xml = r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions><time><beats>2</beats><beat-type>4</beat-type></time></attributes><barline location="left"><repeat direction="forward"/></barline><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note><note><grace slash="yes" steal-time-previous="25"/><pitch><step>D</step><octave>4</octave></pitch></note><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note><barline location="right"><repeat direction="backward"/></barline></measure></part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        let notes = &performance.tracks[0].notes;
        assert_eq!(
            notes
                .iter()
                .map(|n| (n.pitch, n.start, n.end))
                .collect::<Vec<_>>(),
            [
                (60, 0, 720),
                (62, 720, 960),
                (64, 960, 1920),
                (60, 1920, 2640),
                (62, 2640, 2880),
                (64, 2880, 3840)
            ]
        );
        let plan = crate::score_playback::build_playback_plan(&score.parts[0], Default::default());
        let events = crate::score_playback::expand_event_occurrences(&score.parts[0], &plan);
        let grace: Vec<_> = events
            .iter()
            .filter(|e| e.id.source_id.ordinal == 1)
            .collect();
        assert_eq!(grace.len(), 2);
        assert_eq!(grace[0].onset, ScoreTime::new(3, 4));
        assert_eq!(grace[1].onset, ScoreTime::new(11, 4));
        assert_eq!(grace[0].duration, Some(ScoreTime::new(1, 4)));
        assert_eq!(grace[0].id.source_id, grace[1].id.source_id);
        assert_ne!(grace[0].id, grace[1].id);
        let midi = midi_file::MidiFile::from_smf("previous-grace", &performance.smf).unwrap();
        assert!((midi.duration.as_secs_f64() - 2.0).abs() < 0.00001);
    }

    #[test]
    fn several_previous_graces_shorten_the_whole_preceding_chord() {
        let xml = r#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions><time><beats>2</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note><note><chord/><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note><note><grace steal-time-previous="20"/><pitch><step>D</step><octave>4</octave></pitch></note><note><grace steal-time-previous="30"/><pitch><step>F</step><octave>4</octave></pitch></note><note><pitch><step>G</step><octave>4</octave></pitch><duration>4</duration></note></measure></part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        assert_eq!(
            performance.tracks[0]
                .notes
                .iter()
                .map(|n| (n.pitch, n.start, n.end))
                .collect::<Vec<_>>(),
            [
                (60, 0, 480),
                (64, 0, 480),
                (62, 480, 672),
                (65, 672, 960),
                (67, 960, 1920)
            ]
        );
    }

    #[test]
    fn make_time_uses_only_an_existing_voice_gap_and_its_own_divisions() {
        let xml = r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions><time><beats>2</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note><attributes><divisions>8</divisions></attributes><forward><duration>4</duration></forward><note><grace slash="no" make-time="4"/><pitch><step>D</step><octave>4</octave></pitch></note><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note></measure></part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        assert_eq!(
            performance.tracks[0]
                .notes
                .iter()
                .map(|n| (n.pitch, n.start, n.end))
                .collect::<Vec<_>>(),
            [(60, 0, 960), (62, 960, 1440), (64, 1440, 1920)]
        );
        let plan = crate::score_playback::build_playback_plan(&score.parts[0], Default::default());
        let events = crate::score_playback::expand_event_occurrences(&score.parts[0], &plan);
        let grace = events.iter().find(|e| e.id.source_id.ordinal == 1).unwrap();
        assert_eq!(grace.onset, ScoreTime::new(1, 1));
        assert_eq!(grace.duration, Some(ScoreTime::new(1, 2)));
        let midi = midi_file::MidiFile::from_smf("gap-grace", &performance.smf).unwrap();
        assert!((midi.duration.as_secs_f64() - 1.0).abs() < 0.00001);
    }

    #[test]
    fn previous_and_make_time_reject_overlong_or_conflicting_hints() {
        for (hint, expected) in [
            ("steal-time-previous=\"100\"", "百分数"),
            (
                "steal-time-previous=\"70\" steal-time-following=\"20\"",
                "一种计时方式",
            ),
            ("make-time=\"1\"", "足够的同声部空隙"),
            ("make-time=\"0\"", "make-time 须为正数"),
        ] {
            let xml = format!(
                r#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note><note><grace {hint}/><pitch><step>D</step><octave>4</octave></pitch></note><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note></measure></part></score-partwise>"#
            );
            let score = import_musicxml_document(xml.as_bytes()).unwrap();
            let error = generate(&score, 120).err().unwrap();
            assert!(
                error.contains("第 1 小节") && error.contains(expected),
                "{hint}: {error}"
            );
        }
    }

    #[test]
    fn ambiguous_or_unquantized_grace_notes_report_the_written_measure() {
        let normal =
            r#"<note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note>"#;
        for grace in [
            r#"<grace slash="yes"/>"#,
            r#"<grace slash="yes" steal-time-previous="10"/>"#,
            r#"<grace slash="no" make-time="1"/>"#,
            r#"<grace slash="no" steal-time-following="100"/>"#,
            r#"<grace slash="yes" steal-time-following="33.3333"/>"#,
        ] {
            let xml = format!(
                r#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions></attributes><note>{grace}<pitch><step>D</step><octave>4</octave></pitch></note>{normal}</measure></part></score-partwise>"#
            );
            let score = import_musicxml_document(xml.as_bytes()).unwrap();
            let error = generate(&score, 120).err().unwrap();
            assert!(error.contains("第 1 小节"), "{grace}: {error}");
        }
    }

    #[test]
    fn appoggiatura_with_the_official_thirty_three_percent_hint_uses_exact_midi_time() {
        // MusicXML's Appoggiatura example uses slash="no" and
        // steal-time-following="33"; the latter is an exact percentage.
        let xml = r#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><grace slash="no" steal-time-following="33"/><pitch><step>D</step><octave>4</octave></pitch></note><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note></measure></part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        let notes = &performance.tracks[0].notes;
        assert_eq!(
            notes
                .iter()
                .map(|note| (note.pitch, note.start, note.end))
                .collect::<Vec<_>>(),
            [(62, 0, 1584), (64, 1584, 4800)]
        );
        let midi = midi_file::MidiFile::from_smf("appoggiatura", &performance.smf).unwrap();
        assert!(
            (midi.duration.as_secs_f64() - 0.5).abs() < 0.00001,
            "duration={:?}",
            midi.duration
        );
    }

    #[test]
    fn consecutive_grace_notes_share_one_principal_without_extending_the_measure() {
        let xml = r#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><grace slash="yes" steal-time-following="20"/><pitch><step>C</step><octave>4</octave></pitch></note><note><grace slash="yes" steal-time-following="30"/><pitch><step>D</step><octave>4</octave></pitch></note><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note></measure></part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        assert_eq!(
            performance.tracks[0]
                .notes
                .iter()
                .map(|note| (note.pitch, note.start, note.end))
                .collect::<Vec<_>>(),
            [(60, 0, 192), (62, 192, 480), (64, 480, 960)]
        );
        let midi = midi_file::MidiFile::from_smf("grace-group", &performance.smf).unwrap();
        assert!((midi.duration.as_secs_f64() - 0.5).abs() < 0.00001);
    }
    #[test]
    fn numeric_fine_keeps_midi_notes_occurrences_and_end_of_track_in_sync() {
        for (fine, final_end) in [("0.5", 4320_u64), ("2", 5760_u64)] {
            let xml = format!(
                r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">
            <measure number="1"><attributes><divisions>4</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note></measure>
            <measure number="2"><note><pitch><step>D</step><octave>4</octave></pitch><duration>4</duration></note><sound fine="{fine}"/></measure>
            <measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note><sound dacapo="yes"/></measure>
            </part></score-partwise>"#
            );
            let score = import_musicxml_document(xml.as_bytes()).unwrap();
            let performance = generate(&score, 120).unwrap();
            let notes = &performance.tracks[0].notes;
            assert_eq!(
                notes.iter().map(|note| note.pitch).collect::<Vec<_>>(),
                [60, 62, 64, 60, 62]
            );
            assert_eq!(notes[1].end, 1920);
            assert_eq!(notes[4].start, 3840);
            assert_eq!(notes[4].end, final_end);
            for track in &performance.smf.tracks {
                let end = track
                    .iter()
                    .map(|event| u64::from(event.delta.as_int()))
                    .sum::<u64>();
                assert_eq!(end, final_end);
                assert!(matches!(
                    track.last().unwrap().kind,
                    TrackEventKind::Meta(MetaMessage::EndOfTrack)
                ));
            }
            let file = midi_file::MidiFile::from_smf("numeric-fine", &performance.smf).unwrap();
            assert!((file.duration.as_secs_f64() - final_end as f64 / 1920.0).abs() < 0.00001);
        }
    }

    #[test]
    fn numeric_fine_rest_extends_the_silent_midi_tail() {
        let xml = r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">
        <measure number="1"><attributes><divisions>4</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note></measure>
        <measure number="2"><note><rest/><duration>4</duration></note><sound fine="2"/></measure>
        <measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note><sound dacapo="yes"/></measure>
        </part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        assert_eq!(
            performance.tracks[0]
                .notes
                .iter()
                .map(|note| note.pitch)
                .collect::<Vec<_>>(),
            [60, 64, 60]
        );
        for track in &performance.smf.tracks {
            assert_eq!(
                track
                    .iter()
                    .map(|event| u64::from(event.delta.as_int()))
                    .sum::<u64>(),
                5760
            );
        }
    }

    #[test]
    fn numeric_fine_rejects_multiple_parts_with_unresolved_final_durations() {
        let xml = r#"<score-partwise version="4.0"><part-list>
        <score-part id="P1"><part-name>Upper</part-name></score-part>
        <score-part id="P2"><part-name>Lower</part-name></score-part>
        </part-list>
        <part id="P1"><measure number="1"><attributes><divisions>4</divisions></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note><sound fine="2"/></measure><measure number="2"><note><rest/><duration>4</duration></note><sound dacapo="yes"/></measure></part>
        <part id="P2"><measure number="1"><attributes><divisions>4</divisions></attributes><note><pitch><step>C</step><octave>3</octave></pitch><duration>2</duration></note><note><rest/><duration>2</duration></note></measure><measure number="2"><note><rest/><duration>4</duration></note></measure></part>
        </score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        assert!(generate(&score, 120).err().unwrap().contains("多个分谱"));
    }

    #[test]
    fn shortened_numeric_fine_omits_later_pedal_events_from_final_return() {
        let xml = r#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">
        <measure number="1"><attributes><divisions>4</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note></measure>
        <measure number="2"><note><pitch><step>D</step><octave>4</octave></pitch><duration>4</duration></note><direction><direction-type><pedal type="start"/></direction-type><offset>-1</offset></direction><sound fine="0.5"/></measure>
        <measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note><sound dacapo="yes"/></measure>
        </part></score-partwise>"#;
        let score = import_musicxml_document(xml.as_bytes()).unwrap();
        let performance = generate(&score, 120).unwrap();
        assert_eq!(performance.tracks[0].notes.last().unwrap().end, 4320);
        for track in &performance.smf.tracks {
            assert_eq!(
                track
                    .iter()
                    .map(|event| u64::from(event.delta.as_int()))
                    .sum::<u64>(),
                4320
            );
        }
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
