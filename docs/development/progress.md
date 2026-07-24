# Development progress

Updated: 2026-07-25

## Current milestone

**M1 — Practice intelligence**

Overall state: **IN PROGRESS**

## Active cycle

### Cycle 080 — Score-note to MIDI-note alignment

State: **DONE**

Delivered:

- aligned pitched score events to concrete MIDI `track_id` / `note_index`
  identities while preserving stable score IDs;
- matched each pitch as an order-preserving sequence so MusicXML chord order
  cannot disturb simultaneous notes and gaps do not shift later repeats;
- used dynamic programming with explicit score/MIDI skip costs and refused
  matches outside a 250 ms onset window;
- reported unmatched notes on both sides instead of forcing false equivalence;
- derived confidence from onset error, duration error and exact/inexact PPQ
  projection;
- excluded General MIDI drum channel 10 from piano candidates;
- bounded each pitch matrix to one million cells and supplied a deterministic
  linear fallback for unusually large inputs;
- completed `MUS-001C3` and the core `MUS-001C` alignment milestone.

Verification:

- an extra MIDI note between valid notes remains unmatched without consuming a
  later score note;
- a missing middle occurrence of a repeated pitch does not shift the final
  occurrence;
- equal pitches separated by 400 ms remain explicit gaps on both sides;
- 127 core, 64 application and four MIDI-file tests pass;
- Clippy, release build, both native smokes, corpus audit, formatting and diff
  checks pass with only pre-existing warnings;
- implementation commit: `e2e4905`.

Next:

- model repeat/ending playback semantics before claiming complete alignment for
  non-linear scores;
- add a user-facing compatibility/diagnostic surface before consuming matches
  in synchronized notation.

### Cycle 079 — Score-to-MIDI time projection

State: **DONE**

Delivered:

- projected rational quarter-note positions to the paired MIDI PPQ grid;
- passed projected pulses through the paired MIDI's complete tempo map;
- calculated note durations from separately projected start/end positions so
  notes crossing tempo changes remain correct;
- marked exact-pulse and nearest-pulse projections explicitly;
- retained unprojectable negative events as identified gaps;
- sorted projected events deterministically by playback time and stable ID;
- completed `MUS-001C2`.

Verification:

- 1/3 quarter at PPQ 480 maps exactly to pulse 160 and 166,666 µs;
- 1/7 quarter rounds to pulse 69 and is flagged inexact;
- two quarter notes across a 120→60 BPM change map to 1.5 seconds;
- projected note duration and direction offset preserve their stable IDs;
- a negative direction offset is returned as unprojected;
- 124 core, 64 application, four MIDI-file tests pass;
- Clippy, release build, both native smokes, corpus audit, formatting and diff
  checks pass with only pre-existing warnings;
- implementation commit: `b1fa679`.

Next:

- `MUS-001C3`: align pitched score notes to concrete MIDI track/note IDs;
- report confidence and unmatched notes instead of forcing an apparently
  complete mapping.

### Cycle 078 — Stable score event identities

State: **DONE**

Delivered:

- added one shared identity type for semantic notes and directions;
- based identity on the MusicXML part ID, document-order measure ordinal,
  event kind and kind-specific ordinal;
- kept displayed measure numbers out of identity because pickups, repeats and
  exporter conventions may duplicate them;
- reset note and direction ordinals independently per measure;
- completed `MUS-001C1`.

Verification:

- repeated imports of identical source produce an equal score including IDs;
- two measures both displayed as “1” receive distinct measure ordinals;
- note ordinals remain stable when a direction precedes the note;
- a chord and later second voice receive deterministic note IDs;
- the eight-score compatibility corpus remains green;
- 121 core, 64 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, corpus audit, formatting and diff
  checks pass with only pre-existing warnings;
- implementation commit: `e418ed9`.

Next:

- `MUS-001C2`: exact score-time projection through a paired MIDI tempo map;
- `MUS-001C3`: confidence-bearing score-note to MIDI-note alignment with
  explicit gaps rather than forced matches.

### Cycle 077 — MusicXML pedal semantics

State: **DONE**

Delivered:

- added pedal marks to semantic score directions at exact score positions;
- modeled damper start/stop/change and sostenuto plus line
  continue/discontinue/resume;
- retained overlapping pedal numbers and line, sign and abbreviation engraving
  preferences;
- preserved unknown future pedal types instead of failing an otherwise usable
  score;
- exposed pedal-mark counts in the compatibility inspector;
- completed `MUS-001F`.

Verification:

- a focused fixture preserves start and change marks, number 2, line/sign and
  abbreviation choices;
- the MuseScore 3.6 pedal fixture recognizes all seven marks;
- all six prior pedal warnings disappear;
- 120 core, 64 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, corpus audit, formatting and diff
  checks pass with only pre-existing warnings;
- implementation commit: `bcfda27`.

Next:

- give score events stable identities and align them with MIDI events so pedal
  marks can become CC64 practice targets;
- preserve wedge spans for dynamics coaching after the same identity boundary
  exists.

### Cycle 076 — MusicXML tuplet semantics

State: **DONE**

Delivered:

- added explicit tuplet ratios with actual/normal note counts, normal note type
  and normal dots;
- added numbered tuplet start/stop spans with bracket and displayed-number
  preferences;
- retained exact rational event duration independently of engraving metadata;
- rejected incomplete/zero ratios with a localized warning instead of creating
  invalid semantic data;
- exposed recognized tuplet-note counts in the compatibility inspector;
- completed `MUS-001E`.

Verification:

- a 3:2 eighth-note fixture retains a 1/3-quarter duration, ratio and bracket
  span;
- the Clementi Andante corpus score recognizes 267 tuplet notes;
- the broad MuseScore 2.3 fixture recognizes 42 tuplet notes;
- all previous tuplet warnings disappear while three genuine ornament warnings
  remain in Clementi;
- 120 core, 64 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, corpus audit, formatting and diff
  checks pass with only pre-existing warnings;
- implementation commit: `1f67fe9`.

Next:

- model MusicXML pedal directions, because the dedicated real-score fixture now
  exposes six affected measures;
- connect slur, articulation and tuplet semantics to phrase-aware fingering
  only after stable score/MIDI note identities exist.

### Cycle 075 — Real-score MusicXML compatibility audit

State: **DONE**

Delivered:

- added a compact file inspector for parts, measures, notes, directions and
  warning summaries;
- added a reproducible compatibility runner that downloads eight BSD-licensed
  OSMD fixtures at a pinned revision and verifies every SHA-256 hash;
- exercised MuseScore 1.2, 2.3 and 3.6 export shapes, an older uncompressed
  form, a real MXL, voice alignment, grace notes, pedal directions and a broad
  notation feature score;
- fixed older exporters' explicit `<rest></rest>` form;
- made empty deferred elements such as `<pedal/>` report warnings;
- distinguished exact preserved tuplet timing from still-missing tuplet
  notation instead of reporting both as lost;
- completed `MUS-001B3A`; retained true cross-exporter coverage as
  `MUS-001B3B`.

Verification:

- all eight pinned files import successfully;
- the corpus covers 261 part-measures and 2,183 note/rest events;
- Bach BWV 846 imports 35 measures and 750 events without a warning;
- unsupported ornaments, tuplet notation and pedal directions remain visible
  and summarized rather than silently discarded;
- 119 core, 64 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `bfa1e05`.

Next:

- acquire explicitly licensed current MuseScore, Dorico and Finale piano
  exports for `MUS-001B3B`;
- then implement the highest-frequency semantic gap observed across exporters,
  currently tuplet notation and pedal directions.

### Cycle 074 — Safe compressed MusicXML input

State: **DONE**

Delivered:

- added file- and memory-level entry points that detect MXL by ZIP signature
  instead of trusting an extension;
- followed the standard `META-INF/container.xml` root-file declaration and
  preferred the complete MusicXML media type;
- supported stored and deflated MXL entries;
- bounded archive bytes, entry count, container bytes and decompressed score
  bytes;
- rejected absolute, parent-traversing and Windows-separator root paths before
  any score lookup;
- accepted a missing/invalid mimetype marker for compatibility while recording
  a visible conformance warning;
- completed `MUS-001B1` and retained timewise/corpus work separately.

Verification:

- valid deflated MXL imports the same annotated piano score as raw MusicXML;
- a missing mimetype produces a warning without discarding the score;
- path traversal and missing-container archives fail with typed errors;
- 117 core, 64 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `49843b7`.

Next:

- `MUS-001B3`: build the cross-exporter fixture corpus before extending more
  schema coverage;
- use corpus failures to decide whether `MUS-001B2` timewise conversion has
  practical priority.

### Cycle 073 — MusicXML semantic foundation

State: **DONE**

Delivered:

- separated notation semantics from both MusicXML document structure and any
  future engraving engine;
- added exact rational score time so tuplets and changing divisions do not
  accumulate floating-point drift;
- imported partwise score metadata, parts, measures, voices, staves, pitches,
  rests, chords, directions, dynamics, tempo, fingering, ties, slurs and
  articulations;
- modeled grand-staff attributes including divisions, key, meter and clefs;
- handled multi-voice `backup`/`forward` timing and emitted explicit warnings
  for recognized but deferred notation;
- compared native engraving, Verovio and MuseScore boundaries and selected an
  isolated Verovio SVG spike as the next renderer experiment;
- completed `MUS-001A` and `MUS-R01A`.

Verification:

- a two-staff, two-voice piano fixture retains simultaneous chord notes,
  rewinds the second voice correctly and preserves learning annotations;
