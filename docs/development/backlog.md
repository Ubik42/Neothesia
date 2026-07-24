# Development backlog

Items are ordered within each horizon. IDs remain stable after completion.

## NOW — M0 trustworthy practice baseline

- [x] `PRA-001` Start playback after the MIDI picker returns.
- [x] `PRA-002` Make wait-for-notes the default with an obvious in-player toggle.
- [x] `VIS-001` Show one-based measure numbers.
- [x] `VIS-002` Add optional quarter-note subdivision lines.
- [x] `QA-001` Run open/play/wait/loop/exit smoke test on a real two-hand MIDI.
- [x] `DOC-001` Establish roadmap, backlog, progress and cycle log.

## NEXT — M1 practice intelligence

- [x] `PRA-010` Extract a deterministic practice matcher from `MidiPlayer`.
- [x] `PRA-011` Define timing windows and correct/wrong/missed/early/late results.
- [x] `PRA-012` Match chords, repeated pitches and overlapping same-pitch notes.
- [x] `PRA-013` Expose a stable live practice snapshot to the UI.
- [x] `PRA-014` Show a compact live accuracy/timing panel.
- [x] `PRA-015` Show an end-of-attempt summary.
- [x] `PRA-016` Aggregate results by measure and hand.
- [x] `PRA-017` Add count-in and attempt reset for loop practice.
- [x] `PRA-018` Add threshold-based adaptive tempo with explicit opt-in.
- [x] `PRA-019` Compare the current, last and best loop attempts.
- [x] `DATA-010` Persist versioned practice sessions atomically.
- [x] `DATA-011` Identify songs by MIDI content across moves and renames.

## NEXT — reliability and sound

- [x] `MIDI-010` Audit stop/seek/loop/output-change panic behaviour.
- [x] `MIDI-011` Preserve and test sustain, continuous pedal and pitch bend.
- [x] `MIDI-012` Add a visible panic action and stable keyboard shortcut.
- [x] `AUD-013` Show the active output and backend type persistently in the
  player.
- [x] `PRA-020` Add in-player both/right/left-hand practice controls.
- [ ] `AUD-010` Document and validate external Pianoteq routing.
- [x] `AUD-011` Add a MIDI/Pianoteq acceptance test checklist.
- [ ] `AUD-012` Complete and record the 30-minute physical Pianoteq soak test.
- [ ] `QA-010` Add deterministic player tests that do not use wall-clock sleeps.

## LATER — repertoire and learning

- [x] `LIB-001` Scan watched folders and build a searchable local library.
- [ ] `LIB-002` Add stable content-based song identity and metadata sidecars.
- [x] `LIB-003` Save per-song hand/track, loop and speed settings.
- [x] `LIB-004` Add recent, favourite and practice-queue views.
- [x] `LIB-005` Add a recent-practice library with verified missing-file repair.
- [ ] `MUS-001` Add a MusicXML/grand-staff feasibility prototype.
- [ ] `MUS-002` Add manual finger hints in portable sidecars.
- [ ] `MUS-003` Prototype explainable fingering suggestions.
- [x] `MUS-004` Capture and display descriptive pedal/dynamics evidence.
- [x] `MUS-005` Add calibrated pedal timing and dynamics-contour feedback.
- [x] `MUS-006` Add note-duration and articulation evidence.
- [x] `PRA-021` Add persistent manual input-latency compensation.
- [x] `PRA-022` Add evidence-based guided input-latency calibration.
- [x] `COACH-006` Compare robust left/right-hand timing profiles.
- [x] `COACH-007` Rank reliable measure-level rhythm trouble spots.
- [x] `MUS-007` Measure block-chord attack synchronization safely.
- [x] `UI-010` Split completion feedback into Overview/Technique/History tabs.
- [x] `COACH-005` Report signed timing bias and robust consistency.
- [x] `COACH-001` Recommend weak passages from multiple attempts and explain
  the evidence.
- [x] `COACH-003` Start a structured loop directly from a weak-passage
  recommendation.
- [x] `COACH-004` Show current-song attempt trends and persistent weak-measure
  ranking.
- [x] `COACH-002` Schedule local spaced review.
- [x] `EX-001A` Define deterministic scale, arpeggio and chord exercise plans.
- [x] `EX-001B` Convert exercise plans to stable in-memory Type-1 MIDI.
- [x] `EX-001C` Persist the last exercise and add bidirectional selectors.
- [x] `EX-001D` Unify tempo/hand progression and record effective BPM.
- [x] `EX-001E` Reopen generated exercises from Practice Library.
- [x] `EX-001F` Add one-, two-, four- and eight-pass exercise sessions.
- [x] `EX-001G` Compare accuracy and timing stability across exercise passes.
- [x] `EX-001H` Add natural, harmonic and directional melodic minor scales.
- [x] `EX-001` Add scales, arpeggios and chord exercise mode.
- [x] `EX-002` Persist favourite exercise presets and recent exercise variants.
- [ ] `EX-003` Add reviewed key- and hand-specific fingering guidance.

## RESEARCH — architecture spikes

- [ ] `AUD-R01` Compare maintained Rust VST3 host libraries with Pianoteq.
- [ ] `AUD-R02` Prototype the block audio boundary behind a feature flag.
- [x] `UI-R01` Define semantic UI action IDs for reliable automation.
- [x] `UI-R03` Add a debug-only semantic action channel and practice snapshot.
- [x] `UI-R04` Return accepted/rejected results for debug semantic actions.
- [x] `UI-R05` Expose the debug harness through a loopback-only local driver.
- [x] `UI-R06` Check in an isolated real-process practice smoke runner.
- [x] `UI-R07` Cover loop activation and scope restart in the native smoke run.
- [x] `UI-R08` Inject scored MIDI input through the native Debug driver.
- [x] `UI-R09` Reach completion tabs and Retry with a deterministic MIDI fixture.
- [ ] `UI-R02` Evaluate a React library/analytics panel only after its API exists.
- [ ] `MUS-R01` Compare direct MusicXML rendering with an embedded notation engine.

## Definition of done

Each checked item must have:

- acceptance criteria demonstrated;
- tests proportional to risk;
- format and regression checks passing;
- release build passing when application code changes;
- user-facing documentation when behaviour changes;
- a development-log entry and commit hash.
