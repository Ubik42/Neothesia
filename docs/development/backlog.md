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

## NOW — performance, practice, MIDI and sound reliability

- [x] `MIDI-010` Audit stop/seek/loop/output-change panic behaviour.
- [x] `MIDI-011` Preserve and test sustain, continuous pedal and pitch bend.
- [x] `MIDI-012` Add a visible panic action and stable keyboard shortcut.
- [x] `AUD-013` Show the active output and backend type persistently in the
  player.
- [x] `PRA-020` Add in-player both/right/left-hand practice controls.
- [ ] `AUD-010` Document and validate external Pianoteq routing.
- [x] `AUD-010A` Add a same-backend route diagnostic for visible, saved and
  openable MIDI outputs.
- [ ] `AUD-010B` Create/select the local virtual cable and record a successful
  route diagnostic.
- [x] `AUD-011` Add a MIDI/Pianoteq acceptance test checklist.
- [ ] `AUD-012` Complete and record the 30-minute physical Pianoteq soak test.
- [x] `QA-010` Add deterministic player tests that do not use wall-clock sleeps.
- [ ] `MIDI-013` Detect selected MIDI input/output disappearance, silence the
  old route safely and offer an explicit reconnect when it returns.
- [ ] `MIDI-014` Persist device-scoped input-latency calibration instead of
  applying one global offset to every keyboard and route.
- [ ] `PRA-023` Run a representative real-keyboard acceptance suite across
  wait/flow, hands, loops, dynamics, continuous pedal and rapid repeated notes.
- [ ] `QA-011` Record a 60-minute transport/device-churn soak with repeated
  pause, seek, loop, restart and route restoration.
- [ ] `LIB-008` Add numeric indexing progress and a content-safe cache so the
  2,000+ piece library does not require a full rescan on every cold start.
- [ ] `REL-001` Produce a repeatable Windows package and one-click launch path
  that does not require a Rust development environment.

## LATER — repertoire and learning

Further MusicXML engraving and score-reading work is paused until the open
performance/MIDI reliability items above have physical-device evidence.

- [x] `LIB-001` Scan watched folders and build a searchable local library.
- [x] `LIB-002` Add stable content-based song identity and metadata sidecars.
- [x] `LIB-002A` Add versioned content-bound sidecar storage with atomic saves.
- [x] `LIB-002B` Load, merge, display and search portable song metadata.
- [x] `LIB-002C` Add a native metadata editor to Practice Library.
- [x] `LIB-003` Save per-song hand/track, loop and speed settings.
- [x] `LIB-004` Add recent, favourite and practice-queue views.
- [x] `LIB-005` Add a recent-practice library with verified missing-file repair.
- [x] `LIB-006` Add a reproducible, checksum-verified public practice-corpus
  sync for classical performance, classical teaching and pop-style excerpts.
- [x] `LIB-007` Surface corpus source, license and teaching category from the
  generated catalogue inside Practice Library.
- [x] `MUS-001` Add a MusicXML/grand-staff feasibility prototype.
- [x] `MUS-001A` Add a notation-neutral score model and a tested uncompressed
  partwise MusicXML importer.
- [x] `MUS-001B1` Add bounded compressed MXL input with container validation.
- [ ] `MUS-001B2` Add score-timewise conversion.
- [ ] `MUS-001B3` Add a representative cross-exporter and cross-version piano
  fixture corpus.
- [x] `MUS-001B3A` Add a pinned, hash-verified MuseScore 1.2–3.6 / OSMD
  compatibility audit and compact score inspector.
- [ ] `MUS-001B3B` Add licensed Dorico, Finale and current MuseScore exports
  with stable semantic assertions.
- [x] `MUS-001C` Align imported score events with the performance timeline and
  expose stable note/measure identities.
- [x] `MUS-001C1` Assign deterministic part/measure/kind/ordinal identities to
  score notes and directions.
- [x] `MUS-001C2` Project exact score time through the paired MIDI tempo map.
- [x] `MUS-001C3` Align pitched score notes to MIDI track/note identities with
  confidence and explicit unmatched results.
