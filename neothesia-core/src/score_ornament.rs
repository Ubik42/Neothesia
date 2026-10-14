//! Exact, shared playback expansion for supported MusicXML ornaments.

use crate::musicxml::{MordentMark, ScoreTime, TrillMark};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrillStrike {
    pub onset: ScoreTime,
    pub duration: ScoreTime,
    pub pitch: u8,
    pub ordinal: u16,
}

fn add(left: ScoreTime, right: ScoreTime) -> Result<ScoreTime, String> {
    let denominator = left
        .denominator
        .checked_mul(right.denominator)
        .ok_or("颤音时间分母超出范围")?;
    let numerator = left
        .numerator
        .checked_mul(i64::from(right.denominator))
        .and_then(|v| {
            right
                .numerator
                .checked_mul(i64::from(left.denominator))
                .and_then(|r| v.checked_add(r))
        })
        .ok_or("颤音时间超出范围")?;
    Ok(ScoreTime::new(numerator, denominator))
}

fn offset(duration: ScoreTime, percent_num: u32, percent_den: u32) -> Result<ScoreTime, String> {
    let numerator = duration
        .numerator
        .checked_mul(i64::from(percent_num))
        .ok_or("颤音时间超出范围")?;
    let denominator = duration
        .denominator
        .checked_mul(100)
        .and_then(|v| v.checked_mul(percent_den))
        .ok_or("颤音时间分母超出范围")?;
    Ok(ScoreTime::new(numerator, denominator))
}

/// Return one score-time interval for each performed key strike. MusicXML's
/// `beats` counts distinct notes, including the starting note. The default
/// trill has three strikes, with its second and last starts at 12% and 24%
/// of the written duration. Unsupported playback hints fail explicitly.
pub fn expand_trill(
    mark: &TrillMark,
    pitch: u8,
    onset: ScoreTime,
    duration: ScoreTime,
) -> Result<Vec<TrillStrike>, String> {
    if let Some(feature) = mark.unsupported.first() {
        return Err(format!("颤音含暂不支持的标记：{feature}"));
    }
    if duration.numerator <= 0 {
        return Err("颤音主音必须有正时值".into());
    }
    if mark.accelerate.as_deref().is_some_and(|v| v != "no") {
        return Err("颤音 accelerate 目前只支持 no".into());
    }
    if mark.two_note_turn.as_deref().is_some_and(|v| v != "none") {
        return Err("颤音结尾回转暂不支持，请先核对原谱".into());
    }
    let beats = match mark.beats.as_deref() {
        None => 3,
        Some(raw) => raw
            .parse::<u16>()
            .ok()
            .filter(|value| (3..=32).contains(value))
            .ok_or("颤音 beats 目前只支持 3 至 32 的整数；小数及双击颤音须人工校对")?,
    };
    let start_note = mark.start_note.as_deref().unwrap_or("main");
    if !matches!(start_note, "main" | "upper" | "below") {
        return Err(format!("颤音 start-note={start_note} 暂不支持"));
    }
    let step = match mark.trill_step.as_deref().unwrap_or("whole") {
        "whole" => 2i16,
        "half" => 1,
        "unison" => 0,
        value => return Err(format!("颤音 trill-step={value} 暂不支持")),
    };
    let parse_percent = |value: Option<&str>, default: u32, name: &str| -> Result<u32, String> {
        match value {
            None => Ok(default),
            Some(raw) => raw
                .parse::<u32>()
                .ok()
                .filter(|percent| *percent <= 100)
                .ok_or_else(|| format!("颤音 {name} 目前只支持 0 至 100 的整数百分比")),
        }
    };
    let second = parse_percent(mark.second_beat.as_deref(), 12, "second-beat")?;
    let last = parse_percent(mark.last_beat.as_deref(), 24, "last-beat")?;
    if second == 0 || second >= last || last >= 100 {
        return Err("颤音击键落点必须满足 0 < second-beat < last-beat < 100".into());
    }
    let upper = i16::from(pitch) + step;
    let lower = i16::from(pitch) - step;
    if upper > 127 || lower < 0 {
        return Err("颤音相邻音超出 MIDI 音高范围".into());
    }
    let mut starts = Vec::with_capacity(usize::from(beats) + 1);
    starts.push(onset);
    for index in 1..beats {
        let (num, den) = if index == 1 {
            (second, 1)
        } else if index == beats - 1 {
            (last, 1)
        } else {
            let between = u32::from(beats - 2);
            (
                second * between + (last - second) * u32::from(index - 1),
                between,
            )
        };
        starts.push(add(onset, offset(duration, num, den)?)?);
    }
    starts.push(add(onset, duration)?);
    let mut strikes = Vec::with_capacity(usize::from(beats));
    for index in 0..usize::from(beats) {
        let pitch = match (start_note, index % 2) {
            ("main", 0) | ("upper", 1) | ("below", 1) => pitch,
            ("upper", 0) => upper as u8,
            ("below", 0) => lower as u8,
            _ => upper as u8,
        };
        let segment = starts[index + 1].subtract(starts[index]);
        if segment.numerator <= 0 {
            return Err("颤音击键时间重叠，请调整 beats 或落点".into());
        }
        strikes.push(TrillStrike {
            onset: starts[index],
            duration: segment,
            pitch,
            ordinal: index as u16,
        });
    }
    Ok(strikes)
}

