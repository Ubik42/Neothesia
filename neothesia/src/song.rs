use midi_file::MidiTrack;
use neothesia_core::practice::PracticePart;
use std::collections::HashMap;

use crate::context::Context;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlayerConfig {
    Mute,
    Auto,
    Human,
}

#[derive(Debug, Clone)]
pub struct TrackConfig {
    pub track_id: usize,
    pub player: PlayerConfig,
    pub visible: bool,
    pub practice_part: PracticePart,
}

#[derive(Default, Debug, Clone)]
pub struct SongConfig {
    pub tracks: Box<[TrackConfig]>,
}

impl SongConfig {
    fn new(tracks: &[MidiTrack]) -> Self {
        let practice_parts = infer_practice_parts(tracks);
        let tracks: Vec<_> = tracks
            .iter()
            .map(|t| {
                let is_drums = t.has_drums && !t.has_other_than_drums;
                TrackConfig {
                    track_id: t.track_id,
                    player: if t.notes.is_empty() || is_drums {
                        PlayerConfig::Auto
                    } else {
                        PlayerConfig::Human
                    },
                    visible: !is_drums,
                    practice_part: practice_parts.get(&t.track_id).copied().unwrap_or_default(),
                }
            })
            .collect();
        Self {
            tracks: tracks.into(),
        }
    }
}

fn infer_practice_parts(tracks: &[MidiTrack]) -> HashMap<usize, PracticePart> {
    let centers: Vec<_> = tracks
        .iter()
        .filter(|track| {
            let is_drums = track.has_drums && !track.has_other_than_drums;
            !track.notes.is_empty() && !is_drums
        })
        .map(|track| {
            let pitch_sum: usize = track.notes.iter().map(|note| note.note as usize).sum();
            (track.track_id, pitch_sum as f32 / track.notes.len() as f32)
        })
        .collect();

    infer_practice_parts_from_centers(&centers)
}

fn infer_practice_parts_from_centers(centers: &[(usize, f32)]) -> HashMap<usize, PracticePart> {
    let mut parts: HashMap<_, _> = centers
        .iter()
        .map(|(track_id, _)| (*track_id, PracticePart::Other))
        .collect();

    if centers.len() == 2 {
        let mut ordered = centers.to_vec();
        ordered.sort_by(|(left_id, left_pitch), (right_id, right_pitch)| {
            left_pitch
                .total_cmp(right_pitch)
                .then_with(|| left_id.cmp(right_id))
        });
        parts.insert(ordered[0].0, PracticePart::LeftHand);
        parts.insert(ordered[1].0, PracticePart::RightHand);
    }

    parts
}

#[derive(Debug, Clone)]
pub struct Song {
    pub file: midi_file::MidiFile,
    pub config: SongConfig,
}

impl Song {
    pub fn new(file: midi_file::MidiFile) -> Self {
        let config = SongConfig::new(&file.tracks);
        Self { file, config }
    }

    pub fn from_env(ctx: &Context) -> Option<Self> {
        let args: Vec<String> = std::env::args().collect();
        let midi_file = if args.len() > 1 {
            midi_file::MidiFile::new(&args[1]).ok()
        } else if let Some(last) = ctx.config.last_opened_song() {
            midi_file::MidiFile::new(last).ok()
        } else {
            None
        };

        Some(Self::new(midi_file?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_piano_tracks_are_assigned_by_pitch_center() {
        let parts = infer_practice_parts_from_centers(&[(7, 76.0), (3, 48.0)]);

        assert_eq!(parts[&7], PracticePart::RightHand);
        assert_eq!(parts[&3], PracticePart::LeftHand);
    }

    #[test]
    fn ambiguous_multi_part_arrangement_stays_unassigned() {
        let parts = infer_practice_parts_from_centers(&[(1, 48.0), (2, 60.0), (3, 72.0)]);

        assert!(parts.values().all(|part| *part == PracticePart::Other));
    }
}
