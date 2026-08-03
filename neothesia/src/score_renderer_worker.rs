use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use neothesia_core::score_view::{ScorePageRequest, VerovioManifest, VerovioManifestError};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{NeothesiaEvent, song::Song};

const CACHE_VERSION: &str = "verovio-6.1.0-schema2";
const MAX_RASTER_WIDTH: u32 = 1_600;
const MAX_RASTER_HEIGHT: u32 = 2_400;
const MAX_RASTER_PIXELS: f32 = 4_000_000.0;

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

    pub fn is_current(&self, generation: ScoreRenderGeneration) -> bool {
        generation.0 == self.current && self.completed
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
    pub synchronization: Option<ScoreSynchronization>,
}

#[derive(Debug, Clone)]
pub struct ScoreSynchronization {
    pub timeline: neothesia_core::score_view::ScoreHighlightTimeline,
    pub index: neothesia_core::score_view::ScoreRenderIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterizedScorePage {
    pub width: u32,
    pub height: u32,
    pub rgba: bytes::Bytes,
}

impl RenderedScoreArtifact {
    pub fn read_page(&self, page_index: usize) -> Result<Vec<u8>, VerovioManifestError> {
        self.manifest.read_verified_page(&self.root, page_index)
    }
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
            synchronization: None,
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
    let midi = song.file.clone();
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
                let mut artifact = config
                    .render(&score_path, &source_hash)
                    .map_err(|error| error.to_string())?;
                artifact.synchronization = build_synchronization(&score_path, &midi, &artifact);
                Ok(artifact)
            })();
            let _ = proxy.send_event(NeothesiaEvent::ScoreArtifactReady { generation, result });
        })
        .is_ok()
}

fn build_synchronization(
    score_path: &Path,
    midi: &midi_file::MidiFile,
    artifact: &RenderedScoreArtifact,
) -> Option<ScoreSynchronization> {
    let score = match neothesia_core::musicxml::import_musicxml_file(score_path) {
        Ok(score) => score,
        Err(error) => {
            log::warn!("Could not prepare score synchronization: {error}");
            return None;
        }
    };
    let performance = neothesia_core::score_alignment::performance_notes(midi);
    let alignment = neothesia_core::score_alignment::align_score_to_midi(&score, midi);
    let native = neothesia_core::score_view::native_score_note_evidence(&score, &midi.tempo_track);
    let correlation = neothesia_core::score_view::correlate_score_notes(
        native,
        artifact.manifest.renderer_evidence(),
    );
    let index = match neothesia_core::score_view::ScoreRenderIndex::new(
        artifact.manifest.page_count,
        correlation.elements,
    ) {
        Ok(index) => index,
        Err(error) => {
            log::warn!("Could not index engraved score: {error}");
            return None;
        }
    };
    let timeline = neothesia_core::score_view::ScoreHighlightTimeline::from_alignment(
        &alignment,
        &performance,
    );
    Some(ScoreSynchronization { timeline, index })
}

pub fn start_page_loads(
    artifact: &RenderedScoreArtifact,
    generation: ScoreRenderGeneration,
    requests: impl IntoIterator<Item = ScorePageRequest>,
    proxy: winit::event_loop::EventLoopProxy<NeothesiaEvent>,
) {
    for request in requests {
        let artifact = artifact.clone();
        let proxy = proxy.clone();
        let _ = std::thread::Builder::new()
            .name(format!("score-page-{}", request.page_index))
            .spawn(move || {
                let result = artifact
                    .read_page(request.page_index)
                    .map_err(|error| error.to_string())
                    .and_then(|svg| rasterize_svg(&svg));
                let _ = proxy.send_event(NeothesiaEvent::ScorePageReady {
                    generation,
                    request,
                    result,
                });
            });
    }
}

fn rasterize_svg(svg: &[u8]) -> Result<RasterizedScorePage, String> {
    let options = resvg::usvg::Options {
        fontdb: score_font_database(),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_data(svg, &options)
        .map_err(|error| format!("SVG could not be parsed: {error}"))?;
    let source = tree.size();
    let source_width = source.width();
    let source_height = source.height();
    if !source_width.is_finite()
        || !source_height.is_finite()
        || source_width <= 0.0
        || source_height <= 0.0
    {
        return Err("SVG has invalid page dimensions".to_owned());
    }

    let scale = 1.0_f32
        .min(MAX_RASTER_WIDTH as f32 / source_width)
        .min(MAX_RASTER_HEIGHT as f32 / source_height)
        .min((MAX_RASTER_PIXELS / (source_width * source_height)).sqrt());
    let width = (source_width * scale).round().max(1.0) as u32;
    let height = (source_height * scale).round().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "SVG raster surface is too large".to_owned())?;
    pixmap.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(RasterizedScorePage {
        width,
        height,
        rgba: pixmap.take().into(),
    })
}

fn score_font_database() -> Arc<resvg::usvg::fontdb::Database> {
    static FONT_DATABASE: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    FONT_DATABASE
        .get_or_init(|| {
            let mut database = resvg::usvg::fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
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
        synchronization: None,
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
        assert!(coordinator.is_current(second));
        assert!(!coordinator.accept(second));
        let third = coordinator.begin_scene();
        assert!(!coordinator.is_current(second));
        assert!(!coordinator.is_current(third));
        assert!(coordinator.accept(third));
    }

    #[test]
    fn rasterizes_svg_to_bounded_opaque_rgba() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
            <rect width="200" height="100" fill="#c83264"/>
        </svg>"##;
        let page = rasterize_svg(svg).unwrap();
        assert_eq!((page.width, page.height), (200, 100));
        assert_eq!(page.rgba.len(), 200 * 100 * 4);
        assert_eq!(&page.rgba[..4], &[200, 50, 100, 255]);
    }

    #[test]
    fn rasterization_caps_large_pages_without_changing_aspect_ratio() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8000" height="12000"/>"#;
        let page = rasterize_svg(svg).unwrap();
        assert!(page.width <= MAX_RASTER_WIDTH);
        assert!(page.height <= MAX_RASTER_HEIGHT);
        assert!((page.width as usize * page.height as usize) <= MAX_RASTER_PIXELS as usize);
        assert_eq!(page.width * 3, page.height * 2);
    }
}
