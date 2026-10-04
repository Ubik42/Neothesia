//! Local original-file links are deliberately separate from portable score copies.
use crate::{
    library_scores, library_workspace,
    notation::ScoreAsset,
    preferences::{Preferences, ScoreVersion},
    *,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub content_id: String,
    pub version_id: String,
    pub midi_path: PathBuf,
    pub path: PathBuf,
    pub baseline: String,
    pub default_bpm: u16,
    #[serde(default)]
    pub acknowledged: Option<String>,
}
#[derive(Default, Deserialize, Serialize)]
struct Store {
    entries: BTreeMap<String, Source>,
}
fn load(data: &Path) -> Result<Store, String> {
    match std::fs::File::open(data.join("score-sources.json")) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(16_000_001)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 16_000_000 {
                return Err("原谱关联资料过大".into());
            }
            let store: Store =
                serde_json::from_slice(&bytes).map_err(|e| format!("原谱关联资料无法读取：{e}"))?;
            if store.entries.len() > 10_000 {
                return Err("原谱关联数量超过上限".into());
            }
            Ok(store)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Store::default()),
        Err(e) => Err(e.to_string()),
    }
}
fn save(data: &Path, store: &Store) -> Result<(), String> {
    std::fs::create_dir_all(data).map_err(|e| e.to_string())?;
    let path = data.join("score-sources.json");
    let tmp = data.join("score-sources.json.tmp");
    let bytes = serde_json::to_vec_pretty(store).map_err(|e| e.to_string())?;
    if bytes.len() > 16_000_000 {
        return Err("原谱关联资料达到容量上限，请先整理关联".into());
    }
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}
pub(super) fn read_original(path: &Path) -> Result<Vec<u8>, String> {
    if !path.is_absolute() {
        return Err("请选择原谱文件的完整位置".into());
    }
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    if !["xml", "musicxml", "mxl"].contains(&ext.as_str()) {
        return Err("原谱关联支持 MusicXML/XML/MXL".into());
    }
    let file = std::fs::File::open(path).map_err(|e| format!("原谱无法读取：{e}"))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("原谱位置不是文件".into());
    }
    let mut bytes = Vec::new();
    file.take(4_000_001)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > 4_000_000 {
        return Err("原谱文件为空或超过 4 MB".into());
    }
    Ok(bytes)
}
fn version<'a>(prefs: &'a Preferences, source: &Source) -> Option<&'a ScoreVersion> {
    prefs
        .score_versions
        .get(&source.content_id)?
        .iter()
        .find(|v| v.id == source.version_id && v.content_id == source.baseline)
}
fn fingerprint(source: &Source, hash: &str) -> String {
    let mut h = blake3::Hasher::new();
    h.update(&serde_json::to_vec(source).unwrap());
    h.update(hash.as_bytes());
    h.finalize().to_hex().to_string()
}
pub(super) fn register(
    data: &Path,
    cid: &str,
    midi: &Path,
    version_id: &str,
    path: &Path,
    bpm: u16,
) -> Result<Value, String> {
    if !(20..=300).contains(&bpm) {
        return Err("缺省速度需为 20 至 300 BPM".into());
    }
    let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    let bytes = read_original(&path)?;
    let hash = blake3::hash(&bytes).to_hex().to_string();
    let prefs = Preferences::load(&data.join("web-preferences.json"))?;
    let v = prefs
        .score_versions
        .get(cid)
        .and_then(|v| v.iter().find(|v| v.id == version_id))
        .ok_or("谱面版本已移除，请刷新列表")?;
    if hash != v.content_id {
        return Err("原文件与保存版本不同；请先关联原来的文件，或把新内容添加为新版本".into());
    }
    let id = blake3::hash(format!("{cid}\n{version_id}").as_bytes())
        .to_hex()
        .to_string();
    let lock = library_workspace::lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "原谱关联资料正忙")?;
    let mut store = load(data)?;
    store.entries.retain(|_, s| version(&prefs, s).is_some());
    if store.entries.len() >= 10_000 && !store.entries.contains_key(&id) {
        return Err("已关联 10000 份原谱，请先整理".into());
    }
    store.entries.insert(
        id.clone(),
        Source {
            id: id.clone(),
            content_id: cid.into(),
            version_id: version_id.into(),
            midi_path: midi.into(),
            path,
            baseline: hash,
            default_bpm: bpm,
            acknowledged: None,
        },
    );
    save(data, &store)?;
    Ok(json!({"id":id,"linked":true}))
}
fn checked(data: &Path, id: &str, expected: &str) -> Result<(Source, Vec<u8>), String> {
    let source = load(data)?
        .entries
        .remove(id)
        .ok_or("原谱关联已取消，请刷新列表")?;
    let prefs = Preferences::load(&data.join("web-preferences.json"))?;
    version(&prefs, &source).ok_or("保存版本已经改变或移除，请刷新列表")?;
    let bytes = read_original(&source.path)?;
    let hash = blake3::hash(&bytes).to_hex().to_string();
    if fingerprint(&source, &hash) != expected {
        return Err("原文件或关联已经改变，请重新检查并预览".into());
    }
    Ok((source, bytes))
}
pub fn inspect(data: &Path, cid: Option<&str>) -> Result<Value, String> {
    let store = load(data)?;
    let prefs = Preferences::load(&data.join("web-preferences.json"))?;
    let mut rows = Vec::new();
    for source in store
        .entries
        .values()
        .filter(|s| cid.is_none_or(|cid| s.content_id == cid))
    {
        let Some(v) = version(&prefs, source) else {
            continue;
        };
        let read = read_original(&source.path);
        let (status, hash, error) = match read {
            Ok(bytes) => {
                let hash = blake3::hash(&bytes).to_hex().to_string();
                (
                    if hash == source.baseline {
                        "ready"
                    } else if source.acknowledged.as_ref() == Some(&hash) {
                        "acknowledged"
                    } else {
                        "changed"
                    },
                    Some(hash),
                    None,
                )
            }
            Err(e) => (
                if source.path.exists() {
                    "unreadable"
                } else {
                    "missing"
                },
                None,
                Some(e),
            ),
        };
        let song_title =
            neothesia_core::library::load_song_sidecar(&source.midi_path, &source.content_id)
                .ok()
                .and_then(|s| s.metadata.title)
                .unwrap_or_else(|| v.name.clone());
        let active =
            neothesia_core::library::load_song_sidecar(&source.midi_path, &source.content_id)
                .ok()
                .and_then(|s| s.score)
                .is_some_and(|a| {
                    neothesia_core::library::resolve_score_path(&source.midi_path, &a) == v.path
                });
        rows.push(json!({"id":source.id,"contentId":source.content_id,"versionId":source.version_id,"name":v.name,"active":active,"songTitle":song_title,"midiPath":source.midi_path,"path":source.path,"status":status,"pending":status=="changed","defaultBpm":source.default_bpm,"fingerprint":hash.as_ref().map(|h|fingerprint(source,h)),"error":error}));
    }
    rows.sort_by_key(|r| {
        (
            !r["pending"].as_bool().unwrap_or(false),
            r["songTitle"].as_str().unwrap_or("").to_string(),
            r["name"].as_str().unwrap_or("").to_string(),
            !r["active"].as_bool().unwrap_or(false),
        )
    });
    Ok(json!({"entries":rows}))
}
pub fn preview(data: &Path, id: &str, expected: &str) -> Result<Value, String> {
    let (source, bytes) = checked(data, id, expected)?;
    let score =
        neothesia_core::musicxml::import_musicxml_document(&bytes).map_err(|e| e.to_string())?;
    let name = source
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let generated = neothesia_core::score_performance::generate(&score, source.default_bpm)
        .and_then(|performance| {
            let file = MidiFile::from_smf(&name, &performance.smf)?;
            let notes = file.tracks.iter().map(|t| t.notes.len()).sum::<usize>();
            if notes == 0 {
                return Err("此原谱没有可生成的演奏音符，可保存为谱面版本".into());
            }
            Ok((file, notes, performance.warnings))
        });
    let (_, old_file) = library_scores::location(data, &source.midi_path, &source.content_id)?;
    let pairing = old_file
        .as_ref()
        .map(|f| ScoreAsset::new(bytes.clone(), name.clone(), f).map(|a|{let summary=neothesia_core::score_alignment::summarize_alignment(&a.alignment);json!({"coverage":summary.coverage_percent,"readiness":summary.readiness,"diagnostics":summary.navigation_diagnostics})}))
        .transpose()?;
    let (new_cid, notes, duration, same, warnings, error) = match generated {
        Ok((file, notes, warnings)) => (
            Some(file.content_id.clone()),
            Some(notes),
            Some(file.duration.as_secs_f64()),
            Some(file.content_id == source.content_id),
            warnings,
            None,
        ),
        Err(error) => (None, None, None, None, Vec::new(), Some(error)),
    };
    let written = score
        .parts
        .iter()
        .flat_map(|p| &p.measures)
        .flat_map(|m| &m.events)
        .filter(|e| matches!(e,neothesia_core::musicxml::ScoreEvent::Note(n) if n.pitch.is_some()))
        .count();
    Ok(
        json!({"id":id,"fingerprint":expected,"name":name,"title":score.title,"newContentId":new_cid,"performanceReady":error.is_none(),"performanceError":error,"writtenNotes":written,"samePerformance":same,"notes":notes,"duration":duration,"oldAvailable":old_file.is_some(),"pairing":pairing.map(|p|json!({"coverage":p["coverage"],"readiness":p["readiness"],"diagnostics":p["diagnostics"]})),"warnings":warnings}),
    )
}
pub(super) fn acknowledge(data: &Path, id: &str, expected: &str) -> Result<Value, String> {
    let (source, bytes) = checked(data, id, expected)?;
    let hash = blake3::hash(&bytes).to_hex().to_string();
    let lock = library_workspace::lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "原谱关联资料正忙")?;
    let mut store = load(data)?;
    let current = store.entries.get_mut(id).ok_or("原谱关联已取消")?;
    if fingerprint(current, &hash) != expected {
        return Err("关联已改变，请刷新列表".into());
    }
    if current.path != source.path {
        return Err("原谱位置已改变".into());
    }
    current.acknowledged = Some(hash);
    save(data, &store)?;
    Ok(json!({"kept":true}))
}
pub(super) fn unlink(data: &Path, id: &str, path: &Path) -> Result<Value, String> {
    let lock = library_workspace::lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "原谱关联资料正忙")?;
    let mut store = load(data)?;
    if store.entries.get(id).is_some_and(|s| s.path != path) {
        return Err("原谱关联已改变，请刷新列表后操作".into());
    }
    store.entries.remove(id);
    save(data, &store)?;
    Ok(json!({"unlinked":true}))
}

