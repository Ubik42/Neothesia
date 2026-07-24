# Development progress

Updated: 2026-07-25

## Current milestone

**M0 — Trustworthy practice baseline**

Overall state: **IN PROGRESS**

## Active cycle

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

Continue `PRA-011` and `PRA-012`: define a missed-note lifecycle and replace the
pitch-keyed transient maps with occurrence-aware matching for repeated and
overlapping same-pitch notes. Then the live panel can grow into a trustworthy
attempt summary instead of a misleading whole-song percentage.

## Known constraints

- The custom GPU UI has no DOM and limited accessibility/automation semantics.
- The CLI/video package requires local FFmpeg development dependencies.
- The new matcher is deterministic, but same-pitch overlapping note occurrences
  and missed-note finalization still need explicit identity and lifecycle rules.
- Native VST3 hosting is a realtime and lifecycle project, not merely a picker.
