# Product and engineering roadmap

Current execution (2026-10-02): the owner requested a long-running full-product development goal. Follow [the detailed Chinese product roadmap](product-roadmap.md); native milestones below are the original product foundation, not proof that every feature has reached the new UI.

## Product vision

Neothesia should help a pianist move from “I can see the notes” to “I can play
this passage reliably and musically.” It should support the whole loop:

```text
Import a piece -> choose a goal -> practise a short passage
       ^                                  |
       |                                  v
Keep repertoire <- review progress <- receive precise feedback
```

The target user owns a MIDI keyboard, practises local MIDI files and values a
high-quality piano sound. Beginners need note and fingering guidance; advancing
players need timing, consistency, pedal and dynamics feedback without a
game-like interface getting in the way.

## Reference products and lessons

This roadmap borrows outcomes, not implementations:

- Synthesia: wait-for-correct-note practice, separate-hand practice, notation,
  finger hints and long-term progress.
  <https://synthesiagame.com/>
- PianoBooster: accompaniment that follows the player, early/late timing
  markers, accuracy feedback and repeatable bar ranges.
  <https://www.pianobooster.org/>
- Openthesia: playback/learning/free-play modes, recording, SoundFonts, video
  export and VST support in a compact desktop product.
  <https://github.com/ImAxel0/Openthesia>
- Neothesia upstream: fast native falling-note rendering, MIDI input/output,
  track selection, looping, SoundFont playback and video rendering.

## Product capability map

| Area | Current fork | Target |
| --- | --- | --- |
| Guided playback | Wait-by-default, hand controls, loops, count-in, adaptive tempo | Editable goals and passage recommendations |
| Feedback | Note/timing/measure/hand feedback, dynamics, duration and pedal | Calibration and deeper multi-attempt coaching |
| Learning aids | Falling notes, measures/beats, exercises, fingering, synchronized score prototype | Dense-score reading modes, chords and key/scale context |
| Repertoire | Watched folders, search, editable metadata, source/license provenance, favourites, queue, missing-file repair | Category filters, mastery views and a polished library workflow |
| Progress | Persistent sessions, trends, weak passages and recommendations | Passage-level spaced review and export |
| Sound | SoundFont/MIDI plus verified direct Pianoteq VST3 audio, panic and visible route | State/editor integration and physical soak evidence |
| Creation | Free-play recording, video CLI and portable song sidecars | Recording review and practice-plan annotations |
| UX | Native GPU UI, semantic automation hooks and persisted score controls | First-run setup, accessibility and release polish |

## Architecture direction

The delivered desktop application uses a Tauri shell and a React interface. Rust retains the MIDI timeline, audio routing, matching/scoring, library identity, annotations and persistence. The native wgpu interface remains available as the original fork foundation; its capabilities do not automatically count as delivered Web capabilities. New interface work sends commands and reads shared engine state rather than reimplementing learning rules in React.

The original domain boundary diagram below remains a guide for extracting logic, but its native UI box is now also served by the React/Tauri interface:

```text
                 Native UI / wgpu scenes
                         |
      +------------------+------------------+
      |                  |                  |
 Practice domain    Library domain    Instrument domain
 matching/scoring   songs/metadata     SoundFont/MIDI/VST3
 sessions/mastery  history/search     devices/latency/state
      |                  |                  |
      +------------ application services --+
                         |
      MIDI timeline + semantic score + persistence
```

### Required boundaries

- `practice`: deterministic matching, scoring, attempts and recommendations;
  it must be testable without a window, audio device or wall clock.
- `library`: stable song identity derived from content, not only file path.
- `instrument`: one interface for SoundFont, MIDI out and future VST3.
- `storage`: versioned, atomic local data with migration tests.
- `score`: notation-neutral parts, measures, voices and annotations with exact
  musical time; MusicXML parsers and engraving engines remain adapters.
- `ui`: scenes render state and send actions; they do not own learning rules.

The React/Tauri interface is the current product surface. Browser automation exercises user workflows against the same Rust command/query service; desktop checks verify the packaged interface, devices and embedded assets. Compatibility checks are release validation, not the development objective. The detailed Chinese roadmap records actual completed scope and remaining work.

## Execution priority — updated 2026-08-11

Milestone numbers describe capability groups, not the order of current work.
The active order is now:

1. finish direct Pianoteq VST3 hosting: reliable errors/fallback, state,
   native editor, transport correctness and long-session audio stability;
2. harden everyday MIDI performance: device loss/reconnect, transport panic,
   keyboard-specific latency and representative real-hardware acceptance;
3. remove cold-start friction from the 2,000+ piece library; packaging is not
   a blocker for the local development workflow;
4. deepen deliberate-practice recommendations only where real session data
   reveals a useful next action;
5. return to dense-score reading modes and broader MusicXML compatibility
   after the performance path is dependable.