- timewise input is rejected explicitly instead of being misread;
- XML entities and typed composer metadata survive import;
- 113 core, 64 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `7448f4b`.

Next:

- `MUS-001B`: compressed MXL and a real-world piano fixture corpus;
- `MUS-R01B`: an isolated Verovio measurement spike before any UI dependency
  is accepted.

### Cycle 072 — Rapid repeated-note alternation

State: **DONE**

Delivered:

- distinguished a sustained rapid repeated-note run from a slow/isolated
  repeat;
- required at least three same-pitch notes with onset gaps no greater than
  250 ms;
- preferred adjacent finger changes while retaining all manual anchors;
- kept the previous same-finger rule for slow repetition;
- added an explicit rapid-repeat reason at a moderate 76% confidence;
- documented university textbook/performance-study evidence and exceptions;
- completed `MUS-003I`.

Verification:

- four C notes at 120 ms intervals produce 3–2–1–2;
- every transition in that run reports the rapid-repeat reason;
- the existing 500 ms repeat keeps finger 3 and reports ordinary repetition;
- 110 core, 66 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `b6a2332`.

### Cycle 071 — Melody over a held chord

State: **DONE**

Delivered:

- carried the previous chord's still-sounding finger occupancy into immediately
  following single-note onsets;
- continued that context across the complete melody tail until the chord's
  final held note ends;
- restored ordinary melodic transition cost between tail notes so “free
  finger” selection does not repeat one finger unnaturally;
- enforced overlap constraints if a tail note itself is still down at the next
  onset;
- added retry-safe read-only semantic snapshots without replaying actions;
- completed `MUS-003F2A2`.

Verification:

- held low C prevents both following right-hand A and B from reusing/crossing
  its finger;
- the ascending A–B tail advances fingers instead of repeating one free finger;
- both tail notes retain the held-position explanation;
- 109 core, 66 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `eb0abdd`.

### Cycle 070 — Held-tone finger occupancy

State: **DONE**

Delivered:

- added note-off time to the fingering-domain input and wired it from parsed
  MIDI notes;
- kept a finger unavailable while its earlier chord tone is still sounding;
- rejected new assignments that cross a held note/finger pair in either hand;
- segmented around an onset with no valid held-tone transition rather than
  manufacturing an impossible shape;
- added the `HeldChordPosition` explanation at an 82% communication tier;
- completed `MUS-003F2A`; retained explicit substitution as `MUS-003F2B`.

Verification:

- with low C held, a later right-hand A–B dyad neither reuses C's finger nor
  crosses to its left;
- the later notes report the held-position explanation;
- left-hand mirrored ordering accepts 5–3 below a held high note and rejects
  reusing its finger;
- all previous melodic, chord, voice-leading and refusal tests remain green;
- 108 core, 66 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `13fa40f`.

### Cycle 069 — Common-tone chord voice leading

State: **DONE**

Delivered:

- grouped consecutive valid chord onsets into a candidate-state sequence;
- used dynamic programming across all legal vertical shapes rather than a
  greedy previous-chord choice;
- preferred common tones to keep the same finger and reduced unnecessary
  movement for fingers used in both chords;
- added an explainable `ChordConnection` reason at an 84% communication tier;
- segmented around unsupported/contradictory chords so one invalid group does
  not suppress later valid suggestions;
- calibrated the transition as a light tie-breaker after rejecting an
  over-strong version that produced cramped shapes;
- completed `MUS-003F1`; retained held-note and phrase work as F2/F3.

Verification:

- C–E–G → C–E–A keeps C on 1 and E on 3;
- E–G–C and D–G–B independently disagree on the G finger, while joint planning
  chooses ergonomic 1–3–5 for both and keeps G on 3;
- existing scale turns, isolated chords, anchors, wide-span warnings and
  refusal boundaries remain green;
- 107 core, 66 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `6c53e86`.

### Cycle 068 — Independent right/left-hand spans

State: **DONE**

Delivered:

- split the Practice settings control into **Right Hand Span** and **Left Hand
  Span**;
- routed each suggestion through the profile matching its classified hand;
- retained the old shared value as a migration fallback rather than silently
  resetting existing Compact/Large users;
- stored only explicit per-hand overrides above that fallback;
- kept a shared setter as a compatibility/reset boundary;
- completed `MUS-003D3`.

Verification:

- a legacy Compact value resolves to Compact for both hands;
- overriding only the right hand to Large leaves the left hand Compact;
- the asymmetric result survives RON serialization and rebuilding;
- an explicit legacy-RON fixture preserves Compact with no fabricated
  overrides;
- 105 core, 66 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `1cc9c37`.

### Cycle 067 — Whole-chord preview and atomic acceptance

State: **DONE**

Delivered:

- changed the pending suggestion from one selected digit to a complete set of
  exact-note assignments;
- rendered every pending chord digit cyan while keeping the selected note
  identifiable;
- displayed the low-to-high shape (for example 1–3–5) and explicit
  “accepts all 3” copy;
- accepted all chord hints through one sidecar save instead of three partial
  writes;
- advanced past the accepted group while preserving single-note 1–5 editing;
- exposed the pending assignment count through semantic automation;
- upgraded the native fixture to prove three previews become three live and
  persisted hints;
- completed `MUS-003G`.

Verification:

- the native snapshot reports selected finger 1, pending count 3 and 78%
  confidence before acceptance;
- after Enter, both live fingering count and manual sidecar count equal 3;
- the saved sidecar contains exactly three track IDs, note indices and valid
  finger values;
- 104 core, 66 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `2db5a0e`.

### Cycle 066 — Safe polyphonic chord fingering

State: **DONE**

Delivered:

- replaced the old blanket chord refusal with a vertical hand-shape model for
  two through five distinct simultaneous notes;
- enumerated only unique fingers in anatomically ordered right/left-hand
  shapes;
- scored pitch-to-finger proportionality, profile-specific reach and black-key
  placement;
- treated manual chord hints as hard anchors;
- returned no suggestion for six-note groups, duplicate pitches or conflicting
  anchors;
- added ordinary chord and low-confidence wide-chord explanations;
- upgraded the native MIDI fixture to select a real same-track C-major chord,
  preview it and persist only the accepted note;
- completed `MUS-003D` / `MUS-003D2`;
- retained chord progression and held-note context as `MUS-003F`.

Verification:

- right-hand C–E–G produces 1–3–5 and left hand produces 5–3–1;
- anchor compatibility and conflict refusal are covered;
- oversized and duplicate-pitch chord refusal is covered;
- Compact profile marks an octave dyad as a low-confidence wide shape;
- the native FingeringFixture previews right-hand finger 1 at 78% and persists
  exactly one accepted hint;
- 104 core, 66 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `ee3ad43`.

### Cycle 065 — Personalized hand-span profiles

State: **DONE**

Delivered:

- added Compact, Standard and Large hand-span profiles to the explainable
  fingering domain;
- changed finger-pair comfortable distances inside the dynamic-programming
  transition cost instead of merely relabeling output;
- exposed a persistent **Settings → Practice → Hand Span** selector;
- kept Standard as the backward-compatible default for old settings;
- made live G-key previews use the selected profile;
- split personalized melody work from the still-open polyphonic chord model;
- hardened the native exercise smoke against Windows PowerShell JSON integer
  type differences while retaining failing-pitch diagnostics;
- completed `MUS-003D1`; retained parent `MUS-003D` and `MUS-003D2`.

Verification:

- a domain regression proves Compact and Large can select different plans for
  wide phrases;
- configuration tests cover default migration and persistent mutation;
- the FingeringFixture still previews finger 3 at 65% and saves one hint;
- the two-hand G-sharp primary-chord fixture exposes pitches 44 and 68, matches
  all six required notes and completes both passes;
- 101 core, 66 application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `581cff8`.

### Cycle 064 — Visual fingering selection and preview

State: **DONE**

Delivered:

- rendered a cyan dot on the exact selected note before it has a finger;
- replaced the dot with the cyan suggested digit after preview;
- let an accepted digit return to normal saved-guidance white after advancing;
- retained gold exclusively for reviewed thumb-under/finger-over landings;
- cleared transient selection state when Finger edit closes;
- completed `MUS-003E`.

Verification:

- glyph precedence proves preview overrides a saved digit only while selected;
- unassigned selection produces a marker, ordinary unselected notes do not;
- stored hints remain ordinary digits outside selection;
- real-process preview/accept and sidecar persistence continue to pass;
- ninety-nine core, sixty-six application and two MIDI-file tests pass;
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings;
- implementation commit: `de7a2f0`.

### Cycle 063 — Explainable fingering suggestion prototype

State: **DONE**

Delivered:

- added a deterministic five-finger dynamic program for left/right-hand
  monophonic passages;
- modeled repeated notes, natural finger order, conservative span, black-key
  thumb cost, crossings and large position shifts;
- treated existing manual hints as hard contextual anchors while reevaluating
  the selected note;
- attached a plain-language reason and bounded confidence tier to every
  suggestion;
- refused chord and ambiguous-hand suggestions instead of guessing;
- added `G` preview and explicit Enter acceptance to Finger edit;
- exposed pending suggestion/confidence to semantic automation;
- documented the research basis, cost terms and honest boundary;
- completed `MUS-003A` through `MUS-003C` and prototype parent `MUS-003`;
  retained `MUS-003D` for personalized/polyphonic expansion.

Verification:

- five-note positions follow opposite natural ordering in the two hands;
- a C-major octave produces the classical right-hand 1–2–3–1–2–3–4–5 turn;
- repeats retain a finger and manual anchors remain fixed;
- black-key tests avoid unnecessary thumb use;
- chords produce no false suggestion;
- real-process automation previews finger 3 at 65% before separately accepting
  and persisting it;
