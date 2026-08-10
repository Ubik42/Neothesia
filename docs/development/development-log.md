# Development log

This is an append-only engineering log. Newest entries go first. Every closed
cycle records the user outcome, implementation, verification, known limitations
and commit.

## 2026-08-10 — Cycle 103: Public development handoff and practice corpus (DONE)

### Outcome

The active piano-learning fork now has an honest public entry point, a durable
local tool location and a reproducible source-aware MIDI corpus for daily
practice and feature testing.

### Implemented

- Migrated the music tool collection from `D:\Music\Tools` to
  `D:\Music\_tools` without losing the Neothesia, Openthesia or Transkun Git
  histories.
- Updated the Transkun pipeline default, Skill links, REAPER reference and
  desktop Neothesia shortcut to the new root.
- Added `scripts/sync-practice-library.ps1` with checksum-verified MAESTRO and
  Pop-K acquisition, bounded excerpt extraction and teaching-oriented Mutopia
  collection.
- Made per-piece Mutopia failures recoverable so one stale public link cannot
  prevent the remaining corpus and catalogue from being published locally.
- Added a generated catalogue contract that records category, title, composer,
  license, source URL and local relative path.
- Reworked the root README and roadmap capability map to distinguish working,
  experimental and future functionality.

### Verification

- The old root is absent, the new root exists and all three nested repositories
  resolve valid `HEAD` commits.
- The PowerShell sync script parses under PowerShell 7.
- MAESTRO and Pop-K archives match their official SHA-256 and MD5 values.
- Rust, documentation and release gates are recorded after the final run.

### Known limitations

- MAESTRO and Pop-K carry non-commercial Creative Commons terms and therefore
  are downloaded locally rather than bundled in this repository.
- Mutopia licenses vary per piece; consumers must retain the generated source
  and license metadata.
- Direct VST3 hosting remains future work. The supported high-quality sound
  path is external MIDI routing to standalone Pianoteq.

## 2026-08-04 — Cycle 102: Persistent engraved-score sizing (DONE)

### Outcome

The engraved score now opens at a more readable 62% size and has clear minus
and plus controls in its own header. The chosen size survives restart while the
keyboard, automatic page following and active-note overlays remain intact.

### Implemented

- Added a backwards-compatible `score_zoom_percent` appearance preference.
- Bounded the preference to 40–72% and changed the default from the previous
  fixed 52% layout to 62%.
- Added compact minus and plus buttons beside the page label instead of adding
  more controls to the practice toolbar.
- Kept a fixed 160-pixel keyboard reserve at the supported 670×620 minimum
  window, even when persisted input is outside the valid range.
- Added semantic zoom-in and zoom-out actions plus the current percentage to
  the debug snapshot.
- Extended the real score smoke through repeated zoom, upper-bound idempotence,
  restored highlights and settings persistence.

### Verification

- 153 core, 64 default application, 79 feature application and four MIDI-file
  tests pass.
- Layout tests cover minimum-window containment, monotonic growth and invalid
  persisted values clamped at both boundaries.
- The native score process starts at 62%, reaches and holds at 72%, returns to
  62%, preserves four visible highlights and persists the restored value.
- Page-one and page-two 1620×1138 screenshots show the larger score, visible
  controls, keyboard, player toolbar and automatic page transition together.
- Default/feature Clippy and Release builds, exercise/fingering native smokes,
  the eight-score MusicXML corpus and VitePress build pass with only the three
  existing Windows warnings.

Implementation commit: `3686de5` (`feat: add persistent score zoom controls`).

### Known limitations

- Zoom changes fitted page size; it does not crop to one system or rerasterize
  above the worker's existing bounded source texture.
- The synthetic two-page fixture is intentionally sparse. Dense real scores at
  minimum/default/maximum sizes remain the next visual-readability audit.
- The score viewer remains behind the default-off `score-verovio` feature and
  still requires the pinned developer-provided Verovio runtime.

## 2026-08-04 — Cycle 101: Active engraved-note highlights (DONE)

### Outcome

The score now identifies the notes being played, follows them across pages and
keeps the piano controls usable. Highlighting is a native overlay driven by
semantic playback state, not a renderer-controlled playback effect.

### Implemented

- Rewrote Verovio HTML `data-id` attributes only in the verified in-memory SVG
  copy used by `usvg`, allowing safe renderer-ID geometry lookup.
- Extracted the first notehead group's absolute stroked bounds on page threads.
- Scaled and clipped integer note geometry into the displayed page rectangle.
- Added a low-opacity blue fill and opaque two-pixel outline so state is not
  communicated by colour alone.
- Kept cached RGBA immutable and avoided texture replacement on every frame.
- Added synchronized-note, active-highlight and visible-highlight debug counts.
- Treated zero unique renderer correlations as a failed synchronization rather
  than a misleading ready state.
- Added `auto` and `encoded` Verovio break modes and included the selected mode
  in the content cache namespace.
- Replaced the one-page smoke source with a valid two-staff, two-page piano
  fixture whose eight renderer, score and MIDI notes align exactly.

### Verification

- 152 core, 64 default application, 78 feature application and four MIDI-file
  tests pass.
- Geometry tests cover exact notehead bounds, absent renderer IDs, invalid UTF-8
  and bounded page scaling.
- Layout-mode tests prove encoded and automatic output cannot share a cache.
- The real score process reports eight synchronized notes, two cached pages and
  four visible highlights on each focused page.
- It captures page 1, performs the first four-note event, follows to page 2,
  captures page 2 and completes the existing practice-control sequence.
- Hiding the score keeps all four active semantic notes while visible geometry
  drops to zero; showing it restores the same cached overlays.
- Default/feature Clippy and Release builds, exercise/fingering smokes and the
  eight-score MusicXML corpus pass with only the three existing Windows
  warnings.

Implementation commit: `39de55d` (`feat: highlight active engraved score notes`).

### Known limitations

- Highlight colour and thickness are fixed; high-contrast and colour-blind
  palette settings remain future accessibility work.
- Very sparse pages make notation physically small because the viewer fits the
  complete page; zoom and crop-to-system modes are the next readability need.
- Renderer IDs without usable geometry remain semantic diagnostics and are not
  guessed from neighbouring glyphs.

## 2026-08-04 — Cycle 100: Playback-driven score page following (DONE)

### Outcome

The engraved score now follows the performance instead of remaining fixed on
page zero. Page choice comes from the same native semantic alignment used by
learning analysis, including repeated occurrences, and rests hold the most
recent page.

### Implemented

- Extracted stable non-drum performance-note identities as a shared alignment
  primitive.
- Projected native score notes to renderer correlation keys using paired MIDI
  tempo, pitch and duration.
- Correlated only unique native/Verovio semantic note groups.
- Built a validated render index plus occurrence-aware timeline off the UI
  thread for both fresh renders and content-addressed cache hits.
- Evaluated playback focus after every player update.
- Shifted the existing bounded page cache only when the semantic page changes.
- Released the previous texture, uploaded an already-cached target immediately
  or asynchronously loaded the new focused/neighbour window.
- Added synchronization readiness to semantic debug snapshots and real-process
  assertions.

### Verification

- Native evidence tests cover exact 0/500 ms timing, duration and chromatic
  pitch conversion.
- Page-follow tests cover no pre-note focus, first page, rest retention and the
  next page transition.
- 152 core, 64 default application, 73 feature application and four MIDI-file
  tests pass.
- Default/feature Clippy and Release builds pass with only the existing Windows
  warnings.
- The score process smoke constructs real Verovio synchronization, uploads an
  840×1188 verified page, exercises visibility and the practice controls, and
  exits with code zero.
- Exercise/fingering native smokes and the eight-file MusicXML corpus pass.

Implementation commit: `9fa8449` (`feat: follow engraved score playback focus`).

### Known limitations

- The current process fixture has one rendered page; page-two transition is
  covered deterministically but not yet by a native screenshot.
- Active engraved notes are not painted yet; the validated frame already
  contains their renderer identities for the next cycle.
- Semantic correlation intentionally leaves ambiguous unisons unhighlighted
  instead of choosing a visually plausible but potentially wrong voice.

## 2026-08-04 — Cycle 099: Persistent score visibility (DONE)

### Outcome

Players with a paired score now get an obvious `Score: ON` / `Score: OFF`
control in the existing native top bar. Visibility defaults on, survives
restart and can be changed without rerunning engraving work.

### Implemented

- Added a backwards-compatible `score_visible` appearance preference.
- Exposed the control only after a verified score artifact is available.
- Removed the focused GPU image immediately when the score is hidden.
- Kept the content-verified CPU page cache and focus intact while hidden.
- Re-uploaded the cached focused page when visibility is restored.
- Prevented late page loads from uploading a texture while visibility is off.
- Added the stable semantic action `practice.player.score` and snapshot state.
- Extended the native score smoke through hide, cache verification, restore,
  persistence and screenshot capture.

### Verification

- Configuration tests cover new and legacy defaults plus toggling.
- 151 core, 64 default application, 72 feature application and four MIDI-file
  tests pass.
- Default and feature Clippy and Release builds pass with only the three
  pre-existing Windows warnings.
- The native score smoke hid and restored the 840×1188 focused texture without
  losing its single cached page or page-zero focus, persisted the restored
  state and exited with code zero.
- The restored 1620×1138 screenshot shows the blue score control in the native
  top bar while the keyboard and practice controls remain available.
- Exercise/fingering process smokes, eight MusicXML corpus files and the
  VitePress build pass.
- The optional whole-workspace CLI target remains blocked by its pre-existing
  external FFmpeg/vcpkg requirement; all in-scope packages pass.

Implementation commit: `afca3b1` (`feat: add persistent score visibility control`).

### Known limitations

- Playback focus still remains on page zero; automatic page following is the
  next slice.
- Engraved notes are not highlighted yet.
- Visibility is available only in builds that opt into `score-verovio` and for
  songs with a valid content-bound score association.

## 2026-08-04 — Cycle 098: Focused engraved-score texture (DONE)

### Outcome

A paired MusicXML score can now cross the entire opt-in boundary and appear as
a real native GPU surface in the practice player. SVG parsing remains off the
event loop, adjacent pages remain CPU-only and the existing piano, waterfall
and control layers stay available.

### Implemented

- Added optional `resvg` 0.48.1 to the existing `score-verovio` feature.
- Rasterized only verified SVG bytes on named background page threads.
- Bounded decoded dimensions and total pixels before allocation.
- Reused one process-wide system-font database across page work.
- Stored reference-counted RGBA pages in the bounded three-page cache.
- Added direct RGBA upload to the existing native image renderer.
- Replaced the previous focused texture instead of accumulating GPU images.
- Composed a centred score panel below the top control layer with a page label.
- Extended debug snapshots with artifact/cache/focus/texture evidence.
- Added a self-contained score association helper and real-process score smoke.
- Added optional DPI-aware HWND screenshot capture to the smoke runner.
- Added root `PRODUCT.md` for the product register, audience, purpose,
  anti-references, design principles and accessibility target.

### Verification

- Raster tests prove exact opaque RGBA output and aspect-preserving size caps.
- Layout tests cover the supported 670×620 minimum window.
- Upload-decision tests reject no-focus, current-texture and neighbour-only
  cases.
- 150 core, 64 default application, 72 feature application and four MIDI-file
  tests pass.
- The feature smoke rendered the real fixture to an 840×1188 focused texture,
  completed the practice-control sequence and exited with code zero.
- A 1620×1138 DPI-aware native screenshot confirms the score is centred, the
  keyboard remains visible and the top controls render above it.
- Exercise and fingering real-process smokes, the eight-score MusicXML corpus
  and the VitePress documentation build pass.
- Default and feature Clippy and Release builds pass with only the three
  pre-existing Windows warnings.

Implementation commit: `272dd83` (`feat: render verified score page texture`).

### Known limitations

- Score visibility is feature/configuration driven; there is no player toggle
  or persisted user preference yet.
- Focus remains on page zero; playback-driven page following and highlights are
  the next slice.
- Raster size is a fixed bounded quality target rather than viewport/DPI-aware
  rerasterization.
- Node and the pinned Verovio package remain developer-provided dependencies.

## 2026-08-04 — Cycle 097: Asynchronous verified page loading (DONE)

### Outcome

PlayingScene now receives actual verified page bytes without blocking the GPU
event loop or loading the full score into memory. The artifact, song and page
generations all remain intact up to the future display boundary.

### Implemented

- Created the three-page cache when an artifact enters PlayingScene.
- Requested only the focused first page and available next page.
- Loaded pages on named background threads.
- Revalidated content immediately before returning bytes.
- Added a page-ready user event with song and page identities.
- Accepted/rejected results through both generation layers.
- Made failed requests retryable and exposed cache state to debug snapshots.

### Verification

- Core tests cover read-time page verification and retry semantics.
- Existing tests continue to cover eviction and stale result rejection.
- 150 core, 64 default application, 68 feature application and four MIDI-file
  tests pass.
- Default/feature Release builds, exercise/fingering smokes and Clippy pass.

Implementation commit: `410a246` (`feat: load verified score pages asynchronously`).

### Known limitations

- Cached values are verified SVG bytes, not decoded/renderable textures.
- Focus remains on page zero until playback highlight/page following is wired.
- Page-load failure is logged but has no visible retry/status control yet.

## 2026-08-04 — Cycle 096: Generation-safe score worker events (DONE)

### Outcome

An obsolete Verovio job can no longer mutate whichever screen happens to be
open when it finishes. Verified artifacts now cross the same application event
loop as other state changes and are gated by explicit scene generation.

### Implemented

- Added render generations to the application coordinator.
- Advanced generations for Play, menu, settings and free-play transitions.
- Resolved and verified paired score associations before rendering.
- Sent worker results through a feature-gated application user event.
- Accepted the current generation only once.
- Stored accepted artifacts in PlayingScene and exposed readiness to debug
  snapshots.

### Verification

- A deterministic test simulates an old result after a new scene begins and
  proves it cannot change the new scene's artifact state.
- Duplicate current-generation completion is rejected.
- Default and feature test/build/Clippy matrices pass.
- 150 core, 64 default application, 68 feature application and four MIDI-file
  tests pass.

Implementation commit: `e77c776` (`feat: reject stale score worker events`).

### Known limitations

- PlayingScene stores the accepted artifact but does not load page bytes yet.
- Environment configuration is a prototype boundary, not a packaged runtime.
- There is no visible notation surface or user-facing worker status yet.

## 2026-08-04 — Cycle 095: Feature-gated Verovio worker lifecycle (DONE)

### Outcome

Neothesia can now invoke Verovio as an isolated, opt-in worker and obtain a
fully verified, reusable artifact directory. Default users acquire no Node or
Verovio runtime dependency, and half-written renderer output is never published.

### Implemented

- Added the default-off `score-verovio` Cargo feature.
- Added a strict single-score worker script.
- Used direct process arguments and unique staging directories.
- Published a manifest only after every SVG page was written.
- Validated source, manifest and page bytes before atomic cache publication.
- Reused valid cache content and failed closed on corrupt existing entries.
- Bounded captured worker errors and rejected non-SHA cache identities.

### Verification

- Feature tests prove cache hits bypass a deliberately nonexistent Node binary.
- Corrupt cached SVG fails before worker launch.
- Traversal-like source identities are rejected.
- A real process rendered one page/39 notes and passed native verification.
- Default/feature Release builds and feature Clippy pass.
- 150 core, 64 application and four MIDI-file tests pass.

Implementation commit: `de3f54b` (`feat: add feature-gated Verovio worker`).

### Known limitations

- Worker calls are synchronous and must be placed on the existing task thread.
- Application events do not yet carry the document/page generation.
- Cache cleanup/retention policy and native SVG display remain future slices.

## 2026-08-04 — Cycle 094: Bounded synchronized score page cache (DONE)

### Outcome

The future notation view now has a deterministic memory and concurrency model.
Only the focused page and its neighbours can survive, and late asynchronous
output cannot replace pages for another song or an obsolete focus window.

### Implemented

- Added a generic three-page cache independent of SVG/display technology.
- Prioritized current, previous and next page requests.
- Evicted loaded and pending pages when focus moved away.
- Added per-document generations to every worker request.
- Classified stale generation, outside window and unexpected completion
  separately.

### Verification

- Three new cache tests cover priority/eviction, old document/window results,
  duplicate completion and invalid ranges.
- Twelve focused score-view tests pass.
- 150 core, 64 application and four MIDI-file tests pass.
- Clippy, release build, formatting and diff checks pass with only pre-existing
  warnings.

Implementation commit: `2a280a6` (`feat: bound synchronized score page cache`).

### Known limitations

- The cache currently receives generic page values; no worker process feeds it.
- Cancellation is logical rejection, not yet operating-system process
  termination.
- Native SVG decoding/display remains behind the future adapter feature.

## 2026-08-04 — Cycle 093: Content-verified score pages (DONE)

### Outcome

Every generated notation page is now independently trustworthy. The future
page cache can reject truncated, stale or replaced SVG files before displaying
them, even when the surrounding manifest still parses.

### Implemented

- Added per-page SHA-256 to manifest schema 2.
- Verified page byte length and content after reading from disk.
- Added explicit errors for missing hash, I/O, size and fingerprint failures.
- Kept schema 1 readable but ineligible for verified page loading.
- Extended the native inspector to validate the complete artifact set.

### Verification

- All eleven pages across eight pinned fixtures pass native verification.
- A same-byte-length mutation fails with `PageHashMismatch`.
- Nine focused score-view tests pass.
- 147 core, 64 application and four MIDI-file tests pass.
- Clippy, release build, formatting and diff checks pass with only pre-existing
  warnings.

Implementation commit: `1d404ca` (`feat: verify rendered score page content`).

### Known limitations

- Page verification currently reads all pages in the inspector; application
  integration must verify lazily as pages enter the bounded cache.
- Worker request generations and stale-result cancellation are not yet modeled.
- SVG parsing/display remains outside the default application build.

## 2026-08-04 — Cycle 092: Versioned Verovio artifact manifest (DONE)

### Outcome

Generated notation is now a fingerprint-bound, independently validated artifact
instead of an implicit collection of SVG files. A stale, incomplete or unsafe
renderer output cannot silently enter the future player view.

### Implemented

- Added manifest schema version 1 to the pinned Verovio worker.
- Bound every manifest to its exact MusicXML/MXL SHA-256.
- Recorded page files/sizes and all renderer note evidence.
- Added native parsing plus structural, path, fingerprint and value validation.
- Added a native manifest inspector and invoked it from the benchmark wrapper.
- Converted validated notes directly into correlation evidence.

### Verification

- All eight fixture manifests validate against their pinned source hashes.
- Eleven SVG pages and 1,945 note entries cross the worker/native boundary.
- Tests cover valid conversion, stale source, unsupported schema and traversal.
- 146 core, 64 application and four MIDI-file tests pass.
- Clippy, release build, both native smokes, formatting and diff checks pass
  with only pre-existing warnings.

Implementation commit: `2caebfc` (`feat: validate versioned Verovio manifests`).

### Known limitations

- Manifests are generated by the isolated benchmark, not yet by an application
  worker lifecycle.
- Page files are structurally validated but not yet content-hashed individually.
- Native texture/display loading and page eviction remain for `MUS-001D2`.

## 2026-08-04 — Cycle 091: Fail-closed engraved-note correlation (DONE)

### Outcome

The future notation view will not highlight the wrong voice merely because two
notes share a pitch and timestamp. Renderer correlation now succeeds only when
its semantic evidence is unique and exposes every unresolved note explicitly.

### Implemented

- Collected Verovio MIDI time, pitch and duration for every rendered note ID.
- Counted semantic-key collisions in the reproducible benchmark report.
- Added one-to-one native/rendered note correlation.
- Preserved ambiguous groups and one-sided gaps as structured results.
- Refused to use document order as an invisible ambiguity tie-breaker.

### Verification

- 1,945 of 1,945 rendered note IDs returned semantic evidence.
- Six fixtures had no collisions; voice alignment had two ambiguous pairs and
  the MXL fixture had thirteen.
- Six focused score-view tests pass.
- 144 core, 64 application and four MIDI-file tests pass.
- Clippy, release build, formatting and diff checks pass with only pre-existing
  warnings.

Implementation commit: `18b6cf4` (`feat: correlate engraved notes fail closed`).

### Known limitations

- Millisecond semantic evidence still needs a versioned worker manifest.
- Same-time, same-pitch, same-duration unisons remain unresolved until the
  adapter exposes a trustworthy voice/staff discriminator.
- The adapter and native display boundary remain feature-flagged future work.

## 2026-08-04 — Cycle 090: Renderer-neutral score synchronization (DONE)

### Outcome

Neothesia can now translate aligned player time into engraved-note highlights
without letting Verovio, SVG or any future renderer define musical identity.
The next visual prototype has a small, testable boundary instead of direct
renderer calls spread through the player.

### Implemented

- Added a validated native-score to renderer-element/page index.
- Built a highlight timeline from alignment matches and real MIDI note spans.
- Returned simultaneous active highlights for chords.
- Kept page focus on the latest started note through rests.
- Reused written elements for repeat visits while preserving occurrence IDs.
- Kept missing performance and rendering mappings explicit and non-fatal.

### Verification

- Four new focused tests pass.
- 142 core, 64 application and four MIDI-file tests pass.
- Clippy, release build, pinned corpus, exercise smoke, fingering smoke,
  formatting and diff checks pass with only pre-existing warnings.