The feature-gated score prototype remains maintained and regression-tested,
but it is not on the active delivery path. Direct Pianoteq VST3 hosting is now
the active sound-source milestone; general-purpose DAW features remain out of
scope.

## Milestones

### M0 — Trustworthy practice baseline

**Outcome:** opening a song and beginning a useful practice session is reliable.

- Wait-for-notes is the default and can be changed during playback.
- Piano tracks default to human practice while accompaniment remains automatic.
- Measure numbers and optional quarter-note lines are visible.
- File selection, seek, pause, loop and exit are free of hangs and stuck notes.
- Establish the development ledger, automated checks and smoke-test routine.

**Exit:** focused tests and release build pass; one real MIDI completes a manual
open/play/loop/exit smoke test.

### M1 — Practice intelligence

**Outcome:** every attempt gives precise, credible feedback.

- Deterministic expected-note matcher with configurable timing windows.
- Correct, wrong, missed, early and late classifications.
- Chord matching that does not penalize harmless note ordering.
- Live unobtrusive feedback and an end-of-attempt summary.
- Per-measure accuracy/timing heatmap.
- Loop attempts with count-in, reset and best/last comparison.
- Adaptive tempo: increase only after a configurable mastery threshold.
- Separate left/right/both-hand results.

**Exit:** synthetic MIDI tests cover chords, repeated pitches, overlaps, pedal
and tempo changes; results persist across restart.

### M2 — Repertoire and practice continuity

**Outcome:** the learner can resume the right piece and passage in seconds.

- Watched folders, fast search, recent/favourite songs and missing-file repair.
- Content-based song identity and editable title/artist/composer metadata.
- Saved track/hand assignments, loop ranges, tempo and visual preferences.
- Practice queue: new, learning, review and mastered.
- Local session history with export and reset.
- Integrate the concise three-variant transcription workflow.

**Exit:** moving or renaming a MIDI file does not lose its practice history.

### M3 — Musical guidance (notation expansion deferred)

**Outcome:** visual guidance teaches transferable piano skills.

- MusicXML/notation feasibility prototype, then synchronized grand staff.
- Manual finger hints stored in a sidecar file.
- Fingering suggestion prototype with hand-span and movement constraints.
- Chord/key labels and scale-degree view as optional learning layers.
- Pedal lane with half-pedal values.
- Dynamics/velocity target view and legato/staccato duration feedback.
- Calibration for keyboard/audio/MIDI latency.

**Exit:** guidance layers can be independently disabled and never change the
original MIDI.

### M4 — Pianoteq-quality instrument workflow

**Outcome:** excellent sound is dependable enough for daily practice.

- Harden and document external Pianoteq routing first.
- Guarantee panic/all-notes-off on stop, seek, loop and output changes.
- Introduce a block-based realtime-safe audio boundary.
- Add optional single-instrument VST3 hosting on Windows.
- Persist Pianoteq state and expose latency/overload diagnostics.

Detailed milestones and safety rules live in
[the plug-in roadmap](../pages/plugin-hosting-roadmap.md).

### M5 — Deliberate-practice coach

**Outcome:** the application turns results into a sensible next action.

- Automatically propose difficult two-to-eight-bar passages.
- Spaced review scheduling based on passage stability, not engagement tricks.
- Consistency metrics across several attempts.
- Goal types: notes, rhythm, tempo, pedal and dynamics.
- Exercise mode for scales, arpeggios and chord progressions.
- Optional teacher notes and portable practice-plan export.

**Exit:** every recommendation cites the measured weakness that caused it and
can be dismissed or edited by the learner.

### M6 — Visual polish, accessibility and release quality

**Outcome:** the application feels intentional and remains maintainable.

- Practice-focused home screen and clear first-run device setup.
- Design tokens for colour, spacing, typography, motion and states.
- Colour-blind-safe hand palettes, high contrast and scalable text.
- Keyboard-only operation and stable semantic automation actions.
- Smooth empty/loading/error states and recoverable diagnostics.
- Crash-safe settings/history writes, migration strategy and backup/export.
- Windows installer/update plan and repeatable release checklist.

**Exit:** common flows pass keyboard, DPI, resize and long-session testing on
the supported Windows baseline.

## Priority rules

When choosing between tasks, prefer:

1. correctness and recovery over new surface area;
2. feedback quality over decorative gamification;
3. passage-level practice over whole-song score chasing;
4. testable domain logic over logic embedded in rendering scenes;
5. preserving expressive MIDI data over destructive transformations;
6. a complete vertical slice over several half-built controls.

## Explicit non-goals

- Becoming a DAW, notation editor or plug-in effect rack.
- Replacing a teacher with opaque “AI” judgments.
- Accounts, social feeds or cloud lock-in before local practice is excellent.
- A full React rewrite of the native player.
- Reward systems that optimize time-in-app rather than playing quality.
