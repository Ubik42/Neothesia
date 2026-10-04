use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
};
pub struct Export {
    path: PathBuf,
    temp: PathBuf,
    file: Option<File>,
    size: u64,
    written: u64,
}
impl Drop for Export {
    fn drop(&mut self) {
        self.file.take();
        let _ = std::fs::remove_file(&self.temp);
    }
}
impl Export {
    fn append(&mut self, offset: u64, bytes: &[u8]) -> Result<(), String> {
        if bytes.is_empty()
            || bytes.len() > 1_048_576
            || offset != self.written
            || self.written + bytes.len() as u64 > self.size
        {
            return Err("批次写入位置或大小无效".into());
        }
        self.file
            .as_mut()
            .ok_or("导出已结束")?
            .write_all(bytes)
            .map_err(|e| format!("批次写入失败：{e}"))?;
        self.written += bytes.len() as u64;
        Ok(())
    }
    fn complete(mut self) -> Result<String, String> {
        if self.written != self.size {
            return Err("批次文件尚未写入完整".into());
        }
        self.file
            .take()
            .ok_or("导出已结束")?
            .sync_all()
            .map_err(|e| format!("批次写入失败：{e}"))?;
        std::fs::rename(&self.temp, &self.path).map_err(|e| format!("无法保存批次文件：{e}"))?;
        Ok(self.path.to_string_lossy().into())
    }
}
#[derive(Default)]
pub struct Sessions(pub Mutex<HashMap<String, Export>>);
#[tauri::command]
pub async fn begin_score_batch_export(
    state: tauri::State<'_, crate::State>,
    sessions: tauri::State<'_, Sessions>,
    name: String,
    size: u64,
) -> Result<Option<String>, String> {
    if size < 12 || size > 514_000_012 {
        return Err("批次大小无效".into());
    }
    if sessions.0.lock().map_err(|e| e.to_string())?.len() >= 2 {
        return Err("请等待正在导出的批次完成".into());
    }
    let name = name
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect::<String>();
    let Some(file) = crate::file_dialog(&state.data, "scoreBatchExport")
        .set_title("导出曲谱整理批次")
        .add_filter("曲谱整理批次", &["neoscorebatch"])
        .set_file_name(name)
        .save_file()
        .await
    else {
        return Ok(None);
    };
    let path = file.path().to_path_buf();
    crate::remember_location(&state.data, "scoreBatchExport", &path, false);
    let token = blake3::hash(
        format!(
            "{}:{:?}:{}",
            std::process::id(),
            std::time::SystemTime::now(),
            path.display()
        )
        .as_bytes(),
    )
    .to_hex()
    .to_string();
    let temp = path.with_file_name(format!(".neothesia-batch-{token}.tmp"));
    let writer = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)
        .map_err(|e| format!("无法创建批次导出文件：{e}"))?;
    let export = Export {
        path,
        temp,
        file: Some(writer),
        size,
        written: 0,
    };
    let mut active = sessions.0.lock().map_err(|e| e.to_string())?;
    if active.len() >= 2 {
        return Err("请等待正在导出的批次完成".into());
    }
    active.insert(token.clone(), export);
    Ok(Some(token))
}
#[tauri::command]
pub fn write_score_batch_export(
    sessions: tauri::State<'_, Sessions>,
    session: String,
    offset: u64,
    bytes: Vec<u8>,
) -> Result<(), String> {
    let mut all = sessions.0.lock().map_err(|e| e.to_string())?;
    let s = all.get_mut(&session).ok_or("导出会话已结束")?;
    s.append(offset, &bytes)
}
#[tauri::command]
pub fn finish_score_batch_export(
    sessions: tauri::State<'_, Sessions>,
    session: String,
) -> Result<String, String> {
    let s = sessions
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&session)
        .ok_or("导出会话已结束")?;
    s.complete()
}
#[tauri::command]
pub fn cancel_score_batch_export(
    sessions: tauri::State<'_, Sessions>,
    session: String,
) -> Result<(), String> {
    sessions
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&session);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunk_order_finish_and_cancel_preserve_existing_file() {
        let root = std::env::temp_dir().join(format!(
            "neothesia-batch-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("batch.neoscorebatch");
        std::fs::write(&path, b"previous").unwrap();
        let temp = root.join("partial.tmp");
        let mut s = Export {
            path: path.clone(),
            temp: temp.clone(),
            file: Some(
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temp)
                    .unwrap(),
            ),
            size: 12,
            written: 0,
        };
        assert!(s.append(1, b"bad").is_err());
        s.append(0, b"NEOBAT01").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"previous");
        assert!(s.append(8, b"too large").is_err());
        s.append(8, &[0, 0, 0, 0]).unwrap();
        assert!(s.append(12, b"extra").is_err());
        s.complete().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"NEOBAT01\0\0\0\0");
        assert!(!temp.exists());
        let s = Export {
            path: path.clone(),
            temp: temp.clone(),
            file: Some(
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temp)
                    .unwrap(),
            ),
            size: 12,
            written: 0,
        };
        assert!(s.complete().is_err());
        assert!(!temp.exists());
        assert_eq!(std::fs::read(&path).unwrap(), b"NEOBAT01\0\0\0\0");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
