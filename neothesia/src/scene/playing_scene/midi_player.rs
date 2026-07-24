use midi_file::midly::{MidiMessage, num::u4};

use crate::{
    output_manager::OutputConnection,
    song::{PlayerConfig, Song},
};
use neothesia_core::{
    piano_layout,
    practice::{AttemptSummary, PracticeHands, PracticeMatcher, PracticeSnapshot, PracticeTarget},
};
use std::{collections::HashMap, time::Duration};

pub struct MidiPlayer {
    playback: midi_file::PlaybackState,
    output: OutputConnection,
    song: Song,
    practice: PracticeMatcher,
    session_time: Duration,
    separate_channels: bool,
    wait_for_notes: bool,
    target_durations: HashMap<(usize, u8, Duration), Duration>,
}

impl MidiPlayer {
    pub fn new(
        output: OutputConnection,
        song: Song,
        user_keyboard_range: piano_layout::KeyboardRange,
        separate_channels: bool,
        wait_for_notes: bool,
    ) -> Self {
        Self::new_with_lead_in(
            output,
            song,
            user_keyboard_range,
            separate_channels,
            wait_for_notes,
            Duration::from_secs(3),
        )
    }

    pub fn new_with_lead_in(
        output: OutputConnection,
        song: Song,
        user_keyboard_range: piano_layout::KeyboardRange,
        separate_channels: bool,
        wait_for_notes: bool,
        lead_in: Duration,
    ) -> Self {
        let target_durations = song
            .file
            .tracks
            .iter()
            .flat_map(|track| {
                track
                    .notes
                    .iter()
                    .map(|note| ((note.track_id, note.note, note.start), note.duration))
            })
            .collect();
        let mut player = Self {
            playback: midi_file::PlaybackState::new(lead_in, song.file.tracks.clone()),
            output,
            practice: PracticeMatcher::new(user_keyboard_range),
            session_time: Duration::ZERO,
            song,
            separate_channels,
            wait_for_notes,
            target_durations,
        };
        // Let's reset programs,
        // for timestamp 0 most likely all programs will be 0, so this should clean any leftovers
        // from previous songs
        player.send_midi_programs_for_timestamp(&player.playback.time());
        player.update(Duration::ZERO);

        player
    }

    pub fn song(&self) -> &Song {
        &self.song
    }

    /// When playing: returns midi events
    ///
    /// When paused: returns None
    pub fn update(&mut self, delta: Duration) -> Vec<&midi_file::MidiEvent> {
        let events = self.playback.update(delta);

        events.iter().for_each(|event| {
            let config = &self.song.config.tracks[event.track_id];

            let channel = if self.separate_channels {
                event.track_color_id as u8
            } else {
                event.channel
            };
            match config.player {
                PlayerConfig::Auto => {
                    self.output // TODO: Send to multiple outputs
                        .midi_event(u4::new(channel), event.message);
                }
                PlayerConfig::Human => {
                    if let Some((note, active)) = note_state(&event.message) {
                        let velocity = match event.message {
                            MidiMessage::NoteOn { vel, .. } if active => vel.as_int(),
                            _ => 0,
                        };
                        let measure = self
                            .song
                            .file
                            .measures
                            .partition_point(|start| *start <= event.timestamp);
                        self.practice.score_target(
                            self.session_time,
                            PracticeTarget {
                                note,
                                velocity,
                                score_time: event.timestamp,
                                duration: self
                                    .target_durations
                                    .get(&(event.track_id, note, event.timestamp))
                                    .copied()
                                    .unwrap_or_default(),
                                track_id: event.track_id,
                                measure,
                                part: config.practice_part,
                            },
                            active,
                        );
                    }
                    if let MidiMessage::Controller { controller, value } = event.message
                        && controller.as_int() == 64
                    {
                        self.practice.score_pedal(value.as_int());
                    }

                    if self.wait_for_notes {
                        // In Human mode note events from the file are targets for the player,
                        // not notes to be played by the synthesizer. Keep forwarding controller
                        // and other non-note events so the track still sounds as intended.
                        if should_forward_human_event(&event.message) {
                            self.output.midi_event(u4::new(channel), event.message);
                        }
                    } else {
                        self.output.midi_event(u4::new(channel), event.message);
                    }
                }
                PlayerConfig::Mute => {}
            }
        });

        events
    }

    fn clear(&mut self) {
        self.output.stop_all();
    }
}

impl Drop for MidiPlayer {
    fn drop(&mut self) {
        self.clear();
    }
}

impl MidiPlayer {
    pub fn pause_resume(&mut self) {
        if self.playback.is_paused() {
            self.resume();
        } else {
            self.pause();
        }
    }

