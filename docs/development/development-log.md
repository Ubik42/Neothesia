# Development log

This is an append-only engineering log. Newest entries go first. Every closed
cycle records the user outcome, implementation, verification, known limitations
and commit.

## 2026-07-25 — Cycle 041: Persistent exercise choices (DONE)

### Outcome

Technique Studio now behaves like a durable practice workspace. Every choice
can move backward or forward explicitly, and the next launch resumes the last
exercise the learner actually started.

### Implemented

- Replaced click-to-cycle cards with visible previous and next buttons.
- Added bidirectional wrapping for all twelve keys, three patterns, three
  directions, three hand modes, three octave spans and the graduated tempo
  list.
- Preserved a clear centered value panel between each arrow pair.
- Added stable semantic action IDs to all fourteen selector buttons.
- Used one adjustment implementation for native controls and automated tests.
- Added `last_exercise_spec` to the versioned history/settings section.
- Saved the specification only after generation succeeds and the learner
  starts the exercise.
- Restored the saved specification in new menu scenes.
- Added public structural validation to `ExerciseSpec`.
- Fell back to the default exercise if persisted numeric values are invalid.
- Kept legacy settings compatible through Serde defaults.
- Extended the native smoke fixture to select C♯ and inspect the generated
  required notes.
- Inspected the settings file after clean process exit to verify persistence.

### Verification

- Previous/next unit coverage for key, pattern, direction, hands, octaves and
  tempo boundary wrapping.
- Full specification RON serialization round trip.
- Legacy history settings load without `last_exercise_spec`.
- Invalid tonic 99 falls back safely.
- Semantic action IDs remain unique and namespaced.
- Real-process fixture confirms:
  - C♯ selection is accepted;
  - first required notes are MIDI 37 and 61;
  - played notes score;
  - hand and loop controls still work;
  - `settings.ron` contains tonic 1;
  - process exits with code 0.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, sixty core tests and fifty-two
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `e613901`
(`feat: persist technique studio choices`).

### Known limitations

- Only the last-started exercise is persisted; named favourites and a recent
  variant list remain under `EX-002`.
- The selector page does not yet show due/review evidence for an exercise.
- Fingering remains deferred pending reviewed per-key/per-hand tables.

## 2026-07-25 — Cycle 040: Technique Studio (DONE)

### Outcome

The exercise mode is now usable from the home screen. A learner can choose a
musical target, generate it instantly and practise it with the same
wait-for-notes, scoring, looping and feedback system used for repertoire.

### Implemented

- Added a Technique Studio home-screen action.
- Added a dedicated native exercise page with a compact two-column layout.
- Added clickable choices for:
  - all twelve tonic pitch classes;
  - major or minor;
  - scale, arpeggio or primary chords;
  - ascending, descending or up and down;
  - right, left or both hands;
  - one, two or three octaves;
  - a graduated 30–200 BPM practice range.
- Added a live descriptive preset name.
- Added Start Exercise and Enter-key launch paths.
- Added Back and Escape return paths.
- Generated against the user's configured keyboard range and displayed
  generation errors without leaving the page.
- Added `Song::from_exercise` so generated tracks receive explicit hand
  identity, including single-hand exercises.
- Reused the existing MIDI output connection, Pianoteq routing, wait mode,
  practice matcher, loops, hand controls and completion flow.
- Added stable semantic actions for opening Technique Studio and starting the
  exercise.
- Added an `-ExerciseFixture` native smoke path.
- Tightened the home layout so the extra action remains inside the supported
  minimum window height.

### Verification

- Unit test for cyclic pattern, direction, hands and tempo choices.
- Integration tests for generated both-hand and single-hand song setup.
- Real-process automation:
  - opened Technique Studio from a clean menu;
  - generated and started the default C-major exercise;
  - confirmed wait-for-notes defaults on;
  - confirmed Both-hand mode;
  - injected required C notes and scored two matches;
  - switched to Right hand;
  - enabled a valid 1–2 measure loop;
  - restarted without losing the loop;
  - disabled the loop, returned to the menu and exited with code 0.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty-eight core tests and fifty-two
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `017697d`
(`feat: add technique studio exercise flow`).

### Known limitations

- Exercise choices are kept only for the current menu session.
- Generated variants are not yet presented as a distinct source type in
  Practice Library.
- Clicking cycles choices forward; richer keyboard and decrement controls are
  still to come.
- Fingering remains deferred pending reviewed per-key/per-hand tables.

## 2026-07-25 — Cycle 039: Exercise plans as playable MIDI (DONE)

### Outcome

Generated exercises are now valid songs rather than isolated note lists. A
scale, arpeggio or chord plan can enter Neothesia's existing timing, rendering,
wait-mode and scoring pipeline with its hands recognized automatically.

### Implemented

- Converted `ExercisePlan` to memory-backed Type-1 MIDI.
- Added a 480-PPQ conductor track with requested tempo and 4/4 meter.
- Added separate named tracks and MIDI channels for right and left hands.
- Used an 80% note gate so successive exercise notes retain audible separation.
- Preserved one-beat melodic steps and two-beat chord steps.
- Generated concise names containing key, tonality, pattern, hand scope and
  tempo.
