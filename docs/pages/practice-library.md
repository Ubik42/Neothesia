# Public practice library

Neothesia does not bundle third-party repertoire. The helper script
`scripts/sync-practice-library.ps1` builds a local, source-aware practice and
test library under `D:\Music\MusicLib\MIDI` by default.

## Included sources

| Collection | Purpose | Local selection | License handling |
| --- | --- | --- | --- |
| MAESTRO v3.0.0 | Classical performance MIDI | Complete 1,276-file MIDI release | CC BY-NC-SA 4.0; downloaded locally and checksum verified |
| Mutopia Project | Classical teaching and public-domain repertoire | 521 piano-focused files from composer and technique collections in the current sync | Per-piece license retained in `catalog.csv` |
| Pop-K v1.0 | Modern pop-style melody examples | First 256 numbered excerpts by default | CC BY-NC 4.0; downloaded locally and checksum verified |

These sources are useful for private practice and development coverage, but
their terms are not identical. In particular, the MAESTRO and Pop-K datasets
have non-commercial restrictions. Do not redistribute the downloaded corpus as
part of a Neothesia binary or release.

## Run the sync

From PowerShell at the repository root:

```powershell
.\scripts\sync-practice-library.ps1
```

Use `-PopKCount` to change the bounded Pop-K subset, `-LibraryRoot` to select a
different local destination and `-Force` to re-download or re-expand existing
content. The script verifies official archive checksums before extraction.

The resulting `catalog.csv` records category, title, composer, license, source,
source URL and a library-relative path. Downloaded archives and MIDI files stay
outside Git. Mutopia links can disappear over time; an unavailable individual
piece is reported and skipped without invalidating the rest of the catalogue.

## Product integration status

The native Practice Library scans `catalog.csv` beside watched MIDI folders in
the same background indexing task. Catalogue title, composer, teaching
category, source and license are searchable and appear as read-only provenance
in the song list and information header. User-edited sidecar metadata remains
authoritative and is never overwritten by catalogue data.

Catalogue paths are treated as untrusted input: absolute paths and parent
traversal are rejected, both Windows and portable separators are accepted, and
one malformed row cannot prevent valid songs from loading. Run the real-corpus
inspection without opening the GPU interface with:

```powershell
cargo run -q -p neothesia-core --example library-inspect -- `
  "D:\Music\MusicLib\MIDI" "Bach Mutopia"
```
