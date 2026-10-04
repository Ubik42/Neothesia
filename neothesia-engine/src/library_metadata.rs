use crate::library_workspace::Workspace;
use neothesia_core::library::{
    MetadataError, SongMetadata, load_song_metadata, save_song_metadata,
};
use std::path::{Path, PathBuf};
fn actual(
    workspace: &mut Workspace,
    path: &PathBuf,
    expected: Option<&str>,
) -> Result<(String, SongMetadata), String> {
    let entry = workspace.inspect(path.clone(), true)?;
    if !entry.available {
        return Err(entry
            .error
            .unwrap_or_else(|| "文件不可用，草稿尚未保存".into()));
    }
    let cid = entry.content_id.ok_or("无法识别曲目，草稿尚未保存")?;
    if expected.is_some_and(|id| id != cid) {
        return Err("文件内容已改变，草稿未写入另一首曲目；请取消后重新检查文件".into());
    }
    let metadata = match load_song_metadata(path, &cid) {
        Ok(value) => value,
        Err(MetadataError::Read(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            SongMetadata::default()
        }
        Err(e) => return Err(format!("附加资料无法读取，未修改文件：{e}")),
    };
    Ok((cid, metadata))
}
pub fn read(
    workspace: &mut Workspace,
    path: PathBuf,
    expected: Option<String>,
) -> Result<serde_json::Value, String> {
    let (cid, value) = actual(workspace, &path, expected.as_deref())?;
    Ok(serde_json::json!({"contentId":cid,"value":value}))
}
pub fn save(
    workspace: &mut Workspace,
    data: &Path,
    path: PathBuf,
    cid: String,
    value: SongMetadata,
    expected: SongMetadata,
) -> Result<serde_json::Value, String> {
    save_operation(workspace, data, path, cid, value, expected, None)
}
fn save_operation(
    workspace: &mut Workspace,
    data: &Path,
    path: PathBuf,
    cid: String,
    value: SongMetadata,
    expected: SongMetadata,
    restored_from: Option<String>,
) -> Result<serde_json::Value, String> {
    if cid.len() != 64 || !cid.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("曲目内容身份无效".into());
    }
    for (label, text, limit) in [
        ("曲名", &value.title, 2000),
        ("作曲家", &value.composer, 2000),
        ("演奏者", &value.artist, 2000),
        ("作品集", &value.collection, 2000),
        ("难度", &value.difficulty, 2000),
        ("备注", &value.notes, 10000),
    ] {
        if text.as_ref().is_some_and(|s| s.chars().count() > limit) {
            return Err(format!("{label}最多 {limit} 字"));
        }
    }
    if value.tags.len() > 100 || value.tags.iter().any(|s| s.chars().count() > 128) {
        return Err("最多 100 个标签，每个最多 128 字".into());
    }
    let (_, current) = actual(workspace, &path, Some(&cid))?;
    if current != expected {
        return Ok(serde_json::json!({"status":"conflict","current":current,"contentId":cid}));
    }
    let value = value.normalized();
    if value == current {
        return Ok(
            serde_json::json!({"status":"saved","value":current,"contentId":cid,"unchanged":true}),
        );
    }
    let id =
        crate::metadata_history::prepare(data, &path, &cid, current, value.clone(), restored_from)?;
    if let Err(e) = save_song_metadata(&path, &cid, value.clone()) {
        let _ = crate::metadata_history::finish(data, &path, &id, false);
        return Err(e.to_string());
    }
    let mut warnings = vec![];
    if let Err(e) = crate::metadata_history::finish(data, &path, &id, true) {
        warnings.push(format!("资料已保存，历史写入状态尚未确认：{e}"));
    }
    if let Err(e) = workspace.inspect(path.clone(), true) {
        warnings.push(format!("资料已保存，曲库索引尚未刷新：{e}"));
    }
    Ok(
        serde_json::json!({"status":"saved","value":value,"contentId":cid,"historyId":id,"warnings":warnings}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guarded_metadata_preserves_hints_conflicts_and_refuses_replaced_or_invalid_sources() {
        let root = std::env::temp_dir().join(format!(
            "metadata-edit-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("piece.mid");
        std::fs::write(&path, include_bytes!("../assets/five-finger.mid")).unwrap();
        let mut w = Workspace::default();
        let initial = read(&mut w, path.clone(), None).unwrap();
        let cid = initial["contentId"].as_str().unwrap().to_string();
        let original: SongMetadata = serde_json::from_value(initial["value"].clone()).unwrap();
        neothesia_core::library::save_song_fingerings(
            &path,
            &cid,
            vec![neothesia_core::library::FingerHint {
                track_id: 0,
                note_index: 0,
                finger: 2,
            }],
        )
        .unwrap();
        let mut theirs = original.clone();
        theirs.composer = Some("另一处作曲家".into());
        save_song_metadata(&path, &cid, theirs.clone()).unwrap();
        let mut mine = original.clone();
        mine.notes = Some("我的备注".into());
        let result = save(
            &mut w,
            &root,
            path.clone(),
            cid.clone(),
            mine.clone(),
            original,
        )
        .unwrap();
        assert_eq!(result["status"], "conflict");
        assert_eq!(load_song_metadata(&path, &cid).unwrap(), theirs);
        mine.composer = theirs.composer.clone();
        let result = save(
            &mut w,
            &root,
            path.clone(),
            cid.clone(),
            mine.clone(),
            theirs,
        )
        .unwrap();
        assert_eq!(result["status"], "saved");
        assert_eq!(
            neothesia_core::library::load_song_sidecar(&path, &cid)
                .unwrap()
                .fingerings[0]
                .finger,
            2
        );
        let sidecar = neothesia_core::library::metadata_sidecar_path(&path);
        let bytes = std::fs::read(&sidecar).unwrap();
        std::fs::write(&path, include_bytes!("../assets/basic-chords.mid")).unwrap();
        assert!(
            save(
                &mut w,
                &root,
                path.clone(),
                cid.clone(),
                mine.clone(),
                mine.clone()
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&sidecar).unwrap(), bytes);
        std::fs::write(&path, include_bytes!("../assets/five-finger.mid")).unwrap();
        std::fs::write(&sidecar, b"broken").unwrap();
        assert!(read(&mut w, path.clone(), Some(cid.clone())).is_err());
        assert!(save(&mut w, &root, path, cid, mine.clone(), mine).is_err());
        assert_eq!(std::fs::read(&sidecar).unwrap(), b"broken");
    }
    #[test]
    fn history_restore_preserves_newer_fields_rejects_stale_content_and_records_reverse() {
        let root = std::env::temp_dir().join(format!(
            "metadata-history-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("piece.mid");
        std::fs::write(&path, include_bytes!("../assets/five-finger.mid")).unwrap();
        let mut w = Workspace::default();
        let initial = read(&mut w, path.clone(), None).unwrap();
        let cid = initial["contentId"].as_str().unwrap().to_string();
        let original: SongMetadata = serde_json::from_value(initial["value"].clone()).unwrap();
        neothesia_core::library::save_song_fingerings(
            &path,
            &cid,
            vec![neothesia_core::library::FingerHint {
                track_id: 0,
                note_index: 0,
                finger: 2,
            }],
        )
        .unwrap();
        let mut first = original.clone();
        first.tags = vec!["课堂".into()];
        first.notes = Some("老师提示".into());
        let first_reply = save(
            &mut w,
            &root,
            path.clone(),
            cid.clone(),
            first.clone(),
            original.clone(),
        )
        .unwrap();
        let id = first_reply["historyId"].as_str().unwrap().to_string();
        let mut later = first.clone();
        later.artist = Some("保留演奏者".into());
        later.notes = Some("后来备注".into());
        save_song_metadata(&path, &cid, later.clone()).unwrap();
        let preview =
            restore_preview(&mut w, &root, path.clone(), cid.clone(), id.clone()).unwrap();
        assert_eq!(preview["conflicts"], serde_json::json!(["notes"]));
        assert_eq!(preview["value"]["artist"], "保留演奏者");
        let stale = restore(
            &mut w,
            &root,
            path.clone(),
            cid.clone(),
            id.clone(),
            vec!["tags".into()],
            first,
        )
        .unwrap();
        assert_eq!(stale["status"], "conflict");
        assert_eq!(load_song_metadata(&path, &cid).unwrap(), later);
        let result = restore(
            &mut w,
            &root,
            path.clone(),
            cid.clone(),
            id.clone(),
            vec!["tags".into()],
            later.clone(),
        )
        .unwrap();
        let undo_id = result["historyId"].as_str().unwrap().to_string();
        let after = load_song_metadata(&path, &cid).unwrap();
        assert_eq!(after.tags, original.tags);
        assert_eq!(after.notes, later.notes);
        assert_eq!(after.artist, later.artist);
        assert_eq!(
            neothesia_core::library::load_song_sidecar(&path, &cid)
                .unwrap()
                .fingerings[0]
                .finger,
            2
        );
        let listed = history(&mut w, &root, path.clone(), cid.clone()).unwrap();
        assert_eq!(listed["records"].as_array().unwrap().len(), 2);
        assert_eq!(listed["records"][0]["restoredFrom"], id);
        restore(
            &mut w,
            &root,
            path.clone(),
            cid.clone(),
            undo_id,
            vec!["tags".into()],
            after,
        )
        .unwrap();
        assert_eq!(load_song_metadata(&path, &cid).unwrap().tags, vec!["课堂"]);
        let count = history(&mut w, &root, path.clone(), cid.clone()).unwrap()["records"]
            .as_array()
            .unwrap()
            .len();
        save(
            &mut w,
            &root,
            path.clone(),
            cid.clone(),
            later.clone(),
            later.clone(),
        )
        .unwrap();
        assert_eq!(
            history(&mut w, &root, path.clone(), cid.clone()).unwrap()["records"]
                .as_array()
                .unwrap()
                .len(),
            count
        );
        let sidecar = neothesia_core::library::metadata_sidecar_path(&path);
        let bytes = std::fs::read(&sidecar).unwrap();
        std::fs::write(&path, include_bytes!("../assets/basic-chords.mid")).unwrap();
        assert!(
            restore(
                &mut w,
                &root,
                path.clone(),
                cid.clone(),
                id.clone(),
                vec!["tags".into()],
                later.clone()
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&sidecar).unwrap(), bytes);
        assert_eq!(
            history(&mut w, &root, path.clone(), cid.clone()).unwrap()["available"],
            false
        );
        std::fs::write(&path, include_bytes!("../assets/five-finger.mid")).unwrap();
        let prepared = crate::metadata_history::prepare(
            &root,
            &path,
            &cid,
            later.clone(),
            original.clone(),
            None,
        )
        .unwrap();
        assert!(restore_preview(&mut w, &root, path.clone(), cid.clone(), prepared).is_err());
        let journal = root.join("metadata-history").join(format!(
            "{}.json",
            blake3::hash(crate::library_workspace::key(&path).as_bytes())
        ));
        std::fs::write(&journal, b"broken").unwrap();
        assert!(save(&mut w, &root, path, cid, original, later).is_err());
        assert_eq!(std::fs::read(sidecar).unwrap(), bytes);
        assert_eq!(std::fs::read(journal).unwrap(), b"broken");
    }
}

pub fn preview(
    workspace: &mut Workspace,
    path: PathBuf,
    expected: Option<String>,
    rules: crate::metadata_templates::Rules,
) -> Result<serde_json::Value, String> {
    let (cid, base) = actual(workspace, &path, expected.as_deref())?;
    let value = crate::metadata_templates::apply(base.clone(), &rules)?;
    Ok(serde_json::json!({"contentId":cid,"base":base,"value":value}))
}

pub fn history(
    workspace: &mut Workspace,
    data: &Path,
    path: PathBuf,
    cid: String,
) -> Result<serde_json::Value, String> {
    let actual = actual(workspace, &path, Some(&cid));
    let mut value =
        crate::metadata_history::inspect(data, &path, &cid, actual.as_ref().ok().map(|(_, m)| m))?;
    value["available"] = serde_json::json!(actual.is_ok());
    value["error"] = serde_json::json!(actual.err());
    Ok(value)
}
pub fn restore_preview(
    workspace: &mut Workspace,
    data: &Path,
    path: PathBuf,
    cid: String,
    id: String,
) -> Result<serde_json::Value, String> {
    let (_, base) = actual(workspace, &path, Some(&cid))?;
    let record = crate::metadata_history::record(data, &path, &cid, &id)?;
    let fields = crate::metadata_history::changed(&record.before, &record.after);
    let value = crate::metadata_history::proposed(&record, &base, &fields)?;
    let before = serde_json::to_value(&record.before).unwrap();
    let after = serde_json::to_value(&record.after).unwrap();
    let current = serde_json::to_value(&base).unwrap();
    let conflicts = fields
        .iter()
        .filter(|field| {
            current[field.as_str()] != after[field.as_str()]
                && current[field.as_str()] != before[field.as_str()]
        })
        .collect::<Vec<_>>();
    Ok(
        serde_json::json!({"base":base,"value":value,"fields":fields,"conflicts":conflicts,"after":record.after}),
    )
}
pub fn restore(
    workspace: &mut Workspace,
    data: &Path,
    path: PathBuf,
    cid: String,
    id: String,
    fields: Vec<String>,
    expected: SongMetadata,
) -> Result<serde_json::Value, String> {
    let (_, current) = actual(workspace, &path, Some(&cid))?;
    if current != expected {
        return Ok(serde_json::json!({"status":"conflict","current":current,"contentId":cid}));
    }
    let record = crate::metadata_history::record(data, &path, &cid, &id)?;
    let value = crate::metadata_history::proposed(&record, &current, &fields)?;
    save_operation(workspace, data, path, cid, value, expected, Some(id))
}
