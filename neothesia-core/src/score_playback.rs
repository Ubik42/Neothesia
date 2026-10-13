//! Bounded performed-order planning for notation-neutral scores.
//!
//! The imported [`Part`](crate::musicxml::Part) always remains in written
//! document order. This module produces visits into that immutable source.

use std::collections::{BTreeMap, BTreeSet};

use crate::musicxml::{
    BarlineLocation, EndingType, Part, RepeatDirection, ScoreEvent, ScoreEventId, ScoreTime,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackLimits {
    pub max_visits: usize,
    pub max_repeat_passes: u16,
}

impl Default for PlaybackLimits {
    fn default() -> Self {
        Self {
            max_visits: 100_000,
            max_repeat_passes: 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureVisit {
    pub part_id: String,
    pub source_measure_ordinal: u32,
    pub occurrence_ordinal: u32,
    pub repeat_pass: u16,
    pub performed_start: ScoreTime,
    /// Half-open written interval within this measure, in quarter-note units.
    pub source_start: ScoreTime,
    pub source_end: ScoreTime,
    /// The last written note/rest can have a different duration on the final
    /// return to Fine. The source interval stays in written coordinates.
    pub fine_tail: Option<FineTail>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FineTail {
    pub onset: ScoreTime,
    pub written_duration: ScoreTime,
    pub actual_duration: ScoreTime,
}

impl MeasureVisit {
    pub fn performed_end(&self) -> ScoreTime {
        self.performed_start
            .add(self.source_end.subtract(self.source_start))
            .add(self.fine_tail.map_or(ScoreTime::default(), |tail| {
                tail.actual_duration.subtract(tail.written_duration)
            }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScoreEventOccurrenceId {
    pub source_id: ScoreEventId,
    pub measure_occurrence_ordinal: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformedScoreEvent {
    pub id: ScoreEventOccurrenceId,
    pub onset: ScoreTime,
    pub duration: Option<ScoreTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackDiagnostic {
    pub measure_ordinal: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackPlan {
    pub visits: Vec<MeasureVisit>,
    pub diagnostics: Vec<PlaybackDiagnostic>,
    pub complete: bool,
}

#[derive(Clone, Debug)]
struct Navigation {
    offset: ScoreTime,
    attributes: BTreeMap<String, String>,
    passes: Option<Vec<u16>>,
}

#[derive(Clone, Debug)]
struct RepeatFrame {
    start: (usize, ScoreTime),
    pass: u16,
}

fn numeric_fine_duration(value: &str) -> Option<ScoreTime> {
    // MusicXML does not state a unit for numeric Fine. A published
    // nwc2musicxml D.C. al Fine fixture uses divisions=480, a final half note
    // with duration=960, and fine=3. Interpreting 3 as quarter notes produces
    // its intended three-beat ending; treating it as divisions would produce
    // a 3/480-beat ending. See TestFiles/TestDCalFine.xml in musescore/nwc2musicxml.
    let (whole, fraction) = value.trim().split_once('.').unwrap_or((value.trim(), ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 3
        || (value.contains('.') && fraction.is_empty())
    {
        return None;
    }
    let scale = 10_u32.checked_pow(fraction.len() as u32)?;
    let units = whole
        .parse::<u64>()
        .ok()?
        .checked_mul(u64::from(scale))?
        .checked_add(if fraction.is_empty() {
            0
        } else {
            fraction.parse::<u64>().ok()?
        })?;
    if units == 0 || units > u64::try_from(i64::MAX).ok()? {
        return None;
    }
    let duration = ScoreTime::new(units as i64, scale);
    (duration <= ScoreTime::new(64, 1)).then_some(duration)
}

fn numeric_fine_tail(
    part: &Part,
    index: usize,
    offset: ScoreTime,
    value: &str,
) -> Result<FineTail, String> {
    let measure = &part.measures[index];
    let actual_duration = numeric_fine_duration(value).ok_or_else(|| {
        "数值 Fine 须为大于零、最多三位小数且不超过 64 拍的四分音符时值".to_string()
    })?;
    let terminal: Vec<_> = measure
        .events
        .iter()
        .filter_map(|event| match event {
            ScoreEvent::Note(note)
                if !note.grace
                    && note.duration > ScoreTime::default()
                    && note.onset.subtract(measure.start).add(note.duration) == offset =>
            {
                Some(note)
            }
            _ => None,
        })
        .collect();
    let Some(last) = terminal.first() else {
        return Err("数值 Fine 前没有在该位置结束的音符或休止符".into());
    };
    let onset = last.onset.subtract(measure.start);
    if terminal.iter().any(|note| {
        note.onset != last.onset
            || note.duration != last.duration
            || note.staff != last.staff
            || note.voice != last.voice
            || note.pitch.is_none() != last.pitch.is_none()
            || !note.ties.is_empty()
    }) || !last.ties.is_empty()
        || measure.events.iter().any(|event| {
            matches!(event, ScoreEvent::Note(note)
                if !note.grace
                    && note.duration > ScoreTime::default()
                    && note.onset.subtract(measure.start) < offset
                    && note.onset.subtract(measure.start).add(note.duration) > onset
                    && (note.onset != last.onset
                        || note.duration != last.duration
                        || note.staff != last.staff
                        || note.voice != last.voice))
        })
    {
        return Err("数值 Fine 涉及多个声部、谱表或延音线，无法唯一确定末音时值".into());
    }
    Ok(FineTail {
        onset,
        written_duration: last.duration,
        actual_duration,
    })
}

fn forward_boundaries(
    part: &Part,
    navigation: &[Vec<Navigation>],
    plan: &mut PlaybackPlan,
) -> Vec<Vec<ScoreTime>> {
    let mut boundaries = vec![Vec::new(); part.measures.len()];
    for (index, measure) in part.measures.iter().enumerate() {
        let backward_count = measure
            .barlines
            .iter()
            .filter_map(|barline| barline.repeat.as_ref())
            .filter(|repeat| repeat.direction == RepeatDirection::Backward)
            .count();
        if backward_count > 1 {
            diagnose(
                plan,
                Some(index),
                "同一小节有多个反向反复记号，无法确定结束位置".into(),
            );
        }
        for barline in &measure.barlines {
            if !matches!(
                barline.repeat.as_ref().map(|repeat| &repeat.direction),
                Some(RepeatDirection::Forward)
            ) {
                continue;
            }
            let target = match barline.location {
                BarlineLocation::Left => index,
                BarlineLocation::Right => index + 1,
                BarlineLocation::Middle | BarlineLocation::Other(_) => {
                    diagnose(
                        plan,
                        Some(index),
                        "正向反复记号位于小节中间，但谱面没有给出准确拍位".into(),
                    );
                    continue;
                }
            };
            if let Some(local) = boundaries.get_mut(target) {
                local.push(ScoreTime::default());
            } else {
                diagnose(plan, Some(index), "正向反复记号位于乐曲结束之后".into());
            }
        }
        for mark in &navigation[index] {
            if !mark
                .attributes
                .get("forward-repeat")
                .is_some_and(|value| value == "yes")
            {
                continue;
            }
            if mark.offset == measure.duration {
                if let Some(local) = boundaries.get_mut(index + 1) {
                    local.push(ScoreTime::default());
                } else {
                    diagnose(plan, Some(index), "正向反复记号位于乐曲结束之后".into());
                }
            } else {
                boundaries[index].push(mark.offset);
            }
        }
    }
    for local in &mut boundaries {
        local.sort();
        local.dedup();
    }
    boundaries
}

fn navigation(
    part: &Part,
    plan: &mut PlaybackPlan,
) -> (
    Vec<Vec<Navigation>>,
    BTreeMap<(String, String), (usize, ScoreTime)>,
) {
    let mut marks = vec![vec![]; part.measures.len()];
    let mut targets = BTreeMap::new();
    let mut text_fines = Vec::new();
    let mut text_da_capos = Vec::new();
    let mut text_dal_segnos = Vec::new();
    let mut printed_segnos = Vec::new();
    for (index, m) in part.measures.iter().enumerate() {
        for e in &m.events {
            let ScoreEvent::Direction(d) = e else {
                continue;
            };
            let offset = d.onset.subtract(m.start);
            let printed_line = d.words.join(" ");
            let printed_command: String = printed_line
                .chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect();
            for word in &d.words {
                let plain: String = word
                    .chars()
                    .filter(|character| character.is_ascii_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .collect();
                if plain == "fine" {
                    text_fines.push((index, offset));
                }
            }
            let recognized_dc = printed_command == "dcalfine" || printed_command == "dacapoalfine";
            let recognized_ds =
                printed_command == "dsalfine" || printed_command == "dalsegnoalfine";
            if recognized_dc {
                text_da_capos.push((index, offset));
            }
            if recognized_ds {
                text_dal_segnos.push((index, offset));
            }
            if d.segno || printed_command == "segno" {
                printed_segnos.push((index, offset));
            }
            let has_command = d.navigation.iter().any(|s| {
                ["dacapo", "dalsegno", "tocoda", "fine"]
                    .iter()
                    .any(|k| s.attributes.contains_key(*k))
            }) || m
                .events
                .iter()
                .filter_map(|e| {
                    if let ScoreEvent::Direction(other) = e {
                        Some(other)
                    } else {
                        None
                    }
                })
                .flat_map(|other| &other.navigation)
                .any(|s| {
                    s.onset == d.onset
                        && ["dacapo", "dalsegno", "tocoda", "fine"]
                            .iter()
                            .any(|k| s.attributes.contains_key(*k))
                });
            if !has_command
                && !recognized_dc
                && !recognized_ds
                && d.words.iter().any(|w| {
                    let w = w.to_lowercase().replace(' ', "");
                    w.contains("d.c.")
                        || w.contains("d.s.")
                        || w.contains("dacapo")
                        || w.contains("dalsegno")
                })
            {
                diagnose(
                    plan,
                    Some(index),
                    "D.C./D.S. 只有文字，没有可执行的 MusicXML sound 跳转指令".into(),
                );
            }
            for sound in &d.navigation {
                let offset = sound.onset.subtract(m.start);
                if offset < ScoreTime::default() || offset > m.duration {
                    diagnose(plan, Some(index), "跳转记号的位置超出本小节".into());
                    continue;
                }
                let passes = sound.attributes.get("time-only").map(|v| {
                    let values: Option<Vec<u16>> = v
                        .split(',')
                        .map(|p| p.trim().parse::<u16>().ok().filter(|p| *p > 0))
                        .collect();
                    if values
                        .as_ref()
                        .is_none_or(|p| p.is_empty() || p.windows(2).any(|w| w[0] >= w[1]))
                    {
                        diagnose(
                            plan,
                            Some(index),
                            "time-only 须为递增的正整数次数列表".into(),
                        );
                    }
                    values.unwrap_or_default()
                });
                for kind in ["segno", "coda", "dalsegno", "tocoda"] {
                    if sound
                        .attributes
                        .get(kind)
                        .is_some_and(|v| v.trim().is_empty())
                    {
                        diagnose(plan, Some(index), format!("{kind} 的目标名称不能为空"));
                    }
                }
                if sound
                    .attributes
                    .get("forward-repeat")
                    .is_some_and(|v| v != "yes")
                {
                    diagnose(plan, Some(index), "forward-repeat 只能使用 yes/no".into());
                }
                if let Some(v) = sound.attributes.get("dacapo") {
                    if v != "yes" {
                        diagnose(plan, Some(index), "dacapo 只能使用 yes".into());
                    }
                }
                if let Some(v) = sound.attributes.get("fine") {
                    if v != "yes" {
                        if let Err(message) = numeric_fine_tail(part, index, offset, v) {
                            diagnose(plan, Some(index), message);
                        }
                    }
                }
                for kind in ["segno", "coda"] {
                    if let Some(label) = sound.attributes.get(kind) {
                        let point = if offset == m.duration {
                            (index + 1, ScoreTime::default())
                        } else {
                            (index, offset)
                        };
                        if targets
                            .insert((kind.into(), label.clone()), point)
                            .is_some_and(|old| old != point)
                        {
                            diagnose(
                                plan,
                                Some(index),
                                format!("{kind} 目标 {label} 在多个位置重复，无法确定跳转"),
                            );
                        }
                    }
                }
                marks[index].push(Navigation {
                    offset,
                    attributes: sound.attributes.clone(),
                    passes,
                });
            }
        }
        marks[index].sort_by(|a, b| {
            a.offset
                .cmp(&b.offset)
                .then(a.attributes.cmp(&b.attributes))
        });
        marks[index].dedup_by(|a, b| a.offset == b.offset && a.attributes == b.attributes);
    }
    // Printed navigation is executable only when the return, destination and
    // cut point each have one position. The written part remains untouched.
    let mut fine_positions: BTreeSet<(usize, ScoreTime)> = text_fines.iter().copied().collect();
    let mut dc_positions = BTreeSet::new();
    let mut ds_positions = BTreeMap::new();
    let mut has_coda_jump = false;
    for (index, local) in marks.iter().enumerate() {
        for mark in local {
            if mark.attributes.contains_key("fine") {
                fine_positions.insert((index, mark.offset));
            }
            if mark.attributes.contains_key("dacapo") {
                dc_positions.insert((index, mark.offset));
            }
            if let Some(label) = mark.attributes.get("dalsegno") {
                ds_positions.insert((index, mark.offset), label.clone());
            }
            has_coda_jump |= mark.attributes.contains_key("tocoda");
        }
    }
    let printed_navigation =
        !text_fines.is_empty() || !text_da_capos.is_empty() || !text_dal_segnos.is_empty();
    if printed_navigation && fine_positions.len() != 1 {
        diagnose(
            plan,
            text_da_capos
                .first()
                .or(text_dal_segnos.first())
                .or(text_fines.first())
                .map(|point| point.0),
            "al Fine 需要唯一且位置明确的 Fine 终点".into(),
        );
    }
    if printed_navigation {
        if dc_positions.len() + ds_positions.len() > 1
            || text_da_capos.len() + text_dal_segnos.len() > 1
            || has_coda_jump
            || !text_da_capos.is_empty() && !ds_positions.is_empty()
            || !text_dal_segnos.is_empty() && !dc_positions.is_empty()
        {
            diagnose(
                plan,
                text_da_capos
                    .first()
                    .or(text_dal_segnos.first())
                    .or(text_fines.first())
                    .map(|p| p.0),
                "文字 al Fine 与其他跳转冲突，无法确定演奏顺序".into(),
            );
        }
        if dc_positions.is_empty()
            && ds_positions.is_empty()
            && text_da_capos.is_empty()
            && text_dal_segnos.is_empty()
        {
            diagnose(
                plan,
                text_fines.first().map(|point| point.0),
                "文字 Fine 没有明确的 D.C./D.S. al Fine 返回指令，无法确定终点".into(),
            );
        }
        for &(index, offset) in &text_da_capos {
            if offset < ScoreTime::default() || offset > part.measures[index].duration {
                diagnose(
                    plan,
                    Some(index),
                    "文字 D.C. al Fine 的位置超出本小节".into(),
                );
                continue;
            }
            if !dc_positions.contains(&(index, offset)) {
                if !dc_positions.is_empty() {
                    diagnose(
                        plan,
                        Some(index),
                        "文字 D.C. al Fine 与 sound dacapo 不在同一位置".into(),
                    );
                } else {
                    marks[index].push(Navigation {
                        offset,
                        attributes: BTreeMap::from([("dacapo".into(), "yes".into())]),
                        passes: None,
                    });
                    dc_positions.insert((index, offset));
                }
            }
        }
        let printed_segno_positions: BTreeSet<_> = printed_segnos.iter().copied().collect();
        if !text_dal_segnos.is_empty() || !text_fines.is_empty() && !ds_positions.is_empty() {
            if printed_segno_positions.len() > 1 {
                diagnose(
                    plan,
                    printed_segnos.first().map(|p| p.0),
                    "印刷 Segno 在多个位置出现，无法确定 D.S. 目标".into(),
                );
            }
            for &(index, offset) in &text_dal_segnos {
                if offset < ScoreTime::default() || offset > part.measures[index].duration {
                    diagnose(
                        plan,
                        Some(index),
                        "文字 D.S. al Fine 的位置超出本小节".into(),
                    );
                    continue;
                }
                if ds_positions.contains_key(&(index, offset)) {
                    continue;
                }
                if !ds_positions.is_empty() {
                    diagnose(
                        plan,
                        Some(index),
                        "文字 D.S. al Fine 与 sound dalsegno 不在同一位置".into(),
                    );
                    continue;
                }
                let sound_targets: Vec<_> = targets
                    .iter()
                    .filter(|((kind, _), _)| kind == "segno")
                    .map(|((_, label), point)| (label.clone(), *point))
                    .collect();
                let chosen = if sound_targets.len() == 1 {
                    Some(sound_targets[0].clone())
                } else if sound_targets.is_empty() && printed_segno_positions.len() == 1 {
                    Some((
                        "__printed_segno__".to_string(),
                        *printed_segno_positions.iter().next().unwrap(),
                    ))
                } else {
                    None
                };
                let Some((label, point)) = chosen else {
                    diagnose(
                        plan,
                        Some(index),
                        "D.S. al Fine 需要唯一且位置明确的 Segno 目标".into(),
                    );
                    continue;
                };
                if !printed_segno_positions.is_empty() && !printed_segno_positions.contains(&point)
                {
                    diagnose(
                        plan,
                        Some(index),
                        "印刷 Segno 与 sound segno 指向不同位置".into(),
                    );
                    continue;
                }
                targets.insert(("segno".into(), label.clone()), point);
                marks[index].push(Navigation {
                    offset,
                    attributes: BTreeMap::from([("dalsegno".into(), label.clone())]),
                    passes: None,
                });
                ds_positions.insert((index, offset), label);
            }
            // An explicit dalsegno may name the sole printed sign even when
            // the score does not carry a separate sound segno destination.
            if let Some((&command, label)) = ds_positions.iter().next() {
                if !targets.contains_key(&("segno".into(), label.clone()))
                    && printed_segno_positions.len() == 1
                {
                    targets.insert(
                        ("segno".into(), label.clone()),
                        *printed_segno_positions.iter().next().unwrap(),
                    );
                }
                if let (Some(&printed), Some(&target)) = (
                    printed_segno_positions.iter().next(),
                    targets.get(&("segno".into(), label.clone())),
                ) {
                    if printed != target {
                        diagnose(
                            plan,
                            Some(command.0),
                            "印刷 Segno 与 sound segno 指向不同位置".into(),
                        );
                    }
                }
            }
        }
        if let Some(&fine) = fine_positions.iter().next() {
            if dc_positions.iter().any(|&dc| fine >= dc)
                || ds_positions.keys().any(|&ds| fine >= ds)
            {
                diagnose(plan, Some(fine.0), "Fine 必须位于返回指令之前".into());
            }
            if let Some((_, label)) = ds_positions.iter().next() {
                if let Some(&segno) = targets.get(&("segno".into(), label.clone())) {
                    if fine <= segno {
                        diagnose(plan, Some(fine.0), "Fine 必须位于 Segno 之后".into());
                    }
                }
            }
        }
        for &(index, offset) in &text_fines {
            if offset < ScoreTime::default() || offset > part.measures[index].duration {
                diagnose(plan, Some(index), "文字 Fine 的位置超出本小节".into());
                continue;
            }
            if !marks[index]
                .iter()
                .any(|mark| mark.offset == offset && mark.attributes.contains_key("fine"))
            {
                marks[index].push(Navigation {
                    offset,
                    attributes: BTreeMap::from([("fine".into(), "yes".into())]),
                    passes: None,
                });
            }
        }
        for local in &mut marks {
            local.sort_by(|a, b| {
                a.offset
                    .cmp(&b.offset)
                    .then(a.attributes.cmp(&b.attributes))
            });
            local.dedup_by(|a, b| a.offset == b.offset && a.attributes == b.attributes);
        }
    }
    for (index, local) in marks.iter().enumerate() {
        for n in local {
            for (command, target) in [("dalsegno", "segno"), ("tocoda", "coda")] {
                if let Some(label) = n.attributes.get(command) {
                    if !targets.contains_key(&(target.into(), label.clone())) {
                        diagnose(
                            plan,
                            Some(index),
                            format!("{command} 找不到对应的 {target} 目标 {label}"),
                        );
                    }
                }
            }
        }
    }
    (marks, targets)
}

fn return_repeat_pass(part: &Part, index: usize) -> u16 {
    part.measures[index..]
        .iter()
        .flat_map(|m| &m.barlines)
        .filter_map(|b| b.repeat.as_ref())
        .find(|r| r.direction == RepeatDirection::Backward)
        .map_or(2, |r| {
            if r.after_jump == Some(true) {
                1
            } else {
                r.times.unwrap_or(2).max(1)
            }
        })
}

/// Expands common forward/backward repeats and numbered endings into performed
/// measure visits. Unsupported or malformed navigation is diagnosed, and the
/// visit cap makes every result finite.
pub fn build_playback_plan(part: &Part, limits: PlaybackLimits) -> PlaybackPlan {
    let mut plan = PlaybackPlan {
        visits: Vec::new(),
        diagnostics: Vec::new(),
        complete: true,
    };
    if part.measures.is_empty() {
        return plan;
    }

    let endings = ending_membership(part, &mut plan);
    let (navigation, targets) = navigation(part, &mut plan);
    let forward = forward_boundaries(part, &navigation, &mut plan);
    if !plan.complete
        && !plan.diagnostics.is_empty()
        && navigation.iter().flatten().next().is_some()
    {
        return plan;
    }
    let mut source_start = ScoreTime::default();
    let mut jump_pass = 1u16;
    let mut used = BTreeSet::new();
    let mut steps = 0usize;
    let mut cursor = 0_usize;
    let mut repeat_frames: Vec<RepeatFrame> = Vec::new();
    let mut last_repeat_pass = 1_u16;
    let mut ending_pass = None;
    let mut performed_cursor = ScoreTime::default();

    while cursor < part.measures.len() {
        steps = steps.saturating_add(1);
        if steps > limits.max_visits.saturating_mul(4).saturating_add(16) {
            diagnose(&mut plan, Some(cursor), "跳转次数超过演奏展开上限".into());
            break;
        }
        if plan.visits.len() >= limits.max_visits {
            diagnose(
                &mut plan,
                Some(cursor),
                format!(
                    "演奏顺序达到设置的 {} 次小节经过上限，后续小节尚未展开",
                    limits.max_visits
                ),
            );
            break;
        }

        let measure = &part.measures[cursor];
        if forward[cursor].contains(&source_start)
            && repeat_frames
                .last()
                .is_none_or(|frame| frame.start != (cursor, source_start))
        {
            let pass = if jump_pass > 1 {
                return_repeat_pass(part, cursor)
            } else {
                1
            };
            repeat_frames.push(RepeatFrame {
                start: (cursor, source_start),
                pass,
            });
            ending_pass = None;
        }
        let frame_pass = repeat_frames
            .last()
            .map_or(last_repeat_pass, |frame| frame.pass);
        let repeat_pass = if endings[cursor].is_some() {
            ending_pass.unwrap_or(frame_pass)
        } else {
            ending_pass = None;
            frame_pass
        };
        let next_forward = forward[cursor]
            .iter()
            .copied()
            .find(|offset| *offset > source_start);
        let included = endings[cursor]
            .as_ref()
            .is_none_or(|passes| passes.contains(&repeat_pass));
        // A return passage skips ordinary repeats unless after-jump=yes says otherwise.
        // Numbered endings use their final pass on a return without repeats.
        let included = if jump_pass > 1 {
            endings[cursor].as_ref().is_none_or(|passes| {
                passes.contains(&if !repeat_frames.is_empty() {
                    repeat_pass
                } else {
                    repeat_pass.max(2)
                })
            })
        } else {
            included
        };
        let mut action = None;
        if included {
            for (mark_index, n) in navigation[cursor].iter().enumerate() {
                if n.offset < source_start || next_forward.is_some_and(|end| n.offset >= end) {
                    continue;
                }
                let pass = repeat_pass.max(jump_pass);
                let applies = n.passes.as_ref().is_none_or(|p| p.contains(&pass));
                let mut commands = vec![];
                for kind in ["fine", "tocoda", "dacapo", "dalsegno"] {
                    let Some(value) = n.attributes.get(kind) else {
                        continue;
                    };
                    let default_applies = match kind {
                        "fine" | "tocoda" => jump_pass > 1,
                        _ => jump_pass == 1,
                    };
                    let eligible = if n.passes.is_some() {
                        applies
                    } else {
                        default_applies
                    };
                    let key = (
                        cursor,
                        mark_index,
                        kind.to_string(),
                        if n.passes.is_some() { pass } else { 0 },
                    );
                    if eligible && !used.contains(&key) {
                        commands.push((kind, value.clone(), key));
                    }
                }
                if commands.is_empty() {
                    continue;
                }
                if commands.len() > 1
                    || action
                        .as_ref()
                        .is_some_and(|(_, offset, _, _, _)| *offset == n.offset)
                {
                    diagnose(
                        &mut plan,
                        Some(cursor),
                        "同一位置有相互冲突的跳转指令".into(),
                    );
                    return plan;
                }
                if action.is_some() {
                    break;
                }
                let (kind, label, key) = commands.pop().unwrap();
                action = Some((kind, n.offset, label, key, n.passes.is_some()));
            }
            let fine_tail = action.as_ref().and_then(|(kind, offset, value, _, _)| {
                (kind == &"fine" && value != "yes")
                    .then(|| numeric_fine_tail(part, cursor, *offset, value))
            });
            let fine_tail = match fine_tail {
                Some(Err(message)) => {
                    diagnose(&mut plan, Some(cursor), message);
                    return plan;
                }
                Some(Ok(tail)) if tail.onset < source_start => {
                    diagnose(
                        &mut plan,
                        Some(cursor),
                        "数值 Fine 的末音跨过当前反复起点，无法确定实际演奏时值".into(),
                    );
                    return plan;
                }
                Some(Ok(tail)) => Some(tail),
                None => None,
            };
            let source_end = action
                .as_ref()
                .map_or(measure.duration, |(_, offset, _, _, _)| *offset)
                .min(next_forward.unwrap_or(measure.duration));
            if source_end > source_start {
                let Ok(occurrence_ordinal) = u32::try_from(plan.visits.len()) else {
                    diagnose(
                        &mut plan,
                        Some(cursor),
                        "演奏经过次数超过可记录的上限".into(),
                    );
                    break;
                };
                let visit = MeasureVisit {
                    part_id: part.id.clone(),
                    source_measure_ordinal: cursor as u32,
                    occurrence_ordinal,
                    repeat_pass: repeat_pass.max(jump_pass),
                    performed_start: performed_cursor,
                    source_start,
                    source_end,
                    fine_tail,
                };
                performed_cursor = visit.performed_end();
                plan.visits.push(visit);
            }
        }
        if let Some(next_forward) = next_forward {
            if action.is_none() {
                source_start = next_forward;
                continue;
            }
        }
        if let Some((kind, offset, label, key, _)) = action {
            used.insert(key);
            if kind == "fine" {
                break;
            }
            let target = match kind {
                "dacapo" => (0, ScoreTime::default()),
                "dalsegno" => targets[&("segno".into(), label)],
                "tocoda" => targets[&("coda".into(), label)],
                _ => unreachable!(),
            };
            let from = (cursor, offset);
            if (kind == "tocoda" && target <= from) || (kind != "tocoda" && target >= from) {
                diagnose(
                    &mut plan,
                    Some(cursor),
                    "跳转目标方向无效：D.C./D.S. 须向前返回，Coda 须向后继续".into(),
                );
                break;
            }
            if kind != "tocoda" {
                jump_pass = jump_pass.saturating_add(1);
            }
            repeat_frames.clear();
            ending_pass = None;
            last_repeat_pass = if jump_pass > 1 && target.0 < part.measures.len() {
                return_repeat_pass(part, target.0)
            } else {
                1
            };
            cursor = target.0;
            source_start = target.1;
            continue;
        }
        source_start = ScoreTime::default();

        let backward = measure
            .barlines
            .iter()
            .filter_map(|barline| barline.repeat.as_ref())
            .find(|repeat| repeat.direction == RepeatDirection::Backward);
        if let Some(backward) = backward.filter(|r| jump_pass == 1 || r.after_jump == Some(true)) {
            if repeat_frames.is_empty() {
                repeat_frames.push(RepeatFrame {
                    start: (0, ScoreTime::default()),
                    pass: if jump_pass > 1 { 1 } else { last_repeat_pass },
                });
            }
            let requested_passes = backward.times.unwrap_or(2).max(1);
            let total_passes = requested_passes.min(limits.max_repeat_passes.max(1));
            if requested_passes > total_passes {
                diagnose(
                    &mut plan,
                    Some(cursor),
                    format!("反复记号要求演奏 {requested_passes} 遍，当前上限为 {total_passes} 遍"),
                );
            }
            let frame = repeat_frames.last_mut().expect("implicit repeat frame");
            if frame.pass < total_passes {
                frame.pass += 1;
                cursor = frame.start.0;
                source_start = frame.start.1;
                continue;
            }
            last_repeat_pass = frame.pass;
            ending_pass = Some(frame.pass);
            repeat_frames.pop();
        } else if backward.is_some() && jump_pass > 1 {
            repeat_frames.clear();
            ending_pass = None;
            last_repeat_pass = return_repeat_pass(part, cursor);
        }

        for repeat in measure
            .barlines
            .iter()
            .filter_map(|barline| barline.repeat.as_ref())
            .filter(|repeat| matches!(repeat.direction, RepeatDirection::Other(_)))
        {
            diagnose(
                &mut plan,
                Some(cursor),
                format!(
                    "无法识别反复记号方向 {:?}，这一段尚未展开",
                    repeat.direction
                ),
            );
        }
        cursor += 1;
    }

    if cursor >= part.measures.len() && !repeat_frames.is_empty() {
        for frame in &repeat_frames {
            diagnose(
                &mut plan,
                Some(frame.start.0),
                "正向反复记号没有对应的反向反复结束位置".into(),
            );
        }
    }

    plan
}

/// Expands note and direction events through a previously built playback plan.
/// Every occurrence retains its stable source ID and adds the measure-visit ID.
pub fn expand_event_occurrences(part: &Part, plan: &PlaybackPlan) -> Vec<PerformedScoreEvent> {
    let mut result = Vec::new();
    for visit in &plan.visits {
        if visit.part_id != part.id {
            continue;
        }
        let Some(measure) = part.measures.get(visit.source_measure_ordinal as usize) else {
            continue;
        };
        for event in &measure.events {
            let (source_id, onset, duration) = match event {
                ScoreEvent::Note(note) => (&note.id, note.onset, Some(note.duration)),
                ScoreEvent::Direction(direction) => (&direction.id, direction.onset, None),
            };
            let relative_onset = onset.subtract(measure.start);
            if relative_onset >= visit.source_end
                || duration.map_or(relative_onset < visit.source_start, |d| {
                    relative_onset.add(d) <= visit.source_start
                })
            {
                continue;
            }
            let clipped_onset = relative_onset.max(visit.source_start);
            let duration = duration.map(|d| {
                if let Some(tail) = visit
                    .fine_tail
                    .filter(|tail| relative_onset == tail.onset && d == tail.written_duration)
                {
                    tail.actual_duration
                } else {
                    relative_onset
                        .add(d)
                        .min(visit.source_end)
                        .subtract(clipped_onset)
                }
            });
            result.push(PerformedScoreEvent {
                id: ScoreEventOccurrenceId {
                    source_id: source_id.clone(),
                    measure_occurrence_ordinal: visit.occurrence_ordinal,
                },
                onset: visit
                    .performed_start
                    .add(clipped_onset.subtract(visit.source_start)),
                duration,
            });
        }
    }
    result
}

fn ending_membership(part: &Part, plan: &mut PlaybackPlan) -> Vec<Option<Vec<u16>>> {
    let mut result = vec![None; part.measures.len()];
    let mut active: Option<Vec<u16>> = None;

    for (measure_index, measure) in part.measures.iter().enumerate() {
        for ending in measure
            .barlines
            .iter()
            .flat_map(|barline| &barline.endings)
            .filter(|ending| ending.kind == EndingType::Start)
        {
            if active.is_some() {
                diagnose(
                    plan,
                    Some(measure_index),
                    "两个不同遍次的结尾范围发生重叠，无法确定演奏顺序".into(),
                );
            }
            if ending.passes.is_empty() {
                diagnose(
                    plan,
                    Some(measure_index),
                    format!(
                        "结尾标记 {:?} 没有可识别的演奏遍次，无法筛选这一小节",
                        ending.number
                    ),
                );
                active = None;
            } else {
                active = Some(ending.passes.clone());
            }
        }

        result[measure_index] = active.clone();

        for ending in measure.barlines.iter().flat_map(|barline| &barline.endings) {
            match ending.kind {
                EndingType::Stop | EndingType::Discontinue => {
                    if active.is_none() {
                        diagnose(
                            plan,
                            Some(measure_index),
                            "结尾结束记号缺少对应的起始记号".into(),
                        );
                    }
                    active = None;
                }
                EndingType::Other(ref value) => diagnose(
                    plan,
                    Some(measure_index),
                    format!("无法识别结尾记号类型 {value:?}，这一段尚未展开"),
                ),
                EndingType::Start => {}
            }
        }
    }

    if active.is_some() {
        diagnose(
            plan,
            part.measures.len().checked_sub(1),
            "结尾范围延伸到曲目末尾，却没有结束记号".into(),
        );
    }
    result
}

fn diagnose(plan: &mut PlaybackPlan, measure: Option<usize>, message: String) {
    plan.complete = false;
    plan.diagnostics.push(PlaybackDiagnostic {
        measure_ordinal: measure.and_then(|value| u32::try_from(value).ok()),
        message,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::musicxml::import_musicxml;

    fn part(measures: &str) -> Part {
        import_musicxml(
            format!(
                r#"<score-partwise>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1">{measures}</part></score-partwise>"#
            )
            .as_bytes(),
        )
        .unwrap()
        .parts
        .remove(0)
    }

    fn measure(number: u8, left: &str, right: &str) -> String {
        format!(
            r#"<measure number="{number}">{left}
<note><rest/><duration>1</duration></note>{right}</measure>"#
        )
    }

    fn ordinals(plan: &PlaybackPlan) -> Vec<u32> {
        plan.visits
            .iter()
            .map(|visit| visit.source_measure_ordinal)
            .collect()
    }

    #[test]
    fn numeric_fine_changes_only_the_returned_terminal_occurrence() {
        for (fine, expected) in [("0.5", ScoreTime::new(1, 2)), ("2", ScoreTime::new(2, 1))] {
            let source = format!(
                r#"<measure number="1"><attributes><divisions>4</divisions></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note></measure>
                <measure number="2"><note><pitch><step>D</step><octave>4</octave></pitch><duration>4</duration></note><sound fine="{fine}"/></measure>
                <measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note><sound dacapo="yes"/></measure>"#
            );
            let part = part(&source);
            let plan = build_playback_plan(&part, PlaybackLimits::default());
            assert!(plan.complete, "{fine}: {:?}", plan.diagnostics);
            assert_eq!(ordinals(&plan), [0, 1, 2, 0, 1]);
            assert_eq!(plan.visits[1].fine_tail, None);
            assert_eq!(plan.visits[4].fine_tail.unwrap().actual_duration, expected);
            assert_eq!(
                plan.visits[4].performed_end(),
                ScoreTime::new(4, 1).add(expected)
            );
            let notes: Vec<_> = expand_event_occurrences(&part, &plan)
                .into_iter()
                .filter(|event| event.id.source_id.kind == crate::musicxml::ScoreEventKind::Note)
                .collect();
            assert_eq!(notes[1].duration, Some(ScoreTime::new(1, 1)));
            assert_eq!(notes[4].duration, Some(expected));
            assert_ne!(notes[1].id, notes[4].id);
            assert_eq!(notes[1].id.source_id, notes[4].id.source_id);
        }
    }

    #[test]
    fn numeric_fine_three_is_three_quarter_notes_even_with_480_divisions() {
        // Mirrors the divisions=480, half-note ending, fine="3" relationship
        // in musescore/nwc2musicxml TestFiles/TestDCalFine.xml.
        let source = r#"<measure number="1"><attributes><divisions>480</divisions></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>480</duration></note></measure>
        <measure number="2"><note><pitch><step>D</step><octave>4</octave></pitch><duration>960</duration></note><direction><direction-type><words>Fine</words></direction-type><sound fine="3"/></direction></measure>
        <measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>480</duration></note><sound dacapo="yes"/></measure>"#;
        let part = part(source);
        let plan = build_playback_plan(&part, PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(
            plan.visits
                .last()
                .unwrap()
                .fine_tail
                .unwrap()
                .actual_duration,
            ScoreTime::new(3, 1)
        );
        let occurrences = expand_event_occurrences(&part, &plan);
        assert_eq!(
            occurrences.iter().filter_map(|e| e.duration).last(),
            Some(ScoreTime::new(3, 1))
        );
    }

    #[test]
    fn numeric_fine_accepts_a_terminal_rest_and_reports_ambiguous_voices() {
        let base = r#"<measure number="1"><attributes><divisions>4</divisions></attributes><note><rest/><duration>4</duration></note><sound fine="2"/></measure>
        <measure number="2"><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note><sound dacapo="yes"/></measure>"#;
        let plan = build_playback_plan(&part(base), PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(
            plan.visits.last().unwrap().performed_end(),
            ScoreTime::new(4, 1)
        );

        let ambiguous = base.replace(
            r#"<sound fine="2"/>"#,
            r#"<backup><duration>4</duration></backup><note><rest/><duration>4</duration><voice>2</voice></note><sound fine="2"/>"#,
        );
        let plan = build_playback_plan(&part(&ambiguous), PlaybackLimits::default());
        assert!(!plan.complete);
        assert!(
            plan.diagnostics
                .iter()
                .any(|d| d.message.contains("多个声部"))
        );

        let printed = base.replace(
            r#"<sound fine="2"/>"#,
            r#"<direction><direction-type><words>Fine</words></direction-type><sound fine="2"/></direction>"#,
        );
        let plan = build_playback_plan(&part(&printed), PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(
            plan.visits
                .last()
                .unwrap()
                .fine_tail
                .unwrap()
                .actual_duration,
            ScoreTime::new(2, 1)
        );
    }

    #[test]
    fn expands_a_default_two_pass_repeat() {
        let source = [
            measure(
                1,
                r#"<barline location="left"><repeat direction="forward"/></barline>"#,
                "",
            ),
            measure(
                2,
                "",
                r#"<barline><repeat direction="backward"/></barline>"#,
            ),
            measure(3, "", ""),
        ]
        .join("");
        let plan = build_playback_plan(&part(&source), PlaybackLimits::default());

        assert_eq!(ordinals(&plan), [0, 1, 0, 1, 2]);
        assert_eq!(
            plan.visits
                .iter()
                .map(|visit| visit.repeat_pass)
                .collect::<Vec<_>>(),
            [1, 1, 2, 2, 2]
        );
        assert_eq!(
            plan.visits
                .iter()
                .map(|visit| visit.performed_start)
                .collect::<Vec<_>>(),
            [
                ScoreTime::new(0, 1),
                ScoreTime::new(1, 1),
                ScoreTime::new(2, 1),
                ScoreTime::new(3, 1),
                ScoreTime::new(4, 1),
            ]
        );
        assert!(plan.complete);
    }

    #[test]
    fn nested_repeats_restart_the_inner_pass_on_each_outer_pass() {
        let source = [
            measure(
                1,
                r#"<barline location="left"><repeat direction="forward"/></barline>"#,
                "",
            ),
            measure(
                2,
                r#"<barline location="left"><repeat direction="forward"/></barline>"#,
                "",
            ),
            measure(
                3,
                "",
                r#"<barline><repeat direction="backward"/></barline>"#,
            ),
            measure(
                4,
                "",
                r#"<barline><repeat direction="backward"/></barline>"#,
            ),
            measure(5, "", ""),
        ]
        .join("");
        let plan = build_playback_plan(&part(&source), PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(ordinals(&plan), [0, 1, 2, 1, 2, 3, 0, 1, 2, 1, 2, 3, 4]);
        assert_eq!(
            plan.visits
                .iter()
                .map(|visit| visit.repeat_pass)
                .collect::<Vec<_>>(),
            [1, 1, 1, 2, 2, 1, 2, 1, 1, 2, 2, 2, 2]
        );
    }

    #[test]
    fn nested_repeats_apply_the_inner_numbered_endings_before_resuming_outer_pass() {
        let source = [
            measure(1, r#"<barline location="left"><repeat direction="forward"/></barline>"#, ""),
            measure(2, r#"<barline location="left"><repeat direction="forward"/></barline>"#, ""),
            measure(3, r#"<barline location="left"><ending number="1" type="start"/></barline>"#,
                r#"<barline><ending number="1" type="stop"/><repeat direction="backward"/></barline>"#),
            measure(4, r#"<barline location="left"><ending number="2" type="start"/></barline>"#,
                r#"<barline><ending number="2" type="stop"/></barline>"#),
            measure(5, "", r#"<barline><repeat direction="backward"/></barline>"#),
            measure(6, "", ""),
        ]
        .join("");
        let plan = build_playback_plan(&part(&source), PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(ordinals(&plan), [0, 1, 2, 1, 3, 4, 0, 1, 2, 1, 3, 4, 5]);
    }

    #[test]
    fn a_backward_repeat_without_a_forward_mark_starts_at_the_first_measure() {
        let source = [
            measure(1, "", ""),
            measure(
                2,
                "",
                r#"<barline><repeat direction="backward"/></barline>"#,
            ),
            measure(3, "", ""),
        ]
        .join("");
        let plan = build_playback_plan(&part(&source), PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(ordinals(&plan), [0, 1, 0, 1, 2]);
    }

    #[test]
    fn an_in_measure_forward_repeat_restarts_at_its_actual_offset() {
        let source = [
            r#"<measure number="1"><note><rest/><duration>1</duration></note><sound forward-repeat="yes"/><note><rest/><duration>1</duration></note></measure>"#.to_string(),
            measure(2, "", r#"<barline><repeat direction="backward"/></barline>"#),
        ]
        .join("");
        let plan = build_playback_plan(&part(&source), PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(ordinals(&plan), [0, 0, 1, 0, 1]);
        assert_eq!(
            plan.visits
                .iter()
                .map(|v| (v.source_start, v.source_end))
                .collect::<Vec<_>>(),
            [
                (ScoreTime::new(0, 1), ScoreTime::new(1, 1)),
                (ScoreTime::new(1, 1), ScoreTime::new(2, 1)),
                (ScoreTime::new(0, 1), ScoreTime::new(1, 1)),
                (ScoreTime::new(1, 1), ScoreTime::new(2, 1)),
                (ScoreTime::new(0, 1), ScoreTime::new(1, 1)),
            ]
        );
    }

    #[test]
    fn unclosed_outer_repeat_remains_incomplete_after_an_inner_backward() {
        let source = [
            measure(
                1,
                r#"<barline location="left"><repeat direction="forward"/></barline>"#,
                "",
            ),
            measure(
                2,
                r#"<barline location="left"><repeat direction="forward"/></barline>"#,
                "",
            ),
            measure(
                3,
                "",
                r#"<barline><repeat direction="backward"/></barline>"#,
            ),
        ]
        .join("");
        let plan = build_playback_plan(&part(&source), PlaybackLimits::default());
        assert_eq!(ordinals(&plan), [0, 1, 2, 1, 2]);
        assert!(!plan.complete);
        assert!(
            plan.diagnostics
                .iter()
                .any(|d| d.measure_ordinal == Some(0))
        );
    }

    #[test]
    fn multiple_backward_marks_in_one_measure_are_diagnosed_as_ambiguous() {
        let source = measure(
            1,
            "",
            r#"<barline><repeat direction="backward"/></barline><barline><repeat direction="backward"/></barline>"#,
        );
        let plan = build_playback_plan(&part(&source), PlaybackLimits::default());
        assert!(!plan.complete);
        assert!(
            plan.diagnostics
                .iter()
                .any(|d| d.message.contains("多个反向反复"))
        );
    }

    #[test]
    fn selects_first_and_second_endings_without_losing_repeat_control() {
        let source = [
            measure(
                1,
                r#"<barline location="left"><repeat direction="forward"/></barline>"#,
                "",
            ),
            measure(2, "", ""),
            measure(
                3,
                r#"<barline location="left"><ending number="1" type="start"/></barline>"#,
                r#"<barline><ending number="1" type="stop"/><repeat direction="backward"/></barline>"#,
            ),
            measure(
                4,
                r#"<barline location="left"><ending number="2" type="start"/></barline>"#,
                r#"<barline><ending number="2" type="stop"/></barline>"#,
            ),
            measure(5, "", ""),
        ]
        .join("");
        let plan = build_playback_plan(&part(&source), PlaybackLimits::default());

        assert_eq!(ordinals(&plan), [0, 1, 2, 0, 1, 3, 4]);
        assert_eq!(plan.visits[5].repeat_pass, 2);
        assert!(plan.complete);
    }

    #[test]
    fn caps_repeat_counts_and_visit_growth_with_diagnostics() {
        let source = measure(
            1,
            "",
            r#"<barline><repeat direction="backward" times="99"/></barline>"#,
        );
        let plan = build_playback_plan(
            &part(&source),
            PlaybackLimits {
                max_visits: 3,
                max_repeat_passes: 4,
            },
        );

        assert_eq!(ordinals(&plan), [0, 0, 0]);
        assert!(!plan.complete);
        assert!(
            plan.diagnostics
                .iter()
                .any(|item| item.message.contains("上限为 4 遍"))
        );
        assert!(
            plan.diagnostics
                .iter()
                .any(|item| item.message.contains("3 次小节经过上限"))
        );
    }

    #[test]
    fn malformed_endings_remain_playable_but_diagnostic() {
        let source = measure(
            1,
            r#"<barline location="left"><ending number="finale" type="start"/></barline>"#,
            "",
        );
        let plan = build_playback_plan(&part(&source), PlaybackLimits::default());

        assert_eq!(ordinals(&plan), [0]);
        assert!(!plan.complete);
        assert_eq!(plan.diagnostics.len(), 1);
    }

    #[test]
    fn repeated_event_occurrences_keep_source_identity_and_gain_new_time() {
        let source = measure(
            1,
            r#"<barline location="left"><repeat direction="forward"/></barline>"#,
            r#"<barline><repeat direction="backward"/></barline>"#,
        );
        let part = part(&source);
        let plan = build_playback_plan(&part, PlaybackLimits::default());
        let events = expand_event_occurrences(&part, &plan);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id.source_id, events[1].id.source_id);
        assert_eq!(events[0].id.measure_occurrence_ordinal, 0);
        assert_eq!(events[1].id.measure_occurrence_ordinal, 1);
        assert_eq!(events[0].onset, ScoreTime::new(0, 1));
        assert_eq!(events[1].onset, ScoreTime::new(1, 1));
        assert_eq!(events[1].duration, Some(ScoreTime::new(1, 1)));
    }
    #[test]
    fn da_capo_fine_and_dal_segno_coda_preserve_written_occurrences() {
        let dc = [measure(1, "", ""), measure(2, "", r#"<sound fine="yes"/>"#),
            measure(3, "", r#"<direction><direction-type><words>D.C. al Fine</words></direction-type><sound dacapo="yes"/></direction>"#)].join("");
        let p = part(&dc);
        let plan = build_playback_plan(&p, PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(ordinals(&plan), [0, 1, 2, 0, 1]);
        let events = expand_event_occurrences(&p, &plan);
        let notes: Vec<_> = events.iter().filter(|e| e.duration.is_some()).collect();
        assert_eq!(notes.len(), 5);
        assert_eq!(notes[0].id.source_id, notes[3].id.source_id);
        assert_ne!(notes[0].id, notes[3].id);
        let ds = [
            measure(1, "", ""),
            measure(2, r#"<sound segno="S"/>"#, ""),
            measure(3, "", r#"<sound tocoda="C"/>"#),
            measure(4, "", ""),
            measure(5, "", r#"<sound dalsegno="S"/>"#),
            measure(6, r#"<sound coda="C"/>"#, ""),
            measure(7, "", ""),
        ]
        .join("");
        let plan = build_playback_plan(&part(&ds), PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(ordinals(&plan), [0, 1, 2, 3, 4, 1, 2, 5, 6]);
    }

    #[test]
    fn printed_fine_closes_a_unique_da_capo_return_at_the_actual_note() {
        for dc in [
            r#"<direction><direction-type><words>D.C. al Fine</words></direction-type></direction>"#,
            r#"<direction><direction-type><words>D.C. al Fine</words></direction-type><sound dacapo="yes"/></direction>"#,
            r#"<sound dacapo="yes"/>"#,
        ] {
            let source = [
                measure(1, "", ""),
                measure(2, "", r#"<direction><direction-type><words>Fine</words></direction-type></direction>"#),
                measure(3, "", dc),
            ].join("");
            let part = part(&source);
            let plan = build_playback_plan(&part, PlaybackLimits::default());
            assert!(plan.complete, "{dc}: {:?}", plan.diagnostics);
            assert_eq!(ordinals(&plan), [0, 1, 2, 0, 1]);
            let notes: Vec<_> = expand_event_occurrences(&part, &plan)
                .into_iter()
                .filter(|event| event.duration.is_some())
                .collect();
            assert_eq!(notes.len(), 5);
            assert_eq!(notes.last().unwrap().id.source_id.measure_ordinal, 1);
            assert_eq!(notes.last().unwrap().onset, ScoreTime::new(4, 1));
        }
    }

    #[test]
    fn printed_dal_segno_fine_uses_the_unique_printed_sign_and_keeps_occurrences() {
        for segno in ["<segno/>", "<segno></segno>"] {
            let source = [
                measure(1, "", ""),
                measure(2, &format!("<direction><direction-type>{segno}</direction-type></direction>"), ""),
                measure(3, "", r#"<direction><direction-type><words>Fine</words></direction-type></direction>"#),
                measure(4, "", ""),
                measure(5, "", r#"<direction><direction-type><words>D.S.</words><words>al Fine</words></direction-type></direction>"#),
            ].join("");
            let part = part(&source);
            let plan = build_playback_plan(&part, PlaybackLimits::default());
            assert!(plan.complete, "{segno}: {:?}", plan.diagnostics);
            assert_eq!(ordinals(&plan), [0, 1, 2, 3, 4, 1, 2]);
            let notes: Vec<_> = expand_event_occurrences(&part, &plan)
                .into_iter()
                .filter(|event| event.duration.is_some())
                .collect();
            assert_eq!(notes[1].id.source_id, notes[5].id.source_id);
            assert_ne!(notes[1].id, notes[5].id);
            assert_eq!(notes[6].onset, ScoreTime::new(6, 1));
        }
    }

    #[test]
    fn printed_dal_segno_fine_respects_explicit_sound_label() {
        for sign in [
            r#"<direction><direction-type><segno/></direction-type><sound segno="A"/></direction>"#,
            r#"<direction><direction-type><segno/></direction-type></direction>"#,
        ] {
            let source = [
                measure(1, "", ""),
                measure(2, sign, ""),
                measure(3, "", r#"<direction><direction-type><words>Fine</words></direction-type></direction>"#),
                measure(4, "", r#"<direction><direction-type><words>D.S. al Fine</words></direction-type><sound dalsegno="A"/></direction>"#),
            ].join("");
            let plan = build_playback_plan(&part(&source), PlaybackLimits::default());
            assert!(plan.complete, "{sign}: {:?}", plan.diagnostics);
            assert_eq!(ordinals(&plan), [0, 1, 2, 3, 1, 2]);
        }
    }

    #[test]
    fn printed_dal_segno_fine_rejects_ambiguous_or_conflicting_targets() {
        let ds = r#"<direction><direction-type><words>D.S. al Fine</words></direction-type></direction>"#;
        let fine = r#"<direction><direction-type><words>Fine</words></direction-type></direction>"#;
        for source in [
            [measure(1, "", ""), measure(2, "", fine), measure(3, "", ds)].join(""),
            [measure(1, r#"<direction><direction-type><segno/></direction-type></direction>"#, ""), measure(2, r#"<direction><direction-type><segno/></direction-type></direction>"#, fine), measure(3, "", ds)].join(""),
            [measure(1, r#"<direction><direction-type><segno/></direction-type><sound segno="A"/></direction>"#, ""), measure(2, r#"<sound segno="B"/>"#, fine), measure(3, "", ds)].join(""),
            [measure(1, r#"<direction><direction-type><segno/></direction-type></direction>"#, ""), measure(2, r#"<sound segno="A"/>"#, fine), measure(3, "", r#"<direction><direction-type><words>D.S. al Fine</words></direction-type><sound dalsegno="A"/></direction>"#)].join(""),
            [measure(1, &format!(r#"<direction><direction-type><segno/></direction-type></direction>{fine}"#), ""), measure(2, "", ds)].join(""),
        ] {
            let plan = build_playback_plan(&part(&source), PlaybackLimits::default());
            assert!(!plan.complete, "{source}");
            assert!(!plan.diagnostics.is_empty());
        }
    }

    #[test]
    fn printed_da_capo_requires_one_unambiguous_fine_before_its_position() {
        let text_dc = r#"<direction><direction-type><words>D.C. al Fine</words></direction-type></direction>"#;
        let fine = r#"<direction><direction-type><words>Fine</words></direction-type></direction>"#;
        for source in [
            [measure(1, "", ""), measure(2, "", text_dc)].join(""),
            [measure(1, "", fine), measure(2, "", fine), measure(3, "", text_dc)].join(""),
            [measure(1, "", ""), measure(2, "", text_dc), measure(3, "", fine)].join(""),
            [measure(1, "", fine), measure(2, "", r#"<sound dalsegno="S"/>"#), measure(3, "", text_dc)].join(""),
            [measure(1, "", fine), measure(2, "", r#"<sound dacapo="yes"/>"#), measure(3, "", text_dc)].join(""),
            [measure(1, "", r#"<sound fine="960"/>"#), measure(2, "", text_dc)].join(""),
            [measure(1, "", fine), measure(2, "", "")].join(""),
            [measure(1, "", r#"<direction><direction-type><words>D.S. al Fine</words></direction-type></direction>"#), measure(2, "", fine)].join(""),
        ] {
            let plan = build_playback_plan(&part(&source), PlaybackLimits::default());
            assert!(!plan.complete, "{source}");
            assert!(!plan.diagnostics.is_empty());
        }
        let split_words = [
            measure(1, "", ""),
            measure(2, "", fine),
            measure(3, "", r#"<direction><direction-type><words>D.C.</words><words>al Fine</words></direction-type></direction>"#),
        ].join("");
        let plan = build_playback_plan(&part(&split_words), PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(ordinals(&plan), [0, 1, 2, 0, 1]);
    }

    #[test]
    fn jumps_respect_sound_offsets_time_only_and_return_repeat_policy() {
        let source = measure(
            1,
            "",
            r#"<sound time-only="2" dacapo="yes"/><barline><repeat direction="backward"/></barline>"#,
        );
        let plan = build_playback_plan(&part(&source), PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(ordinals(&plan), [0, 0, 0]);
        for (attribute, expected) in [
            ("", vec![0, 1, 0, 1, 2, 0, 1]),
            (r#" after-jump="yes""#, vec![0, 1, 0, 1, 2, 0, 1, 0, 1]),
        ] {
            let source = [measure(1, r#"<barline location="left"><repeat direction="forward"/></barline>"#, ""),
                measure(2, "", &format!(r#"<sound fine="yes"/><barline><repeat direction="backward"{attribute}/></barline>"#)),
                measure(3, "", r#"<sound dacapo="yes"/>"#)].join("");
            // Fine is reached before a repeat barline: remove it to test the repeat itself,
            // and place Fine at the following measure's beginning.
            let source = source.replace(r#"<sound fine="yes"/>"#, "").replace(
                r#"<measure number="3">"#,
                r#"<measure number="3"><sound fine="yes"/>"#,
            );
            let plan = build_playback_plan(&part(&source), PlaybackLimits::default());
            assert!(plan.complete, "{:?}", plan.diagnostics);
            assert_eq!(ordinals(&plan), expected);
        }
        let source = [
            measure(
                1,
                "",
                r#"<sound segno="S"><offset>-1</offset></sound><sound fine="yes"/>"#,
            ),
            measure(2, "", r#"<sound dalsegno="S"/>"#),
        ]
        .join("");
        // Offset -1 on a one-quarter measure is its start, exercising standalone
        // sound with non-empty content rather than just self-closing elements.
        let p = part(&source);
        let plan = build_playback_plan(&p, PlaybackLimits::default());
        assert!(plan.complete, "{:?}", plan.diagnostics);
        assert_eq!(ordinals(&plan), [0, 1, 0]);
    }

    #[test]
    fn malformed_navigation_is_bounded_and_reports_the_written_location() {
        for fragment in [
            r#"<sound dalsegno="missing"/>"#,
            r#"<sound dacapo="yes" time-only="0"/>"#,
            r#"<direction><direction-type><words>D.C. al Fine</words></direction-type></direction>"#,
            r#"<sound segno="S"/><sound dalsegno="S" tocoda="C"/><sound coda="C"/>"#,
        ] {
            let plan =
                build_playback_plan(&part(&measure(1, "", fragment)), PlaybackLimits::default());
            assert!(!plan.complete, "{fragment}");
            assert!(!plan.diagnostics.is_empty());
            assert!(
                plan.diagnostics
                    .iter()
                    .all(|d| d.measure_ordinal == Some(0))
            );
        }
        let source = [
            measure(1, r#"<sound segno="S"/>"#, ""),
            measure(2, r#"<sound segno="S"/>"#, r#"<sound dalsegno="S"/>"#),
        ]
        .join("");
        assert!(!build_playback_plan(&part(&source), PlaybackLimits::default()).complete);
    }
}
