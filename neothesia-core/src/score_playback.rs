//! Bounded performed-order planning for notation-neutral scores.
//!
//! The imported [`Part`](crate::musicxml::Part) always remains in written
//! document order. This module produces visits into that immutable source.

use crate::musicxml::{EndingType, Part, RepeatDirection};

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
    let mut cursor = 0_usize;
    let mut active_repeat_start = None;
    let mut repeat_pass = 1_u16;

    while cursor < part.measures.len() {
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
            .filter(|repeat| repeat.direction == RepeatDirection::Forward)
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
                repeat_pass = 1;
            }
        }

        let included = endings[cursor]
            .as_ref()
            .is_none_or(|passes| passes.contains(&repeat_pass));
        if included {
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
                repeat_pass,
            });
        }

        let backward = measure
            .barlines
            .iter()
            .filter_map(|barline| barline.repeat.as_ref())
            .find(|repeat| repeat.direction == RepeatDirection::Backward);
        if let Some(backward) = backward {
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
}
