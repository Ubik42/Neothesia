# Portable song metadata sidecars

Neothesia keeps descriptive repertoire data beside the original MIDI instead
of changing the MIDI file. A sidecar uses the complete MIDI filename plus
`.neothesia.ron`:

```text
Clair de Lune.mid
Clair de Lune.mid.neothesia.ron
```

Keeping both files together makes the title, credits, difficulty, tags and
study notes portable across folders and machines.

## Version 1 fields

```ron
MetadataSidecar(
    version: 1,
    content_id: "<BLAKE3 identity of the MIDI bytes>",
    metadata: SongMetadata(
        title: Some("Clair de Lune"),
        artist: Some("Walter Gieseking"),
        composer: Some("Claude Debussy"),
        collection: Some("Suite bergamasque"),
        difficulty: Some("Advanced"),
        tags: ["impressionism", "voicing"],
        notes: Some("Keep the melody above the accompaniment."),
    ),
)
```

Every descriptive field is optional. Empty strings are removed, surrounding
spaces are trimmed, and tags are sorted and deduplicated without regard to
case.

## Identity safety

The `content_id` is derived from the MIDI bytes, not its path or filename.
Neothesia loads a sidecar only when that identity matches the adjacent MIDI.
Replacing the MIDI while leaving an old sidecar therefore cannot silently give
the new piece the old title or study notes.

Moving or renaming a piece is safe when the MIDI and sidecar move together.
If identical MIDI files exist in more than one watched folder, Neothesia:

1. processes paths in deterministic sorted order;
2. keeps the first non-empty scalar value;
3. fills missing scalar values from later matching sidecars;
4. merges and deduplicates all tags.

## Library behavior

Practice Library uses a sidecar title in place of the filename and shows artist
credit, falling back to composer when artist is absent. Search covers:

- title, artist and composer;
- collection and difficulty;
- tags and study notes;
- the source path.

The scan summary reports loaded and invalid sidecars separately. A damaged,
unsupported or content-mismatched sidecar is ignored without making the MIDI
unavailable.

## Persistence guarantees

Neothesia writes a complete temporary file in the same directory, flushes it,
then atomically replaces the previous sidecar. A failed save removes the
temporary file and leaves the previous metadata intact.

The model and storage API are complete. The native in-app metadata editor is a
separate backlog item; until it lands, sidecar creation is considered a
developer-facing capability rather than a finished learner workflow.