- ninety-eight core, sixty-six application and two MIDI-file tests pass;
- Clippy, release build, manual-fingering and Technique Studio native smokes,
  formatting and diff checks pass with only pre-existing warnings;
- implementation commit: `8a85c3c`.

### Cycle 062 — Portable manual finger hints

State: **DONE**

Delivered:

- extended content-bound song sidecars with exact `track_id + note_index`
  finger hints;
- accepted only piano fingers 1 through 5 and preserved the previous sidecar
  after invalid edits;
- made metadata and finger edits preserve each other;
- mapped imported hints to exact start/pitch/channel/track waterfall notes;
- generalized the reviewed-fingering renderer for imported songs;
- added **Edit fingers** / `Ctrl+I`, paused score-order navigation, immediate
  1–5 assignment, automatic advance and deletion;
- exposed persistent edit state plus hand, measure, pitch and current-finger
  feedback;
- moved output/panic badges below the expanded top bar so the new and existing
  finger controls remain clickable at minimum window width;
- added native automation for edit, live rendering state and sidecar
  persistence;
- completed `MUS-002A` through `MUS-002C` and parent `MUS-002`.

Verification:

- metadata/fingering preservation and invalid-finger rollback pass;
- an imported song reload maps a saved hint to one exact note;
- target ordering, bounded navigation, pitch naming and replace/clear behavior
  are deterministic;
- real-process automation starts with zero hints, pauses in edit mode, assigns
  finger 1, observes one live/manual hint and verifies the adjacent sidecar;
- ninety-two core, sixty-six application and two MIDI-file tests pass;
- Clippy, release build, manual-fingering and Technique Studio native smokes,
  formatting and diff checks pass with only pre-existing warnings;
- implementation commit: `f012ef4`.

### Cycle 061 — Native song metadata editor

State: **DONE**

Delivered:

- added an **Info** action beside every available local MIDI in Practice
  Library;
- added a native seven-field editor for title, performer, composer, collection,
  difficulty, comma-separated tags and study notes;
- supported mouse field selection plus Tab, Shift+Tab, arrows, Backspace,
  Delete, Enter, Escape and Ctrl+S;
- saved through the content-bound atomic sidecar API;
- refreshed title, credit and search immediately after a successful save;
- showed persistence errors in place without discarding the edit;
- completed `LIB-002C` and parent `LIB-002`.

Verification:

- editor conversion trims optional scalar values and parses comma-separated
  tags;
- field navigation clamps safely at both ends;
- core round-trip, identity mismatch and atomic replacement tests continue to
  cover the actual save boundary;
- ninety core, sixty-two application and two MIDI-file tests pass;
- Clippy, release build, native Technique Studio smoke, formatting and diff
  checks pass with only pre-existing warnings;
- implementation commit: `8fc184e`.

### Cycle 060 — Portable song metadata foundation

State: **DONE**

Delivered:

- added a versioned adjacent `.neothesia.ron` sidecar model;
- bound every sidecar to the MIDI's existing BLAKE3 content identity;
- normalized optional title, artist, composer, collection, difficulty, tags
  and study notes;
- wrote sidecars with same-directory atomic replacement;
- loaded and merged metadata deterministically across duplicate MIDI files;
- used metadata title/credit in Practice Library and indexed every field;
- reported loaded and invalid sidecars separately;
- documented the format, identity rules and current editor boundary;
- completed `LIB-002A` and `LIB-002B`; retained `LIB-002C` and parent
  `LIB-002` for the native editing workflow.

Verification:

- round trip preserves normalized metadata and exact content identity;
- mismatched content is ignored instead of relabeling another song;
- duplicate files retain stable scalar precedence while merging tags;
- repeat saves atomically replace the sidecar without orphan temp files;
- library search spans title, performer, composer, collection, difficulty,
  tags, notes and path;
- ninety core, sixty application and two MIDI-file tests pass;
- Clippy, release build, native Technique Studio smoke, formatting and diff
  checks pass with only pre-existing warnings;
- implementation commit: `763030c`.

### Cycle 059 — Deterministic player clock

State: **DONE**

Delivered:

- audited the practice matcher, `MidiPlayer` and MIDI playback timeline for
  hidden wall-clock access;
- confirmed all three advance only through caller-supplied `Duration` values;
- added a focused player regression for explicit advancement, paused clock
  freezing and deterministic resumed advancement;
- completed `QA-010`.

Verification:

- source audit finds no sleep, `Instant::now` or `SystemTime::now` in the
  matcher, player or playback timeline;
- the new test advances to 250 ms, ignores a simulated 30-second pause and
  resumes exactly at 375 ms;
- no test waits for real time or depends on scheduler timing;
- eighty-six core, sixty application and two MIDI-file tests pass;
- Clippy, release build, formatting and diff checks pass with only pre-existing
  warnings;
- implementation commit: `3ad052b`.

### Cycle 058 — Pianoteq route diagnostics

State: **DONE**

Delivered:

- added a console diagnostic that enumerates the same MIDI input/output ports
  used by Neothesia;
- added exact expected-port, saved-selection and non-destructive open probes;
- added a one-command PowerShell route check with optional Pianoteq-process
  enforcement;
- documented what the automated check proves and the physical/audio boundary
  it cannot prove;
- split route readiness from the separate 30-minute physical soak.

Verification:

- two argument-boundary tests pass;
- a visible Keystation output passes the exact-name and open probe;
- a deliberately absent `Neothesia to Pianoteq` endpoint fails with exit code
  1 and a specific remediation message;
- current local discovery finds Pianoteq 6 STAGE installed, but no virtual MIDI
  cable and an internal `Buildin Synth` saved output;
- eighty-six core, fifty-nine application and two MIDI-file tests pass;
- Clippy, release build, native Technique Studio smoke, formatting and diff
  checks pass with only pre-existing warnings;
- `AUD-010A` is complete; parent `AUD-010`, `AUD-010B` and physical `AUD-012`
  remain open until that external route exists.
- implementation commit: `368936f`.

### Cycle 057 — Primary-chord fingerings

State: **DONE**

Delivered:

- added simultaneous right-hand 1–3–5 and left-hand 5–3–1 assignments;
- covered every I–IV–V–I and i–iv–V–i generated chord in all keys;
- kept blocked chords free of false scale-style turn highlights;
- changed native smoke coverage to G♯ minor primary chords;
- completed `EX-003J` and the parent `EX-003`.

Verification:

- up/down C-major progression produces seven correctly fingered triads;
- chord crossings are explicitly all false;
- eighty-six core, fifty-seven application and two MIDI-file tests pass;
- native automation matches six simultaneous two-hand notes and persists the
  G♯ minor primary-chord attempt;
- Clippy, release build, formatting and diff checks pass with only pre-existing
  warnings;
- implementation commit: `81dd7be`.

### Cycle 056 — Hand-turn highlighting

State: **DONE**

Delivered:

- detected thumb-under and finger-over landing notes per hand and direction;
- carried crossing metadata from exercise plans through generated songs;
- rendered hand-turn finger numbers in gold and ordinary numbers in white;
- explained the color in Technique Studio;
- exposed crossing count to native automation;
- completed `EX-003I`.

Verification:

- C-major up/down assertions cover two right- and two left-hand turns;
- generated-song mapping preserves exact timestamp/pitch/channel markers;
- eighty-six core, fifty-seven application and two MIDI-file tests pass;
- native G♯ minor-arpeggio automation verifies at least one rendered turn;
- Clippy, release build, formatting and diff checks pass with only pre-existing
  warnings;
- implementation commit: `667cbc8`.

### Cycle 055 — Reviewed arpeggio fingerings

State: **DONE**

Delivered:

- added right/left guidance for all twelve major and minor triad arpeggios;
- grouped tables by reviewed white/black-key keyboard shapes;
- extended the patterns through octave span, direction and repetition;
- kept primary-chord progressions unavailable pending separate voicing tables;
- changed native smoke coverage to G♯ minor arpeggio;
- completed `EX-003H`.

Verification:

- exact two-octave tables cover all twenty-four tonic/tonality combinations;
- eighty-five core, fifty-seven application and two MIDI-file tests pass;
- native automation completes and persists a G♯ minor-arpeggio attempt;
- Clippy, release build, formatting and diff checks pass with only pre-existing
  warnings;
- implementation commit: `d46a395`.

### Cycle 054 — Direction-aware melodic-minor fingering

State: **DONE**

Delivered:

- completed per-hand melodic-minor coverage for all twelve keys;
- separated ascending and descending fingering sources;
- used melodic-minor tables upward and natural-minor tables downward;
- joined both forms at one apex for up-and-down exercises;
- changed native smoke coverage to G♯ melodic minor;
- completed `EX-003G`.

Verification:

- exact fifteen-note ascending tables cover all twelve melodic minors;
- B♭ verifies distinct ascending-melodic and descending-natural fingering;
- eighty-four core, fifty-seven application and two MIDI-file tests pass;
- native automation completes and persists a G♯ melodic-minor two-pass attempt;
- Clippy, release build, formatting and diff checks pass with only pre-existing
  warnings;
- implementation commit: `fe85447`.

### Cycle 053 — All harmonic-minor fingering tables

State: **DONE**

Delivered:

- completed per-hand fingering coverage for all twelve harmonic-minor keys;
- added a first-octave/later-octave model for changing crossings;
- represented exceptional C♯, F♯, G♯ and B endpoints without approximation;
- kept non-C melodic minor safely unavailable;
- changed native smoke coverage to G♯ harmonic minor;
- completed `EX-003F`.

