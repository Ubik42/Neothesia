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

The IDs are assigned inside Nuon's retained interaction state, making in-process
automation deterministic and independent of translated labels. They are not
yet exposed through Windows UI Automation or an external inspection protocol.

The next automation layer should:

1. expose the active semantic action catalogue in debug/test builds;
2. allow activation by ID through the application event loop;
3. expose a small read-only state snapshot for assertions;
4. capture deterministic screenshots at supported window sizes;
5. keep the external test interface disabled in release builds.

Screen-coordinate automation remains a temporary smoke-test fallback and must
not become the primary regression suite.