- Kept source paths empty for generated material.
- Produced deterministic content identities from the generated MIDI.
- Verified both-hand track pitch centers through the normal `SongConfig`
  inference path.
- Reused the existing Both, Right and Left practice-hand controls.

### Verification

- Stable Type-1 format, track count, hand roots and note-count test.
- Exact note-duration tests at 60 and 120 BPM.
- Stable-identity and tempo-sensitive-identity tests.
- Application integration test for generated-song hand recognition.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty-eight core tests and fifty
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `7fe91f4`
(`feat: convert exercise plans to MIDI`).

### Known limitations

- Exercise creation is not exposed in the menu yet.
- Generated exercises still need explicit source metadata before history and
  repertoire can distinguish them without relying on content identity.
- Single-hand plans intentionally cannot offer a two-hand toggle.
- Fingering remains deferred pending reviewed per-key/per-hand tables.

## 2026-07-25 — Cycle 038: Deterministic exercise plans (DONE)

### Outcome

Exercise mode now has a tested musical domain model instead of a placeholder
screen. It can deterministically describe playable scale, arpeggio and
primary-chord material that will enter the existing practice player in the
next integration layer.

### Implemented

- Added `neothesia_core::exercise`.
- Added exercise pattern types for Scale, Arpeggio and Primary Chords.
- Added Major and Minor tonalities.
- Added Ascending, Descending and Up-and-Down directions.
- Added Right, Left and Both hand scopes.
- Added a serializable specification with tonic, one-to-three octaves and
  20–240 BPM.
- Generated:
  - major and natural-minor scale intervals;
  - tonic major/minor arpeggios;
  - I–IV–V–I major cadences;
  - i–iv–V–i minor cadences with a functional major dominant.
- Positioned right-hand tonic at C4–B4 and left-hand tonic at C2–B2.
- Kept both hands parallel and two octaves apart.
- Removed duplicate apex moments from up/down exercises.
- Assigned one beat to melodic moments and two beats to chord moments.
- Rejected every generated note outside the configured keyboard range.
- Added explicit errors for invalid tonic, octave span, tempo and range.
- Documented why fingering is deferred until reviewed key/hand-specific tables
  exist.
- Documented the required in-memory MIDI and UI integration layers.

### Verification

- Exact C-major both-hand up/down sequence test.
- Exact A-minor descending arpeggio test.
- Exact c-minor i–iv–V–i voicing test.
- B-major three-octave 88-key boundary test.
- Restricted-keyboard rejection test.
- Invalid tonic, four-octave and 241 BPM rejection tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty-six core tests and forty-nine
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `d45b7c5`
(`feat: add deterministic exercise plans`).

### Known limitations

- Plans are not converted into a `MidiFile` yet.
- There is no exercise selection UI or persisted exercise identity yet.
- Fingering is intentionally absent pending reviewed per-key/per-hand data.
- Harmonic minor scales and contrary-motion exercises are future variants.

## 2026-07-25 — Cycle 037: Pedal timing and dynamics contour (DONE)

### Outcome

Technique feedback now describes whether sustain changes tend to land early or
late and whether performed dynamics follow the score's rising/falling shape.
Both signals use explicit evidence thresholds and avoid treating raw MIDI
values as an absolute musical verdict.

### Implemented

- Added timestamps to user and score CC64 evidence.
- Detected sustain transitions only when values cross the half-pedal threshold.
- Kept raw controller-change and continuous-value counts intact.
- Paired pedal-down events with pedal-down references and pedal-up events with
  pedal-up references.
- Calculated signed user-minus-score transition offsets.
- Summarized offsets with median and median absolute deviation.
- Persisted paired sample count, transition counts, median offset and spread.
- Required:
  - at least four paired transitions;
  - equal user/score transition counts;
  - every target transition to have a pair;
  - no more than 120 ms median deviation.
- Reported insufficient samples, mismatched transitions and unstable timing
  separately.
- Grouped matched note velocities by exact score onset.
- Averaged chord members before comparing consecutive dynamic points.
- Ignored target changes smaller than six velocity units.
- Classified played changes smaller than three units as flat.
- Counted aligned, flat and opposite directions.
- Required six shaped steps for learner-facing contour feedback.
- Added two new Technique lines without overlapping completion actions at the
  minimum panel height.
- Added backward-compatible defaults to every persisted field.
- Marked `MUS-005` complete.

### Verification

- A seven-onset crescendo/decrescendo fixture produces:
  - six shaped steps;
  - six aligned;
  - zero flat;
  - zero opposite.
- Chord members at the same onset are averaged rather than treated as separate
  contour steps.
- Four target/user sustain transitions offset by +40 ms produce:
  - four timing pairs;
  - median 40 ms late;
  - zero median deviation.
- UI copy tests cover stable contour/timing, incomplete references and
  transition mismatch.
- Legacy expression RON loads with zero contour/timing fields.
- Re-ran the deterministic native completion fixture; Overview, Technique,
  History, Retry and clean exit all pass.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty-one core tests and forty-nine
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `3f99cde`
(`feat: add pedal timing and dynamics contour`).

