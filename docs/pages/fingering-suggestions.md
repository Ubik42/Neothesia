# Explainable fingering suggestions

Neothesia can propose one finger or one complete same-onset chord shape while
the manual finger editor is active. Suggestions are previews, never automatic
edits:

1. Open **Edit fingers** for an imported MIDI.
2. Select a note with `Left` / `Right`.
3. Press `G` to preview a suggestion.
4. Read the finger, confidence and reason.
5. Press `Enter` to accept, choose `1`–`5` yourself, or move on. On a chord,
   Enter accepts the complete visible shape in one save.

Only acceptance writes the content-bound sidecar.

The selected unassigned note carries a cyan dot directly on the waterfall.
After `G`, that dot becomes the cyan preview finger. On a chord, every proposed
digit appears cyan together. Accepting writes the whole group atomically;
unselected saved numbers return to white while the current selection stays
cyan. Reviewed hand-turn landings remain gold. Cyan, white and gold therefore
mean selection/preview, saved guidance and technical turn respectively.

## Why a cost model

Automatic fingering is a combinatorial problem with multiple valid answers.
The prototype follows the established approach of representing successive
finger choices as a path and using dynamic programming to find the lowest-cost
path. The model is deliberately small enough that every selected transition
can be classified and explained.

The research basis:

- Al Kasimi, Nichols and Raphael describe dynamic programming over a
  user-adjustable fingering cost function:
  <https://ismir2007.ismir.net/posters/ISMIR2007_p355_kasimi_poster.pdf>
- Nakamura and colleagues emphasize both the usefulness of constraint/cost
  methods and the real individual variation between pianists:
  <https://doi.org/10.1016/j.ins.2019.12.068>
- Baylor Piano Basics teaches the keyboard-shape rule used by the static key
  cost: fingers 2–3 cover two-black-key groups, 2–3–4 cover three-black-key
  groups, and thumbs normally take white keys:
  <https://openbooks.library.baylor.edu/pianobasics/chapter/d-flat-and-g-flat-major-scales/>

These references justify an explainable prototype, not a claim that its output
is an editorial or teacher-approved fingering.

## Current cost terms

The dynamic program evaluates all five fingers at every modeled note and
penalizes:

- changing finger on an immediately repeated pitch;
- moving against the natural finger order without a thumb/finger crossing;
- using the thumb, and to a lesser extent finger 5, on a black key;
- stretching farther than a conservative finger-pair span;
- crossing across a large interval;
- keeping one finger while changing pitch.

It rewards:

- stable fingers on repeated notes;
- finger order that follows the melodic direction for the selected hand;
- ordinary in-position movement;
- thumb-under and finger-over turns when continuation needs them;
- previously saved manual hints, which act as fixed contextual anchors.

The currently selected note is intentionally unanchored so the learner can ask
for a genuine alternative.

## Chord shapes

Two- through five-note chords on one hand track receive a vertical shape
suggestion. The model:

- sorts the simultaneous notes by pitch without depending on MIDI event order;
- enumerates every unique, hand-ordered subset of fingers 1–5;
- rejects assignments that contradict a saved manual anchor;
- compares pitch spacing with the physical spacing of the candidate fingers;
- applies the selected hand-span profile and black-key costs;
- returns the lowest-cost legal shape to all notes in the chord.

For example, a close-position C–E–G triad produces right-hand 1–3–5 and
left-hand 5–3–1. This is a transparent geometric starting point, not an
editorial claim for every inversion, voicing or musical phrase.

Six-note clusters, duplicate pitches inside one track/onset and contradictory
anchors return no suggestion. A chord wider than the selected profile still
shows the obvious ordered outer-finger shape at low confidence, with the
explicit warning “do not force the reach.” The learner can roll, redistribute
or omit the chord instead.

The editor keeps the whole shape pending under one preview transaction. `Enter`
updates all exact-note hints in memory and performs one atomic sidecar
replacement, so a failed write cannot leave half a chord saved. Direct `1`–`5`
input remains a single-note override.

## Hand-span personalization

Open **Settings → Practice → Hand Span** to choose:

- **Compact · up to a 7th** for smaller hands or learners who should reposition
  instead of being encouraged into broad stretches;
- **Standard · up to an octave**, the backward-compatible default;
- **Large · up to a 9th** for pianists who can comfortably cover wider shapes.

The setting is persistent and changes the comfortable distance assigned to
each finger pair. Notes beyond that distance cost progressively more, so the
lowest-cost path can choose a position shift or different finger pattern. It
does not prohibit a large interval: melodic leaps can still require a shift,
and the preview remains advice rather than an anatomical safety assessment.
Old settings files default to Standard.

## Reasons and confidence

Every preview reports one of:

- saved manual anchor;
- balanced phrase start;
- repeated note;
- in-position movement;
- thumb-under;
- finger-over;
- position shift after a leap.

Confidence is a communication tier tied to the transition type, not a
statistical probability. Manual anchors report 100%; repeated and in-position
choices are high; crossings are medium; phrase starts and large shifts are
lower because hand size and musical context matter more.

## Honest boundary

Version 1 works on tracks already classified as left or right hand. Melodic
runs use a sequential dynamic program; simultaneous two- through five-note
groups use an independent vertical hand-shape model. Tracks with ambiguous
hand ownership receive no suggestion.

Future work can add:

- separate left/right-hand profiles and finer anatomy calibration;
- chord-to-chord voice leading and held-note substitutions;
- phrase/slur and articulation context;
- comparison against expert-annotated datasets;
- alternative suggestions instead of only the lowest-cost path.
