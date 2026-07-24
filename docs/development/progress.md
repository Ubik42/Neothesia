# Development progress

Updated: 2026-07-25

## Current milestone

**M0 — Trustworthy practice baseline**

Overall state: **IN PROGRESS**

## Active cycle

### Cycle 003 — Repeated notes and missed-note lifecycle

State: **DONE**

Delivered:

- replaced pitch-keyed single entries with FIFO occurrence queues;
- matched repeated and overlapping same-pitch targets independently;
- added a configurable late window and explicit missed-note totals;
- enabled the same matcher in flow practice when wait mode is off;
- kept guided wait targets pending indefinitely instead of misclassifying the
  learner while they search for the note;
- expanded the live status to show hits, wrong notes, misses and current need.

Verification:

- eight practice-domain tests and ten application tests pass;
- application integration tests distinguish guided waits from flow misses;
- release build passes;
- implementation commit: `fbee89d`.

### Cycle 002 — Deterministic practice feedback

State: **DONE**

Delivered:

- extracted matching and session totals from `MidiPlayer` into
  `neothesia_core::practice`;
- replaced direct wall-clock reads with a caller-supplied monotonic session
  time, allowing precise tests without sleeps;
- classified matched notes as early, on-time or late with configurable windows;
- counted expired and duplicate presses as wrong notes;
- tracked unordered target chords and exposed the number of notes still needed;
- added a compact live `Correct / Wrong / Waiting` status in the player;
- froze practice timing while paused while continuing it during guided waits.

Verification:

- five focused matcher tests cover timing, expiry, chords, keyboard range and
  duplicate presses;
- full `neothesia-core` and `neothesia` target tests pass;
- release build passes;
- implementation commit: `bab60ea`.

### Cycle 001 — Guided-practice baseline

Goal: make a selected two-hand piano MIDI immediately usable for practice.

Included:

- default piano note tracks to human-controlled practice;
- wait-for-notes enabled by default;
- obvious `Wait: ON/OFF` control in the player;
- optional quarter-note guidelines;
- one-based measure numbers;
- configuration migration defaults and focused tests.

Acceptance checklist:

- [x] Legacy settings deserialize with practice-friendly defaults.
- [x] Wait mode can release a blocked player when disabled.
- [x] MIDI parsing produces ordered measure and beat timestamps.
- [x] Focused tests pass.
- [x] Native release build passes.
- [ ] Real MIDI smoke test covers open, wait, toggle, loop and exit.
- [x] Implementation committed as `a1755bc`.
- [x] Cycle documentation committed.

## Product health snapshot

| Capability | State | Notes |
| --- | --- | --- |
| MIDI open/play | Working | File-picker transition fix committed |
| Guided wait | Verifying | Current Cycle 001 |
| Measure/beat grid | Verifying | Current Cycle 001 |
| Loop practice | Basic | Drag handles exist; no attempt/count-in model |
| Performance feedback | Missing | Cycle 002 target |
| Practice history | Missing | Planned for M1/M2 |
| Built-in piano | Working | SoundFont fallback |
| External Pianoteq | Possible | MIDI routing needs validation guide |
| Native VST3 | Planned | Separate long-term roadmap |
| Library | Minimal | File picker and recent path only |
| UI automation | Partial | OS input/screenshot; semantic actions planned |

## Next decision

Begin `PRA-015` and `PRA-016`: attach score-note context (track/hand, MIDI time
and measure) to match outcomes, aggregate a completed attempt, and present an
end-of-attempt summary. This keeps the next UI grounded in passage-level data
instead of a misleading whole-song percentage.

## Known constraints

- The custom GPU UI has no DOM and limited accessibility/automation semantics.
- The CLI/video package requires local FFmpeg development dependencies.
- Live totals are not yet grouped by measure or hand and are not persisted.
- Native VST3 hosting is a realtime and lifecycle project, not merely a picker.
