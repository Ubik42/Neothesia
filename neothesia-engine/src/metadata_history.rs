use neothesia_core::library::SongMetadata;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
            r.id.len() != 64
                || r.content_id.len() != 64
                || !["prepared", "applied", "failed"].contains(&r.state.as_str())
        })
    {
        return Err("资料历史记录无效，未覆盖".into());
    }
    Ok(journal)
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
