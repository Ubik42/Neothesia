use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    time::Duration,
};

use piano_layout::KeyboardRange;
use serde::{Deserialize, Serialize};

pub type NoteId = u8;

#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PracticeHands {
    Both,
    Right,
    Left,
    Custom,
    #[default]
    Unspecified,
}

impl PracticeHands {
    pub fn label(self) -> &'static str {
        match self {
            Self::Both => "Both",
            Self::Right => "Right",
            Self::Left => "Left",
            Self::Custom => "Custom",
            Self::Unspecified => "Unspecified",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Both => Self::Right,
            Self::Right => Self::Left,
            Self::Left | Self::Custom | Self::Unspecified => Self::Both,
        }
    }
}

#[derive(
    Debug, Clone, Copy, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum PracticePart {
    LeftHand,
    RightHand,
    #[default]
    Other,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PracticeTarget {
    pub note: NoteId,
    pub velocity: u8,
    pub score_time: Duration,
    pub track_id: usize,
    /// One-based measure number. Zero means that measure context is unknown.
    pub measure: usize,
    pub part: PracticePart,
}

impl PracticeTarget {
    pub fn unknown(note: NoteId) -> Self {
        Self {
            note,
            velocity: 0,
            score_time: Duration::ZERO,
            track_id: 0,
            measure: 0,
            part: PracticePart::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PracticeWindows {
    /// A user note may wait this long for its matching score note.
    pub early_match: Duration,
    /// A matched note inside this window is considered on time.
    pub on_time: Duration,
    /// In flow practice, an expected note becomes missed after this delay.
    pub late_match: Duration,
}

impl Default for PracticeWindows {
    fn default() -> Self {
        Self {
            early_match: Duration::from_millis(500),
            on_time: Duration::from_millis(80),
            late_match: Duration::from_millis(500),
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PracticeJudgement {
    Matched(TimingGrade),
    Missed,
    Wrong { played_note: NoteId },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PracticeResult {
    pub target: Option<PracticeTarget>,
    pub judgement: PracticeJudgement,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PracticeSnapshot {
    pub matched_notes: usize,
    pub on_time_notes: usize,
    pub early_notes: usize,
    pub late_notes: usize,
    pub wrong_notes: usize,
    pub missed_notes: usize,
    pub required_notes: usize,
}

impl PracticeSnapshot {
    pub fn accuracy(self) -> Option<f32> {
        let judged = self.matched_notes + self.wrong_notes + self.missed_notes;
        (judged != 0).then(|| self.matched_notes as f32 / judged as f32)
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PracticeBreakdown {
    pub target_notes: usize,
    pub matched_notes: usize,
    pub on_time_notes: usize,
    pub early_notes: usize,
    pub late_notes: usize,
    pub wrong_notes: usize,
    pub missed_notes: usize,
}

impl PracticeBreakdown {
    pub fn accuracy(self) -> Option<f32> {
        let judged = self.matched_notes + self.wrong_notes + self.missed_notes;
        (judged != 0).then(|| self.matched_notes as f32 / judged as f32)
    }

    fn record(&mut self, judgement: PracticeJudgement) {
        match judgement {
            PracticeJudgement::Matched(timing) => {
                self.target_notes += 1;
                self.matched_notes += 1;
                match timing {
                    TimingGrade::Early(_) => self.early_notes += 1,
                    TimingGrade::OnTime => self.on_time_notes += 1,
                    TimingGrade::Late(_) => self.late_notes += 1,
                }
            }
            PracticeJudgement::Missed => {
                self.target_notes += 1;
                self.missed_notes += 1;
            }
            PracticeJudgement::Wrong { .. } => self.wrong_notes += 1,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct MeasureSummary {
    pub measure: usize,
    pub breakdown: PracticeBreakdown,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct PartSummary {
    pub part: PracticePart,
    pub breakdown: PracticeBreakdown,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
pub struct AttemptSummary {
    pub overall: PracticeSnapshot,
    pub measures: Vec<MeasureSummary>,
    pub parts: Vec<PartSummary>,
    #[serde(default)]
    pub expression: ExpressionSummary,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct VelocitySummary {
    pub matched_samples: usize,
    pub mean_abs_difference: Option<u8>,
    pub played_min: Option<u8>,
    pub played_max: Option<u8>,
    pub target_min: Option<u8>,
    pub target_max: Option<u8>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PedalSummary {
    pub user_changes: usize,
    pub target_changes: usize,
    pub user_used: bool,
    pub target_present: bool,
    pub user_continuous_samples: usize,
    pub target_continuous_samples: usize,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExpressionSummary {
    pub velocity: VelocitySummary,
    pub pedal: PedalSummary,
}

impl ExpressionSummary {
    pub fn has_velocity_evidence(self) -> bool {
        self.velocity.matched_samples >= 4
    }

    pub fn has_pedal_evidence(self) -> bool {
        self.pedal.user_used || self.pedal.target_present
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AttemptHistory {
    completed: usize,
    last: Option<AttemptSummary>,
    best: Option<AttemptSummary>,
}

impl AttemptHistory {
    pub fn record(&mut self, summary: AttemptSummary) {
        self.completed += 1;
        if self
            .best
            .as_ref()
            .is_none_or(|best| is_better_attempt(&summary, best))
        {
            self.best = Some(summary.clone());
        }
        self.last = Some(summary);
    }

    /// One-based number of the attempt currently being practised.
    pub fn current_attempt(&self) -> usize {
        self.completed + 1
    }

    pub fn completed(&self) -> usize {
        self.completed
    }

    pub fn last(&self) -> Option<&AttemptSummary> {
        self.last.as_ref()
    }

    pub fn best(&self) -> Option<&AttemptSummary> {
        self.best.as_ref()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

fn is_better_attempt(candidate: &AttemptSummary, best: &AttemptSummary) -> bool {
    let candidate_accuracy = candidate.overall.accuracy().unwrap_or(0.0);
    let best_accuracy = best.overall.accuracy().unwrap_or(0.0);

    candidate_accuracy > best_accuracy
        || (candidate_accuracy == best_accuracy
            && candidate.overall.on_time_notes > best.overall.on_time_notes)
        || (candidate_accuracy == best_accuracy
            && candidate.overall.on_time_notes == best.overall.on_time_notes
            && candidate.overall.matched_notes > best.overall.matched_notes)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveTempoRules {
    pub mastery_accuracy: f32,
    pub mastery_timing: f32,
    pub struggle_accuracy: f32,
    pub step: f32,
    pub min_speed: f32,
    pub max_speed: f32,
    pub mastery_takes: usize,
}

impl Default for AdaptiveTempoRules {
    fn default() -> Self {
        Self {
            mastery_accuracy: 0.9,
            mastery_timing: 0.7,
            struggle_accuracy: 0.7,
            step: 0.05,
            min_speed: 0.5,
            max_speed: 1.0,
            mastery_takes: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AdaptiveTempoReason {
    NoJudgedNotes,
    BuildingMastery { completed: usize, required: usize },
    Mastered,
    NeedsAccuracy,
    KeepPractising,
    AtMaximum,
    AtMinimum,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveTempoDecision {
    pub previous_speed: f32,
    pub speed: f32,
    pub reason: AdaptiveTempoReason,
}

impl AdaptiveTempoDecision {
    pub fn changed(self) -> bool {
        self.previous_speed != self.speed
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct AdaptiveTempoCoach {
    mastery_streak: usize,
}

impl AdaptiveTempoCoach {
    pub fn evaluate(
        &mut self,
        summary: &AttemptSummary,
        current_speed: f32,
        rules: AdaptiveTempoRules,
    ) -> AdaptiveTempoDecision {
        let accuracy = summary.overall.accuracy();
        let timing = (summary.overall.matched_notes != 0)
            .then(|| summary.overall.on_time_notes as f32 / summary.overall.matched_notes as f32);
        let min_speed = rules.min_speed.min(rules.max_speed);
        let max_speed = rules.max_speed.max(rules.min_speed);
        let current_speed = current_speed.clamp(min_speed, max_speed);

        let Some(accuracy) = accuracy else {
            self.mastery_streak = 0;
            return decision(
                current_speed,
                current_speed,
                AdaptiveTempoReason::NoJudgedNotes,
            );
        };

        let mastered = accuracy >= rules.mastery_accuracy
            && timing.is_some_and(|timing| timing >= rules.mastery_timing);
        if mastered {
            self.mastery_streak += 1;
            let required = rules.mastery_takes.max(1);
            if self.mastery_streak < required {
                return decision(
                    current_speed,
                    current_speed,
                    AdaptiveTempoReason::BuildingMastery {
                        completed: self.mastery_streak,
                        required,
                    },
                );
            }

            self.mastery_streak = 0;
            if current_speed >= max_speed {
                return decision(current_speed, current_speed, AdaptiveTempoReason::AtMaximum);
            }
            return decision(
                current_speed,
                round_speed((current_speed + rules.step).min(max_speed)),
                AdaptiveTempoReason::Mastered,
            );
        }

        self.mastery_streak = 0;
        if accuracy < rules.struggle_accuracy {
            if current_speed <= min_speed {
                return decision(current_speed, current_speed, AdaptiveTempoReason::AtMinimum);
            }
            return decision(
                current_speed,
                round_speed((current_speed - rules.step).max(min_speed)),
                AdaptiveTempoReason::NeedsAccuracy,
            );
        }

        decision(
            current_speed,
            current_speed,
            AdaptiveTempoReason::KeepPractising,
        )
    }

    pub fn reset(&mut self) {
        self.mastery_streak = 0;
    }

    pub fn mastery_streak(&self) -> usize {
        self.mastery_streak
    }
}

fn decision(previous_speed: f32, speed: f32, reason: AdaptiveTempoReason) -> AdaptiveTempoDecision {
    AdaptiveTempoDecision {
        previous_speed,
        speed,
        reason,
    }
}

fn round_speed(speed: f32) -> f32 {
    (speed * 100.0).round() / 100.0
}

#[derive(Debug, Clone, Copy)]
struct NotePress {
    timestamp: Duration,
    note: NoteId,
    velocity: u8,
}

#[derive(Debug, Clone, Copy)]
struct TargetPress {
    timestamp: Duration,
    target: PracticeTarget,
}

#[derive(Debug, Default)]
struct PedalEvidence {
    last_value: Option<u8>,
    changes: usize,
    used: bool,
    continuous_samples: usize,
}

impl PedalEvidence {
    fn record(&mut self, value: u8) {
        if self.last_value == Some(value) {
            return;
        }
        if self.last_value.is_some() {
            self.changes += 1;
        }
        self.last_value = Some(value);
        self.used |= value > 0;
        self.continuous_samples += usize::from((1..127).contains(&value));
    }
}

#[derive(Debug, Default)]
struct ExpressionTracker {
    velocity_samples: usize,
    velocity_difference_sum: u64,
    played_min: Option<u8>,
    played_max: Option<u8>,
    target_min: Option<u8>,
    target_max: Option<u8>,
    user_pedal: PedalEvidence,
    target_pedal: PedalEvidence,
}

impl ExpressionTracker {
    fn record_velocity(&mut self, played: u8, target: u8) {
        if played == 0 || target == 0 {
            return;
        }
        self.velocity_samples += 1;
        self.velocity_difference_sum += u64::from(played.abs_diff(target));
        update_min_max(&mut self.played_min, &mut self.played_max, played);
        update_min_max(&mut self.target_min, &mut self.target_max, target);
    }

    fn summary(&self) -> ExpressionSummary {
        ExpressionSummary {
            velocity: VelocitySummary {
                matched_samples: self.velocity_samples,
                mean_abs_difference: (self.velocity_samples != 0).then(|| {
                    (self.velocity_difference_sum / self.velocity_samples as u64)
                        .min(u64::from(u8::MAX)) as u8
                }),
                played_min: self.played_min,
                played_max: self.played_max,
                target_min: self.target_min,
                target_max: self.target_max,
            },
            pedal: PedalSummary {
                user_changes: self.user_pedal.changes,
                target_changes: self.target_pedal.changes,
                user_used: self.user_pedal.used,
                target_present: self.target_pedal.last_value.is_some(),
                user_continuous_samples: self.user_pedal.continuous_samples,
                target_continuous_samples: self.target_pedal.continuous_samples,
            },
        }
    }
}

fn update_min_max(minimum: &mut Option<u8>, maximum: &mut Option<u8>, value: u8) {
    *minimum = Some(minimum.map_or(value, |current| current.min(value)));
    *maximum = Some(maximum.map_or(value, |current| current.max(value)));
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

    required_notes: HashMap<NoteId, VecDeque<TargetPress>>,
    user_pressed_recently: HashMap<NoteId, VecDeque<NotePress>>,

    snapshot: PracticeSnapshot,
    results: Vec<PracticeResult>,
    last_target: Option<PracticeTarget>,
    expression: ExpressionTracker,
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
            snapshot: PracticeSnapshot::default(),
            results: Vec::new(),
            last_target: None,
            expression: ExpressionTracker::default(),
        }
    }

    pub fn tick(&mut self, now: Duration) {
        let context = self.current_context();
        let expired = drain_expired(
            &mut self.user_pressed_recently,
            now,
            self.windows.early_match,
        );
        self.snapshot.wrong_notes += expired.len();
        self.results
            .extend(expired.into_iter().map(|press| PracticeResult {
                target: context,
                judgement: PracticeJudgement::Wrong {
                    played_note: press.note,
                },
            }));
    }

    /// Finalizes score notes that were not played in the allowed late window.
    ///
    /// Guided wait mode should not call this: its required notes intentionally
    /// remain pending until the pianist plays them.
    pub fn finalize_missed(&mut self, now: Duration) {
        let expired = drain_expired(&mut self.required_notes, now, self.windows.late_match);
        self.snapshot.missed_notes += expired.len();
        self.results
            .extend(expired.into_iter().map(|press| PracticeResult {
                target: Some(press.target),
                judgement: PracticeJudgement::Missed,
            }));
        self.update_required_count();
    }

    pub fn user_note(&mut self, now: Duration, note: NoteId, active: bool) -> Option<MatchedNote> {
        self.user_note_with_velocity(now, note, active, 100)
    }

    pub fn user_note_with_velocity(
        &mut self,
        now: Duration,
        note: NoteId,
        active: bool,
        velocity: u8,
    ) -> Option<MatchedNote> {
        if !active || !self.user_keyboard_range.contains(note) {
            return None;
        }

        if let Some(required) = pop_front(&mut self.required_notes, note) {
            self.update_required_count();
            return Some(self.record_match(
                required.target,
                TimingGrade::Late(now.saturating_sub(required.timestamp)),
                velocity,
            ));
        }

        self.user_pressed_recently
            .entry(note)
            .or_default()
            .push_back(NotePress {
                timestamp: now,
                note,
                velocity,
            });

        None
    }

    pub fn score_note(&mut self, now: Duration, note: NoteId, active: bool) -> Option<MatchedNote> {
        self.score_target(now, PracticeTarget::unknown(note), active)
    }

    pub fn score_target(
        &mut self,
        now: Duration,
        target: PracticeTarget,
        active: bool,
    ) -> Option<MatchedNote> {
        let note = target.note;
        if !self.user_keyboard_range.contains(note) {
            return None;
        }

        if !active {
            return None;
        }

        self.last_target = Some(target);

        if let Some(press) = pop_front(&mut self.user_pressed_recently, note) {
            return Some(self.record_match(
                target,
                TimingGrade::Early(now.saturating_sub(press.timestamp)),
                press.velocity,
            ));
        }

        self.required_notes
            .entry(note)
            .or_default()
            .push_back(TargetPress {
                timestamp: now,
                target,
            });
        self.update_required_count();
        None
    }

    pub fn user_pedal(&mut self, value: u8) {
        self.expression.user_pedal.record(value);
    }

    pub fn score_pedal(&mut self, value: u8) {
        self.expression.target_pedal.record(value);
    }

    pub fn snapshot(&self) -> PracticeSnapshot {
        self.snapshot
    }

    pub fn results(&self) -> &[PracticeResult] {
        &self.results
    }

    pub fn finish(&mut self) {
        let context = self.current_context();
        for (_, queue) in self.user_pressed_recently.drain() {
            for press in queue {
                self.snapshot.wrong_notes += 1;
                self.results.push(PracticeResult {
                    target: context,
                    judgement: PracticeJudgement::Wrong {
                        played_note: press.note,
                    },
                });
            }
        }

        for (_, queue) in self.required_notes.drain() {
            for press in queue {
                self.snapshot.missed_notes += 1;
                self.results.push(PracticeResult {
                    target: Some(press.target),
                    judgement: PracticeJudgement::Missed,
                });
            }
        }
        self.snapshot.required_notes = 0;
    }

    pub fn summary(&self) -> AttemptSummary {
        let mut measures = BTreeMap::<usize, PracticeBreakdown>::new();
        let mut parts = BTreeMap::<PracticePart, PracticeBreakdown>::new();

        for result in &self.results {
            let Some(target) = result.target else {
                continue;
            };

            if target.measure != 0 {
                measures
                    .entry(target.measure)
                    .or_default()
                    .record(result.judgement);
            }
            parts
                .entry(target.part)
                .or_default()
                .record(result.judgement);
        }

        AttemptSummary {
            overall: self.snapshot,
            measures: measures
                .into_iter()
                .map(|(measure, breakdown)| MeasureSummary { measure, breakdown })
                .collect(),
            parts: parts
                .into_iter()
                .map(|(part, breakdown)| PartSummary { part, breakdown })
                .collect(),
            expression: self.expression.summary(),
        }
    }

    pub fn are_required_notes_pressed(&self) -> bool {
        self.required_notes.is_empty()
    }

    /// Clears transient matching state while retaining the session totals.
    pub fn clear_pending(&mut self) {
        self.required_notes.clear();
        self.user_pressed_recently.clear();
        self.snapshot.required_notes = 0;
        self.last_target = None;
    }

    pub fn reset(&mut self) {
        self.clear_pending();
        self.snapshot = PracticeSnapshot::default();
        self.results.clear();
        self.expression = ExpressionTracker::default();
    }

    fn record_match(
        &mut self,
        target: PracticeTarget,
        timing: TimingGrade,
        played_velocity: u8,
    ) -> MatchedNote {
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

        self.results.push(PracticeResult {
            target: Some(target),
            judgement: PracticeJudgement::Matched(timing),
        });
        self.expression
            .record_velocity(played_velocity, target.velocity);

        MatchedNote {
            note: target.note,
            timing,
        }
    }

    fn update_required_count(&mut self) {
        self.snapshot.required_notes = self.required_notes.values().map(VecDeque::len).sum();
    }

    fn current_context(&self) -> Option<PracticeTarget> {
        self.required_notes
            .values()
            .filter_map(|queue| queue.front())
            .min_by_key(|press| press.timestamp)
            .map(|press| press.target)
            .or(self.last_target)
    }
}

trait Timestamped {
    fn timestamp(&self) -> Duration;
}

impl Timestamped for NotePress {
    fn timestamp(&self) -> Duration {
        self.timestamp
    }
}

impl Timestamped for TargetPress {
    fn timestamp(&self) -> Duration {
        self.timestamp
    }
}

fn pop_front<T>(notes: &mut HashMap<NoteId, VecDeque<T>>, note: NoteId) -> Option<T> {
    let queue = notes.get_mut(&note)?;
    let press = queue.pop_front();
    if queue.is_empty() {
        notes.remove(&note);
    }
    press
}

fn drain_expired<T: Timestamped>(
    notes: &mut HashMap<NoteId, VecDeque<T>>,
    now: Duration,
    window: Duration,
) -> Vec<T> {
    let mut expired = Vec::new();
    notes.retain(|_, queue| {
        while queue
            .front()
            .is_some_and(|press| now.saturating_sub(press.timestamp()) > window)
        {
            if let Some(press) = queue.pop_front() {
                expired.push(press);
            }
        }
        !queue.is_empty()
    });
    expired
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
                late_match: Duration::from_millis(500),
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
                missed_notes: 0,
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
    fn repeated_same_pitch_presses_match_distinct_score_occurrences() {
        let mut matcher = matcher();
        matcher.user_note(Duration::from_millis(100), 60, true);
        matcher.user_note(Duration::from_millis(200), 60, true);

        assert_eq!(
            matcher.score_note(Duration::from_millis(250), 60, true),
            Some(MatchedNote {
                note: 60,
                timing: TimingGrade::Early(Duration::from_millis(150)),
            })
        );
        assert_eq!(
            matcher.score_note(Duration::from_millis(260), 60, true),
            Some(MatchedNote {
                note: 60,
                timing: TimingGrade::OnTime,
            })
        );
        assert_eq!(matcher.snapshot().matched_notes, 2);
        assert_eq!(matcher.snapshot().wrong_notes, 0);
    }

    #[test]
    fn overlapping_same_pitch_targets_keep_separate_occurrences() {
        let mut matcher = matcher();
        matcher.score_note(Duration::from_millis(100), 60, true);
        matcher.score_note(Duration::from_millis(200), 60, true);

        assert_eq!(matcher.snapshot().required_notes, 2);
        matcher.user_note(Duration::from_millis(300), 60, true);
        assert_eq!(matcher.snapshot().required_notes, 1);
        matcher.user_note(Duration::from_millis(400), 60, true);
        assert_eq!(matcher.snapshot().required_notes, 0);
    }

    #[test]
    fn flow_practice_finalizes_late_targets_as_missed() {
        let mut matcher = matcher();
        matcher.score_note(Duration::from_secs(1), 60, true);
        matcher.score_note(Duration::from_millis(1_200), 64, true);

        matcher.finalize_missed(Duration::from_millis(1_500));
        assert_eq!(matcher.snapshot().missed_notes, 0);
        assert_eq!(matcher.snapshot().required_notes, 2);

        matcher.finalize_missed(Duration::from_millis(1_501));
        assert_eq!(matcher.snapshot().missed_notes, 1);
        assert_eq!(matcher.snapshot().required_notes, 1);
        assert_eq!(matcher.snapshot().accuracy(), Some(0.0));
    }

    #[test]
    fn summary_groups_results_by_measure_and_hand() {
        let mut matcher = matcher();
        let right = PracticeTarget {
            note: 72,
            velocity: 90,
            score_time: Duration::from_secs(2),
            track_id: 1,
            measure: 3,
            part: PracticePart::RightHand,
        };
        let left = PracticeTarget {
            note: 48,
            velocity: 70,
            score_time: Duration::from_secs(4),
            track_id: 2,
            measure: 5,
            part: PracticePart::LeftHand,
        };

        matcher.score_target(Duration::from_secs(2), right, true);
        matcher.user_note(Duration::from_millis(2_040), 72, true);

        matcher.score_target(Duration::from_secs(4), left, true);
        matcher.user_note(Duration::from_millis(4_100), 49, true);
        matcher.tick(Duration::from_millis(4_601));
        matcher.finish();

        let summary = matcher.summary();
        assert_eq!(summary.overall.matched_notes, 1);
        assert_eq!(summary.overall.wrong_notes, 1);
        assert_eq!(summary.overall.missed_notes, 1);

        let measure_3 = summary
            .measures
            .iter()
            .find(|item| item.measure == 3)
            .unwrap();
        assert_eq!(measure_3.breakdown.matched_notes, 1);
        assert_eq!(measure_3.breakdown.on_time_notes, 1);

        let measure_5 = summary
            .measures
            .iter()
            .find(|item| item.measure == 5)
            .unwrap();
        assert_eq!(measure_5.breakdown.missed_notes, 1);
        assert_eq!(measure_5.breakdown.wrong_notes, 1);

        let left_hand = summary
            .parts
            .iter()
            .find(|item| item.part == PracticePart::LeftHand)
            .unwrap();
        assert_eq!(left_hand.breakdown.missed_notes, 1);
        assert_eq!(left_hand.breakdown.wrong_notes, 1);
    }

    #[test]
    fn expression_summary_pairs_velocity_and_tracks_pedal_evidence() {
        let mut matcher = matcher();
        let target = |note, velocity, millis| PracticeTarget {
            note,
            velocity,
            score_time: Duration::from_millis(millis),
            track_id: 1,
            measure: 1,
            part: PracticePart::RightHand,
        };

        matcher.score_target(Duration::from_millis(100), target(60, 40, 100), true);
        matcher.user_note_with_velocity(Duration::from_millis(110), 60, true, 55);
        matcher.user_note_with_velocity(Duration::from_millis(190), 62, true, 100);
        matcher.score_target(Duration::from_millis(200), target(62, 80, 200), true);
        matcher.score_target(Duration::from_millis(300), target(64, 70, 300), true);
        matcher.user_note_with_velocity(Duration::from_millis(310), 64, true, 60);
        matcher.score_target(Duration::from_millis(400), target(65, 90, 400), true);
        matcher.user_note_with_velocity(Duration::from_millis(410), 65, true, 85);

        matcher.user_pedal(0);
        matcher.user_pedal(64);
        matcher.user_pedal(127);
        matcher.user_pedal(127);
        matcher.score_pedal(0);
        matcher.score_pedal(127);

        let expression = matcher.summary().expression;
        assert!(expression.has_velocity_evidence());
        assert!(expression.has_pedal_evidence());
        assert_eq!(
            expression.velocity,
            VelocitySummary {
                matched_samples: 4,
                mean_abs_difference: Some(12),
                played_min: Some(55),
                played_max: Some(100),
                target_min: Some(40),
                target_max: Some(90),
            }
        );
        assert_eq!(
            expression.pedal,
            PedalSummary {
                user_changes: 2,
                target_changes: 1,
                user_used: true,
                target_present: true,
                user_continuous_samples: 1,
                target_continuous_samples: 0,
            }
        );
    }

    #[test]
    fn expression_evidence_resets_between_attempts() {
        let mut matcher = matcher();
        matcher.user_pedal(127);
        matcher.score_pedal(0);
        matcher.reset();

        assert_eq!(matcher.summary().expression, ExpressionSummary::default());
    }

    #[test]
    fn attempt_history_keeps_last_and_best_results() {
        let summary = |matched, on_time, wrong, missed| AttemptSummary {
            overall: PracticeSnapshot {
                matched_notes: matched,
                on_time_notes: on_time,
                wrong_notes: wrong,
                missed_notes: missed,
                ..PracticeSnapshot::default()
            },
            ..AttemptSummary::default()
        };

        let mut history = AttemptHistory::default();
        history.record(summary(8, 5, 2, 0));
        history.record(summary(9, 6, 1, 0));
        history.record(summary(8, 8, 2, 0));

        assert_eq!(history.completed(), 3);
        assert_eq!(history.current_attempt(), 4);
        assert_eq!(history.last().unwrap().overall.on_time_notes, 8);
        assert_eq!(history.best().unwrap().overall.matched_notes, 9);
    }

    #[test]
    fn attempt_history_uses_on_time_notes_as_accuracy_tiebreaker() {
        let mut history = AttemptHistory::default();
        let attempt = |on_time| AttemptSummary {
            overall: PracticeSnapshot {
                matched_notes: 8,
                on_time_notes: on_time,
                wrong_notes: 2,
                ..PracticeSnapshot::default()
            },
            ..AttemptSummary::default()
        };

        history.record(attempt(4));
        history.record(attempt(7));

        assert_eq!(history.best().unwrap().overall.on_time_notes, 7);
    }

    fn tempo_summary(matched: usize, on_time: usize, wrong: usize) -> AttemptSummary {
        AttemptSummary {
            overall: PracticeSnapshot {
                matched_notes: matched,
                on_time_notes: on_time,
                wrong_notes: wrong,
                ..PracticeSnapshot::default()
            },
            ..AttemptSummary::default()
        }
    }

    #[test]
    fn adaptive_tempo_requires_two_mastered_takes_before_increasing() {
        let mut coach = AdaptiveTempoCoach::default();
        let rules = AdaptiveTempoRules::default();
        let mastered = tempo_summary(19, 16, 1);

        let first = coach.evaluate(&mastered, 0.7, rules);
        assert_eq!(first.speed, 0.7);
        assert_eq!(
            first.reason,
            AdaptiveTempoReason::BuildingMastery {
                completed: 1,
                required: 2,
            }
        );

        let second = coach.evaluate(&mastered, 0.7, rules);
        assert_eq!(second.speed, 0.75);
        assert_eq!(second.reason, AdaptiveTempoReason::Mastered);
    }

    #[test]
    fn adaptive_tempo_reduces_speed_after_a_weak_take() {
        let mut coach = AdaptiveTempoCoach::default();
        let weak = tempo_summary(6, 5, 4);

        let decision = coach.evaluate(&weak, 0.8, AdaptiveTempoRules::default());

        assert_eq!(decision.speed, 0.75);
        assert_eq!(decision.reason, AdaptiveTempoReason::NeedsAccuracy);
    }

    #[test]
    fn adaptive_tempo_holds_moderate_take_and_respects_limits() {
        let mut coach = AdaptiveTempoCoach::default();
        let moderate = tempo_summary(8, 6, 2);
        let weak = tempo_summary(1, 1, 9);

        let hold = coach.evaluate(&moderate, 0.8, AdaptiveTempoRules::default());
        assert_eq!(hold.speed, 0.8);
        assert_eq!(hold.reason, AdaptiveTempoReason::KeepPractising);

        let minimum = coach.evaluate(&weak, 0.5, AdaptiveTempoRules::default());
        assert_eq!(minimum.speed, 0.5);
        assert_eq!(minimum.reason, AdaptiveTempoReason::AtMinimum);
    }
}
