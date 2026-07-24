use midi_file::MidiTrack;
use neothesia_core::practice::{PracticeHands, PracticePart};
use neothesia_core::practice_history::{PracticeTrackMode, PracticeTrackSetup, SongPracticeSetup};
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

        let mut song = Self::new(midi_file?);
        song.apply_saved_setup(ctx);
        Some(song)
    }

    pub fn apply_saved_setup(&mut self, ctx: &Context) -> bool {
        ctx.practice_history
            .setup(&self.file.content_id)
            .is_some_and(|setup| self.config.apply_practice_setup(setup))
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

    #[test]
    fn hand_modes_keep_the_other_hand_as_accompaniment() {
        let mut config = SongConfig {
            tracks: vec![
                TrackConfig {
                    track_id: 0,
                    player: PlayerConfig::Human,
                    visible: true,
                    practice_part: PracticePart::LeftHand,
                },
                TrackConfig {
                    track_id: 1,
                    player: PlayerConfig::Human,
                    visible: true,
                    practice_part: PracticePart::RightHand,
                },
                TrackConfig {
                    track_id: 2,
                    player: PlayerConfig::Mute,
                    visible: false,
                    practice_part: PracticePart::Other,
                },
            ]
            .into(),
        };

        assert_eq!(config.practice_hands(), Some(PracticeHands::Both));
        assert!(config.set_practice_hands(PracticeHands::Right));
        assert_eq!(config.tracks[0].player, PlayerConfig::Auto);
        assert_eq!(config.tracks[1].player, PlayerConfig::Human);
        assert_eq!(config.tracks[2].player, PlayerConfig::Mute);
        assert_eq!(config.practice_hands(), Some(PracticeHands::Right));

        assert!(config.set_practice_hands(PracticeHands::Left));
        assert_eq!(config.tracks[0].player, PlayerConfig::Human);
        assert_eq!(config.tracks[1].player, PlayerConfig::Auto);
    }

    #[test]
    fn ambiguous_parts_do_not_offer_hand_shortcuts() {
        let mut config = SongConfig {
            tracks: vec![TrackConfig {
                track_id: 0,
                player: PlayerConfig::Human,
                visible: true,
                practice_part: PracticePart::Other,
            }]
            .into(),
        };

        assert_eq!(config.practice_hands(), None);
        assert!(!config.set_practice_hands(PracticeHands::Right));
    }

    #[test]
    fn saved_track_setup_requires_the_same_track_structure() {
        let mut config = SongConfig {
            tracks: vec![TrackConfig {
                track_id: 4,
                player: PlayerConfig::Human,
                visible: true,
                practice_part: PracticePart::RightHand,
            }]
            .into(),
        };
        let valid = SongPracticeSetup {
            tracks: vec![PracticeTrackSetup {
                track_id: 4,
                mode: PracticeTrackMode::Auto,
                visible: false,
            }],
            speed: 0.7,
            loop_setup: None,
        };
        assert!(config.apply_practice_setup(&valid));
        assert_eq!(config.tracks[0].player, PlayerConfig::Auto);
        assert!(!config.tracks[0].visible);

        let incompatible = SongPracticeSetup {
            tracks: vec![PracticeTrackSetup {
                track_id: 99,
                mode: PracticeTrackMode::Mute,
                visible: true,
            }],
            ..valid
        };
        assert!(!config.apply_practice_setup(&incompatible));
        assert_eq!(config.tracks[0].player, PlayerConfig::Auto);
    }
}
