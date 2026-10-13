//! User-reviewed, read-only equivalence for the historical left-only track 1.
//! Saved attempts remain byte-for-byte untouched; approvals live separately.
use crate::{Player, PracticeNote, practice_conditions};
use midi_file::MidiFile;
use neothesia_core::{
    exercise::ExerciseHands,
    library::{self, SongSidecar},
    practice_history::{PracticeSession, PracticeTrackMode, SongPracticeSetup},
    song_config::SongConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Approval {
    old_id: String,
    old_scope: String,
    new_scope: String,
    old_condition: String,
    source_hash: String,
    old_sidecar_hash: String,
    active_hash: String,
    active_sidecar_hash: String,
    source_path: PathBuf,
    active_path: PathBuf,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ledger {
    version: u8,
    content_id: String,
    approvals: Vec<Approval>,
}
fn ledger_path(data: &Path, cid: &str) -> PathBuf {
    data.join("legacy-track-reviews")
        .join(cid)
        .join("history-aliases.json")
}
fn valid_id(cid: &str) -> bool {
    cid.len() == 64 && cid.bytes().all(|b| b.is_ascii_hexdigit())
}
fn read_ledger(data: &Path, cid: &str) -> Result<Ledger, String> {
    if !valid_id(cid) {
        return Err("曲目身份无效".into());
    }
    let path = ledger_path(data, cid);
    let raw = match fs::read(&path) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Ledger {
                version: 1,
                content_id: cid.into(),
                approvals: vec![],
            });
        }
        Err(e) => return Err(e.to_string()),
    };
    let file: Ledger =
        serde_json::from_slice(&raw).map_err(|e| format!("旧成绩映射表无法读取：{e}"))?;
    if file.version != 1 || file.content_id != cid {
        return Err("旧成绩映射表版本或曲目身份无效".into());
    }
    Ok(file)
}
fn save_ledger(data: &Path, cid: &str, value: &Ledger) -> Result<(), String> {
    let path = ledger_path(data, cid);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let current = fs::read(&path).unwrap_or_default();
    if !current.is_empty() {
        let hash = blake3::hash(&current).to_hex().to_string();
        let backup = path.with_file_name(format!("history-aliases-{}.backup.json", &hash[..16]));
        if !backup.exists() {
            fs::write(&backup, &current).map_err(|e| format!("旧映射副本无法保存：{e}"))?;
        }
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let temp = path.with_extension("json.pending");
    fs::write(&temp, &bytes).map_err(|e| e.to_string())?;
    fs::rename(&temp, &path).map_err(|e| e.to_string())
}
fn digest(path: &Path) -> Result<String, String> {
    Ok(
        blake3::hash(&fs::read(path).map_err(|e| format!("{} 无法读取：{e}", path.display()))?)
            .to_hex()
            .to_string(),
    )
}
fn condition_digest(s: &PracticeSession) -> String {
    let mut copy = s.clone();
    copy.annotation = None;
    copy.replay_clips.clear();
    blake3::hash(&serde_json::to_vec(&copy).unwrap_or_default())
        .to_hex()
        .to_string()
}
fn file_scope(file: &MidiFile, sidecar: &SongSidecar, session: &PracticeSession) -> Option<String> {
    let context = session.context.as_ref()?;
    let mut config = SongConfig::new(&file.tracks);
    if !config.apply_practice_setup(&SongPracticeSetup {
        tracks: context.tracks.clone(),
        ..Default::default()
    }) {
        return None;
    }
    for track in &mut config.tracks {
        if let Some(part) = context.parts.get(&track.track_id) {
            track.practice_part = *part;
        }
    }
    let hands: HashMap<_, _> = sidecar
        .hands
        .iter()
        .map(|h| ((h.track_id, h.note_index), h.part))
        .collect();
    let mut notes: Vec<_> = file
        .tracks
        .iter()
        .filter(|t| !t.has_drums || t.has_other_than_drums)
        .flat_map(|t| {
            t.notes.iter().enumerate().map(|(index, n)| PracticeNote {
                inner: n.clone(),
                index,
            })
        })
        .collect();
    notes.sort_by_key(|n| (n.start, n.track_id, n.note));
    Some(practice_conditions::scope(
        Some(file),
        &notes,
        &config,
        &hands,
        &context.hands,
    ))
}
fn old_layout(s: &PracticeSession) -> bool {
    s.context.as_ref().is_some_and(|c| {
        c.exercise_spec
            .is_some_and(|x| x.hands == ExerciseHands::Left)
            && c.tracks
                .iter()
                .any(|t| t.track_id == 1 && t.mode == PracticeTrackMode::Human)
            && !c.tracks.iter().any(|t| t.track_id == 2)
    })
}
fn new_layout(s: &PracticeSession) -> bool {
    s.context.as_ref().is_some_and(|c| {
        c.exercise_spec
            .is_some_and(|x| x.hands == ExerciseHands::Left)
            && c.tracks
                .iter()
                .any(|t| t.track_id == 2 && t.mode == PracticeTrackMode::Human)
            && !c
                .tracks
                .iter()
                .any(|t| t.track_id == 1 && t.mode == PracticeTrackMode::Human)
    })
}
struct Evidence {
    old: MidiFile,
    active: MidiFile,
    old_sidecar: SongSidecar,
    active_sidecar: SongSidecar,
    source: PathBuf,
    active_path: PathBuf,
    source_hash: String,
    old_sidecar_hash: String,
    active_hash: String,
    active_sidecar_hash: String,
    active_spec: neothesia_core::exercise::ExerciseSpec,
}
fn evidence(p: &Player, cid: &str) -> Result<Evidence, String> {
    let file = p.file.as_ref().ok_or("请先打开单左手技术练习")?;
    if file.content_id != cid || p.exercise.is_none_or(|s| s.hands != ExerciseHands::Left) {
        return Err("当前不是指定的单左手技术练习".into());
    }
    let source = p.data.join("song-assets").join(format!("{cid}.mid"));
    let active_path = p.annotation_path(file)?;
    if source == active_path {
        return Err("没有找到旧版单左手 MIDI 原件".into());
    }
    let old = MidiFile::new(&source).map_err(|e| format!("旧 MIDI 无法读取：{e}"))?;
    let old_sidecar =
        library::load_song_sidecar(&source, cid).map_err(|e| format!("旧个人资料无法核对：{e}"))?;
    let active_sidecar = library::load_song_sidecar(&active_path, cid)
        .map_err(|e| format!("当前个人资料无法核对：{e}"))?;
    let old_notes: Vec<_> = old.tracks.iter().filter(|t| !t.notes.is_empty()).collect();
    let active_notes: Vec<_> = file.tracks.iter().filter(|t| !t.notes.is_empty()).collect();
    if old_notes.len() != 1
        || old_notes[0].track_id != 1
        || active_notes.len() != 1
        || active_notes[0].track_id != 2
        || old_notes[0].notes.len() != active_notes[0].notes.len()
        || old_notes[0].notes.is_empty()
        || old_notes[0]
            .notes
            .iter()
            .zip(active_notes[0].notes.iter())
            .any(|(a, b)| {
                a.note != b.note
                    || a.start != b.start
                    || a.end != b.end
                    || a.channel != 1
                    || b.channel != 1
            })
    {
        return Err("旧版与当前左手音轨的音符身份不一致".into());
    }
    Ok(Evidence {
        old,
        active: file.clone(),
        old_sidecar,
        active_sidecar,
        source_hash: digest(&source)?,
        old_sidecar_hash: digest(&library::metadata_sidecar_path(&source))?,
        active_hash: digest(&active_path)?,
        active_sidecar_hash: digest(&library::metadata_sidecar_path(&active_path))?,
        active_spec: p.exercise.unwrap(),
        source,
        active_path,
    })
}
fn pair_reason(
    e: &Evidence,
    old: &PracticeSession,
    current: &PracticeSession,
) -> Option<&'static str> {
    if !old_layout(old) || !new_layout(current) {
        return Some("旧记录或新记录的单左手音轨条件不完整");
    }
    if old.context.as_ref().and_then(|c| c.exercise_spec)
        != current.context.as_ref().and_then(|c| c.exercise_spec)
        || current.context.as_ref().and_then(|c| c.exercise_spec) != Some(e.active_spec)
    {
        return Some("两个成绩使用不同的技术练习变体");
    }
    if file_scope(&e.old, &e.old_sidecar, old).as_deref() != old.scope.as_deref() {
        return Some("旧成绩的音符范围无法由保留的旧 MIDI 和分手资料重算证明");
    }
    if file_scope(&e.active, &e.active_sidecar, current).as_deref() != current.scope.as_deref() {
        return Some("新成绩的音符范围已与当前 MIDI 或分手资料不同");
    }
    if !practice_conditions::same_reviewed_left_layout(old, current) {
        return Some("速度、音符覆盖或其他演奏条件不同");
    }
    None
}
fn fingerprint(p: &Player, cid: &str, e: &Evidence) -> Result<String, String> {
    let mut h = blake3::Hasher::new();
    for part in [
        &e.source_hash,
        &e.old_sidecar_hash,
        &e.active_hash,
        &e.active_sidecar_hash,
    ] {
        h.update(part.as_bytes());
    }
    h.update(&p.history.snapshot_bytes().map_err(|e| e.to_string())?);
    h.update(&fs::read(ledger_path(&p.data, cid)).unwrap_or_default());
    Ok(h.finalize().to_hex().to_string())
}
pub(super) fn preview(p: &Player, cid: &str) -> Result<Value, String> {
    let song = p.history.song(cid).ok_or("这首练习尚无保存的成绩")?;
    if song.sessions.is_empty(){return Err("这首练习尚无保存的成绩；请先完整练习并保存一轮".into());}
    let e = evidence(p, cid)?;
    let current = song
        .sessions
        .iter()
        .rev()
        .find(|s| {
            new_layout(s) && s.context.as_ref().and_then(|c| c.exercise_spec) == Some(e.active_spec)
        })
        .ok_or("请先保存一条当前版本的单左手成绩，再核对旧成绩")?;
    let ledger = read_ledger(&p.data, cid)?;
    let rows = song
        .sessions
        .iter()
        .filter(|s| old_layout(s))
        .map(|old| {
            let reason = pair_reason(&e, old, current);
            let approved = ledger
                .approvals
                .iter()
                .any(|a| approval_matches(&e, a, old, current));
            json!({"oldId":old.stable_id(),"recordedAt":old.recorded_at_unix_ms,
            "oldScope":old.scope,"currentId":current.stable_id(),"currentScope":current.scope,
            "oldAccuracy":old.summary.overall.accuracy(),
            "currentAccuracy":current.summary.overall.accuracy(),
            "noteCount":e.old.tracks.iter().map(|t|t.notes.len()).sum::<usize>(),
            "eligible":reason.is_none(),"reason":reason,"approved":approved})
        })
        .collect::<Vec<_>>();
    Ok(json!({"contentId":cid,"fingerprint":fingerprint(p,cid,&e)?,
        "sourcePath":e.source,"activePath":e.active_path,"currentId":current.stable_id(),
        "rows":rows,"policy":"旧成绩原文不变；只将已核验并确认的记录用于同条件比较。条件变化或原资料变化后，映射自动失效。"}))
}
fn approval_matches(
    e: &Evidence,
    a: &Approval,
    old: &PracticeSession,
    current: &PracticeSession,
) -> bool {
    a.old_id == old.stable_id()
        && a.old_scope == old.scope.as_deref().unwrap_or("")
        && a.new_scope == current.scope.as_deref().unwrap_or("")
        && a.old_condition == condition_digest(old)
        && a.source_hash == e.source_hash
        && a.old_sidecar_hash == e.old_sidecar_hash
        && a.active_hash == e.active_hash
        && a.active_sidecar_hash == e.active_sidecar_hash
        && a.source_path == e.source
        && a.active_path == e.active_path
        && pair_reason(e, old, current).is_none()
}
pub(super) fn update(
    p: &Player,
    cid: &str,
    old_id: &str,
    current_id: &str,
    expected: &str,
    approve: bool,
) -> Result<Value, String> {
    let e = evidence(p, cid)?;
    if fingerprint(p, cid, &e)? != expected {
        return Err("预览后成绩或文件已变化，请重新检查".into());
    }
    let song = p.history.song(cid).ok_or("曲目成绩不存在")?;
    let old = song
        .sessions
        .iter()
        .find(|s| s.stable_id() == old_id)
        .ok_or("旧成绩已变化")?;
    let current = song
        .sessions
        .iter()
        .find(|s| s.stable_id() == current_id)
        .ok_or("当前成绩已变化")?;
    if !old_layout(old) || !new_layout(current) {
        return Err("成绩音轨身份不符合旧版单左手映射".into());
    }
    let mut ledger = read_ledger(&p.data, cid)?;
    if approve {
        if let Some(reason) = pair_reason(&e, old, current) {
            return Err(reason.into());
        }
        ledger.approvals.retain(|a| a.old_id != old_id);
        ledger.approvals.push(Approval {
            old_id: old_id.into(),
            old_scope: old.scope.clone().unwrap(),
            new_scope: current.scope.clone().unwrap(),
            old_condition: condition_digest(old),
            source_hash: e.source_hash,
            old_sidecar_hash: e.old_sidecar_hash,
            active_hash: e.active_hash,
            active_sidecar_hash: e.active_sidecar_hash,
            source_path: e.source,
            active_path: e.active_path,
        });
    } else {
        ledger.approvals.retain(|a| a.old_id != old_id);
    }
    save_ledger(&p.data, cid, &ledger)?;
    preview(p, cid)
}
pub(super) fn active_approvals(data: &Path, cid: &str) -> Vec<Approval> {
    if !valid_id(cid) {
        return vec![];
    }
    let Ok(ledger) = read_ledger(data, cid) else {
        return vec![];
    };
    ledger
        .approvals
        .into_iter()
        .filter(|a| {
            a.source_path.starts_with(data)
                && a.active_path.starts_with(data)
                && digest(&a.source_path).ok().as_deref() == Some(&a.source_hash)
                && digest(&library::metadata_sidecar_path(&a.source_path))
                    .ok()
                    .as_deref()
                    == Some(&a.old_sidecar_hash)
                && digest(&a.active_path).ok().as_deref() == Some(&a.active_hash)
                && digest(&library::metadata_sidecar_path(&a.active_path))
                    .ok()
                    .as_deref()
                    == Some(&a.active_sidecar_hash)
        })
        .collect()
}
pub(super) fn same_reviewed(
    approvals: &[Approval],
    a: &PracticeSession,
    b: &PracticeSession,
) -> bool {
    if practice_conditions::same_attempt_conditions(a, b) {
        return true;
    }
    let (old, current) = if old_layout(a) && new_layout(b) {
        (a, b)
    } else if old_layout(b) && new_layout(a) {
        (b, a)
    } else {
        return false;
    };
    approvals.iter().any(|approval| {
        approval.old_id == old.stable_id()
            && approval.old_scope == old.scope.as_deref().unwrap_or("")
            && approval.new_scope == current.scope.as_deref().unwrap_or("")
            && approval.old_condition == condition_digest(old)
            && practice_conditions::same_reviewed_left_layout(old, current)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, KeyboardRange};
    use neothesia_core::{
        exercise::{ExercisePlan, ExerciseSpec},
        practice::{AttemptSummary, PracticeHands, PracticePart},
        practice_history::{PracticeSessionContext, PracticeSessionKind},
    };
    use std::{collections::BTreeMap, io::Write};
    fn session(
        file: &MidiFile,
        sidecar: &SongSidecar,
        spec: ExerciseSpec,
        id: &str,
        track: usize,
    ) -> PracticeSession {
        let mut parts = BTreeMap::new();
        parts.insert(0, PracticePart::Other);
        parts.insert(track, PracticePart::LeftHand);
        if track == 2 {
            parts.insert(1, PracticePart::RightHand);
        }
        let mut value = PracticeSession::new(
            PracticeSessionKind::WholeSong,
            PracticeHands::Both,
            1.,
            AttemptSummary::default(),
        );
        value.id = Some(id.into());
        value.mode = Some("flow".into());
        value.effective_tempo_bpm = Some(spec.tempo_bpm);
        value.context = Some(PracticeSessionContext {
            routine: None,
            ladder: None,
            target_notes: file.tracks.iter().map(|t| t.notes.len()).sum(),
            judged_notes: file.tracks.iter().map(|t| t.notes.len()).sum(),
            tracks: SongConfig::new(&file.tracks).practice_track_setup(),
            parts,
            hands: "both".into(),
            count_in: 1,
            metronome: false,
            latency: 0,
            pedal_latency_ms: None,
            rounds: 0,
            adaptive: false,
            grid: practice_conditions::grid(file),
            source_path: None,
            exercise_spec: Some(spec),
        });
        value.scope = file_scope(file, sidecar, &value);
        value
    }
    #[test]
    fn reviewed_alias_compares_proven_history_without_editing_attempts_and_revokes() {
        let root = std::env::temp_dir().join(format!(
            "legacy-history-{}",
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
        let plan = ExercisePlan::generate(spec, &KeyboardRange::new(21..=108)).unwrap();
        let cid = plan.practice_id();
        let source = root.join("song-assets").join(format!("{cid}.mid"));
        let mut smf = plan.to_smf().unwrap();
        smf.tracks.remove(1);
        let mut bytes = vec![];
        smf.write_std(&mut bytes).unwrap();
        fs::File::create(&source)
            .unwrap()
            .write_all(&bytes)
            .unwrap();
        library::save_song_sidecar(&source, &cid, SongSidecar::default()).unwrap();
        let mut p = Player::new(root.clone(), PathBuf::new(), true);
        p.command(Command::Generate { spec }).unwrap();
        let e = evidence(&p, &cid).unwrap();
        let old = session(&e.old, &e.old_sidecar, spec, "old-left", 1);
        let current = session(&e.active, &e.active_sidecar, spec, "new-left", 2);
        assert_ne!(old.scope, current.scope);
        assert!(practice_conditions::same_reviewed_left_layout(
            &old, &current
        ));
        p.history
            .record_session(&cid, "左手练习", old.clone())
            .unwrap();
        p.history
            .record_session(&cid, "左手练习", current.clone())
            .unwrap();
        let history_before = p.history.snapshot_bytes().unwrap();
        let candidate = preview(&p, &cid).unwrap();
        assert_eq!(candidate["rows"][0]["eligible"], true);
        assert!(
            update(&p, &cid, "old-left", "new-left", "outdated", true)
                .unwrap_err()
                .contains("重新检查")
        );
        let mut slower = current.clone();
        slower.speed = 0.8;
        assert_eq!(
            pair_reason(&e, &old, &slower),
            Some("速度、音符覆盖或其他演奏条件不同")
        );
        assert_eq!(
            p.history_detail(&cid, "new-left").unwrap()["comparison"]["count"],
            0
        );
        let token = candidate["fingerprint"].as_str().unwrap();
        update(&p, &cid, "old-left", "new-left", token, true).unwrap();
        let detail = p.history_detail(&cid, "new-left").unwrap();
        assert_eq!(detail["comparison"]["count"], 1);
        assert_eq!(detail["comparison"]["reviewedLegacyCount"], 1);
        assert_eq!(p.history.snapshot_bytes().unwrap(), history_before);
        let mut reopened = Player::new(root.clone(), PathBuf::new(), true);
        assert_eq!(
            reopened.history_detail(&cid, "new-left").unwrap()["comparison"]["count"],
            1
        );
        reopened.command(Command::Generate { spec }).unwrap();
        assert_eq!(
            reopened.history_detail(&cid, "new-left").unwrap()["comparison"]["count"],
            1
        );
        let after = preview(&reopened, &cid).unwrap();
        update(
            &reopened,
            &cid,
            "old-left",
            "new-left",
            after["fingerprint"].as_str().unwrap(),
            false,
        )
        .unwrap();
        assert_eq!(
            reopened.history_detail(&cid, "new-left").unwrap()["comparison"]["count"],
            0
        );
        let fresh = preview(&reopened, &cid).unwrap();
        update(
            &reopened,
            &cid,
            "old-left",
            "new-left",
            fresh["fingerprint"].as_str().unwrap(),
            true,
        )
        .unwrap();
        assert_eq!(
            reopened.history_detail(&cid, "new-left").unwrap()["comparison"]["count"],
            1
        );
        let current_side = library::metadata_sidecar_path(&e.active_path);
        let mut changed = library::load_song_sidecar(&e.active_path, &cid).unwrap();
        changed.metadata.notes = Some("后来改动的资料".into());
        library::save_song_sidecar(&e.active_path, &cid, changed).unwrap();
        assert_eq!(
            reopened.history_detail(&cid, "new-left").unwrap()["comparison"]["count"],
            0
        );
        assert!(current_side.is_file());
        let mut altered = old;
        altered.scope = Some("practice-v1-invalid".into());
        assert_eq!(
            pair_reason(&e, &altered, &current),
            Some("旧成绩的音符范围无法由保留的旧 MIDI 和分手资料重算证明")
        );
    }
}
