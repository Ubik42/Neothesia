use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use neothesia_core::score_view::{VerovioManifest, VerovioManifestError};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{NeothesiaEvent, song::Song};

const CACHE_VERSION: &str = "verovio-6.1.0-schema2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreRenderGeneration(u64);

#[derive(Debug, Default)]
pub struct ScoreRenderCoordinator {
    current: u64,
    completed: bool,
}

impl ScoreRenderCoordinator {
    pub fn begin_scene(&mut self) -> ScoreRenderGeneration {
        self.current = self.current.wrapping_add(1).max(1);
        self.completed = false;
        ScoreRenderGeneration(self.current)
    }

    pub fn accept(&mut self, generation: ScoreRenderGeneration) -> bool {
        if generation.0 != self.current || self.completed {
            return false;
        }
        self.completed = true;
        true
    }
}

#[derive(Debug, Clone)]
pub struct VerovioWorkerConfig {
    pub node_executable: PathBuf,
    pub worker_script: PathBuf,
    pub package_root: PathBuf,
    pub cache_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RenderedScoreArtifact {
    pub root: PathBuf,
    pub manifest: VerovioManifest,
}

#[derive(Debug, Error)]
pub enum VerovioWorkerError {
    #[error("score fingerprint must be a lowercase SHA-256 value")]
    InvalidSourceHash,
    #[error("score file could not be read: {0}")]
    SourceIo(#[source] std::io::Error),
    #[error("score fingerprint changed before rendering")]
    SourceMismatch,
    #[error("renderer cache could not be prepared: {0}")]
    CacheIo(#[source] std::io::Error),
    #[error("Verovio worker could not be started: {0}")]
    WorkerIo(#[source] std::io::Error),
    #[error("Verovio worker failed: {0}")]
    WorkerFailed(String),
    #[error("renderer manifest could not be read: {0}")]
    ManifestIo(#[source] std::io::Error),
    #[error(transparent)]
    InvalidArtifact(#[from] VerovioManifestError),
}

impl VerovioWorkerConfig {
    pub fn from_environment() -> Option<Self> {
        let package_root = std::env::var_os("NEOTHESIA_VEROVIO_PACKAGE_ROOT")?;
        let node_executable = std::env::var_os("NEOTHESIA_NODE")
            .map(PathBuf::from)
            .unwrap_or_else(|| "node".into());
        let worker_script = std::env::var_os("NEOTHESIA_VEROVIO_WORKER")
            .map(PathBuf::from)
            .unwrap_or_else(|| "scripts/verovio-render-worker.mjs".into());
        let cache_root = std::env::var_os("NEOTHESIA_SCORE_CACHE")
            .map(PathBuf::from)
            .unwrap_or_else(|| "score-render-cache".into());
        Some(Self {
            node_executable,
            worker_script,
            package_root: package_root.into(),
            cache_root,
        })
    }

    pub fn render(
        &self,
        source_path: &Path,
        expected_source_sha256: &str,
    ) -> Result<RenderedScoreArtifact, VerovioWorkerError> {
        if let Some(cached) = self.load_cached(expected_source_sha256)? {
            return Ok(cached);
        }
        let source = fs::read(source_path).map_err(VerovioWorkerError::SourceIo)?;
        if sha256(&source) != expected_source_sha256 {
            return Err(VerovioWorkerError::SourceMismatch);
        }

        fs::create_dir_all(&self.cache_root).map_err(VerovioWorkerError::CacheIo)?;
        let staging = self.staging_path(expected_source_sha256);
        let output = Command::new(&self.node_executable)
            .arg(&self.worker_script)
            .arg("--package-root")
            .arg(&self.package_root)
            .arg("--source")
            .arg(source_path)
            .arg("--output")
            .arg(&staging)
            .output()
            .map_err(VerovioWorkerError::WorkerIo)?;
        if !output.status.success() {
            remove_staging(&staging);
            return Err(VerovioWorkerError::WorkerFailed(truncate_output(
                &output.stderr,
            )));
        }
        let artifact = match load_artifact(&staging, expected_source_sha256) {
            Ok(artifact) => artifact,
            Err(error) => {
                remove_staging(&staging);
                return Err(error);
            }
        };

        let final_root = self.cache_path(expected_source_sha256);
        let parent = final_root.parent().expect("cache path always has a parent");
        fs::create_dir_all(parent).map_err(VerovioWorkerError::CacheIo)?;
        if final_root.exists() {
            let existing = load_artifact(&final_root, expected_source_sha256)?;
            remove_staging(&staging);
            return Ok(existing);
        }
        fs::rename(&staging, &final_root).map_err(|error| {
            remove_staging(&staging);
            VerovioWorkerError::CacheIo(error)
        })?;
        Ok(RenderedScoreArtifact {
            root: final_root,
            manifest: artifact.manifest,
        })
    }

    pub fn load_cached(
        &self,
        source_sha256: &str,
    ) -> Result<Option<RenderedScoreArtifact>, VerovioWorkerError> {
        if !valid_sha256(source_sha256) {
            return Err(VerovioWorkerError::InvalidSourceHash);
        }
        let root = self.cache_path(source_sha256);
        if !root.join("manifest.json").exists() {
            return Ok(None);
        }
        load_artifact(&root, source_sha256).map(Some)
    }

    fn cache_path(&self, source_sha256: &str) -> PathBuf {
        self.cache_root.join(source_sha256).join(CACHE_VERSION)
    }

    fn staging_path(&self, source_sha256: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        self.cache_root.join(format!(
            ".render-{}-{}-{nonce}",
            &source_sha256[..source_sha256.len().min(12)],
            std::process::id()
        ))
    }
}

pub fn start_for_song(
    song: &Song,
    generation: ScoreRenderGeneration,
    proxy: winit::event_loop::EventLoopProxy<NeothesiaEvent>,
) -> bool {
    let Some(config) = VerovioWorkerConfig::from_environment() else {
        return false;
    };
    let Some(midi_path) = song.file.source_path.clone() else {
        return false;
    };
    let sidecar =
        match neothesia_core::library::load_song_sidecar(&midi_path, &song.file.content_id) {
            Ok(sidecar) => sidecar,
            Err(neothesia_core::library::MetadataError::Read(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                return false;
            }
            Err(error) => {
                log::warn!("Could not prepare score rendering: {error}");
                return false;
            }
        };
    let Some(association) = sidecar.score else {
        return false;
    };
    let score_path = neothesia_core::library::resolve_score_path(&midi_path, &association);
    std::thread::Builder::new()
        .name("score-render-worker".into())
        .spawn(move || {
            let result = (|| {
                if !neothesia_core::library::verify_score_association(&midi_path, &association)
                    .map_err(|error| error.to_string())?
                {
                    return Err("paired score content has changed".to_owned());
                }
                let source_hash = source_sha256(&score_path).map_err(|error| error.to_string())?;
                config
                    .render(&score_path, &source_hash)
                    .map_err(|error| error.to_string())
            })();
            let _ = proxy.send_event(NeothesiaEvent::ScoreArtifactReady { generation, result });
        })
        .is_ok()
}

fn load_artifact(
    root: &Path,
    expected_source_sha256: &str,
) -> Result<RenderedScoreArtifact, VerovioWorkerError> {
    let json =
        fs::read_to_string(root.join("manifest.json")).map_err(VerovioWorkerError::ManifestIo)?;
    let manifest = VerovioManifest::parse_and_validate(&json, expected_source_sha256)?;
    manifest.validate_page_files(root)?;
    Ok(RenderedScoreArtifact {
        root: root.to_owned(),
        manifest,
    })
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn source_sha256(path: &Path) -> Result<String, std::io::Error> {
    fs::read(path).map(|bytes| sha256(&bytes))
}

fn valid_sha256(hash: &str) -> bool {
    hash.len() == 64
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        && hash == hash.to_ascii_lowercase()
}

fn truncate_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.chars()
        .take(4_096)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn remove_staging(path: &Path) {
    if path
        .file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with(".render-"))
    {
        let _ = fs::remove_dir_all(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "neothesia-worker-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn config(root: &Path) -> VerovioWorkerConfig {
        VerovioWorkerConfig {
            node_executable: "node-does-not-exist".into(),
            worker_script: root.join("worker.mjs"),
            package_root: root.join("package"),
            cache_root: root.join("cache"),
        }
    }

    fn write_cached_artifact(config: &VerovioWorkerConfig, source_hash: &str, svg: &[u8]) {
        let artifact_root = config.cache_path(source_hash);
        fs::create_dir_all(&artifact_root).unwrap();
        fs::write(artifact_root.join("page-1.svg"), svg).unwrap();
        let manifest = format!(
            r#"{{
  "schemaVersion": 2,
  "rendererName": "verovio",
  "rendererVersion": "6.1.0",
  "sourceSha256": "{source_hash}",
  "sourceBytes": 10,
  "pageCount": 1,
  "pages": [{{
    "pageIndex": 0,
    "svgFile": "page-1.svg",
    "svgBytes": {},
    "svgSha256": "{}"
  }}],
  "notes": []
}}"#,
            svg.len(),
            sha256(svg)
        );
        fs::write(artifact_root.join("manifest.json"), manifest).unwrap();
    }

    #[test]
    fn valid_cache_hit_does_not_start_worker() {
        let root = temp_root();
        let config = config(&root);
        let source_hash = sha256(b"score");
        write_cached_artifact(&config, &source_hash, b"<svg/>");
        let artifact = config
            .render(&root.join("missing.musicxml"), &source_hash)
            .unwrap();
        assert_eq!(artifact.manifest.page_count, 1);
        assert_eq!(artifact.root, config.cache_path(&source_hash));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupted_cache_fails_before_worker_launch() {
        let root = temp_root();
        let config = config(&root);
        let source_hash = sha256(b"score");
        write_cached_artifact(&config, &source_hash, b"<svg/>");
        fs::write(
            config.cache_path(&source_hash).join("page-1.svg"),
            b"<bad/>",
        )
        .unwrap();
        assert!(matches!(
            config.render(&root.join("missing.musicxml"), &source_hash),
            Err(VerovioWorkerError::InvalidArtifact(
                VerovioManifestError::PageHashMismatch(_)
            ))
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_hashes_that_could_escape_the_cache_root() {
        let root = temp_root();
        let config = config(&root);
        assert!(matches!(
            config.render(&root.join("score.musicxml"), "../outside"),
            Err(VerovioWorkerError::InvalidSourceHash)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn coordinator_accepts_only_current_generation_once() {
        let mut coordinator = ScoreRenderCoordinator::default();
        let first = coordinator.begin_scene();
        let second = coordinator.begin_scene();
        let mut scene_artifact = "new-scene-empty";
        if coordinator.accept(first) {
            scene_artifact = "old-song-artifact";
        }
        assert_eq!(scene_artifact, "new-scene-empty");
        if coordinator.accept(second) {
            scene_artifact = "current-song-artifact";
        }
        assert_eq!(scene_artifact, "current-song-artifact");
        assert!(!coordinator.accept(second));
        let third = coordinator.begin_scene();
        assert!(coordinator.accept(third));
    }
}