Verification:

- exact fifteen-note right/left tables cover all twelve harmonic minors;
- eighty-two core, fifty-seven application and two MIDI-file tests pass;
- native automation completes and persists a G♯ harmonic-minor two-pass attempt;
- Clippy, release build, formatting and diff checks pass with only pre-existing
  warnings;
- implementation commit: `a7adbfa`.

### Cycle 052 — All natural-minor fingering tables

State: **DONE**

Delivered:

- completed per-hand fingering coverage for all twelve natural-minor keys;
- added explicit C♯, E♭, F♯, G♯ and B♭ black-root two-octave tables;
- retained reviewed white-root patterns and the existing C-minor family;
- distinguished A♭ major from G♯ minor in selectors and generated titles;
- kept unreviewed non-C harmonic/melodic minor forms safely unavailable;
- changed native smoke coverage from B major to G♯ natural minor;
- completed `EX-003E`.

Verification:

- exact fifteen-note right/left sequences cover every black-root natural minor;
- all twelve natural-minor keys report reviewed guidance;
- eighty-one core, fifty-seven application and two MIDI-file tests pass;
- native automation completes and persists a G♯ natural-minor two-pass attempt;
- normal Clippy and release build pass with only pre-existing warnings;
- strict warning denial remains blocked by those pre-existing warnings;
- implementation commit: `ae8101a`.

### Cycle 051 — Persistent fingering visibility

State: **DONE**

Delivered:

- added a backward-compatible appearance preference for exercise fingerings;
- defaulted old and new settings to visible guidance;
- initialized each generated exercise from the saved preference;
- saved player toggle changes immediately;
- added an Exercise Fingerings row to Settings;
- restored note names when finger guidance is off and note labels are enabled;
- preserved the existing public note-label constructor;
- upgraded smoke persistence diagnostics to name the missing field;
- completed `EX-003D`.

Verification:

- legacy appearance RON defaults fingerings on;
- explicit off state survives settings serialization;
- eighty-one core tests and fifty-seven application tests pass;
- native automation toggles off/on, verifies both snapshots and inspects the
  persisted final preference;
- Clippy and release build report only pre-existing warnings;
- the optional CLI target remains blocked on this machine by its pre-existing
  system FFmpeg/vcpkg requirement, while its original renderer API remains
  source-compatible;
- implementation commit: `0d6c428`.

### Cycle 050 — All major-scale fingering tables

State: **DONE**

Delivered:

- completed per-hand fingering coverage for all twelve major pitch classes;
- added distinct D♭, E♭, G♭, A♭, B♭ and B patterns;
- handled non-thumb starting fingers and intermediate-octave tonic changes;
- kept endpoint-only finger 5 behavior in the common right-hand group;
- preserved explicit no-guidance behavior for unreviewed minor keys;
- used readable flat spellings for flat major keys in selectors and titles;
- changed native smoke to B major's exceptional left hand;
- completed `EX-003C`.

Verification:

- exact two-octave right/left tables for the six newly covered keys;
- exact display-name distinction between D♭ major and C♯ minor;
- non-C minor and non-scale patterns remain unsupported;
- seventy-nine core tests and fifty-seven application tests pass;
- native automation starts B major, verifies both tonic pitches and exercises
  the default-on/two-way fingering control;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `2c3b468`.

### Cycle 049 — Common major-scale fingering group

State: **DONE**

Delivered:

- verified the common C/G/D/A/E group against a university piano text;
- verified F major's distinct right hand against a dedicated reference;
- expanded reviewed guidance from C to six major keys;
- retained standard fingering for both hands in C/G/D/A/E;
- added F right-hand `1234–1234` with the standard left hand;
- extended every table across directions, octaves and repetitions;
- refused to inherit a parallel-major table for unreviewed minor keys;
- added an availability line to the Technique Studio preview;
- completed `EX-003B`.

Verification:

- exact two-octave standard right/left sequences for five major keys;
- exact two-octave F-major right/left sequences;
- G minor explicitly remains unsupported;
- seventy-seven core tests and fifty-seven application tests pass;
- native automation selects F major, verifies guidance defaults on, toggles it
  both ways and completes the two-pass exercise;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `2af19e5`.

### Cycle 048 — Reviewed fingering foundation

State: **DONE**

Delivered:

- added deterministic fingering sequences to generated exercise plans;
- covered C major and all three C minor scale forms;
- handled both hands, every direction, one to three octaves and repetitions;
- attached finger numbers to exact serialized note time/pitch/channel identity;
- rendered 1–5 directly on falling notes;
- centered digits independently for white- and black-key note widths;
- enabled reviewed fingerings by default when available;
- added an obvious `Fingers: ON/OFF` player control;
- preserved regular note-name labels for imported MIDI and Free Play;
- returned no fingering for unreviewed keys and patterns;
- completed `EX-003A`.

Verification:

- exact two-octave right/left up-and-down sequences repeated twice;
- unsupported key and arpeggio return no guidance;
- generated Song maps first notes and apex to the expected hand/channel finger;
- semantic action IDs remain unique and explicitly supported;
- native automation verifies availability, default-on state and two-way toggle;
- two MIDI-file tests, seventy-five core tests and fifty-seven application
  tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `fd1f446`.

### Cycle 047 — Reusable exercise presets

State: **DONE**

Delivered:

- added versioned recent and favourite exercise collections;
- retained eight deduplicated recent variants in newest-first order;
- retained twelve explicitly saved favourite variants;
- restored all eight exercise parameters from either selector;
- added visible Recent/Favourites cards and a stateful star action;
- persisted favourites immediately and recent variants on start;
- filtered invalid and duplicate records when loading;
- kept older settings backward compatible;
- added semantic actions for native UI automation;
- completed `EX-002`.

Verification:

- recent ordering, deduplication, capacity and invalid-record filtering tests;
- favourite add/remove and settings round-trip test;
- full-spec preset selection and wraparound test;
- fifty-seven application tests and seventy-three core tests pass;
- native automation saves, cycles and restores recent/favourite variants,
  restarts the exercise, then reopens it from Practice Library;
- settings inspection confirms both collections and the selected C♯ 70 BPM
  two-pass specification;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `bbb8adb`.

### Cycle 046 — Complete minor scale forms

State: **DONE**

Delivered:

- split scale form from the broader major/minor harmony model;
- added explicitly named Natural, Harmonic and Melodic minor choices;
- generated classical melodic minor with raised sixth/seventh ascending and
  natural minor descending;
- retained the raised seventh in both directions for harmonic minor;
- kept the apex single in every up-and-down scale;
- rejected minor scale forms on major, arpeggio and chord specifications;
- reset scale-only form state when selecting another pattern;
- separated practice history identities for each valid minor scale form;
- defaulted older saved exercises to Natural minor;
- hardened the real-process smoke runner against first-frame action timing;
- completed `EX-001H`.

Verification:

- exact A melodic-minor up/down pitch sequence;
- exact descending A harmonic-minor pitch sequence;
- invalid major/form and arpeggio/form combinations rejected;
- legacy exercise deserialization defaults to Natural;
- selector cycles in both directions and pattern changes reset form safely;
- native automation completes, persists and reopens a two-pass generated
  exercise;
- two MIDI-file tests, seventy-one practice/core tests and fifty-six
  application tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `756fc40`.

### Cycle 045 — Pass-by-pass exercise consistency

State: **DONE**

Delivered:

- retained exact generated phrase duration on exercise songs;
- matched phrase duration to serialized MIDI microsecond timing;
- grouped scored results into one-based exercise passes;
- summarized accuracy and robust timing evidence per pass;
- persisted pass summaries with backward-compatible defaults;
- showed pass sequences in Technique completion feedback;
- classified improving, steady, declining and mixed evidence;
- avoided unsupported claims about fatigue or physical cause;
- completed `EX-001G`.

Verification:

- exact one-second boundaries assign targets to the correct pass;
- 50% then 100% produces two distinct pass summaries;
- one-pass sessions emit no redundant pass evidence;
- 70 BPM generated boundary exactly matches the first note of pass two;
- completion copy reports 80% → 100% and timing spread 24→12 ms;
- legacy attempt summaries load with no pass evidence;
- native automation completes two C♯ 70 BPM passes and inspects persisted pass
  two;
- two MIDI-file tests, sixty-nine practice/core tests and fifty-six
  application tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `e661362`.

### Cycle 044 — Multi-pass exercise sessions

State: **DONE**

Delivered:

- added repetition count to the serializable exercise specification;
- offered one, two, four and eight complete passes in Technique Studio;
- repeated whole phrases without changing their internal note order;
- included repeat count in generated names while keeping one-pass names clean;
- treated repeat count as an attempt dimension, not a new learning target;
- validated a safe one-to-eight core range;
- defaulted older saved specifications to one repetition;
- added stable semantic actions for both repetition arrows;
- completed `EX-001F`.

Verification:

- four-pass arpeggio has exactly four times the one-pass moments;
- every repeated phrase begins with the exact original sequence;
- one- and four-pass variants share practice identity;
- zero and nine repetitions are rejected;
- legacy specifications deserialize with one repetition;
- UI selectors wrap 1 → 2 → 4 → 8 in both directions;
- native automation exercises both arrows and confirms one pass is persisted;
- two MIDI-file tests, sixty-seven practice/core tests and fifty-four
  application tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `53cb2f8`.

### Cycle 043 — Reopen generated exercises

State: **DONE**

Delivered:

- added backward-compatible generated-source metadata to saved song setups;
- surfaced generated specifications in recent Practice Library summaries;
- treated generated exercises as available without a filesystem path;
- labeled generated rows with the direct `Practice` action;
- rebuilt exercises synchronously from their saved specification;
- restored saved tracks, hand mode, speed and loop state before playback;
- retained Open/Locate behavior for file-backed MIDI;
- added semantic navigation and reopen actions for native automation;
- completed `EX-001E`.

Verification:

- generated source metadata survives disk round trip;
- older setup records without source metadata remain readable;
- recent-library summaries expose the generated spec and no fake path;
- native automation completes and saves C♯ 70 BPM;
- it returns to Practice Library and reopens the exercise without a picker;
- the reopened player restores the latest Right-hand setup;
- two MIDI-file tests, sixty-five practice/core tests and fifty-four
  application tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `ed0f789`.

### Cycle 042 — Continuous exercise tempo progression

State: **DONE**

Delivered:

- separated raw generated-MIDI identity from musical practice identity;
- normalized exercise history by key, tonality, pattern, direction and span;
- kept tempo and hands as attempt dimensions under the same exercise;
- ensured different musical targets remain isolated;
- persisted effective BPM for every completed generated-exercise attempt;
- preferred real BPM in recent attempts and tempo trends;
- retained multiplier-only display for imported MIDI and legacy sessions;
- extended the real-process fixture through full exercise completion;
- completed `EX-001D`.

Verification:

- 60 and 120 BPM variants share practice identity;
- right, left and both-hand variants share practice identity;
- different keys produce different identities;
- effective tempo incorporates the player multiplier;
- overview reports a 60 → 84 BPM change as +24 BPM;
- sessions saved before effective BPM remain readable;
- native automation completes C♯ at 70 BPM and inspects persisted BPM evidence;
- two MIDI-file tests, sixty-three practice/core tests and fifty-four
  application tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `041062e`.

### Cycle 041 — Persistent exercise choices

State: **DONE**

Delivered:

- replaced one-way parameter cycling with explicit previous/next controls;
- covered key, tonality, pattern, direction, hands, octaves and tempo;
- gave every selector control a stable semantic automation identity;
- persisted the complete last-started exercise specification;
- restored that specification when the application starts again;
- kept older settings files compatible through a defaulted field;
- rejected invalid persisted tonic, span or tempo before rendering;
- shared one adjustment path between visible controls and automation;
- completed `EX-001C`.

Verification:

- selectors wrap safely in both directions;
- the full exercise specification survives RON serialization;
- settings without the new field load the default exercise;
- invalid persisted exercise values fall back safely;
- real-process automation changes C to C♯ and observes MIDI notes 37 and 61;
- the process writes the selected C♯ specification to `settings.ron`;
- two MIDI-file tests, sixty practice/core tests and fifty-two application
  tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `e613901`.

### Cycle 040 — Technique Studio

State: **DONE**

Delivered:

- added a prominent Technique Studio entry to the native home screen;
- added a focused two-column exercise editor for key, tonality, pattern,
  direction, hands, range and tempo;
- previewed the complete generated exercise name before starting;
- launched generated material through the normal practice player;
- retained wait-for-notes, scoring, hand switching, loop practice, tempo,
  output routing and completion feedback;
- assigned explicit hand identity to generated single-hand tracks;
- added semantic automation actions for opening and starting an exercise;
- extended the real-process smoke runner with a generated-exercise fixture;
- completed the full `EX-001` exercise-mode backlog item.

Verification:

- option cycling remains inside valid enum and tempo states;
- both single-hand variants retain their requested practice part;
- the real process opens Technique Studio and starts a generated exercise;
- the generated exercise defaults to wait mode and Both hands;
- injected C notes score, the hand control reaches Right, loop/restart works
  and the process exits cleanly;
- two MIDI-file tests, fifty-eight practice/core tests and fifty-two
  application tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `017697d`.

### Cycle 039 — Exercise plans as playable MIDI

State: **DONE**

Delivered:

- converted every exercise plan to an in-memory Type-1 MIDI;
- emitted a conductor track with requested tempo and 4/4 timing;
- emitted named, separate right- and left-hand tracks on distinct channels;
- preserved melodic and chord spacing with an 80% note gate;
- generated concise exercise names and stable content identities;
- passed both-hand exercises through the normal `Song` configuration;
- verified that existing both/right/left-hand practice controls recognize the
  generated tracks;
- completed `EX-001B` while leaving the menu/player-facing `EX-001` open.

Verification:

- repeated conversion of the same plan produces the same content identity;
- changing tempo changes both real note timing and content identity;
- C-major 60 BPM notes last 800 ms and 120 BPM notes last 400 ms;
- generated both-hand songs infer exactly one left and one right practice part;
- two MIDI-file tests, fifty-eight practice/core tests and fifty application
  tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `7fe91f4`.

### Cycle 038 — Deterministic exercise plans

State: **DONE**

Delivered:

- defined serializable exercise specifications for tonic, tonality, pattern,
  direction, hands, octave span and tempo;
- generated major/minor scales, tonic arpeggios and primary-chord cadences;
- generated right, left or parallel both-hand moments in stable registers;
- supported ascending, descending and apex-deduplicated up/down forms;
- validated tonic, one-to-three-octave span, 20–240 BPM and keyboard range;
- represented melodic and chord durations without coupling to rendering;
- documented the in-memory MIDI, player, identity, UI and future fingering
  boundaries;
- completed `EX-001A` while leaving the full `EX-001` mode open.

Verification:

- C major both-hand up/down pitches are exact;
- A minor descending arpeggio uses the minor third;
- c minor cadence uses i–iv–V–i with a major functional dominant;
- three-octave B major fits the 88-key range and fails a smaller range;
- invalid tonic, octave span and tempo are rejected;
- two MIDI-file tests, fifty-six practice/core tests and forty-nine application
  tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `d45b7c5`.

### Cycle 037 — Pedal timing and dynamics contour

State: **DONE**

Delivered:

- timestamped score and performed sustain transitions on the shared practice
  clock;
- compared pedal-down and pedal-up moves separately using robust median offset
  and median absolute deviation;
- required four fully paired transitions, equal transition counts and at most
  120 ms spread before presenting a stable pedal profile;
- aggregated chord velocities by exact score onset before comparing consecutive
  dynamic directions;
- classified shaped steps as followed, flat or opposite;
- required six shaped steps before presenting contour evidence;
- expanded Technique expression feedback to five compact, explicit lines;
- preserved old practice-history compatibility through field defaults;
- closed `MUS-005`.

Verification:

- deterministic evidence produces six of six aligned contour steps;
- four paired pedal moves at +40 ms produce median `40 ms late`, spread `0`;
- transition mismatch and insufficient evidence produce guarded copy;
- legacy expression data loads with zeroed new evidence;
- two MIDI-file tests, fifty-one practice/core tests and forty-nine application
  tests pass;
- deterministic completion smoke passes all tabs and Retry;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `3f99cde`.

### Cycle 036 — Deterministic native completion flow

State: **DONE**

Delivered:

- added a runtime-generated 115-byte Type-1, two-hand MIDI fixture;
- kept the fixture, settings, history and SoundFont inside the isolated smoke
  directory;
- automatically performed every blocked fixture target until completion;
- asserted the completion screen opens on Overview;
- asserted semantic switching through Technique, History and Overview;
- asserted completion Retry returns to the player and resets matched notes;
- retained the existing full-song smoke path as a separate parameter set.

Verification:

- completion fixture reports two matched notes;
- Overview, Technique and History checks pass;
- Retry resets matched notes to zero;
- fixture process exits cleanly with code `0`;
- the prepared full “Look at the Sky” smoke path still passes independently;
- the full Rust test, Clippy and release gates from Cycle 035 remain green;
- implementation commit: `1fc952c`.

### Cycle 035 — Scored MIDI injection in native smoke

State: **DONE**

Delivered:

- exposed sorted current required pitches without weakening matcher
  encapsulation;
- added an acknowledged, range-validated Debug MIDI note event;
- routed injected notes through the active scene's normal MIDI handler;
- added a bounded `MIDI channel note velocity` local-driver command;
- exposed required-note count and pitches in practice snapshots;
- extended the real-process smoke runner to wait for score targets, play them
  and assert that the practice matcher count increases.

Verification:

- the prepared “Look at the Sky” MIDI exposed two required notes;
- injected NoteOn/NoteOff pairs were accepted by the active player;
- `matched_notes` increased to `2` without direct statistic mutation;
- invalid MIDI channel/note ranges are rejected by protocol tests;
- two MIDI-file tests, fifty practice/core tests and forty-eight application
  tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `6fd60aa`.

### Cycle 034 — Native loop and restart coverage

State: **DONE**

Delivered:

- routed the visible loop control and semantic action through one shared
  implementation;
- added stable loop-toggle and current-scope restart actions;
- added `R` as a learner-facing current-scope restart shortcut;
- exposed loop activation, measure range, count-in and pause state in Debug
  snapshots;
- extended the real-process smoke runner through loop enable, loop-preserving
  restart and loop disable;
- documented the shortcut and automation contract;
- closed `QA-001`.

Verification:

- the prepared two-hand “Look at the Sky” MIDI opens and starts in the real
  application;
- wait mode reports `true → false`;
- default loop reports measures `1–2`;
- restart preserves measures `1–2` and keeps the loop active;
- the second loop toggle disables it;
- return to menu and clean exit code `0` pass;
- two MIDI-file tests, fifty practice/core tests and forty-eight application
  tests pass;
- Clippy and release build report only pre-existing warnings;
- implementation commit: `f49cd2c`.

### Cycle 033 — Reusable native practice smoke runner

State: **DONE**

Delivered:

- added a checked-in PowerShell runner for a real Debug application process;
- isolated settings, history and the copied SoundFont in a disposable
  per-run directory;
- selected an unused loopback port automatically;
- asserted menu-to-player start, default wait mode, wait toggling, hand-mode
  cycling, return to menu and exit code;
- added an acknowledged Debug exit command so the app shuts down through its
  event loop;
- retained exact-process termination only as failure cleanup.

Verification:

- the runner passes with the prepared “Look at the Sky” two-hand MIDI;
- observed wait `true → false`, hands `Both → Right` and exit code `0`;
- two MIDI-file tests, fifty practice/core tests and forty-eight application
  tests pass;
- Clippy reports only the repository's pre-existing warnings;
- release build passes without the driver or Debug exit event;
- implementation commit: `5f16f5c`.

### Cycle 032 — Loopback debug practice driver

State: **DONE**

Delivered:

- exposed the debug harness to local test processes through an opt-in TCP
  driver;
- restricted binding to IPv4/IPv6 loopback addresses and capped command size;
- added JSON action acknowledgements and practice snapshots with timeouts;
- added a semantic menu-start action so an automation run can enter a
  command-line-loaded song without coordinate clicks;
- kept the driver off by default and compile-time absent from release builds;
- documented configuration, commands, results and security boundary.

Verification:

- two MIDI-file tests, fifty practice/core tests and forty-eight application
  tests pass;
- protocol parsing, JSON scalar formatting and loopback enforcement are tested;
- a real Debug process loaded the prepared “Look at the Sky” MIDI, accepted
  menu start, reported wait mode on, accepted the wait toggle, reported it off,
  and accepted return to menu;
- Clippy reports only the repository's pre-existing warnings;
- release build passes without the driver module;
- implementation commit: `eaf40f3`.

### Cycle 031 — Confirmed semantic action dispatch

State: **DONE**

Delivered:

- changed debug action dispatch from queue-only success to an explicit
  accepted/rejected result from the active scene;
- added caller-controlled timeouts so a stalled event loop cannot hang a test;
- requested redraws only for actions accepted by the active scene;
- documented the worker-thread requirement and the stronger assertion
  contract.

Verification:

- two MIDI-file tests, fifty practice/core tests and forty-five application
  tests pass;
- all semantic mapping and stable-ID tests remain green;
- Clippy reports only the repository's pre-existing warnings;
- release build passes without debug action events;
- implementation commit: `19a7d13`.

### Cycle 030 — Debug practice automation harness

State: **DONE**

Delivered:

- added a debug-only harness that sends semantic actions through the normal
  application event loop;
- exposed a read-only practice snapshot for wait mode, Tempo Coach, hands,
  completion tab, attempt counts and input latency;
- routed wait, coach and retry through the same product methods used by visible
  controls;
- supported player navigation, mode toggles, completion tabs and retry/back;
- documented the exact supported boundary instead of claiming click-only
  parameterized actions;
- compile-time excluded the harness and event variants from release builds.

Verification:

- two MIDI-file tests, fifty practice/core tests and forty-five application
  tests pass;
- tests cover the explicit action mapping, unsupported boundary and stable ID
  catalogue;
- Clippy reports only the repository's pre-existing warnings;
- release build passes without debug-harness code or new warnings;
- implementation commit: `d428acf`.

### Cycle 029 — Stable practice action identities

State: **DONE**

Delivered:

- established a unique `practice.*` semantic action namespace;
- identified core player controls, completion tabs and completion actions;
- kept IDs stable across responsive placements and changing visible labels;
- added a catalogue uniqueness test;
- documented the present in-process boundary and future automation layers.

Verification:

- two MIDI-file tests, fifty practice/core tests and forty-four application
  tests pass;
- catalogue namespace/uniqueness and all existing interaction tests pass;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `a549e07`.

### Cycle 028 — Completion feedback information architecture

State: **DONE**

Delivered:

- split completion feedback into Overview, Technique and History;
- reserved Overview for outcomes and practice actions;
- moved detailed timing/expression evidence and calibration to Technique;
- retained trends and weak-history detail in History;
- kept primary retry/back actions global;
- added responsive compact title behavior and a tested 480 px tab layout;
- added wraparound left/right keyboard navigation.

Verification:

- two MIDI-file tests, fifty practice/core tests and forty-three application
  tests pass;
- tests cover minimum-width geometry, keyboard navigation and all existing
  learner-facing feedback helpers;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `a8bcf89`.

### Cycle 027 — Block-chord synchronization evidence

State: **DONE**

Delivered:

- grouped only exact-onset, known-measure score targets as chord candidates;
- separated complete from incomplete chords;
- summarized earliest-to-latest attack span only for fully matched chords;
- required four complete chords before showing median and maximum evidence;
- excluded written different-onset arpeggios;
- persisted the evidence with legacy defaults and descriptive UI wording.

Verification:

- two MIDI-file tests, fifty practice/core tests and forty-one application
  tests pass;
- tests cover known spans, incomplete chords, different-onset arpeggios,
  evidence thresholds, migration and copy;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `01289b0`.

### Cycle 026 — Reliable rhythm trouble spots

State: **DONE**

Delivered:

- persisted robust timing evidence per measure;
- aggregated repeated measure evidence within one practice kind/hand scope;
- required two attempts, twelve matched notes and explicit bias/spread
  thresholds;
- ignored stable and under-sampled measures;
- ranked rhythm issues separately from wrong/missed-note accuracy;
- added separate Notes and Rhythm loop actions with compact responsive layout.

Verification:

- two MIDI-file tests, forty-nine practice/core tests and forty application
  tests pass;
- tests cover measure aggregation, repeated evidence, thresholds, ranking,
  scope isolation, migration and existing note recommendations;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `ffdb072`.

### Cycle 025 — Left/right-hand timing profiles

State: **DONE**

Delivered:

- retained raw timing offset on each matched structured result;
- aggregated robust timing profiles independently by practice part;
- required eight matched notes from each hand rather than pooling evidence;
- showed each hand's early/late median and typical spread;
- reported the remaining sample count for an under-evidenced hand;
- migrated older persisted part summaries safely.

Verification:

- two MIDI-file tests, forty-six practice/core tests and forty application
  tests pass;
- tests cover distinct hand biases, isolated sample thresholds, migration and
  learner-facing copy;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `c153721`.

### Cycle 024 — Conservative calibration suggestions

State: **DONE**

Delivered:

- converted stable signed timing evidence into an explicit offset suggestion;
- required 24 notes, ≤35 ms typical spread and ≥10 ms median bias;
- capped one correction at 50 ms and retained the ±250 ms global bounds;
- explained every unavailable state instead of hiding the decision;
- required confirmation through an inline apply-and-retry action;
- restarted the piece immediately so the learner verifies the new value.

Verification:

- two MIDI-file tests, forty-four practice/core tests and thirty-nine
  application tests pass;
- tests cover every evidence gate, deadband, both correction directions,
  per-step cap and calibration limits;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `27ae4f2`.

### Cycle 023 — Robust timing profile

State: **DONE**

Delivered:

- retained signed raw timing offsets before grade normalization;
- computed median early/late bias and median absolute deviation;
- required eight notes before presenting timing-profile claims;
- showed bias and typical spread beside the familiar timing counts;
- persisted the profile with legacy defaults and reset it per attempt.

Verification:

- two MIDI-file tests, forty-two practice/core tests and thirty-nine
  application tests pass;
- tests cover signed offsets, even-sample medians, robust deviation,
  insufficient evidence, copy and legacy data;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `f69d51d`.

### Cycle 022 — Input-latency compensation

State: **DONE**

Delivered:

- added a persistent, signed practice input timing offset;
- bounded it to ±250 ms and exposed 5 ms Settings adjustments;
- shifted assessment timestamps without delaying or rewriting MIDI output;
- preserved key-hold duration by applying one constant offset to both edges;
- handled session-start underflow safely;
- showed every active non-zero offset in the player status line;
- migrated older settings to a neutral zero offset.

Verification:

- two MIDI-file tests, forty-one practice/core tests and thirty-eight
  application tests pass;
- tests cover defaults, legacy settings, clamp boundaries, positive/negative
  arithmetic and an exact on-time player judgement after compensation;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `94fb28c`.

### Cycle 021 — Key-hold duration evidence

State: **DONE**

Delivered:

- paired matched live note-on/note-off events with parsed score durations;
- handled both ordinary late matching and play/release-before-target matching;
- computed a median duration ratio resistant to isolated long held notes;
- exposed exact `<75%`, `75–125%` and `>125%` distribution counts;
- required four completed notes before showing the comparison;
- kept sustain-pedal use separate from physical key-hold evidence;
- cleared incomplete duration state safely on transport resets;
- migrated Cycle 020 expression histories with a default articulation summary.

Verification:

- two MIDI-file tests, thirty-nine practice/core tests and thirty-seven
  application tests pass;
- tests cover early release, shorter/equal/longer holds, median calculation,
  parsed MIDI duration lookup, exact MIDI forwarding and nested legacy data;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `91fd14f`.

### Cycle 020 — Descriptive expression evidence

State: **DONE**

Delivered:

- captured played and reference velocity only for successfully paired notes;
- summarized sample count, played/reference ranges and mean absolute gap;
- captured sustain use, distinct CC64 transitions and continuous half-pedal
  values for both score and live input;
- added a default-on but optional Expression Summary on the completion screen;
- kept claims descriptive and explicitly avoided grading tone or pedalling
  without calibration;
- persisted the new evidence with backward-compatible history defaults;
- proved external MIDI/Pianoteq messages retain their original channel and
  values.