### Known limitations

- MIDI sustain references are performance data, not engraved pedal notation.
- The profile does not yet distinguish flutter, half-pedal depth or
  repedalling intent.
- Dynamic direction follows MIDI velocity shape and cannot infer phrasing where
  the source MIDI is mechanically flat.
- Evidence remains descriptive; it does not assign a pedal or dynamics grade.

## 2026-07-25 — Cycle 036: Deterministic native completion flow (DONE)

### Outcome

Native automation can now finish a complete practice take, inspect all three
completion sections and prove that Retry returns to a clean attempt. The
completion UI is covered in the real GPU application rather than only by
helper-unit tests.

### Implemented

- Added a `-CompletionFixture` parameter set to the native smoke runner.
- Embedded a compact, deterministic MIDI fixture as Base64 test data.
- Generated the fixture only inside the per-run temporary directory.
- Defined a Type-1 file with:
  - 480 PPQ;
  - 4/4 at 120 BPM;
  - a C5 right-hand track;
  - a C3 left-hand track;
  - a finite four-beat take.
- Repeatedly read required pitches and performed them through Debug MIDI input.
- Waited for a non-null completion tab rather than relying on elapsed time
  alone.
- Asserted completion starts on Overview.
- Activated and asserted Technique, History and Overview by semantic ID.
- Activated completion Retry.
- Asserted Retry removes the completion screen and resets matched notes to
  zero.
- Returned to menu and exited cleanly.
- Documented both real-song and deterministic-completion commands.

### Verification

- `.\scripts\debug-practice-smoke.ps1 -CompletionFixture`
  - `CompletionTab = overview`;
  - `MatchedNotes = 2`;
  - Technique, History and Overview passed;
  - `RetryReset = 0`;
  - `ExitCode = 0`.
- Re-ran the prepared “Look at the Sky” path with `-SkipBuild`:
  - wait `True → False`;
  - matched input `2`;
  - hands `Both → Right`;
  - loop `1–2`, restart preserved, disable passed;
  - exit code `0`.
- Cycle 035's immediately preceding full Rust test, Clippy, release build,
  formatting and diff checks remain authoritative because Cycle 036 changes
  only the tested PowerShell runner and its documentation.

Implementation commit: `1fc952c`
(`test: cover completion flow in native smoke`).

### Known limitations

- The completion fixture is intentionally too small to produce meaningful
  timing calibration, history trends or weak-passage recommendations.
- GPU pixels are not captured or compared.
- The fixture validates exact semantic state, not visual styling.

## 2026-07-25 — Cycle 035: Scored MIDI injection in native smoke (DONE)

### Outcome

The real-process smoke test now plays the notes Neothesia is actually waiting
for and proves that they reach the normal player input path and practice
matcher. It no longer verifies wait mode using state toggles alone.

### Implemented

- Added `PracticeMatcher::required_note_pitches()`.
- Preserved duplicate occurrences and returned pitches in stable sorted order.
- Added a matcher assertion for the required C-major chord pitches.
- Added a Debug-only required-pitch delegate on `MidiPlayer`.
- Added required-note count and pitch list to the practice snapshot.
- Added an acknowledged `DebugMidiInput` application event.
- Added a player-scene Debug MIDI handler that calls the normal scene
  `midi_event` path.
- Added safe in-process channel/note/velocity validation.
- Added `MIDI <channel> <note> <velocity>` to the loopback protocol.
- Restricted channels to 0–15 and notes/velocities to 0–127.
- Defined velocity zero as a release, matching live MIDI normalization.
- Added stable JSON array output for required pitches.
- Extended the native smoke runner to:
  - wait until the score blocks on required notes;
  - assert count/list agreement;
  - inject NoteOn and NoteOff for every required pitch;
  - assert that `matched_notes` increases.
- Updated the automation contract and next-step boundary.

### Verification

- Ran the checked-in smoke runner against “Look at the Sky - Porter Robinson,
  original key, auto-aligned.”
- The first blocked target exposed two required notes.
- Both note-on and note-off events were accepted.
- The matcher reported `MatchedAfterInput = 2`.
- Existing wait, hands, loop, restart, back and clean-exit assertions remained
  green.
- Protocol tests reject channel 16 and note 128.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-eight
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `6fd60aa`
(`test: inject scored MIDI in native smoke`).

### Known limitations

- The smoke runner performs only the first blocked target, not a whole song.
- It does not yet control inter-note timing or note-hold duration.
- A purpose-built short MIDI fixture is still needed to reach and automate the
  completion screen.
- GPU screenshot comparison remains future work.

## 2026-07-25 — Cycle 034: Native loop and restart coverage (DONE)

### Outcome

The checked-in native smoke run now proves the full baseline interaction path:
open a real two-hand MIDI, start it, inspect and toggle wait mode, enable a
measure-snapped loop, restart that practice scope, disable the loop, return to
the menu and exit cleanly.

### Implemented

- Extracted loop toggling from the rendered button into shared product logic.
- Kept loop disable behavior for count-in cancellation, attempt reset, Tempo
  Coach reset and playback resume unchanged.
