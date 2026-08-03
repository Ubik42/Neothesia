# Verovio 6.1.0 renderer benchmark

Cycle 089 measured Verovio outside the application before committing Neothesia
to a web view, JavaScript runtime or notation-engine dependency. The upstream
source is pinned at `version-6.1.0` / `682d60684450e780f0a1dcb7e394bda12eea4501`
under the local reference tree. The runnable test shell and generated artifacts
live under `D:\cs\_test\neothesia-verovio`.

Run the benchmark from the repository root:

```powershell
.\scripts\verovio-benchmark.ps1
```

The wrapper verifies the same eight pinned OSMD/MuseScore fixtures used by the
native importer audit, invokes the pinned NPM/WASM package, writes a JSON report
and retains every SVG page for visual inspection. It does not add Verovio to the
shipping application.

## Results

Measured on the Windows development machine on 2026-08-04. These are warm
single-process development measurements, not release-device guarantees.

| Measure | Result |
| --- | ---: |
| Inputs loaded | 8 / 8 |
| Module initialization | 76.06 ms |
| Uncompressed ESM/WASM module | 6,983,039 bytes (6.66 MiB) |
| Median / maximum score load | 35.40 / 144.27 ms |
| Median / maximum full-score SVG render | 16.21 / 46.32 ms |
| Worst mean note time+attribute lookup | 0.180 ms |
| Total SVG for eleven pages | 2,511,338 bytes |

The largest fixture was Bach's BWV 846 prelude: 618 note identities across two
pages and 739,594 bytes of SVG. Its full-score render took 46.32 ms. The
compressed MXL fixture loaded and rendered normally.

## Visual and semantic inspection

- the Bach grand staff was readable, correctly grouped and consistently spaced;
- the Clementi fixture retained 78 tuplets, ten slurs, dynamics, ornaments and
  two-staff alignment;
- the compressed MXL path rendered staff notation and tablature, proving the
  archive-to-render path, although it is not a representative piano fixture;
- all sampled note identities resolved through `getTimeForElement` and
  `getElementAttr`, so a native player can address notation without deriving
  timing from SVG geometry;
- the pedal fixture rendered seven pedal elements but emitted warnings for two
  time-spanning pedal pairs whose starts did not precede their ends. Pedal
  fidelity must remain diagnostic, not silently claimed as complete.

## Decision

The spike passes the threshold for a synchronized grand-staff proof of concept.
It does **not** justify coupling practice logic to Verovio or loading a complete
score as one live DOM tree.

The proof of concept should therefore:

1. keep the existing Rust score model and alignment identities authoritative;
2. render only the current page plus a small neighbour window;
3. use Verovio XML identities/time queries for highlights;
4. isolate the engine behind a renderer adapter and feature flag;
5. surface import/render warnings, especially pedal and navigation issues;
6. measure cold startup, memory and highlight paint latency inside the actual
   native display boundary before adopting the dependency for production.

The 6.66 MiB module and up to 740 KiB of SVG per tested score are acceptable for
an opt-in prototype but require page virtualization and explicit packaging
measurement.
