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
    fingerings: [
        FingerHint(track_id: 1, note_index: 0, finger: 5),
        FingerHint(track_id: 2, note_index: 0, finger: 1),
    ],
)
```

Every descriptive field is optional. Empty strings are removed, surrounding
spaces are trimmed, and tags are sorted and deduplicated without regard to
case.

Finger hints identify one exact note by MIDI track and its zero-based index
inside that track. Fingers must be 1 through 5. Neothesia writes these technical
identifiers itself; the player workflow never asks the learner to type them.

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

## Edit inside Practice Library

1. Open **Practice Library**.
2. Select **Info** beside a local MIDI.
3. Click a field and type its value.
4. Use `Tab`, `Shift+Tab` or the arrow keys to change fields.
5. Press `Backspace` to edit or `Delete` to clear the active field.
6. Select **Save information** or press `Ctrl+S`.

Tags are entered as a comma-separated list. Saving returns to the library and
refreshes the title, credit and search index. **Cancel**, `Escape` and the mouse
back button discard the unsaved edit.

## Add finger hints in the player

1. Open an imported local MIDI.
2. Select **Edit fingers** in the top bar, or press `Ctrl+I`.
3. Neothesia pauses and selects the nearest visible piano note.
4. Use `Left` / `Right` to move through notes. Notes sharing an onset are
   ordered from low to high, so chord fingers can be entered one at a time.
5. Press `1` through `5` to assign a finger. The selection advances
   automatically.
   Alternatively, press `G` to preview an
   [explainable suggestion](fingering-suggestions.md), then `Enter` to accept.
6. Press `Delete` or `Backspace` to clear the selected hint.
7. Select **Edit: ON**, press `Ctrl+I`, or press `Escape` to leave edit mode.

The status message names the hand/part, one-based measure and piano pitch, and
shows an existing finger when present. Every assignment is saved immediately;
there is no unsaved batch to lose. Manual numbers use the same independently
switchable **Fingers: ON/OFF** layer as reviewed Technique Studio guidance.

Finger editing is offered only for an imported MIDI with a real source file.
Generated Technique Studio exercises already use reviewed tables and must be
exported before they can own a portable adjacent sidecar.

## Persistence guarantees

Neothesia writes a complete temporary file in the same directory, flushes it,
then atomically replaces the previous sidecar. A failed save removes the
temporary file and leaves the previous metadata and finger hints intact.
Editing descriptive metadata preserves finger hints, and editing finger hints
preserves descriptive metadata.
