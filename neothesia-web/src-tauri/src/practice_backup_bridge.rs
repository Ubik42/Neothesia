use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
const MAX: u64 = 256_000_000;
const CHUNK: usize = 1_048_576;
struct Transfer {
    file: Option<File>,
    temp: Option<PathBuf>,
    size: u64,
    offset: u64,
}
impl Drop for Transfer {
    fn drop(&mut self) {
        self.file.take();
        if let Some(path) = &self.temp {
            let _ = std::fs::remove_file(path);
        }
    }
}
impl Transfer {
    fn write(&mut self, offset: u64, bytes: &[u8]) -> Result<(), String> {
        if self.temp.is_none()
            || bytes.is_empty()
            || bytes.len() > CHUNK
            || offset != self.offset
            || bytes.len() as u64 > self.size.saturating_sub(self.offset)
        {
            return Err("备份分块位置或大小无效".into());
        }
        self.file
            .as_mut()
            .ok_or("备份会话已结束")?
            .write_all(bytes)
            .map_err(|e| e.to_string())?;
        self.offset += bytes.len() as u64;
        Ok(())
    }
    fn read(&mut self, offset: u64, length: usize) -> Result<Vec<u8>, String> {
        if self.temp.is_some()
            || length == 0
            || length > CHUNK
            || offset != self.offset
            || length as u64 > self.size.saturating_sub(offset)
        {
            return Err("备份分块位置或大小无效".into());
        }
        let mut bytes = vec![0; length];
        self.file
            .as_mut()
            .ok_or("备份会话已结束")?
            .read_exact(&mut bytes)
            .map_err(|e| e.to_string())?;
        self.offset += length as u64;
        Ok(bytes)
    }
    fn staged_bytes(&mut self) -> Result<Vec<u8>, String> {
        if self.temp.is_none() || self.offset != self.size {
            return Err("备份文件尚未写入完整".into());
        }
        let file = self.file.as_mut().ok_or("备份会话已结束")?;
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take(MAX + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 != self.size {
            return Err("备份文件大小发生变化".into());
        }
        Ok(bytes)
    }
}
#[derive(Default)]
pub struct Sessions(Mutex<HashMap<String, Transfer>>);
fn token() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    blake3::hash(
        format!(
            "{}:{:?}:{}",
            std::process::id(),
            std::time::SystemTime::now(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )
        .as_bytes(),
    )
    .to_hex()
    .to_string()
}
fn insert(sessions: &Sessions, transfer: Transfer) -> Result<String, String> {
    let mut all = sessions.0.lock().map_err(|e| e.to_string())?;
    if all.len() >= 2 {
        return Err("请等待正在传输的练习备份完成".into());
    }
    let id = token();
    all.insert(id.clone(), transfer);
    Ok(id)
}
#[tauri::command]
pub async fn begin_practice_backup_read(
    state: tauri::State<'_, crate::State>,
    sessions: tauri::State<'_, Sessions>,
    selection: neothesia_engine::practice_backup::Selection,
) -> Result<serde_json::Value, String> {
    if sessions.0.lock().map_err(|e| e.to_string())?.len() >= 2 {
        return Err("请等待正在传输的练习备份完成".into());
    }
    let engine = state.engine.clone();
    let (file, size) = tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
        let result =
            engine.command(neothesia_engine::Command::ExportPracticeBackup { selection })?;
        let path = result["path"].as_str().ok_or("备份导出路径缺失")?;
        let file = File::open(path).map_err(|e| e.to_string())?;
        let size = file.metadata().map_err(|e| e.to_string())?.len();
        if size == 0 || size > MAX {
            return Err("练习备份大小无效，最大支持 256 MB".into());
        }
        Ok((file, size))
    })
    .await
    .map_err(|e| e.to_string())??;
    let session = insert(
        &sessions,
        Transfer {
            file: Some(file),
            temp: None,
            size,
            offset: 0,
        },
    )?;
    Ok(serde_json::json!({"session":session,"size":size}))
}
#[tauri::command]
pub fn read_practice_backup_chunk(
    sessions: tauri::State<'_, Sessions>,
    session: String,
    offset: u64,
    length: usize,
) -> Result<Vec<u8>, String> {
    let mut all = sessions.0.lock().map_err(|e| e.to_string())?;
    let result = all
        .get_mut(&session)
        .ok_or("备份会话已结束")?
        .read(offset, length);
    if result.is_err() {
        all.remove(&session);
    }
    result
}
#[tauri::command]
pub fn begin_practice_backup_stage(
    state: tauri::State<'_, crate::State>,
    sessions: tauri::State<'_, Sessions>,
    size: u64,
) -> Result<String, String> {
    if size == 0 || size > MAX {
        return Err("练习备份大小无效，最大支持 256 MB".into());
    }
    let dir = state.data.join("practice-bridge");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.tmp", token()));
    let file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    insert(
        &sessions,
        Transfer {
            file: Some(file),
            temp: Some(path),
            size,
            offset: 0,
        },
    )
}
#[tauri::command]
pub fn write_practice_backup_chunk(
    sessions: tauri::State<'_, Sessions>,
    session: String,
    offset: u64,
    bytes: Vec<u8>,
) -> Result<(), String> {
    let mut all = sessions.0.lock().map_err(|e| e.to_string())?;
    let result = all
        .get_mut(&session)
        .ok_or("备份会话已结束")?
        .write(offset, &bytes);
    if result.is_err() {
        all.remove(&session);
    }
    result
}
#[tauri::command]
pub async fn finish_practice_backup_stage(
    state: tauri::State<'_, crate::State>,
    sessions: tauri::State<'_, Sessions>,
    session: String,
) -> Result<serde_json::Value, String> {
    let mut transfer = sessions
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&session)
        .ok_or("备份会话已结束")?;
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = transfer.staged_bytes()?;
        engine.stage_practice_backup(bytes)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn finish_practice_backup_read(
    sessions: tauri::State<'_, Sessions>,
    session: String,
) -> Result<(), String> {
    let transfer = sessions
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&session)
        .ok_or("备份会话已结束")?;
    if transfer.temp.is_some() || transfer.offset != transfer.size {
        return Err("备份文件尚未读取完整".into());
    }
    Ok(())
}
#[tauri::command]
pub fn cancel_practice_backup_transfer(
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
    fn bounded_ordered_transfer_and_cleanup() {
        let path = std::env::temp_dir().join(format!("neothesia-bridge-{}.tmp", token()));
        let file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let mut transfer = Transfer {
            file: Some(file),
            temp: Some(path.clone()),
            size: 12,
            offset: 0,
        };
        assert!(transfer.write(1, b"bad").is_err());
        assert!(transfer.write(0, &vec![0; CHUNK + 1]).is_err());
        transfer.write(0, b"NEOPRAC1").unwrap();
        assert!(transfer.staged_bytes().is_err());
        transfer.write(8, b"test").unwrap();
        assert_eq!(transfer.staged_bytes().unwrap(), b"NEOPRAC1test");
        assert!(transfer.write(12, b"extra").is_err());
        let mut read = Transfer {
            file: Some(File::open(&path).unwrap()),
            temp: None,
            size: 12,
            offset: 0,
        };
        assert!(read.read(1, 4).is_err());
        assert!(read.read(0, CHUNK + 1).is_err());
        assert_eq!(read.read(0, 8).unwrap(), b"NEOPRAC1");
        assert_eq!(read.read(8, 4).unwrap(), b"test");
        drop(read);
        assert!(path.exists(), "reading must preserve the source backup");
        drop(transfer);
        assert!(!path.exists());
        let sessions = Sessions::default();
        for _ in 0..2 {
            insert(
                &sessions,
                Transfer {
                    file: None,
                    temp: None,
                    size: 1,
                    offset: 0,
                },
            )
            .unwrap();
        }
        assert!(
            insert(
                &sessions,
                Transfer {
                    file: None,
                    temp: None,
                    size: 1,
                    offset: 0
                }
            )
            .is_err()
        );
    }
}