- Assigned `practice.player.loop` to the visible repeat control.
- Added `practice.player.restart` for current-scope restart.
- Restarted the loop take when looping and the whole-song take otherwise.
- Added the `R` shortcut and a confirmation toast.
- Added loop active, start/end measures, count-in and pause state to the Debug
  snapshot.
- Added stable nullable numeric encoding to the local driver JSON.
- Extended the PowerShell smoke runner to assert:
  - loop activation;
  - a valid measure range;
  - range preservation across restart;
  - loop deactivation.
- Updated the shortcut page and native automation contract.
- Marked `QA-001` complete based on real-process evidence.

### Verification

- Ran the checked-in smoke script against “Look at the Sky - Porter Robinson,
  original key, auto-aligned.”
- Observed wait mode `True → False`.
- Observed hands `Both → Right`.
- Observed loop range `1–2`.
- Observed `LoopRestarted = True`.
- Observed `LoopDisabled = True`.
- Observed exit code `0`.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-eight
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `f49cd2c`
(`test: cover loop practice in native smoke`).

### Known limitations

- The smoke run validates state transitions but does not inject performed MIDI
  notes or wait for a completed loop attempt.
- Completion tabs and completion Retry still require a deterministic
  performance-input fixture.
- GPU screenshot comparison remains future work.

## 2026-07-25 — Cycle 033: Reusable native practice smoke runner (DONE)

### Outcome

The proven native automation sequence is now a repeatable repository command,
not a one-off terminal experiment. It launches the actual GPU application,
drives semantic controls, asserts practice state and exits cleanly.

### Implemented

- Added `scripts/debug-practice-smoke.ps1`.
- Built the Debug executable by default with an opt-out for repeated runs.
- Passed Unicode and space-containing MIDI paths as structured process
  arguments.
- Selected an available IPv4 loopback port at runtime.
- Ran the app in a uniquely named temporary working directory.
- Copied the local SoundFont into that isolated directory.
- Asserted that the initial scene is the menu.
- Started the loaded song and waited for a player snapshot.
- Asserted that wait-for-notes defaults on.
- Toggled wait mode and asserted the state changed.
- Cycled practice hands when the song exposes a hand scope and asserted the
  state changed.
- Returned to the menu and asserted that the player snapshot disappeared.
- Added a debug-only acknowledged `EXIT` command and asserted process exit code
  zero.
- Removed the temporary settings, history and SoundFont copy after every run.
- Kept exact-process termination as a guarded failure fallback.
- Documented the runnable command.

### Verification

- Ran the checked-in script twice against “Look at the Sky - Porter Robinson,
  original key, auto-aligned,” including once under PowerShell strict mode.
- Observed wait mode `True → False`.
- Observed hand mode `Both → Right`.
- Observed clean application exit code `0`.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-eight
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `5f16f5c`
(`test: add native practice smoke runner`).

### Known limitations

- The runner does not yet create a loop or reach completion/retry.
- It does not inject MIDI performance input.
- It does not capture or compare GPU-rendered screenshots.
- `QA-001` remains open until loop and real input/output behavior are covered.

## 2026-07-25 — Cycle 032: Loopback debug practice driver (DONE)

### Outcome

A separate local test process can now launch a Debug build, enter a loaded
song, operate stable practice controls and assert semantic state. The path no
longer depends on mouse coordinates or translated labels.

### Implemented

- Added an opt-in TCP debug driver controlled by
  `NEOTHESIA_DEBUG_DRIVER_ADDR`.
- Allowed only explicit IPv4 or IPv6 loopback socket addresses.
- Kept the listener disabled when the environment variable is absent.
- Limited commands to 4096 bytes and action/snapshot waits to two seconds.
- Added newline-delimited `ACTION <id>` and `SNAPSHOT` commands.
- Returned compact JSON results suitable for an external smoke-test process.
- Added `practice.menu.start` to the stable semantic catalogue and visible Play
  control.
- Let the menu accept start only when a song is actually loaded.
- Kept the entire listener module and its startup path out of release builds.
- Documented the protocol and its intentionally narrow security boundary.

### Verification

- Tested valid commands, malformed commands and stable JSON scalar output.
- Tested acceptance of `127.0.0.1` and `::1`.
- Tested rejection of wildcard and LAN addresses.
- Ran a real Debug application with “Look at the Sky - Porter Robinson,
  original key, auto-aligned”:
  - menu snapshot returned `null`, as designed;
  - `practice.menu.start` returned accepted;
  - player snapshot reported wait mode on and both hands;
  - `practice.player.wait` returned accepted;
  - the next snapshot reported wait mode off;
  - `practice.player.back` returned accepted.
- Removed the temporary practice-history file created by the smoke run.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-eight
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `eaf40f3`
(`feat: expose local debug practice driver`).

### Known limitations

- The smoke sequence is proven manually from a test process but is not yet a
  checked-in reusable launcher.
- The driver does not capture rendered frames.
- Calibration and recommendation actions remain click-only.
- The protocol is intentionally single-command-per-connection and local-only.

