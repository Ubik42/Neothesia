# Development log

This is an append-only engineering log. Newest entries go first. Every closed
cycle records the user outcome, implementation, verification, known limitations
and commit.

## 2026-07-25 — Cycle 017: Watched-folder indexing and search (DONE)

### Outcome

The Practice Library can now discover a real MIDI collection instead of only
remembering files opened in the past. Scanning does not freeze the renderer,
duplicates do not clutter the list, and a learner can find a piece using any
combination of title or folder terms.

### Implemented

- Added a tested `library` domain independent of scene rendering.
- Added persistent watched-folder configuration with backward-compatible
  defaults and path-aware duplicate prevention.
- Added recursive MIDI discovery without following directory symlinks.
- Parsed candidates on a named worker thread.
- Counted and reported unreadable MIDI while retaining valid results.
- Deduplicated files by content identity and retained alternate source paths.
- Cached normalized searchable text in the index.
- Added type-anywhere multi-term search over filenames and full paths.
- Merged indexed pieces with recent sessions and latest accuracy.
- Added Add Folder and Refresh actions in Practice Library.
- Added watched-folder add/remove controls in Settings.
- Kept direct open and content-verified missing-file repair.

### Verification

- Added recursive scan, duplicate-content, malformed-file, search, config
  uniqueness and settings-migration tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `e993f2c`
(`feat: index and search watched MIDI folders`).

### Known limitations

- The index is rebuilt when the library is opened or refreshed; an incremental
  disk cache and filesystem watcher are future performance work.
- Search currently uses filename and path because editable musical metadata is
  not implemented yet.
- Very large first scans cannot yet be cancelled, although they remain off the
  render thread.
- Favourites, ordered practice queues and mastery filters are next.

## 2026-07-25 — Cycle 016: Recent practice library (DONE)

### Outcome

Previously practised MIDI files are now reachable from inside Neothesia instead
of relying on the operating-system picker every time. A moved file can be
relinked safely, and choosing a different file cannot silently attach the old
practice record to it.

### Implemented

- Added source-path provenance to parsed MIDI files and saved song setup.
- Added last-used timestamps and a sorted recent-song query.
- Added a scrollable Practice Library page and home-screen entry.
- Displayed session count and latest accuracy.
- Loaded recent songs away from the render thread.
- Added missing-file detection and Locate recovery.
- Compared the selected replacement's BLAKE3 content identity before opening.
- Updated the source path only after successful verification.
- Added clear in-page feedback for missing, unreadable and mismatched files.
- Added Unicode-safe title shortening and a more compact home layout.

### Verification

- Added source-path, ordering and Unicode-label tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `e9338da`
(`feat: add recent practice library`).

### Known limitations

- The library currently contains pieces that have already been opened; watched
  folder discovery and text search are next.
- Legacy records without a saved source path require one successful manual
  reopen before they appear as directly available.
- Favourites, queues, metadata editing and bulk missing-file repair remain
  future work.

## 2026-07-25 — Cycle 015: Per-song practice setup (DONE)

### Outcome

Reopening the same MIDI now resumes the learner's working context instead of
silently returning to generic defaults. Renaming or moving the file does not
break the association because the setup uses MIDI content identity.

### Implemented

- Added serializable track, loop and song-setup records.
- Stored setup alongside versioned practice history using the existing atomic
  writer and corruption quarantine.
- Restored mute/automatic/human track roles and waterfall visibility.
- Restored exact playback speed, including zero-speed study state.
- Stored loop boundaries as inclusive one-based measure numbers.
- Restored active loops with count-in and retained disabled loop ranges.
- Saved player changes at every explicit interaction point and adaptive-coach
  speed change.
- Guarded track restoration by exact track-ID structure.
- Guarded loop restoration against invalid or stale measure ranges.
- Maintained compatibility with files that predate setup and hand-scope fields.

### Verification

- Added content-identity, rename, track-structure, loop-boundary and migration
  tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `f1a566f`
(`feat: restore per-song practice setup`).

### Known limitations

- Saved songs do not yet appear in a library browser; reopening still starts
  from the file picker or last-opened path.
- Track layouts are restored only for an exact track-ID structure. A changed
  MIDI is correctly treated as a different song.
- History/setup reset and export controls remain future work.

## 2026-07-25 — Cycle 014: In-player hand practice modes (DONE)

### Outcome

A learner can now move from both-hands practice to right-hand or left-hand work
without leaving the player. The other hand remains musical accompaniment, and
the visible goal, attempt state and saved progress all change together.

### Implemented

