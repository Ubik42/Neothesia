# Product and engineering roadmap

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
| Guided playback | Wait, structured loops, adaptive tempo | Goals, editable recommendations |
| Feedback | Internal rudimentary counters | Correct/wrong/missed, early/late, duration, pedal, dynamics |
| Learning aids | Falling notes, labels, measures/beats | Fingering, notation, chords, key/scale context |
| Repertoire | File picker and last file | Searchable library, metadata, favourites, practice queue |
| Progress | Current-song history, trends and weak action | Library history, mastery |
| Sound | Built-in SoundFont, MIDI output | Reliable external Pianoteq flow, then native VST3 |
| Creation | Free-play recording, video CLI | Recording review, annotations, shareable song metadata |
| UX | Native custom GPU UI | Coherent practice workspace, accessible themes, automation hooks |

## Architecture direction

The application remains native Rust. A full React rewrite would duplicate the
renderer and weaken the realtime path without improving piano-learning logic.
New product logic should move out of scene code into explicit domains:

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
             MIDI timeline + persistence
```

### Required boundaries

- `practice`: deterministic matching, scoring, attempts and recommendations;
  it must be testable without a window, audio device or wall clock.
- `library`: stable song identity derived from content, not only file path.
- `instrument`: one interface for SoundFont, MIDI out and future VST3.
- `storage`: versioned, atomic local data with migration tests.
- `ui`: scenes render state and send actions; they do not own learning rules.

An optional HTML/React surface may later be prototyped for a rich library or
analytics dashboard, but it is not a prerequisite and must communicate through
a narrow command/query API.

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

### M3 — Musical guidance

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
