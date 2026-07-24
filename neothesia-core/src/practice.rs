use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use piano_layout::KeyboardRange;

pub type NoteId = u8;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PracticeWindows {
    /// A user note may wait this long for its matching score note.
    pub early_match: Duration,
    /// A matched note inside this window is considered on time.
    pub on_time: Duration,
}

impl Default for PracticeWindows {
    fn default() -> Self {
        Self {
            early_match: Duration::from_millis(500),
            on_time: Duration::from_millis(80),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TimingGrade {
    Early(Duration),
    OnTime,
    Late(Duration),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MatchedNote {
    pub note: NoteId,
    pub timing: TimingGrade,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct PracticeSnapshot {
    pub matched_notes: usize,
    pub on_time_notes: usize,
    pub early_notes: usize,
    pub late_notes: usize,
    pub wrong_notes: usize,
    pub required_notes: usize,
}

impl PracticeSnapshot {
    pub fn accuracy(self) -> Option<f32> {
        let judged = self.matched_notes + self.wrong_notes;
        (judged != 0).then(|| self.matched_notes as f32 / judged as f32)
    }
}

#[derive(Debug, Clone, Copy)]
struct NotePress {
    timestamp: Duration,
}

/// Matches score note events with live keyboard input.
///
/// All timestamps come from the caller's monotonic session clock. This keeps
/// the matcher deterministic in tests and lets the clock continue while
/// wait-for-notes pauses the MIDI timeline.
#[derive(Debug)]
pub struct PracticeMatcher {
    user_keyboard_range: KeyboardRange,
    windows: PracticeWindows,

    required_notes: HashMap<NoteId, NotePress>,
    user_pressed_recently: HashMap<NoteId, NotePress>,
    in_progress_score_notes: HashSet<NoteId>,

    snapshot: PracticeSnapshot,
}

impl PracticeMatcher {
    pub fn new(user_keyboard_range: KeyboardRange) -> Self {
        Self::with_windows(user_keyboard_range, PracticeWindows::default())
    }

    pub fn with_windows(user_keyboard_range: KeyboardRange, windows: PracticeWindows) -> Self {
        Self {
            user_keyboard_range,
            windows,
            required_notes: HashMap::new(),
            user_pressed_recently: HashMap::new(),
            in_progress_score_notes: HashSet::new(),
            snapshot: PracticeSnapshot::default(),
        }
    }

    pub fn tick(&mut self, now: Duration) {
        let before = self.user_pressed_recently.len();
        self.user_pressed_recently
            .retain(|_, press| now.saturating_sub(press.timestamp) <= self.windows.early_match);
        self.snapshot.wrong_notes += before - self.user_pressed_recently.len();
    }

    pub fn user_note(&mut self, now: Duration, note: NoteId, active: bool) -> Option<MatchedNote> {
        if !active || !self.user_keyboard_range.contains(note) {
            return None;
        }

        if let Some(required) = self.required_notes.remove(&note) {
            self.snapshot.required_notes = self.required_notes.len();
            return Some(self.record_match(
                note,
                TimingGrade::Late(now.saturating_sub(required.timestamp)),
            ));
        }

        let replaced = self
            .user_pressed_recently
            .insert(note, NotePress { timestamp: now })
            .is_some();
        if replaced {
            self.snapshot.wrong_notes += 1;
        }

        None
    }

    pub fn score_note(&mut self, now: Duration, note: NoteId, active: bool) -> Option<MatchedNote> {
        if !self.user_keyboard_range.contains(note) {
            return None;
        }

        if !active {
            self.in_progress_score_notes.remove(&note);
            return None;
        }

        if !self.in_progress_score_notes.insert(note) {
            return None;
        }

        if let Some(press) = self.user_pressed_recently.remove(&note) {
            return Some(self.record_match(
                note,
                TimingGrade::Early(now.saturating_sub(press.timestamp)),
            ));
        }

        self.required_notes
            .insert(note, NotePress { timestamp: now });
        self.snapshot.required_notes = self.required_notes.len();
        None
    }

    pub fn snapshot(&self) -> PracticeSnapshot {
        self.snapshot
    }

    pub fn are_required_notes_pressed(&self) -> bool {
        self.required_notes.is_empty()
    }

    /// Clears transient matching state while retaining the session totals.
    pub fn clear_pending(&mut self) {
        self.required_notes.clear();
        self.user_pressed_recently.clear();
        self.in_progress_score_notes.clear();
        self.snapshot.required_notes = 0;
    }

    pub fn reset(&mut self) {
        self.clear_pending();
        self.snapshot = PracticeSnapshot::default();
    }

    fn record_match(&mut self, note: NoteId, timing: TimingGrade) -> MatchedNote {
        let timing = match timing {
            TimingGrade::Early(delta) if delta <= self.windows.on_time => TimingGrade::OnTime,
            TimingGrade::Late(delta) if delta <= self.windows.on_time => TimingGrade::OnTime,
            timing => timing,
        };

        self.snapshot.matched_notes += 1;
        match timing {
            TimingGrade::Early(_) => self.snapshot.early_notes += 1,
            TimingGrade::OnTime => self.snapshot.on_time_notes += 1,
            TimingGrade::Late(_) => self.snapshot.late_notes += 1,
        }

        MatchedNote { note, timing }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher() -> PracticeMatcher {
        PracticeMatcher::with_windows(
            KeyboardRange::new(21..=108),
            PracticeWindows {
                early_match: Duration::from_millis(500),
                on_time: Duration::from_millis(80),
            },
        )
    }

    #[test]
    fn matches_early_on_time_and_late_notes_deterministically() {
        let mut matcher = matcher();

        matcher.user_note(Duration::from_millis(950), 60, true);
        assert_eq!(
            matcher.score_note(Duration::from_millis(1_000), 60, true),
            Some(MatchedNote {
                note: 60,
                timing: TimingGrade::OnTime,
            })
        );

        matcher.user_note(Duration::from_millis(1_200), 62, true);
        assert_eq!(
            matcher.score_note(Duration::from_millis(1_400), 62, true),
            Some(MatchedNote {
                note: 62,
                timing: TimingGrade::Early(Duration::from_millis(200)),
            })
        );

        matcher.score_note(Duration::from_millis(2_000), 64, true);
        assert_eq!(
            matcher.user_note(Duration::from_millis(2_220), 64, true),
            Some(MatchedNote {
                note: 64,
                timing: TimingGrade::Late(Duration::from_millis(220)),
            })
        );

        assert_eq!(
            matcher.snapshot(),
            PracticeSnapshot {
                matched_notes: 3,
                on_time_notes: 1,
                early_notes: 1,
                late_notes: 1,
                wrong_notes: 0,
                required_notes: 0,
            }
        );
    }

    #[test]
    fn expires_unmatched_user_notes_as_wrong() {
        let mut matcher = matcher();
        matcher.user_note(Duration::from_secs(1), 61, true);

        matcher.tick(Duration::from_millis(1_500));
        assert_eq!(matcher.snapshot().wrong_notes, 0);

        matcher.tick(Duration::from_millis(1_501));
        assert_eq!(matcher.snapshot().wrong_notes, 1);
        assert_eq!(matcher.snapshot().accuracy(), Some(0.0));
    }

    #[test]
    fn tracks_required_chord_notes_without_order_penalty() {
        let mut matcher = matcher();
        let target_time = Duration::from_secs(2);

        matcher.score_note(target_time, 60, true);
        matcher.score_note(target_time, 64, true);
        matcher.score_note(target_time, 67, true);
        assert_eq!(matcher.snapshot().required_notes, 3);
        assert!(!matcher.are_required_notes_pressed());

        matcher.user_note(Duration::from_millis(2_100), 67, true);
        matcher.user_note(Duration::from_millis(2_110), 60, true);
        matcher.user_note(Duration::from_millis(2_120), 64, true);

        assert!(matcher.are_required_notes_pressed());
        assert_eq!(matcher.snapshot().matched_notes, 3);
    }

    #[test]
    fn ignores_notes_outside_the_configured_keyboard_range() {
        let mut matcher = PracticeMatcher::new(KeyboardRange::new(48..=72));

        matcher.user_note(Duration::ZERO, 36, true);
        matcher.score_note(Duration::ZERO, 84, true);
        matcher.tick(Duration::from_secs(1));

        assert_eq!(matcher.snapshot(), PracticeSnapshot::default());
    }

    #[test]
    fn duplicate_press_is_counted_as_wrong_without_losing_latest_press() {
        let mut matcher = matcher();
        matcher.user_note(Duration::from_millis(100), 60, true);
        matcher.user_note(Duration::from_millis(200), 60, true);

        assert_eq!(matcher.snapshot().wrong_notes, 1);
        assert_eq!(
            matcher.score_note(Duration::from_millis(250), 60, true),
            Some(MatchedNote {
                note: 60,
                timing: TimingGrade::OnTime,
            })
        );
    }
}
