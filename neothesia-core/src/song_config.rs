use crate::practice::{PracticeHands, PracticePart};
use crate::practice_history::{PracticeTrackMode, PracticeTrackSetup, SongPracticeSetup};
use midi_file::MidiTrack;
use std::collections::HashMap;
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
    pub fn new(tracks: &[MidiTrack]) -> Self {
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

    pub fn practice_hands(&self) -> Option<PracticeHands> {
        let left: Vec<_> = self
            .tracks
            .iter()
            .filter(|track| track.practice_part == PracticePart::LeftHand)
            .collect();
        let right: Vec<_> = self
            .tracks
            .iter()
            .filter(|track| track.practice_part == PracticePart::RightHand)
            .collect();
        if left.is_empty() || right.is_empty() {
            return None;
        }

        let all =
            |tracks: &[&TrackConfig], player| tracks.iter().all(|track| track.player == player);
        Some(
            if all(&left, PlayerConfig::Human) && all(&right, PlayerConfig::Human) {
                PracticeHands::Both
            } else if all(&left, PlayerConfig::Auto) && all(&right, PlayerConfig::Human) {
                PracticeHands::Right
            } else if all(&left, PlayerConfig::Human) && all(&right, PlayerConfig::Auto) {
                PracticeHands::Left
            } else {
                PracticeHands::Custom
            },
        )
    }

    pub fn set_practice_hands(&mut self, mode: PracticeHands) -> bool {
        if matches!(mode, PracticeHands::Custom | PracticeHands::Unspecified)
            || self.practice_hands().is_none()
        {
            return false;
        }

        for track in &mut self.tracks {
            track.player = match (mode, track.practice_part) {
                (PracticeHands::Both, PracticePart::LeftHand | PracticePart::RightHand) => {
                    PlayerConfig::Human
                }
                (PracticeHands::Right, PracticePart::RightHand)
                | (PracticeHands::Left, PracticePart::LeftHand) => PlayerConfig::Human,
                (PracticeHands::Right, PracticePart::LeftHand)
                | (PracticeHands::Left, PracticePart::RightHand) => PlayerConfig::Auto,
                _ => track.player,
            };
        }
        true
    }

    pub fn practice_track_setup(&self) -> Vec<PracticeTrackSetup> {
        self.tracks
            .iter()
            .map(|track| PracticeTrackSetup {
                track_id: track.track_id,
                mode: match track.player {
                    PlayerConfig::Mute => PracticeTrackMode::Mute,
                    PlayerConfig::Auto => PracticeTrackMode::Auto,
                    PlayerConfig::Human => PracticeTrackMode::Human,
                },
                visible: track.visible,
            })
            .collect()
    }

    pub fn apply_practice_setup(&mut self, setup: &SongPracticeSetup) -> bool {
        if setup.tracks.len() != self.tracks.len()
            || setup.tracks.iter().any(|saved| {
                !self
                    .tracks
                    .iter()
                    .any(|track| track.track_id == saved.track_id)
            })
        {
            return false;
        }

        for saved in &setup.tracks {
            let track = self
                .tracks
                .iter_mut()
                .find(|track| track.track_id == saved.track_id)
                .expect("validated track IDs must remain present");
            track.player = match saved.mode {
                PracticeTrackMode::Mute => PlayerConfig::Mute,
                PracticeTrackMode::Auto => PlayerConfig::Auto,
                PracticeTrackMode::Human => PlayerConfig::Human,
            };
            track.visible = saved.visible;
        }
        true
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

pub fn infer_practice_parts_from_centers(centers: &[(usize, f32)]) -> HashMap<usize, PracticePart> {
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