- [x] `MUS-001D` Render a synchronized grand-staff proof of concept without
  coupling practice logic to the renderer.
- [x] `MUS-001D1` Define a renderer-neutral page/element index and derive
  occurrence-aware highlight frames from the aligned MIDI timeline.
- [x] `MUS-001D2` Build the feature-flagged Verovio adapter and native display
  boundary with current-page virtualization.
- [x] `MUS-001D2A` Measure Verovio semantic note evidence and add fail-closed
  native-to-renderer correlation with explicit unison ambiguity.
- [x] `MUS-001D2B` Export a versioned page/element manifest from the pinned
  Verovio worker and validate it through the native render index.
- [x] `MUS-001D2C` Bind every SVG page to its own content fingerprint and
  verify page bytes before native loading.
- [x] `MUS-001D2D` Add a bounded current/previous/next page cache with explicit
  eviction and cancellation of stale worker results.
- [x] `MUS-001D2E` Add the feature-flagged worker process lifecycle, verified
  artifact cache directory and request-generation handoff.
- [x] `MUS-001D2E1` Add the default-off Node/Verovio worker, staged atomic
  publication and content-verified cache reuse.
- [x] `MUS-001D2E2` Hand document/page generations through the application
  event loop and discard obsolete worker responses before scene mutation.
- [x] `MUS-001D2F` Instantiate the verified three-page cache in PlayingScene and
  load page bytes asynchronously from the accepted artifact.
- [x] `MUS-001D2G` Rasterize verified SVG pages behind the feature flag and
  upload only the focused page texture to the native GPU renderer.
- [x] `MUS-001D3` Integrate score visibility, page following and highlight
  paint into the player with semantic debug coverage.
- [x] `MUS-001D3A` Add an explicit persisted player score toggle that releases
  only the focused GPU texture while retaining the verified CPU page cache.
- [x] `MUS-001D3B` Drive bounded page focus and neighbour prefetch from the
  occurrence-aware playback focus timeline.
- [x] `MUS-001D3C` Paint active score-note highlights from validated renderer
  element identities without moving practice authority into the renderer.
- [x] `MUS-001D4` Add persisted bounded score sizing with in-context controls,
  minimum-window keyboard reserve and semantic process coverage.
- [x] `MUS-001E` Preserve tuplet ratios, normal note types and display spans
  while retaining exact rational event timing.
- [x] `MUS-001F` Preserve damper/sostenuto pedal directions and engraving
  preferences for future score-to-performance feedback.
- [x] `MUS-001G` Support common non-linear score playback order for alignment.
- [x] `MUS-001G1` Preserve barline location, forward/backward repeats, repeat
  count and numbered-ending semantics.
- [x] `MUS-001G2` Expand common repeats and first/second endings into a bounded,
  diagnostic playback plan.
- [x] `MUS-001G3` Project repeated score-event occurrences onto flattened MIDI
  time without changing their stable source identities.
- [x] `MUS-001G4` Align repeated pitched occurrences to MIDI note identities
  and verify compatible navigation across piano parts.
- [x] `MUS-001H` Summarize alignment as structured Ready/Review/Poor/Blocked
  evidence for a future import UI.
- [x] `MUS-001I` Add a native score-pairing and compatibility diagnostics
  workflow before synchronized notation is enabled.
- [x] `MUS-001I1` Persist a validated, content-fingerprinted MusicXML/MXL
  association in the existing portable song sidecar.
- [x] `MUS-001I2` Add native pair/replace/remove controls and render association
  health in Practice Library.
- [x] `MUS-001I3` Run alignment off the UI thread and display the structured
  compatibility verdict.
- [x] `MUS-002` Add manual finger hints in portable sidecars.
- [x] `MUS-002A` Add exact-note, content-bound finger hints to song sidecars.
- [x] `MUS-002B` Load manual hints into the independently switchable waterfall
  guidance layer.
