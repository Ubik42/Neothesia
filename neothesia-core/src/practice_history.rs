use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use ron::extensions::Extensions;
use serde::{Deserialize, Serialize};

use crate::practice::{AttemptSummary, PracticeBreakdown};

const MAX_SESSIONS_PER_SONG: usize = 200;

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub enum PracticeSessionKind {
    WholeSong,
    Loop {
        start_measure: usize,
        end_measure: usize,
    },
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct PracticeSession {
    pub recorded_at_unix_ms: u64,
    pub kind: PracticeSessionKind,
    pub speed: f32,
    pub summary: AttemptSummary,
}

impl PracticeSession {
    pub fn new(kind: PracticeSessionKind, speed: f32, summary: AttemptSummary) -> Self {
        Self {
            recorded_at_unix_ms: unix_time_ms(),
            kind,
            speed,
            summary,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
pub struct SongPracticeHistory {
    pub display_name: String,
    pub sessions: Vec<PracticeSession>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeakMeasure {
    pub measure: usize,
    pub accuracy: f32,
    pub judged_notes: usize,
    pub attempts: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeakPassageRecommendation {
    pub start_measure: usize,
    pub end_measure: usize,
    pub weakest_accuracy: f32,
    pub judged_notes: usize,
    pub attempts: usize,
}

impl SongPracticeHistory {
    pub fn weak_measures(&self, limit: usize) -> Vec<WeakMeasure> {
        let mut measures = BTreeMap::<usize, (PracticeBreakdown, usize)>::new();
        for session in &self.sessions {
            for item in &session.summary.measures {
                let (total, attempts) = measures.entry(item.measure).or_default();
                total.target_notes += item.breakdown.target_notes;
                total.matched_notes += item.breakdown.matched_notes;
                total.on_time_notes += item.breakdown.on_time_notes;
                total.early_notes += item.breakdown.early_notes;
                total.late_notes += item.breakdown.late_notes;
                total.wrong_notes += item.breakdown.wrong_notes;
                total.missed_notes += item.breakdown.missed_notes;
                *attempts += 1;
            }
        }

        let mut weak: Vec<_> = measures
            .into_iter()
            .filter_map(|(measure, (breakdown, attempts))| {
                let judged =
                    breakdown.matched_notes + breakdown.wrong_notes + breakdown.missed_notes;
                Some(WeakMeasure {
                    measure,
                    accuracy: breakdown.accuracy()?,
                    judged_notes: judged,
                    attempts,
                })
            })
            .collect();
        weak.sort_by(|left, right| {
            left.accuracy
                .total_cmp(&right.accuracy)
                .then_with(|| right.judged_notes.cmp(&left.judged_notes))
                .then_with(|| left.measure.cmp(&right.measure))
        });
        weak.truncate(limit);
        weak
    }

    /// Returns a conservative two-measure practice target. A recommendation
    /// needs repeated evidence so a single exploratory take cannot create a
    /// persistent "weak" label.
    pub fn recommended_passage(&self) -> Option<WeakPassageRecommendation> {
        let weakest = self.weak_measures(usize::MAX).into_iter().find(|measure| {
            measure.attempts >= 2 && measure.judged_notes >= 8 && measure.accuracy < 0.9
        })?;

        Some(WeakPassageRecommendation {
            start_measure: weakest.measure,
            end_measure: weakest.measure.saturating_add(1),
            weakest_accuracy: weakest.accuracy,
            judged_notes: weakest.judged_notes,
            attempts: weakest.attempts,
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
struct PracticeHistoryV1 {
    songs: BTreeMap<String, SongPracticeHistory>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
enum PracticeHistoryFile {
    V1(PracticeHistoryV1),
}

impl Default for PracticeHistoryFile {
    fn default() -> Self {
        Self::V1(PracticeHistoryV1::default())
    }
}

#[derive(Debug)]
pub struct PracticeHistoryStore {
    path: PathBuf,
    file: PracticeHistoryFile,
}

impl PracticeHistoryStore {
    pub fn load(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let file = match fs::read_to_string(&path) {
            Ok(contents) => match ron_options().from_str(&contents) {
                Ok(file) => file,
                Err(error) => {
                    log::error!("Practice history is damaged and will be quarantined: {error:#?}");
                    quarantine_corrupt_file(&path);
                    PracticeHistoryFile::default()
                }
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => PracticeHistoryFile::default(),
            Err(error) => {
                log::error!("Could not read practice history: {error}");
                PracticeHistoryFile::default()
            }
        };

        Self { path, file }
    }

    pub fn record_session(
        &mut self,
        song_id: &str,
        display_name: &str,
        session: PracticeSession,
    ) -> Result<(), PracticeHistoryError> {
        let song = self
            .songs_mut()
            .entry(song_id.to_owned())
            .or_insert_with(|| SongPracticeHistory {
                display_name: display_name.to_owned(),
                sessions: Vec::new(),
            });
        song.display_name = display_name.to_owned();
        song.sessions.push(session);
        let excess = song.sessions.len().saturating_sub(MAX_SESSIONS_PER_SONG);
        if excess != 0 {
            song.sessions.drain(..excess);
        }
        self.save()
    }

    pub fn song(&self, song_id: &str) -> Option<&SongPracticeHistory> {
        self.songs().get(song_id)
    }

    pub fn session_count(&self, song_id: &str) -> usize {
        self.song(song_id).map_or(0, |song| song.sessions.len())
    }

    pub fn save(&self) -> Result<(), PracticeHistoryError> {
        let contents = ron_options().to_string_pretty(
            &self.file,
            ron::ser::PrettyConfig::default().struct_names(true),
        )?;
        atomic_write(&self.path, contents.as_bytes())?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn songs(&self) -> &BTreeMap<String, SongPracticeHistory> {
        match &self.file {
            PracticeHistoryFile::V1(file) => &file.songs,
        }
    }

    fn songs_mut(&mut self) -> &mut BTreeMap<String, SongPracticeHistory> {
        match &mut self.file {
            PracticeHistoryFile::V1(file) => &mut file.songs,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PracticeHistoryError {
    #[error("could not serialize practice history: {0}")]
    Serialize(#[from] ron::Error),
    #[error("could not write practice history: {0}")]
    Io(#[from] io::Error),
}

fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(Extensions::UNWRAP_VARIANT_NEWTYPES)
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

fn quarantine_corrupt_file(path: &Path) {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("practice-history");
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("ron");
    let quarantine =
        path.with_file_name(format!("{stem}.corrupt-{}.{}", unix_time_ms(), extension));
    if let Err(error) = fs::rename(path, &quarantine) {
        log::error!("Could not quarantine damaged practice history: {error}");
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("practice-history.ron");
    let temporary = parent.join(format!(
        ".{file_name}.tmp-{}-{}",
        std::process::id(),
        unix_time_ms()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::practice::{MeasureSummary, PracticeSnapshot};

    fn temp_history_path(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "neothesia-history-test-{}-{}-{name}",
            std::process::id(),
            unix_time_ms()
        ));
        fs::create_dir_all(&directory).unwrap();
        directory.join("practice-history.ron")
    }

    fn session(measure: usize, matched: usize, missed: usize) -> PracticeSession {
        PracticeSession {
            recorded_at_unix_ms: 1,
            kind: PracticeSessionKind::WholeSong,
            speed: 0.75,
            summary: AttemptSummary {
                overall: PracticeSnapshot {
                    matched_notes: matched,
                    missed_notes: missed,
                    ..PracticeSnapshot::default()
                },
                measures: vec![MeasureSummary {
                    measure,
                    breakdown: PracticeBreakdown {
                        target_notes: matched + missed,
                        matched_notes: matched,
                        missed_notes: missed,
                        ..PracticeBreakdown::default()
                    },
                }],
                parts: Vec::new(),
            },
        }
    }

    #[test]
    fn round_trip_survives_rename_of_the_source_song() {
        let path = temp_history_path("round-trip");
        let mut store = PracticeHistoryStore::load(&path);
        store
            .record_session("content-id", "Old Name.mid", session(2, 8, 2))
            .unwrap();
        store
            .record_session("content-id", "New Name.mid", session(3, 9, 1))
            .unwrap();

        let loaded = PracticeHistoryStore::load(&path);
        let song = loaded.song("content-id").unwrap();
        assert_eq!(song.display_name, "New Name.mid");
        assert_eq!(song.sessions.len(), 2);

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn damaged_history_is_quarantined_and_does_not_block_startup() {
        let path = temp_history_path("corrupt");
        fs::write(&path, "this is not valid RON").unwrap();

        let store = PracticeHistoryStore::load(&path);

        assert_eq!(store.session_count("anything"), 0);
        assert!(!path.exists());
        assert!(
            fs::read_dir(path.parent().unwrap())
                .unwrap()
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"))
        );

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn weak_measures_aggregate_across_sessions() {
        let history = SongPracticeHistory {
            display_name: "Song.mid".to_owned(),
            sessions: vec![session(4, 5, 5), session(4, 8, 2), session(7, 9, 1)],
        };

        let weak = history.weak_measures(2);

        assert_eq!(weak[0].measure, 4);
        assert_eq!(weak[0].judged_notes, 20);
        assert_eq!(weak[0].attempts, 2);
        assert!((weak[0].accuracy - 0.65).abs() < f32::EPSILON);
        assert_eq!(weak[1].measure, 7);
    }

    #[test]
    fn passage_recommendation_requires_repeated_evidence() {
        let mut history = SongPracticeHistory {
            display_name: "Song.mid".to_owned(),
            sessions: vec![session(4, 4, 6), session(8, 9, 1)],
        };

        assert_eq!(history.recommended_passage(), None);

        history.sessions.push(session(4, 6, 4));
        let recommendation = history.recommended_passage().unwrap();
        assert_eq!(recommendation.start_measure, 4);
        assert_eq!(recommendation.end_measure, 5);
        assert_eq!(recommendation.attempts, 2);
        assert_eq!(recommendation.judged_notes, 20);
        assert!((recommendation.weakest_accuracy - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn passage_recommendation_ignores_small_samples_and_mastered_measures() {
        let history = SongPracticeHistory {
            display_name: "Song.mid".to_owned(),
            sessions: vec![
                session(2, 2, 0),
                session(2, 1, 1),
                session(6, 9, 1),
                session(6, 9, 1),
            ],
        };

        assert_eq!(history.recommended_passage(), None);
    }

    #[test]
    fn history_retains_only_the_most_recent_sessions() {
        let path = temp_history_path("retention");
        let mut store = PracticeHistoryStore::load(&path);
        for index in 0..(MAX_SESSIONS_PER_SONG + 3) {
            let mut item = session(1, 1, 0);
            item.recorded_at_unix_ms = index as u64;
            store.record_session("song", "Song.mid", item).unwrap();
        }

        let song = store.song("song").unwrap();
        assert_eq!(song.sessions.len(), MAX_SESSIONS_PER_SONG);
        assert_eq!(song.sessions.first().unwrap().recorded_at_unix_ms, 3);

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