## 2026-07-25 — Cycle 031: Confirmed semantic action dispatch (DONE)

### Outcome

An automation caller can now distinguish “the action was accepted by the
active scene” from “the message was merely placed on the event queue.” This
removes a major source of false-positive native UI tests.

### Implemented

- Added a one-shot reply channel to every debug semantic action event.
- Returned the active scene's accepted/rejected decision to the caller.
- Added a caller-supplied timeout to prevent indefinite waits.
- Requested a redraw only after an accepted action.
- Documented that activation must run on a worker thread.
- Updated the automation contract and removed action acknowledgement from the
  remaining work list.

### Verification

- Existing tests cover supported and unsupported semantic action mappings.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-five
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `19a7d13`
(`feat: acknowledge debug practice actions`).

### Known limitations

- The acknowledgement is available only to in-process debug callers.
- No external local driver endpoint exists yet.
- Deterministic screenshot capture and parameterized completion actions remain
  future work.

## 2026-07-25 — Cycle 030: Debug practice automation harness (DONE)

### Outcome

Debug builds can now drive important practice controls by stable semantic ID
and inspect learner-facing state without screen coordinates. Actions travel
through the real application event loop and reuse the same behavior as visible
buttons.

### Implemented

- Added a debug-only event-loop proxy harness.
- Added semantic action and practice snapshot application events.
- Routed the active scene's debug requests through the normal event loop.
- Exposed wait mode, Tempo Coach, hand scope, completion tab, matched/wrong/
  missed counts and input-latency compensation in a read-only snapshot.
- Activated player back, wait, coach and hands by semantic ID.
- Activated completion Overview, Technique, History, retry and back by semantic
  ID when the completion screen is active.
- Extracted shared wait, coach and retry methods so automation cannot drift
  from visible button behavior.
- Added an explicit supported-action parser and mapping test.
- Kept calibration and recommendation actions unsupported until their
  parameterized product logic can be shared safely.
- Compile-time excluded the harness, events and snapshot from release builds.
- Updated the native UI automation contract.

### Verification

- Verified the supported semantic mapping and rejected unsupported IDs.
- Verified the complete stable-ID catalogue remains unique and namespaced.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-five
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `d428acf`
(`feat: add debug practice automation harness`).

### Known limitations

- The harness is currently in-process; no controlled external driver endpoint
  exists yet.
- Action dispatch reports queueing success, not whether the active scene
  accepted the action.
- Deterministic GPU screenshot capture is not implemented.
- Calibration and recommendation actions remain click-only.
- OS accessibility still depends on future Nuon platform integration.

## 2026-07-25 — Cycle 029: Stable practice action identities (DONE)

### Outcome

Core practice controls now have stable semantic identities independent of
English labels and screen coordinates. This establishes the first dependable
boundary for native UI automation.

### Implemented

- Defined a namespaced practice action catalogue.
- Assigned IDs to player back, wait, adaptive coach and hand-mode controls.
- Assigned IDs to all three completion tabs.
- Assigned IDs to calibration, note-loop, rhythm-loop, retry and back actions.
- Kept the same hand-mode ID across responsive toolbar placements.
- Added a uniqueness and namespace test for the complete catalogue.
- Added `docs/development/ui-automation.md` as the automation contract.
- Documented that Nuon IDs are currently in-process identities, not a
  fabricated Windows accessibility implementation.
- Defined the next debug-only activation/state/screenshot boundary.

### Verification

- Verified every catalogue ID is unique and begins with `practice.`.
- Existing button behaviour, completion navigation and responsive layout tests
  pass with explicit IDs.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-four
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `a549e07`
(`feat: add stable practice action identities`).

### Known limitations

- IDs are not externally discoverable yet.
- There is no debug command channel to activate an action by ID.
- Semantic state assertions and deterministic GPU screenshots remain future
  layers.
- OS accessibility still depends on future Nuon platform integration.

## 2026-07-25 — Cycle 028: Completion feedback information architecture (DONE)

### Outcome

The completion screen is no longer one dense wall of metrics. Overview,
Technique and History now have separate, predictable responsibilities while
the primary retry/back controls remain available everywhere.

### Implemented

- Replaced the two-tab header with Overview, Technique and History.
- Kept accuracy, counts, hand accuracy, weak measures and focused-loop actions
  in Overview.
- Moved timing profile, calibration confirmation, hand timing, chord
  synchronization, dynamics, pedal and key-hold evidence into Technique.
- Kept saved-session trends and weak-history detail in History.
- Hid note/rhythm recommendation actions outside Overview.
- Kept Practice again and Back to songs available on every tab.
- Added an explicit disabled-state explanation when Expression Summary is off.
- Added compact header copy for panels below 600 px.
- Calculated a fixed three-tab layout that fits the 480 px minimum panel.
- Added left/right arrow-key cycling with wraparound.

### Verification

- Added exact minimum-width tab-boundary coverage.
- Added forward/backward keyboard-cycle coverage.
- Existing feedback copy, recommendation, calibration and history tests pass.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-three
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `a8bcf89`
(`feat: organize completion feedback tabs`).

