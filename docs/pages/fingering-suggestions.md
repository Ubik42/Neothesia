# Explainable fingering suggestions

Neothesia can propose one finger while the manual finger editor is active.
Suggestions are previews, never automatic edits:

1. Open **Edit fingers** for an imported MIDI.
2. Select a note with `Left` / `Right`.
3. Press `G` to preview a suggestion.
4. Read the finger, confidence and reason.
5. Press `Enter` to accept, choose `1`–`5` yourself, or move on.

Only acceptance writes the content-bound sidecar.

The selected unassigned note carries a cyan dot directly on the waterfall.
After `G`, that dot becomes the cyan preview finger. Accepting and advancing
returns saved numbers to white; reviewed hand-turn landings remain gold. Cyan,
white and gold therefore mean selection/preview, saved guidance and technical
turn respectively.

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

Version 1 suggests only monophonic notes on tracks already classified as left
or right hand. Simultaneous chord notes receive no suggestion, because a chord
requires a vertical hand-shape model rather than pretending its low-to-high
notes are a melody. Tracks with ambiguous hand ownership also receive no
suggestion.

Future work can add:

- configurable hand span and anatomy profiles;
- chord and held-note state;
- phrase/slur and articulation context;
- comparison against expert-annotated datasets;
- alternative suggestions instead of only the lowest-cost path.
