# Exercise mode design

Exercise mode is a generated practice source, not a separate scoring system.
Generated exercises must enter the same `Song` → `PlayingScene` →
`PracticeMatcher` path as imported MIDI so wait mode, hands, loops, Tempo
Coach, history and completion feedback remain consistent.

## Using Technique Studio

Open **Technique Studio** from the home screen. Use each parameter's left and
right arrows, then choose **Start Exercise** or press Enter. The initial preset
is C major scale, up and down, both hands, one octave at 60 BPM. After starting
an exercise, that complete selection becomes the default for the next launch.
Escape returns to the home screen.

The generated exercise opens in the normal practice player with wait-for-notes
enabled. Hand switching, loops, tempo changes, scoring, completion feedback and
external MIDI output therefore work exactly as they do for an imported song.

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

1. ~~convert a plan to an in-memory Type-1 MIDI with named right/left tracks;~~
2. ~~identify generated exercises separately from file-backed repertoire;~~
3. ~~add a focused preset editor and preview to the home screen;~~
4. ~~pass the generated `Song` into the existing player;~~
5. ~~save exercise progress by normalized specification identity;~~
6. add fingering only from reviewed per-key/per-hand tables.

The MIDI conversion uses a Type-1 file with a conductor track plus distinct
right- and left-hand tracks. It emits 480 ticks per beat, 4/4 meter, the
requested tempo and an 80% note gate. The result is memory-backed, has a stable
content identity for the same specification and is accepted by the regular
`Song` configuration. Both-hand plans therefore inherit the existing hand
practice shortcuts without a second player implementation.

The player entry path also assigns hand identity explicitly from the generated
track contract. This keeps right-only and left-only exercise feedback correctly
scoped even though generic one-track MIDI cannot safely infer a hand.

The last-used specification is stored in the versioned application settings.
Older settings default to the initial C-major preset. Structurally invalid
persisted values are rejected before the menu renders, preventing an invalid
tonic, octave span or tempo from crashing the selector.

## Learning identity and tempo

Exercise history is keyed by the musical target: tonic, tonality, pattern,
direction and octave span. Tempo and hand scope are attempt dimensions rather
than separate songs. Moving the same scale from 60 to 80 BPM, or progressing
from separate hands to both hands, therefore keeps one continuous history.
Different keys and patterns never share an identity.

Every completed exercise attempt also records its effective BPM after the
player speed multiplier is applied. History rows show real BPM plus the
multiplier, and the trend prefers BPM change when that evidence exists. Legacy
song attempts without BPM data continue to display multiplier-only speed.

## Practice Library

The latest complete generated specification is stored with the song setup in
practice history. Practice Library recognizes this as an available generated
source, labels its primary action **Practice**, rebuilds it without a file
picker and restores the saved tracks, hand mode, speed and loop. File-backed
MIDI keeps its existing Open/Locate behavior.

Fingering is deliberately absent from the generator today. Generic
one-pattern-fits-all scale fingering would teach incorrect crossings in several
keys.