### Known limitations

- Layout geometry is tested, but a captured GPU-rendered completion fixture is
  not automated yet.
- Tab selection is not persisted, intentionally returning each take to
  Overview.
- The custom GPU UI still lacks stable semantic action IDs for external UI
  automation.

## 2026-07-25 — Cycle 027: Block-chord synchronization evidence (DONE)

### Outcome

Completed takes now describe how closely the attacks of written block chords
landed together. Written arpeggios and incomplete chords are protected from
misleading synchronization statistics.

### Implemented

- Grouped score targets only when their exact MIDI onset and measure match.
- Required at least two target notes for a chord candidate.
- Excluded unknown-context matcher events from chord grouping.
- Counted complete and incomplete target chords separately.
- Calculated attack span from the earliest to latest matched note.
- Included only fully matched chords in median and maximum span statistics.
- Required four complete chords before presenting an aggregate profile.
- Added a compact completion line with explicit “descriptive” wording.
- Persisted chord evidence with a backward-compatible default.
- Kept different-onset written arpeggios outside the chord evidence entirely.

### Verification

- Added a deterministic fixture with four complete block chords at 20, 30, 40
  and 50 ms attack spans.
- Verified the even-sample median is 35 ms and maximum is 50 ms.
- Verified one missing-note chord is counted incomplete and excluded from span.
- Verified three notes written 40 ms apart are not treated as a chord.
- Added learner-facing no-evidence, insufficient-evidence and profile copy
  tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-one
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `01289b0`
(`feat: summarize block chord synchronization`).

### Known limitations

- A MIDI author can encode an intended roll with identical onsets; Neothesia
  therefore describes span and never labels it correct or wrong.
- The profile is whole-attempt rather than separated by hand.
- Chord-size and register-specific expectations are not modeled.
- The completion screen is now information-dense and needs a dedicated
  technique tab.

## 2026-07-25 — Cycle 026: Reliable rhythm trouble spots (DONE)

### Outcome

Neothesia now distinguishes note-accuracy problems from rhythm-stability
problems at measure level. A learner can start a focused loop for either reason
from two separate, evidence-labelled actions.

### Implemented

- Added backward-compatible robust timing profiles to measure summaries.
- Aggregated raw matched-note offsets independently for every measure.
- Combined repeated attempts using medians of per-attempt bias and deviation.
- Kept only attempts with at least four timing samples in that measure.
- Required at least two attempts and twelve cumulative matched notes.
- Flagged a rhythm problem only at ≥60 ms median bias or ≥40 ms median
  deviation.
- Ranked reliable measures by transparent bias-plus-deviation severity.
- Scoped evidence to the latest session kind and hand goal.
- Added a dedicated purple Rhythm action beside the orange Notes action.
- Split the recommendation row cleanly when both actions are available.
- Started the selected two-measure rhythm loop with an explicit focus toast.

### Verification

- Added repeated-evidence, insufficient-evidence and stable-measure tests.
- Added scope isolation between whole-song and loop sessions.
- Verified exact measure timing aggregation and old-summary migration.
- Preserved the existing note-accuracy recommendation tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, forty-nine core tests and forty
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `ffdb072`
(`feat: recommend weak rhythm measures`).

### Known limitations

- Cross-attempt aggregation uses per-attempt medians rather than retaining every
  raw historical note offset.
- Thresholds are fixed policy constants and are not level-specific yet.
- Recommended passages remain two measures long.
- Chord simultaneity and intentional rolled chords are not distinguished yet.

## 2026-07-25 — Cycle 025: Left/right-hand timing profiles (DONE)

### Outcome

The completion screen can now show whether the two hands have different timing
biases or consistency. Each hand must earn its own evidence; notes from the
other hand never fill its sample requirement.

### Implemented

- Retained raw signed timing offset on every matched structured result.
- Kept wrong and missed results explicitly free of timing offsets.
- Aggregated timing offsets independently by practice part.
- Added a backward-compatible timing profile to persisted part summaries.
- Required eight matched notes per hand.
- Displayed median early/late bias and typical spread for each qualified hand.
- Displayed the exact number of additional notes needed for an under-sampled
  hand.
- Preserved the existing right/left accuracy line above the timing comparison.
- Kept ambiguous/custom parts out of left/right claims.

### Verification

- Added a deterministic 16-note two-hand fixture with distinct 20 ms and 60 ms
  hand biases.
- Proved both profiles retain zero within-hand deviation independently.
- Added migration coverage for old part summaries without timing data.
- Added UI-copy coverage where one hand qualifies and the other does not.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, forty-six core tests and forty
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `c153721`
(`feat: compare left and right hand timing`).

### Known limitations

- The view describes each hand; it does not yet generate a hand-specific
  practice assignment.
- MIDI files without reliable left/right track inference remain unlabelled.
- Timing is aggregated over the attempt, so measure-level rhythm trouble spots
  are not yet visible.
- Cross-hand chord synchronization is not measured separately.

## 2026-07-25 — Cycle 024: Conservative calibration suggestions (DONE)

### Outcome

