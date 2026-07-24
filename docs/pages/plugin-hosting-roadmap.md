# Piano plug-in hosting roadmap

This document proposes a long-term path for using high-quality software
instruments, including Pianoteq, directly inside Neothesia.

## Current status

Neothesia does **not** currently host VST3 plug-ins.

Pianoteq can still be used today in its standalone mode:

1. Create or select a MIDI output route that is visible to both applications.
2. Select that route as Neothesia's output.
3. Select the same route as Pianoteq's MIDI input.
4. Let Pianoteq own the audio device and configure its buffer size.

This gives users Pianoteq audio while practising with Neothesia, but the two
applications keep separate audio settings, windows, presets, and lifecycles.
Native hosting is intended to remove that friction.

Pianoteq supports standalone operation and the VST3 plug-in format on Windows,
macOS, and Linux. Its documentation recommends ASIO on Windows for low-latency
operation.

## Product direction

The objective is a reliable, low-latency **single-instrument practice mode**,
not a general-purpose DAW.

The first supported plug-in format should be VST3. The implementation should
remain behind an internal interface so another host library or plug-in format
can be added without changing the practice, MIDI, or rendering systems.

### Primary use case

- Load one 64-bit VST3 instrument.
- Send live keyboard and MIDI-file events to it.
- Hear the instrument through Neothesia's selected audio device.
- Preserve velocity, sustain and continuous pedal values, pitch bend, and
  channel pressure.
- Restore the chosen plug-in and its state on the next launch.
- Recover safely from device and plug-in errors.

### Non-goals for the first releases

- Multiple instrument tracks or effect chains
- Audio recording and editing
- Parameter automation
- Mixing, routing, or mastering features
- VST2 support
- A replacement for a full DAW

## Architecture

The existing output abstraction is the main integration point:

```text
Practice state / MIDI playback / live keyboard
                       |
                 OutputConnection
          +------------+-------------+
          |            |             |
      SoundFont     MIDI Out     Plug-in instrument
                                    |
                              Audio block runner
                                    |
                              Audio device output
```

The plug-in implementation should be separated into four layers:

1. **Catalog** — discovers plug-ins and caches metadata outside the audio
   thread.
2. **Instance** — owns plug-in activation, buses, parameters, and saved state.
3. **Realtime runner** — exchanges timestamped MIDI events and audio blocks
   without allocation, blocking, logging, or filesystem access.
4. **Editor bridge** — opens and closes the plug-in's native editor without
   coupling it to the renderer.

A small internal interface should be introduced before selecting a permanent
third-party host library:

```rust
trait InstrumentBackend {
    fn descriptor(&self) -> &InstrumentDescriptor;
    fn send_midi(&self, event: TimedMidiEvent);
    fn set_gain(&self, gain: f32);
    fn stop_all(&self);
}
```

The concrete API may differ, but UI and practice code must not depend directly
on VST3 SDK types.

## Milestones

### Phase 0 — External Pianoteq workflow

**Goal:** document and validate the immediately available MIDI Out workflow.

- Add a user guide for routing Neothesia MIDI Out to Pianoteq standalone.
- Verify note on/off, all 127 velocity values, CC64 sustain, continuous pedal
  values, pitch bend, pause, seek, restart, and stop.
- Document a low-latency starting configuration and troubleshooting steps.
- Ensure stopping or changing a song always sends All Notes Off and All Sound
  Off.

**Exit criteria**

- A user can complete a 30-minute practice session without stuck notes.
- The workflow is repeatable after both applications restart.
- Limitations of the external route are clearly documented.

### Phase 1 — Audio engine boundary

**Goal:** make the current synthesizer path safe for block-based plug-ins.

- Separate audio-device ownership from SoundFont synthesis.
- Introduce planar/interleaved stereo block buffers as required by the host.
- Replace the current per-sample event polling with a bounded, realtime-safe
  event queue drained once per audio block.
- Attach sample offsets to events occurring within a block.
- Handle sample-rate, buffer-size, stream-loss, and output-device changes.
- Add underrun/xrun counters that are collected without logging on the audio
  thread.
- Retain SoundFont output as a regression-tested fallback.

**Exit criteria**

- No heap allocation, locks, filesystem access, or UI calls occur in the audio
  callback.
- SoundFont playback has no audible regression.
- Automated tests cover event ordering, queue overflow policy, panic-safe
  shutdown, and All Notes Off.

### Phase 2 — Windows VST3 proof of concept

**Goal:** load one explicitly selected VST3 instrument and produce audio.

- Add a build-time `vst3-host` feature.
- Load a plug-in from an explicitly selected VST3 bundle; scanning is deferred.
- Activate one stereo instrument bus at the audio device's sample rate and
  block size.
- Translate Neothesia MIDI messages into sample-accurate VST3 events.
- Implement load, activate, deactivate, unload, and audio-device restart.
- Add a safe timeout/error path that returns the user to SoundFont or MIDI Out.
- Keep the native plug-in editor optional; a generic status panel is enough.

