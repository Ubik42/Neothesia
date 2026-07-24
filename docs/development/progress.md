# Development progress

Updated: 2026-07-25

## Current milestone

**M1 — Practice intelligence**

Overall state: **IN PROGRESS**

## Active cycle

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
| Practice history | Working | Hand-scoped trends and weak-passage action |
| Built-in piano | Working | Fresh-install default; active route visible |
| External Pianoteq | Usable workflow | Active route visible; device soak pending |
| Native VST3 | Planned | Separate long-term roadmap |
| Library | Minimal | File picker and recent path only |
| UI automation | Partial | OS input/screenshot; semantic actions planned |

## Next decision

Begin Cycle 015 by preserving each song's practice setup. Restore the selected
hand/track roles, loop range and speed after reopening the same MIDI, while
remaining robust when the file is moved, renamed or structurally changed.

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
  balance, pedal and dynamics need later goal-specific recommendation rules.
- Native VST3 hosting is a realtime and lifecycle project, not merely a picker.
