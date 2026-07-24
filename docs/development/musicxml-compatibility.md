# MusicXML compatibility matrix

This matrix turns importer compatibility into reproducible evidence. Run:

```powershell
.\scripts\musicxml-compatibility.ps1
```

The runner downloads eight fixtures from OpenSheetMusicDisplay revision
`45ece8fa9f4ff1b51a6dfe29515e18530a64b1aa`, verifies fixed SHA-256 hashes and
stores them only in the system temporary directory. OSMD uses the BSD-3-Clause
license. No network request occurs when the verified cache already exists.

## Current corpus

| Fixture | Export shape | Measures | Events | Tuplets | Pedal | Expected diagnostic |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Bach BWV 846 Prelude | MuseScore 1.2 | 35 | 750 | 0 | 0 | none |
| Clementi Sonatina Allegro | older/unspecified | 76 part-measures | 389 | 0 | 0 | none |
| Clementi Sonatina Andante | MuseScore 1.2 | 52 part-measures | 341 | 267 | 0 | ornaments |
| Pedal function test | MuseScore 3.6.2 | 15 | 67 | 0 | 7 | none |
| Voice alignment | MuseScore 3.6.2 | 4 | 47 | 0 | 0 | none |
| Grace notes | MuseScore 2.3.1 | 4 | 40 | 0 | 0 | none |
| Brooke West sample | compressed MXL | 34 part-measures | 323 | 0 | 0 | invalid/missing mimetype |
| Broad function test | MuseScore 2.3.2 | 41 | 226 | 42 | 0 | ornaments |

Totals: 261 part-measures and 2,183 note/rest events.

## Findings that changed the importer

- Older exports use both `<rest/>` and `<rest></rest>`; both now import.
- Deferred empty elements such as `<pedal/>` must trigger the same diagnostic
  as explicit start/end tags.
- Tuplet event time is exact through `duration/divisions`; ratios and
  bracket/number spans are now preserved independently for display and coaching.
- Warning locations include part plus measure because two piano parts can reuse
  the same measure number.

## Admission rule

A fixture enters the permanent matrix only when:

- its source and redistribution terms are recorded;
- its upstream revision and SHA-256 are pinned;
- expected counts and diagnostics are explicit;
- a failure exits nonzero;
- it adds a distinct exporter, version or notation risk.

The next required additions are current MuseScore plus licensed Dorico and
Finale piano exports. Network-backed corpus checks supplement unit tests; they
do not replace the offline synthetic regression suite.
