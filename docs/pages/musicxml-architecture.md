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

Each included visit now carries cumulative performed score time. Notes and
directions expand into occurrence records identified by stable source event ID
plus measure-visit occurrence ID; their measure-relative onsets are projected
through the paired MIDI PPQ and tempo map. This correctly gives repeated source
notes distinct player-clock times without duplicating source semantics.

The primary gap-aware MIDI-note matcher now consumes these repeated
occurrences. Match and unmatched evidence uses stable source-plus-occurrence
IDs, and performed measure/pass signatures are compared across piano parts.
Navigation conflicts and bounded-plan warnings remain explicit in the
alignment result. Da capo, dal segno, coda and fine require a later explicit
navigation model.

## Compatibility verdict

The core reduces alignment evidence to a conservative structured verdict:

| Verdict | Meaning |
| --- | --- |
| Ready | navigation complete, at least 95% coverage, at least 85% mean confidence and no missing score notes |
| Review | navigation complete, at least 75% coverage and at least 70% mean confidence |
| Poor | navigation works but note evidence is below the review threshold |
| Blocked | navigation is incomplete or conflicting, regardless of note percentage |

The summary also retains matched notes, score gaps, MIDI gaps, inexact PPQ
projections and navigation diagnostics. UI layers must show this evidence
rather than replacing it with an unrelated compatibility calculation.

## Song association

The existing MIDI-content-bound `.neothesia.ron` sidecar can hold one optional
score association. A score must parse before it is saved; the association
stores a BLAKE3 fingerprint and uses a MIDI-relative path when possible.
Loading can therefore distinguish missing, invalid, replaced and verified
scores without touching the original MIDI or score. Practice Library Info now
exposes native Pair, Replace and Remove controls plus this health state.
Analyze runs the complete alignment pipeline off the UI thread and caches the
structured readiness evidence against the score fingerprint. Pair, Replace and
Remove invalidate stale analysis automatically.

## Renderer synchronization contract

The renderer does not receive practice authority. An adapter returns a bounded
page count plus a one-to-one index from native `ScoreEventId` values to its own
private element IDs. The index rejects empty IDs, duplicate score mappings,
duplicate renderer IDs and out-of-range pages before playback can consume it.

The native alignment produces an occurrence-aware highlight timeline from the
matched MIDI note timestamps. A frame contains active notes plus a stable focus
for page following. During rests, focus remains on the latest started note
instead of jumping ahead. Repeated passages reuse the same written renderer
element while retaining their distinct performed occurrence identities.

Missing MIDI or renderer elements remain explicit diagnostics and never cause
the player to guess or panic. This protocol is implemented without Verovio,
JavaScript, SVG or web-view types; the next adapter must satisfy it behind a
feature flag.

## Renderer decision

| Option | Strength | Cost or risk | Decision |
| --- | --- | --- | --- |
| Build engraving in native wgpu | Full visual control | Professional notation layout is a large product by itself | Do not pursue |
| Verovio to SVG | Portable, open source, embeddable, MusicXML input | 6.66 MiB module; requires page virtualization and a native display boundary | Approved for an isolated proof of concept |
| MuseScore conversion | Broad import/export and mature engraving | Large external application; poor embedded runtime boundary | Optional authoring/export tool |
| React rewrite | Easy SVG/DOM display | Replaces a working native interaction/render loop without improving score semantics | Not required |

MusicXML 4.0 supports much more than this first slice. The importer emits
warnings for recognized deferred notation rather than silently claiming full
support. See the [MusicXML 4.0 specification](https://www.w3.org/2021/06/musicxml40/),
[Verovio input documentation](https://book.verovio.org/toolkit-reference/input-formats.html)
and [MuseScore MusicXML guidance](https://handbook.musescore.org/file-management/working-with-musicxml-files).
The pinned real-score evidence and exact counts live in the
[compatibility matrix](../development/musicxml-compatibility).
The renderer measurements and adoption constraints live in the
[Verovio benchmark](../development/verovio-benchmark).

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
   highlight latency outside the main application. This spike passed with
   explicit size, virtualization and pedal-fidelity constraints.
6. Add a feature-flagged synchronized grand-staff proof of concept behind a
   renderer adapter, keeping the Rust semantic model authoritative.

The original MIDI remains untouched. Notation layers must stay independently
switchable and must report unsupported source features visibly.
