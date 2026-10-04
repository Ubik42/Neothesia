//! Bounded performed-order planning for notation-neutral scores.
//!
//! The imported [`Part`](crate::musicxml::Part) always remains in written
//! document order. This module produces visits into that immutable source.

use std::collections::{BTreeMap, BTreeSet};

use crate::musicxml::{EndingType, Part, RepeatDirection, ScoreEvent, ScoreEventId, ScoreTime};

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
}

impl MeasureVisit {
    pub fn performed_end(&self) -> ScoreTime {
        self.performed_start
            .add(self.source_end.subtract(self.source_start))
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

fn navigation(
    part: &Part,
    plan: &mut PlaybackPlan,
) -> (
    Vec<Vec<Navigation>>,
    BTreeMap<(String, String), (usize, ScoreTime)>,
) {
    let mut marks = vec![vec![]; part.measures.len()];
    let mut targets = BTreeMap::new();
    for (index, m) in part.measures.iter().enumerate() {
        for e in &m.events {
            let ScoreEvent::Direction(d) = e else {
                continue;
            };
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
                        diagnose(
                            plan,
                            Some(index),
                            "数值 Fine 的最后音延长时值尚未支持，请使用明确时值与 fine=yes".into(),
                        );
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
                if sound
                    .attributes
                    .get("forward-repeat")
                    .is_some_and(|v| v == "yes")
                    && offset != ScoreTime::default()
                {
                    diagnose(plan, Some(index), "小节内的隐式正向反复尚未支持".into());
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
    let mut active_repeat_start = None;
    let mut repeat_pass = 1_u16;
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
                    "playback plan stopped at the configured {}-visit limit",
                    limits.max_visits
                ),
            );
            break;
        }

        let measure = &part.measures[cursor];
        for repeat in measure
            .barlines
            .iter()
            .filter_map(|barline| barline.repeat.as_ref())
            .filter(|repeat| {
                repeat.direction == RepeatDirection::Forward && source_start == ScoreTime::default()
            })
        {
            let _ = repeat;
            if let Some(start) = active_repeat_start {
                if start != cursor {
                    diagnose(
                        &mut plan,
                        Some(cursor),
                        "nested forward repeats are not expanded yet".into(),
                    );
                }
            } else {
                active_repeat_start = Some(cursor);
                repeat_pass = if jump_pass > 1 {
                    return_repeat_pass(part, cursor)
                } else {
                    1
                };
            }
        }

        if navigation[cursor].iter().any(|n| {
            n.offset == ScoreTime::default()
                && n.attributes
                    .get("forward-repeat")
                    .is_some_and(|v| v == "yes")
        }) && active_repeat_start.is_none()
        {
            active_repeat_start = Some(cursor);
            repeat_pass = if jump_pass > 1 {
                return_repeat_pass(part, cursor)
            } else {
                1
            };
        }
        let included = endings[cursor]
            .as_ref()
            .is_none_or(|passes| passes.contains(&repeat_pass));
        // A return passage skips ordinary repeats unless after-jump=yes says otherwise.
        // Numbered endings use their final pass on a return without repeats.
        let included = if jump_pass > 1 {
            endings[cursor].as_ref().is_none_or(|passes| {
                passes.contains(&if active_repeat_start.is_some() {
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
                if n.offset < source_start {
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
            let source_end = action
                .as_ref()
                .map_or(measure.duration, |(_, offset, _, _, _)| *offset);
            if source_end > source_start {
                let Ok(occurrence_ordinal) = u32::try_from(plan.visits.len()) else {
                    diagnose(
                        &mut plan,
                        Some(cursor),
                        "playback occurrence identity exceeded u32".into(),
                    );
                    break;
                };
                plan.visits.push(MeasureVisit {
                    part_id: part.id.clone(),
                    source_measure_ordinal: cursor as u32,
                    occurrence_ordinal,
                    repeat_pass: repeat_pass.max(jump_pass),
                    performed_start: performed_cursor,
                    source_start,
                    source_end,
                });
                performed_cursor = performed_cursor.add(source_end.subtract(source_start));
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
            active_repeat_start = None;
            repeat_pass = if jump_pass > 1 && target.0 < part.measures.len() {
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
            let repeat_start = active_repeat_start.unwrap_or(0);
            let requested_passes = backward.times.unwrap_or(2).max(1);
            let total_passes = requested_passes.min(limits.max_repeat_passes.max(1));
            if requested_passes > total_passes {
                diagnose(
                    &mut plan,
                    Some(cursor),
                    format!("repeat count {requested_passes} was capped at {total_passes} passes"),
                );
            }
            if repeat_pass < total_passes {
                repeat_pass += 1;
                cursor = repeat_start;
                continue;
            }
            active_repeat_start = None;
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
                    "unknown repeat direction {:?} was not expanded",
                    repeat.direction
                ),
            );
        }
        cursor += 1;
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
                relative_onset
                    .add(d)
                    .min(visit.source_end)
                    .subtract(clipped_onset)
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
                    "overlapping numbered endings are not expanded yet".into(),
                );
            }
            if ending.passes.is_empty() {
                diagnose(
                    plan,
                    Some(measure_index),
                    format!(
                        "ending {:?} has no usable numeric pass and remains unfiltered",
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
                            "numbered-ending stop has no active start".into(),
                        );
                    }
                    active = None;
                }
                EndingType::Other(ref value) => diagnose(
                    plan,
                    Some(measure_index),
                    format!("unknown ending type {value:?} was not expanded"),
                ),
                EndingType::Start => {}
            }
        }
    }

    if active.is_some() {
        diagnose(
            plan,
            part.measures.len().checked_sub(1),
            "numbered ending reaches end of part without stop or discontinue".into(),
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
                .any(|item| item.message.contains("capped at 4"))
        );
        assert!(
            plan.diagnostics
                .iter()
                .any(|item| item.message.contains("3-visit limit"))
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