Neothesia can now turn a stable timing profile into an explicit input-offset
suggestion. It never changes calibration silently: the pianist must confirm the
displayed value, after which the same piece restarts for verification.

### Implemented

- Added a pure, explainable calibration policy.
- Required at least 24 matched notes.
- Rejected takes whose median absolute deviation exceeds 35 ms.
- Treated a median bias below 10 ms as already centered.
- Capped each confirmed correction to 50 ms.
- Respected the global ±250 ms safety bounds.
- Explained insufficient, unstable, centered and limit states in the timing
  profile line.
- Added an inline `Apply … & retry` action only when all gates pass.
- Persisted the confirmed offset, updated the current player and immediately
  restarted a clean attempt.
- Kept raw MIDI/audio forwarding completely outside the calibration path.

### Verification

- Added threshold tests at 23/24 samples and 35/36 ms deviation.
- Added deadband, positive/negative correction, step-cap and global-limit tests.
- Updated learner-facing wording coverage.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, forty-four core tests and thirty-nine
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `27ae4f2`
(`feat: suggest conservative timing calibration`).

### Known limitations

- Suggestions use a normal musical take rather than a dedicated metronome
  calibration exercise.
- Stable intentional rubato can still resemble route latency; explicit
  confirmation and immediate verification are therefore mandatory.
- Calibration remains global rather than per MIDI device/route.
- The current evidence is whole-attempt rather than hand-specific.

## 2026-07-25 — Cycle 023: Robust timing profile (DONE)

### Outcome

The completion screen now distinguishes a consistent early/late bias from
general timing inconsistency. This is more useful than counts alone and
provides a defensible evidence layer for guided latency calibration.

### Implemented

- Preserved the signed millisecond offset of every matched note before the
  on-time grade normalized it.
- Defined negative values as early and positive values as late.
- Calculated median signed bias.
- Calculated median absolute deviation from that bias as a robust consistency
  measure.
- Required eight matched notes before presenting a profile.
- Kept the existing learner-facing early/on-time/late counts.
- Added a compact completion line using plain “median” and “typical spread”
  wording.
- Persisted timing profiles with backward-compatible defaults.
- Cleared timing samples between attempts.

### Verification

- Added exact signed-offset, median and deviation tests.
- Verified on-time notes retain their original small early/late offset.
- Added empty-evidence and learner-facing wording coverage.
- Existing legacy-summary coverage now verifies the timing default.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, forty-two core tests and thirty-nine
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `f69d51d`
(`feat: add robust practice timing profile`).

### Known limitations

- The completion line is descriptive and does not yet propose a calibration.
- One take can contain intentional rubato, so timing bias must not be treated
  automatically as device latency.
- Profiles are whole-attempt aggregates; hand and measure timing profiles are
  not persisted separately yet.
- “Typical spread” is median absolute deviation, not a percentile guarantee.

## 2026-07-25 — Cycle 022: Input-latency compensation (DONE)

### Outcome

Pianists can now correct a consistent keyboard/driver timing delay without
delaying sound or modifying MIDI sent to Pianoteq. Timing judgement uses the
compensated timestamp; monitoring remains immediate.

### Implemented

- Added a persistent input timing offset with backward-compatible zero default.
- Added a Settings control in 5 ms steps.
- Bounded the adjustment to `-250..=250 ms`.
- Applied positive offsets by moving judgement timestamps earlier.
- Supported negative offsets for unusual routes that require later judgement.
- Used saturating duration arithmetic at session start.
- Applied the same constant offset to note-on and note-off so physical key-hold
  duration is unchanged.
- Kept raw live MIDI forwarding ahead of and independent from assessment.
- Displayed every non-zero active offset in the player status line.

### Verification

- Added default, legacy-settings and clamp tests.
- Added a real player-path test where a 200 ms arrival with `+120 ms`
  compensation lands exactly on the 80 ms on-time boundary.
- Covered positive underflow and negative adjustment explicitly.
- Existing exact expressive-MIDI forwarding tests continue to pass.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, forty-one core tests and thirty-eight
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `94fb28c`
(`feat: compensate practice input latency`).

### Known limitations

- The offset is manual; there is no guided tap-to-calibrate workflow yet.
- One global value is used for every MIDI input/output route.
- Audio output latency is intentionally not delayed or estimated.
- A timing profile is still needed before the app can propose an offset from
  repeated evidence.

## 2026-07-25 — Cycle 021: Key-hold duration evidence (DONE)

### Outcome

Completed takes now show how long matched keys were physically held relative
to the reference MIDI. This makes staccato/legato investigation possible
without pretending that raw MIDI duration alone determines good articulation.

### Implemented

- Carried each parsed reference note duration into its practice target.
- Indexed duration lookup by track, pitch and score onset when opening a song.
- Paired live note-on and note-off events with matched score occurrences.
- Preserved duration evidence when a pianist played and released slightly
  before the score onset.
- Kept repeated-pitch occurrences in FIFO order.
- Calculated a robust median key-hold ratio rather than an outlier-sensitive
  mean.
