use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

const METADATA_VERSION: u16 = 1;
const SIDECAR_EXTENSION: &str = "neothesia.ron";

#[derive(Debug, Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SongMetadata {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub composer: Option<String>,
    #[serde(default)]
    pub collection: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LibraryProvenance {
    pub category: String,
    pub title: String,
    pub composer: String,
    pub license: String,
    pub source: String,
    pub source_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CatalogRecord {
    category: String,
    title: String,
    composer: String,
    license: String,
    source: String,
    source_url: String,
    local_path: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct FingerHint {
    pub track_id: usize,
    pub note_index: usize,
    pub finger: u8,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScoreAssociation {
    pub path: PathBuf,
    pub content_id: String,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScoreAnalysisSnapshot {
    pub score_content_id: String,
    pub readiness: crate::score_alignment::AlignmentReadiness,
    pub matched_notes: usize,
    pub unmatched_score_notes: usize,
    pub unmatched_midi_notes: usize,
    pub inexact_projection_matches: usize,
    pub coverage_percent: u8,
    pub mean_confidence_percent: u8,
    pub navigation_diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct SongSidecar {
    pub metadata: SongMetadata,
    pub fingerings: Vec<FingerHint>,
    pub score: Option<ScoreAssociation>,
    pub score_analysis: Option<ScoreAnalysisSnapshot>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
struct MetadataSidecar {
    version: u16,
    content_id: String,
    metadata: SongMetadata,
    #[serde(default)]
    fingerings: Vec<FingerHint>,
    #[serde(default)]
    score: Option<ScoreAssociation>,
    #[serde(default)]
    score_analysis: Option<ScoreAnalysisSnapshot>,
}

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("could not read metadata sidecar: {0}")]
    Read(#[source] io::Error),
    #[error("could not parse metadata sidecar: {0}")]
    Parse(#[source] ron::error::SpannedError),
    #[error("unsupported metadata version {0}")]
    UnsupportedVersion(u16),
    #[error("metadata belongs to MIDI content {actual}, expected {expected}")]
    ContentMismatch { expected: String, actual: String },
    #[error(
        "finger hint for track {track_id}, note {note_index} uses invalid finger {finger}; \
         expected 1 through 5"
    )]
    InvalidFinger {
        track_id: usize,
        note_index: usize,
        finger: u8,
    },
    #[error("paired score is not usable MusicXML: {0}")]
    InvalidScore(#[source] crate::musicxml::ImportError),
    #[error("could not serialize metadata sidecar: {0}")]
    Serialize(#[source] ron::Error),
    #[error("could not save metadata sidecar: {0}")]
    Write(#[source] io::Error),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LibrarySong {
    pub content_id: String,
    pub display_name: String,
    pub metadata: SongMetadata,
    pub provenance: Option<LibraryProvenance>,
    pub metadata_path: Option<PathBuf>,
    pub source_paths: Vec<PathBuf>,
    searchable_text: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LibraryIndex {
    pub songs: Vec<LibrarySong>,
    pub midi_files_seen: usize,
    pub unreadable_files: usize,
    pub metadata_files_seen: usize,
    pub invalid_metadata_files: usize,
    pub catalog_files_seen: usize,
    pub catalog_entries_loaded: usize,
    pub invalid_catalog_entries: usize,
}

impl LibraryIndex {
    pub fn scan(roots: &[PathBuf]) -> Self {
        let (mut paths, mut catalog_paths) = collect_library_paths(roots);
        paths.sort();
        paths.dedup();
        catalog_paths.sort();
        catalog_paths.dedup();

        let (catalog_entries, invalid_catalog_entries) = load_catalogs(&catalog_paths);

        let mut songs = BTreeMap::<String, LibrarySong>::new();
        let mut unreadable_files = 0;
        let mut metadata_files_seen = 0;
        let mut invalid_metadata_files = 0;
        for path in &paths {
            match midi_file::MidiFile::new(path) {
                Ok(file) => {
                    let sidecar_path = metadata_sidecar_path(path);
                    let sidecar_exists = sidecar_path.is_file();
                    let metadata = if sidecar_exists {
                        metadata_files_seen += 1;
                        match load_song_metadata(path, &file.content_id) {
                            Ok(metadata) => Some((metadata, sidecar_path)),
                            Err(error) => {
                                invalid_metadata_files += 1;
                                log::warn!(
                                    "Ignoring metadata sidecar '{}': {error}",
                                    sidecar_path.display()
                                );
                                None
                            }
                        }
                    } else {
                        None
                    };
                    let song =
                        songs
                            .entry(file.content_id.clone())
                            .or_insert_with(|| LibrarySong {
                                content_id: file.content_id,
                                display_name: file.name,
                                metadata: SongMetadata::default(),
                                provenance: None,
                                metadata_path: None,
                                source_paths: Vec::new(),
                                searchable_text: String::new(),
                            });
                    song.source_paths.push(path.clone());
                    if song.provenance.is_none() {
                        song.provenance = catalog_entries.get(&path_identity(path)).cloned();
                    }
                    if let Some((metadata, metadata_path)) = metadata {
                        song.metadata.merge(metadata);
                        if song.metadata_path.is_none() {
                            song.metadata_path = Some(metadata_path);
                        }
                    }
                }
                Err(error) => {
                    unreadable_files += 1;
                    log::warn!("Skipping unreadable MIDI '{}': {error}", path.display());
                }
            }
        }

        let mut songs: Vec<_> = songs.into_values().collect();
        for song in &mut songs {
            if let Some(title) = song.metadata.title.as_deref() {
                song.display_name = title.to_owned();
            } else if let Some(provenance) = song.provenance.as_ref() {
                song.display_name = provenance.title.clone();
            }
            song.searchable_text = searchable_text(
                &song.display_name,
                &song.metadata,
                song.provenance.as_ref(),
                &song.source_paths,
            );
        }

        Self {
            songs,
            midi_files_seen: paths.len(),
            unreadable_files,
            metadata_files_seen,
            invalid_metadata_files,
            catalog_files_seen: catalog_paths.len(),
            catalog_entries_loaded: catalog_entries.len(),
            invalid_catalog_entries,
        }
    }

    pub fn search(&self, query: &str) -> Vec<&LibrarySong> {
        let terms: Vec<_> = query
            .split_whitespace()
            .map(str::to_lowercase)
            .filter(|term| !term.is_empty())
            .collect();
        self.songs
            .iter()
            .filter(|song| {
                if terms.is_empty() {
                    return true;
                }
                terms.iter().all(|term| song.searchable_text.contains(term))
            })
            .collect()
    }
}

impl SongMetadata {
    fn normalized(mut self) -> Self {
        self.title = normalized_text(self.title);
        self.artist = normalized_text(self.artist);
        self.composer = normalized_text(self.composer);
        self.collection = normalized_text(self.collection);
        self.difficulty = normalized_text(self.difficulty);
        self.notes = normalized_text(self.notes);
        self.tags = self
            .tags
            .into_iter()
            .filter_map(|tag| normalized_text(Some(tag)))
            .collect();
        self.tags.sort_by_key(|tag| tag.to_lowercase());
        self.tags
            .dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        self
    }

    fn merge(&mut self, other: Self) {
        let other = other.normalized();
        merge_first(&mut self.title, other.title);
        merge_first(&mut self.artist, other.artist);
        merge_first(&mut self.composer, other.composer);
        merge_first(&mut self.collection, other.collection);
        merge_first(&mut self.difficulty, other.difficulty);
        merge_first(&mut self.notes, other.notes);
        self.tags.extend(other.tags);
        self.tags.sort_by_key(|tag| tag.to_lowercase());
        self.tags
            .dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    }
}

fn merge_first(target: &mut Option<String>, source: Option<String>) {
    if target.is_none() {
        *target = source;
    }
}

fn normalized_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

pub fn metadata_sidecar_path(midi_path: &Path) -> PathBuf {
    let mut name = midi_path
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("song.mid"));
    name.push(".");
    name.push(SIDECAR_EXTENSION);
    midi_path.with_file_name(name)
}

pub fn load_song_metadata(
    midi_path: &Path,
    expected_content_id: &str,
) -> Result<SongMetadata, MetadataError> {
    Ok(load_song_sidecar(midi_path, expected_content_id)?.metadata)
}

pub fn load_song_sidecar(
    midi_path: &Path,
    expected_content_id: &str,
) -> Result<SongSidecar, MetadataError> {
    let path = metadata_sidecar_path(midi_path);
    let contents = fs::read_to_string(path).map_err(MetadataError::Read)?;
    let sidecar: MetadataSidecar = ron::from_str(&contents).map_err(MetadataError::Parse)?;
    if sidecar.version != METADATA_VERSION {
        return Err(MetadataError::UnsupportedVersion(sidecar.version));
    }
    if sidecar.content_id != expected_content_id {
        return Err(MetadataError::ContentMismatch {
            expected: expected_content_id.to_owned(),
            actual: sidecar.content_id,
        });
    }
    Ok(SongSidecar {
        metadata: sidecar.metadata.normalized(),
        fingerings: normalize_fingerings(sidecar.fingerings)?,
        score: sidecar.score,
        score_analysis: sidecar.score_analysis,
    })
}

pub fn save_song_metadata(
    midi_path: &Path,
    content_id: &str,
    metadata: SongMetadata,
) -> Result<PathBuf, MetadataError> {
    let mut sidecar = existing_sidecar_or_default(midi_path, content_id)?;
    sidecar.metadata = metadata;
    save_song_sidecar(midi_path, content_id, sidecar)
}

pub fn save_song_fingerings(
    midi_path: &Path,
    content_id: &str,
    fingerings: Vec<FingerHint>,
) -> Result<PathBuf, MetadataError> {
    let mut sidecar = existing_sidecar_or_default(midi_path, content_id)?;
    sidecar.fingerings = fingerings;
    save_song_sidecar(midi_path, content_id, sidecar)
}

pub fn save_score_association(
    midi_path: &Path,
    content_id: &str,
    score_path: &Path,
) -> Result<PathBuf, MetadataError> {
    let source = fs::read(score_path)
        .map_err(|error| MetadataError::InvalidScore(crate::musicxml::ImportError::Io(error)))?;
    crate::musicxml::import_musicxml_document(&source).map_err(MetadataError::InvalidScore)?;
    let mut sidecar = existing_sidecar_or_default(midi_path, content_id)?;
    sidecar.score = Some(ScoreAssociation {
        path: portable_score_path(midi_path, score_path),
        content_id: blake3::hash(&source).to_hex().to_string(),
    });
    sidecar.score_analysis = None;
    save_song_sidecar(midi_path, content_id, sidecar)
}

pub fn clear_score_association(
    midi_path: &Path,
    content_id: &str,
) -> Result<PathBuf, MetadataError> {
    let mut sidecar = existing_sidecar_or_default(midi_path, content_id)?;
    sidecar.score = None;
    sidecar.score_analysis = None;
    save_song_sidecar(midi_path, content_id, sidecar)
}

pub fn resolve_score_path(midi_path: &Path, association: &ScoreAssociation) -> PathBuf {
    if association.path.is_absolute() {
        association.path.clone()
    } else {
        midi_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(&association.path)
    }
}

pub fn save_score_analysis(
    midi_path: &Path,
    content_id: &str,
    analysis: ScoreAnalysisSnapshot,
) -> Result<PathBuf, MetadataError> {
    let mut sidecar = existing_sidecar_or_default(midi_path, content_id)?;
    sidecar.score_analysis = Some(analysis);
    save_song_sidecar(midi_path, content_id, sidecar)
}

pub fn verify_score_association(
    midi_path: &Path,
    association: &ScoreAssociation,
) -> Result<bool, MetadataError> {
    let source = fs::read(resolve_score_path(midi_path, association))
        .map_err(|error| MetadataError::InvalidScore(crate::musicxml::ImportError::Io(error)))?;
    crate::musicxml::import_musicxml_document(&source).map_err(MetadataError::InvalidScore)?;
    Ok(blake3::hash(&source).to_hex().as_str() == association.content_id)
}

fn portable_score_path(midi_path: &Path, score_path: &Path) -> PathBuf {
    let Some(parent) = midi_path.parent() else {
        return score_path.to_path_buf();
    };
    score_path
        .strip_prefix(parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| score_path.to_path_buf())
}

fn existing_sidecar_or_default(
    midi_path: &Path,
    content_id: &str,
) -> Result<SongSidecar, MetadataError> {
    if metadata_sidecar_path(midi_path).is_file() {
        load_song_sidecar(midi_path, content_id)
    } else {
        Ok(SongSidecar::default())
    }
}

fn save_song_sidecar(
    midi_path: &Path,
    content_id: &str,
    sidecar: SongSidecar,
) -> Result<PathBuf, MetadataError> {
    let path = metadata_sidecar_path(midi_path);
    let sidecar = MetadataSidecar {
        version: METADATA_VERSION,
        content_id: content_id.to_owned(),
        metadata: sidecar.metadata.normalized(),
        fingerings: normalize_fingerings(sidecar.fingerings)?,
        score: sidecar.score,
        score_analysis: sidecar.score_analysis,
    };
    let contents = ron::ser::to_string_pretty(
        &sidecar,
        ron::ser::PrettyConfig::default().struct_names(true),
    )
    .map_err(MetadataError::Serialize)?;
    atomic_write(&path, contents.as_bytes()).map_err(MetadataError::Write)?;
    Ok(path)
}

fn normalize_fingerings(fingerings: Vec<FingerHint>) -> Result<Vec<FingerHint>, MetadataError> {
    let mut by_note = BTreeMap::new();
    for hint in fingerings {
        if !(1..=5).contains(&hint.finger) {
            return Err(MetadataError::InvalidFinger {
                track_id: hint.track_id,
                note_index: hint.note_index,
                finger: hint.finger,
            });
        }
        by_note.insert((hint.track_id, hint.note_index), hint);
    }
    Ok(by_note.into_values().collect())
}

fn searchable_text(
    display_name: &str,
    metadata: &SongMetadata,
    provenance: Option<&LibraryProvenance>,
    paths: &[PathBuf],
) -> String {
    let mut searchable = display_name.to_lowercase();
    for value in [
        metadata.artist.as_deref(),
        metadata.composer.as_deref(),
        metadata.collection.as_deref(),
        metadata.difficulty.as_deref(),
        metadata.notes.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        searchable.push(' ');
        searchable.push_str(&value.to_lowercase());
    }
    for tag in &metadata.tags {
        searchable.push(' ');
        searchable.push_str(&tag.to_lowercase());
    }
    if let Some(provenance) = provenance {
        for value in [
            &provenance.category,
            &provenance.title,
            &provenance.composer,
            &provenance.license,
            &provenance.source,
            &provenance.source_url,
        ] {
            searchable.push(' ');
            searchable.push_str(&value.to_lowercase());
        }
    }
    for path in paths {
        searchable.push(' ');
        searchable.push_str(&path.to_string_lossy().to_lowercase());
    }
    searchable
}

fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("song.mid.neothesia.ron");
    let temporary = parent.join(format!(
        ".{file_name}.tmp-{}-{}",
        std::process::id(),
        unique_nonce()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        replace_file(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn unique_nonce() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(not(target_os = "windows"))]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(target_os = "windows")]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    use std::{iter, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source: Vec<_> = source
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let destination: Vec<_> = destination
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let success = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if success == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn collect_library_paths(roots: &[PathBuf]) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut midi_files = Vec::new();
    let mut catalogs = Vec::new();
    let mut pending: Vec<_> = roots.iter().filter(|root| root.is_dir()).cloned().collect();
    let mut visited = BTreeSet::new();

    while let Some(directory) = pending.pop() {
        let identity = directory
            .canonicalize()
            .unwrap_or_else(|_| directory.clone());
        if !visited.insert(identity) {
            continue;
        }
        let Ok(entries) = fs::read_dir(&directory) else {
            log::warn!("Could not scan library directory '{}'", directory.display());
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file() && is_midi_path(&path) {
                midi_files.push(path);
            } else if file_type.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case("catalog.csv"))
            {
                catalogs.push(path);
            }
        }
    }
    (midi_files, catalogs)
}

fn load_catalogs(catalog_paths: &[PathBuf]) -> (BTreeMap<PathBuf, LibraryProvenance>, usize) {
    let mut entries = BTreeMap::new();
    let mut invalid = 0;
    for catalog_path in catalog_paths {
        let Some(root) = catalog_path.parent() else {
            invalid += 1;
            continue;
        };
        let mut reader = match csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_path(catalog_path)
        {
            Ok(reader) => reader,
            Err(error) => {
                invalid += 1;
                log::warn!(
                    "Ignoring practice catalog '{}': {error}",
                    catalog_path.display()
                );
                continue;
            }
        };
        for record in reader.deserialize::<CatalogRecord>() {
            let record = match record {
                Ok(record) => record,
                Err(error) => {
                    invalid += 1;
                    log::warn!(
                        "Ignoring invalid row in practice catalog '{}': {error}",
                        catalog_path.display()
                    );
                    continue;
                }
            };
            let Some(relative_path) = safe_catalog_relative_path(&record.local_path) else {
                invalid += 1;
                log::warn!(
                    "Ignoring unsafe MIDI path '{}' in practice catalog '{}'",
                    record.local_path,
                    catalog_path.display()
                );
                continue;
            };
            let provenance = LibraryProvenance {
                category: record.category.trim().to_owned(),
                title: record.title.trim().to_owned(),
                composer: record.composer.trim().to_owned(),
                license: record.license.trim().to_owned(),
                source: record.source.trim().to_owned(),
                source_url: record.source_url.trim().to_owned(),
            };
            if provenance.category.is_empty()
                || provenance.title.is_empty()
                || provenance.license.is_empty()
                || provenance.source.is_empty()
            {
                invalid += 1;
                log::warn!(
                    "Ignoring incomplete provenance for '{}' in practice catalog '{}'",
                    record.local_path,
                    catalog_path.display()
                );
                continue;
            }
            let target = root.join(relative_path);
            if !is_midi_path(&target) {
                invalid += 1;
                continue;
            }
            entries.entry(path_identity(&target)).or_insert(provenance);
        }
    }
    (entries, invalid)
}

fn safe_catalog_relative_path(value: &str) -> Option<PathBuf> {
    let value = value.trim();
    if value.is_empty()
        || value.starts_with(['/', '\\'])
        || value.as_bytes().get(1).is_some_and(|byte| *byte == b':')
    {
        return None;
    }
    let mut safe = PathBuf::new();
    for component in value.split(['/', '\\']) {
        match component.trim() {
            "" | "." => {}
            ".." => return None,
            part => safe.push(part),
        }
    }
    (!safe.as_os_str().is_empty()).then_some(safe)
}

fn path_identity(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn is_midi_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("mid") || extension.eq_ignore_ascii_case("midi")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use midi_file::midly::{
        Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
        num::{u4, u7, u15, u28},
    };

    fn write_midi(path: &Path, note: u8) {
        let smf = Smf {
            header: Header::new(Format::SingleTrack, Timing::Metrical(u15::new(480))),
            tracks: vec![vec![
                TrackEvent {
                    delta: u28::new(0),
                    kind: TrackEventKind::Midi {
                        channel: u4::new(0),
                        message: MidiMessage::NoteOn {
                            key: u7::new(note),
                            vel: u7::new(90),
                        },
                    },
                },
                TrackEvent {
                    delta: u28::new(480),
                    kind: TrackEventKind::Midi {
                        channel: u4::new(0),
                        message: MidiMessage::NoteOff {
                            key: u7::new(note),
                            vel: u7::new(0),
                        },
                    },
                },
                TrackEvent {
                    delta: u28::new(0),
                    kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
                },
            ]],
        };
        let mut bytes = Vec::new();
        smf.write_std(&mut bytes).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn temp_directory(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "neothesia-library-{}-{}-{name}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn recursive_scan_deduplicates_identical_midi_content() {
        let root = temp_directory("dedupe");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).unwrap();
        write_midi(&root.join("Original.mid"), 60);
        fs::copy(root.join("Original.mid"), nested.join("Copy.MIDI")).unwrap();
        write_midi(&root.join("Different.mid"), 64);
        fs::write(root.join("Broken.mid"), "not a MIDI").unwrap();
        fs::write(root.join("ignore.txt"), "not midi").unwrap();

        let index = LibraryIndex::scan(std::slice::from_ref(&root));

        assert_eq!(index.midi_files_seen, 4);
        assert_eq!(index.unreadable_files, 1);
        assert_eq!(index.metadata_files_seen, 0);
        assert_eq!(index.invalid_metadata_files, 0);
        assert_eq!(index.catalog_files_seen, 0);
        assert_eq!(index.catalog_entries_loaded, 0);
        assert_eq!(index.invalid_catalog_entries, 0);
        assert_eq!(index.songs.len(), 2);
        assert_eq!(
            index
                .songs
                .iter()
                .find(|song| song.source_paths.len() == 2)
                .unwrap()
                .source_paths
                .len(),
            2
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn search_matches_all_terms_across_title_and_path() {
        let index = LibraryIndex {
            songs: vec![LibrarySong {
                content_id: "id".into(),
                display_name: "Clair de Lune.mid".into(),
                metadata: SongMetadata::default(),
                provenance: None,
                metadata_path: None,
                source_paths: vec![PathBuf::from("D:/Piano/Debussy/Clair de Lune.mid")],
                searchable_text: searchable_text(
                    "Clair de Lune.mid",
                    &SongMetadata::default(),
                    None,
                    &[PathBuf::from("D:/Piano/Debussy/Clair de Lune.mid")],
                ),
            }],
            midi_files_seen: 1,
            unreadable_files: 0,
            metadata_files_seen: 0,
            invalid_metadata_files: 0,
            catalog_files_seen: 0,
            catalog_entries_loaded: 0,
            invalid_catalog_entries: 0,
        };

        assert_eq!(index.search("debussy lune").len(), 1);
        assert!(index.search("debussy moonlight").is_empty());
    }

    #[test]
    fn catalog_provenance_is_safe_searchable_and_secondary_to_user_metadata() {
        let root = temp_directory("catalog");
        let teaching = root.join("Teaching");
        fs::create_dir_all(&teaching).unwrap();
        let midi_path = teaching.join("Etude, No 1.mid");
        write_midi(&midi_path, 60);
        fs::write(
            root.join("catalog.csv"),
            concat!(
                "\u{feff}Category,Title,Composer,License,Source,SourceUrl,LocalPath\n",
                "Teaching/初级,\"Étude, No. 1\",Example Composer,CC0,Mutopia,",
                "https://example.test/etude,\"Teaching\\Etude, No 1.mid\"\n",
                "Unsafe,Escape,Unknown,CC0,Test,https://example.test,../outside.mid\n",
                "Incomplete,Missing license,Unknown,,Test,https://example.test,missing.mid\n"
            ),
        )
        .unwrap();
        let content_id = midi_file::MidiFile::new(&midi_path).unwrap().content_id;
        save_song_metadata(
            &midi_path,
            &content_id,
            SongMetadata {
                title: Some("My study copy".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let index = LibraryIndex::scan(std::slice::from_ref(&root));

        assert_eq!(index.catalog_files_seen, 1);
        assert_eq!(index.catalog_entries_loaded, 1);
        assert_eq!(index.invalid_catalog_entries, 2);
        assert_eq!(index.songs.len(), 1);
        assert_eq!(index.songs[0].display_name, "My study copy");
        let provenance = index.songs[0].provenance.as_ref().unwrap();
        assert_eq!(provenance.title, "Étude, No. 1");
        assert_eq!(provenance.category, "Teaching/初级");
        assert_eq!(provenance.license, "CC0");
        assert_eq!(index.search("初级 mutopia cc0").len(), 1);
        assert_eq!(index.search("example etude").len(), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn catalog_paths_accept_both_separators_and_reject_absolute_or_parent_paths() {
        assert_eq!(
            safe_catalog_relative_path("Classical/Bach/Prelude.mid"),
            Some(PathBuf::from("Classical").join("Bach").join("Prelude.mid"))
        );
        assert_eq!(
            safe_catalog_relative_path("Classical\\Bach\\Prelude.mid"),
            Some(PathBuf::from("Classical").join("Bach").join("Prelude.mid"))
        );
        assert_eq!(safe_catalog_relative_path("../secret.mid"), None);
        assert_eq!(safe_catalog_relative_path("C:\\secret.mid"), None);
        assert_eq!(safe_catalog_relative_path("/secret.mid"), None);
        assert_eq!(safe_catalog_relative_path("\\\\server\\share.mid"), None);
    }

    #[test]
    fn sidecar_round_trip_is_content_bound_and_searchable() {
        let root = temp_directory("metadata");
        let midi_path = root.join("Unhelpful Filename.mid");
        write_midi(&midi_path, 60);
        let content_id = midi_file::MidiFile::new(&midi_path).unwrap().content_id;
        let metadata = SongMetadata {
            title: Some(" Clair de Lune ".into()),
            artist: Some("Walter Gieseking".into()),
            composer: Some("Claude Debussy".into()),
            collection: Some("Suite bergamasque".into()),
            difficulty: Some("Advanced".into()),
            tags: vec!["Impressionism".into(), " impressionism ".into()],
            notes: Some("Voicing and soft pedal".into()),
        };

        let sidecar = save_song_metadata(&midi_path, &content_id, metadata).unwrap();
        assert_eq!(
            sidecar.file_name().unwrap(),
            "Unhelpful Filename.mid.neothesia.ron"
        );
        let loaded = load_song_metadata(&midi_path, &content_id).unwrap();
        assert_eq!(loaded.title.as_deref(), Some("Clair de Lune"));
        assert_eq!(loaded.tags, ["Impressionism"]);

        let index = LibraryIndex::scan(std::slice::from_ref(&root));
        assert_eq!(index.metadata_files_seen, 1);
        assert_eq!(index.invalid_metadata_files, 0);
        assert_eq!(index.songs[0].display_name, "Clair de Lune");
        assert_eq!(index.songs[0].metadata, loaded);
        assert_eq!(
            index.songs[0].metadata_path.as_deref(),
            Some(sidecar.as_path())
        );
        assert_eq!(index.search("debussy advanced voicing").len(), 1);
        assert_eq!(index.search("gieseking impressionism").len(), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn content_mismatch_is_rejected_instead_of_relabeling_a_song() {
        let root = temp_directory("metadata-mismatch");
        let midi_path = root.join("Actual Song.mid");
        write_midi(&midi_path, 64);
        save_song_metadata(
            &midi_path,
            "different-content",
            SongMetadata {
                title: Some("Wrong Song".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let index = LibraryIndex::scan(std::slice::from_ref(&root));
        assert_eq!(index.metadata_files_seen, 1);
        assert_eq!(index.invalid_metadata_files, 1);
        assert_eq!(index.songs[0].display_name, "Actual Song.mid");
        assert_eq!(index.songs[0].metadata, SongMetadata::default());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_content_merges_tags_with_stable_scalar_precedence() {
        let root = temp_directory("metadata-merge");
        let first = root.join("A.mid");
        let second = root.join("B.mid");
        write_midi(&first, 67);
        fs::copy(&first, &second).unwrap();
        let content_id = midi_file::MidiFile::new(&first).unwrap().content_id;
        save_song_metadata(
            &first,
            &content_id,
            SongMetadata {
                title: Some("Preferred title".into()),
                tags: vec!["Romantic".into()],
                ..Default::default()
            },
        )
        .unwrap();
        save_song_metadata(
            &second,
            &content_id,
            SongMetadata {
                title: Some("Later title".into()),
                composer: Some("Example Composer".into()),
                tags: vec!["Etude".into()],
                ..Default::default()
            },
        )
        .unwrap();

        let index = LibraryIndex::scan(std::slice::from_ref(&root));
        assert_eq!(index.songs.len(), 1);
        assert_eq!(index.songs[0].display_name, "Preferred title");
        assert_eq!(
            index.songs[0].metadata.composer.as_deref(),
            Some("Example Composer")
        );
        assert_eq!(index.songs[0].metadata.tags, ["Etude", "Romantic"]);
        assert_eq!(index.search("preferred etude composer").len(), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn saving_metadata_atomically_replaces_an_existing_sidecar() {
        let root = temp_directory("metadata-replace");
        let midi_path = root.join("Song.mid");
        write_midi(&midi_path, 69);
        let content_id = midi_file::MidiFile::new(&midi_path).unwrap().content_id;
        save_song_metadata(
            &midi_path,
            &content_id,
            SongMetadata {
                title: Some("First".into()),
                ..Default::default()
            },
        )
        .unwrap();
        save_song_metadata(
            &midi_path,
            &content_id,
            SongMetadata {
                title: Some("Second".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            load_song_metadata(&midi_path, &content_id)
                .unwrap()
                .title
                .as_deref(),
            Some("Second")
        );
        assert!(
            fs::read_dir(&root)
                .unwrap()
                .flatten()
                .all(|entry| !entry.file_name().to_string_lossy().contains(".tmp-"))
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn metadata_and_fingerings_preserve_each_other_across_separate_edits() {
        let root = temp_directory("fingering-preserve");
        let midi_path = root.join("Song.mid");
        write_midi(&midi_path, 71);
        let content_id = midi_file::MidiFile::new(&midi_path).unwrap().content_id;
        let hints = vec![
            FingerHint {
                track_id: 1,
                note_index: 4,
                finger: 5,
            },
            FingerHint {
                track_id: 0,
                note_index: 0,
                finger: 1,
            },
        ];
        save_song_fingerings(&midi_path, &content_id, hints).unwrap();
        save_song_metadata(
            &midi_path,
            &content_id,
            SongMetadata {
                title: Some("Named after fingering".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let sidecar = load_song_sidecar(&midi_path, &content_id).unwrap();
        assert_eq!(
            sidecar.metadata.title.as_deref(),
            Some("Named after fingering")
        );
        assert_eq!(
            sidecar.fingerings,
            [
                FingerHint {
                    track_id: 0,
                    note_index: 0,
                    finger: 1,
                },
                FingerHint {
                    track_id: 1,
                    note_index: 4,
                    finger: 5,
                }
            ]
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_fingers_are_rejected_without_replacing_valid_hints() {
        let root = temp_directory("fingering-invalid");
        let midi_path = root.join("Song.mid");
        write_midi(&midi_path, 72);
        let content_id = midi_file::MidiFile::new(&midi_path).unwrap().content_id;
        let valid = FingerHint {
            track_id: 0,
            note_index: 0,
            finger: 3,
        };
        save_song_fingerings(&midi_path, &content_id, vec![valid]).unwrap();
        let error = save_song_fingerings(
            &midi_path,
            &content_id,
            vec![FingerHint { finger: 0, ..valid }],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            MetadataError::InvalidFinger { finger: 0, .. }
        ));
        assert_eq!(
            load_song_sidecar(&midi_path, &content_id)
                .unwrap()
                .fingerings,
            [valid]
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn score_association_is_validated_content_bound_and_portable() {
        let root = temp_directory("score-association");
        let midi_path = root.join("Song.mid");
        let score_path = root.join("Song.musicxml");
        write_midi(&midi_path, 60);
        fs::write(
            &score_path,
            br#"<score-partwise>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1"><measure number="1"><note><rest/><duration>1</duration></note></measure></part>
</score-partwise>"#,
        )
        .unwrap();
        let content_id = midi_file::MidiFile::new(&midi_path).unwrap().content_id;

        save_score_association(&midi_path, &content_id, &score_path).unwrap();
        let sidecar = load_song_sidecar(&midi_path, &content_id).unwrap();
        let association = sidecar.score.unwrap();
        assert_eq!(association.path, PathBuf::from("Song.musicxml"));
        assert_eq!(resolve_score_path(&midi_path, &association), score_path);
        assert!(verify_score_association(&midi_path, &association).unwrap());
        save_score_analysis(
            &midi_path,
            &content_id,
            ScoreAnalysisSnapshot {
                score_content_id: association.content_id.clone(),
                readiness: crate::score_alignment::AlignmentReadiness::Poor,
                matched_notes: 0,
                unmatched_score_notes: 1,
                unmatched_midi_notes: 1,
                inexact_projection_matches: 0,
                coverage_percent: 0,
                mean_confidence_percent: 0,
                navigation_diagnostics: Vec::new(),
            },
        )
        .unwrap();
        assert!(
            load_song_sidecar(&midi_path, &content_id)
                .unwrap()
                .score_analysis
                .is_some()
        );
        save_score_association(&midi_path, &content_id, &score_path).unwrap();
        assert!(
            load_song_sidecar(&midi_path, &content_id)
                .unwrap()
                .score_analysis
                .is_none()
        );

        fs::write(
            resolve_score_path(&midi_path, &association),
            br#"<score-partwise><part-list/></score-partwise>"#,
        )
        .unwrap();
        assert!(!verify_score_association(&midi_path, &association).unwrap());
        clear_score_association(&midi_path, &content_id).unwrap();
        assert!(
            load_song_sidecar(&midi_path, &content_id)
                .unwrap()
                .score
                .is_none()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_score_is_rejected_without_creating_a_sidecar() {
        let root = temp_directory("invalid-score-association");
        let midi_path = root.join("Song.mid");
        let score_path = root.join("Broken.musicxml");
        write_midi(&midi_path, 60);
        fs::write(&score_path, b"not XML").unwrap();
        let content_id = midi_file::MidiFile::new(&midi_path).unwrap().content_id;

        assert!(matches!(
            save_score_association(&midi_path, &content_id, &score_path),
            Err(MetadataError::InvalidScore(_))
        ));
        assert!(!metadata_sidecar_path(&midi_path).exists());
        fs::remove_dir_all(root).unwrap();
    }
}
