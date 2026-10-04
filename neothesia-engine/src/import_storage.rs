use serde::Serialize;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovery {
    pub preserved_path: PathBuf,
    pub restored_path: PathBuf,
    pub carried_annotations: bool,
}
pub struct Stored {
    pub path: PathBuf,
    pub recovery: Option<Recovery>,
}
impl Stored {
    pub fn warning(&self) -> Option<String> {
        self.recovery.as_ref().map(|r| {
            format!(
                "原导入文件或附加资料已变化，原件保留在 {}；演奏恢复到 {}{}",
                r.preserved_path.display(),
                r.restored_path.display(),
                if r.carried_annotations {
                    "，已保留同一内容的个人资料"
                } else {
                    ""
                }
            )
        })
    }
}
fn usable(path: &Path, bytes_id: &str, cid: &str) -> bool {
    let Ok(stat) = std::fs::metadata(path) else {
        return false;
    };
    if !stat.is_file() || stat.len() > 32_000_000 {
        return false;
    }
    if midi_file::MidiFile::new(path)
        .ok()
        .is_none_or(|f| f.content_id != bytes_id)
    {
        return false;
    }
    match neothesia_core::library::load_song_sidecar(path, cid) {
        Ok(_) => true,
        Err(neothesia_core::library::MetadataError::Read(e))
            if e.kind() == std::io::ErrorKind::NotFound =>
        {
            true
        }
        _ => false,
    }
}
pub fn store(root: &Path, stem: &str, cid: &str, bytes: &[u8]) -> Result<Stored, String> {
    store_with_identity(root, stem, cid, cid, bytes)
}
pub fn store_with_identity(
    root: &Path,
    stem: &str,
    bytes_id: &str,
    cid: &str,
    bytes: &[u8],
) -> Result<Stored, String> {
    if stem.len() != 64
        || !stem.bytes().all(|b| b.is_ascii_hexdigit())
        || cid.len() != 64
        || !cid.bytes().all(|b| b.is_ascii_hexdigit())
        || blake3::hash(bytes).to_hex().as_str() != bytes_id
    {
        return Err("导入内容身份无效".into());
    }
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let original = root.join(format!("{stem}.mid"));
    let annotations = neothesia_core::library::load_song_sidecar(&original, cid).ok();
    for index in 0..1000 {
        let path = if index == 0 {
            original.clone()
        } else {
            root.join(format!("{stem}-restored-{index}.mid"))
        };
        if path.exists() {
            if !usable(&path, bytes_id, cid) {
                continue;
            }
            let carried_annotations =
                neothesia_core::library::load_song_sidecar(&path, cid).is_ok();
            return Ok(Stored {
                path: path.clone(),
                recovery: (index > 0).then_some(Recovery {
                    preserved_path: original,
                    restored_path: path,
                    carried_annotations,
                }),
            });
        }
        let sidecar = neothesia_core::library::metadata_sidecar_path(&path);
        if sidecar.exists() && neothesia_core::library::load_song_sidecar(&path, cid).is_err() {
            continue;
        }
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        };
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        if index > 0 && !sidecar.exists() {
            if let Some(value) = &annotations {
                neothesia_core::library::save_song_sidecar(&path, cid, value.clone())
                    .map_err(|e| e.to_string())?;
            }
        }
        return Ok(Stored {
            path: path.clone(),
            recovery: (index > 0).then_some(Recovery {
                preserved_path: original,
                restored_path: path,
                carried_annotations: annotations.is_some(),
            }),
        });
    }
    Err("恢复副本过多，请先整理导入文件位置".into())
}
