# Native practice UI automation contract

Neothesia uses a custom GPU-rendered UI rather than a DOM. Visible labels are
product copy and may change; automation must target stable action IDs instead
of text or screen coordinates.

## Stable practice actions

| Action ID | Meaning |
| --- | --- |
| `practice.menu.start` | Start the currently loaded song |
| `practice.player.back` | Leave the active player |
| `practice.player.wait` | Toggle wait-for-notes |
| `practice.player.coach` | Toggle adaptive tempo |
| `practice.player.hands` | Cycle both/right/left-hand practice |
| `practice.player.loop` | Toggle the current measure-snapped loop |
| `practice.player.restart` | Restart the current whole-song or loop scope |
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

The snapshot currently exposes wait mode, Tempo Coach, selected hands, loop
activation/range/count-in state, pause state, completion tab,
matched/wrong/missed totals, currently required note pitches and input-latency
compensation. It also exposes additive engraved-score state: artifact readiness,
cached-page count, focused page, uploaded texture page and texture dimensions.
It also reports persisted score visibility independently of texture presence.
Synchronization readiness is separate again, so a rendered page cannot conceal
a failed native-to-MIDI-to-renderer correlation boundary.
The harness can start the currently loaded song, activate player
back/wait/coach/hands, and navigate completion tabs, retry and back. Each
activation waits for an explicit accepted or rejected result from the active
scene, with a caller-supplied timeout.
Calibration and recommendation actions remain click-only because their
parameters are derived from the rendered completion model; they must not be
reported as supported by an external driver yet.

Both the harness and its event variants are excluded from release builds with
`debug_assertions`.

## Local debug driver

Debug builds can expose the harness to a local test process by setting
`NEOTHESIA_DEBUG_DRIVER_ADDR` to an explicit loopback socket such as
`127.0.0.1:32123` before launch. The driver is off by default, rejects
non-loopback addresses and is not compiled into release builds.

Open one TCP connection per command and send one newline-terminated command:

| Command | Result |
| --- | --- |
| `ACTION practice.player.wait` | JSON with `ok` and `accepted` |
| `MIDI 0 60 100` | Inject channel, note and velocity through player MIDI input |
| `SNAPSHOT` | JSON with `ok` and a snapshot object or `null` |
| `EXIT` | JSON acknowledgement followed by a clean application exit |

Commands are limited to 4096 bytes and action/state waits time out after two
seconds. MIDI channels must be 0–15; notes and velocities must be 0–127. A
velocity of zero is a note release. An accepted value of `false` means the
active scene does not support that action in its current state. A `null`
snapshot means the active scene is not the player.

This is a narrow test protocol, not a general remote-control API or a Windows
UI Automation implementation.

On Windows, run the checked-in end-to-end smoke sequence with:

```powershell
.\scripts\debug-practice-smoke.ps1 -MidiPath "D:\path\to\song.mid"
```

The script builds the Debug executable, selects an unused loopback port, uses
an isolated temporary working directory, starts the loaded song, asserts the
default wait state, waits for required pitches, injects their note-on/releases
through the real scene MIDI path, asserts that the matcher count increases,
toggles wait mode, cycles hands when available, exercises loop/restart, returns
to the menu and requests a clean exit. It removes its temporary settings,
history and SoundFont copy afterward.

Run the deterministic completion path without supplying a song:

```powershell
.\scripts\debug-practice-smoke.ps1 -CompletionFixture
```

This mode creates a tiny Type-1 two-hand MIDI inside the isolated run
directory, performs both required notes, verifies completion opens on Overview,
switches through Technique and History, returns to Overview, selects Retry and
asserts that the scored attempt resets. The fixture is deleted with the run
directory.

Run the complete feature-gated engraving boundary with:

```powershell
.\scripts\debug-practice-smoke.ps1 -ScoreFixture
```

This mode creates an isolated MIDI and MusicXML pair, writes their normal
content-bound sidecar through the `score-associate` developer example, starts
the pinned Verovio worker, waits for a verified raster and asserts that page zero
is the sole focused GPU texture. It then completes the ordinary wait, MIDI,
hands, loop, restart, menu and clean-exit sequence. Before that sequence it
invokes `practice.player.score`, proves that hiding releases only the GPU image,
invokes it again, proves the same cached texture is restored, and checks the
restored preference in `settings.ron`. The default package root is
`D:\cs\_test\neothesia-verovio\node_modules\verovio` and can be overridden with
`-VerovioPackageRoot`.

For a DPI-aware native-window artifact, add:

```powershell
.\scripts\debug-practice-smoke.ps1 -ScoreFixture `
  -ScreenshotPath ".\target\score-smoke.png"
```

The optional capture uses the real HWND and accounts for per-window DPI. It is
visual evidence only; semantic snapshot assertions remain the regression gate.

The next automation layer should:

1. add explicit minimum/default/large window sizing to screenshot capture;
2. cover parameterized calibration and recommendation actions without
   duplicating their product logic;
3. extend the completion fixture through calibration and recommendations when
   it can carry enough meaningful evidence;
4. retain compile-time exclusion from release builds.

Screen-coordinate automation remains a temporary smoke-test fallback and must
not become the primary regression suite.
