use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq)]
pub struct LibrarySong {
    pub content_id: String,
    pub display_name: String,
    pub source_paths: Vec<PathBuf>,
    searchable_text: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LibraryIndex {
    pub songs: Vec<LibrarySong>,
    pub midi_files_seen: usize,
    pub unreadable_files: usize,
}

impl LibraryIndex {
    pub fn scan(roots: &[PathBuf]) -> Self {
        let mut paths = collect_midi_paths(roots);
        paths.sort();
        paths.dedup();

        let mut songs = BTreeMap::<String, LibrarySong>::new();
        let mut unreadable_files = 0;
        for path in &paths {
            match midi_file::MidiFile::new(path) {
                Ok(file) => {
                    let song =
                        songs
                            .entry(file.content_id.clone())
                            .or_insert_with(|| LibrarySong {
                                content_id: file.content_id,
                                display_name: file.name,
                                source_paths: Vec::new(),
                                searchable_text: String::new(),
                            });
                    song.source_paths.push(path.clone());
                }
                Err(error) => {
                    unreadable_files += 1;
                    log::warn!("Skipping unreadable MIDI '{}': {error}", path.display());
                }
            }
        }

        let mut songs: Vec<_> = songs.into_values().collect();
        for song in &mut songs {
            song.searchable_text = searchable_text(&song.display_name, &song.source_paths);
        }

        Self {
            songs,
            midi_files_seen: paths.len(),
            unreadable_files,
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

fn searchable_text(display_name: &str, paths: &[PathBuf]) -> String {
    let mut searchable = display_name.to_lowercase();
    for path in paths {
        searchable.push(' ');
        searchable.push_str(&path.to_string_lossy().to_lowercase());
    }
    searchable
}

fn collect_midi_paths(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
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
                files.push(path);
            }
        }
    }
    files
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
                source_paths: vec![PathBuf::from("D:/Piano/Debussy/Clair de Lune.mid")],
                searchable_text: searchable_text(
                    "Clair de Lune.mid",
                    &[PathBuf::from("D:/Piano/Debussy/Clair de Lune.mid")],
                ),
            }],
            midi_files_seen: 1,
            unreadable_files: 0,
        };

        assert_eq!(index.search("debussy lune").len(), 1);
        assert!(index.search("debussy moonlight").is_empty());
    }
}
