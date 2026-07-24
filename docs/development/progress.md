# Development progress

Updated: 2026-07-25

## Current milestone

**M0 — Trustworthy practice baseline**

Overall state: **IN PROGRESS**

## Active cycle

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

Begin `PRA-010` while the manual portion of `QA-001` remains visible: extract
deterministic note matching from the playback scene. This is the foundation for
live feedback, summaries, measure heatmaps, adaptive tempo and persistent
progress.

## Known constraints

- The custom GPU UI has no DOM and limited accessibility/automation semantics.
- The CLI/video package requires local FFmpeg development dependencies.
- Current practice statistics use `Instant`, which makes musical-time tests and
  speed-adjusted scoring unreliable; replace them rather than extending them.
- Native VST3 hosting is a realtime and lifecycle project, not merely a picker.