- Added a shared `PracticeHands` domain model.
- Added conservative hand-mode detection and mutation to song configuration.
- Added a purple `Hands` player control with responsive placement.
- Kept the opposite hand on automatic playback and preserved unrelated tracks.
- Restarted whole-song practice or the active loop with its count-in.
- Reset keyboard, matcher, attempt comparison and tempo-coach state.
- Disabled shortcuts for MIDI arrangements without reliable hand assignments.
- Added hand scope to the backward-compatible version-one history record.
- Scoped trends, persistent weak measures and recommendations to the latest
  hand goal.
- Exposed hand scope in recent-attempt and trend labels.

### Verification

- Added hand-mode, ambiguous-assignment, scope-isolation and legacy-history
  migration tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings. The full-workspace Clippy command
still requires local FFmpeg development packages for the optional video crate.

Implementation commit: `f630ea4`
(`feat: add in-player hand practice modes`).

### Known limitations

- Automatic hand assignment remains intentionally conservative: MIDI files
  with more than two playable note tracks require explicit track setup.
- The selected hand mode, loop and speed are not yet restored after closing
  the song; this is the next cycle.
- Physical visual and Pianoteq smoke testing still requires the user's active
  desktop and connected instrument.

## 2026-07-25 — Cycle 013: Active output visibility (DONE)

### Outcome

The player now answers “what is producing my sound?” continuously. Built-in
SoundFont, external MIDI/Pianoteq routing and accidental silence have distinct
labels and colours, and the same badge is the shortest path to correcting the
selection.

### Implemented

- Exposed the active output descriptor from the output manager.
- Added backend and detailed learner-facing status labels.
- Added a persistent green, blue or red output badge.
- Displayed the MIDI port name where space permits.
- Added responsive compact mode and removed colliding statistics on narrow
  windows.
- Added Unicode-safe device-label shortening.
- Added a direct player-to-settings event and settings-page constructor.
- Preserved the loaded song across the transition.

### Verification

- Added output-state, default-device and label-truncation tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `41f3f95`
(`feat: show active output with settings shortcut`).

### Known limitations

- Changing an output in settings takes effect when playback is started again;
  live hot-swapping inside an active player remains intentionally unsupported.
- Very long MIDI port names are shortened in the badge but remain complete in
  settings.
- Physical Pianoteq confirmation still requires the documented soak test.

## 2026-07-25 — Cycle 012: Global emergency panic (DONE)

### Outcome

A stuck external note or pedal now has an immediate, discoverable recovery
path. The red player action remains visible while the toolbar is collapsed, and
F12 performs the same emergency stop in every active scene.

### Implemented

- Added the persistent `PANIC F12` player control.
- Added application-level F12 interception before scene key handling.
- Added a scene emergency-stop contract with a safe default.
- Paused song/preview playback and cleared transient practice matching.
- Cancelled visual count-in and reset keyboard plus mouse-held state.
- Cleared free-play chord state.
- Corrected zero-velocity visual releases.

### Verification

- Added emergency-stop state and zero-velocity visual tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commits:

- `2b52463` (`feat: add persistent emergency MIDI panic`)
- `235be25` (`fix: make F12 panic global across scenes`)

### Known limitations

- The visible button is player-specific; free-play uses the globally documented
  F12 shortcut.
- Panic intentionally pauses playback. The learner chooses when to resume or
  restart the current attempt.
- Hardware confirmation still belongs to the Pianoteq soak checklist.

## 2026-07-25 — Cycle 011: Expressive MIDI fidelity (DONE)

### Outcome

The automated boundary now proves that Pianoteq-relevant expressive MIDI is not
silently reduced to notes and binary pedal events. Live input and guided MIDI
playback retain their original values through the external-output boundary.

### Implemented

- Made the deterministic test output capture complete MIDI messages.
- Added live-input forwarding tests.
- Added a parsed synthetic human-track fixture for wait-mode forwarding.
- Covered intermediate CC64 values 23 and 91.
- Covered non-centred 14-bit Pitch Bend and Channel Aftertouch.
- Added exact MIDI wire-byte assertions for controller, bend and pressure.

### Verification

- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `2fda08d`
(`test: prove expressive MIDI output fidelity`).

### Known limitations

- The automated output records and serializes MIDI but cannot hear or inspect a
  physical Pianoteq instance.
- Polyphonic aftertouch, release velocity and every possible controller are not
  exhaustively enumerated; the generic forwarding path is shared.
- Pedal quality feedback is not yet part of practice scoring.

## 2026-07-25 — Cycle 010: External MIDI and Pianoteq safety (DONE)

### Outcome

