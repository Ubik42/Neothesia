use crate::{LoadedSong, Note};
use midi_file::MidiFile;
use neothesia_core::song_config::{PlayerConfig, SongConfig};
use serde::Serialize;
use std::{collections::HashMap, time::Duration};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bar {
    pub number: usize,
    pub start: f64,
    pub end: f64,
    pub start_tick: u64,
    pub end_tick: u64,
    pub numerator: u8,
    pub denominator: u16,
    pub explicit: bool,
    pub partial: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tempo {
    pub tick: u64,
    pub seconds: f64,
    pub bpm: f64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: usize,
    pub name: String,
    pub source_name:String,
    pub notes: usize,
    pub mode: String,
    pub part: String,
    pub visible: bool,
    pub inferred: bool,
}
pub fn build(
    file: &MidiFile,
    title: &str,
    duration: f64,
    config: &SongConfig,
    fingers: &HashMap<(usize, Duration, u8), u8>,
    assigned: &HashMap<(usize, usize), neothesia_core::practice::PracticePart>,
    meter_correction: Option<neothesia_core::library::MeterCorrection>,
    generated: bool,
    has_score: bool,
    score_revision: u64,
) -> LoadedSong {
    let mut notes: Vec<_> = file
        .tracks
        .iter()
        .filter(|t| {
            config
                .tracks
                .iter()
                .find(|c| c.track_id == t.track_id)
                .is_some_and(|c| c.visible)
        })
        .flat_map(|t| t.notes.iter().enumerate())
        .map(|(index, n)| Note {
            index,
            part: crate::hands::label(crate::hands::part(config, assigned, n.track_id, index))
                .into(),
            manual_hand: assigned.contains_key(&(n.track_id, index)),
            pitch: n.note,
            start: n.start.as_secs_f64(),
            duration: n.duration.as_secs_f64(),
            velocity: n.velocity,
            track: n.track_id,
            tick: file.tempo_track.seconds_to_pulses(n.start.as_secs_f64()),
            end_tick: file.tempo_track.seconds_to_pulses(n.end.as_secs_f64()),
            finger: fingers.get(&(n.track_id, n.start, n.note)).copied(),
        })
        .collect();
    notes.sort_by(|a, b| a.start.total_cmp(&b.start).then(a.pitch.cmp(&b.pitch)));
    let grid = &file.musical_time;
    let measures = grid
        .measures
        .iter()
        .filter(|m| {
            file.tempo_track
                .pulses_to_duration(m.start_tick)
                .as_secs_f64()
                < duration
        })
        .map(|m| Bar {
            number: m.number,
            start: file
                .tempo_track
                .pulses_to_duration(m.start_tick)
                .as_secs_f64(),
            end: file
                .tempo_track
                .pulses_to_duration(m.end_tick)
                .as_secs_f64(),
            start_tick: m.start_tick,
            end_tick: m.end_tick,
            numerator: m.numerator,
            denominator: m.denominator,
            explicit: m.explicit_meter,
            partial: m.partial,
        })
        .collect();
    let mut tempo = vec![Tempo {
        tick: 0,
        seconds: 0.,
        bpm: 120.,
    }];
    for e in file.tempo_track.events() {
        if e.absolute_pulses == 0 {
            tempo.clear()
        }
        tempo.push(Tempo {
            tick: e.absolute_pulses,
            seconds: e.timestamp.as_secs_f64(),
            bpm: 60_000_000. / f64::from(e.tempo),
        })
    }
    let tracks = config
        .tracks
        .iter()
        .filter_map(|c| {
            file.tracks
                .iter()
                .find(|t| t.track_id == c.track_id)
                .filter(|t| !t.notes.is_empty())
                .map(|t| Track {
                    id: c.track_id,
                    name:t.name.clone().unwrap_or_else(||format!("音轨 {}",c.track_id+1)),
                    source_name:t.name.clone().unwrap_or_else(||format!("音轨 {}",c.track_id+1)),
                    notes: t.notes.len(),
                    mode: match c.player {
                        PlayerConfig::Human => "human",
                        PlayerConfig::Auto => "auto",
                        PlayerConfig::Mute => "mute",
                    }
                    .into(),
                    part: match c.practice_part {
                        neothesia_core::practice::PracticePart::LeftHand => "left",
                        neothesia_core::practice::PracticePart::RightHand => "right",
                        _ => "other",
                    }
                    .into(),
                    visible: c.visible,
                    inferred: !generated,
                })
        })
        .collect();
    LoadedSong {
        title: title.into(),
        content_id: file.content_id.clone(),
        source_path: file.source_path.clone().unwrap_or_default(),
        duration,
        notes,
        measures,
        tempo,
        ppq: grid.ppq,
        tracks,
        generated,
        has_score,
        score_revision,
        meter_correction,
    }
}