Verification:

- two MIDI-file tests, thirty-eight practice/core tests and thirty-seven
  application tests pass;
- tests cover early/late velocity pairing, pedal transitions, continuous
  values, reset, legacy history and exact output forwarding;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `1a04bc2`.

### Cycle 019 — Explainable spaced review

State: **DONE**

Delivered:

- added a deterministic, local spaced-review policy based on measured accuracy
  and on-time-note ratio;
- required both 90% note accuracy and 70% on-time matches for a mastered take;
- scheduled first, second and repeated mastery at conservative 1, 3, 7 and
  eventually 14-day intervals;
- made weak or unmeasured latest takes due immediately instead of granting a
  misleading retention interval;
- computed mastery streaks only within the same whole-song/loop and hand scope;
- added a Due library view ordered by oldest due date;
- displayed an explicit reason: needs measured take, reinforce, retention
  check, or days remaining with mastery streak;
- retained the existing per-row Queue action so due work can be placed into the
  learner's ordered plan.

Verification:

- two MIDI-file tests, thirty-five practice/core tests and thirty-five
  application tests pass;
- tests cover immediate reinforcement, repeated-mastery intervals, due-date
  boundaries, days remaining and learner-facing explanations;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `c5ee410`.

### Cycle 018 — Favourites and ordered practice queue

State: **DONE**

Delivered:

- added persistent favourites keyed by MIDI content identity;
- sorted favourites ahead of ordinary pieces in the full library;
- added a persistent ordered practice queue with add and remove actions;
- added a focused queue view with explicit up/down reordering;
- normalized queue positions after removal so ordering stays compact and
  deterministic;
- retained organization for moved files through the existing content identity
  and source-path repair model;
- displayed latest accuracy and reliable weak-measure recommendations directly
  on library/queue rows;
- allowed newly indexed, never-opened songs to be favourited or queued;
- migrated older history files to empty organization state.

Verification:

- two MIDI-file tests, thirty-three practice/core tests and thirty-four
  application tests pass;
- tests cover persistent favourites, queue insertion/removal/reordering,
  boundary moves, position normalization and legacy migration;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `936b95c`.

### Cycle 017 — Watched-folder indexing and search

State: **DONE**

Delivered:

- added persistent watched-folder configuration with add and remove controls;
- recursively scanned `.mid` and `.midi` files away from the render thread;
- avoided following directory symlinks and tolerated unreadable directories;
- parsed candidate files and skipped malformed MIDI without aborting the scan;
- deduplicated identical files by BLAKE3 MIDI content identity while retaining
  every discovered path;
- cached normalized title/path text for fast multi-term search;
- merged discovered repertoire with existing sessions, accuracy and relocation
  information in Practice Library;
- added type-anywhere search, Backspace editing, Escape-to-clear and a refresh
  action;
- retained direct open and hash-verified Locate behaviour for every result;
- migrated older settings files to an empty watched-folder list automatically.

Verification:

- two MIDI-file tests, thirty-one practice/core tests and thirty-four
  application tests pass;
- tests cover recursive discovery, content deduplication, malformed MIDI,
  multi-term title/path search, unique/removable folder configuration and
  legacy-settings migration;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `e993f2c`.

### Cycle 016 — Recent practice library

State: **DONE**

Delivered:

- added a dedicated Practice Library page to the native home screen;
- listed recent pieces by actual practice/setup activity rather than filename;
- showed session count and latest measured accuracy for each piece;
- reopened available files directly from their saved source path;
- detected missing files and offered a Locate recovery action;
- verified relocated files by MIDI content hash before accepting them;
- preserved the original history and setup when the wrong replacement is
  selected;
- updated the stored source path automatically after successful relocation;
- shortened long and Unicode song titles safely in the compact list.

Verification:

- two MIDI-file tests, twenty-seven practice/core tests and thirty-three
  application tests pass;
- tests cover source-path capture, recent ordering, content identity, setup
  persistence and Unicode-safe labels;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `e9338da`.

### Cycle 015 — Per-song practice setup

State: **DONE**

Delivered:

- persisted track play roles and visibility for each MIDI content identity;
- persisted each song's playback speed, loop enabled state and inclusive
  measure range;
- restored saved track choices before entering track selection or playback;
- restored an active loop with a fresh count-in, or retained an inactive loop
  range for the next time it is enabled;
- saved changes from hand switching, speed buttons, keyboard speed controls,
  adaptive coaching, loop toggles and loop-handle edits;
- rejected saved track layouts that no longer match the MIDI structure;
- rejected zero, reversed and out-of-range saved measure loops;
- extended the existing atomic history file without breaking older files.

Verification:

- two MIDI-file tests, twenty-six practice/core tests and thirty-two application
  tests pass;
- tests cover content-based setup persistence, renamed files, exact track
  restoration, structural rejection, loop restoration and legacy migration;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `f1a566f`.

### Cycle 014 — In-player hand practice modes

State: **DONE**

Delivered:

- added a persistent player control for both-hands, right-hand and left-hand
  practice;
- kept the unpractised hand audible as automatic accompaniment instead of
  muting it;
- preserved non-hand accompaniment and custom track choices;
- restarted the active whole-song or loop attempt safely after every switch,
  including output panic, matcher reset and loop count-in;
- refused to guess when a MIDI does not have reliable left/right assignments;
- kept the control visible at the minimum window width with a responsive
  toolbar layout;
- stored hand scope in practice history so trends, weak measures and passage
  recommendations never mix both-hand and single-hand attempts;
- retained compatibility with history files written before hand scope existed.

Verification:

- two MIDI-file tests, twenty-five practice/core tests and thirty application
  tests pass;
- tests cover accompaniment semantics, ambiguous arrangements, scoped trends,
  scoped weakness ranking and legacy-history migration;
- Clippy reports only the repository's pre-existing warnings;
- release build passes;
- implementation commit: `f630ea4`.

### Cycle 013 — Active output visibility

State: **DONE**

Delivered:

- added a persistent, colour-coded output badge beside the Panic control;
- distinguished built-in SoundFont, external MIDI and silent/no-output states;
- showed the external MIDI port name on normal-width windows;
- used a compact backend label on narrow windows and hid overlapping practice
  statistics at the minimum supported width;
- used Unicode-safe truncation for long device names;
- made the badge clickable, opening the settings page directly while retaining
  the current song;
- confirmed the fresh-install default remains the built-in SoundFont piano.

Verification:

- two MIDI-file tests, twenty-three practice/core tests and twenty-eight
  application tests pass;
- tests cover backend status labels, default output selection and Unicode-safe
  device-name truncation;
- responsive positions respect the 670-pixel minimum window width;
- Clippy reports no new warnings;
- release build passes;
- implementation commit: `41f3f95`.

### Cycle 012 — Global emergency panic

State: **DONE**

Delivered:

- added a high-contrast `PANIC F12` control that remains visible even while the
  rest of the player toolbar is collapsed;
- promoted F12 to an application-level emergency shortcut across player,
  completion, free-play and preview scenes;
- silenced the instrument, paused playback and cleared pending practice input;
- cancelled an active loop count-in without changing the song or completed
  history;
- cleared user/file keyboard highlights, mouse-held notes and free-play chord
  display;
- fixed zero-velocity Note On visual handling so release messages cannot leave
  a highlighted key behind.

Verification:

- two MIDI-file tests, twenty-three practice/core tests and twenty-six
  application tests pass;
- emergency-stop tests cover output panic, paused state and pending matcher
  cleanup;
- visual note-state tests cover zero-velocity releases;
- Clippy reports no new warnings;
- release build passes;
- implementation commits: `2b52463`, `235be25`.

### Cycle 011 — Expressive MIDI fidelity

State: **DONE**

Delivered:

- upgraded the test output to retain exact channel and MIDI-message values;
- proved live keyboard/controller input is forwarded without transformation;
- proved controller data on human practice tracks remains audible in guided
  wait mode while score notes remain learner targets;
- covered two non-binary CC64 values for continuous/half-pedal behaviour;
- covered a non-centred 14-bit Pitch Bend value and Channel Aftertouch;
- verified final external MIDI bytes retain the controller, bend and pressure
  values exactly.

Verification:

- two MIDI-file tests, twenty-three practice/core tests and twenty-four
  application tests pass;
- synthetic MIDI fixtures exercise expressive events on a real parsed human
  track;
- serialization assertions cover CC, two-byte Pitch Bend and Channel
  Aftertouch wire bytes;
- Clippy reports no new warnings;
- release build passes;
- implementation commit: `2fda08d`.

### Cycle 010 — External MIDI and Pianoteq safety

State: **DONE**

Delivered:

- fixed zero-velocity Note On tracking so it releases rather than registers an
  active note;
- expanded external MIDI panic from tracked Note Off events to a conservative
  all-channel sequence;
- sent sustain-off, All Notes Off, All Sound Off and Reset All Controllers on
  every one of the 16 MIDI channels;
- silenced the old instrument before replacing an output connection;
- prevented temporary shared-connection clones from unexpectedly silencing an
  active player;
- verified pause, seek, restart and player teardown all invoke the centralized
  silence path;
- added an external Pianoteq setup, daily check, troubleshooting guide and
  device acceptance checklist.

Verification:

- two MIDI-file tests, twenty-three practice/core tests and twenty-one
  application tests pass;
- MIDI tests cover zero-velocity releases, panic controller order, all-channel
  coverage and transport lifecycle calls;
- Clippy reports no new warnings;
- release build passes;
- implementation commit: `daf86c3`.

### Cycle 009 — Current-song practice history

State: **DONE**

