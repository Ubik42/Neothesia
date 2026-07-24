# External Pianoteq practice workflow

This is the supported low-friction path to Pianoteq-quality sound before native
VST3 hosting is implemented. Pianoteq runs as a standalone application and owns
the audio device; Neothesia sends it MIDI through a virtual MIDI cable.

```text
Physical keyboard
       |
       v
Neothesia MIDI input -> practice matching and visualization
       |
       v
Virtual MIDI cable -> Pianoteq standalone -> ASIO audio device
```

## One-time Windows setup

1. Create one virtual MIDI port with a loopback MIDI driver and give it a clear
   name such as `Neothesia to Pianoteq`.
2. Start Pianoteq standalone and open its `Devices` settings.
3. Enable the virtual port as Pianoteq's MIDI input.
4. Select the sound card's native ASIO driver when available.
5. Start with a 48 kHz sample rate and a 128- or 256-sample buffer. Increase the
   buffer if Pianoteq reports overloads or the audio crackles.
6. In Neothesia, select the physical piano as **Input** and the virtual port as
   **Output**.
7. Keep Pianoteq's direct physical-keyboard input disabled when Neothesia is
   forwarding the same keyboard. Enabling both paths causes doubled notes and
   misleading dynamics.

Pianoteq's current manual confirms that standalone `Devices` settings select
the MIDI input, audio driver, sample rate and buffer size, and recommends ASIO
for low latency on Windows:
<https://www.modartt.com/user_manual?product=pianoteq>

## Daily start

1. Start the virtual MIDI cable service if it is not persistent.
2. Start Pianoteq and load the desired instrument preset.
3. Start Neothesia and confirm the physical input plus virtual output.
4. Play one soft, one medium and one loud note.
5. Press and release the sustain pedal once.
6. Pause and resume once before beginning a long practice session.

The short check catches a wrong input, a duplicate route, a reversed pedal and
an unusable audio buffer before practice begins.

## Verify the route from Neothesia

Run this from the repository before the first session and whenever Windows
renames or loses a device:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\check-pianoteq-route.ps1
```

The check uses the same `midi-io` backend as Neothesia. It requires all three
configuration-side conditions:

1. Windows exposes an output named exactly `Neothesia to Pianoteq`.
2. Neothesia has saved that exact output selection.
3. The endpoint can be opened without sending any MIDI data.

It also reports whether a process whose name begins with `Pianoteq` is running.
Require that condition when checking the daily startup sequence:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\check-pianoteq-route.ps1 `
  -RequirePianoteq
```

Use `-PortName "Your exact port name"` when the cable has a different name.
`RouteReady = True` proves the operating-system endpoint, saved Neothesia
selection and non-destructive open probe. It deliberately reports
`PhysicalAudioVerified = False`: software cannot prove what reached the
speakers, whether the latency felt acceptable or whether a 30-minute session
remained clean. Record those results in the checklist below.

For lower-level diagnosis, list every port without requiring a configured
route:

```powershell
cargo run -p neothesia --bin midi-diagnostics --
```

## Pianoteq acceptance checklist

Use a two-hand MIDI containing velocity changes and sustain events. Record the
date, Pianoteq version, audio driver, buffer size, keyboard, exact virtual-port
name, diagnostic output and result.

### Notes and dynamics

- [ ] A physical key sounds exactly once through Pianoteq.
- [ ] Soft, medium and loud playing produces clearly different dynamics.
- [ ] MIDI-file velocities reach Pianoteq without audible flattening.
- [ ] Repeated same-pitch notes release and retrigger cleanly.
- [ ] Left- and right-hand tracks use the expected channels.

### Pedal and expressive MIDI

- [ ] CC64 pedal down and pedal up both reach Pianoteq.
- [ ] A continuous pedal produces intermediate values rather than only 0/127.
- [ ] Releasing the pedal damps sustained notes normally.
- [ ] Pitch bend and channel pressure pass through when present in the source.

### Transport safety

- [ ] Pausing while notes and pedal are held leaves no sounding note.
- [ ] Seeking backward and forward leaves no sounding note from the old time.
- [ ] Every loop boundary clears notes and pedal from the previous take.
- [ ] Restarting a song clears the previous final chord.
- [ ] Returning to the song menu clears all sound.
- [ ] Changing the selected output clears the old instrument first.
- [ ] Closing Neothesia clears all sound.

Neothesia's panic path sends explicit tracked Note Off messages, then CC64
pedal-up, All Notes Off, All Sound Off and Reset All Controllers on all 16 MIDI
channels. It runs on pause, seek, loop/restart, scene teardown and output
replacement.

### Stability

- [ ] A 30-minute practice session has no stuck notes.
- [ ] Ten rapid pause/resume cycles have no stuck notes.
- [ ] Ten loop-boundary repetitions have no stuck notes.
- [ ] Both applications can be restarted and the route restored.
- [ ] Pianoteq's performance meter shows no sustained overload.

## Troubleshooting

### No sound

- Run the route check and resolve its first failed condition.
- Confirm Pianoteq receives the virtual port, not only the physical keyboard.
- Confirm Neothesia's output is the same virtual port.
- Check Pianoteq's audio device and output channels.
- Play directly in Pianoteq to separate audio configuration from MIDI routing.

### Doubled or unnaturally loud notes

The physical keyboard is probably reaching Pianoteq both directly and through
Neothesia. Disable the direct Pianoteq input and retain the virtual route.

### Pedal remains held

Confirm the pedal produces CC64 and that its released value reaches zero.
Run the transport-safety checklist. If only Pianoteq remains stuck, capture the
Pianoteq version, MIDI route and the exact operation that triggered it.

### Crackles or delayed sound

Use the hardware manufacturer's ASIO driver when available. Raise the buffer
from 128 to 256 samples, then to 512 only if necessary. Close applications
competing for the same audio device and inspect Pianoteq's performance meter.

## Current boundary

This route provides Pianoteq sound but does not embed its editor, audio engine,
presets or state inside Neothesia. Pianoteq must be started separately. Native
single-instrument VST3 hosting remains a later, optional phase described in the
[plug-in hosting roadmap](plugin-hosting-roadmap.md).

The diagnostic opens only Neothesia's sending endpoint. It cannot inspect
Pianoteq's private device selection, confirm that Pianoteq consumed a message
or validate audio output. Those remain explicit manual acceptance boundaries,
not inferred success.