- [x] `MUS-002C` Add a native paused, sequential finger-annotation workflow.
- [x] `MUS-003` Prototype explainable fingering suggestions.
- [x] `MUS-003A` Add a deterministic hand-aware dynamic-programming cost model.
- [x] `MUS-003B` Explain every modeled suggestion and expose confidence tiers.
- [x] `MUS-003C` Add explicit preview/accept behavior to manual finger editing.
- [x] `MUS-003E` Render selected notes and unaccepted suggestions distinctly.
- [x] `MUS-003D` Complete personalized hand-span and polyphonic chord-state
  suggestions.
- [x] `MUS-003D1` Add persistent Compact, Standard and Large hand-span profiles
  that alter melodic fingering costs.
- [x] `MUS-003D2` Add safe polyphonic chord-state suggestions.
- [x] `MUS-003D3` Allow independent right/left-hand profiles while migrating
  the legacy shared profile.
- [x] `MUS-003G` Preview and atomically accept a complete suggested chord shape.
- [ ] `MUS-003F` Model chord-to-chord voice leading, held-note substitutions
  and phrase context.
- [x] `MUS-003F1` Optimize consecutive chord shapes together and preserve
  common-tone fingers when ergonomically comparable.
- [ ] `MUS-003F2` Model held-note finger substitutions explicitly.
- [x] `MUS-003F2A` Keep fingers occupied by still-sounding chord tones out of
  later assignments and preserve hand order around them.
- [x] `MUS-003F2A2` Carry held-chord occupancy into a following single-note
  melody tail while retaining melodic finger progression.
- [ ] `MUS-003F2B` Represent and edit an intentional finger substitution on one
  continuously held key.
- [ ] `MUS-003F3` Use phrase, slur and articulation context.
- [x] `MUS-003I` Alternate adjacent fingers for sustained runs of rapid repeated
  notes while preserving same-finger slow repeats and manual anchors.
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
- [x] `EX-003A` Add the reviewed fingering model, renderer and C-scale family.
- [x] `EX-003B` Cover the reviewed C/G/D/A/E/F major teaching group.
- [x] `EX-003C` Complete reviewed fingering tables for all twelve major keys.
- [x] `EX-003D` Persist fingering visibility and expose it in Settings.
- [x] `EX-003E` Complete reviewed natural-minor fingering tables for all twelve
  keys.
- [x] `EX-003F` Complete reviewed harmonic-minor fingering tables for all
  twelve keys.
- [x] `EX-003G` Complete direction-aware melodic-minor fingering tables for all
  twelve keys.
- [x] `EX-003H` Add reviewed major/minor arpeggio fingerings for all twelve
  keys.
- [x] `EX-003I` Highlight hand-turn landing notes in reviewed exercises.
- [x] `EX-003J` Add root-position primary-chord fingerings.
- [x] `EX-003` Add reviewed key- and hand-specific fingering guidance.

## RESEARCH — architecture spikes

- [x] `AUD-R01` Compare maintained Rust VST3 host libraries with Pianoteq.
- [x] `AUD-R02` Prototype the block audio boundary behind a feature flag.
- [x] `AUD-001` Surface VST3 load/audio failures in the native UI and fall back
  safely without losing the previous output.
- [x] `AUD-002` Persist and atomically restore Pianoteq component/controller
  state.
- [ ] `AUD-003` Runtime-validate opening, resizing and closing Pianoteq's native
  editor on Windows (implementation is wired).
- [ ] `AUD-004` Preserve timeline sample offsets through the VST3 event queue.
- [ ] `AUD-005` Complete velocity, CC64 half-pedal, transport, reselect and
  two-hour soak acceptance.
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
- [x] `MUS-R01A` Record the native, Verovio and MuseScore boundaries and choose
  the next renderer spike.
- [x] `MUS-R01B` Measure Verovio import fidelity, SVG generation time, binary
  size and interactive highlight latency on representative piano scores.

## Definition of done

Each checked item must have:

- acceptance criteria demonstrated;
- tests proportional to risk;
- format and regression checks passing;
- release build passing when application code changes;
- user-facing documentation when behaviour changes;
- a development-log entry and commit hash.