    pub fn pause(&mut self) {
        self.clear();
        self.playback.pause();
    }

    pub fn emergency_stop(&mut self) {
        self.clear();
        self.playback.pause();
        self.practice.clear_pending();
    }

    pub fn resume(&mut self) {
        self.playback.resume();
        self.practice.clear_pending();
    }

    fn send_midi_programs_for_timestamp(&self, time: &Duration) {
        for (&channel, &p) in self.song.file.program_track.program_for_timestamp(time) {
            self.output.midi_event(
                u4::new(channel),
                midi_file::midly::MidiMessage::ProgramChange {
                    program: midi_file::midly::num::u7::new(p),
                },
            );
        }
    }

    pub fn set_time(&mut self, time: Duration) {
        self.playback.set_time(time);

        self.clear();
        self.practice.clear_pending();
        self.send_midi_programs_for_timestamp(&time);
    }

    pub fn rewind(&mut self, delta: i64) {
        let mut time = self.playback.time();

        if delta < 0 {
            let delta = Duration::from_millis((-delta) as u64);
            time = time.saturating_sub(delta);
        } else {
            let delta = Duration::from_millis(delta as u64);
            time = time.saturating_add(delta);
        }

        self.set_time(time);
    }

    pub fn percentage_to_time(&self, p: f32) -> Duration {
        Duration::from_secs_f32((p * self.playback.length().as_secs_f32()).max(0.0))
    }

    pub fn time_to_percentage(&self, time: &Duration) -> f32 {
        time.as_secs_f32() / self.playback.length().as_secs_f32()
    }

    pub fn set_percentage_time(&mut self, p: f32) {
        self.set_time(self.percentage_to_time(p));
    }

    pub fn leed_in(&self) -> &Duration {
        self.playback.leed_in()
    }

    pub fn length(&self) -> Duration {
        self.playback.length()
    }

    pub fn percentage(&self) -> f32 {
        self.playback.percentage()
    }

    pub fn is_finished(&self) -> bool {
        self.playback.is_finished()
    }

    pub fn time(&self) -> Duration {
        self.playback.time()
    }

    pub fn time_without_lead_in(&self) -> f32 {
        self.playback.time().as_secs_f32() - self.playback.leed_in().as_secs_f32()
    }

    pub fn is_paused(&self) -> bool {
        self.playback.is_paused()
    }
}

impl MidiPlayer {
    pub fn tick_practice_clock(&mut self, delta: Duration) {
        if self.playback.is_paused() {
            return;
        }
        self.session_time += delta;
        self.practice.tick(self.session_time);
        if !self.wait_for_notes {
            self.practice.finalize_missed(self.session_time);
        }
    }

    pub fn should_advance(&self) -> bool {
        !self.wait_for_notes || self.practice.are_required_notes_pressed()
    }

    pub fn wait_for_notes(&self) -> bool {
        self.wait_for_notes
    }

    pub fn practice_hands(&self) -> Option<PracticeHands> {
        self.song.config.practice_hands()
    }

    pub fn set_practice_hands(&mut self, mode: PracticeHands) -> bool {
        if self.song.config.practice_hands() == Some(mode) {
            return false;
        }
        if !self.song.config.set_practice_hands(mode) {
            return false;
        }
        self.practice.reset();
        true
    }

    pub fn set_wait_for_notes(&mut self, wait_for_notes: bool) {
        if self.wait_for_notes != wait_for_notes {
            self.wait_for_notes = wait_for_notes;
            self.practice.clear_pending();
        }
    }

    pub fn practice_snapshot(&self) -> PracticeSnapshot {
        self.practice.snapshot()
    }

    pub fn finish_practice(&mut self) -> AttemptSummary {
        self.practice.finish();
        self.practice.summary()
    }

    pub fn restart_practice(&mut self) {
        self.practice.reset();
        self.session_time = Duration::ZERO;
        self.set_time(Duration::ZERO);
        self.resume();
    }

    pub fn reset_practice_attempt(&mut self) {
        self.practice.reset();
    }

    pub fn user_midi_event(&mut self, channel: u8, message: &MidiMessage) {
        self.output.midi_event(u4::new(channel), *message);
        if !self.playback.is_paused() {
            if let Some((note, active)) = note_state(message) {
                let velocity = match message {
                    MidiMessage::NoteOn { vel, .. } if active => vel.as_int(),
                    _ => 0,
                };
                self.practice
                    .user_note_with_velocity(self.session_time, note, active, velocity);
            }
            if let MidiMessage::Controller { controller, value } = message
                && controller.as_int() == 64
            {
                self.practice.user_pedal(value.as_int());
            }
        }
    }
}

