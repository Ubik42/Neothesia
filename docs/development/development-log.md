# Development log

This is an append-only engineering log. Newest entries go first. Every closed
cycle records the user outcome, implementation, verification, known limitations
and commit.

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