/// Expand either MusicXML mordent sign into exact performed key strikes.
/// MusicXML's lined `mordent` uses the lower neighbor; its unlined
/// `inverted-mordent` uses the upper neighbor.
pub fn expand_mordent(
    mark: &MordentMark,
    pitch: u8,
    onset: ScoreTime,
    duration: ScoreTime,
) -> Result<Vec<TrillStrike>, String> {
    if let Some(feature) = mark.unsupported.first() {
        return Err(format!("波音含暂不支持的标记：{feature}"));
    }
    if mark.long.as_deref().is_some_and(|v| v != "no") {
        return Err("波音 long 目前只支持 no".into());
    }
    if mark.approach.is_some() || mark.departure.is_some() {
        return Err("波音 approach/departure 组合装饰音暂不支持".into());
    }
    let neighbor_name = if mark.inverted { "upper" } else { "below" };
    let start = mark.start_note.as_deref().unwrap_or("main");
    if start != "main" && start != neighbor_name {
        return Err(format!("波音 start-note={start} 与此波音方向不符"));
    }
    let trill = TrillMark {
        beats: mark.beats.clone(),
        start_note: Some("main".into()),
        trill_step: mark.trill_step.clone(),
        second_beat: mark.second_beat.clone(),
        last_beat: mark.last_beat.clone(),
        accelerate: mark.accelerate.clone(),
        two_note_turn: mark.two_note_turn.clone(),
        unsupported: Vec::new(),
    };
    // A safe reference pitch lets the shared expander validate timing while
    // the mordent's own direction selects the performed neighbor.
    let mut strikes = expand_trill(&trill, 64, onset, duration)?;
    let step = match mark.trill_step.as_deref().unwrap_or("whole") {
        "whole" => 2i16,
        "half" => 1,
        "unison" => 0,
        _ => unreachable!("expand_trill validated trill-step"),
    };
    let neighbor = i16::from(pitch) + if mark.inverted { step } else { -step };
    if !(0..=127).contains(&neighbor) {
        return Err("波音相邻音超出 MIDI 音高范围".into());
    }
    for strike in &mut strikes {
        let neighbor_attack = (strike.ordinal % 2 == 1) == (start == "main");
        strike.pitch = if neighbor_attack { neighbor as u8 } else { pitch };
    }
    Ok(strikes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_trill_respects_musicxml_landings() {
        let strikes = expand_trill(
            &TrillMark::default(),
            60,
            ScoreTime::new(2, 1),
            ScoreTime::new(1, 1),
        )
        .unwrap();
        assert_eq!(
            strikes.iter().map(|s| s.pitch).collect::<Vec<_>>(),
            [60, 62, 60]
        );
        assert_eq!(
            strikes.iter().map(|s| s.onset).collect::<Vec<_>>(),
            [
                ScoreTime::new(2, 1),
                ScoreTime::new(53, 25),
                ScoreTime::new(56, 25),
            ]
        );
        assert_eq!(strikes.last().unwrap().duration, ScoreTime::new(19, 25));
    }

    #[test]
    fn rejects_unsupported_playback_instead_of_ignoring_it() {
        let mut mark = TrillMark::default();
        mark.accelerate = Some("yes".into());
        assert!(
            expand_trill(&mark, 60, ScoreTime::default(), ScoreTime::new(1, 1))
                .unwrap_err()
                .contains("accelerate")
        );
        mark.accelerate = None;
        mark.beats = Some("3.5".into());
        assert!(
            expand_trill(&mark, 60, ScoreTime::default(), ScoreTime::new(1, 1))
                .unwrap_err()
                .contains("beats")
        );
    }

    #[test]
    fn musicxml_mordent_directions_share_exact_landings() {
        for (inverted, expected) in [(false, [60, 58, 60]), (true, [60, 62, 60])] {
            let mark = MordentMark { inverted, ..Default::default() };
            let strikes = expand_mordent(&mark, 60, ScoreTime::default(), ScoreTime::new(1, 1)).unwrap();
            assert_eq!(strikes.iter().map(|s| s.pitch).collect::<Vec<_>>(), expected);
            assert_eq!(strikes.iter().map(|s| s.onset).collect::<Vec<_>>(), [ScoreTime::default(), ScoreTime::new(3, 25), ScoreTime::new(6, 25)]);
            assert_eq!(strikes.iter().map(|s| s.ordinal).collect::<Vec<_>>(), [0, 1, 2]);
        }
    }

    #[test]
    fn mordent_rejects_compound_playback_and_out_of_range_neighbor() {
        let mut mark = MordentMark::default();
        mark.long = Some("yes".into());
        assert!(expand_mordent(&mark, 60, ScoreTime::default(), ScoreTime::new(1, 1)).unwrap_err().contains("long"));
        mark.long = None;
        mark.approach = Some("above".into());
        assert!(expand_mordent(&mark, 60, ScoreTime::default(), ScoreTime::new(1, 1)).unwrap_err().contains("approach"));
        mark.approach = None;
        assert!(expand_mordent(&mark, 0, ScoreTime::default(), ScoreTime::new(1, 1)).unwrap_err().contains("MIDI"));
    }

    #[test]
    fn mordent_uses_written_playback_attributes() {
        let mark = MordentMark {
            beats: Some("4".into()),
            start_note: Some("below".into()),
            trill_step: Some("half".into()),
            second_beat: Some("25".into()),
            last_beat: Some("75".into()),
            ..Default::default()
        };
        let strikes = expand_mordent(&mark, 60, ScoreTime::default(), ScoreTime::new(1, 1)).unwrap();
        assert_eq!(strikes.iter().map(|s| s.pitch).collect::<Vec<_>>(), [59, 60, 59, 60]);
        assert_eq!(strikes.iter().map(|s| s.onset).collect::<Vec<_>>(), [ScoreTime::default(), ScoreTime::new(1, 4), ScoreTime::new(1, 2), ScoreTime::new(3, 4)]);
    }
}