Implementation commit: `536dc99` (`feat: define synchronized score render protocol`).

### Known limitations

- No Verovio adapter or native SVG/display surface exists yet.
- Adapter correlation between generated Verovio IDs and native event order must
  fail closed when ambiguous.
- Page preloading, cold startup, memory and real highlight paint latency remain
  for `MUS-001D2`/`MUS-001D3`.

## 2026-08-04 — Cycle 089: Isolated Verovio renderer benchmark (DONE)

### Outcome

Verovio 6.1.0 passes the evidence gate for a synchronized grand-staff proof of
concept. Neothesia now has reproducible size, speed, identity-query and visual
evidence without taking a runtime dependency or mixing upstream/test artifacts
into the product repository.

### Implemented

- Pinned the Verovio upstream source in the local reference tree.
- Created an isolated test shell connected to that source by a junction.
- Added a hash-verified wrapper over the existing eight-fixture corpus.
- Measured module initialization, import, page rendering, SVG size, MIDI index
  creation and stable note time/attribute queries.
- Retained every rendered SVG page for visual inspection.
- Recorded an adapter/feature-flag/page-virtualization adoption boundary.

### Verification

- All eight MusicXML/MXL inputs loaded and rendered.
- All sampled note identities returned valid score times.
- Median/maximum whole-score rendering was 16.21/46.32 ms.
- Visual inspection covered grand staff, tuplets, slurs, dynamics, ornaments,
  pedal marks and compressed MXL.
- `git diff --check` passes.

Tooling commit: `7328549` (`tooling: benchmark Verovio score rendering`).

### Known limitations

- The 6.66 MiB module is measured uncompressed and outside application
  packaging.
- SVG totals require page virtualization rather than a whole-score live tree.
- Two invalid/reversed pedal spans trigger Verovio warnings.
- Cold application startup, memory and actual highlight paint latency remain
  part of the native display proof of concept.

## 2026-07-25 — Cycle 088: Native score compatibility analysis (DONE)

### Outcome

Practice Library can now answer whether a paired score actually corresponds to
the MIDI, not merely whether the file still exists. Analysis is asynchronous
and its evidence survives reopening the application.

### Implemented

- Added an Analyze action to the paired-score row.
- Ran the complete semantic alignment pipeline on a worker thread.
- Displayed readiness, coverage and mean confidence.
- Persisted matched/gap/inexact counts and navigation diagnostics.
- Bound the cache to both MIDI sidecar identity and score fingerprint.
- Invalidated cached analysis whenever the association changes or is removed.

### Verification

- Cached analysis round-trips through the backward-compatible sidecar.
- Re-pairing clears stale analysis before a new run.
- 138 core, 64 application and four MIDI-file tests pass.
- Corpus audit, Clippy, release build, both native smokes, formatting and diff
  checks pass with only pre-existing warnings.

Implementation commit: `b33a56c` (`feat: analyze paired score compatibility`).

### Known limitations

- The compact row does not yet open a detailed diagnostics panel.
- Thresholds still need calibration on licensed paired exports.
- Native score rendering has not started.

## 2026-07-25 — Cycle 087: Native score pairing controls (DONE)

### Outcome

Learners can pair, replace or remove a MusicXML/MXL score from the same Practice
Library Info page used for title and study metadata. The page immediately
distinguishes a verified score from a missing, invalid or replaced file.

### Implemented

- Loaded the existing score association when opening the Info editor.
- Rendered concise association health and filename.
- Made the health row itself the Replace action and added explicit Pair/Remove.
- Used an extension-filtered asynchronous native picker.
- Moved parsing, fingerprinting and atomic sidecar saving to a worker thread.
- Refreshed the library after successful pairing or removal.
- Kept cancellation on the editor and returned failures to the library message.

### Verification

- 138 core, 64 application and four MIDI-file tests pass.
- The pinned corpus, release build, Clippy, both native smokes, formatting and
  diff checks pass with only pre-existing warnings.

Implementation commit: `9052657` (`feat: pair scores from Practice Library`).

### Known limitations

- The current health row verifies parsing and content identity, not MIDI note
  alignment readiness.
- Native file dialogs are intentionally outside semantic debug automation.
- Pairing currently returns to the library; the editor must be reopened to see
  the verified label.

## 2026-07-25 — Cycle 086: Portable score association (DONE)

### Outcome

A MIDI can now own a durable, validated link to its MusicXML/MXL score in the
same sidecar already used for metadata and fingering. Moving the MIDI, score
and sidecar together preserves an adjacent-score link.

### Implemented

- Added optional score path and BLAKE3 content ID to the backward-compatible
  sidecar schema.
- Required successful MusicXML/MXL parsing before persistence.
- Stored paths relative to the MIDI directory when possible.
- Added deterministic path resolution and parse-plus-content verification.
- Reused atomic sidecar saving while preserving metadata and finger hints.

### Verification

- Valid adjacent score association saves, loads, resolves and verifies.
- Replacing the score with other valid MusicXML produces `false` verification.
- Invalid XML creates no sidecar.
- 138 core, 64 application and four MIDI-file tests pass.
- The pinned corpus, Clippy, release build, both native smokes, formatting and
  diff checks pass with only pre-existing warnings.

Implementation commit: `b6e90fc` (`feat: persist validated score associations`).

### Known limitations

- Practice Library has no native pair/replace/remove control yet.
- Verification reports errors structurally but no UI health badge consumes it.
- External absolute paths are inherently less portable.

## 2026-07-25 — Cycle 085: Alignment readiness summary (DONE)

### Outcome

The future score-import UI can present a conservative compatibility verdict
instead of asking learners to interpret raw matching internals. Navigation
failure cannot be hidden behind a superficially high percentage.

### Implemented

- Added structured Ready, Review, Poor and Blocked readiness.
- Classified incomplete navigation as Blocked before considering note metrics.
- Required 95% coverage, 85% mean confidence and zero unmatched score notes for
  Ready.
- Classified 75% coverage and 70% confidence as Review; lower evidence is Poor.
- Preserved counts for matches, score gaps, MIDI gaps and inexact PPQ-projected
  matches.
- Copied navigation diagnostics into the summary for direct UI consumption.

### Verification

- Complete repeated C alignment is Ready.
- The extra-MIDI fixture with 75% aggregate coverage is Review.
- The same-pitch 400 ms refusal fixture is Poor.
- Conflicting P1/P2 navigation is Blocked.
- A manually flagged inexact match increments the independent projection count.
- 136 core, 64 application and four MIDI-file tests pass.
- The pinned compatibility corpus, Clippy, release build, both native smokes,
  formatting and diff checks pass with only pre-existing warnings.

Implementation commit: `12c910a` (`feat: summarize score alignment readiness`).

### Known limitations

- Thresholds are conservative first policy and need calibration against paired
  real-world score/MIDI exports.
- The summary is structured core data; no picker or diagnostics panel renders
  it yet.
- Global offset/transposition estimation remains absent.

## 2026-07-25 — Cycle 084: Repeated occurrence to MIDI alignment (DONE)

### Outcome

Common MusicXML repeats and numbered endings now reach the concrete MIDI-note
matcher. Each match answers both “which written note?” and “which performed
visit?”, so repeated passages can later drive synchronized notation and
practice evidence without identity collisions.

### Implemented

- Migrated aligned and unmatched score evidence to `ScoreEventOccurrenceId`.
- Made `align_score_to_midi` build bounded performed-order plans by default.
- Projected each part's repeated pitched occurrences through the MIDI tempo map
  before using the existing gap-aware matcher.
- Preserved the explicit linear `align_score_notes` API with deterministic
  occurrence identities for callers that need written order.
- Kept rests and directions out of pitched alignment while retaining invalid or
  unprojectable pitched occurrences as explicit unmatched score evidence.
- Compared source-measure/repeat-pass visit signatures across score parts.
- Added navigation completeness and readable part/measure diagnostics to the
  aggregate result.

### Verification

- A source C repeated twice matches MIDI occurrences at 0 and 500 ms.
- Both matches retain the same stable source ID and distinct visit IDs.
- A repeated P1 and linear P2 are reported as incompatible navigation.
- Prior extra MIDI note, missing repeated note and 400 ms refusal regressions
  remain green after the identity change.
- 136 core, 64 application and four MIDI-file tests pass.
- The pinned compatibility corpus, Clippy, release build, both native smokes,
  formatting and diff checks pass with only pre-existing warnings.

Implementation commit: `5f99c7a` (`feat: align repeated score occurrences`).

### Known limitations

- Nested repeats and textual jump navigation are still diagnostic-only.
- Navigation diagnostics exist in the core but are not shown in an import UI.
- A global offset or transposition estimator is still absent.

## 2026-07-25 — Cycle 083: Repeated event occurrence timing (DONE)

### Outcome

Repeated written notes now become distinct performed occurrences on the same
clock as the MIDI player. A renderer or matcher can distinguish “the C in
measure 1 on pass one” from the same source note on pass two without inventing
a second source identity.

### Implemented

- Added cumulative performed score time to every included measure visit.
- Advanced performed time only for measures actually played, so skipped first
  endings do not consume time on later passes.
- Expanded source notes and directions into performed occurrences.
- Identified occurrences with stable source `ScoreEventId` plus the containing
  measure-visit occurrence ordinal.
- Calculated each event's measure-relative onset before adding the performed
  visit start.
- Projected repeated event starts and ends through the paired MIDI PPQ and
  tempo map.
- Preserved exact pulse projection and explicit unprojected occurrence IDs.

### Verification

- A repeated one-quarter measure creates two note occurrences with one stable
  source ID and distinct visit IDs zero and one.
- Flattened score onset advances from quarter zero to quarter one.
- At default 120 BPM, projected onset advances from 0 to 500 ms and both note
  durations are 500 ms.
- 134 core, 64 application and four MIDI-file tests pass.
- The pinned compatibility corpus, Clippy, release build, both native smokes,
  formatting and diff checks pass with only pre-existing warnings.

Implementation commit: `618c456` (`feat: project repeated score occurrences`).

### Known limitations

- Pitched repeated occurrences are not yet passed to the MIDI-note matcher.
- Multiple parts can describe conflicting repeat navigation and are not yet
  compared.
- Nested repeats and textual jump navigation retain the prior explicit
  diagnostics.

## 2026-07-25 — Cycle 082: Bounded repeat playback plans (DONE)

### Outcome

Written measure order and performed measure order are now separate, explicit
concepts. Common repeats and first/second endings can be flattened safely for
future MIDI alignment while every visit still points back to the immutable
source measure.

### Implemented

- Added configurable limits for total visits and repeat passes.
- Added deterministic measure visits containing part ID, source measure
  ordinal, occurrence ordinal and active repeat pass.
- Expanded explicit forward/backward repeats and backward repeats with an
  implicit part-start boundary.
- Honored optional MusicXML repeat counts with a conservative default of two
  passes.
- Built ending membership from start/stop/discontinue spans and filtered visits
  by active pass.
- Kept navigation controls active inside skipped endings so the second pass can
  leave a first ending and reach the second ending correctly.
- Returned diagnostics and `complete=false` for capped counts, visit limits,
  malformed/unknown endings and currently unsupported nested repeats.

### Verification

- Default repeat order is `0,1,0,1,2`.
- First/second-ending order is `0,1,2,0,1,3,4`, with pass two attached to the
  second ending.
- A 99-pass source is capped at four and then stopped at a separate three-visit
  test limit.
- A textual `finale` ending is retained as playable written order and reported
  as incomplete instead of being dropped.
- 132 core, 64 application and four MIDI-file tests pass.
- The pinned compatibility corpus, Clippy, release build, both native smokes,
  formatting and diff checks pass with only pre-existing warnings.

Implementation commit: `0f596fe` (`feat: expand MusicXML repeat playback plans`).

### Known limitations

- Nested repeats are diagnosed but not expanded.
- Da capo, dal segno, coda and fine are not modeled.
- The plan contains measure visits; repeated note/event occurrence timing is
  the next slice.
- Multiple score parts are not yet checked for equal navigation plans.

## 2026-07-25 — Cycle 081: MusicXML repeat and ending semantics (DONE)

### Outcome

Neothesia no longer discards the score facts needed to distinguish written
measure order from performed order. Repeat barlines and numbered endings are
available to learning and alignment code without introducing renderer types or
prematurely guessing an ambiguous playback path.

### Implemented

- Added notation-neutral barline, repeat and ending types to every measure.
- Preserved left/middle/right locations plus unknown future location values.
- Preserved forward/backward repeat direction, optional repeat count and winged
  engraving preference.
- Preserved ending start/stop/discontinue, original number text and parsed pass
  numbers.
- Parsed comma-separated and ranged ending values such as `1, 3-4`.
- Bounded parsed ending passes at 128 while retaining the complete raw value,
  preventing hostile ranges from creating unbounded allocations.
- Supported both empty and explicit MusicXML element forms.

### Verification

- The focused fixture retains all barline, repeat, count, wing and ending facts.
- A missing barline location follows MusicXML's right-side default.
- 128 core, 64 application and four MIDI-file tests pass.
- The pinned eight-score compatibility audit is unchanged.
- Clippy, release build, both native smoke suites, formatting and diff checks
  pass with only pre-existing warnings.

Implementation commit: `6d098f7` (`feat: preserve MusicXML repeat semantics`).

### Known limitations

- Measures are not yet expanded into performed visit order.
- Da capo, dal segno, coda and fine navigation remain outside this slice.
- Unknown ending labels remain available as raw text but cannot yet drive a
  playback plan.

## 2026-07-25 — Cycle 080: Score-note to MIDI-note alignment (DONE)

### Outcome

Imported score notes can now be associated with the exact MIDI notes used by
the player. Downstream notation, fingering and practice feedback can refer to a
stable score event and a concrete track/note occurrence without pretending
that every source pair aligns perfectly.

### Implemented

- Added public MIDI note IDs, performance-note inputs, aligned-note evidence
  and an aggregate alignment result.
- Collected non-drum MIDI notes directly from the parsed `MidiFile`.
- Converted written MusicXML pitch and octave to MIDI pitch with explicit
  out-of-range handling.
- Split matching by pitch, then used order-preserving dynamic programming with
  explicit gaps. This avoids dependence on MusicXML chord ordering and prevents
  an omitted repeated note from shifting every later occurrence.
- Refused candidates more than 250 ms apart.
- Recorded signed onset and duration deltas plus whether the score time landed
  exactly on the paired MIDI's PPQ grid.
- Calculated explainable confidence from onset, duration and projection
  precision, along with aggregate coverage and mean confidence.
- Returned unmatched stable score IDs and unmatched MIDI track/note IDs.
- Bounded dynamic-programming memory to one million cells per pitch and used a
  deterministic chronological fallback above that bound.

### Verification

- An unrelated extra MIDI F between C–D–C does not displace any correspondence.
- C–C–C in the score against first/last C in MIDI matches ordinals zero and two
  and leaves ordinal one unmatched.
- A same-pitch note 400 ms away is rejected and leaves explicit gaps on both
  sides.
- 127 core, 64 application and four MIDI-file tests pass.
- The eight-score pinned compatibility audit is unchanged.
- Clippy, release build, both native smoke suites, formatting and diff checks
  pass with only pre-existing warnings.

Implementation commit: `e2e4905` (`feat: align score notes with MIDI`).

### Known limitations

- Repeat and ending expansion is not modeled, so non-linear score playback can
  still diverge structurally from a flattened MIDI.
- The matcher does not yet estimate a global time offset or transposition.
- The 250 ms gate is a conservative import heuristic, not a learner timing
  grade.
- The alignment API is not yet surfaced in the player or notation UI.

## 2026-07-25 — Cycle 079: Score-to-MIDI time projection (DONE)

### Outcome

Semantic score events can now be placed on the exact clock used by the current
MIDI player. This is the first executable bridge between MusicXML annotations
and the falling-note/practice timeline.

### Implemented

- Added quarter-fraction projection to the MIDI tempo track.
- Rounded only when a fraction cannot land on the file's PPQ grid and exposed
  that fact as `exact_to_pulse`.
- Reused the MIDI tempo map for every timestamp.
- Added a score timeline projection containing stable event ID, timestamp,
  optional duration and exactness.
- Projected note ends independently so tempo changes inside a held note are
  handled correctly.
- Returned negative/unrepresentable events in an explicit unprojected list.
- Added deterministic timestamp/identity ordering.

### Verification

- Exact triplet and inexact septuplet pulse cases.
- Tempo change at pulse 480 with a two-quarter projection of 1.5 seconds.
- Note duration, direction offset and stable identity projection.
- Negative direction offset remains an explicit gap.
- 124 core, 64 application and four MIDI-file tests pass.
- Clippy, release build, both native smoke suites, corpus audit, formatting and
  diff checks pass with only pre-existing warnings.

Implementation commit: `b1fa679` (`feat: project score events onto MIDI time`).

### Known limitations

- Projection establishes comparable clocks but does not yet assert that a score
  note and MIDI note are the same musical event.
- Repeats/ending expansion can still make timelines structurally different.
- The current pulse rounding is reported per event; no user-facing compatibility
  summary exists yet.

## 2026-07-25 — Cycle 078: Stable score event identities (DONE)

### Outcome

Every imported note and direction now has a reproducible semantic identity.
Fingering, slurs, tuplets and pedal marks can later attach to falling notes and
practice evidence without relying on transient vector addresses or display
measure numbers.

### Identity contract

An identity contains:

- the required MusicXML part ID;
- zero-based measure ordinal in document order;
- note or direction kind;
- zero-based ordinal within that kind and measure.

Separating kind ordinals means inserting a tempo/dynamic direction does not
renumber every note in the measure. Using the document ordinal avoids
collisions when two measures display the same number.

### Verification

- Importing the same source twice yields equal IDs.
- Duplicate displayed measure “1” values remain distinct.
- Direction ordinal zero and note ordinal zero coexist without collision.
- The annotated chord/voice fixture assigns note ordinals 0 through 3.
- The full pinned compatibility corpus still imports.
- 121 core, 64 application and two MIDI-file tests pass.
- Clippy, release build, both native smoke suites, corpus audit, formatting and
  diff checks pass with only pre-existing warnings.

Implementation commit: `e418ed9` (`feat: assign stable score event identities`).

### Known limitations

- IDs are source-document identities, not content hashes; editing or reordering
  measures may legitimately change them.
- A paired MIDI note ID and confidence mapping does not exist yet.
- Repeats and endings need expansion policy before score/MIDI alignment can
  claim complete coverage.

## 2026-07-25 — Cycle 077: MusicXML pedal semantics (DONE)

### Outcome

Pedal marks are now structured score data rather than discarded graphics. This
creates the source side of a future comparison between notated pedaling and the
player's real continuous CC64 evidence.

### Implemented

- Added `PedalMark` to score directions.
- Added typed start, stop, sostenuto, change, continue, discontinue and resume
  pedal actions.
- Preserved the MusicXML overlap number.
- Preserved line, sign and abbreviated-symbol preferences.
- Kept unknown future type strings in an explicit `Other` variant.
- Parsed both spec-standard empty pedal elements and tolerant explicit forms.
- Added pedal-mark totals to the compatibility inspector.

MusicXML intentionally does not use this element for soft pedal text; that
remains representable through direction words, matching the MusicXML 4.0
definition:
<https://www.w3.org/2021/06/musicxml40/musicxml-reference/elements/pedal/>.

### Verification

- Unit fixture covers numbered damper start/change with display attributes.
- Pinned MuseScore 3.6 pedal score preserves all seven marks.
- The corpus pedal score now imports without warnings.
- 120 core, 64 application and two MIDI-file tests pass.
- Clippy, release build, both native smoke suites, corpus audit, formatting and
  diff checks pass with only pre-existing warnings.

Implementation commit: `bcfda27` (`feat: preserve MusicXML pedal marks`).

### Known limitations

- Pedal marks are not yet paired into intervals or mapped to MIDI CC64.
- Sostenuto has score semantics but no separate live-controller feedback path.
- Visual pedal lane and grand-staff symbols remain renderer work.

## 2026-07-25 — Cycle 076: MusicXML tuplet semantics (DONE)

### Outcome

Neothesia can distinguish a notated tuplet from an ordinary note with the same
elapsed duration. This supplies the information a future grand staff and rhythm
coach need to display and explain 3:2 and other irregular divisions correctly.

### Implemented

- Added `TupletRatio` with actual notes, normal notes, normal type and dot count.
- Added `TupletSpan` with start/stop/continue, number, bracket visibility and
  displayed-number preference.
- Attached both structures to semantic notes.
- Parsed explicit and self-closing tuplet span forms.
- Preserved the exact `duration/divisions` time separately.
- Added a localized warning for incomplete or zero ratio data.
- Added a tuplet-note column to `musicxml-inspect`.

### Verification

- Unit fixture: 3:2 eighth-note tuplet, exact 1/3-quarter event duration,
  numbered bracket start and stop.
- Pinned Clementi Andante: 267 tuplet notes, with prior tuplet diagnostics
  reduced to zero.
- Pinned broad MuseScore 2.3 fixture: 42 tuplet notes, with only six ornament
  diagnostics remaining.
- 120 core, 64 application and two MIDI-file tests pass.
- Clippy, release build, both native smoke suites, corpus audit, formatting and
  diff checks pass with only pre-existing warnings.

Implementation commit: `1f67fe9` (`feat: preserve MusicXML tuplet semantics`).

### Known limitations

- Nested/overlapping tuplets are represented but not yet validated as complete
  span pairs.
