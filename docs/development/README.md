# Neothesia sustained development

This directory is the source of truth for the long-term development of this
fork. The product is a piano practice application first and a MIDI visualizer
second.

## Documents

- Product context (`PRODUCT.md` at the repository root) — audience, purpose,
  product register, design principles and anti-references
- [Current product roadmap](product-roadmap.md) — current Web/desktop feature coverage and remaining implementation
- [Original roadmap](roadmap.md) — earlier fork direction, architecture and milestones
- [Backlog](backlog.md) — ordered, testable work items
- [Progress](progress.md) — current release, active cycle and product status
- [Development log](development-log.md) — append-only record of completed cycles
- [Piano plug-in hosting](../pages/plugin-hosting-roadmap.md) — VST3/Pianoteq plan
- [External Pianoteq routing](../pages/pianoteq-external-routing.md) — current
  standalone workflow and acceptance checklist
- [Song metadata sidecars](../pages/song-metadata-sidecars.md) — portable,
  content-bound repertoire metadata
- [Explainable fingering suggestions](../pages/fingering-suggestions.md) —
  preview/accept cost model and honest scope
- [Public practice library](../pages/practice-library.md) — reproducible,
  license-aware local MIDI corpus
- [Verovio renderer benchmark](verovio-benchmark.md) — pinned fidelity,
  performance, size and interaction evidence

- [Piece packages](../pages/piece-packages.md) — portable MIDI, scores, annotations and practice passages

## Working agreement

Development runs in small, complete cycles:

1. Choose one user-visible outcome and its acceptance criteria.
2. Update `progress.md` and mark the selected backlog item `IN PROGRESS`.
3. Implement the smallest coherent vertical slice.
4. Add automated tests for logic and a manual smoke-test checklist for GPU UI.
5. Run formatting, focused tests, the wider regression suite and a release build.
6. Update the backlog, progress page and development log in the same commit.
7. Commit only when the cycle is usable and recoverable.

A feature is not complete merely because its UI exists. Input, playback,
recovery, persistence, tests and documentation are part of the feature.

## Product principles

1. **Practice must produce feedback.** Show what was wrong, how it was wrong,
   and what to do next.
2. **Make the next repetition effortless.** Difficult bars, hands, speed and
   instrument settings should survive a restart.
3. **Preserve musical expression.** Velocity, note duration, pedal and timing
   remain intact unless the learner explicitly chooses quantization.
4. **The audio path must be trustworthy.** Stop, seek, loop and device changes
   must never leave stuck notes.
5. **Calm visual hierarchy.** The notes, keyboard and current practice target
   dominate; secondary controls stay discoverable without covering the music.
6. **Native core, narrow boundaries.** Keep the Rust/wgpu renderer. Isolate
   learning, library, persistence and instrument engines behind testable APIs.
7. **Local-first.** Practice history and song metadata work without an account
   and remain exportable.

## Status vocabulary

- `NOW` — committed to the current milestone
- `NEXT` — prepared and likely to follow
- `LATER` — valuable but not yet scheduled
- `RESEARCH` — requires a prototype or product decision
- `DONE` — acceptance criteria passed and the result was logged
- `BLOCKED` — has a named external or technical blocker

- [Practice history](../pages/practice-history.md) — filters, saved conditions, fair comparisons and reopening attempts (Cycle 116).

- [Speed ladders](../pages/speed-ladders.md) — saved progression rules, real round grading, checkpoints and continuation (Cycle 117).

- [Practice routines](../pages/practice-routines.md) — reusable plans, dated snapshots, real completion evidence and resuming work (Cycle 118).

- [Practice routines](../pages/practice-routines.md) — daily snapshots, grading and weekly recurrence
- [Practice backups](../pages/practice-backups.md) — plans, dates, records and conflict-aware restoration
