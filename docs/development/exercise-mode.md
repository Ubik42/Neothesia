# Exercise mode design

Exercise mode is a generated practice source, not a separate scoring system.
Generated exercises must enter the same `Song` → `PlayingScene` →
`PracticeMatcher` path as imported MIDI so wait mode, hands, loops, Tempo
Coach, history and completion feedback remain consistent.

## Core specification

An exercise is defined by:

- tonic pitch class C–B;
- major or minor tonality;
- scale, arpeggio or primary-chord pattern;
- ascending, descending or up-and-down direction;
- right, left or both hands;
- one to three octaves;
- 20–240 BPM.

The initial register is C4–B4 for the right hand and C2–B2 for the left. Every
generated note must fit the configured keyboard range. Invalid tonic, span,
tempo or range combinations fail before playback.

Scale exercises use major or natural-minor steps. Arpeggios use the tonic
major/minor triad. Primary-chord exercises use I–IV–V–I in major and
i–iv–V–i in minor, intentionally raising the minor leading tone in the
functional dominant.

Both hands move in parallel two octaves apart. Up-and-down exercises play the
apex once. Melodic moments last one beat; chord moments last two beats.

## Integration boundary

`neothesia_core::exercise::ExercisePlan` is deterministic and independent of
rendering. The next layers are:

1. convert a plan to an in-memory Type-1 MIDI with named right/left tracks;
2. identify generated exercises separately from file-backed repertoire;
3. add a focused preset editor and preview to the home screen;
4. pass the generated `Song` into the existing player;
5. save exercise progress by normalized specification identity;
6. add fingering only from reviewed per-key/per-hand tables.

Fingering is deliberately absent from the generator today. Generic
one-pattern-fits-all scale fingering would teach incorrect crossings in several
keys.