fn should_forward_human_event(message: &MidiMessage) -> bool {
    !matches!(
        message,
        MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. }
    )
}

fn note_state(message: &MidiMessage) -> Option<(u8, bool)> {
    match message {
        MidiMessage::NoteOn { key, vel } => Some((key.as_int(), vel.as_int() > 0)),
        MidiMessage::NoteOff { key, .. } => Some((key.as_int(), false)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        output_manager::{OutputConnection, TestOutputEvent},
        song::Song,
    };
    use midi_file::midly::{
        Format, Header, MetaMessage, PitchBend, Smf, Timing, TrackEvent, TrackEventKind,
        num::{u4, u7},
    };

    #[test]
    fn wait_mode_can_release_a_blocked_practice_track() {
        let file = midi_file::MidiFile::new("../test.mid").unwrap();
        let song = Song::new(file);
        let mut player = MidiPlayer::new_with_lead_in(
            OutputConnection::DummyOutput,
            song,
            piano_layout::KeyboardRange::new(21..=108),
            false,
            true,
            Duration::ZERO,
        );

        player.update(Duration::from_secs(10));
        assert!(!player.should_advance());
        player.tick_practice_clock(Duration::from_secs(30));
        assert_eq!(player.practice_snapshot().missed_notes, 0);

        player.set_wait_for_notes(false);
        assert!(player.should_advance());
    }

    #[test]
    fn flow_mode_finalizes_unplayed_targets_as_missed() {
        let file = midi_file::MidiFile::new("../test.mid").unwrap();
        let song = Song::new(file);
        let mut player = MidiPlayer::new_with_lead_in(
            OutputConnection::DummyOutput,
            song,
            piano_layout::KeyboardRange::new(21..=108),
            false,
            false,
            Duration::ZERO,
        );

        player.tick_practice_clock(Duration::from_secs(10));
        player.update(Duration::from_secs(10));
        player.tick_practice_clock(Duration::from_millis(501));

        assert!(player.practice_snapshot().missed_notes > 0);
        assert!(player.should_advance());

        let summary = player.finish_practice();
        assert!(summary.overall.missed_notes > 0);
        assert!(!summary.measures.is_empty());
        assert!(summary.measures.iter().all(|item| item.measure > 0));
    }

    #[test]
    fn transport_changes_and_drop_silence_the_output() {
        let file = midi_file::MidiFile::new("../test.mid").unwrap();
        let song = Song::new(file);
        let (output, events) = OutputConnection::test();
        let mut player = MidiPlayer::new_with_lead_in(
            output,
            song,
            piano_layout::KeyboardRange::new(21..=108),
            false,
            false,
            Duration::ZERO,
        );
        events.borrow_mut().clear();

        player.pause();
        player.resume();
        player.set_time(Duration::from_millis(250));
        player.restart_practice();
        drop(player);

        let panic_count = events
            .borrow()
            .iter()
            .filter(|event| matches!(event, TestOutputEvent::StopAll))
            .count();
        assert_eq!(panic_count, 4);
    }

    #[test]
    fn emergency_stop_pauses_and_clears_pending_practice_input() {
        let file = midi_file::MidiFile::new("../test.mid").unwrap();
        let song = Song::new(file);
        let (output, events) = OutputConnection::test();
        let mut player = MidiPlayer::new_with_lead_in(
            output,
            song,
            piano_layout::KeyboardRange::new(21..=108),
            false,
            true,
            Duration::ZERO,
        );
        player.update(Duration::from_secs(10));
        assert!(!player.should_advance());
        events.borrow_mut().clear();

        player.emergency_stop();

        assert!(player.is_paused());
        assert!(player.should_advance());
        assert_eq!(player.practice_snapshot().required_notes, 0);
        assert!(matches!(
            events.borrow().as_slice(),
            [TestOutputEvent::StopAll]
        ));
    }

    #[test]
    fn live_expressive_midi_is_forwarded_without_value_changes() {
        let file = midi_file::MidiFile::new("../test.mid").unwrap();
        let song = Song::new(file);
        let (output, events) = OutputConnection::test();
        let mut player = MidiPlayer::new_with_lead_in(
            output,
            song,
            piano_layout::KeyboardRange::new(21..=108),
            false,
            true,
            Duration::ZERO,
        );
        events.borrow_mut().clear();
        let messages = expressive_messages();

        for message in messages {
            player.user_midi_event(5, &message);
        }

        let expected: Vec<_> = messages
            .into_iter()
            .map(|message| TestOutputEvent::Midi {
                channel: u4::new(5),
                message,
            })
            .collect();
        assert_eq!(events.borrow().as_slice(), expected.as_slice());
    }

    #[test]
    fn human_track_expressive_midi_survives_wait_mode() {
        let messages = expressive_messages();
        let song = expressive_song(&messages);
        let (output, events) = OutputConnection::test();
        let mut player = MidiPlayer::new_with_lead_in(
            output,
            song,
            piano_layout::KeyboardRange::new(21..=108),
            false,
            true,
            Duration::ZERO,
        );
        events.borrow_mut().clear();

        player.update(Duration::from_secs(1));

        let forwarded: Vec<_> = events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                TestOutputEvent::Midi { channel, message } => Some((*channel, *message)),
                TestOutputEvent::StopAll => None,
            })
            .collect();
        let expected: Vec<_> = messages
            .into_iter()
            .map(|message| (u4::new(2), message))
            .collect();
        assert_eq!(forwarded, expected);
    }

    #[test]
    fn player_captures_score_and_live_expression_without_altering_output() {
        let score_messages = expressive_messages();
        let song = expressive_song(&score_messages);
        let (output, events) = OutputConnection::test();
        let mut player = MidiPlayer::new_with_lead_in(
            output,
            song,
            piano_layout::KeyboardRange::new(21..=108),
            false,
            true,
            Duration::ZERO,
        );
        events.borrow_mut().clear();
        player.update(Duration::from_secs(1));

        let note_on = MidiMessage::NoteOn {
            key: u7::new(60),
            vel: u7::new(70),
        };
        let note_off = MidiMessage::NoteOff {
            key: u7::new(60),
            vel: u7::new(0),
        };
        let pedal_messages = [
            MidiMessage::Controller {
                controller: u7::new(64),
                value: u7::new(0),
            },
            MidiMessage::Controller {
                controller: u7::new(64),
                value: u7::new(64),
            },
            MidiMessage::Controller {
                controller: u7::new(64),
                value: u7::new(127),
            },
        ];
        player.user_midi_event(5, &note_on);
        player.tick_practice_clock(Duration::from_millis(250));
        player.user_midi_event(5, &note_off);
        for message in pedal_messages {
            player.user_midi_event(5, &message);
        }

        let summary = player.finish_practice();
        assert_eq!(summary.expression.velocity.matched_samples, 1);
        assert_eq!(summary.expression.velocity.mean_abs_difference, Some(30));
        assert_eq!(summary.expression.pedal.target_changes, 1);
        assert_eq!(summary.expression.pedal.user_changes, 2);
        assert_eq!(summary.expression.pedal.user_continuous_samples, 1);
        assert_eq!(summary.expression.articulation.matched_samples, 1);
        assert_eq!(
            summary
                .expression
                .articulation
                .median_duration_ratio_percent,
            Some(50)
        );

        let expected_user = [note_on, note_off]
            .into_iter()
            .chain(pedal_messages)
            .collect::<Vec<_>>();
        let forwarded_user: Vec<_> = events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                TestOutputEvent::Midi { channel, message } if *channel == u4::new(5) => {
                    Some(*message)
                }
                _ => None,
            })
            .collect();
        assert_eq!(forwarded_user, expected_user);
    }

    fn expressive_messages() -> [MidiMessage; 4] {
        [
            MidiMessage::Controller {
                controller: u7::new(64),
                value: u7::new(23),
            },
            MidiMessage::Controller {
                controller: u7::new(64),
                value: u7::new(91),
            },
            MidiMessage::PitchBend {
                bend: PitchBend::from_int(1_337),
            },
            MidiMessage::ChannelAftertouch { vel: u7::new(77) },
        ]
    }

    fn expressive_song(messages: &[MidiMessage]) -> Song {
        let mut track = vec![
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(500_000.into())),
            },
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
            },
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Midi {
                    channel: u4::new(2),
                    message: MidiMessage::NoteOn {
                        key: u7::new(60),
                        vel: u7::new(100),
                    },
                },
            },
        ];
        for message in messages {
            track.push(TrackEvent {
                delta: 1.into(),
                kind: TrackEventKind::Midi {
                    channel: u4::new(2),
                    message: *message,
                },
            });
        }
        track.extend([
            TrackEvent {
                delta: 480.into(),
                kind: TrackEventKind::Midi {
                    channel: u4::new(2),
                    message: MidiMessage::NoteOff {
                        key: u7::new(60),
                        vel: u7::new(0),
                    },
                },
            },
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            },
        ]);
        let smf = Smf {
            header: Header {
                format: Format::SingleTrack,
                timing: Timing::Metrical(480.into()),
            },
            tracks: vec![track],
        };
        Song::new(midi_file::MidiFile::from_smf("expressive.mid", &smf).unwrap())
    }
}
