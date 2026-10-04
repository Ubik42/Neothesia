use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
pub const KINDS: &[(&str, &str)] = &[
    ("music", "曲目与乐谱"),
    ("library", "曲库文件夹"),
    ("notation", "MusicXML 乐谱"),
    ("paper", "PDF 与图片"),
    ("piece", "曲目包"),
    ("backup", "练习备份"),
    ("exportMidi", "MIDI 导出"),
    ("exportScore", "乐谱导出"),
    ("exportPaper", "谱页导出"),
    ("exportPiece", "曲目包导出"),
    ("exportBackup", "练习备份导出"),
    ("repair", "重新定位文件"),
];
#[derive(Default, Serialize, Deserialize)]
struct Locations {
    #[serde(default)]
    version: u8,
    #[serde(default)]
    kinds: BTreeMap<String, Vec<PathBuf>>,
}
fn validate(kind: &str) -> Result<(), String> {
    if KINDS.iter().any(|(k, _)| *k == kind) {
        Ok(())
    } else {
        Err("文件位置类别无效".into())
    }
}
fn read(data: &Path) -> Result<Locations, String> {
    match std::fs::read(data.join("file-dialog-locations.json")) {
        Ok(bytes) => {
            let value: Locations =
                serde_json::from_slice(&bytes).map_err(|e| format!("文件位置记录无法读取：{e}"))?;
            if value.version != 1 {
                return Err("文件位置记录版本不支持".into());
            }
            if value.kinds.iter().any(|(k, v)| {
                validate(k).is_err() || v.len() > 20 || v.iter().any(|p| !p.is_absolute())
            }) {
                return Err("文件位置记录无效".into());
            }
            Ok(value)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Locations {
            version: 1,
            ..Default::default()
        }),
        Err(e) => Err(e.to_string()),
    }
}
fn write(data: &Path, value: &Locations) -> Result<(), String> {
    std::fs::create_dir_all(data).map_err(|e| e.to_string())?;
    let tmp = data.join("file-dialog-locations.json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(tmp, data.join("file-dialog-locations.json")).map_err(|e| e.to_string())
}
fn promote(paths: &mut Vec<PathBuf>, path: PathBuf) {
    let key = crate::library_workspace::key(&path);
    paths.retain(|p| crate::library_workspace::key(p) != key);
    paths.insert(0, path);
    paths.truncate(20);
}
pub fn remember(data: &Path, kind: &str, path: &Path, folder: bool) -> Result<(), String> {
    validate(kind)?;
    let path = if folder {
        path
    } else {
        path.parent().ok_or("文件上级位置无效")?
    };
    if !path.is_absolute() || !path.is_dir() {
        return Err("文件位置不可用".into());
    }
    let lock = crate::library_workspace::lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "文件位置正忙")?;
    let mut value = read(data)?;
    promote(value.kinds.entry(kind.into()).or_default(), path.to_owned());
    write(data, &value)
}
pub fn initial(data: &Path, kind: &str) -> Option<PathBuf> {
    validate(kind).ok()?;
    let value = read(data).ok()?;
    let mut path = value.kinds.get(kind)?.first()?.clone();
    loop {
        if path.is_dir() {
            return Some(path);
        }
        if !path.pop() {
            return None;
        }
    }
}
pub fn inspect(data: &Path) -> Result<serde_json::Value, String> {
    let lock = crate::library_workspace::lock_for(data)?;
    let _guard = lock.lock().map_err(|_| "文件位置正忙")?;
    let value = read(data)?;
    Ok(
        serde_json::json!({"kinds":KINDS.iter().map(|(kind,label)|serde_json::json!({"id":kind,"label":label,"paths":value.kinds.get(*kind).into_iter().flatten().map(|p|serde_json::json!({"path":p,"available":p.is_dir()})).collect::<Vec<_>>()})).collect::<Vec<_>>()}),
    )
}
pub fn update(
    data: &Path,
    kind: &str,
    path: Option<PathBuf>,
    clear: bool,
) -> Result<serde_json::Value, String> {
    validate(kind)?;
    {
        let lock = crate::library_workspace::lock_for(data)?;
        let _guard = lock.lock().map_err(|_| "文件位置正忙")?;
        let mut value = read(data)?;
        let paths = value.kinds.entry(kind.into()).or_default();
        if clear {
            if let Some(path) = path {
                let key = crate::library_workspace::key(&path);
                paths.retain(|p| crate::library_workspace::key(p) != key);
            } else {
                paths.clear();
            }
        } else {
            let path = path.ok_or("请选择已记录的文件位置")?;
            let key = crate::library_workspace::key(&path);
            let registered = paths
                .iter()
                .find(|p| crate::library_workspace::key(p) == key)
                .ok_or("此位置未登记，请通过文件选择窗口使用一次")?
                .clone();
            if !registered.is_dir() {
                return Err("此文件夹当前不可用".into());
            }
            promote(paths, registered);
        }
        write(data, &value)?;
    }
    inspect(data)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn categories_persist_select_forget_and_fall_back() {
        let root = std::env::temp_dir().join(format!(
            "neothesia-locations-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let data = root.join("data");
        let a = root.join("scores");
        let b = root.join("piano");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        remember(&data, "music", &a.join("piece.mid"), false).unwrap();
        remember(&data, "music", &b, true).unwrap();
        remember(&data, "notation", &a, true).unwrap();
        assert_eq!(initial(&data, "music"), Some(b.clone()));
        update(&data, "music", Some(a.clone()), false).unwrap();
        assert_eq!(initial(&data, "music"), Some(a.clone()));
        let missing = a.join("offline");
        std::fs::create_dir_all(&missing).unwrap();
        remember(&data, "music", &missing, true).unwrap();
        std::fs::remove_dir(&missing).unwrap();
        assert_eq!(initial(&data, "music"), Some(a.clone()));
        assert!(update(&data, "music", Some(missing.clone()), false).is_err());
        update(&data, "music", Some(missing), true).unwrap();
        update(&data, "music", None, true).unwrap();
        assert_eq!(initial(&data, "music"), None);
        assert_eq!(initial(&data, "notation"), Some(a.clone()));
        assert!(a.is_dir());
        assert!(remember(&data, "invalid", &b, true).is_err());
    }
}
