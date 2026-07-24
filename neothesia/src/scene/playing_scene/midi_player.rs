use midi_file::midly::{MidiMessage, num::u4};

use crate::{
    output_manager::OutputConnection,
    song::{PlayerConfig, Song},
};
use neothesia_core::{
    piano_layout,
    practice::{PracticeMatcher, PracticeSnapshot},
};
use std::time::Duration;

pub struct MidiPlayer {
    playback: midi_file::PlaybackState,
    output: OutputConnection,
    song: Song,
    practice: PracticeMatcher,
    session_time: Duration,
    separate_channels: bool,
    wait_for_notes: bool,
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
        let mut player = Self {
            playback: midi_file::PlaybackState::new(lead_in, song.file.tracks.clone()),
            output,
            practice: PracticeMatcher::new(user_keyboard_range),
            session_time: Duration::ZERO,
            song,
            separate_channels,
            wait_for_notes,
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
                    if self.wait_for_notes {
                        if let Some((note, active)) = note_state(&event.message) {
                            self.practice.score_note(self.session_time, note, active);
                        }

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

        // Discard all of the events till that point
        let events = self.playback.update(Duration::ZERO);
        std::mem::drop(events);

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
    }

    pub fn should_advance(&self) -> bool {
        !self.wait_for_notes || self.practice.are_required_notes_pressed()
    }

    pub fn wait_for_notes(&self) -> bool {
        self.wait_for_notes
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

    pub fn user_midi_event(&mut self, channel: u8, message: &MidiMessage) {
        self.output.midi_event(u4::new(channel), *message);
        if !self.playback.is_paused()
            && let Some((note, active)) = note_state(message)
        {
            self.practice.user_note(self.session_time, note, active);
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
    use crate::{output_manager::OutputConnection, song::Song};

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

        player.set_wait_for_notes(false);
        assert!(player.should_advance());
    }
}
