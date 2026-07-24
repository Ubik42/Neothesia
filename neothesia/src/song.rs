use midi_file::MidiTrack;
use neothesia_core::exercise::{ExercisePlan, ExerciseSpec};
use neothesia_core::practice::{PracticeHands, PracticePart};
use neothesia_core::practice_history::{PracticeTrackMode, PracticeTrackSetup, SongPracticeSetup};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

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
    pub exercise_spec: Option<ExerciseSpec>,
    pub exercise_phrase_duration: Option<Duration>,
    pub exercise_fingerings: HashMap<(Duration, u8, u8), u8>,
    pub exercise_fingering_crossings: HashSet<(Duration, u8, u8)>,
}

impl Song {
    pub fn new(file: midi_file::MidiFile) -> Self {
        let config = SongConfig::new(&file.tracks);
        Self {
            file,
            config,
            exercise_spec: None,
            exercise_phrase_duration: None,
            exercise_fingerings: HashMap::new(),
            exercise_fingering_crossings: HashSet::new(),
        }
    }

    pub fn from_exercise(plan: &ExercisePlan) -> Result<Self, String> {
        let mut song = Self::new(plan.to_midi_file()?);
        song.file.content_id = plan.practice_id();
        song.exercise_spec = Some(plan.spec);
        let micros_per_beat = 60_000_000 / u64::from(plan.spec.tempo_bpm);
        song.exercise_phrase_duration = Some(Duration::from_micros(
            u64::from(plan.beats_per_repetition()) * micros_per_beat,
        ));
        if let Some(fingerings) = plan.fingerings() {
            for track in song.file.tracks.iter() {
                let (fingers, crossings) = match track.notes.first().map(|note| note.channel) {
                    Some(0) => (&fingerings.right, &fingerings.right_crossings),
                    Some(1) => (&fingerings.left, &fingerings.left_crossings),
                    _ => continue,
                };
                for ((note, finger), crossing) in track.notes.iter().zip(fingers).zip(crossings) {
                    let key = (note.start, note.note, note.channel);
                    song.exercise_fingerings.insert(key, *finger);
                    if *crossing {
                        song.exercise_fingering_crossings.insert(key);
                    }
                }
            }
        }
        for track in &mut song.config.tracks {
            let channel = song.file.tracks[track.track_id]
                .notes
                .first()
                .map(|note| note.channel);
            track.practice_part = match channel {
                Some(0) => PracticePart::RightHand,
                Some(1) => PracticePart::LeftHand,
                _ => track.practice_part,
            };
        }
        Ok(song)
    }

    pub fn effective_tempo_bpm(&self, speed: f32) -> Option<u16> {
        let base = f32::from(self.exercise_spec?.tempo_bpm);
        if !speed.is_finite() || speed <= 0.0 {
            return None;
        }
        Some((base * speed).round().clamp(1.0, f32::from(u16::MAX)) as u16)
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
    use neothesia_core::exercise::{ExercisePlan, ExerciseSpec};
    use piano_layout::KeyboardRange;

    #[test]
    fn generated_both_hand_exercise_enters_song_with_hand_shortcuts() {
        let plan =
            ExercisePlan::generate(ExerciseSpec::default(), &KeyboardRange::standard_88_keys())
                .unwrap();
        let song = Song::from_exercise(&plan).unwrap();

        assert_eq!(song.config.practice_hands(), Some(PracticeHands::Both));
        assert_eq!(song.config.tracks.len(), 3);
        assert_eq!(song.exercise_fingerings.len(), 30);
        assert_eq!(song.exercise_fingering_crossings.len(), 4);
        assert!(
            song.exercise_fingering_crossings
                .contains(&(Duration::from_secs(3), 65, 0))
        );
        assert!(
            song.exercise_fingering_crossings
                .contains(&(Duration::from_secs(5), 45, 1))
        );
        assert_eq!(
            song.exercise_fingerings.get(&(Duration::ZERO, 60, 0)),
            Some(&1)
        );
        assert_eq!(
            song.exercise_fingerings.get(&(Duration::ZERO, 36, 1)),
            Some(&5)
        );
        assert_eq!(
            song.exercise_fingerings
                .get(&(Duration::from_secs(7), 72, 0)),
            Some(&5)
        );
        assert_eq!(
            song.config
                .tracks
                .iter()
                .filter(|track| track.practice_part == PracticePart::LeftHand)
                .count(),
            1
        );
        assert_eq!(
            song.config
                .tracks
                .iter()
                .filter(|track| track.practice_part == PracticePart::RightHand)
                .count(),
            1
        );
    }

    #[test]
    fn generated_single_hand_exercises_keep_explicit_part_identity() {
        use neothesia_core::exercise::ExerciseHands;

        for (hands, expected) in [
            (ExerciseHands::Right, PracticePart::RightHand),
            (ExerciseHands::Left, PracticePart::LeftHand),
        ] {
            let plan = ExercisePlan::generate(
                ExerciseSpec {
                    hands,
                    ..Default::default()
                },
                &KeyboardRange::standard_88_keys(),
            )
            .unwrap();
            let song = Song::from_exercise(&plan).unwrap();
            let played_track = song
                .config
                .tracks
                .iter()
                .find(|track| track.player == PlayerConfig::Human)
                .unwrap();

            assert_eq!(played_track.practice_part, expected);
        }
    }

    #[test]
    fn generated_song_identity_survives_tempo_and_hand_progression() {
        use neothesia_core::exercise::ExerciseHands;

        let keyboard = KeyboardRange::standard_88_keys();
        let slow = ExercisePlan::generate(ExerciseSpec::default(), &keyboard).unwrap();
        let fast_left = ExercisePlan::generate(
            ExerciseSpec {
                tempo_bpm: 120,
                hands: ExerciseHands::Left,
                ..Default::default()
            },
            &keyboard,
        )
        .unwrap();
        let slow = Song::from_exercise(&slow).unwrap();
        let fast_left = Song::from_exercise(&fast_left).unwrap();

        assert_eq!(slow.file.content_id, fast_left.file.content_id);
        assert_eq!(slow.effective_tempo_bpm(0.75), Some(45));
        assert_eq!(fast_left.effective_tempo_bpm(0.75), Some(90));
        assert_eq!(slow.effective_tempo_bpm(f32::NAN), None);
    }

    #[test]
    fn generated_phrase_boundary_matches_serialized_midi_timing() {
        let plan = ExercisePlan::generate(
            ExerciseSpec {
                repetitions: 2,
                tempo_bpm: 70,
                ..Default::default()
            },
            &KeyboardRange::standard_88_keys(),
        )
        .unwrap();
        let song = Song::from_exercise(&plan).unwrap();
        let boundary = song.exercise_phrase_duration.unwrap();
        let right_track = song
            .file
            .tracks
            .iter()
            .find(|track| track.notes.first().is_some_and(|note| note.channel == 0))
            .unwrap();

        assert_eq!(right_track.notes[15].start, boundary);
    }

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
            source_path: None,
            exercise_spec: None,
            last_used_unix_ms: 0,
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
