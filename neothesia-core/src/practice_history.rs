use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use ron::extensions::Extensions;
use serde::{Deserialize, Serialize};

use crate::practice::{AttemptSummary, PracticeBreakdown, PracticeHands};

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
    #[serde(default)]
    pub hands: PracticeHands,
    pub speed: f32,
    pub summary: AttemptSummary,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub enum PracticeTrackMode {
    Mute,
    Auto,
    Human,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub struct PracticeTrackSetup {
    pub track_id: usize,
    pub mode: PracticeTrackMode,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub struct PracticeLoopSetup {
    pub enabled: bool,
    pub start_measure: usize,
    pub end_measure: usize,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
pub struct SongPracticeSetup {
    pub tracks: Vec<PracticeTrackSetup>,
    pub speed: f32,
    pub loop_setup: Option<PracticeLoopSetup>,
    #[serde(default)]
    pub source_path: Option<PathBuf>,
    #[serde(default)]
    pub last_used_unix_ms: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SongLibraryState {
    pub source_path: Option<PathBuf>,
    pub favorite: bool,
    pub queue_position: Option<usize>,
}

impl PracticeSession {
    pub fn new(
        kind: PracticeSessionKind,
        hands: PracticeHands,
        speed: f32,
        summary: AttemptSummary,
    ) -> Self {
        Self {
            recorded_at_unix_ms: unix_time_ms(),
            kind,
            hands,
            speed,
            summary,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
pub struct SongPracticeHistory {
    pub display_name: String,
    #[serde(default)]
    pub setup: Option<SongPracticeSetup>,
    #[serde(default)]
    pub library: SongLibraryState,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecentPracticeSummary {
    pub kind: PracticeSessionKind,
    pub hands: PracticeHands,
    pub recorded_at_unix_ms: u64,
    pub accuracy: Option<f32>,
    pub speed: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PracticeHistoryOverview {
    pub total_sessions: usize,
    /// Oldest to newest, so the UI can read it as a progression.
    pub recent: Vec<RecentPracticeSummary>,
    pub trend_kind: Option<PracticeSessionKind>,
    pub trend_hands: Option<PracticeHands>,
    pub trend_attempts: usize,
    pub accuracy_delta: Option<f32>,
    pub speed_delta: Option<f32>,
    pub weak_measures: Vec<WeakMeasure>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecentSongSummary {
    pub content_id: String,
    pub display_name: String,
    pub source_path: Option<PathBuf>,
    pub last_used_unix_ms: u64,
    pub session_count: usize,
    pub latest_accuracy: Option<f32>,
    pub favorite: bool,
    pub queue_position: Option<usize>,
    pub recommended_measures: Option<(usize, usize)>,
    pub review: Option<ReviewStatus>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ReviewReason {
    NeedsEvidence,
    NeedsReinforcement,
    FirstMastery,
    Consolidating,
    Stable,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewStatus {
    pub due_at_unix_ms: u64,
    pub is_due: bool,
    pub interval_days: u16,
    pub days_until_due: u16,
    pub mastery_streak: usize,
    pub latest_accuracy: Option<f32>,
    pub reason: ReviewReason,
}

impl SongPracticeHistory {
    pub fn review_status(&self, now_unix_ms: u64) -> Option<ReviewStatus> {
        let latest = self.sessions.last()?;
        let scope = (latest.kind, latest.hands);
        let mastery_streak = self
            .sessions
            .iter()
            .rev()
            .filter(|session| (session.kind, session.hands) == scope)
            .take_while(|session| session_is_mastered(session))
            .count();
        let latest_accuracy = latest.summary.overall.accuracy();
        let (interval_days, reason) = if latest_accuracy.is_none() {
            (0, ReviewReason::NeedsEvidence)
        } else if mastery_streak == 0 {
            (0, ReviewReason::NeedsReinforcement)
        } else if mastery_streak == 1 {
            (1, ReviewReason::FirstMastery)
        } else if mastery_streak == 2 {
            (3, ReviewReason::Consolidating)
        } else if mastery_streak < 5 {
            (7, ReviewReason::Stable)
        } else {
            (14, ReviewReason::Stable)
        };
        let due_at_unix_ms = latest
            .recorded_at_unix_ms
            .saturating_add(u64::from(interval_days) * 24 * 60 * 60 * 1_000);
        let day_ms = 24 * 60 * 60 * 1_000_u64;
        let days_until_due = due_at_unix_ms
            .saturating_sub(now_unix_ms)
            .div_ceil(day_ms)
            .min(u64::from(u16::MAX)) as u16;
        Some(ReviewStatus {
            due_at_unix_ms,
            is_due: now_unix_ms >= due_at_unix_ms,
            interval_days,
            days_until_due,
            mastery_streak,
            latest_accuracy,
            reason,
        })
    }

    pub fn weak_measures(&self, limit: usize) -> Vec<WeakMeasure> {
        self.weak_measures_for_scope(None, limit)
    }

    fn weak_measures_for_scope(
        &self,
        hands: Option<PracticeHands>,
        limit: usize,
    ) -> Vec<WeakMeasure> {
        let mut measures = BTreeMap::<usize, (PracticeBreakdown, usize)>::new();
        for session in self
            .sessions
            .iter()
            .filter(|session| hands.is_none_or(|hands| session.hands == hands))
        {
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
        let hands = self.sessions.last()?.hands;
        let weakest = self
            .weak_measures_for_scope(Some(hands), usize::MAX)
            .into_iter()
            .find(is_reliable_weakness)?;

        Some(WeakPassageRecommendation {
            start_measure: weakest.measure,
            end_measure: weakest.measure.saturating_add(1),
            weakest_accuracy: weakest.accuracy,
            judged_notes: weakest.judged_notes,
            attempts: weakest.attempts,
        })
    }

    pub fn overview(
        &self,
        recent_limit: usize,
        weak_measure_limit: usize,
    ) -> PracticeHistoryOverview {
        let mut recent: Vec<_> = self
            .sessions
            .iter()
            .rev()
            .take(recent_limit)
            .map(|session| RecentPracticeSummary {
                kind: session.kind,
                hands: session.hands,
                recorded_at_unix_ms: session.recorded_at_unix_ms,
                accuracy: session.summary.overall.accuracy(),
                speed: session.speed,
            })
            .collect();
        recent.reverse();

        let trend_scope = self
            .sessions
            .last()
            .map(|session| (session.kind, session.hands));
        let mut comparable: Vec<_> = self
            .sessions
            .iter()
            .rev()
            .filter(|session| Some((session.kind, session.hands)) == trend_scope)
            .take(recent_limit)
            .collect();
        comparable.reverse();
        let accuracy_delta = first_last_delta(
            comparable
                .iter()
                .filter_map(|session| session.summary.overall.accuracy()),
        );
        let speed_delta = first_last_delta(
            comparable
                .iter()
                .map(|session| session.speed)
                .filter(|speed| speed.is_finite()),
        );

        PracticeHistoryOverview {
            total_sessions: self.sessions.len(),
            recent,
            trend_kind: trend_scope.map(|(kind, _)| kind),
            trend_hands: trend_scope.map(|(_, hands)| hands),
            trend_attempts: comparable.len(),
            accuracy_delta,
            speed_delta,
            weak_measures: self
                .weak_measures_for_scope(trend_scope.map(|(_, hands)| hands), usize::MAX)
                .into_iter()
                .filter(is_reliable_weakness)
                .take(weak_measure_limit)
                .collect(),
        }
    }
}

fn is_reliable_weakness(measure: &WeakMeasure) -> bool {
    measure.attempts >= 2 && measure.judged_notes >= 8 && measure.accuracy < 0.9
}

fn session_is_mastered(session: &PracticeSession) -> bool {
    let snapshot = session.summary.overall;
    let Some(accuracy) = snapshot.accuracy() else {
        return false;
    };
    let on_time_ratio = if snapshot.matched_notes != 0 {
        snapshot.on_time_notes as f32 / snapshot.matched_notes as f32
    } else {
        0.0
    };
    accuracy >= 0.9 && on_time_ratio >= 0.7
}

fn first_last_delta(mut values: impl Iterator<Item = f32>) -> Option<f32> {
    let first = values.next()?;
    let mut last = first;
    let mut count = 1;
    for value in values {
        last = value;
        count += 1;
    }
    (count >= 2).then_some(last - first)
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
                setup: None,
                library: SongLibraryState::default(),
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

    pub fn save_setup(
        &mut self,
        song_id: &str,
        display_name: &str,
        mut setup: SongPracticeSetup,
    ) -> Result<(), PracticeHistoryError> {
        setup.last_used_unix_ms = unix_time_ms();
        let song = self
            .songs_mut()
            .entry(song_id.to_owned())
            .or_insert_with(|| SongPracticeHistory {
                display_name: display_name.to_owned(),
                setup: None,
                library: SongLibraryState::default(),
                sessions: Vec::new(),
            });
        song.display_name = display_name.to_owned();
        if setup.source_path.is_some() {
            song.library.source_path = setup.source_path.clone();
        }
        song.setup = Some(setup);
        self.save()
    }

    pub fn setup(&self, song_id: &str) -> Option<&SongPracticeSetup> {
        self.song(song_id)?.setup.as_ref()
    }

    pub fn recent_songs(&self, limit: usize) -> Vec<RecentSongSummary> {
        let now_unix_ms = unix_time_ms();
        let mut songs: Vec<_> = self
            .songs()
            .iter()
            .filter_map(|(content_id, song)| {
                let setup = song.setup.as_ref();
                if setup.is_none()
                    && song.sessions.is_empty()
                    && !song.library.favorite
                    && song.library.queue_position.is_none()
                {
                    return None;
                }
                let latest_session = song.sessions.last();
                let last_used_unix_ms = setup.map_or(0, |setup| setup.last_used_unix_ms).max(
                    latest_session
                        .map(|session| session.recorded_at_unix_ms)
                        .unwrap_or_default(),
                );
                Some(RecentSongSummary {
                    content_id: content_id.clone(),
                    display_name: song.display_name.clone(),
                    source_path: song
                        .library
                        .source_path
                        .clone()
                        .or_else(|| setup.and_then(|setup| setup.source_path.clone())),
                    last_used_unix_ms,
                    session_count: song.sessions.len(),
                    latest_accuracy: latest_session
                        .and_then(|session| session.summary.overall.accuracy()),
                    favorite: song.library.favorite,
                    queue_position: song.library.queue_position,
                    recommended_measures: song
                        .recommended_passage()
                        .map(|passage| (passage.start_measure, passage.end_measure)),
                    review: song.review_status(now_unix_ms),
                })
            })
            .collect();
        songs.sort_by(|left, right| {
            right
                .last_used_unix_ms
                .cmp(&left.last_used_unix_ms)
                .then_with(|| left.display_name.cmp(&right.display_name))
                .then_with(|| left.content_id.cmp(&right.content_id))
        });
        songs.truncate(limit);
        songs
    }

    pub fn set_favorite(
        &mut self,
        song_id: &str,
        display_name: &str,
        source_path: Option<PathBuf>,
        favorite: bool,
    ) -> Result<(), PracticeHistoryError> {
        let song = self.ensure_song(song_id, display_name);
        song.library.favorite = favorite;
        if source_path.is_some() {
            song.library.source_path = source_path;
        }
        self.save()
    }

    pub fn set_queued(
        &mut self,
        song_id: &str,
        display_name: &str,
        source_path: Option<PathBuf>,
        queued: bool,
    ) -> Result<(), PracticeHistoryError> {
        if queued {
            let next = self
                .songs()
                .values()
                .filter_map(|song| song.library.queue_position)
                .max()
                .map_or(0, |position| position + 1);
            let song = self.ensure_song(song_id, display_name);
            if song.library.queue_position.is_none() {
                song.library.queue_position = Some(next);
            }
            if source_path.is_some() {
                song.library.source_path = source_path;
            }
        } else if let Some(song) = self.songs_mut().get_mut(song_id) {
            song.library.queue_position = None;
        }
        self.normalize_queue();
        self.save()
    }

    pub fn move_in_queue(
        &mut self,
        song_id: &str,
        direction: isize,
    ) -> Result<bool, PracticeHistoryError> {
        let mut queue: Vec<_> = self
            .songs()
            .iter()
            .filter_map(|(id, song)| Some((song.library.queue_position?, id.clone())))
            .collect();
        queue.sort();
        let Some(index) = queue.iter().position(|(_, id)| id == song_id) else {
            return Ok(false);
        };
        let target = index.saturating_add_signed(direction).min(queue.len() - 1);
        if target == index {
            return Ok(false);
        }
        queue.swap(index, target);
        for (position, (_, id)) in queue.into_iter().enumerate() {
            if let Some(song) = self.songs_mut().get_mut(&id) {
                song.library.queue_position = Some(position);
            }
        }
        self.save()?;
        Ok(true)
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

    fn ensure_song(&mut self, song_id: &str, display_name: &str) -> &mut SongPracticeHistory {
        let song = self
            .songs_mut()
            .entry(song_id.to_owned())
            .or_insert_with(|| SongPracticeHistory {
                display_name: display_name.to_owned(),
                setup: None,
                library: SongLibraryState::default(),
                sessions: Vec::new(),
            });
        song.display_name = display_name.to_owned();
        song
    }

    fn normalize_queue(&mut self) {
        let mut queue: Vec<_> = self
            .songs()
            .iter()
            .filter_map(|(id, song)| Some((song.library.queue_position?, id.clone())))
            .collect();
        queue.sort();
        for (position, (_, id)) in queue.into_iter().enumerate() {
            if let Some(song) = self.songs_mut().get_mut(&id) {
                song.library.queue_position = Some(position);
            }
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
            hands: PracticeHands::Both,
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
    fn histories_saved_before_hand_scopes_remain_readable() {
        let path = temp_history_path("legacy-hand-scope");
        let mut store = PracticeHistoryStore::load(&path);
        store
            .record_session("content-id", "Song.mid", session(2, 8, 2))
            .unwrap();

        let legacy_contents = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                !line.starts_with("hands:") && !line.starts_with("setup:")
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&path, legacy_contents).unwrap();

        let loaded = PracticeHistoryStore::load(&path);
        assert_eq!(
            loaded.song("content-id").unwrap().sessions[0].hands,
            PracticeHands::Unspecified
        );
        assert_eq!(loaded.setup("content-id"), None);

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn legacy_song_history_defaults_to_unorganized_library_state() {
        let history: SongPracticeHistory = ron_options()
            .from_str("(display_name:\"Legacy.mid\",sessions:[])")
            .unwrap();
        assert_eq!(history.library, SongLibraryState::default());
    }

    #[test]
    fn song_setup_is_saved_by_content_identity() {
        let path = temp_history_path("song-setup");
        let mut store = PracticeHistoryStore::load(&path);
        let setup = SongPracticeSetup {
            tracks: vec![PracticeTrackSetup {
                track_id: 3,
                mode: PracticeTrackMode::Human,
                visible: true,
            }],
            speed: 0.65,
            loop_setup: Some(PracticeLoopSetup {
                enabled: true,
                start_measure: 7,
                end_measure: 8,
            }),
            source_path: Some(PathBuf::from("C:/Music/Old Name.mid")),
            last_used_unix_ms: 0,
        };
        store
            .save_setup("content-id", "Old Name.mid", setup.clone())
            .unwrap();
        store
            .save_setup("content-id", "New Name.mid", setup.clone())
            .unwrap();

        let loaded = PracticeHistoryStore::load(&path);
        let loaded_setup = loaded.setup("content-id").unwrap();
        assert_eq!(loaded_setup.tracks, setup.tracks);
        assert_eq!(loaded_setup.speed, setup.speed);
        assert_eq!(loaded_setup.loop_setup, setup.loop_setup);
        assert_eq!(loaded_setup.source_path, setup.source_path);
        assert!(loaded_setup.last_used_unix_ms > 0);
        assert_eq!(
            loaded.song("content-id").unwrap().display_name,
            "New Name.mid"
        );

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn recent_songs_are_ordered_by_latest_activity() {
        let path = temp_history_path("recent-songs");
        let mut store = PracticeHistoryStore::load(&path);
        let setup = |path: &str| SongPracticeSetup {
            tracks: Vec::new(),
            speed: 1.0,
            loop_setup: None,
            source_path: Some(PathBuf::from(path)),
            last_used_unix_ms: 0,
        };
        store
            .save_setup("older", "Older.mid", setup("C:/Older.mid"))
            .unwrap();
        store
            .save_setup("newer", "Newer.mid", setup("C:/Newer.mid"))
            .unwrap();
        store
            .songs_mut()
            .get_mut("older")
            .unwrap()
            .setup
            .as_mut()
            .unwrap()
            .last_used_unix_ms = 1;
        store
            .songs_mut()
            .get_mut("newer")
            .unwrap()
            .setup
            .as_mut()
            .unwrap()
            .last_used_unix_ms = 2;

        let recent = store.recent_songs(1);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].content_id, "newer");
        assert_eq!(recent[0].source_path, Some(PathBuf::from("C:/Newer.mid")));

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn favorites_and_queue_order_persist_and_move_deterministically() {
        let path = temp_history_path("library-organization");
        let mut store = PracticeHistoryStore::load(&path);
        store
            .set_favorite(
                "alpha",
                "Alpha.mid",
                Some(PathBuf::from("C:/Alpha.mid")),
                true,
            )
            .unwrap();
        store.set_queued("alpha", "Alpha.mid", None, true).unwrap();
        store.set_queued("beta", "Beta.mid", None, true).unwrap();
        store.set_queued("gamma", "Gamma.mid", None, true).unwrap();

        assert!(store.move_in_queue("gamma", -1).unwrap());
        assert!(store.move_in_queue("gamma", -1).unwrap());
        assert!(!store.move_in_queue("gamma", -1).unwrap());
        store.set_queued("alpha", "Alpha.mid", None, false).unwrap();

        let loaded = PracticeHistoryStore::load(&path);
        let summaries = loaded.recent_songs(10);
        assert!(
            summaries
                .iter()
                .find(|song| song.content_id == "alpha")
                .unwrap()
                .favorite
        );
        let mut queued: Vec<_> = summaries
            .iter()
            .filter_map(|song| Some((song.queue_position?, song.content_id.as_str())))
            .collect();
        queued.sort();
        assert_eq!(queued, vec![(0, "gamma"), (1, "beta")]);

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
            setup: None,
            library: SongLibraryState::default(),
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
            setup: None,
            library: SongLibraryState::default(),
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
            setup: None,
            library: SongLibraryState::default(),
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
    fn overview_orders_recent_attempts_and_computes_visible_trends() {
        let mut first = session(2, 5, 5);
        first.recorded_at_unix_ms = 10;
        first.speed = 0.6;
        let mut second = session(2, 7, 3);
        second.recorded_at_unix_ms = 20;
        second.speed = 0.7;
        let mut third = session(2, 9, 1);
        third.recorded_at_unix_ms = 30;
        third.speed = 0.8;
        let history = SongPracticeHistory {
            display_name: "Song.mid".to_owned(),
            setup: None,
            library: SongLibraryState::default(),
            sessions: vec![first, second, third],
        };

        let overview = history.overview(2, 1);

        assert_eq!(overview.total_sessions, 3);
        assert_eq!(overview.trend_kind, Some(PracticeSessionKind::WholeSong));
        assert_eq!(overview.trend_hands, Some(PracticeHands::Both));
        assert_eq!(overview.trend_attempts, 2);
        assert_eq!(
            overview
                .recent
                .iter()
                .map(|item| item.recorded_at_unix_ms)
                .collect::<Vec<_>>(),
            vec![20, 30]
        );
        assert!((overview.accuracy_delta.unwrap() - 0.2).abs() < f32::EPSILON);
        assert!((overview.speed_delta.unwrap() - 0.1).abs() < f32::EPSILON);
        assert_eq!(overview.weak_measures.len(), 1);
    }

    #[test]
    fn overview_omits_trends_without_two_judged_attempts() {
        let mut empty = session(2, 0, 0);
        empty.speed = f32::NAN;
        let history = SongPracticeHistory {
            display_name: "Song.mid".to_owned(),
            setup: None,
            library: SongLibraryState::default(),
            sessions: vec![empty],
        };

        let overview = history.overview(5, 4);

        assert_eq!(overview.accuracy_delta, None);
        assert_eq!(overview.speed_delta, None);
    }

    #[test]
    fn overview_does_not_compare_whole_song_and_loop_accuracy() {
        let mut whole_old = session(2, 5, 5);
        whole_old.speed = 0.6;
        let mut unrelated_loop = session(8, 1, 9);
        unrelated_loop.kind = PracticeSessionKind::Loop {
            start_measure: 8,
            end_measure: 9,
        };
        unrelated_loop.speed = 0.9;
        let mut whole_new = session(2, 8, 2);
        whole_new.speed = 0.7;
        let history = SongPracticeHistory {
            display_name: "Song.mid".to_owned(),
            setup: None,
            library: SongLibraryState::default(),
            sessions: vec![whole_old, unrelated_loop, whole_new],
        };

        let overview = history.overview(5, 4);

        assert_eq!(overview.trend_attempts, 2);
        assert!((overview.accuracy_delta.unwrap() - 0.3).abs() < f32::EPSILON);
        assert!((overview.speed_delta.unwrap() - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn overview_does_not_compare_different_hand_goals() {
        let mut both_old = session(2, 5, 5);
        both_old.speed = 0.6;
        let mut right_only = session(2, 1, 9);
        right_only.hands = PracticeHands::Right;
        right_only.speed = 0.9;
        let mut both_new = session(2, 8, 2);
        both_new.speed = 0.7;
        let history = SongPracticeHistory {
            display_name: "Song.mid".to_owned(),
            setup: None,
            library: SongLibraryState::default(),
            sessions: vec![both_old, right_only, both_new],
        };

        let overview = history.overview(5, 4);

        assert_eq!(overview.trend_hands, Some(PracticeHands::Both));
        assert_eq!(overview.trend_attempts, 2);
        assert!((overview.accuracy_delta.unwrap() - 0.3).abs() < f32::EPSILON);
        assert!((overview.speed_delta.unwrap() - 0.1).abs() < f32::EPSILON);
        assert!((overview.weak_measures[0].accuracy - 0.65).abs() < f32::EPSILON);
    }

    #[test]
    fn spaced_review_uses_mastery_streak_and_explains_due_state() {
        let day = 24 * 60 * 60 * 1_000_u64;
        let mastered = |recorded_at_unix_ms| {
            let mut item = session(2, 10, 0);
            item.recorded_at_unix_ms = recorded_at_unix_ms;
            item.summary.overall.on_time_notes = 8;
            item
        };
        let history = SongPracticeHistory {
            display_name: "Song.mid".to_owned(),
            setup: None,
            library: SongLibraryState::default(),
            sessions: vec![mastered(day), mastered(day * 2), mastered(day * 3)],
        };

        let waiting = history.review_status(day * 9).unwrap();
        assert_eq!(waiting.reason, ReviewReason::Stable);
        assert_eq!(waiting.mastery_streak, 3);
        assert_eq!(waiting.interval_days, 7);
        assert_eq!(waiting.days_until_due, 1);
        assert!(!waiting.is_due);

        let due = history.review_status(day * 10).unwrap();
        assert!(due.is_due);
        assert_eq!(due.days_until_due, 0);
    }

    #[test]
    fn weak_latest_take_is_due_immediately() {
        let mut weak = session(2, 7, 3);
        weak.recorded_at_unix_ms = 100;
        weak.summary.overall.on_time_notes = 7;
        let history = SongPracticeHistory {
            display_name: "Song.mid".to_owned(),
            setup: None,
            library: SongLibraryState::default(),
            sessions: vec![weak],
        };

        let review = history.review_status(100).unwrap();
        assert!(review.is_due);
        assert_eq!(review.interval_days, 0);
        assert_eq!(review.reason, ReviewReason::NeedsReinforcement);
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