impl Player {
    pub(super) fn link_score_source(
        &mut self,
        midi: PathBuf,
        cid: String,
        version_id: String,
        path: PathBuf,
        bpm: u16,
    ) -> Result<Value, String> {
        let midi = if midi.as_os_str().is_empty() {
            let file = self.file.as_ref().filter(|file| file.content_id == cid)
                .ok_or("请先打开需要关联原谱的练习曲")?;
            self.annotation_path(file)?
        } else {
            midi
        };
        let (midi, file) = library_scores::location(&self.data, &midi, &cid)?;
        let bpm = if bpm == 0 {
            file.as_ref()
                .map(|f| f.tempo_track.bpm_at_seconds(0.).round().clamp(20., 300.) as u16)
                .unwrap_or(120)
        } else {
            bpm
        };
        if !(20..=300).contains(&bpm) {
            return Err("缺省速度需为 20 至 300 BPM".into());
        }
        let (mut versions, active) = library_scores::versions(&self.preferences, &midi, &cid)?;
        let v = versions
            .iter()
            .find(|v| v.id == version_id)
            .ok_or("谱面版本不存在")?
            .clone();
        let bytes = read_original(&path)?;
        if blake3::hash(&bytes).to_hex().as_str() != v.content_id {
            return Err("所选原文件与此保存版本不同，未建立关联".into());
        }
        if std::fs::canonicalize(&v.path).ok() == std::fs::canonicalize(&path).ok() {
            let directory = self.data.join("scores");
            std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            let snapshot = directory.join(format!(
                "source-snapshot-{}.{}",
                std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_nanos(),
                if bytes.starts_with(b"PK") {
                    "mxl"
                } else {
                    "musicxml"
                }
            ));
            std::fs::write(&snapshot, &bytes).map_err(|e| e.to_string())?;
            versions
                .iter_mut()
                .find(|row| row.id == version_id)
                .ok_or("谱面版本不存在")?
                .path = snapshot.clone();
            if active.as_ref() == Some(&v.path) {
                neothesia_core::library::save_score_association(&midi, &cid, &snapshot)
                    .map_err(|e| e.to_string())?;
            }
            self.preferences
                .score_versions
                .insert(cid.clone(), versions);
            self.preferences.save(&self.preferences_path)?;
        }
        // Also migrates legacy external associations to an owned immutable copy.
        self.add_library_notation(
            midi.clone(),
            cid.clone(),
            v.name.clone(),
            bytes,
            Some(version_id.clone()),
        )?;
        register(&self.data, &cid, &midi, &version_id, &path, bpm)
    }
    pub(super) fn import_score_original(
        &mut self,
        path: PathBuf,
        bpm: u16,
    ) -> Result<Value, String> {
        let bytes = read_original(&path)?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let result = self.command(Command::ImportScore {
            bytes,
            name: name.clone(),
            default_bpm: bpm,
        })?;
        let cid = result["contentId"].as_str().ok_or("导入曲目身份缺失")?;
        let midi = PathBuf::from(result["sourcePath"].as_str().ok_or("导入曲目位置缺失")?);
        let version = self
            .preferences
            .score_versions
            .get(cid)
            .and_then(|v| {
                v.iter()
                    .rev()
                    .find(|v| v.content_id == hash && v.name == name)
            })
            .ok_or("导入谱面版本缺失")?
            .id
            .clone();
        let linked = self.link_score_source(midi.clone(), cid.to_string(), version, path, bpm);
        let mut result = result;
        if let Err(e) = linked {
            result["importWarnings"] = json!(vec![format!("曲目已导入，原文件关联未完成：{e}")]);
        }
        Ok(result)
    }
    pub(super) fn apply_score_source(
        &mut self,
        id: String,
        expected: String,
        mode: String,
    ) -> Result<Value, String> {
        if !["notation", "performance"].contains(&mode.as_str()) {
            return Err("请选择更新谱面或建立演奏曲目".into());
        }
        if self.recorder.is_recording()
            || self.state.status == "playing"
            || self.routine.is_some()
            || self.ladder.as_ref().is_some_and(|r| r.active)
        {
            return Err("请先停止演奏、录音、练习计划和速度阶梯，再应用原谱更新".into());
        }
        let (source, bytes) = checked(&self.data, &id, &expected)?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let name = source
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let (cid, midi, version, result) = if mode == "performance" {
            let result = self.command(Command::ImportScore {
                bytes,
                name: name.clone(),
                default_bpm: source.default_bpm,
            })?;
            let cid = result["contentId"]
                .as_str()
                .ok_or("曲目身份缺失")?
                .to_string();
            let midi = PathBuf::from(result["sourcePath"].as_str().ok_or("曲目位置缺失")?);
            let v = self
                .preferences
                .score_versions
                .get(&cid)
                .and_then(|v| {
                    v.iter()
                        .rev()
                        .find(|v| v.content_id == hash && v.name == name)
                })
                .ok_or("新谱面版本缺失")?
                .id
                .clone();
            (cid, midi, v, Some(result))
        } else {
            let score = neothesia_core::musicxml::import_musicxml_document(&bytes)
                .map_err(|e| e.to_string())?;
            neothesia_core::score_performance::normalized(&score)?;
            let result = self.add_library_notation(
                source.midi_path.clone(),
                source.content_id.clone(),
                name,
                bytes,
                None,
            )?;
            (
                source.content_id.clone(),
                source.midi_path.clone(),
                result["id"].as_str().ok_or("新谱面版本缺失")?.into(),
                None,
            )
        };
        // Recheck the source on link: mid-save edits cannot be acknowledged as applied.
        let linked = self.link_score_source(
            midi.clone(),
            cid.clone(),
            version.clone(),
            source.path.clone(),
            source.default_bpm,
        );
        let mut warnings = Vec::new();
        if let Err(e) = linked {
            warnings.push(format!("更新已保存，原文件关联未完成：{e}"));
        }
        if let Err(e) = acknowledge(&self.data, &id, &expected) {
            warnings.push(format!("原文件再次变化，请重新检查：{e}"));
        }
        Ok(
            json!({"contentId":cid,"versionId":version,"mode":mode,"song":result,"warnings":warnings}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linking_a_saved_copy_keeps_an_independent_snapshot_and_infers_tempo() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work")
            .join(format!(
                "source-copy-test-{}",
                std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        std::fs::create_dir_all(&data).unwrap();
        let original = data.join("original.musicxml");
        let first = xml("C", "");
        std::fs::write(&original, &first).unwrap();
        let mut player = Player::new(
            data.clone(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../default.sf2"),
            true,
        );
        let song = player
            .command(Command::ImportScoreOriginal {
                path: original,
                default_bpm: 60,
            })
            .unwrap();
        let cid = song["contentId"].as_str().unwrap().to_string();
        let version = player.preferences.score_versions[&cid][0].clone();
        player
            .command(Command::LinkScoreSource {
                midi_path: PathBuf::new(),
                content_id: cid.clone(),
                version_id: version.id.clone(),
                path: version.path.clone(),
                default_bpm: 0,
            })
            .unwrap();
        let snapshot = player.preferences.score_versions[&cid][0].path.clone();
        assert_ne!(snapshot, version.path);
        let row = inspect(&data, Some(&cid)).unwrap()["entries"][0].clone();
        assert_eq!(row["defaultBpm"], 60);
        assert!(
            unlink(
                &data,
                row["id"].as_str().unwrap(),
                Path::new("wrong-original.musicxml")
            )
            .is_err()
        );
        std::fs::write(&version.path, xml("G", "")).unwrap();
        assert_eq!(std::fs::read(snapshot).unwrap(), first);
        assert_eq!(
            inspect(&data, Some(&cid)).unwrap()["entries"][0]["status"],
            "changed"
        );
        std::fs::write(
            &version.path,
            xml("G", r#"<direction><sound dacapo="yes"/></direction>"#),
        )
        .unwrap();
        let row = inspect(&data, Some(&cid)).unwrap()["entries"][0].clone();
        let preview = preview(
            &data,
            row["id"].as_str().unwrap(),
            row["fingerprint"].as_str().unwrap(),
        )
        .unwrap();
        assert_eq!(preview["performanceReady"], false);
        assert!(!preview["performanceError"].as_str().unwrap().is_empty());
        player
            .command(Command::ApplyScoreSource {
                id: row["id"].as_str().unwrap().into(),
                fingerprint: row["fingerprint"].as_str().unwrap().into(),
                mode: "notation".into(),
            })
            .unwrap();
        assert_eq!(player.file.as_ref().unwrap().content_id, cid);
    }
    fn xml(step: &str, extra: &str) -> Vec<u8> {
        format!(r#"<score-partwise version="4.0"><work><work-title>原谱课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>{step}</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note>{extra}</measure></part></score-partwise>"#).into_bytes()
    }
    #[test]
    fn original_updates_preserve_owned_versions_and_piece_identity() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work")
            .join(format!(
                "source-test-{}",
                std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        std::fs::create_dir_all(&data).unwrap();
        let original = data.join("original.musicxml");
        let first = xml("C", "");
        std::fs::write(&original, &first).unwrap();
        let mut player = Player::new(
            data.clone(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../default.sf2"),
            true,
        );
        let song = player
            .command(Command::ImportScoreOriginal {
                path: original.clone(),
                default_bpm: 60,
            })
            .unwrap();
        let cid = song["contentId"].as_str().unwrap().to_string();
        let midi = PathBuf::from(song["sourcePath"].as_str().unwrap());
        let track = song["notes"][0]["track"].as_u64().unwrap() as usize;
        player
            .command(Command::EditFingersFor {
                content_id: cid.clone(),
                edits: vec![fingerings::Edit {
                    track_id: track,
                    note_index: 0,
                    finger: Some(2),
                }],
            })
            .unwrap();
        let row = inspect(&data, Some(&cid)).unwrap()["entries"][0].clone();
        assert_eq!(row["status"], "ready");
        let initial_copy = player.preferences.score_versions[&cid][0].path.clone();
        assert_ne!(initial_copy, original);
        let mut scanner = Scanner::default();
        assert_eq!(scanner.scan(&data).unwrap().0, 0);
        std::fs::write(&original, xml("C", "<!-- engraving changed -->")).unwrap();
        assert_eq!(scanner.scan(&data).unwrap().0, 1);
        let changed = inspect(&data, Some(&cid)).unwrap()["entries"][0].clone();
        let id = changed["id"].as_str().unwrap().to_string();
        let token = changed["fingerprint"].as_str().unwrap().to_string();
        assert!(
            preview(&data, &id, &token).unwrap()["samePerformance"]
                .as_bool()
                .unwrap()
        );
        player
            .command(Command::ApplyScoreSource {
                id: id.clone(),
                fingerprint: token,
                mode: "notation".into(),
            })
            .unwrap();
        assert_eq!(std::fs::read(&initial_copy).unwrap(), first);
        assert_eq!(player.file.as_ref().unwrap().content_id, cid);
        assert_eq!(player.preferences.score_versions[&cid].len(), 2);
        let entries = inspect(&data, Some(&cid)).unwrap();
        assert!(
            entries["entries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["status"] == "acknowledged")
        );
        let new_source = entries["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["status"] == "ready")
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        std::fs::write(&original, xml("D", "")).unwrap();
        let row = inspect(&data, Some(&cid)).unwrap()["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == new_source)
            .unwrap()
            .clone();
        let token = row["fingerprint"].as_str().unwrap().to_string();
        assert!(
            !preview(&data, &new_source, &token).unwrap()["samePerformance"]
                .as_bool()
                .unwrap()
        );
        std::fs::write(&original, xml("E", "")).unwrap();
        assert!(
            player
                .command(Command::ApplyScoreSource {
                    id: new_source.clone(),
                    fingerprint: token,
                    mode: "performance".into()
                })
                .is_err()
        );
        assert_eq!(player.file.as_ref().unwrap().content_id, cid);
        std::fs::write(&original, xml("D", "")).unwrap();
        let row = inspect(&data, Some(&cid)).unwrap()["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == new_source)
            .unwrap()
            .clone();
        let token = row["fingerprint"].as_str().unwrap().to_string();
        player.state.status = "playing".into();
        assert!(
            player
                .command(Command::ApplyScoreSource {
                    id: new_source.clone(),
                    fingerprint: token.clone(),
                    mode: "performance".into()
                })
                .is_err()
        );
        player.state.status = "paused".into();
        let result = player
            .command(Command::ApplyScoreSource {
                id: new_source,
                fingerprint: token,
                mode: "performance".into(),
            })
            .unwrap();
        assert_ne!(result["contentId"], cid);
        assert!(result["warnings"].as_array().unwrap().is_empty());
        let current = player.command(Command::CurrentSong).unwrap();
        assert!(current["notes"][0]["finger"].is_null());
        player
            .command(Command::Load {
                path: midi,
                title: "旧曲目".into(),
            })
            .unwrap();
        assert_eq!(
            player.command(Command::CurrentSong).unwrap()["notes"][0]["finger"],
            2
        );
        assert_eq!(std::fs::read(&initial_copy).unwrap(), first);
        std::fs::remove_file(&original).unwrap();
        assert!(
            inspect(&data, Some(&cid)).unwrap()["entries"]
                .as_array()
                .unwrap()
                .iter()
                .all(|e| e["status"] == "missing")
        );
        assert!(initial_copy.is_file());
    }
}

/// The folder monitor checks only originals whose timestamp/size changed.
#[derive(Default)]
pub struct Scanner {
    cache: BTreeMap<PathBuf, ((u64, u128), Option<String>)>,
    signature: String,
}
impl Scanner {
    pub fn scan(&mut self, data: &Path) -> Result<(usize, usize, bool), String> {
        let store = load(data)?;
        let prefs = Preferences::load(&data.join("web-preferences.json"))?;
        let mut pending = 0;
        let mut missing = 0;
        let mut states = Vec::new();
        let paths = store
            .entries
            .values()
            .map(|s| s.path.clone())
            .collect::<std::collections::BTreeSet<_>>();
        self.cache.retain(|p, _| paths.contains(p));
        for source in store
            .entries
            .values()
            .filter(|s| version(&prefs, s).is_some())
        {
            let stamp = std::fs::metadata(&source.path).ok().map(|m| {
                (
                    m.len(),
                    m.modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_nanos()),
                )
            });
            let hash = if let Some(stamp) = stamp {
                if self
                    .cache
                    .get(&source.path)
                    .is_none_or(|(old, _)| *old != stamp)
                {
                    self.cache.insert(
                        source.path.clone(),
                        (
                            stamp,
                            read_original(&source.path)
                                .ok()
                                .map(|b| blake3::hash(&b).to_hex().to_string()),
                        ),
                    );
                }
                self.cache.get(&source.path).and_then(|(_, h)| h.clone())
            } else {
                self.cache.remove(&source.path);
                None
            };
            if let Some(h) = &hash {
                if h != &source.baseline && source.acknowledged.as_ref() != Some(h) {
                    pending += 1;
                }
            } else {
                missing += 1;
            }
            states.push(json!([source.id, hash, source.acknowledged]));
        }
        let signature = blake3::hash(&serde_json::to_vec(&states).map_err(|e| e.to_string())?)
            .to_hex()
            .to_string();
        let changed = signature != self.signature;
        self.signature = signature;
        Ok((pending, missing, changed))
    }
}
