# MusicXML and grand-staff architecture

Neothesia treats notation as learning data, not merely a picture. MusicXML is
imported into a small internal score model; practice analysis and fingering use
that model; a replaceable renderer is responsible only for layout.

```text
MusicXML / MXL
      |
      v
notation-neutral Score --------> phrase / fingering / score feedback
      |                                      |
      v                                      v
engraving adapter <--------------- stable note and measure identities
      |
      v
grand staff synchronized with the existing player clock
```

## What the first importer preserves

- title, composer, parts and numbered measures;
- exact rational quarter-note positions and durations;
- pitch, rest, chord, voice and staff identity;
- key signature, time signature, divisions and clefs;
- direction words, dynamics, tempo and pedal marks;
- fingering, ties, slurs, articulations and tuplet notation;
- polyphonic `backup` and `forward` timing.
- deterministic part/measure/kind/ordinal identities for notes and directions.
- exact score-time projection through a paired MIDI PPQ and tempo map, including
  explicit pulse-rounding and unprojected gaps.

The model deliberately has no SVG, DOM, webview or wgpu layout types. Learning
features must remain usable if the renderer changes later.

## Renderer decision

| Option | Strength | Cost or risk | Decision |
| --- | --- | --- | --- |
| Build engraving in native wgpu | Full visual control | Professional notation layout is a large product by itself | Do not pursue |
| Verovio to SVG | Portable, open source, embeddable, MusicXML input | Needs fidelity, size, timing and native-display measurements | Next isolated spike |
| MuseScore conversion | Broad import/export and mature engraving | Large external application; poor embedded runtime boundary | Optional authoring/export tool |
| React rewrite | Easy SVG/DOM display | Replaces a working native interaction/render loop without improving score semantics | Not required |

MusicXML 4.0 supports much more than this first slice. The importer emits
warnings for recognized deferred notation rather than silently claiming full
support. See the [MusicXML 4.0 specification](https://www.w3.org/2021/06/musicxml40/),
[Verovio input documentation](https://book.verovio.org/toolkit-reference/input-formats.html)
and [MuseScore MusicXML guidance](https://handbook.musescore.org/file-management/working-with-musicxml-files).
The pinned real-score evidence and exact counts live in the
[compatibility matrix](../development/musicxml-compatibility).

## Planned slices

1. Build a representative corpus of exported piano scores; bounded compressed
   MXL input is already supported.
2. Add score-timewise conversion when corpus evidence justifies its priority.
3. Model repeats/endings, tuplets, transposition, pedal and ornament semantics
   needed by learning features.
4. Give score notes and measures stable identities and align them with the
   player's performance timeline.
5. Measure Verovio import fidelity, SVG generation time, binary size and
   highlight latency outside the main application.
6. Add a synchronized grand-staff proof of concept only after the spike meets
   explicit acceptance thresholds.

The original MIDI remains untouched. Notation layers must stay independently
switchable and must report unsupported source features visibly.