- Tuplet semantics are not rendered or aligned to MIDI note identities yet.
- Ornament realization and pedal directions remain deferred.

## 2026-07-25 — Cycle 075: Real-score MusicXML compatibility audit (DONE)

### Outcome

The importer is now exercised against complete piano scores and versioned
notation-tool exports, not only hand-written XML. The audit found and fixed a
real older-exporter incompatibility while turning unsupported notation into a
prioritized, reproducible queue.

### Implemented

- Added `musicxml-inspect`, a compact command-line score summary.
- Added `scripts/musicxml-compatibility.ps1`.
- Pinned OSMD revision `45ece8fa9f4ff1b51a6dfe29515e18530a64b1aa`
  and SHA-256 hashes for all eight files.
- Kept external fixtures in the temporary cache rather than inflating the
  repository; the runner reuses verified downloads.
- Accepted both `<rest/>` and the older `<rest></rest>` form.
- Reported deferred empty direction elements, including pedal marks.
- Added part identity to warning locations.
- Clarified that MusicXML `duration/divisions` already preserves exact tuplet
  event timing even though ratio/bracket notation is not modeled yet.
- Added permanent unit regressions for explicit rests and empty pedal elements.

### Corpus result

| Fixture class | Result |
| --- | --- |
| Bach BWV 846, MuseScore 1.2 | 35 measures, 750 events, no warnings |
| Clementi Allegro | 76 part-measures, 389 events, no warnings |
| Clementi Andante, MuseScore 1.2 | imports; tuplets/ornaments reported |
| MuseScore 3.6 pedal fixture | imports; six pedal measures reported |
| MuseScore 3.6 voice alignment | imports without warnings |
| MuseScore 2.3 grace notes | imports without warnings |
| Real compressed MXL | imports; nonconforming mimetype reported |
| MuseScore 2.3 broad feature fixture | imports; deferred notation summarized |

Together the corpus exercises 261 part-measures and 2,183 note/rest events.
The files come from OpenSheetMusicDisplay under its BSD-3-Clause repository
license: <https://github.com/opensheetmusicdisplay/opensheetmusicdisplay>.

### Verification

- Hash-verified corpus runner passes on all eight files.
- 119 core, 64 application and two MIDI-file tests pass.
- Clippy, release build, both native smoke suites, formatting and diff checks
  pass with only pre-existing warnings.

Implementation commit: `bfa1e05` (`test: audit real MusicXML piano scores`).

### Known limitations

- The current corpus is cross-version but still mostly MuseScore-derived.
- Dorico and Finale fixtures need explicit redistribution provenance before
  becoming stable assertions.
- Tuplet bracket/ratio, pedal directions and ornaments are diagnosed but not
  yet available to learning features.

## 2026-07-25 — Cycle 074: Safe compressed MusicXML input (DONE)

### Outcome

Neothesia can now open the compressed `.mxl` form used by notation applications
without extracting files to disk or trusting archive paths. The same semantic
importer receives both raw and compressed scores.

### Implemented

- Detected ZIP signatures independently of filename extensions.
- Read `META-INF/container.xml` and selected the declared full-score root.
- Added stored and Deflate decompression with a deliberately narrow ZIP feature
  set.
- Added limits for archive bytes, entry count, container bytes and decompressed
  MusicXML bytes.
- Rejected absolute paths, parent traversal and Windows separators in declared
  root files.
- Added typed errors for invalid archives, missing containers/root files,
  unsafe paths and size limits.
- Treated the standard mimetype marker as a conformance diagnostic rather than
  an unnecessary compatibility blocker.

### Verification

- Round-tripped the annotated two-staff piano fixture through a Deflate MXL.
- Verified missing-mimetype warning behavior.
- Verified traversal rejection before score lookup.
- Verified a ZIP without `META-INF/container.xml` is not treated as a score.
- 117 core, 64 application and two MIDI-file tests pass.
- Clippy, release build, both native smoke suites, formatting and diff checks
  pass with only pre-existing warnings.

Implementation commit: `49843b7` (`feat: import compressed MusicXML safely`).

### Known limitations

- `score-timewise` remains an explicit unsupported error.
- Only the root score is imported; alternate PDF/audio renditions and separate
  part rootfiles are intentionally ignored.
- Cross-exporter real-world fixture coverage is the next compatibility gate.

## 2026-07-25 — Cycle 073: MusicXML semantic foundation (DONE)

### Outcome

Neothesia now has a score-data boundary capable of carrying piano-learning
meaning that MIDI omits. Future sheet rendering, phrase-aware fingering and
score/performance comparison can share this model without depending on one XML
parser or engraving engine.

### Research and architecture decision

- MusicXML 4.0 is an interchange format for notation, education and performance
  software, but its complete schema is intentionally much larger than this
  first learning-focused slice:
  <https://www.w3.org/2021/06/musicxml40/>
- Verovio is a portable open-source engraving engine that accepts MusicXML and
  emits SVG. Its official documentation also makes importer-path and compressed
  MXL constraints explicit:
  <https://book.verovio.org/toolkit-reference/input-formats.html>
- MuseScore offers broad MusicXML import and SVG/PDF export, including command
  line conversion, but is an external desktop application rather than a small
  embeddable boundary:
  <https://handbook.musescore.org/file-management/working-with-musicxml-files>

The selected sequence is: own the small semantic model, measure Verovio in an
isolated spike, then decide how to display SVG in the native wgpu application.
Native staff engraving from scratch is not accepted into the plan.

### Implemented

- Added a streaming MusicXML importer built on `quick-xml`.
- Added score, part, measure, note, direction, attribute, pitch and span types.
- Represented score positions as reduced rational quarter-note values.
- Imported two-staff piano clefs, key and time signatures, voices, rests,
  chords, directions, dynamics, tempo, fingerings, ties, slurs and
  articulations.
- Implemented MusicXML `backup` and `forward` cursor behavior for polyphonic
  measures.
- Preserved XML entities and selected only typed composer creators.
- Rejected timewise input explicitly and reported recognized deferred notation
  such as ornaments, tuplets, transposition and pedal markings.
- Kept compressed MXL and rendering outside this first slice.

### Verification

- Added tests for a two-staff, two-voice, annotated piano measure.
- Verified exact chord onset, second-voice rewind and rational-time reduction.
- Verified typed creator and escaped-title import.
- Verified explicit timewise rejection and deferred-notation warnings.
- 113 core, 64 application and two MIDI-file tests pass.
- Clippy, release build, both native smoke suites, formatting and diff checks
  pass with only pre-existing warnings.

Implementation commit: `7448f4b` (`feat: add MusicXML score semantics`).

### Known limitations

- Only uncompressed `score-partwise` input is accepted.
- The importer is deliberately not a complete MusicXML round-trip editor.
- Repeats, endings, tuplets, transposition, pedal directions and ornaments need
  explicit model work before learning features consume them.
- There is not yet a mapping between score identities and the MIDI performance
  used by the falling-note player.
- No engraving renderer has entered the application dependency graph.

## 2026-07-25 — Cycle 072: Rapid repeated-note alternation (DONE)

### Outcome

The suggestion model no longer gives the same repeated-note advice at every
speed. Slow repeats remain on one finger; a sustained fast run can alternate
adjacent fingers so the key has a clearer chance to release between attacks.

### Research boundary

- Indiana University Press *Class Piano* lists changing fingers on a repeated
  note as a general fingering device, while warning that multiple fingerings
  may be acceptable and should be tested for the individual hand:
  <https://publish.iupress.indiana.edu/read/class-piano/section/d57966c3-dc98-4825-b143-0f8c5f58173f>
- Youmee Kim's Ohio State dissertation describes alternating fingers as
  usually easier for fast repeated notes but calls out black-key, melody and
  body-shift exceptions:
  <https://etd.ohiolink.edu/acprod/odb_etd/ws/send_file/send?accession=osu1199061624&disposition=inline>

These sources support a conservative option, not automatic editorial truth.

### Implemented

- Added a three-note minimum for recognizing a repeated run.
- Added a 250 ms maximum inter-onset interval.
- Replaced the ordinary repeated-note transition only inside that run.
- Penalized same-finger reuse and favored the smallest nonzero finger change.
- Preserved the balanced phrase-start cost, producing 3–2–1–2 on a four-note
  white-key fixture.
- Kept slow repeats and manual anchors unchanged.
- Added `RapidRepeatedNote` with a plain-language explanation and 76%
  confidence tier.
- Completed `MUS-003I`.

### Verification

- A 120 ms four-note C run produces 3–2–1–2.
- All three transitions carry `RapidRepeatedNote`.
- The prior 500 ms repeat test still keeps finger 3.
- Full workspace tests, Clippy, release build, formatting and diff checks pass.
- Whole-chord sidecar and Technique Studio real-process fixtures pass.

All desktop gates passed: two MIDI-file tests, 110 core tests and 66
application tests. Both real-process smokes pass. Clippy and release builds
report only the repository's pre-existing platform-helper and `unused_mut`
warnings.

Implementation commit: `b6a2332`
(`feat: alternate rapid repeated notes`).

### Known limitations

- The threshold is a transparent conservative heuristic, not tempo-, skill- or
  key-action-calibrated.
- Black-key width, accents and phrase role do not yet change the pattern.
- The model offers one alternating path rather than same-finger and alternating
  alternatives side by side.

## 2026-07-25 — Cycle 071: Melody over a held chord (DONE)

### Outcome

A chord-plus-melody texture no longer loses hand occupancy at the first
single-note onset. The melody uses only fingers physically free around the held
chord and still progresses like a melody from one new note to the next.

### Implemented

- Detected a single-note tail immediately following a modeled chord run.
- Reused the chord's selected fingers and MIDI note-off times for every tail
  onset while at least one chord tone remains held.
- Applied held-finger uniqueness and right/left ordering to each new note.
- Added ordinary melodic transition cost after the tail's first note.
- Checked overlapping tail notes against one another as an additional held
  constraint.
- Kept `HeldChordPosition` as the visible explanation.
- Completed `MUS-003F2A2`; explicit on-key substitution remains F2B.
- Added a retry wrapper only for read-only debug snapshots. It retries missing
  properties/transport interruptions up to five times, while state-changing
  action commands are never replayed.

### Verification

- In the focused test, low C remains held while A then B enter.
- Both melody notes avoid/cross neither C's finger.
- B uses a higher finger than A, proving tail-to-tail melodic progression is
  active rather than independent free-finger selection.
- The snapshot-retry harness passes both whole-chord and Technique Studio
  real-process fixtures.
- Full workspace tests, Clippy, release build, formatting and diff checks pass.

All desktop gates passed: two MIDI-file tests, 109 core tests and 66
application tests. Both real-process smokes pass. Clippy and release builds
report only the repository's pre-existing platform-helper and `unused_mut`
warnings.

Implementation commit: `eb0abdd`
(`feat: guide melody around held chords`).

### Known limitations

- The tail context comes from the immediately preceding chord run; it is not a
  general polyphonic voice tracker across arbitrary interleaving.
- Intentional finger substitution on one continuously depressed key remains
  unrepresented.
- Pedal-sustained sound after key release does not occupy a finger, correctly
  for anatomy but without separate acoustic-voice modeling.

## 2026-07-25 — Cycle 070: Held-tone finger occupancy (DONE)

### Outcome

Chord advice now knows that a finger remains physically busy until its MIDI
note ends. A following shape cannot assign that same finger to another pitch or
cross new fingers through the held hand position.

### Implemented

- Added `end` to `FingeringNote` and populated it from parsed MIDI note-off
  timing.
- Checked every still-sounding prior chord tone against every new
  pitch/finger pair.
- Rejected same-finger/different-pitch reuse while the earlier key remains
  down.
- Enforced pitch/finger ordering around held tones for right and left hands.
- Split a chord run when no candidate pair can satisfy the held constraints,
  leaving the impossible onset without false precision.
- Added `HeldChordPosition`: “uses fingers that remain free while earlier chord
  tones are held.”
- Kept ordinary rearticulated common-tone voice leading unchanged when the
  prior note has already ended.
- Completed `MUS-003F2A`; retained actual finger substitution as F2B.

### Verification

- A held low C occupies its selected right-hand finger through a later A–B
  chord; both new fingers remain distinct and physically above it.
- The new notes receive the held-position explanation.
- A direct left-hand mirror test accepts lower pitches on fingers 5–3 beneath a
  held high pitch on finger 1 and rejects finger-1 reuse.
- Thirteen fingering-domain tests pass alongside all existing suites.
- Full workspace tests, Clippy, release build, formatting and diff checks pass.
- Whole-chord sidecar and Technique Studio real-process fixtures pass.

All desktop gates passed: two MIDI-file tests, 108 core tests and 66
application tests. Both real-process smokes pass. Clippy and release builds
report only the repository's pre-existing platform-helper and `unused_mut`
warnings.

Implementation commit: `13fa40f`
(`feat: respect held chord fingers`).

### Known limitations

- Occupancy is derived from key note-off, not sustain-pedal acoustic duration;
  the finger is free once the key is released even if the damper remains up.
- The model does not yet represent transferring one depressed key between
  fingers.
- Held-state constraints currently connect consecutive chord onset groups; a
  single-note onset starts the existing melodic segment.

## 2026-07-25 — Cycle 069: Common-tone chord voice leading (DONE)

### Outcome

Consecutive chord suggestions now behave like a phrase of hand shapes rather
than unrelated screenshots. When two ergonomic choices are close, a common
tone can remain under the same finger and the preview explains that continuity.

### Implemented

- Generated every legal candidate for each consecutive chord onset.
- Ran dynamic programming across the candidate sequence.
- Kept the existing full vertical shape cost at every state.
- Added a transition cost for changing a common pitch's finger and moving one
  reused finger between pitches.
- Added `ChordConnection` with the explanation “keeps a common chord tone under
  the same finger.”
- Split candidate runs around six-note, duplicate-pitch or anchor-conflict
  groups, leaving those groups empty without poisoning later valid chords.
- Calibrated common-tone change to a three-point penalty after testing stronger
  values: stronger weights could choose a cramped 1–2–3 solely for continuity,
  which is the wrong pedagogical trade.
- Completed `MUS-003F1`; retained explicit held-note substitution and
  phrase/articulation context.

### Verification

- C–E–G → C–E–A yields 1–3–5 → 1–3–5 and marks retained C/E tones.
- In isolation, E–G–C assigns G=2 while D–G–B assigns G=3. Joint planning picks
  the still-ergonomic 1–3–5 option for both and keeps G=3, proving the
  transition term changes a real decision rather than only labeling a
  coincidental match.
- Twelve fingering-domain tests pass alongside every existing core/application
  test.
- Full workspace tests, Clippy, release build, formatting and diff checks pass.
- Whole-chord and Technique Studio real-process fixtures pass.

All desktop gates passed: two MIDI-file tests, 107 core tests and 66
application tests. Both real-process smokes pass. Clippy and release builds
report only the repository's pre-existing platform-helper and `unused_mut`
warnings.

Implementation commit: `6c53e86`
(`feat: preserve common chord fingers`).

### Known limitations

- Consecutive means adjacent chord onset groups on one track; a monophonic note
  between them starts a new segment.
- Note duration, key release, sustain pedal and slur data do not yet determine
  whether a common tone is physically held.
- Repeated-note alternation and deliberate finger substitution are not modeled.

## 2026-07-25 — Cycle 068: Independent right/left-hand spans (DONE)

### Outcome

Fingering advice can now reflect asymmetric reach. A learner may select, for
example, Large for the right hand and Compact for the left; suggestions use the
profile belonging to the current hand track.

### Implemented

- Added optional right- and left-hand span overrides to versioned playback
  settings.
- Preserved the previous shared `hand_span_profile` as the migration fallback.
- Split the Settings row into **Right Hand Span** and **Left Hand Span**.
- Added hand-aware config getters/setters.
- Routed suggestion requests through the classified track's hand-specific
  value.
- Made the shared compatibility setter clear both overrides, providing a
  deterministic reset to one common value.
- Completed `MUS-003D3`.

### Verification

- Shared Compact resolves to Compact for both hands before any override.
- Right=Large can coexist with Left=Compact.
- The asymmetric pair survives config serialization/deserialization.
- A literal old playback RON containing only `hand_span_profile: Compact`
  parses with both overrides absent, preserving fallback behavior.
- Full workspace tests, Clippy, release build, formatting and diff checks pass.
- Whole-chord FingeringFixture and Technique Studio native fixtures both pass.

All desktop gates passed: two MIDI-file tests, 105 core tests and 66
application tests. Both real-process smokes pass. Clippy and release builds
report only the repository's pre-existing platform-helper and `unused_mut`
warnings.

Implementation commit: `1cc9c37`
(`feat: personalize hand spans independently`).

### Known limitations

- Compact/Standard/Large are coarse categories rather than measured
  millimeters or a per-finger anatomy map.
- The legacy shared field remains serialized for backward compatibility.
- Suggestions still require an explicit right/left track classification.

## 2026-07-25 — Cycle 067: Whole-chord preview and acceptance (DONE)

### Outcome

A chord suggestion is now a real chord workflow. Pressing G displays all
proposed digits on the simultaneous notes; Enter accepts the visible shape in
one atomic save instead of forcing the learner to repeat preview/accept for
every chord tone.

### Implemented

- Replaced the editor's single pending suggestion with a selected transaction
  containing multiple exact-note assignments.
- Generalized the note-label overlay from one preview digit to a keyed preview
  map.
- Kept every pending digit cyan; retained the centered dot for an unassigned
  selection and white/gold saved-guidance semantics.
- Built a low-to-high `1–3–5`-style shape description in the preview toast.
- Used a single `save_song_fingerings` call for the complete group.
- Updated all live song and label mappings only after that save succeeds.
- Kept direct 1–5 input and Delete as intentional one-note editing operations.
- Advanced selection to the end of the accepted group and cleared transient
  previews.
- Added `suggested_fingering_count` to the loopback-only semantic snapshot.
- Strengthened the real-process fixture to require three pending assignments,
  three live hints and exactly three serialized hints.
- Completed `MUS-003G`.

### Verification

- Before Enter, the real app reports finger 1, three pending assignments and
  78% confidence for C–E–G.
- After Enter, live guidance and manual-hint counts are both three.
- The adjacent sidecar contains exactly three track/note/finger records.
- Full workspace tests, Clippy, release build, formatting and diff checks pass.
- Technique Studio's two-hand primary-chord fixture remains green.

All desktop gates passed: two MIDI-file tests, 104 core tests and 66
application tests. Both real-process smokes pass. Clippy and release builds
report only the repository's pre-existing platform-helper and `unused_mut`
warnings.

Implementation commit: `2db5a0e`
(`feat: preview and accept full chord fingerings`).

### Known limitations

- Preview acceptance is all-or-nothing for one onset; selecting a different
  note clears the pending transaction.
- The next target remains the final chord tone when the chord is the final event
  in the score because there is no later note to advance to.
- Chord-to-chord voice leading and held-note substitutions remain `MUS-003F`.

## 2026-07-25 — Cycle 066: Safe polyphonic chord fingering (DONE)

### Outcome

Finger edit can now help with an actual same-hand chord instead of refusing
every simultaneous onset. The preview proposes a whole ordered hand shape,
explains whether it is ordinary or unusually wide, and still writes nothing
until the learner accepts an individual note.

### Implemented

- Grouped simultaneous notes into independent vertical chord states.
- Supported two through five distinct pitches on a classified hand track.
- Enumerated all finger subsets while enforcing ascending 1→5 physical order
  in the right hand and 5→1 in the left.
- Scored each legal shape by proportional pitch placement, black-key costs and
  the configured hand-span profile.
- Preserved exact manual hints as hard constraints during enumeration.
- Added `ChordShape` and `WideChordShape` reasons with 78% and 50% communication
  tiers.
- Added an explicit “do not force the reach” explanation when the chord exceeds
  the selected comfortable thumb-to-pinky span.
- Rejected six-note groups, duplicate pitches and contradictory anchors instead
  of inventing an impossible assignment.
- Replaced the native fingering fixture's right-hand single note with a
  same-track C–E–G triad and navigated to that chord before preview.
- Completed `MUS-003D` / `MUS-003D2`; retained progression/held-note work as
  `MUS-003F`.

### Verification

- Domain tests prove right-hand 1–3–5 and left-hand 5–3–1 for C–E–G.
- Anchor-preservation and impossible-anchor refusal pass.
- Six-note and duplicate-pitch refusal pass.
- Compact-profile octave dyads use outer fingers with the low-confidence wide
  warning.
- The real-process FingeringFixture previews finger 1 at 78%, accepts it and
  persists exactly one exact-note hint.
- Full workspace tests, Clippy, release build, formatting and diff checks pass.
- The Technique Studio two-hand G-sharp primary-chord fixture still completes
  both passes and persists its attempt.

All desktop gates passed: two MIDI-file tests, 104 core tests and 66
application tests. Both real-process smokes pass. Clippy and release builds
report only the repository's pre-existing platform-helper and `unused_mut`
warnings.

Implementation commit: `ee3ad43`
(`feat: suggest safe chord fingerings`).

### Known limitations

- Each chord is optimized vertically; preceding/following chord voice leading
  does not yet affect the shape.
- Held notes, finger substitution, repeated-note alternation and redistribution
  between hands are not modeled.
- Confidence remains a transparent heuristic tier, not a probability.

## 2026-07-25 — Cycle 065: Personalized hand-span profiles (DONE)

### Outcome

