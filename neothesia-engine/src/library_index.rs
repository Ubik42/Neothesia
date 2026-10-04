use crate::{
    Command,
    library_workspace::{Workspace, key, lock_for},
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Progress {
    pub status: String,
    pub done: usize,
    pub total: usize,
    pub errors: usize,
    pub current: String,
    pub message: String,
}
#[derive(Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct Job {
    version: u16,
    paths: Vec<PathBuf>,
    force: bool,
    progress: Progress,
}
#[derive(Clone)]
pub struct Indexer {
    tx: mpsc::Sender<(Command, mpsc::SyncSender<Result<serde_json::Value, String>>)>,
    progress: Arc<Mutex<Progress>>,
}
impl Indexer {
    pub fn start(data: PathBuf) -> Self {
        let (tx, rx) =
            mpsc::channel::<(Command, mpsc::SyncSender<Result<serde_json::Value, String>>)>();
        let progress = Arc::new(Mutex::new(Progress {
            status: "idle".into(),
            ..Default::default()
        }));
        let shared = progress.clone();
        std::thread::spawn(move || {
            let path = data.join("library-index-job.json");
            let mut job = match std::fs::read(&path) {
                Ok(bytes) => serde_json::from_slice::<Job>(&bytes).unwrap_or_else(|e| Job {
                    progress: Progress {
                        status: "failed".into(),
                        message: format!("上次索引任务无法恢复：{e}"),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                Err(_) => Job {
                    progress: Progress {
                        status: "idle".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            };
            if job.version != 1 && !job.paths.is_empty() {
                job.progress.status = "failed".into();
                job.progress.message = "索引任务版本不兼容，请重新开始索引".into();
            }
            if job.progress.done > job.paths.len() {
                job.progress.status = "failed".into();
                job.progress.message = "索引进度无效，请重新开始".into();
            }
            job.progress.total = job.paths.len();
            let save = |job: &Job| -> Result<(), String> {
                std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
                let tmp = path.with_extension("json.tmp");
                use std::io::Write;
                let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
                file.write_all(&serde_json::to_vec(job).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
                file.sync_all().map_err(|e| e.to_string())?;
                drop(file);
                std::fs::rename(tmp, &path).map_err(|e| e.to_string())?;
                Ok(())
            };
            loop {
                *shared.lock().unwrap() = job.progress.clone();
                let pending = if job.progress.status == "running" {
                    match rx.try_recv() {
                        Ok(v) => Some(v),
                        Err(mpsc::TryRecvError::Disconnected) => break,
                        Err(_) => None,
                    }
                } else {
                    match rx.recv_timeout(Duration::from_millis(200)) {
                        Ok(v) => Some(v),
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(_) => None,
                    }
                };
                if let Some((command, reply)) = pending {
                    let result = (|| -> Result<serde_json::Value, String> {
                        match command {
                            Command::IndexLibrary { paths, force } => {
                                if ["running", "paused"].contains(&job.progress.status.as_str()) {
                                    return Err("已有索引任务，请先完成或取消它".into());
                                }
                                if paths.len() > 50_000 {
                                    return Err("一次最多索引 50000 个文件".into());
                                }
                                let mut seen = std::collections::HashSet::new();
                                let paths: Vec<_> =
                                    paths.into_iter().filter(|p| seen.insert(key(p))).collect();
                                job = Job {
                                    version: 1,
                                    progress: Progress {
                                        status: if paths.is_empty() {
                                            "completed"
                                        } else {
                                            "running"
                                        }
                                        .into(),
                                        total: paths.len(),
                                        ..Default::default()
                                    },
                                    paths,
                                    force,
                                };
                            }
                            Command::PauseIndex => {
                                if job.progress.status == "running" {
                                    job.progress.status = "paused".into();
                                }
                            }
                            Command::ResumeIndex => {
                                if job.progress.status != "paused" {
                                    return Err("没有可继续的暂停索引".into());
                                }
                                job.progress.status = "running".into();
                            }
                            Command::CancelIndex => {
                                job.progress.status = "cancelled".into();
                                job.progress.current.clear();
                            }
                            _ => return Err("索引命令无效".into()),
                        }
                        save(&job)?;
                        *shared.lock().unwrap() = job.progress.clone();
                        serde_json::to_value(&job.progress).map_err(|e| e.to_string())
                    })();
                    let _ = reply.send(result);
                    continue;
                }
                if job.progress.status != "running" {
                    continue;
                }
                let result = (|| -> Result<(), String> {
                    let lock = lock_for(&data)?;
                    let _guard = lock.lock().map_err(|_| "曲库索引锁不可用")?;
                    let mut workspace = Workspace::load(&data)?;
                    let end = (job.progress.done + 32).min(job.paths.len());
                    for i in job.progress.done..end {
                        let file = &job.paths[i];
                        job.progress.current = file
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into();
                        match workspace.inspect(file.clone(), job.force) {
                            Ok(e) => {
                                if e.error.is_some() {
                                    job.progress.errors += 1;
                                }
                            }
                            Err(e) => {
                                job.progress.errors += 1;
                                job.progress.message = e;
                            }
                        }
                        job.progress.done = i + 1;
                        *shared.lock().unwrap() = job.progress.clone();
                    }
                    workspace.save(&data)?;
                    if end == job.paths.len() {
                        job.progress.status = "completed".into();
                        job.progress.current.clear();
                    }
                    save(&job)
                })();
                if let Err(e) = result {
                    job.progress.status = "failed".into();
                    job.progress.message = e;
                    let _ = save(&job);
                }
            }
        });
        Self { tx, progress }
    }
    pub fn command(&self, command: Command) -> Result<serde_json::Value, String> {
        if matches!(command, Command::IndexStatus) {
            return serde_json::to_value(
                self.progress.lock().map_err(|_| "索引状态不可用")?.clone(),
            )
            .map_err(|e| e.to_string());
        }
        let (reply, rx) = mpsc::sync_channel(1);
        self.tx
            .send((command, reply))
            .map_err(|_| "索引服务已停止")?;
        rx.recv_timeout(Duration::from_secs(15))
            .map_err(|_| "索引服务响应超时")?
    }
}