**Pianoteq acceptance suite**

- Notes and releases are never lost during normal practice.
- Velocity values from 1 through 127 reach Pianoteq unchanged.
- CC64 values are preserved, including half-pedalling values.
- Repeated pause, seek, restart, and song changes do not leave sounding notes.
- A two-hour practice soak test produces no crash or unbounded memory growth.
- Audio remains stable at practical buffer sizes; the UI reports overloads
  rather than hiding them.

**Exit criteria**

- Pianoteq can be loaded, played, stopped, unloaded, and loaded again in one
  process.
- A plug-in failure produces an actionable error and does not corrupt saved
  settings.

### Phase 3 — Usable Windows integration

**Goal:** make native hosting convenient enough for everyday practice.

- Discover standard VST3 locations and provide user-added search locations.
- Scan in a worker process or guarded helper so a bad plug-in cannot prevent
  Neothesia from starting.
- Cache plug-in class ID, name, vendor, version, category, path, and scan result.
- Provide rescan, blacklist, and "open containing folder" actions.
- Open the native plug-in editor in a separate window first.
- Persist the selected plug-in and opaque component/controller state
  atomically.
- Add explicit audio device, sample rate, and buffer-size controls.
- Display measured/reported latency and audio overload count.
- Add a one-click panic action that clears queued events and silences all
  channels.

**Exit criteria**

- Pianoteq is restored with its previous instrument preset after relaunch.
- Moving, updating, or removing a plug-in results in a recoverable state.
- Scanning a malformed plug-in cannot permanently break application startup.

### Phase 4 — Reliability and release readiness

**Goal:** make VST3 hosting supportable for general Windows users.

- Add structured diagnostics with plug-in and device details, excluding
  personal paths where possible.
- Maintain a small compatibility matrix headed by Pianoteq.
- Test device unplug/replug, sleep/resume, sample-rate changes, rapid output
  switching, and high-MIDI-density files.
- Add CI tests for host-independent event translation and state handling.
- Add manual release checks using at least two independent VST3 instruments.
- Decide whether full process isolation is necessary based on crash reports and
  the host library's capabilities.

**Exit criteria**

- No known realtime-safety violations.
- All lifecycle and recovery tests pass.
- The feature can be disabled at build time without affecting existing output
  modes.

### Phase 5 — Cross-platform expansion

**Goal:** extend only after the Windows implementation is stable.

- Validate VST3 discovery, editor parenting, and audio backends on macOS and
  Linux.
- Keep state files portable where plug-ins permit it.
- Evaluate CLAP as a second format only if user demand and host-library
  maturity justify the maintenance cost.
- Treat Audio Unit as a separate product decision rather than an automatic
  requirement.

**Exit criteria**

- Platform-specific behavior stays behind narrow adapters.
- Existing SoundFont and MIDI Out behavior remains available on every platform.

## Engineering rules

- The audio callback is a hard realtime boundary.
- Every bounded queue needs a documented overflow policy.
- Plug-in discovery and state serialization never run on the audio thread.
- Stop, disconnect, seek, and panic paths must silence notes deterministically.
- Unknown MIDI controllers are preserved when the plug-in format supports them.
- Plug-in state is opaque data and must be written atomically.
- A failed plug-in load never prevents Neothesia from starting.
- VST3 support remains optional so packagers can build without it.

## Dependency strategy

Rust VST3 host libraries should be evaluated with a small throwaway prototype
before they enter the application dependency graph. Selection criteria:

- VST3 lifecycle and bus negotiation coverage
- Sample-accurate event support
- Realtime-safe processing API
- Native editor support
- Windows, macOS, and Linux behavior
- License compatibility
- Active maintenance and an acceptable unsafe-code surface

Any selected library must be pinned and wrapped by Neothesia's own interfaces.
This is especially important while the Rust hosting ecosystem is young.

## Delivery and issue breakdown

Each phase should be delivered as reviewable changes rather than one large pull
request:

1. Architecture decision record and realtime queue tests
2. Shared block-based audio engine
3. VST3 lifecycle prototype behind a disabled feature
4. Pianoteq MIDI acceptance tests and soak-test checklist
5. Settings and error-recovery UI
6. Scanner/helper process and metadata cache
7. Native editor window and state restoration
8. Documentation, compatibility matrix, and release checklist

Progress should be tracked by milestone. An item is complete only when its exit
criteria pass; elapsed time alone must not advance the roadmap.

## References

- [Pianoteq manual](https://www.modartt.com/user_manual?product=pianoteq)
- [Steinberg VST 3 SDK](https://www.steinberg.net/developers/vstsdk/)
- [Rust `vst3-host` documentation](https://docs.rs/vst3-host/latest/vst3_host/)
- [Rust `clack-host` documentation](https://docs.rs/clack-host/latest/clack_host/)
