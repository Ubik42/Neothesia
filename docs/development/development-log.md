# Development log

This is an append-only engineering log. Newest entries go first. Every closed
cycle records the user outcome, implementation, verification, known limitations
and commit.

## 2026-07-25 — Cycle 006: Adaptive tempo coach (DONE)

### Outcome

Loop practice can now progress speed conservatively without taking control away
from the learner. Every automatic decision is bounded, explainable and optional.

### Implemented

- Added persisted adaptive-coach configuration with an opt-in default.
- Added a prominent in-player coach switch.
- Added settings for mastery accuracy and coached speed limits.
- Required two consecutive mastered takes before a 5% increase.
- Required both note accuracy and on-time consistency for mastery.
- Added a 5% regression only for attempts below the safety threshold.
- Held speed for intermediate results and at configured bounds.
- Reset the coach streak whenever the user manually changes speed.
- Added explicit messages explaining each decision for three seconds.
- Fixed playback scaling so 5% steps are not truncated into 10% buckets.
- Rejected non-finite manual speed values in configuration.

### Verification

- Added deterministic coach-decision and exact-speed tests.
- `cargo test -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing platform
helper and `unused_mut` warnings.

Implementation commit: `be7e77e` (`feat: add adaptive tempo coaching`).

### Known limitations

- Coaching is intentionally limited to structured loop attempts.
- Session history and coach progression are not yet persisted per song.
- Decision explanations are textual; richer visual trend feedback is planned.

## 2026-07-25 — Cycle 005: Structured loop practice (DONE)

### Outcome

The timeline loop now behaves as a deliberate-practice tool. Each repetition
starts cleanly, gives preparation time and can be compared with the previous
and best takes.

### Implemented

- Added reusable attempt history with deterministic best-take rules.
- Snapped loop handles to MIDI measure boundaries.
- Chose a two-measure default range around the current playback position.
- Added a one-measure visual four-count before each take.
- Reset practice matching at each loop boundary.
- Displayed take number plus last and best accuracy.
- Added a measure-range label inside the highlighted loop.
- Cleared attempt history when the practised range changes.
- Fixed playback seek semantics to retain events exactly on the target
  boundary, preventing the first beat of a loop from disappearing.

### Verification

- Added attempt-history, snapping, count-in and exact-boundary seek tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing platform
helper and `unused_mut` warnings.

Implementation commit: `243829b`
(`feat: turn loops into structured practice attempts`).

### Known limitations

- The count-in is visual and does not yet produce metronome clicks.
- Attempt history is in-memory until practice-history persistence is delivered.
- The loop workflow still needs a multi-DPI visual smoke test.

## 2026-07-25 — Cycle 004: Measure-aware attempt summary (DONE)

### Outcome

Finishing a song now produces actionable practice feedback instead of silently
returning to the menu. The learner can see which hand and which measures need
another attempt.

### Implemented

- Added structured target context: pitch, score time, track, measure and part.
- Added retained matched, missed and context-attributed wrong-note results.
- Added per-measure and per-hand aggregation.
- Inferred left/right hand for two playable piano tracks from pitch center.
- Kept arrangements with more than two playable tracks unassigned rather than
  guessing incorrectly.
- Replaced automatic song exit with a native completion overlay.
- Displayed total accuracy, timing categories, wrong/missed notes, hand
  accuracy and the four weakest measures.
- Added immediate retry and return-to-song actions.

### Verification

- Added aggregation, hand inference and real-MIDI integration tests.
- `cargo test -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing platform
helper and `unused_mut` warnings.

Implementation commit: `715ed9c`
(`feat: add measure-aware practice summaries`).

### Known limitations

- Summaries exist only for the current process and are not persisted.
- More-than-two-part arrangements require future explicit part assignment.
- The completion overlay still needs a visual smoke test on several DPI scales.

## 2026-07-25 — Cycle 003: Repeated and missed notes (DONE)

### Outcome

Practice results no longer lose repeated same-pitch presses, and flow practice
can distinguish a note that was never played from an unrelated wrong key.

### Implemented

- Replaced one-entry-per-pitch maps with FIFO occurrence queues.
- Matched successive and overlapping same-pitch occurrences independently.
- Added a configurable late-match window and missed-note finalization.
- Fed human-track targets into the matcher in both guided and flow practice.
- Kept guided targets pending while allowing flow targets to expire as missed.
- Included missed and still-needed counts in the live top-bar status.

### Verification

- Added tests for repeated presses, overlapping targets and missed finalization.
- Added application integration tests proving wait mode does not create misses
  and flow mode does.
- `cargo test -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Existing compiler warnings are unchanged and unrelated.

Implementation commit: `fbee89d`
(`feat: score repeated and missed practice notes`).

### Known limitations

- Match outcomes do not yet carry track, hand, score time or measure identity.
- Session totals are not yet finalized into a summary or saved.

## 2026-07-25 — Cycle 002: Deterministic practice feedback (DONE)

### Outcome

Guided practice now has a deterministic, reusable source of live feedback
instead of hidden counters tied directly to the operating-system clock.

### Implemented

- Added `neothesia_core::practice::PracticeMatcher`.
- Made the caller provide monotonic session time so exact timing cases are
  reproducible without sleeps.
- Added configurable early-match and on-time windows.
- Classified correct notes as early, on-time or late.
- Counted expired unmatched and duplicate user presses as wrong.
- Tracked required chord notes without penalizing the order in which they are
  played.
- Added a stable snapshot containing matched, timing, wrong and waiting totals.
- Added a compact live status line next to the wait-mode control.
- Pauses freeze the practice clock; guided waits continue it.
- Normalized live `NoteOn velocity=0` as a release for matching.

### Verification

- Five new deterministic unit tests.
- `cargo test -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Existing compiler warnings are unchanged and unrelated.

Implementation commit: `bab60ea` (`feat: add deterministic practice feedback`).

### Known limitations

- A pitch is still the transient match key, so overlapping occurrences of the
  same pitch are intentionally deferred to `PRA-012`.
- Missed score notes are not finalized yet; wait mode keeps them required.
- Live totals are session-level and not yet grouped by measure or hand.

## 2026-07-25 — Cycle 001: Guided-practice baseline (IN PROGRESS)

### Intended outcome

A pianist can open a prepared two-hand MIDI and immediately use wait-for-notes
practice, while retaining an obvious way to return to automatic playback. The
falling-note view identifies measures and can optionally show quarter-note
subdivisions.

### Implemented

- Added a persisted practice-friendly wait setting with backward-compatible
  defaults.
- Defaulted non-drum note tracks to human practice.
- Added a visible player toggle that changes wait behaviour without reopening
  the song.
- Preserved non-note controllers on human tracks while note targets wait for
  the user.
- Parsed quarter-note timestamps alongside measure timestamps.
- Added configurable quarter-note lines and one-based measure labels.
- Updated free-play, recorder preview and CLI renderer call sites.

### Verification so far

- `cargo fmt --all`
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `git diff --check`

All completed checks passed. The CLI package remains outside the release build
because the local FFmpeg development environment is not configured.

Implementation commit: `a1755bc` (`feat: add guided piano practice baseline`).

### Remaining before close

- Complete a real-device/manual smoke test for open, wait, toggle, loop and exit.

### Known limitations

- Existing statistics are internal and wall-clock based; they are not yet a
  reliable learner-facing score.
- Measure generation currently follows the MIDI parser's existing time-signature
  assumptions and needs broader fixture coverage.
