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
- confidence-bearing score-note to MIDI-note alignment with stable
  `track_id`/`note_index` identities and explicit gaps on both sides.
- repeat barlines, repeat counts and numbered-ending spans, preserved before
  any playback-order expansion.

The model deliberately has no SVG, DOM, webview or wgpu layout types. Learning
features must remain usable if the renderer changes later.

## Alignment contract

Alignment is evidence, not source mutation. For each written pitch, the matcher
compares an ordered score sequence with the ordered non-drum MIDI sequence. A
bounded dynamic-programming pass can skip either side, so an added note or a
missing repeated pitch does not cascade into false matches. Candidates more
than 250 ms apart are never forced together.

Every accepted pair retains:

- the stable score event ID;
- the MIDI track ID and note index already used by the player;
- signed onset and duration differences;
- whether score time projected exactly to a MIDI pulse;
- confidence derived from those timing facts.

Unmatched score and MIDI identities remain first-class results. A one-million
cell limit per pitch bounds memory; very large sequences use a deterministic
chronological fallback. This core does not yet compensate global offset,
transposition, repeats or endings, and the current player UI does not consume
the mapping.

## Written order and performed order

`Part.measures` always remains in source-document order. Each measure now
preserves its barlines, forward/backward repeats, repeat count and numbered
ending markers. Ending labels retain both their raw text and bounded parsed
passes, so unusual exporter values are inspectable rather than silently lost.

A separate bounded playback plan now gives each performed measure visit a
deterministic occurrence identity while retaining the stable source-measure
identity. Common repeats and numbered endings are expanded without mutating or
duplicating the imported semantic score. Independent pass and visit caps make
malformed input finite; diagnostics mark capped counts, malformed endings,
unknown navigation and currently unsupported nested repeats.

Repeated event occurrences still need flattened score times before the MIDI
matcher can consume this plan. Da capo, dal segno, coda and fine require a
later explicit navigation model.

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
   player's performance timeline. The linear-timeline core is complete; repeat
   and ending playback policy remains.
5. Measure Verovio import fidelity, SVG generation time, binary size and
   highlight latency outside the main application.
6. Add a synchronized grand-staff proof of concept only after the spike meets
   explicit acceptance thresholds.

The original MIDI remains untouched. Notation layers must stay independently
switchable and must report unsupported source features visibly.
