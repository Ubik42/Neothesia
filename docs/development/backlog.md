# Development backlog

Items are ordered within each horizon. IDs remain stable after completion.

## NOW — M0 trustworthy practice baseline

- [x] `PRA-001` Start playback after the MIDI picker returns.
- [x] `PRA-002` Make wait-for-notes the default with an obvious in-player toggle.
- [x] `VIS-001` Show one-based measure numbers.
- [x] `VIS-002` Add optional quarter-note subdivision lines.
- [ ] `QA-001` Run open/play/wait/loop/exit smoke test on a real two-hand MIDI.
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
- [ ] `PRA-018` Add threshold-based adaptive tempo with explicit opt-in.
- [x] `PRA-019` Compare the current, last and best loop attempts.
- [ ] `DATA-010` Persist versioned practice sessions atomically.

## NEXT — reliability and sound

- [ ] `MIDI-010` Audit stop/seek/loop/output-change panic behaviour.
- [ ] `MIDI-011` Preserve and test sustain, continuous pedal and pitch bend.
- [ ] `AUD-010` Document and validate external Pianoteq routing.
- [ ] `AUD-011` Add a MIDI/Pianoteq acceptance test checklist.
- [ ] `QA-010` Add deterministic player tests that do not use wall-clock sleeps.

## LATER — repertoire and learning

- [ ] `LIB-001` Scan watched folders and build a searchable local library.
- [ ] `LIB-002` Add stable content-based song identity and metadata sidecars.
- [ ] `LIB-003` Save per-song hand/track, loop and speed settings.
- [ ] `LIB-004` Add recent, favourite and practice-queue views.
- [ ] `MUS-001` Add a MusicXML/grand-staff feasibility prototype.
- [ ] `MUS-002` Add manual finger hints in portable sidecars.
- [ ] `MUS-003` Prototype explainable fingering suggestions.
- [ ] `MUS-004` Add pedal and dynamics feedback.
- [ ] `COACH-001` Detect weak measures from multiple attempts.
- [ ] `COACH-002` Schedule local spaced review.
- [ ] `EX-001` Add scales, arpeggios and chord exercise mode.

## RESEARCH — architecture spikes

- [ ] `AUD-R01` Compare maintained Rust VST3 host libraries with Pianoteq.
- [ ] `AUD-R02` Prototype the block audio boundary behind a feature flag.
- [ ] `UI-R01` Define semantic UI action IDs for reliable automation.
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
