use neothesia_core::library::SongMetadata;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: String,
    pub content_id: String,
    pub time_ms: u64,
    pub before: SongMetadata,
    pub after: SongMetadata,
    pub state: String,
    pub restored_from: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    version: u8,
    path: PathBuf,
    records: Vec<Record>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Portable {
    pub source_path: PathBuf,
    pub content_id: String,
    pub records: Vec<Record>,
}
pub struct Merge {
    pub files: BTreeMap<String, Vec<u8>>,
    pub added: usize,
    pub duplicates: usize,
    pub unmatched: usize,
    pub ambiguous: usize,
}
pub const FIELDS: &[&str] = &[
    "title",
    "composer",
    "artist",
    "collection",
    "difficulty",
    "tags",
    "notes",
];
fn file(data: &Path, path: &Path) -> PathBuf {
    data.join("metadata-history").join(format!(
        "{}.json",
        blake3::hash(crate::library_workspace::key(path).as_bytes())
    ))
}
fn load(data: &Path, path: &Path) -> Result<Journal, String> {
    let journal = match std::fs::read(file(data, path)) {
        Ok(bytes) => {
            if bytes.len() > 64_000_000 {
                return Err("资料历史文件过大".into());
            }
            serde_json::from_slice::<Journal>(&bytes)
                .map_err(|e| format!("资料历史无法读取，未覆盖：{e}"))?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Journal {
            version: 1,
            path: path.to_owned(),
            records: vec![],
        },
        Err(e) => return Err(e.to_string()),
    };
    if journal.version != 1
        || crate::library_workspace::key(&journal.path) != crate::library_workspace::key(path)
        || journal.records.len() > 200
        || journal.records.iter().any(|r| {
            !hex(&r.id)
                || !hex(&r.content_id)
                || !["prepared", "applied", "failed"].contains(&r.state.as_str())
                || r.restored_from.as_ref().is_some_and(|id| !hex(id))
        })
    {
        return Err("资料历史记录无效，未覆盖".into());
    }
    Ok(journal)
}
fn hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn validate_portable(row: &Portable) -> Result<(), String> {
    if !hex(&row.content_id)
        || row.source_path.as_os_str().is_empty()
        || row.source_path.to_string_lossy().len() > 4096
        || row.records.is_empty()
        || row.records.len() > 200
    {
        return Err("备份曲目资料历史身份或数量无效".into());
    }
    let mut ids = BTreeSet::new();
    for record in &row.records {
        if !hex(&record.id)
            || record.content_id != row.content_id
            || record.state != "applied"
            || record.restored_from.as_ref().is_some_and(|id| !hex(id))
            || !ids.insert(&record.id)
            || changed(&record.before, &record.after).is_empty()
        {
            return Err("备份曲目资料历史记录无效".into());
        }
    }
    Ok(())
}
pub fn validate_backup(rows: &[Portable]) -> Result<(), String> {
    if rows.len() > 10000 || rows.iter().map(|r| r.records.len()).sum::<usize>() > 100_000 {
        return Err("备份曲目资料历史超过容量限制".into());
    }
    for row in rows { validate_portable(row)?; }
    Ok(())
}
pub fn snapshot(data: &Path) -> Result<Vec<Portable>, String> {
    let folder = data.join("metadata-history");
    let entries = match std::fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(format!("资料历史目录无法读取：{e}")),
    };
    let mut result = vec![];
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".json") { continue; }
        let bytes = std::fs::read(entry.path()).map_err(|e| e.to_string())?;
        if bytes.len() > 64_000_000 { return Err("资料历史文件过大".into()); }
        let journal: Journal = serde_json::from_slice(&bytes)
            .map_err(|e| format!("资料历史无法读取，备份已中止：{e}"))?;
        if name != file(data, &journal.path).file_name().unwrap().to_string_lossy() {
            return Err("资料历史路径校验失败，备份已中止".into());
        }
        let checked = load(data, &journal.path)?;
        let mut grouped: BTreeMap<String, Vec<Record>> = BTreeMap::new();
        for record in checked.records.into_iter().filter(|r| r.state == "applied") {
            grouped.entry(record.content_id.clone()).or_default().push(record);
        }
        for (content_id, records) in grouped {
            let row = Portable { source_path: journal.path.clone(), content_id, records };
            validate_portable(&row)?;
            result.push(row);
        }
    }
    validate_backup(&result)?;
    Ok(result)
}
pub fn merge(data: &Path, workspace: &mut crate::library_workspace::Workspace, incoming: &[Portable]) -> Result<Merge, String> {
    validate_backup(incoming)?;
    let mut result = Merge { files: BTreeMap::new(), added: 0, duplicates: 0, unmatched: 0, ambiguous: 0 };
    if incoming.is_empty() { return Ok(result); }
    let mut by_content: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    let indexed = workspace.entries.values().map(|entry| entry.path.clone()).collect::<Vec<_>>();
    for path in indexed {
        let entry = workspace.inspect(path.clone(), true)?;
        if entry.available && entry.error.is_none() {
            if let Some(cid) = entry.content_id {
                let paths = by_content.entry(cid).or_default();
                if !paths.iter().any(|existing| crate::library_workspace::key(existing) == crate::library_workspace::key(&path)) {
                    paths.push(path);
                }
            }
        }
    }
    let mut planned: BTreeMap<String, Journal> = BTreeMap::new();
    for row in incoming {
        let candidates = by_content.get(&row.content_id).map(Vec::as_slice).unwrap_or(&[]);
        let exact = candidates.iter().find(|path| crate::library_workspace::key(path) == crate::library_workspace::key(&row.source_path));
        let target = if let Some(path) = exact { Some(path.clone()) } else if candidates.len() == 1 { Some(candidates[0].clone()) } else { None };
        let Some(target) = target else {
            if candidates.is_empty() { result.unmatched += row.records.len(); }
            else { result.ambiguous += row.records.len(); }
            continue;
        };
        let name = file(data, &target).file_name().unwrap().to_string_lossy().into_owned();
        if !planned.contains_key(&name) { planned.insert(name.clone(), load(data, &target)?); }
        let journal = planned.get_mut(&name).unwrap();
        for record in &row.records {
            if let Some(local) = journal.records.iter().find(|r| r.id == record.id) {
                if local.content_id != record.content_id || serde_json::to_vec(local).map_err(|e|e.to_string())? != serde_json::to_vec(record).map_err(|e|e.to_string())? {
                    return Err("曲目资料历史编号发生内容冲突，导入已中止".into());
                }
                result.duplicates += 1;
            } else {
                journal.records.push(record.clone());
                result.added += 1;
            }
        }
    }
    for (name, journal) in planned {
        if journal.records.len() > 200 {
            return Err("合并后的单曲资料历史超过 200 条，请先整理本地历史".into());
        }
        result.files.insert(name, serde_json::to_vec_pretty(&journal).map_err(|e|e.to_string())?);
    }
    Ok(result)
}
fn write(data: &Path, journal: &Journal) -> Result<(), String> {
    let path = file(data, &journal.path);
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(journal).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}
pub fn changed(before: &SongMetadata, after: &SongMetadata) -> Vec<String> {
    let before = serde_json::to_value(before).unwrap();
    let after = serde_json::to_value(after).unwrap();
    FIELDS
        .iter()
        .filter(|field| before[**field] != after[**field])
        .map(|field| (*field).into())
        .collect()
}
pub fn prepare(
    data: &Path,
    path: &Path,
    cid: &str,
    before: SongMetadata,
    after: SongMetadata,
    restored_from: Option<String>,
) -> Result<String, String> {
    let mut journal = load(data, path)?;
    let time_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;
    let id = blake3::hash(
        &serde_json::to_vec(&(
            cid,
            time_ms,
            &before,
            &after,
            journal.records.last().map(|r| &r.id),
        ))
        .map_err(|e| e.to_string())?,
    )
    .to_hex()
    .to_string();
    journal.records.push(Record {
        id: id.clone(),
        content_id: cid.into(),
        time_ms,
        before,
        after,
        state: "prepared".into(),
        restored_from,
    });
    if journal.records.len() > 200 {
        journal.records.drain(..journal.records.len() - 200);
    }
    write(data, &journal)?;
    Ok(id)
}
pub fn finish(data: &Path, path: &Path, id: &str, applied: bool) -> Result<(), String> {
    let mut journal = load(data, path)?;
    let record = journal
        .records
        .iter_mut()
        .find(|r| r.id == id)
        .ok_or("资料历史记录不见了")?;
    record.state = if applied { "applied" } else { "failed" }.into();
    write(data, &journal)
}
pub fn inspect(
    data: &Path,
    path: &Path,
    cid: &str,
    current: Option<&SongMetadata>,
) -> Result<serde_json::Value, String> {
    let journal = load(data, path)?;
    let mut records = vec![];
    for record in journal
        .records
        .into_iter()
        .rev()
        .filter(|r| r.content_id == cid)
    {
        let mut value = serde_json::to_value(&record).map_err(|e| e.to_string())?;
        value["fields"] = serde_json::json!(changed(&record.before, &record.after));
        value["matchesCurrent"] = serde_json::json!(current == Some(&record.after));
        records.push(value);
    }
    Ok(serde_json::json!({"records":records,"retention":200}))
}
pub fn record(data: &Path, path: &Path, cid: &str, id: &str) -> Result<Record, String> {
    let journal = load(data, path)?;
    let record = journal
        .records
        .into_iter()
        .find(|r| r.id == id && r.content_id == cid)
        .ok_or("此项资料历史不存在")?;
    if record.state != "applied" {
        return Err("此项写入未确认，不能作为恢复来源".into());
    }
    Ok(record)
}
pub fn proposed(
    record: &Record,
    current: &SongMetadata,
    fields: &[String],
) -> Result<SongMetadata, String> {
    let allowed = changed(&record.before, &record.after);
    if fields.is_empty() || fields.len() > 7 || fields.iter().any(|field| !allowed.contains(field))
    {
        return Err("请选择这次修改中的字段".into());
    }
    let mut value = serde_json::to_value(current).map_err(|e| e.to_string())?;
    let before = serde_json::to_value(&record.before).map_err(|e| e.to_string())?;
    for field in fields {
        value[field] = before[field].clone();
    }
    serde_json::from_value(value).map_err(|e| e.to_string())
}
