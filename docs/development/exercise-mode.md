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

## Recent and favourite presets

Starting an exercise automatically moves its complete specification to the
front of the **Recent** selector. The list is deduplicated and retains the eight
most recently used variants. **Save favourite** stores the current complete
specification; the same button removes it when already saved. The
**Favourites** selector retains up to twelve variants.

Both selectors restore every visible parameter in one action: key, scale form,
pattern, direction, hands, octave span, tempo and repetition count. Their cards
show the selected position plus a compact key/pattern label, while the main
parameter grid remains the authoritative detail view. Invalid or duplicated
records in a manually edited settings file are filtered when read. Older
settings default to empty preset lists.

## Fingering guidance

All twelve major scales, all twelve natural-minor scales, plus C harmonic-minor
and melodic-minor carry reviewed finger numbers for both hands. The mapping
follows the exact generated sequence, including ascending, descending and
up-and-down direction, one to three octaves and every repetition. Finger
numbers 1–5 are centered directly on the falling notes and are enabled by
default when available. The player shows a clear **Fingers: ON/OFF** control.

That control is a persistent appearance preference, not a per-attempt
temporary state. Changing it saves immediately and the same option appears as
**Exercise Fingerings** in Settings. Older settings default to visible.
Turning guidance off keeps it available for later re-enabling; if ordinary
note-name labels are enabled, they become visible again while finger numbers
are off.

C/G/D/A/E use the common `12312345` right-hand and `54321321` left-hand
one-octave pattern. F major retains the common left hand but uses
`12341234` in the right hand so finger 4 plays B♭. The reviewed basis is
[Baylor Piano Basics](https://openbooks.library.baylor.edu/pianobasics/chapter/one-octave-major-scales/)
for the shared five-key group and the
[F major reference at piano.org](https://piano.org/scales/major/f/) for the
exception (checked 2026-07-25).

The remaining major tables follow the two-octave visual charts at
[Piano-ology](https://piano-ology.com/piano-technique/major-scale-fingering/).
B major keeps the common right hand but starts left hand on 4. D♭, E♭, A♭ and
B♭ use their reviewed flat-key starts and thumb crossings. G♭ uses 2–3 and
2–3–4 over the black-key groups. The selector and generated title prefer D♭,
E♭, A♭ and B♭ spellings for major keys, while keeping C♯ for C♯ minor.

Natural-minor tables follow the per-key, two-hand visual charts in
[Piano-ology's natural-minor fingering reference](https://piano-ology.com/wp-content/uploads/2024/01/piano-ology-piano-technique-fingering-charts-natural-minor-scales.pdf)
(checked 2026-07-25). Black-root keys use their own starting fingers and
crossings rather than inheriting the parallel major. A♭ major and G♯ minor are
spelled differently in both the selector and generated title.

Coverage is deliberately explicit and Technique Studio says whether the
current selection has reviewed guidance before playback. Unreviewed
harmonic/melodic minor keys and non-scale patterns show no fingering control or
numbers. This avoids silently teaching a generic crossing pattern in keys where
the accepted fingering differs.
Note-name labels continue to work for imported MIDI and Free Play; on a
supported generated exercise, enabled finger numbers take visual precedence.

## Core specification

An exercise is defined by:

- tonic pitch class C–B;
- major, natural-minor, harmonic-minor or melodic-minor scale form;
- scale, arpeggio or primary-chord pattern;
- ascending, descending or up-and-down direction;
- right, left or both hands;
- one to three octaves;
- one, two, four or eight complete repetitions;
- 20–240 BPM.

The initial register is C4–B4 for the right hand and C2–B2 for the left. Every
generated note must fit the configured keyboard range. Invalid tonic, span,
tempo or range combinations fail before playback.

Scale exercises support major plus all three classical minor forms. Natural
minor is unchanged in either direction. Harmonic minor retains its raised
seventh in either direction. Melodic minor raises scale degrees six and seven
while ascending and uses natural minor while descending; an up-and-down phrase
changes form after the single apex. The selector names the form explicitly so
the learner is never shown an ambiguous generic “Minor” label.

Minor form is a scale-only dimension. Arpeggios continue to use the tonic
major/minor triad. Primary-chord exercises use I–IV–V–I in major and
i–iv–V–i in minor, intentionally raising the minor leading tone in the
functional dominant. Moving from a scale to either of those patterns resets the
minor form to Natural rather than implying a nonexistent “melodic-minor
arpeggio” mode.

Both hands move in parallel two octaves apart. Up-and-down exercises play the
apex once. Melodic moments last one beat; chord moments last two beats.
Repetition duplicates the complete phrase, including its return to the tonic,
so each pass has the same musical boundary and scoring shape.

For sessions with at least two passes, completion feedback groups every
judgement by the exact phrase boundary. It shows pass-by-pass accuracy and, when
available, the first-to-last robust timing spread. The description distinguishes
steady, improving, declining and mixed evidence. A decline is described as
later-pass accuracy loss; the software does not claim fatigue or another cause
that MIDI evidence cannot establish.

## Integration boundary

`neothesia_core::exercise::ExercisePlan` is deterministic and independent of
rendering. The next layers are:

1. ~~convert a plan to an in-memory Type-1 MIDI with named right/left tracks;~~
2. ~~identify generated exercises separately from file-backed repertoire;~~
3. ~~add a focused preset editor and preview to the home screen;~~
4. ~~pass the generated `Song` into the existing player;~~
5. ~~save exercise progress by normalized specification identity;~~
6. ~~add a reviewed fingering data/rendering boundary with C-scale coverage;~~
7. expand only through reviewed per-key/per-hand tables.

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

Exercise history is keyed by the musical target: tonic, tonality/minor form,
pattern, direction and octave span. Tempo and hand scope are attempt dimensions
rather than separate songs. Repetition count is also an attempt dimension.
Moving the same scale from 60 to 80 BPM, progressing from separate hands to
both hands, or collecting a longer four-pass sample therefore keeps one
continuous history. Different keys, patterns and minor scale forms never share
an identity.

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

Harmonic- and melodic-minor fingering remains intentionally unavailable outside
the reviewed C-minor family. Generic one-pattern-fits-all fingering would teach
incorrect crossings in several keys.
