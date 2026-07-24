# Native practice UI automation contract

Neothesia uses a custom GPU-rendered UI rather than a DOM. Visible labels are
product copy and may change; automation must target stable action IDs instead
of text or screen coordinates.

## Stable practice actions

| Action ID | Meaning |
| --- | --- |
| `practice.player.back` | Leave the active player |
| `practice.player.wait` | Toggle wait-for-notes |
| `practice.player.coach` | Toggle adaptive tempo |
| `practice.player.hands` | Cycle both/right/left-hand practice |
| `practice.completion.tab.overview` | Open completion Overview |
| `practice.completion.tab.technique` | Open completion Technique |
| `practice.completion.tab.history` | Open completion History |
| `practice.completion.calibrate` | Confirm suggested timing offset and retry |
| `practice.completion.notes-loop` | Start the note-accuracy recommendation |
| `practice.completion.rhythm-loop` | Start the rhythm recommendation |
| `practice.completion.retry` | Restart the current practice scope |
| `practice.completion.back` | Return to the song menu |

IDs are namespaced, unique and covered by a catalogue test. Removing or
changing an ID is an automation-contract change and must be recorded in the
development log.

## Current boundary

The IDs are assigned inside Nuon's retained interaction state, making
automation independent of translated labels. Debug builds now expose an
in-process harness that sends semantic actions through the normal application
event loop and requests a read-only practice snapshot through that same loop.

The snapshot currently exposes wait mode, Tempo Coach, selected hands,
completion tab, matched/wrong/missed totals and input-latency compensation.
The harness can activate player back/wait/coach/hands, completion tab
navigation, retry and back. Calibration and recommendation actions remain
click-only because their parameters are derived from the rendered completion
model; they must not be reported as supported by an external driver yet.

Both the harness and its event variants are excluded from release builds with
`debug_assertions`. It is not yet exposed through Windows UI Automation or an
external inspection protocol.

The next automation layer should:

1. expose a controlled driver endpoint for the in-process debug harness;
2. acknowledge whether each action was accepted by the active scene;
3. capture deterministic screenshots at supported window sizes;
4. cover parameterized calibration and recommendation actions without
   duplicating their product logic;
5. retain compile-time exclusion from release builds.

Screen-coordinate automation remains a temporary smoke-test fallback and must
not become the primary regression suite.