Fingering previews no longer assume every learner has the same reach. A learner
can select Compact, Standard or Large in Practice settings, and wide melodic
passages are planned against that comfortable span while manual and reviewed
fingerings remain authoritative.

### Implemented

- Added a serializable `HandSpanProfile` domain type with Compact (seventh),
  Standard (octave) and Large (ninth) choices.
- Parameterized the dynamic-programming transition cost with per-finger-gap
  comfortable spans.
- Retained the original public suggestion function as a Standard-profile
  compatibility wrapper.
- Added a persistent **Hand Span** selector to the Practice settings section.
- Defaulted old settings files to Standard without a schema reset.
- Routed live G-key and semantic-driver previews through the configured
  profile.
- Proved the setting changes planning on wide phrases rather than only changing
  interface text.
- Split completed melodic personalization (`MUS-003D1`) from the honest
  remaining chord-state task (`MUS-003D2`).
- Normalized JSON pitches to integers in the Windows PowerShell native smoke;
  this fixed a false failure where visible pitches 44 and 68 compared unequal
  solely because of runtime numeric types.

### Verification

- A focused domain test finds a wide phrase whose Compact and Large plans
  differ.
- Configuration tests prove Standard migration and profile mutation.
- Full workspace tests, Clippy, release build, formatting and diff checks pass.
- The real-process FingeringFixture previews finger 3 at 65%, accepts it and
  persists exactly one hint.
- The Technique Studio fixture verifies the G-sharp two-hand tonic pitches 44
  and 68, six-note matching, two passes and persisted history.

All desktop gates passed: two MIDI-file tests, 101 core tests and 66
application tests. Both real-process smokes pass. Clippy and release builds
report only the repository's pre-existing platform-helper and `unused_mut`
warnings.

Implementation commit: `581cff8`
(`feat: personalize fingering hand spans`).

### Known limitations

- One profile currently applies to both hands; asymmetric reach is not yet
  represented.
- The three choices are conservative categories, not a medical or ergonomic
  assessment.
- Chords and held-note substitutions still deliberately receive no automatic
  suggestion until `MUS-003D2` provides a vertical hand-shape model.

## 2026-07-25 — Cycle 064: Visual fingering selection and preview (DONE)

### Outcome

The finger editor now communicates on the music itself. A learner can see which
waterfall note is selected and distinguish an unaccepted suggestion from saved
guidance without relying only on a fading status message.

### Implemented

- Expanded the cached finger glyph set with neutral/sharp selection markers.
- Tracked one transient exact-note selection independently of saved hints.
- Rendered an unassigned selected note as a cyan dot.
- Rendered a pending suggested finger as a cyan digit.
- Preserved an existing saved digit under selection while coloring it cyan.
- Returned accepted/unselected hints to white.
- Kept reviewed hand turns gold and lower in color precedence than selection.
- Cleared selection and disabled an otherwise empty guidance layer on editor
  exit.
- Documented the cyan/white/gold visual language.
- Completed `MUS-003E`.

### Verification

- A focused renderer-domain test covers no glyph, selection marker, stored
  digit, preview override and ordinary saved digit.
- The native FingeringFixture still previews finger 3, accepts it and persists
  one exact-note hint.
- Full workspace tests, Clippy, release build, Technique Studio smoke,
  formatting and diff checks pass.

All desktop gates passed: two MIDI-file tests, ninety-nine core tests and
sixty-six application tests. Both real-process smokes pass. Clippy and release
builds report only the repository's pre-existing platform-helper and
`unused_mut` warnings.

Implementation commit: `de7a2f0`
(`feat: visualize fingering selection previews`).

### Known limitations

- The marker is a centered dot rather than a full note-outline shader.
- Color is fixed and does not yet expose a high-contrast/color-blind palette.

## 2026-07-25 — Cycle 063: Explainable fingering suggestions (DONE)

### Outcome

Finger edit can now offer a reasoned starting point without taking authorship
away from the learner. `G` previews one suggestion and explains it; Enter
accepts, while direct 1–5 input remains equally available.

### Implemented

- Added `neothesia_core::fingering` as a window/audio/storage-free domain
  module.
- Used dynamic programming across all five fingers to minimize a documented
  transition/static cost.
- Distinguished right- and left-hand natural motion.
- Penalized changed-finger repeats, contrary motion without a crossing,
  excessive finger-pair spans, same-finger pitch changes and awkward black-key
  thumb/pinky use.
- Recognized in-position, thumb-under, finger-over and position-shift reasons.
- Added confidence tiers while explicitly avoiding probability language.
- Used neighboring manual hints as hard anchors.
- Left simultaneous chord onsets and hand-ambiguous tracks without a
  suggestion.
- Added G preview and Enter acceptance inside paused Finger edit.
- Preserved the existing atomic save path only after acceptance.
- Exposed suggestion finger and confidence through the debug snapshot.
- Upgraded the real-process fixture to prove preview precedes persistence.
- Added a research/behavior guide with primary paper and university-teaching
  references.
- Completed the explainable prototype `MUS-003`; retained personalized
  polyphonic expansion as `MUS-003D`.

### Verification

- Six domain tests cover five-note hand positions, C-major thumb turn, repeats,
  anchors, chords, black keys, explanations and confidence bounds.
- The FingeringFixture real process reports suggested finger 3 at 65%, then
  persists exactly one accepted hint.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build -p neothesia --release`
- `scripts/debug-practice-smoke.ps1 -ExerciseFixture`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, ninety-eight core tests and
sixty-six application tests. Both real-process smokes pass. Clippy and release
builds report only the repository's pre-existing platform-helper and
`unused_mut` warnings.

Implementation commit: `8a85c3c`
(`feat: add explainable fingering suggestions`).

### Known limitations

- Confidence is a transparent heuristic tier, not learned calibration.
- Hand span is a conservative fixed profile.
- Chords, held-note substitutions, phrasing and articulation are outside this
  prototype.
- The lowest-cost path is one plausible option, not the unique correct
  fingering; teacher and learner edits remain authoritative.

## 2026-07-25 — Cycle 062: Portable manual finger hints (DONE)

### Outcome

Imported repertoire can now carry learner-reviewed fingering without changing
the MIDI. Annotation happens directly against the falling notes: pause, step
through the score, press 1–5, and continue. The guidance reappears when the
piece is reopened or moved together with its sidecar.

### Implemented

- Added `FingerHint { track_id, note_index, finger }` to the version-1 sidecar.
- Validated finger range and canonicalized duplicate exact-note hints.
- Preserved fingerings during metadata saves and metadata during fingering
  saves.
- Loaded valid hints automatically when constructing an imported `Song`.
- Included track identity in renderer keys, removing ambiguity between
  same-time/same-pitch notes on different tracks.
- Generalized exercise-only maps and labels into shared finger guidance.
- Added a sequential editor over visible non-drum notes in stable score order.
- Started at the nearest score time, paused playback and sought each selected
  note to the keyboard line.
- Added low-to-high chord navigation, 1–5 assignment, automatic advance and
  Delete/Backspace clearing.
- Saved every edit atomically before changing live state.
- Added a persistent top-bar edit button, Ctrl+I shortcut and detailed target
  toasts.
- Kept generated reviewed exercises read-only until exported.
- Renamed Settings copy from Exercise Fingerings to Finger Guidance.
- Added semantic automation actions and snapshot counts.
- Added a real-process fingering fixture that verifies UI state and sidecar
  bytes.
- Repositioned persistent output/panic pills while the top bar is expanded to
  prevent control overlap.
- Completed `MUS-002`.

### Verification

- Eight library-sidecar tests include mutual preservation and invalid rollback.
- Song reload maps an imported exact-note hint.
- Player tests cover score ordering, navigation, pitch labels and replacement.
- `scripts/debug-practice-smoke.ps1 -FingeringFixture` passes with one live
  manual hint and an adjacent sidecar containing finger 1.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build -p neothesia --release`
- `scripts/debug-practice-smoke.ps1 -ExerciseFixture`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, ninety-two core tests and
sixty-six application tests. Both real-process smokes pass. Clippy and release
builds report only the repository's pre-existing platform-helper and
`unused_mut` warnings.

Implementation commit: `f012ef4`
(`feat: add portable manual finger hints`).

### Known limitations

- The selected unassigned note is identified by score position and toast, not
  yet by a dedicated colored outline.
- Manual hints do not infer thumb-under/finger-over turn markers.
- The editor follows visible tracks; temporarily hidden parts are intentionally
  excluded.
- There is no automatic fingering suggestion yet; that remains `MUS-003`.

## 2026-07-25 — Cycle 061: Native song metadata editor (DONE)

### Outcome

Portable repertoire metadata is now a learner-facing workflow rather than a
file format. A piece can be renamed, credited, tagged and annotated from
Practice Library, then found immediately through those values.

### Implemented

- Added an **Info** button for every available non-generated library MIDI.
- Added a focused Song information page with seven editable fields.
- Added mouse selection and complete keyboard navigation/editing.
- Parsed comma-separated tags while retaining free text for study notes.
- Routed saving through `save_song_metadata`, preserving content binding and
  atomic replacement.
- Kept failed saves on the editor page with an actionable error.
- Returned successful saves to Practice Library and forced a fresh scan.
- Cleared unsaved editor state on Cancel, Escape and mouse back.
- Completed `LIB-002C` and parent `LIB-002`.

### Verification

- Two editor tests cover field conversion/tag parsing and bounded navigation.
- Existing sidecar tests exercise the persistence path used by the editor.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build -p neothesia --release`
- `powershell -ExecutionPolicy Bypass -File
  .\scripts\debug-practice-smoke.ps1 -ExerciseFixture`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, ninety core tests and sixty-two
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `8fc184e`
(`feat: add native song metadata editor`).

### Known limitations

- The editor is single-line; long study notes scroll conceptually through the
  stored value but are visually truncated in the field button.
- Generated Technique Studio exercises have stable source metadata but no
  adjacent MIDI until exported, so their Info action remains unavailable.
- Duplicate MIDI copies can still have conflicting sidecars; scan precedence
  is deterministic but conflict resolution is not interactive.

## 2026-07-25 — Cycle 060: Portable song metadata foundation (DONE)

### Outcome

Repertoire information can now travel with a MIDI without modifying the music
file or depending on its filename. Practice Library can show a clean title and
credit and find a piece by composer, performer, collection, difficulty, tag or
study note.

### Implemented

- Added `SongMetadata` with optional title, artist, composer, collection,
  difficulty, tags and notes.
- Added the version-1 `filename.mid.neothesia.ron` container.
- Stored and checked the MIDI's BLAKE3 content identity in every sidecar.
- Trimmed scalar fields and sorted/deduplicated tags case-insensitively.
- Added same-directory, flush-before-replace atomic persistence.
- Loaded matching sidecars during recursive library scans.
- Ignored malformed, unsupported and content-mismatched sidecars while keeping
  their MIDI playable.
- Merged duplicate-content metadata deterministically by sorted source path.
- Replaced filename with metadata title and showed artist/composer credit.
- Expanded library search over every metadata field.
- Added loaded/invalid sidecar counts to scan feedback.
- Documented the file format and portable identity behavior.
- Completed `LIB-002A` and `LIB-002B`.

### Verification

- Six focused library tests cover scan/deduplication, path search, normalized
  sidecar round trip, mismatch rejection, duplicate merge and atomic replace.
- A menu test proves all query terms can span title, composer, artist, tag and
  source path.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build -p neothesia --release`
- `powershell -ExecutionPolicy Bypass -File
  .\scripts\debug-practice-smoke.ps1 -ExerciseFixture`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, ninety core tests and sixty
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `763030c`
(`feat: add portable song metadata sidecars`).

### Known limitations

- `LIB-002C` and parent `LIB-002` remain open until Practice Library has a
  native editor.
- Metadata follows a move only when the sidecar is moved with its MIDI.
- Conflicting scalar values use deterministic first-path precedence; there is
  not yet an in-app conflict resolver.
- Finger annotations remain a separate future sidecar extension.

## 2026-07-25 — Cycle 059: Deterministic player clock (DONE)

### Outcome

Player timing tests now state and enforce the intended clock boundary directly.
Thirty simulated seconds pass instantly in the regression suite, and pausing
does not leak any of that duration into scoring.

### Implemented

- Audited `neothesia_core::practice`, `MidiPlayer` and
  `midi_file::PlaybackState`.
- Confirmed that scoring time and score playback both use explicit
  caller-provided deltas.
- Added a player regression that:
  - advances the session clock by 250 ms;
  - pauses and supplies a 30-second delta without changing the clock;
  - resumes and advances by exactly another 125 ms.
- Completed `QA-010`.

### Verification

- `cargo test -p neothesia --bin neothesia
  practice_clock_is_delta_driven_and_freezes_while_paused`
- Source search finds no sleep, `Instant::now` or `SystemTime::now` in the
  practice matcher, MIDI player or playback timeline.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build -p neothesia --release`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, eighty-six core tests and sixty
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `3ad052b`
(`test: enforce deterministic player clock`).

### Known limitations

- Render-frame timing still originates from the application event loop, as it
  should in production; deterministic domain/player tests bypass that boundary.
- Toast expiry uses a wall clock but is presentation-only and does not affect
  playback, scoring or practice history.

## 2026-07-25 — Cycle 058: Pianoteq route diagnostics (DONE)

### Outcome

The external Pianoteq workflow now has a truthful machine-checkable boundary.
A single command can distinguish an absent virtual cable, a stale Neothesia
selection and an endpoint Windows exposes but cannot open. It never claims
that inaudible, doubled, delayed or unstable audio has passed.

### Implemented

- Added the `midi-diagnostics` console binary beside the graphical app.
- Enumerated input and output names through the production `midi-io` backend.
- Loaded Neothesia's saved input/output selection from the normal settings.
- Added exact output matching, required saved-selection matching and a
  no-message open probe.
- Added `scripts/check-pianoteq-route.ps1`, including optional verification
  that Pianoteq is running.
- Expanded the external-routing guide with commands, result semantics,
  troubleshooting order and the physical-audio boundary.
- Split `AUD-010` into completed diagnostic infrastructure (`AUD-010A`) and
  pending successful local cable configuration (`AUD-010B`).

### Verification

- `cargo test -p neothesia --bin midi-diagnostics`
- `cargo run -p neothesia --bin midi-diagnostics --`
- An exact `Keystation 61 MK3` output probe succeeds without sending MIDI.
- An exact `Neothesia to Pianoteq` probe fails with exit code 1 because the
  endpoint is absent.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build -p neothesia --release`
- `powershell -ExecutionPolicy Bypass -File
  .\scripts\debug-practice-smoke.ps1 -ExerciseFixture`
- `cargo fmt --all`
- `git diff --check`

The local machine exposes two Keystation inputs, two Keystation outputs and
Microsoft GS Wavetable Synth. Pianoteq 6 STAGE is installed at
`C:\Program Files\Modartt\Pianoteq 6 STAGE`, but no virtual MIDI cable is
installed or visible, and the saved Neothesia output is `Buildin Synth`.
All desktop gates passed: two MIDI-file tests, eighty-six core tests and
fifty-nine application tests including the two diagnostic argument tests.
Clippy and release builds report only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `368936f`
(`feat: add Pianoteq route diagnostics`).

### Known limitations

- `AUD-010`, `AUD-010B` and `AUD-012` remain open: installing/configuring a
  third-party virtual cable is external machine setup, not a repository edit.
- An openable sender endpoint does not reveal Pianoteq's selected MIDI input.
- Process detection does not prove that Pianoteq generated audible output.
- The 30-minute latency, dynamics, pedal and stability acceptance run still
  requires the physical keyboard, Pianoteq and speakers.

## 2026-07-25 — Cycle 057: Primary-chord fingerings (DONE)

### Outcome

All three Technique Studio pattern families now display reviewed finger
guidance. Primary-chord practice assigns every simultaneously played triad note
to a finger without pretending that blocked hand shapes are scale crossings.

### Implemented

- Added right-hand 1–3–5 and left-hand 5–3–1 to every generated root-position
  triad.
- Applied the assignment to major I–IV–V–I and minor i–iv–V–i.
- Preserved hand filtering, direction, octave count and repetition behavior.
- Emitted explicit false crossing flags for all blocked notes.
- Upgraded native smoke to G♯ minor primary chords and asserted zero turns.
- Completed `EX-003J` and the parent `EX-003`.