Pause, seek, restart, teardown and output replacement now use a conservative
silence path suitable for Pianoteq and other external instruments. Pedal-held
sound is no longer left to tracked Note Off events alone.

### Implemented

- Corrected active-note tracking for zero-velocity Note On releases.
- Retained explicit Note Off messages for tracked active notes.
- Added CC64 pedal-up, CC123 All Notes Off, CC120 All Sound Off and CC121 Reset
  All Controllers.
- Sent the panic sequence on all 16 MIDI channels.
- Stopped the old output before selecting a replacement.
- Corrected shared MIDI-connection drop behaviour.
- Added a test output recorder for transport lifecycle verification.
- Added the external Pianoteq routing and acceptance guide.

### Verification

- Added active-note, panic order, channel coverage and player lifecycle tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `daf86c3`
(`fix: harden external MIDI output lifecycle`).

### Known limitations

- Automated tests verify generated MIDI events but cannot prove the behaviour
  of the user's virtual cable and Pianoteq installation.
- The 30-minute device soak checklist remains a manual acceptance gate.
- Native VST3 hosting is still a separate later phase.

## 2026-07-25 — Cycle 009: Current-song practice history (DONE)

### Outcome

The completion experience now distinguishes immediate feedback from long-term
progress. A pianist can inspect recent comparable attempts and persistent weak
measures without leaving the song or reading the raw local history file.

### Implemented

- Added a `This take / History` segmented control to the completion card.
- Added a reusable current-song history overview in the practice domain.
- Listed four recent attempts with scope, accuracy and speed.
- Added accuracy and speed change across comparable attempts.
- Prevented whole-song and loop sessions from being compared as one trend.
- Added a two-column ranking of persistent weak measures.
- Reused the conservative evidence threshold for learner-facing rankings.
- Preserved the recommended-loop action on both tabs.

### Verification

- Added overview ordering, delta, insufficient-sample and mixed-scope tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `ccf3b2f`
(`feat: add current song practice history view`).

### Known limitations

- History is currently entered from whole-song completion, not the player or
  song library.
- Trends are intentionally simple first-to-last deltas rather than a chart or
  statistical confidence estimate.
- Export and reset controls remain deferred until their confirmation and backup
  behaviour is designed.

## 2026-07-25 — Cycle 008: Evidence-based weak-passage practice (DONE)

### Outcome

Persisted practice results now produce a concrete next action. After enough
evidence accumulates, the completion panel explains the weakest passage and can
start a correctly bounded, counted-in loop with one action.

### Implemented

- Added deterministic weak-passage recommendations to the practice domain.
- Required two attempts, eight judged notes and accuracy below 90%.
- Included accuracy, sample size and take count in recommendation evidence.
- Chose a two-measure practice range and clamped it at the song boundary.
- Added a visually prominent recommendation action above the normal completion
  actions.
- Converted one-based inclusive measure ranges into exact MIDI loop boundaries.
- Reused structured attempt reset and count-in behaviour for recommended loops.

### Verification

- Added recommendation threshold and measure-boundary mapping tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `e1d80d0` (`feat: recommend weak passage loops`).

### Known limitations

- The suggestion is available at whole-song completion, not yet from the song
  library or a dedicated history view.
- It uses note accuracy only; timing, pedal and dynamics goals remain separate
  future coaching dimensions.
- Recommended passages are currently two measures long and are not yet editable
  before starting.

## 2026-07-25 — Cycle 007: Durable local practice history (DONE)

### Outcome

Practice results now survive application restarts and MIDI file renames. The
learner's completed attempts form a safe local evidence base for future
weak-passage recommendations instead of disappearing at the end of playback.

### Implemented

- Added BLAKE3 content identities to loaded and generated MIDI files.
- Added a versioned RON schema for whole-song and measure-loop sessions.
- Stored attempt time, speed, overall totals, measures and hand results.
- Recorded loop attempts at each boundary and whole-song attempts at
  completion.
- Added a 200-session per-song retention limit.
- Added deterministic aggregation and ranking of weak measures.
- Wrote history through a temporary file and atomically replaced the live file.
- Used Windows write-through replacement and quarantined malformed files.
- Displayed the saved session count in the completion panel.

### Verification

- Added persistence, rename-continuity, corruption, retention and aggregation
  tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `039a93d` (`feat: persist local practice history`).

### Known limitations

- History has no learner-facing browser, export or reset control yet.
- Weak-measure aggregation exists in the practice domain but is not yet exposed
  as a practice recommendation.
- History follows MIDI content; edited MIDI data intentionally creates a new
  identity.

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
