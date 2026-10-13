//! Reviewable repair for the old left-only generated MIDI layout. Saved attempts
//! are evidence and are deliberately never rewritten here.
use crate::Player;
use neothesia_core::practice_history::PracticeTrackSetup;
use neothesia_core::{
    exercise::ExerciseHands,
    library::{self, SongSidecar},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

struct Review {
    fingerprint: String,
    active: PathBuf,
    original: PathBuf,
    sidecar: SongSidecar,
    source_count: usize,
    target_count: usize,
    setting_tracks: usize,
    history_tracks: usize,
    warnings: Vec<String>,
    captured: Option<Captured>,
    condition_conflict: bool,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Captured {
    content_id: String,
    settings: Option<crate::preferences::SessionSettings>,
    setup: Option<neothesia_core::practice_history::SongPracticeSetup>,
}
fn capture_path(data: &Path, cid: &str) -> PathBuf {
    data.join("legacy-track-reviews")
        .join(cid)
        .join("first-load.json")
}
pub(super) fn capture_before_install(
    p: &Player,
    file: &midi_file::MidiFile,
    spec: neothesia_core::exercise::ExerciseSpec,
) -> Result<(), String> {
    if spec.hands != ExerciseHands::Left {
        return Ok(());
    }
    let cid = &file.content_id;
    let original = p.data.join("song-assets").join(format!("{cid}.mid"));
    if !original.is_file() || library::load_song_sidecar(&original, cid).is_err() {
        return Ok(());
    }
    let Ok(old) = midi_file::MidiFile::new(&original) else {
        return Ok(());
    };
    let nonempty: Vec<_> = old.tracks.iter().filter(|t| !t.notes.is_empty()).collect();
    if nonempty.len() != 1
        || nonempty[0].track_id != 1
        || nonempty[0].notes.iter().any(|n| n.channel != 1)
    {
        return Ok(());
    }
    let settings = p.preferences.songs.get(cid).cloned();
    let setup = p.history.setup(cid).cloned();
    let old_track = |tracks: &[PracticeTrackSetup]| {
        tracks.iter().any(|t| t.track_id == 1) && !tracks.iter().any(|t| t.track_id == 2)
    };
    if !settings
        .as_ref()
        .is_some_and(|s| old_track(&s.tracks) || s.parts.contains_key(&1))
        && !setup.as_ref().is_some_and(|s| old_track(&s.tracks))
    {
        return Ok(());
    }
    let path = capture_path(&p.data, cid);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let encoded = serde_json::to_vec_pretty(&Captured {
        content_id: cid.clone(),
        settings,
        setup,
    })
    .map_err(|e| e.to_string())?;
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut out) => {
            use std::io::Write;
            out.write_all(&encoded)
                .and_then(|_| out.sync_all())
                .map_err(|e| e.to_string())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(format!("旧版练习条件无法保留：{e}")),
    }
}

fn bytes(path: &Path) -> Result<Vec<u8>, String> {
    match fs::read(path) {
        Ok(v) => Ok(v),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("无法读取 {}：{e}", path.display())),
    }
}
fn review(p: &Player, content_id: &str) -> Result<Review, String> {
    let file = p.file.as_ref().ok_or("请先打开技术练习")?;
    if file.content_id != content_id || p.exercise.is_none_or(|s| s.hands != ExerciseHands::Left) {
        return Err("请打开对应的单左手技术练习后重新检查".into());
    }
    let active = p.annotation_path(file)?;
    let original = p.data.join("song-assets").join(format!("{content_id}.mid"));
    if active == original || !original.is_file() {
        return Err("没有找到保留的旧版单左手 MIDI 原件，无法证明旧音轨身份".into());
    }
    let old =
        midi_file::MidiFile::new(&original).map_err(|e| format!("旧版 MIDI 无法读取：{e}"))?;
    library::load_song_sidecar(&original, content_id)
        .map_err(|e| format!("旧版 MIDI 的个人资料身份无法核对：{e}"))?;
    let old_nonempty: Vec<_> = old.tracks.iter().filter(|t| !t.notes.is_empty()).collect();
    if old_nonempty.len() != 1
        || old_nonempty[0].track_id != 1
        || old_nonempty[0].notes.iter().any(|n| n.channel != 1)
    {
        return Err("旧版 MIDI 不是唯一的左手 1 号音轨，不能推断编号迁移".into());
    }
    let target = file
        .tracks
        .iter()
        .find(|t| t.track_id == 2 && !t.notes.is_empty())
        .ok_or("当前练习没有独立的左手 2 号音轨")?;
    if file
        .tracks
        .iter()
        .any(|t| t.track_id == 1 && !t.notes.is_empty())
    {
        return Err("当前 1 号音轨也有音符，迁移目标不唯一".into());
    }
    let source = old_nonempty[0];
    if source.notes.len() != target.notes.len() || source.notes.is_empty() {
        return Err("旧版与当前左手音符数量不同，请先生成相同变体再审阅".into());
    }
    // Compare note order and relative spacing. Tempo can change, but a changed
    // pattern/repetition cannot silently acquire old per-note annotations.
    let old_last = source.notes.last().unwrap().start.as_secs_f64();
    let new_last = target.notes.last().unwrap().start.as_secs_f64();
    for (a, b) in source.notes.iter().zip(target.notes.iter()) {
        let at = if old_last > 0. {
            a.start.as_secs_f64() / old_last
        } else {
            0.
        };
        let bt = if new_last > 0. {
            b.start.as_secs_f64() / new_last
        } else {
            0.
        };
        if a.note != b.note || b.channel != 1 || (at - bt).abs() > 0.0001 {
            return Err("旧版与当前左手音符的音高或位置不同，不能逐音迁移".into());
        }
    }
    let sidecar = library::load_song_sidecar(&active, content_id)
        .map_err(|e| format!("当前附加资料无法核对：{e}"))?;
    let source_count = sidecar
        .fingerings
        .iter()
        .filter(|n| n.track_id == 1)
        .count()
        + sidecar.hands.iter().filter(|n| n.track_id == 1).count()
        + sidecar
            .finger_actions
            .iter()
            .filter(|n| n.track_id == 1 || n.before_track == 1)
            .count()
        + usize::from(sidecar.track_parts.contains_key(&1));
    let target_count = sidecar
        .fingerings
        .iter()
        .filter(|n| n.track_id == 2)
        .count()
        + sidecar.hands.iter().filter(|n| n.track_id == 2).count()
        + sidecar
            .finger_actions
            .iter()
            .filter(|n| n.track_id == 2 || n.before_track == 2)
            .count()
        + usize::from(sidecar.track_parts.contains_key(&2));
    let settings = p.preferences.songs.get(content_id);
    let setting_tracks =
        settings.map_or(0, |s| s.tracks.iter().filter(|t| t.track_id == 1).count());
    let history_tracks = p
        .history
        .setup(content_id)
        .map_or(0, |s| s.tracks.iter().filter(|t| t.track_id == 1).count());
    let captured = match bytes(&capture_path(&p.data, content_id))? {
        value if value.is_empty() => None,
        value => {
            let saved: Captured = serde_json::from_slice(&value)
                .map_err(|_| "首次载入保留的旧练习条件无法读取，请检查原文件".to_string())?;
            if saved.content_id != content_id {
                return Err("首次载入保留的旧练习条件身份不符".into());
            }
            Some(saved)
        }
    };
    let condition_conflict = captured.as_ref().is_some_and(|saved| {
        saved.settings.as_ref().is_some_and(|s| {
            s.tracks.iter().any(|t| t.track_id == 1) && s.tracks.iter().any(|t| t.track_id == 2)
        }) || saved.setup.as_ref().is_some_and(|s| {
            s.tracks.iter().any(|t| t.track_id == 1) && s.tracks.iter().any(|t| t.track_id == 2)
        }) || saved
            .settings
            .as_ref()
            .and_then(|s| s.parts.get(&1))
            .is_some_and(|p| *p != neothesia_core::practice::PracticePart::LeftHand)
    });
    let mut warnings = Vec::new();
    if target_count != 0 && source_count != 0 {
        warnings.push("目标 2 号音轨已有个人资料，请先逐项整理；本次不会覆盖".into());
    }
    if sidecar
        .fingerings
        .iter()
        .any(|n| n.track_id == 1 && n.note_index >= target.notes.len())
        || sidecar
            .hands
            .iter()
            .any(|n| n.track_id == 1 && n.note_index >= target.notes.len())
        || sidecar.finger_actions.iter().any(|n| {
            (n.track_id == 1 && n.note_index >= target.notes.len())
                || (n.before_track == 1 && n.before_index >= target.notes.len())
        })
    {
        warnings.push("旧逐音资料含有越界音符，需人工核对原文件".into());
    }
    if sidecar
        .track_parts
        .get(&1)
        .is_some_and(|v| *v != neothesia_core::practice::PracticePart::LeftHand)
    {
        warnings.push("旧音轨的手别不是左手，不能自动迁移".into());
    }
    if settings.is_some_and(|s| {
        s.tracks.iter().any(|t| t.track_id == 2)
            && s.tracks.iter().any(|t| {
                t.track_id == 1
                    && t.mode != neothesia_core::practice_history::PracticeTrackMode::Auto
            })
    }) {
        warnings.push("保存的演奏条件同时包含 1、2 号音轨，请单独审阅".into());
    }
    let mut hasher = blake3::Hasher::new();
    for path in [
        &original,
        &active,
        &library::metadata_sidecar_path(&original),
        &library::metadata_sidecar_path(&active),
        &p.preferences_path,
        p.history.path(),
        &capture_path(&p.data, content_id),
    ] {
        let data = bytes(path)?;
        hasher.update(&(data.len() as u64).to_le_bytes());
        hasher.update(&data);
    }
    hasher.update(content_id.as_bytes());
    let fingerprint = hasher.finalize().to_hex().to_string();
    Ok(Review {
        fingerprint,
        active,
        original,
        sidecar,
        source_count,
        target_count,
        setting_tracks,
        history_tracks,
        warnings,
        captured,
        condition_conflict,
    })
}
pub(super) fn preview(p: &Player, content_id: &str) -> Result<Value, String> {
    let r = review(p, content_id)?;
    Ok(json!({"contentId":content_id,"fingerprint":r.fingerprint,
        "sourcePath":r.original,"activePath":r.active,"fromTrack":1,"toTrack":2,
        "sourceAnnotations":r.source_count,"targetAnnotations":r.target_count,
        "savedConditionTracks":r.setting_tracks,"recentHistoryTracks":r.history_tracks,
        "firstLoadSavedConditions":r.captured.as_ref().and_then(|c|c.settings.as_ref()).is_some(),
        "firstLoadHistorySetup":r.captured.as_ref().and_then(|c|c.setup.as_ref()).is_some(),
        "conditionConflict":r.condition_conflict,
        "warnings":r.warnings,"canApply":r.source_count > 0 && r.warnings.is_empty(),
        "historyPolicy":"已保存成绩保留原始编号和原始比较条件，不修改旧成绩"}))
}
fn shift_tracks(tracks: &mut [PracticeTrackSetup]) {
    for track in tracks {
        if track.track_id == 1 {
            track.track_id = 2;
        }
    }
}
fn shift_key<T>(values: &mut BTreeMap<usize, T>) {
    if !values.contains_key(&2) {
        if let Some(value) = values.remove(&1) {
            values.insert(2, value);
        }
    }
}
pub(super) fn apply(
    p: &mut Player,
    content_id: &str,
    expected: &str,
    restore_conditions: bool,
) -> Result<Value, String> {
    let r = review(p, content_id)?;
    if r.fingerprint != expected {
        return Err("预览后资料或练习文件已改变，请重新检查".into());
    }
    if r.source_count == 0 || !r.warnings.is_empty() {
        return Err("旧音轨资料有冲突或没有待迁移项目，未修改文件".into());
    }
    if restore_conditions && r.condition_conflict {
        return Err("旧练习条件含有冲突音轨或手别，请保留当前条件并单独审阅".into());
    }
    let backup_dir = p.data.join("legacy-track-reviews").join(&r.fingerprint);
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    let side_path = library::metadata_sidecar_path(&r.active);
    let before_side = bytes(&side_path)?;
    let before_prefs = bytes(&p.preferences_path)?;
    let before_history = bytes(p.history.path())?;
    for (name, original) in [
        ("sidecar.json", &before_side),
        ("preferences.json", &before_prefs),
        ("history.ron", &before_history),
    ] {
        let dest = backup_dir.join(name);
        if !dest.exists() {
            fs::write(&dest, original).map_err(|e| format!("原资料副本无法保存：{e}"))?;
        }
    }
    let mut sidecar = r.sidecar;
    for item in &mut sidecar.fingerings {
        if item.track_id == 1 {
            item.track_id = 2;
        }
    }
    for item in &mut sidecar.hands {
        if item.track_id == 1 {
            item.track_id = 2;
        }
    }
    for item in &mut sidecar.finger_actions {
        if item.track_id == 1 {
            item.track_id = 2;
        }
        if item.before_track == 1 {
            item.before_track = 2;
        }
    }
    shift_key(&mut sidecar.track_parts);
    let old_prefs = p.preferences.clone();
    let old_history = p.history.snapshot();
    if restore_conditions {
        if let Some(saved) = &r.captured {
            if let Some(settings) = &saved.settings {
                p.preferences
                    .songs
                    .insert(content_id.to_owned(), settings.clone());
            }
        }
    }
    if let Some(settings) = p.preferences.songs.get_mut(content_id) {
        if !settings.tracks.iter().any(|t| t.track_id == 2) {
            shift_tracks(&mut settings.tracks);
        }
        shift_key(&mut settings.parts);
    }
    if let Some(v) = p.preferences.track_appearances.get_mut(content_id) {
        shift_key(v);
    }
    if let Some(v) = p.preferences.track_sounds.get_mut(content_id) {
        shift_key(v);
    }
    if let Some(v) = p.preferences.fingering_profiles.get_mut(content_id) {
        shift_key(v);
    }
    let mut setup = if restore_conditions {
        r.captured
            .as_ref()
            .and_then(|c| c.setup.clone())
            .or_else(|| p.history.setup(content_id).cloned())
    } else {
        p.history.setup(content_id).cloned()
    };
    if let Some(value) = &mut setup {
        if !value.tracks.iter().any(|t| t.track_id == 2) {
            shift_tracks(&mut value.tracks);
        }
    }
    let result = (|| -> Result<(), String> {
        library::save_song_sidecar(&r.active, content_id, sidecar).map_err(|e| e.to_string())?;
        p.preferences.save(&p.preferences_path)?;
        if let Some(value) = setup {
            p.history
                .save_setup(content_id, &p.title, value)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        p.preferences = old_prefs;
        p.history.replace_snapshot(old_history);
        for (path, original) in [
            (side_path.as_path(), &before_side),
            (p.preferences_path.as_path(), &before_prefs),
            (p.history.path(), &before_history),
        ] {
            let _ = fs::write(path, original);
        }
        return Err(format!(
            "迁移未完成，已尝试恢复原文件；保留的副本位于 {}：{e}",
            backup_dir.display()
        ));
    }
    let spec = p.exercise.ok_or("技术练习已切换")?;
    let reopened = p.command(crate::Command::Generate { spec });
    Ok(
        json!({"status":"applied","contentId":content_id,"backupPath":backup_dir,
        "migratedAnnotations":r.source_count,"song":reopened.as_ref().ok(),
        "reopenWarning":reopened.err(),
        "historyPolicy":"原始成绩保持不变"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use neothesia_core::practice_history::{PracticeTrackMode, SongPracticeSetup};
    use neothesia_core::{
        exercise::{ExercisePlan, ExerciseSpec},
        library::{FingerHint, NoteHandHint},
        practice::PracticePart,
    };
    use std::io::Write;
    #[test]
    fn old_left_only_annotations_require_review_and_survive_reopen() {
        let root = std::env::temp_dir().join(format!(
            "neothesia-old-left-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("song-assets")).unwrap();
        let spec = ExerciseSpec {
            hands: ExerciseHands::Left,
            ..Default::default()
        };
        let plan = ExercisePlan::generate(spec, &crate::KeyboardRange::new(21..=108)).unwrap();
        let cid = plan.practice_id();
        let original = root.join("song-assets").join(format!("{cid}.mid"));
        let mut smf = plan.to_smf().unwrap();
        smf.tracks.remove(1);
        let mut old_bytes = Vec::new();
        smf.write_std(&mut old_bytes).unwrap();
        fs::File::create(&original)
            .unwrap()
            .write_all(&old_bytes)
            .unwrap();
        let mut old_side = SongSidecar::default();
        old_side.fingerings.push(FingerHint {
            track_id: 1,
            note_index: 0,
            finger: 4,
        });
        old_side.hands.push(NoteHandHint {
            track_id: 1,
            note_index: 0,
            part: PracticePart::LeftHand,
        });
        old_side.track_parts.insert(1, PracticePart::LeftHand);
        library::save_song_sidecar(&original, &cid, old_side.clone()).unwrap();
        let mut p = Player::new(root.clone(), PathBuf::new(), true);
        let old_track = PracticeTrackSetup {
            track_id: 1,
            mode: PracticeTrackMode::Human,
            visible: true,
        };
        let mut old_settings = crate::preferences::SessionSettings::default();
        old_settings.tracks = vec![old_track];
        old_settings.parts.insert(1, PracticePart::LeftHand);
        p.preferences.songs.insert(cid.clone(), old_settings);
        p.preferences.save(&p.preferences_path).unwrap();
        p.history
            .save_setup(
                &cid,
                "旧左手练习",
                SongPracticeSetup {
                    tracks: vec![old_track],
                    ..Default::default()
                },
            )
            .unwrap();
        p.command(crate::Command::Generate { spec }).unwrap();
        let initial = preview(&p, &cid).unwrap();
        assert_eq!(initial["canApply"], true);
        assert_eq!(initial["sourceAnnotations"], 3);
        assert_eq!(initial["firstLoadSavedConditions"], true);
        assert_eq!(initial["firstLoadHistorySetup"], true);
        let active = PathBuf::from(initial["activePath"].as_str().unwrap());
        let original_bytes = fs::read(&original).unwrap();
        let original_side = fs::read(library::metadata_sidecar_path(&original)).unwrap();
        let stale = initial["fingerprint"].as_str().unwrap().to_string();
        let mut updated = library::load_song_sidecar(&active, &cid).unwrap();
        updated.metadata.notes = Some("同时更新的备注".into());
        library::save_song_sidecar(&active, &cid, updated).unwrap();
        assert!(
            apply(&mut p, &cid, &stale, false)
                .unwrap_err()
                .contains("重新检查")
        );
        let fresh = preview(&p, &cid).unwrap()["fingerprint"]
            .as_str()
            .unwrap()
            .to_string();
        let result = apply(&mut p, &cid, &fresh, true).unwrap();
        assert_eq!(result["status"], "applied");
        assert_eq!(fs::read(&original).unwrap(), original_bytes);
        assert_eq!(
            fs::read(library::metadata_sidecar_path(&original)).unwrap(),
            original_side
        );
        let saved = library::load_song_sidecar(&active, &cid).unwrap();
        assert_eq!(saved.fingerings[0].track_id, 2);
        assert_eq!(saved.hands[0].track_id, 2);
        assert_eq!(saved.track_parts.get(&2), Some(&PracticePart::LeftHand));
        assert_eq!(saved.metadata.notes.as_deref(), Some("同时更新的备注"));
        assert!(
            p.preferences.songs[&cid]
                .tracks
                .iter()
                .any(|t| t.track_id == 2 && t.mode == PracticeTrackMode::Human)
        );
        assert!(
            p.history
                .setup(&cid)
                .unwrap()
                .tracks
                .iter()
                .any(|t| t.track_id == 2 && t.mode == PracticeTrackMode::Human)
        );
        let mut reopened = Player::new(root, PathBuf::new(), true);
        reopened.command(crate::Command::Generate { spec }).unwrap();
        assert!(
            reopened
                .saved_hints()
                .unwrap()
                .iter()
                .any(|h| h.track_id == 2 && h.finger == 4)
        );
        assert_eq!(preview(&reopened, &cid).unwrap()["canApply"], false);
    }
}