The root-position convention was checked against
[Baylor Piano Basics](https://openbooks.library.baylor.edu/pianobasics/chapter/triad-inversions/)
on 2026-07-25.

### Verification

- C-major one-octave up/down produces seven triads with exact right/left
  assignments.
- Every chord crossing flag remains false.
- Real-process G♯ minor primary-chord smoke matches a six-note two-hand target,
  toggles guidance and verifies preset/settings/history persistence.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, eighty-six core tests and
fifty-seven application tests. Clippy and release builds report only the
repository's pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `81dd7be`
(`feat: add primary chord fingerings`).

### Known limitations

- Primary chords remain root-position blocks rather than voice-led inversions.
- There is no cadence-specific hand movement explanation yet.
- Fingerings remain pedagogical defaults rather than anatomy-specific variants.

## 2026-07-25 — Cycle 056: Hand-turn highlighting (DONE)

### Outcome

Finger guidance now calls attention to the notes that require preparation.
Thumb-under and finger-over landing numbers are gold, so the learner can see
the technical turn rather than reading every finger number as equally difficult.

### Implemented

- Derived turn landings from pitch direction, finger direction and hand.
- Covered both ascending and descending motion for right and left hands.
- Stored turn flags alongside the existing finger sequences.
- Mapped flags to exact generated MIDI timestamp/pitch/channel identities.
- Added a backward-compatible renderer constructor and a richer guidance path.
- Rendered turn numbers in warm gold while retaining white ordinary numbers.
- Added a Technique Studio legend.
- Exposed the mapped turn count through the debug snapshot and asserted it in
  the real-process exercise smoke.
- Completed `EX-003I`.

### Verification

- C-major up/down test identifies right-hand indices 3/12 and left-hand
  indices 5/10.
- Generated-song test verifies four mapped turns and exact representative keys.
- Real-process G♯ minor-arpeggio smoke requires a positive turn count before
  completing its two-pass workflow.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, eighty-six core tests and
fifty-seven application tests. Clippy and release builds report only the
repository's pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `667cbc8`
(`feat: highlight exercise hand turns`).

### Known limitations

- Gold indicates where the hand turns but does not yet name thumb-under versus
  finger-over separately.
- Color is currently fixed rather than user-configurable.
- Primary-chord finger guidance remains unavailable.

## 2026-07-25 — Cycle 055: Reviewed arpeggio fingerings (DONE)

### Outcome

Switching Technique Studio from a scale to a major or minor triad arpeggio no
longer removes finger guidance. Every key now uses a reviewed two-hand shape
instead of a generic white-key pattern.

### Implemented

- Enabled reviewed guidance for generated arpeggios.
- Added all twelve major and all twelve minor right/left tables.
- Encoded shared keyboard-shape groups plus the B♭ exceptions.
- Extended each pattern through one to three octaves, all directions and every
  repetition.
- Kept primary-chord guidance unavailable pending chord/voicing review.
- Upgraded native smoke to select G♯ minor arpeggio and verify that changing
  from melodic-minor scale resets the scale-only minor form.
- Completed `EX-003H`.

Tables were reviewed against Piano-ology's
[major](https://piano-ology.com/wp-content/uploads/2022/10/piano-ology-piano-technique-fingering-charts-major-triad-arpeggios.pdf)
and
[minor](https://piano-ology.com/wp-content/uploads/2022/10/piano-ology-piano-technique-fingering-charts-minor-triad-arpeggios.pdf)
two-octave charts on 2026-07-25.

### Verification

- Exact two-octave right/left assertions for all twenty-four arpeggios.
- Real-process G♯ minor-arpeggio smoke completes two passes at 70 BPM, toggles
  guidance off/on and verifies preset/settings/history persistence.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, eighty-five core tests and
fifty-seven application tests. Clippy and release builds report only the
repository's pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `d46a395`
(`feat: add reviewed arpeggio fingerings`).

### Known limitations

- Primary-chord progressions still lack reviewed finger/voicing guidance.
- The renderer shows finger numbers but not movement explanations.
- Tables are pedagogical defaults rather than anatomy-specific alternatives.

## 2026-07-25 — Cycle 054: Direction-aware melodic-minor fingering (DONE)

### Outcome

Every melodic-minor key now has reviewed finger guidance that follows the
classical change of form: melodic minor upward and natural minor downward.
Up-and-down practice no longer assumes that reversing the ascending fingers is
always correct.

### Implemented

- Added explicit right/left melodic-minor tables for all twelve keys.
- Split directional fingering into ascending and descending source sequences.
- Selected the melodic-minor table for ascent and the matching reviewed
  natural-minor table for descent.
- Joined both sources at a single apex and retained repetition behavior.
- Added the distinct B♭ right-hand start and direction change.
- Upgraded native smoke to select and persist G♯ melodic minor.
- Completed `EX-003G`.

Ascending patterns were reviewed against
[Hear and Play's twelve-key melodic-minor guide](https://hearandplay.com/main/the-fingering-of-the-melodic-minor-scale/)
on 2026-07-25. Descending patterns reuse the already reviewed natural-minor
tables because the generated classical form descends naturally.

### Verification

- Exact two-octave ascending right/left assertions for all twelve keys.
- B♭ up-and-down assertion proves that ascent and descent use different tables
  and share the apex only once.
- Real-process G♯ melodic-minor smoke completes two passes at 70 BPM, toggles
  guidance off/on and verifies preset/settings/history persistence.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, eighty-four core tests and
fifty-seven application tests. Clippy and release builds report only the
repository's pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `fe85447`
(`feat: add all melodic minor fingerings`).

### Known limitations

- Arpeggio and primary-chord patterns do not yet display reviewed fingers.
- The player still shows numbers rather than crossing explanations or
  anatomy-specific alternatives.
- Direction-aware tables are deterministic defaults; per-user substitutions
  remain future work.

## 2026-07-25 — Cycle 053: All harmonic-minor fingering tables (DONE)

### Outcome

Every harmonic-minor key now has reviewed, two-hand finger guidance. The app
correctly handles scales whose first and later octaves use different crossings,
rather than flattening them into a misleading repeating pattern.

### Implemented

- Enabled reviewed harmonic-minor guidance in all twelve pitch classes.
- Added explicit right/left tables for every key.
- Extended the fingering model with a first-octave lead-in, repeating later
  octave and optional terminal finger.
- Used that boundary for the changing crossings and endpoints in C♯, F♯, G♯
  and B harmonic minor.
- Retained C melodic minor while continuing to reject unreviewed melodic-minor
  keys and non-scale patterns.
- Upgraded native smoke to select and persist G♯ harmonic minor.
- Completed `EX-003F`.

Tables were reviewed against
[Piano-ology's harmonic-minor charts](https://piano-ology.com/wp-content/uploads/2024/01/piano-ology-piano-technique-fingering-charts-harmonic-minor-scales.pdf)
on 2026-07-25.

### Verification

- Exact two-octave right/left assertions for all twelve harmonic-minor keys.
- Real-process G♯ harmonic-minor exercise smoke completes two passes at 70 BPM,
  toggles guidance off/on and verifies preset/settings/history persistence.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, eighty-two core tests and
fifty-seven application tests. Clippy and release builds report only the
repository's pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `a7adbfa`
(`feat: add all harmonic minor fingerings`).

### Known limitations

- Melodic-minor tables beyond C remain intentionally unavailable.
- The renderer shows finger numbers but does not yet teach the reason for a
  crossing or endpoint substitution.
- Fingerings remain reviewed defaults rather than anatomy-specific variants.

## 2026-07-25 — Cycle 052: All natural-minor fingering tables (DONE)

### Outcome

Technique Studio now teaches reviewed finger numbers for natural minor in every
key. Learners can move beyond C minor without receiving a parallel-major
crossing that happens to fit the pitch count but not the keyboard shape.

### Implemented

- Expanded reviewed eligibility from C minor to all twelve natural-minor keys.
- Added explicit right- and left-hand patterns for C♯, E♭, F♯, G♯ and B♭
  natural minor.
- Preserved the established white-root patterns and all three C-minor forms.
- Applied every table through one to three octaves, both directions and every
  repetition using the existing sequence-aware fingering boundary.
- Spelled pitch class 8 as A♭ in major and G♯ in minor in both UI and generated
  names.
- Kept non-C harmonic/melodic minor and non-scale patterns unavailable rather
  than guessing.
- Moved real-process exercise coverage to G♯ natural minor and checked its two
  hand tonic pitches, visibility toggle, settings, preset and attempt history.
- Completed `EX-003E`.

The per-key patterns were reviewed against
[Piano-ology's natural-minor charts](https://piano-ology.com/wp-content/uploads/2024/01/piano-ology-piano-technique-fingering-charts-natural-minor-scales.pdf)
on 2026-07-25.

### Verification

- Exact two-octave right/left assertions for every black-root natural minor.
- Availability assertion for all twelve natural-minor pitch classes.
- Display-name assertions distinguish A♭ major from G♯ minor.
- Real-process G♯ natural-minor exercise smoke completes two passes at 70 BPM,
  toggles guidance off/on and verifies persistence.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, eighty-one core tests and
fifty-seven application tests. Clippy and release builds report only the
repository's pre-existing platform-helper and `unused_mut` warnings. An
additional strict `-D warnings` run stops on those same unchanged warnings.

Implementation commit: `ae8101a`
(`feat: add all natural minor fingerings`).

### Known limitations

- Harmonic- and melodic-minor tables beyond C remain intentionally unavailable.
- Fingerings are reviewed defaults, not anatomy-specific alternatives.
- The current renderer does not yet explain thumb crossings or substitutions.

## 2026-07-25 — Cycle 051: Persistent fingering visibility (DONE)

### Outcome

The learner's finger-number choice now sticks. Turning guidance off for recall
practice no longer resets on the next generated exercise or application launch,
and the preference is discoverable outside the player.

### Implemented

- Added `exercise_fingerings` to versioned appearance settings.
- Defaulted missing legacy values to `true`.
- Added typed Config getter/setter methods.
- Initialized the falling-note renderer from the stored preference.
- Kept reviewed fingering availability separate from current visibility.
- Saved every player toggle immediately.
- Added **Exercise Fingerings** to the normal Settings page.
- Let enabled note-name labels show through whenever finger numbers are off.
- Preserved the existing three-argument `NoteLabels::new` API.
- Added `NoteLabels::with_fingerings` for generated exercises instead of
  forcing Free Play, CLI and downstream callers onto a new constructor.
- Split smoke settings validation into named assertions for actionable failures.
- Completed `EX-003D`.

### Verification

- Default appearance enables exercise fingerings.
- Appearance RON from before the new field also enables them.
- An explicit disabled preference survives Model serialization and rebuild.
- Real-process B-major smoke verifies:
  - guidance available and initially enabled;
  - off action accepted and snapshot false;
  - on action accepted and final settings true;
  - the full completion/preset/library path remains green.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All desktop gates passed: two MIDI-file tests, eighty-one core tests and
fifty-seven application tests. Clippy and release builds report only the
repository's pre-existing platform-helper and `unused_mut` warnings.

An additional `neothesia-cli` check reached its always-on
`ffmpeg-sys-next` build and stopped because this Windows environment has no
vcpkg/pkg-config FFmpeg installation. The note-label public constructor was
kept backward compatible, so this cycle does not require a CLI source change.

Implementation commit: `0d6c428`
(`feat: persist exercise fingering visibility`).

### Known limitations

- Visibility is global rather than stored per preset.
- There is no keyboard shortcut yet; the top-bar and Settings controls are the
  supported interfaces.
- Settings changes save through the existing settings lifecycle, while player
  changes save immediately.

## 2026-07-25 — Cycle 050: All major-scale fingering tables (DONE)

### Outcome

Every major key in Technique Studio now has reviewed two-hand fingering across
the complete generated span. Learners can move around the circle of fifths
without the app silently reusing C-major crossings where they do not fit.

### Implemented

- Reviewed two-octave visual tables for the remaining major keys.
- Added D♭:
  - RH starts 2;
  - LH starts 3.
- Added E♭:
  - RH starts 3;
  - LH starts 3.
- Added G♭:
  - RH starts 2 and groups the black keys under 2–3 / 2–3–4;
  - LH starts 4 with its own crossing cycle.
- Added A♭:
  - RH starts 3–4 before the first thumb;
  - LH starts 3.
- Added B♭:
  - RH starts 2 before the thumb lands on C;
  - LH starts 3.
- Added B:
  - RH retains the standard sharp-key pattern;
  - LH starts 4 so the thumb lands on E rather than F♯.
- Replaced ad hoc per-key generation with an explicit start finger,
  seven-degree continuation and optional final-finger model.
- Correctly distinguished starting, intermediate-tonic and final-tonic fingers
  over two or three octaves.
- Preferred conventional flat major names in Technique Studio and generated
  titles, while retaining C♯ minor spelling.
- Kept unreviewed non-C minor keys unavailable.
- Changed real-process smoke to B major to cover its exceptional left hand.
- Completed `EX-003C`.

### Verification

- Exact fifteen-note right and left sequences for D♭, E♭, G♭, A♭, B♭ and B.
- Common right-hand keys retain final finger 5 only at the endpoint.
- G♭ verifies distinct black-key grouping in both hands.
- B verifies left-hand `43214321 3214321`.
- D♭ major titles use D♭/Db while C♯ minor keeps C♯/C#.
- Non-C minor and arpeggio plans return no fingering.
- Real-process smoke selects B by bidirectional selector actions.
- B2/B4 appear as the required two-hand tonic notes.
- Fingering defaults on, toggles off and on, and the exercise completes,
  persists, restores and reopens.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, seventy-nine core tests and fifty-seven
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `2c3b468`
(`feat: complete major scale fingering tables`).

### Known limitations

- Minor tables beyond C remain unreviewed.
- Enharmonic selector spelling is fixed to the most readable common major/minor
  name rather than user-selectable notation.
- Standard teaching fingering may need a future alternate option for individual
  hand anatomy or a teacher's method.

## 2026-07-25 — Cycle 049: Common major-scale fingering group (DONE)

### Outcome

Reviewed falling-note fingering now covers the six major scales most often
introduced first, including F major's important right-hand exception. Technique
Studio also tells the learner whether guidance exists before an exercise starts.

### Implemented

- Checked the C/G/D/A/E shared fingering group against Baylor Piano Basics.
- Checked F major's separate right-hand pattern against piano.org's dedicated
  scale reference.
- Expanded the fingering eligibility table to:
  - C major;
  - G major;
  - D major;
  - A major;
  - E major;
  - F major.
- Applied the common two-hand C pattern to C/G/D/A/E.
- Added F-major right hand `12341234`; retained common left hand
  `54321321`.
- Generalized multi-octave crossing without turning the intermediate tonic into
  an endpoint finger.
- Reused the existing direction and repetition transformer.
- Kept all non-C minor scales unsupported until their distinct tables are
  reviewed.
- Added a green “Reviewed fingering available” or neutral “not yet reviewed”
  line below the Technique Studio exercise preview.
- Changed native smoke from C to F major to exercise the exceptional right-hand
  branch.
- Completed `EX-003B` without closing the all-key `EX-003` parent.

### Verification

- C/G/D/A/E two-octave right hand:
  `1231234 12312345`.
- C/G/D/A/E two-octave left hand:
  `5432132 14321321`.
- F two-octave right hand:
  `1234123 12341234`.
- F two-octave left hand:
  `5432132 14321321`.
- G minor returns no fingering rather than borrowing G major.
- Real-process smoke selects F through semantic key controls.
- F2/F4 are exposed as the first required two-hand tonic notes.
- Guidance is available and default-on, toggles off and back on, and survives
  the complete practice/preset/library flow.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, seventy-seven core tests and fifty-seven
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `2af19e5`
(`feat: expand reviewed major scale fingerings`).

### Known limitations

- B and the five black-key-root major scales still await table-by-table review.
- Minor coverage still stops at C.
- Fingering preferences can vary with hand anatomy; the current UI presents a
  standard teaching fingering, not an immutable rule.

## 2026-07-25 — Cycle 048: Reviewed fingering foundation (DONE)

### Outcome

Supported C-scale exercises now teach where each hand crosses, not merely which
key comes next. Finger numbers travel with the falling notes and can be hidden
instantly after the pattern begins to settle.

### Implemented

- Added `ExerciseFingerings` as deterministic plan output.
- Implemented the established C-scale finger-number pattern for:
  - right and left hand;
  - C major;
  - C natural, harmonic and melodic minor;
  - ascending, descending and up/down;
  - one, two and three octaves;
  - every configured repetition.
- Reversed the full span for descending guidance and joined up/down at one apex.
- Removed inactive-hand data from single-hand plans.
- Returned `None` for every unreviewed key and non-scale pattern.
- Attached fingers after MIDI serialization by exact start time, pitch and
  channel, avoiding ambiguous pitch-only matching.
- Generalized the note-label renderer to switch between note names and
  per-occurrence finger numbers.
- Cached separately sized digit buffers for white and black note widths.
- Added a stateful `Fingers: ON/OFF` control in the player.
- Added semantic automation state and actions for availability and visibility.
- Kept imported MIDI and Free Play note labels backward compatible.
- Completed `EX-003A` without claiming completion of all-key `EX-003`.

### Verification

- Exact two-octave two-pass right-hand sequence:
  `1231234 12312345 4321321 4321321`, repeated.
- Exact corresponding left-hand sequence in both directions.
- C♯ scale and C arpeggio explicitly return no fingering.
- Generated C exercise maps:
  - right C4 to finger 1;
  - left C2 to finger 5;
  - right apex C5 to finger 5.
- Real-process smoke confirms reviewed guidance is available and on by default.
- Native control turns guidance off, snapshot verifies it, and turns it on
  again before completing the exercise.
- Existing completion, loop, hand mode, preset persistence and library reopen
  flows remain green.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, seventy-five core tests and fifty-seven
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `fd1f446`
(`feat: show reviewed exercise fingerings`).

### Known limitations

- Reviewed coverage currently stops at the C major/minor scale family.
- Fingering is generated exercise metadata, not yet a portable annotation for
  arbitrary imported MIDI.
- The player does not yet score whether the learner used the suggested physical
  finger; ordinary MIDI input does not identify fingers.

## 2026-07-25 — Cycle 047: Reusable exercise presets (DONE)

### Outcome

A learner can now build a useful exercise once and return to it without
re-entering eight parameters. Recent work is captured automatically, while
important warm-ups or technique drills can be kept deliberately as favourites.

### Implemented

- Added backward-compatible recent and favourite exercise vectors to settings.
- Recorded a complete specification whenever an exercise starts.
- Moved reused specifications to the front instead of creating duplicates.
- Bounded recents to eight and favourites to twelve.
- Added exact-spec favourite toggle semantics.
- Saved favourite changes immediately.
- Filtered invalid and duplicate persisted specifications on access.
- Added a fifth Technique Studio selector row:
  - Recent;
  - Favourites.
- Added previous/next controls with stable semantic action IDs.
- Restored key, tonality/form, pattern, direction, hands, octaves, tempo and
  repetitions together.
- Added a visible `☆ Save favourite` / `★ Remove favourite` action next to
  Start Exercise.
- Kept the full parameter grid visible after restoration so the learner can
  verify or refine the preset before starting.
- Extended native automation through favourite save, favourite restore, recent
  restore, second start and Practice Library reopen.
- Completed `EX-002`.

### Verification

- Twelve distinct starts retain only the newest eight recent variants.
- Reusing an older recent moves it to the front without duplication.
- Invalid persisted specifications are excluded.
- Favourite toggle adds, removes and re-adds the exact variant.
- Favourites survive the settings serialization round trip.
- Preset navigation restores a complete spec and wraps in both directions.
- Empty preset collections are safe no-ops.
- Real-process smoke confirms:
  - favourite creation and cycling;
  - recent and favourite restoration;
  - restored exercise start and return;
  - two-pass completion and attempt persistence;
  - Practice Library reopen.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, seventy-three core tests and fifty-seven
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `bbb8adb`
(`feat: save reusable exercise presets`).

### Known limitations

- Favourites use generated summary labels; custom user-entered names are not
  available yet.
- Presets cannot yet be reordered manually or exported.
- A preset is an exact configuration, not a scheduled curriculum item.

## 2026-07-25 — Cycle 046: Complete minor scale forms (DONE)

### Outcome

Technique Studio now teaches the three minor scale forms by their actual names
and pitches. In particular, classical melodic minor changes correctly with
direction instead of applying its ascending alterations on the way down.

### Implemented

- Added a backward-compatible `ExerciseMinorForm` dimension:
  - Natural;
  - Harmonic;
  - Melodic.
- Kept major/minor tonality separate from scale form so arpeggios and functional
  chords retain a coherent harmonic model.
- Generated natural minor identically in both directions.
- Generated harmonic minor with the raised seventh in both directions.
- Generated classical melodic minor with raised sixth and seventh ascending
  and natural minor descending.
- Joined melodic-minor up/down phrases at one apex without repeating it.
- Replaced the generic Minor selector label with the exact selected form.
- Added four-state bidirectional selector cycling.
- Reset non-Natural form when leaving the Scale pattern.
- Rejected invalid form/pattern combinations in the core boundary.
- Included valid minor form in normalized learning identity.
- Defaulted old RON specifications to Natural.
- Added a short accepted-action retry to the real-process smoke runner because
  the loopback driver can become available just before the first rendered frame
  registers its semantic click regions.
- Completed `EX-001H`.

### Verification

- Exact A melodic-minor up/down pitches:
  `A B C D E F♯ G♯ A G F E D C B A`.
- Exact descending A harmonic-minor pitches:
  `A G♯ F E D C B A`.
- Invalid harmonic-major and melodic-minor-arpeggio specifications rejected.
- Legacy serialized exercise loads as Natural minor.
- Harmonic minor receives a distinct practice identity.
- UI selector advances and reverses across all four labels.
- Leaving Scale resets Melodic to Natural.
- Real-process exercise generation, two-pass completion, persistence, Retry,
  hand switch, measure loop and Practice Library reopen smoke.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, seventy-one core tests and fifty-six
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `756fc40`
(`feat: add complete minor scale forms`).

### Known limitations

- Melodic minor currently follows the classical ascending/descending convention;
  jazz melodic minor (raised sixth and seventh in both directions) is not a
  separate option.
- Reviewed key- and hand-specific fingering is still absent.
- Minor scale forms do not yet have named favourite presets or a dedicated
  progression view.

## 2026-07-25 — Cycle 045: Pass-by-pass exercise consistency (DONE)

### Outcome

Repeated exercises now reveal whether execution holds together across passes.
The learner sees each pass's accuracy and first-to-last timing stability instead
of a single average that can conceal a late-session drop.

### Implemented

- Added exact beats-per-repetition calculation to `ExercisePlan`.
- Stored generated phrase duration on `Song`.
- Derived that duration from the same integer microseconds-per-beat value used
  by MIDI serialization, avoiding floating-point boundary drift.
- Added serializable `ExercisePassSummary` with:
  - one-based pass number;
  - matched, wrong and missed breakdown;
  - robust timing median and deviation.
- Added a deterministic result-to-pass summarizer.
- Assigned exact boundary notes to the following pass.
- Enriched generated-exercise summaries in `MidiPlayer::finish_practice`.
- Kept single-pass summaries empty rather than duplicating overall feedback.
- Defaulted older attempt summaries to no pass evidence.
- Replaced the low-value chord-evidence line for repeated exercises with:
  - the complete pass accuracy sequence;
  - improved / held steady / fell later / varied classification;
  - first-to-last timing-spread evidence when available.
- Used descriptive late-pass language and explicitly avoided diagnosing
  fatigue.
- Upgraded native smoke to a full two-pass C♯ 70 BPM performance and verified
  persisted pass 2.

### Verification

- Exact phrase-boundary unit test with targets at 0, 0.5, 1.0 and 1.5 seconds.
- Pass-one 50% and pass-two 100% breakdown assertions.
- Per-pass timing median assertion.
- One-pass evidence suppression test.
- Serialized 70 BPM second-pass boundary equals the generated duration exactly.
- Completion-copy test for 80% → 100%, improved trend and 24→12 ms spread.
- Explicit copy assertion that no fatigue claim is present.
- Legacy summary default assertion.
- Real-process two-pass completion, persistence, Retry, controls and library
  reopen smoke.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, sixty-nine core tests and fifty-six
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `e661362`
(`feat: compare repeated exercise passes`).

### Known limitations

- Pass summaries are shown for the current completion and persisted inside the
  session, but History does not yet chart them across days.
- Loop attempts covering only part of a phrase intentionally omit pass trends
  when fewer than two passes have judged evidence.
- No configurable rest or count-in exists between passes yet.

## 2026-07-25 — Cycle 044: Multi-pass exercise sessions (DONE)

### Outcome

Technique exercises can now run for enough repetitions to establish a stable
motor pattern and collect meaningful timing evidence, instead of ending after
a single short pass.

### Implemented

- Added `repetitions` to `ExerciseSpec`.
- Defaulted saved specifications from earlier versions to one pass.
- Validated one through eight repetitions in the core.
- Exposed learner-focused 1, 2, 4 and 8 choices in Technique Studio.
- Used the previously empty eighth selector slot for Repeat.
- Added previous and next semantic controls.
- Repeated the fully constructed musical phrase before assigning hand notes.
- Preserved each phrase's tonic-to-tonic boundary.
- Added `x2`, `x4` or `x8` to generated names; omitted `x1` noise.
- Excluded repetition count from normalized practice identity so longer
  evidence sessions continue the same learning history.
- Persisted repeat count automatically through the existing settings and
  generated-source history structures.
- Extended native smoke coverage to operate both repetition controls and
  inspect the persisted one-pass value.

### Verification

- Exact four-times phrase-length test.
- Repeated prefix equals the entire original phrase.
- Equal practice identity for one and four passes.
- Display-name repeat suffix test.
- Rejection tests for zero and nine repetitions.
- Legacy RON specification defaults to one pass.
- Bidirectional 1/2/4/8 selector boundary tests.
- Semantic ID uniqueness test includes both new controls.
- Existing full generated-exercise completion, persistence and library-reopen
  smoke remains green.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, sixty-seven core tests and fifty-four
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `53cb2f8`
(`feat: add exercise repetitions`).

### Known limitations

- Repetitions have no configurable rest or count-in between passes yet.
- The completion view aggregates the full session; pass-by-pass consistency
  will require phrase-boundary evidence in the matcher.
- Named favourite variants remain under `EX-002`.

## 2026-07-25 — Cycle 043: Reopen generated exercises (DONE)

### Outcome

Completed exercises are no longer dead-end rows in Practice Library. They can
be reopened directly without a MIDI file, and resume the learner's latest
practice setup.

### Implemented

- Added optional `exercise_spec` source metadata to `SongPracticeSetup`.
- Defaulted the field so existing practice-history files remain compatible.
- Propagated generated-source metadata into `RecentSongSummary`.
- Added source metadata to Practice Library's merged row model.
- Distinguished three primary row actions:
  - Practice for generated exercises;
  - Open for available MIDI files;
  - Locate for missing MIDI files.
- Rendered generated rows with the available-source visual treatment.
- Added a shared exercise-open path used by Technique Studio and Practice
  Library.
- Rebuilt the in-memory Type-1 MIDI from the exact saved specification.
- Applied the existing saved track setup before launching.
- Updated the last-used Technique Studio specification after reopening.
- Added semantic home-to-library and recent-exercise actions.
- Extended native automation to reopen the saved generated exercise and verify
  restored hand mode.

### Verification

- Generated source setup/history file round trip.
- Recent summary contains the same `ExerciseSpec` and no source path.
- Legacy serialized setup without `exercise_spec` loads as `None`.
- Real-process flow:
  - completes C♯ 70 BPM;
  - changes the saved practice scope to Right hand;
  - returns to the menu;
  - opens Practice Library;
  - opens the most recent generated exercise;
  - observes an active player with Right hand restored;
  - returns and exits with code 0.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, sixty-five core tests and fifty-four
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `ed0f789`
(`feat: reopen exercises from practice library`).

### Known limitations

- A normalized exercise stores the latest reconstructable tempo/hand variant;
  named favourite variants are still needed for multiple saved presets.
- Search uses the generated display name but has no dedicated exercise filter.
- Existing pre-Cycle-043 generated rows have no reconstructable source
  metadata and cannot be inferred safely.

## 2026-07-25 — Cycle 042: Continuous exercise tempo progression (DONE)

### Outcome

Increasing an exercise's tempo no longer fragments the learner's history into
unrelated pseudo-songs. The same musical task now accumulates results across
tempo and hand progression, while each completed attempt retains its real BPM.

### Implemented

- Added a versioned canonical practice identity to `ExercisePlan`.
- Included tonic, tonality, pattern, direction and octave span in that identity.
- Excluded tempo so tempo progression remains continuous.
- Excluded requested hands because session history already records and filters
  the performed hand scope.
- Kept raw MIDI content hashes tempo-sensitive for serialization tests.
- Replaced the generated `Song` history key with normalized practice identity.
- Retained the originating `ExerciseSpec` on generated songs.
- Calculated effective BPM as base exercise BPM multiplied by player speed.
- Added backward-compatible optional effective BPM to `PracticeSession`.
- Propagated effective BPM into recent-history summaries.
- Added BPM change to overview trends.
- Preferred `84 BPM · 120% speed` style copy for generated attempts.
- Preserved percentage-only speed copy for imported MIDI and old history.
- Extended native automation to:
  - select C♯;
  - raise the exercise from 60 to 70 BPM;
  - perform every required note to completion;
  - retry and re-check hand/loop behavior;
  - verify `effective_tempo_bpm: Some(70)` in persisted history.

### Verification

- Normalized practice identity groups tempo and hand variants.
- Different tonic produces a different 64-character identity.
- Generated-song effective-tempo calculations cover 75% speed and invalid
  multipliers.
- Overview tempo progression test covers 60 to 84 BPM.
- Legacy session serialization test removes the BPM field and reloads safely.
- UI copy tests prefer BPM evidence and fall back to speed.
- Real-process C♯ 70 BPM completion, persistence, Retry, hands and loop smoke.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, sixty-three core tests and fifty-four
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `041062e`
(`feat: unify exercise tempo progression`).

### Known limitations

- Practice Library still treats generated-history rows as unavailable because
  they have no file path; a generated-source discriminator is next.
- Existing exercise sessions created before this cycle retain their old
  tempo-specific IDs; no speculative migration is attempted.
- Named favourites and recent variants remain under `EX-002`.

## 2026-07-25 — Cycle 041: Persistent exercise choices (DONE)

### Outcome

Technique Studio now behaves like a durable practice workspace. Every choice
can move backward or forward explicitly, and the next launch resumes the last
exercise the learner actually started.

### Implemented

- Replaced click-to-cycle cards with visible previous and next buttons.
- Added bidirectional wrapping for all twelve keys, three patterns, three
  directions, three hand modes, three octave spans and the graduated tempo
  list.
- Preserved a clear centered value panel between each arrow pair.
- Added stable semantic action IDs to all fourteen selector buttons.
- Used one adjustment implementation for native controls and automated tests.
- Added `last_exercise_spec` to the versioned history/settings section.
- Saved the specification only after generation succeeds and the learner
  starts the exercise.
- Restored the saved specification in new menu scenes.
- Added public structural validation to `ExerciseSpec`.
- Fell back to the default exercise if persisted numeric values are invalid.
- Kept legacy settings compatible through Serde defaults.
- Extended the native smoke fixture to select C♯ and inspect the generated
  required notes.
- Inspected the settings file after clean process exit to verify persistence.

### Verification

- Previous/next unit coverage for key, pattern, direction, hands, octaves and
  tempo boundary wrapping.
- Full specification RON serialization round trip.
- Legacy history settings load without `last_exercise_spec`.
- Invalid tonic 99 falls back safely.
- Semantic action IDs remain unique and namespaced.
- Real-process fixture confirms:
  - C♯ selection is accepted;
  - first required notes are MIDI 37 and 61;
  - played notes score;
  - hand and loop controls still work;
  - `settings.ron` contains tonic 1;
  - process exits with code 0.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, sixty core tests and fifty-two
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `e613901`
(`feat: persist technique studio choices`).

### Known limitations

- Only the last-started exercise is persisted; named favourites and a recent
  variant list remain under `EX-002`.
- The selector page does not yet show due/review evidence for an exercise.
- Fingering remains deferred pending reviewed per-key/per-hand tables.

## 2026-07-25 — Cycle 040: Technique Studio (DONE)

### Outcome

The exercise mode is now usable from the home screen. A learner can choose a
musical target, generate it instantly and practise it with the same
wait-for-notes, scoring, looping and feedback system used for repertoire.

### Implemented

- Added a Technique Studio home-screen action.
- Added a dedicated native exercise page with a compact two-column layout.
- Added clickable choices for:
  - all twelve tonic pitch classes;
  - major or minor;
  - scale, arpeggio or primary chords;
  - ascending, descending or up and down;
  - right, left or both hands;
  - one, two or three octaves;
  - a graduated 30–200 BPM practice range.
- Added a live descriptive preset name.
- Added Start Exercise and Enter-key launch paths.
- Added Back and Escape return paths.
- Generated against the user's configured keyboard range and displayed
  generation errors without leaving the page.
- Added `Song::from_exercise` so generated tracks receive explicit hand
  identity, including single-hand exercises.
- Reused the existing MIDI output connection, Pianoteq routing, wait mode,
  practice matcher, loops, hand controls and completion flow.
- Added stable semantic actions for opening Technique Studio and starting the
  exercise.
- Added an `-ExerciseFixture` native smoke path.
- Tightened the home layout so the extra action remains inside the supported
  minimum window height.

### Verification

- Unit test for cyclic pattern, direction, hands and tempo choices.
- Integration tests for generated both-hand and single-hand song setup.
- Real-process automation:
  - opened Technique Studio from a clean menu;
  - generated and started the default C-major exercise;
  - confirmed wait-for-notes defaults on;
  - confirmed Both-hand mode;
  - injected required C notes and scored two matches;
  - switched to Right hand;
  - enabled a valid 1–2 measure loop;
  - restarted without losing the loop;
  - disabled the loop, returned to the menu and exited with code 0.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty-eight core tests and fifty-two
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `017697d`
(`feat: add technique studio exercise flow`).

### Known limitations

- Exercise choices are kept only for the current menu session.
- Generated variants are not yet presented as a distinct source type in
  Practice Library.
- Clicking cycles choices forward; richer keyboard and decrement controls are
  still to come.
- Fingering remains deferred pending reviewed per-key/per-hand tables.

## 2026-07-25 — Cycle 039: Exercise plans as playable MIDI (DONE)

### Outcome

Generated exercises are now valid songs rather than isolated note lists. A
scale, arpeggio or chord plan can enter Neothesia's existing timing, rendering,
wait-mode and scoring pipeline with its hands recognized automatically.

### Implemented

- Converted `ExercisePlan` to memory-backed Type-1 MIDI.
- Added a 480-PPQ conductor track with requested tempo and 4/4 meter.
- Added separate named tracks and MIDI channels for right and left hands.
- Used an 80% note gate so successive exercise notes retain audible separation.
- Preserved one-beat melodic steps and two-beat chord steps.
- Generated concise names containing key, tonality, pattern, hand scope and
  tempo.
- Kept source paths empty for generated material.
- Produced deterministic content identities from the generated MIDI.
- Verified both-hand track pitch centers through the normal `SongConfig`
  inference path.
- Reused the existing Both, Right and Left practice-hand controls.

### Verification

- Stable Type-1 format, track count, hand roots and note-count test.
- Exact note-duration tests at 60 and 120 BPM.
- Stable-identity and tempo-sensitive-identity tests.
- Application integration test for generated-song hand recognition.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty-eight core tests and fifty
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `7fe91f4`
(`feat: convert exercise plans to MIDI`).

### Known limitations

- Exercise creation is not exposed in the menu yet.
- Generated exercises still need explicit source metadata before history and
  repertoire can distinguish them without relying on content identity.
- Single-hand plans intentionally cannot offer a two-hand toggle.
- Fingering remains deferred pending reviewed per-key/per-hand tables.

## 2026-07-25 — Cycle 038: Deterministic exercise plans (DONE)

### Outcome

Exercise mode now has a tested musical domain model instead of a placeholder
screen. It can deterministically describe playable scale, arpeggio and
primary-chord material that will enter the existing practice player in the
next integration layer.

### Implemented

- Added `neothesia_core::exercise`.
- Added exercise pattern types for Scale, Arpeggio and Primary Chords.
- Added Major and Minor tonalities.
- Added Ascending, Descending and Up-and-Down directions.
- Added Right, Left and Both hand scopes.
- Added a serializable specification with tonic, one-to-three octaves and
  20–240 BPM.
- Generated:
  - major and natural-minor scale intervals;
  - tonic major/minor arpeggios;
  - I–IV–V–I major cadences;
  - i–iv–V–i minor cadences with a functional major dominant.
- Positioned right-hand tonic at C4–B4 and left-hand tonic at C2–B2.
- Kept both hands parallel and two octaves apart.
- Removed duplicate apex moments from up/down exercises.
- Assigned one beat to melodic moments and two beats to chord moments.
- Rejected every generated note outside the configured keyboard range.
- Added explicit errors for invalid tonic, octave span, tempo and range.
- Documented why fingering is deferred until reviewed key/hand-specific tables
  exist.
- Documented the required in-memory MIDI and UI integration layers.

### Verification

- Exact C-major both-hand up/down sequence test.
- Exact A-minor descending arpeggio test.
- Exact c-minor i–iv–V–i voicing test.
- B-major three-octave 88-key boundary test.
- Restricted-keyboard rejection test.
- Invalid tonic, four-octave and 241 BPM rejection tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty-six core tests and forty-nine
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `d45b7c5`
(`feat: add deterministic exercise plans`).

### Known limitations

- Plans are not converted into a `MidiFile` yet.
- There is no exercise selection UI or persisted exercise identity yet.
- Fingering is intentionally absent pending reviewed per-key/per-hand data.
- Harmonic minor scales and contrary-motion exercises are future variants.

## 2026-07-25 — Cycle 037: Pedal timing and dynamics contour (DONE)

### Outcome

Technique feedback now describes whether sustain changes tend to land early or
late and whether performed dynamics follow the score's rising/falling shape.
Both signals use explicit evidence thresholds and avoid treating raw MIDI
values as an absolute musical verdict.

### Implemented

- Added timestamps to user and score CC64 evidence.
- Detected sustain transitions only when values cross the half-pedal threshold.
- Kept raw controller-change and continuous-value counts intact.
- Paired pedal-down events with pedal-down references and pedal-up events with
  pedal-up references.
- Calculated signed user-minus-score transition offsets.
- Summarized offsets with median and median absolute deviation.
- Persisted paired sample count, transition counts, median offset and spread.
- Required:
  - at least four paired transitions;
  - equal user/score transition counts;
  - every target transition to have a pair;
  - no more than 120 ms median deviation.
- Reported insufficient samples, mismatched transitions and unstable timing
  separately.
- Grouped matched note velocities by exact score onset.
- Averaged chord members before comparing consecutive dynamic points.
- Ignored target changes smaller than six velocity units.
- Classified played changes smaller than three units as flat.
- Counted aligned, flat and opposite directions.
- Required six shaped steps for learner-facing contour feedback.
- Added two new Technique lines without overlapping completion actions at the
  minimum panel height.
- Added backward-compatible defaults to every persisted field.
- Marked `MUS-005` complete.

### Verification

- A seven-onset crescendo/decrescendo fixture produces:
  - six shaped steps;
  - six aligned;
  - zero flat;
  - zero opposite.
- Chord members at the same onset are averaged rather than treated as separate
  contour steps.
- Four target/user sustain transitions offset by +40 ms produce:
  - four timing pairs;
  - median 40 ms late;
  - zero median deviation.
- UI copy tests cover stable contour/timing, incomplete references and
  transition mismatch.
- Legacy expression RON loads with zero contour/timing fields.
- Re-ran the deterministic native completion fixture; Overview, Technique,
  History, Retry and clean exit all pass.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty-one core tests and forty-nine
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `3f99cde`
(`feat: add pedal timing and dynamics contour`).

### Known limitations

- MIDI sustain references are performance data, not engraved pedal notation.
- The profile does not yet distinguish flutter, half-pedal depth or
  repedalling intent.
- Dynamic direction follows MIDI velocity shape and cannot infer phrasing where
  the source MIDI is mechanically flat.
- Evidence remains descriptive; it does not assign a pedal or dynamics grade.

## 2026-07-25 — Cycle 036: Deterministic native completion flow (DONE)

### Outcome

Native automation can now finish a complete practice take, inspect all three
completion sections and prove that Retry returns to a clean attempt. The
completion UI is covered in the real GPU application rather than only by
helper-unit tests.

### Implemented

- Added a `-CompletionFixture` parameter set to the native smoke runner.
- Embedded a compact, deterministic MIDI fixture as Base64 test data.
- Generated the fixture only inside the per-run temporary directory.
- Defined a Type-1 file with:
  - 480 PPQ;
  - 4/4 at 120 BPM;
  - a C5 right-hand track;
  - a C3 left-hand track;
  - a finite four-beat take.
- Repeatedly read required pitches and performed them through Debug MIDI input.
- Waited for a non-null completion tab rather than relying on elapsed time
  alone.
- Asserted completion starts on Overview.
- Activated and asserted Technique, History and Overview by semantic ID.
- Activated completion Retry.
- Asserted Retry removes the completion screen and resets matched notes to
  zero.
- Returned to menu and exited cleanly.
- Documented both real-song and deterministic-completion commands.

### Verification

- `.\scripts\debug-practice-smoke.ps1 -CompletionFixture`
  - `CompletionTab = overview`;
  - `MatchedNotes = 2`;
  - Technique, History and Overview passed;
  - `RetryReset = 0`;
  - `ExitCode = 0`.
- Re-ran the prepared “Look at the Sky” path with `-SkipBuild`:
  - wait `True → False`;
  - matched input `2`;
  - hands `Both → Right`;
  - loop `1–2`, restart preserved, disable passed;
  - exit code `0`.
- Cycle 035's immediately preceding full Rust test, Clippy, release build,
  formatting and diff checks remain authoritative because Cycle 036 changes
  only the tested PowerShell runner and its documentation.

Implementation commit: `1fc952c`
(`test: cover completion flow in native smoke`).

### Known limitations

- The completion fixture is intentionally too small to produce meaningful
  timing calibration, history trends or weak-passage recommendations.
- GPU pixels are not captured or compared.
- The fixture validates exact semantic state, not visual styling.

## 2026-07-25 — Cycle 035: Scored MIDI injection in native smoke (DONE)

### Outcome

The real-process smoke test now plays the notes Neothesia is actually waiting
for and proves that they reach the normal player input path and practice
matcher. It no longer verifies wait mode using state toggles alone.

### Implemented

- Added `PracticeMatcher::required_note_pitches()`.
- Preserved duplicate occurrences and returned pitches in stable sorted order.
- Added a matcher assertion for the required C-major chord pitches.
- Added a Debug-only required-pitch delegate on `MidiPlayer`.
- Added required-note count and pitch list to the practice snapshot.
- Added an acknowledged `DebugMidiInput` application event.
- Added a player-scene Debug MIDI handler that calls the normal scene
  `midi_event` path.
- Added safe in-process channel/note/velocity validation.
- Added `MIDI <channel> <note> <velocity>` to the loopback protocol.
- Restricted channels to 0–15 and notes/velocities to 0–127.
- Defined velocity zero as a release, matching live MIDI normalization.
- Added stable JSON array output for required pitches.
- Extended the native smoke runner to:
  - wait until the score blocks on required notes;
  - assert count/list agreement;
  - inject NoteOn and NoteOff for every required pitch;
  - assert that `matched_notes` increases.
- Updated the automation contract and next-step boundary.

### Verification

- Ran the checked-in smoke runner against “Look at the Sky - Porter Robinson,
  original key, auto-aligned.”
- The first blocked target exposed two required notes.
- Both note-on and note-off events were accepted.
- The matcher reported `MatchedAfterInput = 2`.
- Existing wait, hands, loop, restart, back and clean-exit assertions remained
  green.
- Protocol tests reject channel 16 and note 128.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-eight
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `6fd60aa`
(`test: inject scored MIDI in native smoke`).

### Known limitations

- The smoke runner performs only the first blocked target, not a whole song.
- It does not yet control inter-note timing or note-hold duration.
- A purpose-built short MIDI fixture is still needed to reach and automate the
  completion screen.
- GPU screenshot comparison remains future work.

## 2026-07-25 — Cycle 034: Native loop and restart coverage (DONE)

### Outcome

The checked-in native smoke run now proves the full baseline interaction path:
open a real two-hand MIDI, start it, inspect and toggle wait mode, enable a
measure-snapped loop, restart that practice scope, disable the loop, return to
the menu and exit cleanly.

### Implemented

- Extracted loop toggling from the rendered button into shared product logic.
- Kept loop disable behavior for count-in cancellation, attempt reset, Tempo
  Coach reset and playback resume unchanged.
- Assigned `practice.player.loop` to the visible repeat control.
- Added `practice.player.restart` for current-scope restart.
- Restarted the loop take when looping and the whole-song take otherwise.
- Added the `R` shortcut and a confirmation toast.
- Added loop active, start/end measures, count-in and pause state to the Debug
  snapshot.
- Added stable nullable numeric encoding to the local driver JSON.
- Extended the PowerShell smoke runner to assert:
  - loop activation;
  - a valid measure range;
  - range preservation across restart;
  - loop deactivation.
- Updated the shortcut page and native automation contract.
- Marked `QA-001` complete based on real-process evidence.

### Verification

- Ran the checked-in smoke script against “Look at the Sky - Porter Robinson,
  original key, auto-aligned.”
- Observed wait mode `True → False`.
- Observed hands `Both → Right`.
- Observed loop range `1–2`.
- Observed `LoopRestarted = True`.
- Observed `LoopDisabled = True`.
- Observed exit code `0`.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-eight
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `f49cd2c`
(`test: cover loop practice in native smoke`).

### Known limitations

- The smoke run validates state transitions but does not inject performed MIDI
  notes or wait for a completed loop attempt.
- Completion tabs and completion Retry still require a deterministic
  performance-input fixture.
- GPU screenshot comparison remains future work.

## 2026-07-25 — Cycle 033: Reusable native practice smoke runner (DONE)

### Outcome

The proven native automation sequence is now a repeatable repository command,
not a one-off terminal experiment. It launches the actual GPU application,
drives semantic controls, asserts practice state and exits cleanly.

### Implemented

- Added `scripts/debug-practice-smoke.ps1`.
- Built the Debug executable by default with an opt-out for repeated runs.
- Passed Unicode and space-containing MIDI paths as structured process
  arguments.
- Selected an available IPv4 loopback port at runtime.
- Ran the app in a uniquely named temporary working directory.
- Copied the local SoundFont into that isolated directory.
- Asserted that the initial scene is the menu.
- Started the loaded song and waited for a player snapshot.
- Asserted that wait-for-notes defaults on.
- Toggled wait mode and asserted the state changed.
- Cycled practice hands when the song exposes a hand scope and asserted the
  state changed.
- Returned to the menu and asserted that the player snapshot disappeared.
- Added a debug-only acknowledged `EXIT` command and asserted process exit code
  zero.
- Removed the temporary settings, history and SoundFont copy after every run.
- Kept exact-process termination as a guarded failure fallback.
- Documented the runnable command.

### Verification

- Ran the checked-in script twice against “Look at the Sky - Porter Robinson,
  original key, auto-aligned,” including once under PowerShell strict mode.
- Observed wait mode `True → False`.
- Observed hand mode `Both → Right`.
- Observed clean application exit code `0`.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-eight
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `5f16f5c`
(`test: add native practice smoke runner`).

### Known limitations

- The runner does not yet create a loop or reach completion/retry.
- It does not inject MIDI performance input.
- It does not capture or compare GPU-rendered screenshots.
- `QA-001` remains open until loop and real input/output behavior are covered.

## 2026-07-25 — Cycle 032: Loopback debug practice driver (DONE)

### Outcome

A separate local test process can now launch a Debug build, enter a loaded
song, operate stable practice controls and assert semantic state. The path no
longer depends on mouse coordinates or translated labels.

### Implemented

- Added an opt-in TCP debug driver controlled by
  `NEOTHESIA_DEBUG_DRIVER_ADDR`.
- Allowed only explicit IPv4 or IPv6 loopback socket addresses.
- Kept the listener disabled when the environment variable is absent.
- Limited commands to 4096 bytes and action/snapshot waits to two seconds.
- Added newline-delimited `ACTION <id>` and `SNAPSHOT` commands.
- Returned compact JSON results suitable for an external smoke-test process.
- Added `practice.menu.start` to the stable semantic catalogue and visible Play
  control.
- Let the menu accept start only when a song is actually loaded.
- Kept the entire listener module and its startup path out of release builds.
- Documented the protocol and its intentionally narrow security boundary.

### Verification

- Tested valid commands, malformed commands and stable JSON scalar output.
- Tested acceptance of `127.0.0.1` and `::1`.
- Tested rejection of wildcard and LAN addresses.
- Ran a real Debug application with “Look at the Sky - Porter Robinson,
  original key, auto-aligned”:
  - menu snapshot returned `null`, as designed;
  - `practice.menu.start` returned accepted;
  - player snapshot reported wait mode on and both hands;
  - `practice.player.wait` returned accepted;
  - the next snapshot reported wait mode off;
  - `practice.player.back` returned accepted.
- Removed the temporary practice-history file created by the smoke run.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-eight
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `eaf40f3`
(`feat: expose local debug practice driver`).

### Known limitations

- The smoke sequence is proven manually from a test process but is not yet a
  checked-in reusable launcher.
- The driver does not capture rendered frames.
- Calibration and recommendation actions remain click-only.
- The protocol is intentionally single-command-per-connection and local-only.

## 2026-07-25 — Cycle 031: Confirmed semantic action dispatch (DONE)

### Outcome

An automation caller can now distinguish “the action was accepted by the
active scene” from “the message was merely placed on the event queue.” This
removes a major source of false-positive native UI tests.

### Implemented

- Added a one-shot reply channel to every debug semantic action event.
- Returned the active scene's accepted/rejected decision to the caller.
- Added a caller-supplied timeout to prevent indefinite waits.
- Requested a redraw only after an accepted action.
- Documented that activation must run on a worker thread.
- Updated the automation contract and removed action acknowledgement from the
  remaining work list.

### Verification

- Existing tests cover supported and unsupported semantic action mappings.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-five
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `19a7d13`
(`feat: acknowledge debug practice actions`).

### Known limitations

- The acknowledgement is available only to in-process debug callers.
- No external local driver endpoint exists yet.
- Deterministic screenshot capture and parameterized completion actions remain
  future work.

## 2026-07-25 — Cycle 030: Debug practice automation harness (DONE)

### Outcome

Debug builds can now drive important practice controls by stable semantic ID
and inspect learner-facing state without screen coordinates. Actions travel
through the real application event loop and reuse the same behavior as visible
buttons.

### Implemented

- Added a debug-only event-loop proxy harness.
- Added semantic action and practice snapshot application events.
- Routed the active scene's debug requests through the normal event loop.
- Exposed wait mode, Tempo Coach, hand scope, completion tab, matched/wrong/
  missed counts and input-latency compensation in a read-only snapshot.
- Activated player back, wait, coach and hands by semantic ID.
- Activated completion Overview, Technique, History, retry and back by semantic
  ID when the completion screen is active.
- Extracted shared wait, coach and retry methods so automation cannot drift
  from visible button behavior.
- Added an explicit supported-action parser and mapping test.
- Kept calibration and recommendation actions unsupported until their
  parameterized product logic can be shared safely.
- Compile-time excluded the harness, events and snapshot from release builds.
- Updated the native UI automation contract.

### Verification

- Verified the supported semantic mapping and rejected unsupported IDs.
- Verified the complete stable-ID catalogue remains unique and namespaced.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-five
application tests. Clippy and release builds report only the repository's
pre-existing platform-helper and `unused_mut` warnings.

Implementation commit: `d428acf`
(`feat: add debug practice automation harness`).

### Known limitations

- The harness is currently in-process; no controlled external driver endpoint
  exists yet.
- Action dispatch reports queueing success, not whether the active scene
  accepted the action.
- Deterministic GPU screenshot capture is not implemented.
- Calibration and recommendation actions remain click-only.
- OS accessibility still depends on future Nuon platform integration.

## 2026-07-25 — Cycle 029: Stable practice action identities (DONE)

### Outcome

Core practice controls now have stable semantic identities independent of
English labels and screen coordinates. This establishes the first dependable
boundary for native UI automation.

### Implemented

- Defined a namespaced practice action catalogue.
- Assigned IDs to player back, wait, adaptive coach and hand-mode controls.
- Assigned IDs to all three completion tabs.
- Assigned IDs to calibration, note-loop, rhythm-loop, retry and back actions.
- Kept the same hand-mode ID across responsive toolbar placements.
- Added a uniqueness and namespace test for the complete catalogue.
- Added `docs/development/ui-automation.md` as the automation contract.
- Documented that Nuon IDs are currently in-process identities, not a
  fabricated Windows accessibility implementation.
- Defined the next debug-only activation/state/screenshot boundary.

### Verification

- Verified every catalogue ID is unique and begins with `practice.`.
- Existing button behaviour, completion navigation and responsive layout tests
  pass with explicit IDs.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-four
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `a549e07`
(`feat: add stable practice action identities`).

### Known limitations

- IDs are not externally discoverable yet.
- There is no debug command channel to activate an action by ID.
- Semantic state assertions and deterministic GPU screenshots remain future
  layers.
- OS accessibility still depends on future Nuon platform integration.

## 2026-07-25 — Cycle 028: Completion feedback information architecture (DONE)

### Outcome

The completion screen is no longer one dense wall of metrics. Overview,
Technique and History now have separate, predictable responsibilities while
the primary retry/back controls remain available everywhere.

### Implemented

- Replaced the two-tab header with Overview, Technique and History.
- Kept accuracy, counts, hand accuracy, weak measures and focused-loop actions
  in Overview.
- Moved timing profile, calibration confirmation, hand timing, chord
  synchronization, dynamics, pedal and key-hold evidence into Technique.
- Kept saved-session trends and weak-history detail in History.
- Hid note/rhythm recommendation actions outside Overview.
- Kept Practice again and Back to songs available on every tab.
- Added an explicit disabled-state explanation when Expression Summary is off.
- Added compact header copy for panels below 600 px.
- Calculated a fixed three-tab layout that fits the 480 px minimum panel.
- Added left/right arrow-key cycling with wraparound.

### Verification

- Added exact minimum-width tab-boundary coverage.
- Added forward/backward keyboard-cycle coverage.
- Existing feedback copy, recommendation, calibration and history tests pass.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-three
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `a8bcf89`
(`feat: organize completion feedback tabs`).

### Known limitations

- Layout geometry is tested, but a captured GPU-rendered completion fixture is
  not automated yet.
- Tab selection is not persisted, intentionally returning each take to
  Overview.
- The custom GPU UI still lacks stable semantic action IDs for external UI
  automation.

## 2026-07-25 — Cycle 027: Block-chord synchronization evidence (DONE)

### Outcome

Completed takes now describe how closely the attacks of written block chords
landed together. Written arpeggios and incomplete chords are protected from
misleading synchronization statistics.

### Implemented

- Grouped score targets only when their exact MIDI onset and measure match.
- Required at least two target notes for a chord candidate.
- Excluded unknown-context matcher events from chord grouping.
- Counted complete and incomplete target chords separately.
- Calculated attack span from the earliest to latest matched note.
- Included only fully matched chords in median and maximum span statistics.
- Required four complete chords before presenting an aggregate profile.
- Added a compact completion line with explicit “descriptive” wording.
- Persisted chord evidence with a backward-compatible default.
- Kept different-onset written arpeggios outside the chord evidence entirely.

### Verification

- Added a deterministic fixture with four complete block chords at 20, 30, 40
  and 50 ms attack spans.
- Verified the even-sample median is 35 ms and maximum is 50 ms.
- Verified one missing-note chord is counted incomplete and excluded from span.
- Verified three notes written 40 ms apart are not treated as a chord.
- Added learner-facing no-evidence, insufficient-evidence and profile copy
  tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, fifty core tests and forty-one
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `01289b0`
(`feat: summarize block chord synchronization`).

### Known limitations

- A MIDI author can encode an intended roll with identical onsets; Neothesia
  therefore describes span and never labels it correct or wrong.
- The profile is whole-attempt rather than separated by hand.
- Chord-size and register-specific expectations are not modeled.
- The completion screen is now information-dense and needs a dedicated
  technique tab.

## 2026-07-25 — Cycle 026: Reliable rhythm trouble spots (DONE)

### Outcome

Neothesia now distinguishes note-accuracy problems from rhythm-stability
problems at measure level. A learner can start a focused loop for either reason
from two separate, evidence-labelled actions.

### Implemented

- Added backward-compatible robust timing profiles to measure summaries.
- Aggregated raw matched-note offsets independently for every measure.
- Combined repeated attempts using medians of per-attempt bias and deviation.
- Kept only attempts with at least four timing samples in that measure.
- Required at least two attempts and twelve cumulative matched notes.
- Flagged a rhythm problem only at ≥60 ms median bias or ≥40 ms median
  deviation.
- Ranked reliable measures by transparent bias-plus-deviation severity.
- Scoped evidence to the latest session kind and hand goal.
- Added a dedicated purple Rhythm action beside the orange Notes action.
- Split the recommendation row cleanly when both actions are available.
- Started the selected two-measure rhythm loop with an explicit focus toast.

### Verification

- Added repeated-evidence, insufficient-evidence and stable-measure tests.
- Added scope isolation between whole-song and loop sessions.
- Verified exact measure timing aggregation and old-summary migration.
- Preserved the existing note-accuracy recommendation tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, forty-nine core tests and forty
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `ffdb072`
(`feat: recommend weak rhythm measures`).

### Known limitations

- Cross-attempt aggregation uses per-attempt medians rather than retaining every
  raw historical note offset.
- Thresholds are fixed policy constants and are not level-specific yet.
- Recommended passages remain two measures long.
- Chord simultaneity and intentional rolled chords are not distinguished yet.

## 2026-07-25 — Cycle 025: Left/right-hand timing profiles (DONE)

### Outcome

The completion screen can now show whether the two hands have different timing
biases or consistency. Each hand must earn its own evidence; notes from the
other hand never fill its sample requirement.

### Implemented

- Retained raw signed timing offset on every matched structured result.
- Kept wrong and missed results explicitly free of timing offsets.
- Aggregated timing offsets independently by practice part.
- Added a backward-compatible timing profile to persisted part summaries.
- Required eight matched notes per hand.
- Displayed median early/late bias and typical spread for each qualified hand.
- Displayed the exact number of additional notes needed for an under-sampled
  hand.
- Preserved the existing right/left accuracy line above the timing comparison.
- Kept ambiguous/custom parts out of left/right claims.

### Verification

- Added a deterministic 16-note two-hand fixture with distinct 20 ms and 60 ms
  hand biases.
- Proved both profiles retain zero within-hand deviation independently.
- Added migration coverage for old part summaries without timing data.
- Added UI-copy coverage where one hand qualifies and the other does not.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, forty-six core tests and forty
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `c153721`
(`feat: compare left and right hand timing`).

### Known limitations

- The view describes each hand; it does not yet generate a hand-specific
  practice assignment.
- MIDI files without reliable left/right track inference remain unlabelled.
- Timing is aggregated over the attempt, so measure-level rhythm trouble spots
  are not yet visible.
- Cross-hand chord synchronization is not measured separately.

## 2026-07-25 — Cycle 024: Conservative calibration suggestions (DONE)

### Outcome

Neothesia can now turn a stable timing profile into an explicit input-offset
suggestion. It never changes calibration silently: the pianist must confirm the
displayed value, after which the same piece restarts for verification.

### Implemented

- Added a pure, explainable calibration policy.
- Required at least 24 matched notes.
- Rejected takes whose median absolute deviation exceeds 35 ms.
- Treated a median bias below 10 ms as already centered.
- Capped each confirmed correction to 50 ms.
- Respected the global ±250 ms safety bounds.
- Explained insufficient, unstable, centered and limit states in the timing
  profile line.
- Added an inline `Apply … & retry` action only when all gates pass.
- Persisted the confirmed offset, updated the current player and immediately
  restarted a clean attempt.
- Kept raw MIDI/audio forwarding completely outside the calibration path.

### Verification

- Added threshold tests at 23/24 samples and 35/36 ms deviation.
- Added deadband, positive/negative correction, step-cap and global-limit tests.
- Updated learner-facing wording coverage.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, forty-four core tests and thirty-nine
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `27ae4f2`
(`feat: suggest conservative timing calibration`).

### Known limitations

- Suggestions use a normal musical take rather than a dedicated metronome
  calibration exercise.
- Stable intentional rubato can still resemble route latency; explicit
  confirmation and immediate verification are therefore mandatory.
- Calibration remains global rather than per MIDI device/route.
- The current evidence is whole-attempt rather than hand-specific.

## 2026-07-25 — Cycle 023: Robust timing profile (DONE)

### Outcome

The completion screen now distinguishes a consistent early/late bias from
general timing inconsistency. This is more useful than counts alone and
provides a defensible evidence layer for guided latency calibration.

### Implemented

- Preserved the signed millisecond offset of every matched note before the
  on-time grade normalized it.
- Defined negative values as early and positive values as late.
- Calculated median signed bias.
- Calculated median absolute deviation from that bias as a robust consistency
  measure.
- Required eight matched notes before presenting a profile.
- Kept the existing learner-facing early/on-time/late counts.
- Added a compact completion line using plain “median” and “typical spread”
  wording.
- Persisted timing profiles with backward-compatible defaults.
- Cleared timing samples between attempts.

### Verification

- Added exact signed-offset, median and deviation tests.
- Verified on-time notes retain their original small early/late offset.
- Added empty-evidence and learner-facing wording coverage.
- Existing legacy-summary coverage now verifies the timing default.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, forty-two core tests and thirty-nine
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `f69d51d`
(`feat: add robust practice timing profile`).

### Known limitations

- The completion line is descriptive and does not yet propose a calibration.
- One take can contain intentional rubato, so timing bias must not be treated
  automatically as device latency.
- Profiles are whole-attempt aggregates; hand and measure timing profiles are
  not persisted separately yet.
- “Typical spread” is median absolute deviation, not a percentile guarantee.

## 2026-07-25 — Cycle 022: Input-latency compensation (DONE)

### Outcome

Pianists can now correct a consistent keyboard/driver timing delay without
delaying sound or modifying MIDI sent to Pianoteq. Timing judgement uses the
compensated timestamp; monitoring remains immediate.

### Implemented

- Added a persistent input timing offset with backward-compatible zero default.
- Added a Settings control in 5 ms steps.
- Bounded the adjustment to `-250..=250 ms`.
- Applied positive offsets by moving judgement timestamps earlier.
- Supported negative offsets for unusual routes that require later judgement.
- Used saturating duration arithmetic at session start.
- Applied the same constant offset to note-on and note-off so physical key-hold
  duration is unchanged.
- Kept raw live MIDI forwarding ahead of and independent from assessment.
- Displayed every non-zero active offset in the player status line.

### Verification

- Added default, legacy-settings and clamp tests.
- Added a real player-path test where a 200 ms arrival with `+120 ms`
  compensation lands exactly on the 80 ms on-time boundary.
- Covered positive underflow and negative adjustment explicitly.
- Existing exact expressive-MIDI forwarding tests continue to pass.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, forty-one core tests and thirty-eight
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `94fb28c`
(`feat: compensate practice input latency`).

### Known limitations

- The offset is manual; there is no guided tap-to-calibrate workflow yet.
- One global value is used for every MIDI input/output route.
- Audio output latency is intentionally not delayed or estimated.
- A timing profile is still needed before the app can propose an offset from
  repeated evidence.

## 2026-07-25 — Cycle 021: Key-hold duration evidence (DONE)

### Outcome

Completed takes now show how long matched keys were physically held relative
to the reference MIDI. This makes staccato/legato investigation possible
without pretending that raw MIDI duration alone determines good articulation.

### Implemented

- Carried each parsed reference note duration into its practice target.
- Indexed duration lookup by track, pitch and score onset when opening a song.
- Paired live note-on and note-off events with matched score occurrences.
- Preserved duration evidence when a pianist played and released slightly
  before the score onset.
- Kept repeated-pitch occurrences in FIFO order.
- Calculated a robust median key-hold ratio rather than an outlier-sensitive
  mean.
- Exposed transparent `<75%`, `75–125%` and `>125%` counts.
- Required four completed notes before showing the aggregate comparison.
- Explicitly excluded sustain-pedal extension from physical key-hold duration.
- Cleared unfinished hold state on transport/panic resets.
- Added backward compatibility for Cycle 020 expression records.

### Verification

- Extended deterministic matcher coverage with four known duration ratios.
- Covered early-release pairing and median/band calculations.
- Added nested expression-history compatibility coverage.
- Extended the player integration test to prove parsed score duration, live
  release timing and unmodified output messages travel through the real path.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, thirty-nine core tests and thirty-seven
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `91fd14f`
(`feat: add key-hold duration evidence`).

### Known limitations

- The ratio describes physical key hold, not acoustic note length under sustain.
- Reference MIDI note lengths may be quantized or edited and are not treated as
  authoritative phrasing.
- The three bands expose distribution; they are not a pass/fail grade.
- Timing evidence still assumes that input-device latency is negligible.

## 2026-07-25 — Cycle 020: Descriptive expression evidence (DONE)

### Outcome

The completion screen now reports how the pianist's matched-note velocity range
compared with the MIDI reference and whether sustain-pedal data was present.
The language stays deliberately descriptive: MIDI velocity and pedal event
counts are evidence, not proof of good tone or correct pedalling.

### Implemented

- Captured played velocity alongside every matched live note.
- Preserved score velocity in structured practice targets.
- Aggregated matched samples, mean absolute velocity gap and played/reference
  ranges.
- Captured CC64 use, value transitions and continuous half-pedal samples for
  both the score and live input.
- Added an optional, default-on Expression Summary setting.
- Added compact dynamics and pedal lines to the completion screen.
- Required four matched velocity samples before presenting a range comparison.
- Persisted expression evidence in practice sessions with legacy-history
  defaults.
- Kept the external MIDI/Pianoteq route byte-for-byte unchanged.

### Verification

- Added deterministic velocity pairing, pedal evidence and reset tests.
- Added legacy practice-history deserialization coverage.
- Added player integration coverage for score/live evidence and exact external
  MIDI forwarding.
- Added learner-facing wording coverage for incomplete references.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed: two MIDI-file tests, thirty-eight core tests and thirty-seven
application tests. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `1a04bc2`
(`feat: summarize dynamics and pedal evidence`).

### Known limitations

- Velocity comparison is not calibrated for the player's keyboard curve,
  Pianoteq preset or listening level.
- Pedal feedback counts evidence and transitions; it does not yet align pedal
  timing to notes, harmonies or score intervals.
- MIDI reference velocity may be mechanical or normalized and is not treated
  as an artistic ground truth.
- Expression evidence does not yet influence spaced-review scheduling.

## 2026-07-25 — Cycle 019: Explainable spaced review (DONE)

### Outcome

Neothesia can now distinguish “needs work now” from “successfully learned;
check retention later.” The schedule is local, conservative and visible: every
due label follows directly from measured note/timing performance rather than an
opaque engagement score.

### Implemented

- Added pure review-status and reason models.
- Reused the established 90% accuracy and 70% on-time mastery thresholds.
- Counted consecutive mastered attempts only inside the latest practice scope.
- Added 1/3/7/14-day review intervals.
- Made weak and evidence-free attempts immediately due.
- Added ceiling-rounded days-remaining calculations.
- Added All, Queue and Due library modes.
- Ordered due work by due timestamp.
- Added compact explanatory labels for reinforcement, retention and waiting.
- Kept queue insertion available directly beside every due item.

### Verification

- Added mastery-streak, interval, due-boundary, immediate-reinforcement and
  UI-explanation tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `c5ee410`
(`feat: add explainable spaced review`).

### Known limitations

- Review intervals are fixed policy bands, not user-configurable yet.
- The schedule uses note/timing mastery; pedal, dynamics and duration evidence
  are not included yet.
- A take with no judged notes is conservatively due but still needs a proper
  calibration/session workflow.
- Queue insertion remains explicit rather than automatic by design.

## 2026-07-25 — Cycle 018: Favourites and ordered practice queue (DONE)

### Outcome

The local library now supports an intentional daily practice list instead of
being only a searchable catalogue. Learners can pin repertoire, build a short
ordered queue and see the latest measured result or reliable weak passage
before opening a piece.

### Implemented

- Added backward-compatible per-song library organization state.
- Added persistent favourite mutation and favourite-first library ordering.
- Added queue insertion, removal, normalization and boundary-safe movement.
- Added a focused Queue/All Pieces view toggle.
- Added row-level favourite and queue actions.
- Added explicit up/down controls in queue view.
- Created organization records for indexed songs before their first practice.
- Preserved known source paths for organized but unopened songs.
- Included reliable weak-measure recommendations in library rows.
- Kept all organization anchored to content identity.

### Verification

- Added persistence, queue-order, normalization, boundary and migration tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `936b95c`
(`feat: add favourites and practice queue`).

### Known limitations

- Queue ordering is manual; spaced-review due work is not inserted
  automatically.
- There is no bulk clear or drag-and-drop ordering yet.
- Favourites currently affect ordering but do not have a separate-only filter.
- Musical metadata still comes from filenames and paths.

## 2026-07-25 — Cycle 017: Watched-folder indexing and search (DONE)

### Outcome

The Practice Library can now discover a real MIDI collection instead of only
remembering files opened in the past. Scanning does not freeze the renderer,
duplicates do not clutter the list, and a learner can find a piece using any
combination of title or folder terms.

### Implemented

- Added a tested `library` domain independent of scene rendering.
- Added persistent watched-folder configuration with backward-compatible
  defaults and path-aware duplicate prevention.
- Added recursive MIDI discovery without following directory symlinks.
- Parsed candidates on a named worker thread.
- Counted and reported unreadable MIDI while retaining valid results.
- Deduplicated files by content identity and retained alternate source paths.
- Cached normalized searchable text in the index.
- Added type-anywhere multi-term search over filenames and full paths.
- Merged indexed pieces with recent sessions and latest accuracy.
- Added Add Folder and Refresh actions in Practice Library.
- Added watched-folder add/remove controls in Settings.
- Kept direct open and content-verified missing-file repair.

### Verification

- Added recursive scan, duplicate-content, malformed-file, search, config
  uniqueness and settings-migration tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `e993f2c`
(`feat: index and search watched MIDI folders`).

### Known limitations

- The index is rebuilt when the library is opened or refreshed; an incremental
  disk cache and filesystem watcher are future performance work.
- Search currently uses filename and path because editable musical metadata is
  not implemented yet.
- Very large first scans cannot yet be cancelled, although they remain off the
  render thread.
- Favourites, ordered practice queues and mastery filters are next.

## 2026-07-25 — Cycle 016: Recent practice library (DONE)

### Outcome

Previously practised MIDI files are now reachable from inside Neothesia instead
of relying on the operating-system picker every time. A moved file can be
relinked safely, and choosing a different file cannot silently attach the old
practice record to it.

### Implemented

- Added source-path provenance to parsed MIDI files and saved song setup.
- Added last-used timestamps and a sorted recent-song query.
- Added a scrollable Practice Library page and home-screen entry.
- Displayed session count and latest accuracy.
- Loaded recent songs away from the render thread.
- Added missing-file detection and Locate recovery.
- Compared the selected replacement's BLAKE3 content identity before opening.
- Updated the source path only after successful verification.
- Added clear in-page feedback for missing, unreadable and mismatched files.
- Added Unicode-safe title shortening and a more compact home layout.

### Verification

- Added source-path, ordering and Unicode-label tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `e9338da`
(`feat: add recent practice library`).

### Known limitations

- The library currently contains pieces that have already been opened; watched
  folder discovery and text search are next.
- Legacy records without a saved source path require one successful manual
  reopen before they appear as directly available.
- Favourites, queues, metadata editing and bulk missing-file repair remain
  future work.

## 2026-07-25 — Cycle 015: Per-song practice setup (DONE)

### Outcome

Reopening the same MIDI now resumes the learner's working context instead of
silently returning to generic defaults. Renaming or moving the file does not
break the association because the setup uses MIDI content identity.

### Implemented

- Added serializable track, loop and song-setup records.
- Stored setup alongside versioned practice history using the existing atomic
  writer and corruption quarantine.
- Restored mute/automatic/human track roles and waterfall visibility.
- Restored exact playback speed, including zero-speed study state.
- Stored loop boundaries as inclusive one-based measure numbers.
- Restored active loops with count-in and retained disabled loop ranges.
- Saved player changes at every explicit interaction point and adaptive-coach
  speed change.
- Guarded track restoration by exact track-ID structure.
- Guarded loop restoration against invalid or stale measure ranges.
- Maintained compatibility with files that predate setup and hand-scope fields.

### Verification

- Added content-identity, rename, track-structure, loop-boundary and migration
  tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `f1a566f`
(`feat: restore per-song practice setup`).

### Known limitations

- Saved songs do not yet appear in a library browser; reopening still starts
  from the file picker or last-opened path.
- Track layouts are restored only for an exact track-ID structure. A changed
  MIDI is correctly treated as a different song.
- History/setup reset and export controls remain future work.

## 2026-07-25 — Cycle 014: In-player hand practice modes (DONE)

### Outcome

A learner can now move from both-hands practice to right-hand or left-hand work
without leaving the player. The other hand remains musical accompaniment, and
the visible goal, attempt state and saved progress all change together.

### Implemented

- Added a shared `PracticeHands` domain model.
- Added conservative hand-mode detection and mutation to song configuration.
- Added a purple `Hands` player control with responsive placement.
- Kept the opposite hand on automatic playback and preserved unrelated tracks.
- Restarted whole-song practice or the active loop with its count-in.
- Reset keyboard, matcher, attempt comparison and tempo-coach state.
- Disabled shortcuts for MIDI arrangements without reliable hand assignments.
- Added hand scope to the backward-compatible version-one history record.
- Scoped trends, persistent weak measures and recommendations to the latest
  hand goal.
- Exposed hand scope in recent-attempt and trend labels.

### Verification

- Added hand-mode, ambiguous-assignment, scope-isolation and legacy-history
  migration tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings. The full-workspace Clippy command
still requires local FFmpeg development packages for the optional video crate.

Implementation commit: `f630ea4`
(`feat: add in-player hand practice modes`).

### Known limitations

- Automatic hand assignment remains intentionally conservative: MIDI files
  with more than two playable note tracks require explicit track setup.
- The selected hand mode, loop and speed are not yet restored after closing
  the song; this is the next cycle.
- Physical visual and Pianoteq smoke testing still requires the user's active
  desktop and connected instrument.

## 2026-07-25 — Cycle 013: Active output visibility (DONE)

### Outcome

The player now answers “what is producing my sound?” continuously. Built-in
SoundFont, external MIDI/Pianoteq routing and accidental silence have distinct
labels and colours, and the same badge is the shortest path to correcting the
selection.

### Implemented

- Exposed the active output descriptor from the output manager.
- Added backend and detailed learner-facing status labels.
- Added a persistent green, blue or red output badge.
- Displayed the MIDI port name where space permits.
- Added responsive compact mode and removed colliding statistics on narrow
  windows.
- Added Unicode-safe device-label shortening.
- Added a direct player-to-settings event and settings-page constructor.
- Preserved the loaded song across the transition.

### Verification

- Added output-state, default-device and label-truncation tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `41f3f95`
(`feat: show active output with settings shortcut`).

### Known limitations

- Changing an output in settings takes effect when playback is started again;
  live hot-swapping inside an active player remains intentionally unsupported.
- Very long MIDI port names are shortened in the badge but remain complete in
  settings.
- Physical Pianoteq confirmation still requires the documented soak test.

## 2026-07-25 — Cycle 012: Global emergency panic (DONE)

### Outcome

A stuck external note or pedal now has an immediate, discoverable recovery
path. The red player action remains visible while the toolbar is collapsed, and
F12 performs the same emergency stop in every active scene.

### Implemented

- Added the persistent `PANIC F12` player control.
- Added application-level F12 interception before scene key handling.
- Added a scene emergency-stop contract with a safe default.
- Paused song/preview playback and cleared transient practice matching.
- Cancelled visual count-in and reset keyboard plus mouse-held state.
- Cleared free-play chord state.
- Corrected zero-velocity visual releases.

### Verification

- Added emergency-stop state and zero-velocity visual tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commits:

- `2b52463` (`feat: add persistent emergency MIDI panic`)
- `235be25` (`fix: make F12 panic global across scenes`)

### Known limitations

- The visible button is player-specific; free-play uses the globally documented
  F12 shortcut.
- Panic intentionally pauses playback. The learner chooses when to resume or
  restart the current attempt.
- Hardware confirmation still belongs to the Pianoteq soak checklist.

## 2026-07-25 — Cycle 011: Expressive MIDI fidelity (DONE)

### Outcome

The automated boundary now proves that Pianoteq-relevant expressive MIDI is not
silently reduced to notes and binary pedal events. Live input and guided MIDI
playback retain their original values through the external-output boundary.

### Implemented

- Made the deterministic test output capture complete MIDI messages.
- Added live-input forwarding tests.
- Added a parsed synthetic human-track fixture for wait-mode forwarding.
- Covered intermediate CC64 values 23 and 91.
- Covered non-centred 14-bit Pitch Bend and Channel Aftertouch.
- Added exact MIDI wire-byte assertions for controller, bend and pressure.

### Verification

- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `2fda08d`
(`test: prove expressive MIDI output fidelity`).

### Known limitations

- The automated output records and serializes MIDI but cannot hear or inspect a
  physical Pianoteq instance.
- Polyphonic aftertouch, release velocity and every possible controller are not
  exhaustively enumerated; the generic forwarding path is shared.
- Pedal quality feedback is not yet part of practice scoring.

## 2026-07-25 — Cycle 010: External MIDI and Pianoteq safety (DONE)

### Outcome

Pause, seek, restart, teardown and output replacement now use a conservative
silence path suitable for Pianoteq and other external instruments. Pedal-held
sound is no longer left to tracked Note Off events alone.

### Implemented

- Corrected active-note tracking for zero-velocity Note On releases.
- Retained explicit Note Off messages for tracked active notes.
- Added CC64 pedal-up, CC123 All Notes Off, CC120 All Sound Off and CC121 Reset
  All Controllers.
- Sent the panic sequence on all 16 MIDI channels.
- Stopped the old output before selecting a replacement.
- Corrected shared MIDI-connection drop behaviour.
- Added a test output recorder for transport lifecycle verification.
- Added the external Pianoteq routing and acceptance guide.

### Verification

- Added active-note, panic order, channel coverage and player lifecycle tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `daf86c3`
(`fix: harden external MIDI output lifecycle`).

### Known limitations

- Automated tests verify generated MIDI events but cannot prove the behaviour
  of the user's virtual cable and Pianoteq installation.
- The 30-minute device soak checklist remains a manual acceptance gate.
- Native VST3 hosting is still a separate later phase.

## 2026-07-25 — Cycle 009: Current-song practice history (DONE)

### Outcome

The completion experience now distinguishes immediate feedback from long-term
progress. A pianist can inspect recent comparable attempts and persistent weak
measures without leaving the song or reading the raw local history file.

### Implemented

- Added a `This take / History` segmented control to the completion card.
- Added a reusable current-song history overview in the practice domain.
- Listed four recent attempts with scope, accuracy and speed.
- Added accuracy and speed change across comparable attempts.
- Prevented whole-song and loop sessions from being compared as one trend.
- Added a two-column ranking of persistent weak measures.
- Reused the conservative evidence threshold for learner-facing rankings.
- Preserved the recommended-loop action on both tabs.

### Verification

- Added overview ordering, delta, insufficient-sample and mixed-scope tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `ccf3b2f`
(`feat: add current song practice history view`).

### Known limitations

- History is currently entered from whole-song completion, not the player or
  song library.
- Trends are intentionally simple first-to-last deltas rather than a chart or
  statistical confidence estimate.
- Export and reset controls remain deferred until their confirmation and backup
  behaviour is designed.

## 2026-07-25 — Cycle 008: Evidence-based weak-passage practice (DONE)

### Outcome

Persisted practice results now produce a concrete next action. After enough
evidence accumulates, the completion panel explains the weakest passage and can
start a correctly bounded, counted-in loop with one action.

### Implemented

- Added deterministic weak-passage recommendations to the practice domain.
- Required two attempts, eight judged notes and accuracy below 90%.
- Included accuracy, sample size and take count in recommendation evidence.
- Chose a two-measure practice range and clamped it at the song boundary.
- Added a visually prominent recommendation action above the normal completion
  actions.
- Converted one-based inclusive measure ranges into exact MIDI loop boundaries.
- Reused structured attempt reset and count-in behaviour for recommended loops.

### Verification

- Added recommendation threshold and measure-boundary mapping tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `e1d80d0` (`feat: recommend weak passage loops`).

### Known limitations

- The suggestion is available at whole-song completion, not yet from the song
  library or a dedicated history view.
- It uses note accuracy only; timing, pedal and dynamics goals remain separate
  future coaching dimensions.
- Recommended passages are currently two measures long and are not yet editable
  before starting.

## 2026-07-25 — Cycle 007: Durable local practice history (DONE)

### Outcome

Practice results now survive application restarts and MIDI file renames. The
learner's completed attempts form a safe local evidence base for future
weak-passage recommendations instead of disappearing at the end of playback.

### Implemented

- Added BLAKE3 content identities to loaded and generated MIDI files.
- Added a versioned RON schema for whole-song and measure-loop sessions.
- Stored attempt time, speed, overall totals, measures and hand results.
- Recorded loop attempts at each boundary and whole-song attempts at
  completion.
- Added a 200-session per-song retention limit.
- Added deterministic aggregation and ranking of weak measures.
- Wrote history through a temporary file and atomically replaced the live file.
- Used Windows write-through replacement and quarantined malformed files.
- Displayed the saved session count in the completion panel.

### Verification

- Added persistence, rename-continuity, corruption, retention and aggregation
  tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing
platform-helper and `unused_mut` warnings.

Implementation commit: `039a93d` (`feat: persist local practice history`).

### Known limitations

- History has no learner-facing browser, export or reset control yet.
- Weak-measure aggregation exists in the practice domain but is not yet exposed
  as a practice recommendation.
- History follows MIDI content; edited MIDI data intentionally creates a new
  identity.

## 2026-07-25 — Cycle 006: Adaptive tempo coach (DONE)

### Outcome

Loop practice can now progress speed conservatively without taking control away
from the learner. Every automatic decision is bounded, explainable and optional.

### Implemented

- Added persisted adaptive-coach configuration with an opt-in default.
- Added a prominent in-player coach switch.
- Added settings for mastery accuracy and coached speed limits.
- Required two consecutive mastered takes before a 5% increase.
- Required both note accuracy and on-time consistency for mastery.
- Added a 5% regression only for attempts below the safety threshold.
- Held speed for intermediate results and at configured bounds.
- Reset the coach streak whenever the user manually changes speed.
- Added explicit messages explaining each decision for three seconds.
- Fixed playback scaling so 5% steps are not truncated into 10% buckets.
- Rejected non-finite manual speed values in configuration.

### Verification

- Added deterministic coach-decision and exact-speed tests.
- `cargo test -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing platform
helper and `unused_mut` warnings.

Implementation commit: `be7e77e` (`feat: add adaptive tempo coaching`).

### Known limitations

- Coaching is intentionally limited to structured loop attempts.
- Session history and coach progression are not yet persisted per song.
- Decision explanations are textual; richer visual trend feedback is planned.

## 2026-07-25 — Cycle 005: Structured loop practice (DONE)

### Outcome

The timeline loop now behaves as a deliberate-practice tool. Each repetition
starts cleanly, gives preparation time and can be compared with the previous
and best takes.

### Implemented

- Added reusable attempt history with deterministic best-take rules.
- Snapped loop handles to MIDI measure boundaries.
- Chose a two-measure default range around the current playback position.
- Added a one-measure visual four-count before each take.
- Reset practice matching at each loop boundary.
- Displayed take number plus last and best accuracy.
- Added a measure-range label inside the highlighted loop.
- Cleared attempt history when the practised range changes.
- Fixed playback seek semantics to retain events exactly on the target
  boundary, preventing the first beat of a loop from disappearing.

### Verification

- Added attempt-history, snapping, count-in and exact-boundary seek tests.
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing platform
helper and `unused_mut` warnings.

Implementation commit: `243829b`
(`feat: turn loops into structured practice attempts`).

### Known limitations

- The count-in is visual and does not yet produce metronome clicks.
- Attempt history is in-memory until practice-history persistence is delivered.
- The loop workflow still needs a multi-DPI visual smoke test.

## 2026-07-25 — Cycle 004: Measure-aware attempt summary (DONE)

### Outcome

Finishing a song now produces actionable practice feedback instead of silently
returning to the menu. The learner can see which hand and which measures need
another attempt.

### Implemented

- Added structured target context: pitch, score time, track, measure and part.
- Added retained matched, missed and context-attributed wrong-note results.
- Added per-measure and per-hand aggregation.
- Inferred left/right hand for two playable piano tracks from pitch center.
- Kept arrangements with more than two playable tracks unassigned rather than
  guessing incorrectly.
- Replaced automatic song exit with a native completion overlay.
- Displayed total accuracy, timing categories, wrong/missed notes, hand
  accuracy and the four weakest measures.
- Added immediate retry and return-to-song actions.

### Verification

- Added aggregation, hand inference and real-MIDI integration tests.
- `cargo test -p neothesia-core -p neothesia --all-targets`
- `cargo clippy -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Clippy reports only the repository's pre-existing platform
helper and `unused_mut` warnings.

Implementation commit: `715ed9c`
(`feat: add measure-aware practice summaries`).

### Known limitations

- Summaries exist only for the current process and are not persisted.
- More-than-two-part arrangements require future explicit part assignment.
- The completion overlay still needs a visual smoke test on several DPI scales.

## 2026-07-25 — Cycle 003: Repeated and missed notes (DONE)

### Outcome

Practice results no longer lose repeated same-pitch presses, and flow practice
can distinguish a note that was never played from an unrelated wrong key.

### Implemented

- Replaced one-entry-per-pitch maps with FIFO occurrence queues.
- Matched successive and overlapping same-pitch occurrences independently.
- Added a configurable late-match window and missed-note finalization.
- Fed human-track targets into the matcher in both guided and flow practice.
- Kept guided targets pending while allowing flow targets to expire as missed.
- Included missed and still-needed counts in the live top-bar status.

### Verification

- Added tests for repeated presses, overlapping targets and missed finalization.
- Added application integration tests proving wait mode does not create misses
  and flow mode does.
- `cargo test -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Existing compiler warnings are unchanged and unrelated.

Implementation commit: `fbee89d`
(`feat: score repeated and missed practice notes`).

### Known limitations

- Match outcomes do not yet carry track, hand, score time or measure identity.
- Session totals are not yet finalized into a summary or saved.

## 2026-07-25 — Cycle 002: Deterministic practice feedback (DONE)

### Outcome

Guided practice now has a deterministic, reusable source of live feedback
instead of hidden counters tied directly to the operating-system clock.

### Implemented

- Added `neothesia_core::practice::PracticeMatcher`.
- Made the caller provide monotonic session time so exact timing cases are
  reproducible without sleeps.
- Added configurable early-match and on-time windows.
- Classified correct notes as early, on-time or late.
- Counted expired unmatched and duplicate user presses as wrong.
- Tracked required chord notes without penalizing the order in which they are
  played.
- Added a stable snapshot containing matched, timing, wrong and waiting totals.
- Added a compact live status line next to the wait-mode control.
- Pauses freeze the practice clock; guided waits continue it.
- Normalized live `NoteOn velocity=0` as a release for matching.

### Verification

- Five new deterministic unit tests.
- `cargo test -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `cargo fmt --all`
- `git diff --check`

All checks passed. Existing compiler warnings are unchanged and unrelated.

Implementation commit: `bab60ea` (`feat: add deterministic practice feedback`).

### Known limitations

- A pitch is still the transient match key, so overlapping occurrences of the
  same pitch are intentionally deferred to `PRA-012`.
- Missed score notes are not finalized yet; wait mode keeps them required.
- Live totals are session-level and not yet grouped by measure or hand.

## 2026-07-25 — Cycle 001: Guided-practice baseline (IN PROGRESS)

### Intended outcome

A pianist can open a prepared two-hand MIDI and immediately use wait-for-notes
practice, while retaining an obvious way to return to automatic playback. The
falling-note view identifies measures and can optionally show quarter-note
subdivisions.

### Implemented

- Added a persisted practice-friendly wait setting with backward-compatible
  defaults.
- Defaulted non-drum note tracks to human practice.
- Added a visible player toggle that changes wait behaviour without reopening
  the song.
- Preserved non-note controllers on human tracks while note targets wait for
  the user.
- Parsed quarter-note timestamps alongside measure timestamps.
- Added configurable quarter-note lines and one-based measure labels.
- Updated free-play, recorder preview and CLI renderer call sites.

### Verification so far

- `cargo fmt --all`
- `cargo test -p midi-file -p neothesia-core -p neothesia --all-targets`
- `cargo build --release -p neothesia`
- `git diff --check`

All completed checks passed. The CLI package remains outside the release build
because the local FFmpeg development environment is not configured.

Implementation commit: `a1755bc` (`feat: add guided piano practice baseline`).

### Remaining before close

- Complete a real-device/manual smoke test for open, wait, toggle, loop and exit.

### Known limitations

- Existing statistics are internal and wall-clock based; they are not yet a
  reliable learner-facing score.
- Measure generation currently follows the MIDI parser's existing time-signature
  assumptions and needs broader fixture coverage.
