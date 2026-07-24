# Development progress

Updated: 2026-07-25

## Current milestone

**M1 — Practice intelligence**

Overall state: **IN PROGRESS**

## Active cycle

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