Delivered:

- added a polished `This take / History` switch to the completion panel;
- summarized total saved sessions and the four most recent attempts;
- displayed attempt scope, accuracy and playback speed for each recent entry;
- calculated accuracy and speed deltas only between comparable practice scopes,
  preventing whole-song and short-loop scores from producing false trends;
- ranked up to four persistent weak measures in a compact two-column view;
- applied the same repeated-evidence threshold to the history ranking;
- kept the suggested-passage action available from either completion tab.

Verification:

- two MIDI-file tests, twenty-three practice/core tests and seventeen
  application tests pass;
- history tests cover ordering, trend deltas, insufficient evidence and
  mixed-scope isolation;
- the completion card respects the application's 670 × 620 minimum window;
- Clippy reports no new warnings;
- release build passes;
- implementation commit: `ccf3b2f`.

### Cycle 008 — Evidence-based weak-passage practice

State: **DONE**

Delivered:

- ranked weak measures from persisted attempts rather than only the latest
  take;
- required at least two attempts and eight judged notes before making a
  recommendation;
- suppressed recommendations at 90% accuracy or above;
- selected a two-measure context around the weakest qualifying measure;
- explained the recommendation with its range, accuracy, judged-note count and
  attempt count;
- added a full-width completion action that creates the suggested measure loop
  and immediately begins its normal visual count-in;
- clamped recommendations and inclusive loop boundaries safely at the end of a
  song.

Verification:

- two MIDI-file tests, twenty practice/core tests and seventeen application
  tests pass;
- recommendation tests cover repeated evidence, small samples and mastered
  measures;
- loop mapping tests prove inclusive measure ranges retain the end boundary;
- Clippy reports no new warnings;
- release build passes;
- implementation commit: `e1d80d0`.

### Cycle 007 — Durable local practice history

State: **DONE**

Delivered:

- assigned every MIDI a stable BLAKE3 content identity, independent of its
  filename or location;
- added a versioned local practice-history schema for whole-song and loop
  sessions;
- persisted speed, overall results, measure results and hand results at the end
  of each completed attempt;
- used same-directory atomic replacement, including write-through replacement
  on Windows;
- quarantined malformed history files and continued startup with an empty
  history;
- retained the 200 most recent sessions per song;
- exposed aggregated weak-measure ranking for the next coaching cycle;
- confirmed saved session count in the whole-song completion panel.

Verification:

- two MIDI-file tests, eighteen practice/core tests and sixteen application
  tests pass;
- history tests cover round trips, rename continuity, corruption recovery,
  retention and cross-session weak-measure aggregation;
- Clippy reports no new warnings;
- release build passes;
- implementation commit: `039a93d`.

### Cycle 006 — Adaptive tempo coach

State: **DONE**

Delivered:

- added an explicit, persisted `Coach: ON/OFF` control that defaults off;
- exposed mastery target plus minimum and maximum coached speed in settings;
- required two consecutive mastered loop takes before raising speed by 5%;
- required both configured note accuracy and 70% on-time notes for mastery;
- reduced speed by 5% only below the 70% safety threshold;
- held speed for moderate attempts and at configured limits;
- reset the mastery streak after manual speed changes, range changes or coach
  toggles;
- displayed a three-second explanation for every hold, increase or decrease;
- replaced the old 10%-quantized playback math with precise proportional
  timing, so 75% and 105% now play at their displayed speeds.

Verification:

- fourteen practice-domain tests and sixteen application tests pass;
- coach tests cover mastery streak, regression, holding and limits;
- playback timing tests prove 75% and 105% scale exactly;
- legacy configuration receives safe opt-in defaults through Serde defaults;
- Clippy reports no new warnings;
- release build passes;
- implementation commit: `be7e77e`.

### Cycle 005 — Structured loop practice

State: **DONE**

Delivered:

- changed the loop from a raw playback jump into isolated practice attempts;
- snapped new and dragged loop boundaries to actual measure starts;
- defaulted a new loop to a two-measure passage around the current position;
- added a one-measure visual `4–3–2–1` count-in before every take;
- reset matching state at each repetition so attempts do not contaminate one
  another;
- tracked take number, previous accuracy and best accuracy in the top bar;
- ranked equal-accuracy attempts by on-time notes, then matched-note count;
- labelled the highlighted timeline range with its measure numbers;
- fixed seeking so notes exactly on the loop boundary are played instead of
  being discarded.

Verification:

- eleven practice-domain tests, fifteen application tests and two MIDI-file
  tests pass;
- boundary tests prove seek retains events exactly at the target timestamp;
- loop helper tests cover snapping, count-in duration and countdown state;
- Clippy reports no new warnings;
- release build passes;
- implementation commit: `243829b`.

### Cycle 004 — Measure-aware attempt summary

State: **DONE**

Delivered:

- attached score time, track ID, one-based measure and practice part to every
  expected note;
- inferred left/right hand for two-track piano files by pitch center, while
  leaving ambiguous multi-part arrangements explicitly unassigned;
- retained matched, missed and context-attributed wrong-note results;
- aggregated results by measure and by hand;
- replaced automatic return-to-menu at song end with a calm completion panel;
- showed overall timing, errors, hand accuracy and up to four weak measures;
- added `Practice again` and `Back to songs` completion actions.

Verification:

- nine practice-domain tests and twelve application tests pass;
- aggregation tests cover correct right-hand notes plus wrong/missed left-hand
  notes in separate measures;
- application integration proves real MIDI events produce one-based measure
  summaries;
- Clippy reports no new warnings;
- release build passes;
- implementation commit: `715ed9c`.

### Cycle 003 — Repeated notes and missed-note lifecycle

State: **DONE**

Delivered:

- replaced pitch-keyed single entries with FIFO occurrence queues;
- matched repeated and overlapping same-pitch targets independently;
- added a configurable late window and explicit missed-note totals;
- enabled the same matcher in flow practice when wait mode is off;
- kept guided wait targets pending indefinitely instead of misclassifying the
  learner while they search for the note;
- expanded the live status to show hits, wrong notes, misses and current need.

Verification:

- eight practice-domain tests and ten application tests pass;
- application integration tests distinguish guided waits from flow misses;
- release build passes;
- implementation commit: `fbee89d`.

### Cycle 002 — Deterministic practice feedback

State: **DONE**

Delivered:

- extracted matching and session totals from `MidiPlayer` into
  `neothesia_core::practice`;
- replaced direct wall-clock reads with a caller-supplied monotonic session
  time, allowing precise tests without sleeps;
- classified matched notes as early, on-time or late with configurable windows;
- counted expired and duplicate presses as wrong notes;
- tracked unordered target chords and exposed the number of notes still needed;
- added a compact live `Correct / Wrong / Waiting` status in the player;
- froze practice timing while paused while continuing it during guided waits.

Verification:

- five focused matcher tests cover timing, expiry, chords, keyboard range and
  duplicate presses;
- full `neothesia-core` and `neothesia` target tests pass;
- release build passes;
- implementation commit: `bab60ea`.

### Cycle 001 — Guided-practice baseline

Goal: make a selected two-hand piano MIDI immediately usable for practice.

Included:

- default piano note tracks to human-controlled practice;
- wait-for-notes enabled by default;
- obvious `Wait: ON/OFF` control in the player;
- optional quarter-note guidelines;
- one-based measure numbers;
- configuration migration defaults and focused tests.

Acceptance checklist:

- [x] Legacy settings deserialize with practice-friendly defaults.
- [x] Wait mode can release a blocked player when disabled.
- [x] MIDI parsing produces ordered measure and beat timestamps.
- [x] Focused tests pass.
- [x] Native release build passes.
- [ ] Real MIDI smoke test covers open, wait, toggle, loop and exit.
- [x] Implementation committed as `a1755bc`.
- [x] Cycle documentation committed.

## Product health snapshot

| Capability | State | Notes |
| --- | --- | --- |
| MIDI open/play | Working | File-picker transition fix committed |
| Guided wait | Working | Default mode with visible player control |
| Measure/beat grid | Verifying | Current Cycle 001 |
| Loop practice | Working | Measure snapping, count-in, attempts and adaptive tempo |
| Performance feedback | Working | Live totals, completion summary and measure/hand detail |
| Practice history | Working | Hand-scoped trends, weak passages and spaced review |
| Built-in piano | Working | Fresh-install default; active route visible |
| External Pianoteq | Usable workflow | Active route visible; device soak pending |
| Native VST3 | Planned | Separate long-term roadmap |
| Library | Working | Search, favourites, ordered queue and relocation repair |
| UI automation | Partial | OS input/screenshot; semantic actions planned |

## Next decision

Begin Cycle 030 with a debug-only semantic action harness. Route action IDs
through the normal application event loop, expose a minimal read-only practice
state snapshot, and prove tab/toggle/retry flows without coordinate clicks or
shipping a control channel in release builds.

## Known constraints

- The custom GPU UI has no DOM and limited accessibility/automation semantics.
- The CLI/video package requires local FFmpeg development dependencies.
- Hand inference is intentionally conservative for arrangements with more than
  two playable note tracks.
- Loop count-in is visual only; an optional metronome click remains future work.
- Adaptive coaching currently operates only on structured loop attempts.
- Practice history is visible after completing a song but does not yet have a
  library-level browser, export or reset screen.
- External MIDI safety is automatically covered, but the documented 30-minute
  Pianoteq device soak test still requires the physical setup.
- Recommendations currently optimize note accuracy; timing consistency, hand
  balance and calibrated expression need later goal-specific recommendation
  rules.
- Native VST3 hosting is a realtime and lifecycle project, not merely a picker.