- Exposed transparent `<75%`, `75–125%` and `>125%` counts.
- Required four completed notes before showing the aggregate comparison.
- Explicitly excluded sustain-pedal extension from physical key-hold duration.
- Cleared unfinished hold state on transport/panic resets.
- Added backward compatibility for Cycle 020 expression records.

### Verification

- Extended deterministic matcher coverage with four known duration ratios.
- Covered early-release pairing and median/band calculations.
- Added nested expression-history compatibility coverage.
- Extended the player integration test to prove parsed score duration, live
  release timing and unmodified output messages travel through the real path.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, thirty-nine core tests and thirty-seven
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `91fd14f`
(`feat: add key-hold duration evidence`).

### Known limitations

- The ratio describes physical key hold, not acoustic note length under sustain.
- Reference MIDI note lengths may be quantized or edited and are not treated as
  authoritative phrasing.
- The three bands expose distribution; they are not a pass/fail grade.
- Timing evidence still assumes that input-device latency is negligible.

## 2026-07-25 — Cycle 020: Descriptive expression evidence (DONE)

### Outcome

The completion screen now reports how the pianist's matched-note velocity range
compared with the MIDI reference and whether sustain-pedal data was present.
The language stays deliberately descriptive: MIDI velocity and pedal event
counts are evidence, not proof of good tone or correct pedalling.

### Implemented

- Captured played velocity alongside every matched live note.
- Preserved score velocity in structured practice targets.
- Aggregated matched samples, mean absolute velocity gap and played/reference
  ranges.
- Captured CC64 use, value transitions and continuous half-pedal samples for
  both the score and live input.
- Added an optional, default-on Expression Summary setting.
- Added compact dynamics and pedal lines to the completion screen.
- Required four matched velocity samples before presenting a range comparison.
- Persisted expression evidence in practice sessions with legacy-history
  defaults.
- Kept the external MIDI/Pianoteq route byte-for-byte unchanged.

### Verification

- Added deterministic velocity pairing, pedal evidence and reset tests.
- Added legacy practice-history deserialization coverage.
- Added player integration coverage for score/live evidence and exact external
  MIDI forwarding.
- Added learner-facing wording coverage for incomplete references.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, thirty-eight core tests and thirty-seven
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `1a04bc2`
(`feat: summarize dynamics and pedal evidence`).

### Known limitations

- Velocity comparison is not calibrated for the player's keyboard curve,
  Pianoteq preset or listening level.
- Pedal feedback counts evidence and transitions; it does not yet align pedal
  timing to notes, harmonies or score intervals.
- MIDI reference velocity may be mechanical or normalized and is not treated
  as an artistic ground truth.
- Expression evidence does not yet influence spaced-review scheduling.

## 2026-07-25 — Cycle 019: Explainable spaced review (DONE)

### Outcome

Neothesia can now distinguish “needs work now” from “successfully learned;
check retention later.” The schedule is local, conservative and visible: every
due label follows directly from measured note/timing performance rather than an
opaque engagement score.

### Implemented

- Added pure review-status and reason models.
- Reused the established 90% accuracy and 70% on-time mastery thresholds.
- Counted consecutive mastered attempts only inside the latest practice scope.
- Added 1/3/7/14-day review intervals.
- Made weak and evidence-free attempts immediately due.
- Added ceiling-rounded days-remaining calculations.
- Added All, Queue and Due library modes.
- Ordered due work by due timestamp.
- Added compact explanatory labels for reinforcement, retention and waiting.
- Kept queue insertion available directly beside every due item.

### Verification

- Added mastery-streak, interval, due-boundary, immediate-reinforcement and
  UI-explanation tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `c5ee410`
(`feat: add explainable spaced review`).

### Known limitations

- Review intervals are fixed policy bands, not user-configurable yet.
- The schedule uses note/timing mastery; pedal, dynamics and duration evidence
  are not included yet.
- A take with no judged notes is conservatively due but still needs a proper
  calibration/session workflow.
- Queue insertion remains explicit rather than automatic by design.

## 2026-07-25 — Cycle 018: Favourites and ordered practice queue (DONE)

### Outcome

The local library now supports an intentional daily practice list instead of
being only a searchable catalogue. Learners can pin repertoire, build a short
ordered queue and see the latest measured result or reliable weak passage
before opening a piece.

### Implemented

- Added backward-compatible per-song library organization state.
- Added persistent favourite mutation and favourite-first library ordering.
- Added queue insertion, removal, normalization and boundary-safe movement.
- Added a focused Queue/All Pieces view toggle.
- Added row-level favourite and queue actions.
- Added explicit up/down controls in queue view.
- Created organization records for indexed songs before their first practice.
- Preserved known source paths for organized but unopened songs.
- Included reliable weak-measure recommendations in library rows.
- Kept all organization anchored to content identity.

### Verification

- Added persistence, queue-order, normalization, boundary and migration tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `936b95c`
(`feat: add favourites and practice queue`).

### Known limitations

- Queue ordering is manual; spaced-review due work is not inserted
  automatically.
- There is no bulk clear or drag-and-drop ordering yet.
- Favourites currently affect ordering but do not have a separate-only filter.
- Musical metadata still comes from filenames and paths.

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
