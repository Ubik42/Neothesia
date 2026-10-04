//! Read-only original scans run on the existing library worker, never on the musical clock.
use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    sync::{Mutex, OnceLock},
    time::UNIX_EPOCH,
};
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Row {
    content_id: String,
    book: String,
    book_name: String,
    asset_id: String,
    page: usize,
    name: String,
    path: PathBuf,
    status: String,
    fingerprint: Option<String>,
    baseline: String,
    error: Option<String>,
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    rows: Vec<Row>,
    errors: Vec<String>,
    last_scan: u64,
    revision: String,
}
static SHARED: OnceLock<Mutex<BTreeMap<PathBuf, Snapshot>>> = OnceLock::new();
pub fn snapshot(data: &Path) -> Result<Value, String> {
    let shared = SHARED
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "原件检查正忙")?;
    serde_json::to_value(shared.get(data).cloned().unwrap_or_default()).map_err(|e| e.to_string())
}
#[derive(Default)]
pub struct Scanner {
    manifests: BTreeMap<PathBuf, ((u64, u128), Store)>,
    files: BTreeMap<PathBuf, ((u64, u128), Result<String, String>)>,
    signature: String,
}
fn stamp(p: &Path) -> Result<(u64, u128), String> {
    let m = std::fs::metadata(p).map_err(|e| e.to_string())?;
    if !m.is_file() {
        return Err("位置不是文件".into());
    }
    Ok((
        m.len(),
        m.modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos()),
    ))
}
impl Scanner {
    pub fn scan(&mut self, data: &Path) -> Result<(usize, usize, bool), String> {
        let root = data.join("score-attachments");
        let mut dirs = Vec::new();
        match std::fs::read_dir(&root) {
            Ok(entries) => {
                for e in entries {
                    let e = e.map_err(|e| e.to_string())?;
                    if e.file_type().map_err(|e| e.to_string())?.is_dir() {
                        dirs.push(e.path());
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
        dirs.sort();
        if dirs.len() > 100000 {
            return Err("谱页曲目数量超过检查上限".into());
        }
        let mut rows = Vec::new();
        let mut errors = Vec::new();
        let mut manifests = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for dir in dirs {
            let p = dir.join("manifest.json");
            manifests.insert(p.clone());
            let loaded = (|| -> Result<Store, String> {
                let st = stamp(&p)?;
                if self.manifests.get(&p).is_none_or(|(old, _)| *old != st) {
                    if st.0 > 8000000 {
                        return Err("谱页目录超过 8 MB".into());
                    }
                    let mut bytes = Vec::new();
                    std::fs::File::open(&p)
                        .map_err(|e| e.to_string())?
                        .take(8000001)
                        .read_to_end(&mut bytes)
                        .map_err(|e| e.to_string())?;
                    if bytes.len() > 8000000 {
                        return Err("谱页目录超过 8 MB".into());
                    }
                    let store: Store = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                    if folder(data, &store.content_id)? != dir || store.attachments.len() > 40 {
                        return Err("谱页曲目身份或版本数量不符".into());
                    }
                    self.manifests.insert(p.clone(), (st, store));
                }
                Ok(self.manifests[&p].1.clone())
            })();
            let store = match loaded {
                Ok(s) => s,
                Err(e) => {
                    self.manifests.remove(&p);
                    errors.push(format!(
                        "{}：{e}",
                        dir.file_name().unwrap_or_default().to_string_lossy()
                    ));
                    continue;
                }
            };
            for a in store.attachments {
                let baseline = attachment_baseline(&a);
                for (i, page) in a.pages.iter().enumerate() {
                    let Some(source) = a.sources.get(&page.id) else {
                        continue;
                    };
                    paths.insert(source.path.clone());
                    let result = match stamp(&source.path) {
                        Ok(st) => {
                            if self
                                .files
                                .get(&source.path)
                                .is_none_or(|(old, result)| *old != st || result.is_err())
                            {
                                let hash = source_bytes(&source.path)
                                    .map(|b| blake3::hash(&b).to_hex().to_string());
                                self.files.insert(source.path.clone(), (st, hash));
                            }
                            self.files[&source.path].1.clone()
                        }
                        Err(e) => {
                            self.files.remove(&source.path);
                            Err(e)
                        }
                    };
                    let (status, fingerprint, error) = match result {
                        Ok(h) => (
                            (if h == page.id {
                                "same"
                            } else if source.ignored.as_ref() == Some(&h) {
                                "ignored"
                            } else {
                                "changed"
                            })
                            .to_string(),
                            Some(h),
                            None,
                        ),
                        Err(e) => (
                            (if !source.path.exists() {
                                "missing"
                            } else {
                                "unavailable"
                            })
                            .to_string(),
                            None,
                            Some(e),
                        ),
                    };
                    rows.push(Row {
                        content_id: store.content_id.clone(),
                        book: a.id.clone(),
                        book_name: a.name.clone(),
                        asset_id: page.id.clone(),
                        page: i + 1,
                        name: page.name.clone(),
                        path: source.path.clone(),
                        status,
                        fingerprint,
                        baseline: baseline.clone(),
                        error,
                    });
                }
            }
        }
        self.manifests.retain(|p, _| manifests.contains(p));
        self.files.retain(|p, _| paths.contains(p));
        let updates = rows.iter().filter(|r| r.status == "changed").count();
        let unavailable = rows
            .iter()
            .filter(|r| ["missing", "unavailable"].contains(&r.status.as_str()))
            .count();
        let signature =
            blake3::hash(&serde_json::to_vec(&(&rows, &errors)).map_err(|e| e.to_string())?)
                .to_hex()
                .to_string();
        let changed = signature != self.signature;
        self.signature = signature.clone();
        let last_scan = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let state = Snapshot {
            rows,
            errors: errors.clone(),
            last_scan,
            revision: signature,
        };
        SHARED
            .get_or_init(Default::default)
            .lock()
            .map_err(|_| "原件检查正忙")?
            .insert(data.into(), state);
        Ok((updates, unavailable, changed))
    }
}
