mod state;
use bytes::Bytes;
use state::{LibraryView, Page, UiState};

mod midi_picker;
use midi_picker::{locate_saved_midi, open_midi_file_picker, open_saved_midi};

mod exercise;

mod neo_btn;
use neo_btn::{neo_btn, neo_btn_icon};

mod settings;
mod tracks;

use std::{collections::BTreeMap, future::Future, hash::Hash, path::PathBuf, time::Duration};

use crate::utils::{BoxFuture, noop_waker_ref, window::WinitEvent};
use neothesia_core::library::{
    LibraryProvenance, ScoreAnalysisSnapshot, ScoreAssociation, SongMetadata,
    clear_score_association, load_song_sidecar, resolve_score_path, save_score_analysis,
    save_score_association, save_song_metadata, verify_score_association,
};
use neothesia_core::musicxml::import_musicxml_file;
use neothesia_core::practice_history::{ReviewReason, ReviewStatus};
use neothesia_core::render::{BgPipeline, ImageIdentifier, QuadRenderer, TextRenderer};
use neothesia_core::score_alignment::{align_score_to_midi, summarize_alignment};

use winit::{
    event::WindowEvent,
    keyboard::{Key, NamedKey},
};

use crate::{NeothesiaEvent, context::Context, icons, scene::Scene, song::Song};

use super::NuonRenderer;

type MsgFn = Box<dyn FnOnce(&mut UiState, &mut Context)>;

fn on_async<T, Fut, FN>(future: Fut, f: FN) -> BoxFuture<MsgFn>
where
    T: 'static,
    Fut: Future<Output = T> + Send + 'static,
    FN: FnOnce(T, &mut UiState, &mut Context) + Send + 'static,
{
    Box::pin(async {
        let res = future.await;
        let f: MsgFn = Box::new(move |data, ctx| f(res, data, ctx));
        f
    })
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
enum Popup {
    #[default]
    None,
    OutputSelector,
    InputSelector,
}

impl Popup {
    fn toggle(&mut self, new: Self) {
        *self = if *self == new { Self::None } else { new }
    }

    fn close(&mut self) {
        *self = Self::None;
    }
}

const METADATA_FIELDS: [&str; 7] = [
    "Title",
    "Artist / performer",
    "Composer",
    "Collection",
    "Difficulty",
    "Tags (comma separated)",
    "Study notes",
];

#[derive(Debug, Clone)]
struct MetadataEditor {
    content_id: String,
    source_path: PathBuf,
    fields: [String; 7],
    active: usize,
    message: Option<String>,
    score: Option<ScoreAssociation>,
    score_status: String,
    analysis_status: Option<String>,
    provenance: Option<LibraryProvenance>,
}

impl MetadataEditor {
    fn new(
        content_id: String,
        source_path: PathBuf,
        metadata: SongMetadata,
        provenance: Option<LibraryProvenance>,
    ) -> Self {
        let sidecar = load_song_sidecar(&source_path, &content_id).ok();
        let score = sidecar.as_ref().and_then(|sidecar| sidecar.score.clone());
        let analysis_status = sidecar
            .as_ref()
            .and_then(|sidecar| sidecar.score_analysis.as_ref())
            .filter(|analysis| {
                score
                    .as_ref()
                    .is_some_and(|score| score.content_id == analysis.score_content_id)
            })
            .map(|analysis| {
                format!(
                    "{:?} · {}% coverage · {}% confidence",
                    analysis.readiness, analysis.coverage_percent, analysis.mean_confidence_percent
                )
            });
        let score_status = score
            .as_ref()
            .map(|association| {
                let name = resolve_score_path(&source_path, association)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                match verify_score_association(&source_path, association) {
                    Ok(true) => format!("Verified · {name}"),
                    Ok(false) => format!("Changed · {name}"),
                    Err(_) => format!("Missing or invalid · {name}"),
                }
            })
            .unwrap_or_else(|| "No score paired".into());
        Self {
            content_id,
            source_path,
            fields: [
                metadata.title.unwrap_or_default(),
                metadata.artist.unwrap_or_default(),
                metadata.composer.unwrap_or_default(),
                metadata.collection.unwrap_or_default(),
                metadata.difficulty.unwrap_or_default(),
                metadata.tags.join(", "),
                metadata.notes.unwrap_or_default(),
            ],
            active: 0,
            message: None,
            score,
            score_status,
            analysis_status,
            provenance,
        }
    }

    fn metadata(&self) -> SongMetadata {
        SongMetadata {
            title: some_text(&self.fields[0]),
            artist: some_text(&self.fields[1]),
            composer: some_text(&self.fields[2]),
            collection: some_text(&self.fields[3]),
            difficulty: some_text(&self.fields[4]),
            tags: self.fields[5]
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_owned)
                .collect(),
            notes: some_text(&self.fields[6]),
        }
    }

    fn select_relative(&mut self, offset: isize) {
        self.active = self
            .active
            .saturating_add_signed(offset)
            .min(self.fields.len() - 1);
    }
}

fn some_text(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

pub struct MenuScene {
    bg_pipeline: BgPipeline,
    text_renderer: TextRenderer,
    nuon_renderer: NuonRenderer,

    logo: ImageIdentifier,

    state: UiState,

    context: std::task::Context<'static>,
    futures: Vec<BoxFuture<MsgFn>>,

    quad_pipeline: QuadRenderer,
    nuon: nuon::Ui,

    tracks_scroll: nuon::ScrollState,
    library_scroll: nuon::ScrollState,
    settings_scroll: nuon::ScrollState,
    popup: Popup,
    metadata_editor: Option<MetadataEditor>,
}

impl MenuScene {
    pub fn new(ctx: &mut Context, song: Option<Song>) -> Self {
        let iced_state = UiState::new(ctx, song);

        let quad_pipeline = ctx.quad_renderer_factory.new_renderer();
        let text_renderer = ctx.text_renderer_factory.new_renderer();

        let mut nuon_renderer = NuonRenderer::new(ctx);

        let logo = Bytes::from_static(include_bytes!("../../../../assets/banner.png"));
        let logo = nuon_renderer.add_image(neothesia_core::render::Image::new(
            &ctx.gpu.device,
            &ctx.gpu.queue,
            logo,
        ));

        Self {
            bg_pipeline: BgPipeline::new(&ctx.gpu),
            text_renderer,
            state: iced_state,
            nuon_renderer,

            logo,

            context: std::task::Context::from_waker(noop_waker_ref()),
            futures: Vec::new(),

            quad_pipeline,
            nuon: nuon::Ui::new(),
            tracks_scroll: nuon::ScrollState::new(),
            library_scroll: nuon::ScrollState::new(),
            settings_scroll: nuon::ScrollState::new(),
            popup: Popup::None,
            metadata_editor: None,
        }
    }

    pub fn new_settings(ctx: &mut Context, song: Option<Song>) -> Self {
        let mut scene = Self::new(ctx, song);
        scene.state.go_to(Page::Settings);
        scene
    }

    fn main_ui(&mut self, ctx: &mut Context) {
        if self.state.is_loading() {
            let width = ctx.window_state.logical_size.width;
            let height = ctx.window_state.logical_size.height;

            nuon::label()
                .size(width, height)
                .font_size(30.0)
                .text("Loading...")
                .text_justify(nuon::TextJustify::Center)
                .build(&mut self.nuon);
            return;
        }

        let mut nuon = std::mem::replace(&mut self.nuon, nuon::Ui::new());

        match self.state.current() {
            Page::Exit => self.exit_page_ui(ctx, &mut nuon),
            Page::Main => self.main_page_ui(ctx, &mut nuon),
            Page::Settings => self.settings_page_ui(ctx, &mut nuon),
            Page::TrackSelection => self.tracks_page_ui(ctx, &mut nuon),
            Page::Library => self.library_page_ui(ctx, &mut nuon),
            Page::Metadata => self.metadata_page_ui(ctx, &mut nuon),
            Page::Exercises => self.exercise_page_ui(ctx, &mut nuon),
        }

        self.nuon = nuon;
    }

    fn exit_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;

        let btn_w = 320.0;
        let btn_h = 50.0;
        let btn_gap = 5.0;

        let text_h = 80.0;

        let full_w = btn_w * 2.0 + btn_gap;
        let full_h = btn_h + text_h;

        nuon::translate()
            .x(nuon::center_x(win_w, full_w))
            .y(nuon::center_y(win_h, full_h))
            .build(ui, |ui| {
                nuon::label()
                    .text("Do you want to exit?")
                    .font_size(30.0)
                    .size(full_w, text_h)
                    .build(ui);

                nuon::translate().y(text_h).add_to_current(ui);

                if neo_btn().size(btn_w, btn_h).label("No").build(ui) {
                    self.state.go_back();
                }

                nuon::translate().x(btn_w).add_to_current(ui);
                nuon::translate().x(btn_gap).add_to_current(ui);

                if neo_btn().size(btn_w, btn_h).label("Yes").build(ui) {
                    ctx.proxy.send_event(NeothesiaEvent::Exit).ok();
                }
            });
    }

    fn main_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;

        let w = 450.0;
        let h = 60.0;
        let gap = 8.0;

        let logo_w = 650.0;
        let logo_h = 118.0;
        let post_logo_gap = 30.0;

        nuon::translate()
            .x(win_w / 2.0)
            .y(win_h / 5.0)
            .build(ui, |ui| {
                nuon::image(self.logo)
                    .x(-logo_w / 2.0)
                    .size(logo_w, logo_h)
                    .build(ui);

                nuon::translate()
                    .x(-w / 2.0)
                    .y(logo_h + post_logo_gap)
                    .build(ui, |ui| {
                        if neo_btn().size(w, h).label("Select File").build(ui) {
                            self.futures.push(open_midi_file_picker(&mut self.state));
                        }

                        nuon::translate().y(h + gap).add_to_current(ui);

                        if neo_btn()
                            .id(super::playing_scene::practice_ui_ids::MENU_EXERCISES)
                            .size(w, h)
                            .label("Technique Studio")
                            .build(ui)
                        {
                            self.state.go_to(Page::Exercises);
                        }

                        nuon::translate().y(h + gap).add_to_current(ui);

                        if neo_btn()
                            .id(super::playing_scene::practice_ui_ids::MENU_LIBRARY)
                            .size(w, h)
                            .label("Practice Library")
                            .build(ui)
                        {
                            self.state.go_to(Page::Library);
                        }

                        nuon::translate().y(h + gap).add_to_current(ui);

                        if neo_btn().size(w, h).label("Settings").build(ui) {
                            self.state.go_to(Page::Settings);
                        }

                        nuon::translate().y(h + gap).add_to_current(ui);

                        if neo_btn().size(w, h).label("Exit").build(ui) {
                            self.state.go_back();
                        }
                    });
            });

        nuon::translate().x(0.0).y(win_h).build(ui, |ui| {
            let gap = 10.0;
            let btn_w = 80.0;
            let btn_h = 60.0;

            nuon::translate().y(-gap).add_to_current(ui);
            nuon::translate().y(-btn_h).add_to_current(ui);

            if let Some(song) = self.state.song() {
                nuon::label()
                    .text(&song.file.name)
                    .size(win_w, 60.0)
                    .font_size(16.0)
                    .build(ui);
            }

            nuon::translate().build(ui, |ui| {
                nuon::translate().x(gap).add_to_current(ui);

                if neo_btn()
                    .size(btn_w, btn_h)
                    .icon(icons::balloon_icon())
                    .color([100; 3])
                    .tooltip("FreePlay")
                    .build(ui)
                {
                    state::freeplay(&self.state, ctx);
                }
            });

            if self.state.song().is_none() {
                return;
            }

            nuon::translate().x(win_w).build(ui, |ui| {
                nuon::translate().x(-btn_w - gap).add_to_current(ui);

                if neo_btn()
                    .id(super::playing_scene::practice_ui_ids::MENU_START)
                    .size(btn_w, btn_h)
                    .icon(icons::play_icon())
                    .tooltip("Play")
                    .build(ui)
                {
                    state::play(&self.state, ctx);
                }

                nuon::translate().x(-btn_w - gap).add_to_current(ui);

                if neo_btn()
                    .size(btn_w, btn_h)
                    .icon(icons::note_list_icon())
                    .tooltip("Tracks")
                    .build(ui)
                {
                    self.state.go_to(Page::TrackSelection);
                }
            });
        });
    }

    fn library_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        if self.state.library_index.is_none() && !self.state.library_scanning {
            self.start_library_scan(ctx.config.watched_folders().to_vec());
        }

        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;
        let row_w = (win_w - 80.0).clamp(420.0, 760.0);
        let query = self.state.library_query.trim().to_owned();
        let mut rows = BTreeMap::<String, LibraryRow>::new();
        let history_songs = ctx.practice_history.recent_songs(usize::MAX);
        let queued_count = history_songs
            .iter()
            .filter(|song| song.queue_position.is_some())
            .count();
        let due_count = history_songs
            .iter()
            .filter(|song| song.review.is_some_and(|review| review.is_due))
            .count();
        for song in history_songs {
            rows.insert(
                song.content_id.clone(),
                LibraryRow {
                    content_id: song.content_id,
                    display_name: song.display_name,
                    metadata: SongMetadata::default(),
                    provenance: None,
                    source_path: song.source_path,
                    exercise_spec: song.exercise_spec,
                    session_count: song.session_count,
                    latest_accuracy: song.latest_accuracy,
                    last_used_unix_ms: song.last_used_unix_ms,
                    favorite: song.favorite,
                    queue_position: song.queue_position,
                    recommended_measures: song.recommended_measures,
                    review: song.review,
                },
            );
        }
        if let Some(index) = &self.state.library_index {
            for song in &index.songs {
                let available_path = song
                    .source_paths
                    .iter()
                    .find(|path| path.is_file())
                    .cloned();
                rows.entry(song.content_id.clone())
                    .and_modify(|row| {
                        if available_path.is_some() {
                            row.source_path = available_path.clone();
                        }
                        if song.metadata.title.is_some() || song.provenance.is_some() {
                            row.display_name = song.display_name.clone();
                        }
                        row.metadata = song.metadata.clone();
                        row.provenance = song.provenance.clone();
                    })
                    .or_insert_with(|| LibraryRow {
                        content_id: song.content_id.clone(),
                        display_name: song.display_name.clone(),
                        metadata: song.metadata.clone(),
                        provenance: song.provenance.clone(),
                        source_path: available_path,
                        exercise_spec: None,
                        session_count: 0,
                        latest_accuracy: None,
                        last_used_unix_ms: 0,
                        favorite: false,
                        queue_position: None,
                        recommended_measures: None,
                        review: None,
                    });
            }
        }
        rows.retain(|_, song| library_row_matches(&query, song));
        let mut rows: Vec<_> = rows.into_values().collect();
        match self.state.library_view {
            LibraryView::Queue => {
                rows.retain(|song| song.queue_position.is_some());
                rows.sort_by(|left, right| left.queue_position.cmp(&right.queue_position));
            }
            LibraryView::Due => {
                rows.retain(|song| song.review.is_some_and(|review| review.is_due));
                rows.sort_by(|left, right| {
                    left.review
                        .map(|review| review.due_at_unix_ms)
                        .cmp(&right.review.map(|review| review.due_at_unix_ms))
                        .then_with(|| left.display_name.cmp(&right.display_name))
                });
            }
            LibraryView::All => {
                rows.sort_by(|left, right| {
                    right
                        .favorite
                        .cmp(&left.favorite)
                        .then_with(|| right.last_used_unix_ms.cmp(&left.last_used_unix_ms))
                        .then_with(|| left.display_name.cmp(&right.display_name))
                });
            }
        }

        nuon::label()
            .x(30.0)
            .y(18.0)
            .size((win_w - 330.0).max(220.0), 46.0)
            .font_size(30.0)
            .bold(true)
            .text("Practice Library")
            .build(ui);
        if nuon::button()
            .x(win_w - 280.0)
            .y(22.0)
            .size(120.0, 34.0)
            .label("Add Folder")
            .color([74, 68, 88])
            .hover_color([109, 78, 164])
            .preseed_color([132, 96, 191])
            .border_radius([6.0; 4])
            .build(ui)
        {
            self.choose_library_folder(ctx.config.watched_folders().to_vec());
        }
        if nuon::button()
            .x(win_w - 150.0)
            .y(22.0)
            .size(120.0, 34.0)
            .label("Refresh")
            .color([74, 68, 88])
            .hover_color([87, 81, 101])
            .preseed_color([109, 78, 164])
            .border_radius([6.0; 4])
            .build(ui)
        {
            self.start_library_scan(ctx.config.watched_folders().to_vec());
        }
        nuon::label()
            .x(30.0)
            .y(60.0)
            .size(win_w - 240.0, 28.0)
            .font_size(15.0)
            .color([178, 175, 190])
            .text(format!(
                "Search: {}  ·  type anywhere{}",
                if query.is_empty() {
                    "all pieces"
                } else {
                    query.as_str()
                },
                if self.state.library_scanning {
                    "  ·  indexing…"
                } else {
                    ""
                }
            ))
            .build(ui);
        if nuon::button()
            .x(win_w - 210.0)
            .y(62.0)
            .size(86.0, 28.0)
            .label(if self.state.library_view == LibraryView::Queue {
                "All".to_owned()
            } else {
                format!("Queue {queued_count}")
            })
            .color(if self.state.library_view == LibraryView::Queue {
                [109, 78, 164]
            } else {
                [74, 68, 88]
            })
            .hover_color([132, 96, 191])
            .preseed_color([144, 108, 203])
            .border_radius([6.0; 4])
            .build(ui)
        {
            self.state.library_view = if self.state.library_view == LibraryView::Queue {
                LibraryView::All
            } else {
                LibraryView::Queue
            };
            self.library_scroll = nuon::ScrollState::new();
        }
        if nuon::button()
            .x(win_w - 116.0)
            .y(62.0)
            .size(86.0, 28.0)
            .label(if self.state.library_view == LibraryView::Due {
                "All".to_owned()
            } else {
                format!("Due {due_count}")
            })
            .color(if self.state.library_view == LibraryView::Due {
                [150, 83, 71]
            } else {
                [74, 68, 88]
            })
            .hover_color([174, 99, 84])
            .preseed_color([194, 112, 95])
            .border_radius([6.0; 4])
            .build(ui)
        {
            self.state.library_view = if self.state.library_view == LibraryView::Due {
                LibraryView::All
            } else {
                LibraryView::Due
            };
            self.library_scroll = nuon::ScrollState::new();
        }

        if let Some(message) = self.state.library_message.as_deref() {
            nuon::label()
                .x(30.0)
                .y(90.0)
                .size(win_w - 60.0, 34.0)
                .font_size(15.0)
                .color([255, 205, 124])
                .text(message)
                .build(ui);
        }

        if rows.is_empty() {
            nuon::label()
                .x(30.0)
                .y(140.0)
                .size(win_w - 60.0, 40.0)
                .font_size(20.0)
                .text(if query.is_empty() {
                    "No pieces yet. Add a MIDI folder or open a file to begin."
                } else {
                    "No pieces match this search."
                })
                .build(ui);
        } else {
            nuon::translate().y(126.0).build(ui, |ui| {
                self.library_scroll = nuon::scroll()
                    .scissor_size(win_w, (win_h - 206.0).max(0.0))
                    .scroll(self.library_scroll)
                    .build(ui, |ui| {
                        for (index, song) in rows.iter().enumerate() {
                            let path_available = song
                                .source_path
                                .as_deref()
                                .is_some_and(|path| path.is_file());
                            let generated_available = song.exercise_spec.is_some();
                            let available = path_available || generated_available;
                            let accuracy = song
                                .latest_accuracy
                                .map(|value| format!(" · latest {}%", (value * 100.0).round()))
                                .unwrap_or_default();
                            let action = if generated_available {
                                "Practice"
                            } else if path_available {
                                "Open"
                            } else {
                                "Locate"
                            };
                            let recommendation = song
                                .recommended_measures
                                .map(|(start, end)| {
                                    if start == end {
                                        format!(" · weak M{start}")
                                    } else {
                                        format!(" · weak M{start}-{end}")
                                    }
                                })
                                .unwrap_or_default();
                            let review = song.review.map(format_review_status).unwrap_or_default();
                            let credit = metadata_credit(&song.metadata)
                                .or_else(|| song.provenance.as_ref().and_then(provenance_credit))
                                .map(|credit| format!(" · {}", truncate_menu_label(&credit, 18)))
                                .unwrap_or_default();
                            let provenance = song
                                .provenance
                                .as_ref()
                                .map(|value| {
                                    format!(
                                        " · {}",
                                        truncate_menu_label(&provenance_summary(value), 34)
                                    )
                                })
                                .unwrap_or_default();
                            let label = format!(
                                "{}{}{}  ·  {} session{}{}{}{}  ·  {}",
                                truncate_menu_label(&song.display_name, 34),
                                credit,
                                provenance,
                                song.session_count,
                                if song.session_count == 1 { "" } else { "s" },
                                accuracy,
                                recommendation,
                                review,
                                action
                            );
                            let row_x = nuon::center_x(win_w, row_w);
                            let reorder_width = if self.state.library_view == LibraryView::Queue {
                                90.0
                            } else {
                                0.0
                            };
                            let open_width = row_w - 198.0 - reorder_width;
                            if nuon::button()
                                .id(nuon::Id::hash_with(|hasher| {
                                    "library-song".hash(hasher);
                                    song.content_id.hash(hasher);
                                }))
                                .x(row_x)
                                .y(index as f32 * 62.0)
                                .size(open_width, 52.0)
                                .label(label)
                                .color(if available {
                                    [48, 91, 82]
                                } else {
                                    [91, 72, 52]
                                })
                                .hover_color(if available {
                                    [57, 112, 99]
                                } else {
                                    [112, 88, 60]
                                })
                                .preseed_color([109, 78, 164])
                                .border_radius([7.0; 4])
                                .build(ui)
                            {
                                if let Some(spec) = song.exercise_spec {
                                    self.open_exercise(ctx, spec);
                                } else if let Some(path) =
                                    song.source_path.clone().filter(|p| p.is_file())
                                {
                                    self.futures.push(open_saved_midi(
                                        &mut self.state,
                                        path,
                                        song.content_id.clone(),
                                    ));
                                } else {
                                    self.futures.push(locate_saved_midi(
                                        &mut self.state,
                                        song.content_id.clone(),
                                    ));
                                }
                            }
                            if nuon::button()
                                .id(nuon::Id::hash_with(|hasher| {
                                    "library-metadata".hash(hasher);
                                    song.content_id.hash(hasher);
                                }))
                                .x(row_x + open_width + 6.0)
                                .y(index as f32 * 62.0)
                                .size(50.0, 52.0)
                                .label(if path_available { "Info" } else { "—" })
                                .color(if path_available {
                                    [65, 62, 73]
                                } else {
                                    [45, 43, 50]
                                })
                                .hover_color(if path_available {
                                    [109, 78, 164]
                                } else {
                                    [45, 43, 50]
                                })
                                .preseed_color([132, 96, 191])
                                .border_radius([7.0; 4])
                                .build(ui)
                                && let Some(path) =
                                    song.source_path.clone().filter(|path| path.is_file())
                            {
                                self.metadata_editor = Some(MetadataEditor::new(
                                    song.content_id.clone(),
                                    path,
                                    song.metadata.clone(),
                                    song.provenance.clone(),
                                ));
                                self.state.go_to(Page::Metadata);
                            }
                            if nuon::button()
                                .id(nuon::Id::hash_with(|hasher| {
                                    "library-favorite".hash(hasher);
                                    song.content_id.hash(hasher);
                                }))
                                .x(row_x + open_width + 62.0)
                                .y(index as f32 * 62.0)
                                .size(60.0, 52.0)
                                .label(if song.favorite { "Fav ✓" } else { "Fav" })
                                .color(if song.favorite {
                                    [130, 91, 42]
                                } else {
                                    [65, 62, 73]
                                })
                                .hover_color([160, 111, 52])
                                .preseed_color([180, 125, 60])
                                .border_radius([7.0; 4])
                                .build(ui)
                            {
                                match ctx.practice_history.set_favorite(
                                    &song.content_id,
                                    &song.display_name,
                                    song.source_path.clone(),
                                    !song.favorite,
                                ) {
                                    Ok(()) => {}
                                    Err(error) => {
                                        self.state.library_message =
                                            Some(format!("Could not save favorite: {error}"));
                                    }
                                }
                            }
                            if nuon::button()
                                .id(nuon::Id::hash_with(|hasher| {
                                    "library-queue".hash(hasher);
                                    song.content_id.hash(hasher);
                                }))
                                .x(row_x + open_width + 128.0)
                                .y(index as f32 * 62.0)
                                .size(70.0, 52.0)
                                .label(if song.queue_position.is_some() {
                                    "Remove"
                                } else {
                                    "Queue"
                                })
                                .color(if song.queue_position.is_some() {
                                    [69, 105, 128]
                                } else {
                                    [65, 62, 73]
                                })
                                .hover_color([82, 128, 156])
                                .preseed_color([94, 145, 176])
                                .border_radius([7.0; 4])
                                .build(ui)
                            {
                                match ctx.practice_history.set_queued(
                                    &song.content_id,
                                    &song.display_name,
                                    song.source_path.clone(),
                                    song.queue_position.is_none(),
                                ) {
                                    Ok(()) => {}
                                    Err(error) => {
                                        self.state.library_message =
                                            Some(format!("Could not update queue: {error}"));
                                    }
                                }
                            }
                            if self.state.library_view == LibraryView::Queue {
                                for (offset, direction, label) in
                                    [(204.0, -1, "↑"), (248.0, 1, "↓")]
                                {
                                    if nuon::button()
                                        .id(nuon::Id::hash_with(|hasher| {
                                            "library-queue-move".hash(hasher);
                                            song.content_id.hash(hasher);
                                            direction.hash(hasher);
                                        }))
                                        .x(row_x + open_width + offset)
                                        .y(index as f32 * 62.0)
                                        .size(40.0, 52.0)
                                        .label(label)
                                        .color([65, 62, 73])
                                        .hover_color([109, 78, 164])
                                        .preseed_color([132, 96, 191])
                                        .border_radius([7.0; 4])
                                        .build(ui)
                                    {
                                        match ctx
                                            .practice_history
                                            .move_in_queue(&song.content_id, direction)
                                        {
                                            Ok(_) => {}
                                            Err(error) => {
                                                self.state.library_message = Some(format!(
                                                    "Could not reorder queue: {error}"
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
            });
        }

        nuon::translate().x(10.0).y(win_h - 70.0).build(ui, |ui| {
            if neo_btn_icon(ui, 80.0, 60.0, icons::left_arrow_icon()) {
                self.state.go_back();
            }
        });
    }

    fn metadata_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;
        let form_w = (win_w - 80.0).clamp(480.0, 760.0);
        let form_x = nuon::center_x(win_w, form_w);
        let mut save = false;
        let mut cancel = false;
        let mut pair_score = false;
        let mut remove_score = false;
        let mut analyze_score = false;

        let Some(editor) = self.metadata_editor.as_mut() else {
            self.state.go_back();
            return;
        };

        nuon::label()
            .x(form_x)
            .y(18.0)
            .size(form_w, 42.0)
            .font_size(30.0)
            .bold(true)
            .text("Song information")
            .build(ui);
        nuon::label()
            .x(form_x)
            .y(58.0)
            .size(form_w, 24.0)
            .font_size(14.0)
            .color([178, 175, 190])
            .text({
                let filename = truncate_menu_label(
                    &editor
                        .source_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    40,
                );
                let provenance = editor
                    .provenance
                    .as_ref()
                    .map(|value| format!(" · {}", provenance_summary(value)))
                    .unwrap_or_default();
                truncate_menu_label(
                    &format!(
                        "{filename}{provenance} · click a field, then type · Tab moves · Ctrl+S saves"
                    ),
                    112,
                )
            })
            .build(ui);

        let row_h = 58.0;
        for (index, label) in METADATA_FIELDS.iter().enumerate() {
            let y = 92.0 + index as f32 * row_h;
            nuon::label()
                .x(form_x)
                .y(y)
                .size(174.0, 44.0)
                .font_size(15.0)
                .color(if editor.active == index {
                    [255, 214, 128]
                } else {
                    [194, 191, 204]
                })
                .text(*label)
                .build(ui);
            let value = if editor.fields[index].is_empty() {
                "(empty)".to_owned()
            } else {
                truncate_menu_label(&editor.fields[index], 68)
            };
            if nuon::button()
                .id(nuon::Id::hash_with(|hasher| {
                    "metadata-field".hash(hasher);
                    index.hash(hasher);
                }))
                .x(form_x + 180.0)
                .y(y)
                .size(form_w - 180.0, 44.0)
                .label(value)
                .color(if editor.active == index {
                    [83, 62, 112]
                } else {
                    [55, 52, 64]
                })
                .hover_color([98, 73, 132])
                .preseed_color([118, 86, 157])
                .border_radius([6.0; 4])
                .build(ui)
            {
                editor.active = index;
                editor.message = None;
            }
        }

        let score_y = 92.0 + METADATA_FIELDS.len() as f32 * row_h;
        nuon::label()
            .x(form_x)
            .y(score_y)
            .size(174.0, 44.0)
            .font_size(15.0)
            .color([194, 191, 204])
            .text("MusicXML score")
            .build(ui);
        if nuon::button()
            .x(form_x + 180.0)
            .y(score_y)
            .size(form_w - 400.0, 44.0)
            .label(truncate_menu_label(
                editor
                    .analysis_status
                    .as_deref()
                    .unwrap_or(&editor.score_status),
                42,
            ))
            .color([55, 52, 64])
            .hover_color([98, 73, 132])
            .preseed_color([118, 86, 157])
            .border_radius([6.0; 4])
            .build(ui)
        {
            pair_score = true;
        }
        if nuon::button()
            .x(form_x + form_w - 214.0)
            .y(score_y)
            .size(94.0, 44.0)
            .label("Analyze")
            .color([65, 62, 73])
            .hover_color([109, 78, 164])
            .preseed_color([132, 96, 191])
            .border_radius([6.0; 4])
            .build(ui)
            && editor.score.is_some()
        {
            analyze_score = true;
        }
        if nuon::button()
            .x(form_x + form_w - 114.0)
            .y(score_y)
            .size(114.0, 44.0)
            .label(if editor.score.is_some() {
                "Remove"
            } else {
                "Pair…"
            })
            .color(if editor.score.is_some() {
                [112, 62, 58]
            } else {
                [48, 91, 82]
            })
            .hover_color([109, 78, 164])
            .preseed_color([132, 96, 191])
            .border_radius([6.0; 4])
            .build(ui)
        {
            if editor.score.is_some() {
                remove_score = true;
            } else {
                pair_score = true;
            }
        }

        if let Some(message) = editor.message.as_deref() {
            nuon::label()
                .x(form_x)
                .y(win_h - 108.0)
                .size(form_w, 26.0)
                .font_size(14.0)
                .color([255, 167, 142])
                .text(message)
                .build(ui);
        }

        if nuon::button()
            .x(form_x)
            .y(win_h - 68.0)
            .size(150.0, 44.0)
            .label("Cancel")
            .color([65, 62, 73])
            .hover_color([87, 81, 101])
            .preseed_color([109, 78, 164])
            .border_radius([7.0; 4])
            .build(ui)
        {
            cancel = true;
        }
        if nuon::button()
            .x(form_x + form_w - 190.0)
            .y(win_h - 68.0)
            .size(190.0, 44.0)
            .label("Save information")
            .color([48, 91, 82])
            .hover_color([57, 112, 99])
            .preseed_color([70, 132, 116])
            .border_radius([7.0; 4])
            .build(ui)
        {
            save = true;
        }

        if analyze_score {
            self.futures.push(analyze_paired_score(
                &mut self.state,
                editor.source_path.clone(),
                editor.content_id.clone(),
            ));
        } else if pair_score {
            self.futures.push(pair_musicxml_score(
                &mut self.state,
                editor.source_path.clone(),
                editor.content_id.clone(),
            ));
        } else if remove_score {
            match clear_score_association(&editor.source_path, &editor.content_id) {
                Ok(_) => {
                    editor.score = None;
                    editor.score_status = "No score paired".into();
                    editor.analysis_status = None;
                    editor.message = Some("Removed paired score; metadata was preserved.".into());
                    self.state.library_index = None;
                }
                Err(error) => {
                    editor.message = Some(format!("Could not remove paired score: {error}"));
                }
            }
        } else if cancel {
            self.metadata_editor = None;
            self.state.go_back();
        } else if save {
            self.save_metadata_editor();
        }
    }

    fn save_metadata_editor(&mut self) -> bool {
        let Some(editor) = self.metadata_editor.as_ref() else {
            return false;
        };
        match save_song_metadata(&editor.source_path, &editor.content_id, editor.metadata()) {
            Ok(path) => {
                let filename = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                self.metadata_editor = None;
                self.state.library_index = None;
                self.state.library_message = Some(format!(
                    "Saved {filename}; refreshing title, credits and search."
                ));
                self.state.go_back();
                true
            }
            Err(error) => {
                if let Some(editor) = self.metadata_editor.as_mut() {
                    editor.message = Some(format!("Could not save song information: {error}"));
                }
                false
            }
        }
    }

    fn start_library_scan(&mut self, roots: Vec<PathBuf>) {
        if self.state.library_scanning {
            return;
        }
        self.state.library_scanning = true;
        self.state.library_message = None;
        self.futures
            .push(on_async(scan_library(roots), |index, state, _ctx| {
                let unique = index.songs.len();
                let seen = index.midi_files_seen;
                let unreadable = index.unreadable_files;
                let metadata = index.metadata_files_seen;
                let invalid_metadata = index.invalid_metadata_files;
                let catalogs = index.catalog_files_seen;
                let catalog_entries = index.catalog_entries_loaded;
                let invalid_catalog_entries = index.invalid_catalog_entries;
                state.library_index = Some(index);
                state.library_scanning = false;
                state.library_message = Some(format!(
                    "Indexed {unique} unique piece{} from {seen} MIDI file{}; \
                     loaded {metadata} metadata sidecar{} and {catalog_entries} catalog entr{} \
                     from {catalogs} catalog{}{}{}{}.",
                    if unique == 1 { "" } else { "s" },
                    if seen == 1 { "" } else { "s" },
                    if metadata == 1 { "" } else { "s" },
                    if catalog_entries == 1 { "y" } else { "ies" },
                    if catalogs == 1 { "" } else { "s" },
                    if unreadable == 0 {
                        String::new()
                    } else {
                        format!("; skipped {unreadable} unreadable MIDI")
                    },
                    if invalid_metadata == 0 {
                        String::new()
                    } else {
                        format!("; ignored {invalid_metadata} invalid metadata")
                    },
                    if invalid_catalog_entries == 0 {
                        String::new()
                    } else {
                        format!("; ignored {invalid_catalog_entries} invalid catalog entries")
                    }
                ));
            }));
    }

    fn choose_library_folder(&mut self, roots: Vec<PathBuf>) {
        if self.state.library_scanning {
            return;
        }
        self.state.library_scanning = true;
        self.futures.push(on_async(
            choose_library_folder(),
            move |folder, state, ctx| {
                state.library_scanning = false;
                let Some(folder) = folder else {
                    return;
                };
                if roots.iter().any(|root| root == &folder)
                    || !ctx.config.add_watched_folder(folder)
                {
                    state.library_message = Some("That folder is already watched.".into());
                    return;
                }
                ctx.config.save();
                state.library_index = None;
                state.library_message = Some("Folder added; indexing will start now.".into());
            },
        ));
    }
}

#[derive(Debug, Clone)]
struct LibraryRow {
    content_id: String,
    display_name: String,
    metadata: SongMetadata,
    provenance: Option<LibraryProvenance>,
    source_path: Option<PathBuf>,
    exercise_spec: Option<neothesia_core::exercise::ExerciseSpec>,
    session_count: usize,
    latest_accuracy: Option<f32>,
    last_used_unix_ms: u64,
    favorite: bool,
    queue_position: Option<usize>,
    recommended_measures: Option<(usize, usize)>,
    review: Option<ReviewStatus>,
}

async fn scan_library(roots: Vec<PathBuf>) -> neothesia_core::library::LibraryIndex {
    crate::utils::task::thread::spawn("midi-library-index".into(), move || {
        neothesia_core::library::LibraryIndex::scan(&roots)
    })
    .join()
    .await
    .unwrap_or_default()
}

fn pair_musicxml_score(
    data: &mut UiState,
    midi_path: PathBuf,
    content_id: String,
) -> BoxFuture<MsgFn> {
    data.is_loading = true;
    on_async(
        async move {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("MusicXML score", &["musicxml", "xml", "mxl"])
                .pick_file()
                .await;
            let Some(file) = file else {
                return Ok(None);
            };
            let score_path = file.path().to_path_buf();
            crate::utils::task::thread::spawn("score-association".into(), move || {
                save_score_association(&midi_path, &content_id, &score_path)
                    .map(|_| score_path)
                    .map_err(|error| error.to_string())
            })
            .join()
            .await
            .map_err(|_| "Score association task failed.".to_owned())?
            .map(Some)
        },
        |result: Result<Option<PathBuf>, String>, data, _ctx| {
            data.is_loading = false;
            match result {
                Ok(Some(path)) => {
                    data.library_index = None;
                    data.library_message = Some(format!(
                        "Paired score {}; reopen Info to verify or replace it.",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                    data.go_back();
                }
                Ok(None) => {}
                Err(error) => {
                    data.library_message = Some(format!("Could not pair score: {error}"));
                    data.go_back();
                }
            }
        },
    )
}

fn analyze_paired_score(
    data: &mut UiState,
    midi_path: PathBuf,
    content_id: String,
) -> BoxFuture<MsgFn> {
    data.is_loading = true;
    on_async(
        async move {
            crate::utils::task::thread::spawn(
                "score-alignment".into(),
                move || -> Result<ScoreAnalysisSnapshot, String> {
                    let sidecar = load_song_sidecar(&midi_path, &content_id)
                        .map_err(|error| error.to_string())?;
                    let association = sidecar
                        .score
                        .ok_or_else(|| "No score is paired with this MIDI.".to_owned())?;
                    if !verify_score_association(&midi_path, &association)
                        .map_err(|error| error.to_string())?
                    {
                        return Err("The paired score changed; replace it before analysis.".into());
                    }
                    let score = import_musicxml_file(resolve_score_path(&midi_path, &association))
                        .map_err(|error| error.to_string())?;
                    let midi =
                        midi_file::MidiFile::new(&midi_path).map_err(|error| error.to_string())?;
                    let compatibility = summarize_alignment(&align_score_to_midi(&score, &midi));
                    let snapshot = ScoreAnalysisSnapshot {
                        score_content_id: association.content_id,
                        readiness: compatibility.readiness,
                        matched_notes: compatibility.matched_notes,
                        unmatched_score_notes: compatibility.unmatched_score_notes,
                        unmatched_midi_notes: compatibility.unmatched_midi_notes,
                        inexact_projection_matches: compatibility.inexact_projection_matches,
                        coverage_percent: compatibility.coverage_percent,
                        mean_confidence_percent: compatibility.mean_confidence_percent,
                        navigation_diagnostics: compatibility.navigation_diagnostics,
                    };
                    save_score_analysis(&midi_path, &content_id, snapshot.clone())
                        .map_err(|error| error.to_string())?;
                    Ok(snapshot)
                },
            )
            .join()
            .await
        },
        |result, data, _ctx| {
            data.is_loading = false;
            data.go_back();
            match result {
                Ok(Ok(snapshot)) => {
                    data.library_index = None;
                    data.library_message = Some(format!(
                        "Score alignment: {:?}, {}% coverage, {}% confidence.",
                        snapshot.readiness,
                        snapshot.coverage_percent,
                        snapshot.mean_confidence_percent
                    ));
                }
                Ok(Err(error)) => {
                    data.library_message = Some(format!("Could not analyze score: {error}"));
                }
                Err(_) => {
                    data.library_message = Some("Score analysis task failed.".into());
                }
            }
        },
    )
}

async fn choose_library_folder() -> Option<PathBuf> {
    let folder = rfd::AsyncFileDialog::new()
        .set_title("Add Piano MIDI Folder")
        .pick_folder()
        .await?;
    Some(folder.path().to_path_buf())
}

fn library_row_matches(query: &str, song: &LibraryRow) -> bool {
    let terms: Vec<_> = query
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|term| !term.is_empty())
        .collect();
    if terms.is_empty() {
        return true;
    }
    let metadata = &song.metadata;
    let searchable = [
        song.display_name.to_lowercase(),
        metadata
            .artist
            .as_deref()
            .unwrap_or_default()
            .to_lowercase(),
        metadata
            .composer
            .as_deref()
            .unwrap_or_default()
            .to_lowercase(),
        metadata
            .collection
            .as_deref()
            .unwrap_or_default()
            .to_lowercase(),
        metadata
            .difficulty
            .as_deref()
            .unwrap_or_default()
            .to_lowercase(),
        metadata.tags.join(" ").to_lowercase(),
        metadata.notes.as_deref().unwrap_or_default().to_lowercase(),
        song.provenance
            .as_ref()
            .map(provenance_summary)
            .unwrap_or_default()
            .to_lowercase(),
        song.source_path
            .as_deref()
            .map(|path| path.to_string_lossy().to_lowercase())
            .unwrap_or_default(),
    ]
    .join(" ");
    terms.iter().all(|term| searchable.contains(term))
}

fn metadata_credit(metadata: &SongMetadata) -> Option<String> {
    metadata
        .artist
        .as_deref()
        .or(metadata.composer.as_deref())
        .map(str::to_owned)
}

fn provenance_credit(provenance: &LibraryProvenance) -> Option<String> {
    (!provenance.composer.trim().is_empty()).then(|| provenance.composer.clone())
}

fn provenance_summary(provenance: &LibraryProvenance) -> String {
    format!(
        "{} · {} · {}",
        provenance.category, provenance.source, provenance.license
    )
}

fn format_review_status(review: ReviewStatus) -> String {
    if review.is_due {
        return match review.reason {
            ReviewReason::NeedsEvidence => " · due: needs measured take".to_owned(),
            ReviewReason::NeedsReinforcement => " · due: reinforce".to_owned(),
            ReviewReason::FirstMastery | ReviewReason::Consolidating | ReviewReason::Stable => {
                " · due: retention check".to_owned()
            }
        };
    }
    format!(
        " · review in {}d ({} mastered)",
        review.days_until_due, review.mastery_streak
    )
}

fn truncate_menu_label(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label.to_owned();
    }
    let mut shortened: String = label.chars().take(max_chars.saturating_sub(1)).collect();
    shortened.push('…');
    shortened
}

impl Scene for MenuScene {
    #[profiling::function]
    fn update(&mut self, ctx: &mut Context, delta: Duration) {
        self.quad_pipeline.clear();
        self.bg_pipeline.update_time(delta);
        self.state.tick(ctx);

        self.futures
            .retain_mut(|f| match f.as_mut().poll(&mut self.context) {
                std::task::Poll::Ready(msg) => {
                    msg(&mut self.state, ctx);
                    false
                }
                std::task::Poll::Pending => true,
            });

        self.state.tick(ctx);

        self.main_ui(ctx);

        super::render_nuon(&mut self.nuon, &mut self.nuon_renderer, ctx);

        self.text_renderer.update(
            ctx.window_state.physical_size,
            ctx.window_state.scale_factor as f32,
        );
        self.quad_pipeline.prepare();
    }

    #[profiling::function]
    fn render<'pass>(&'pass mut self, rpass: &mut wgpu_jumpstart::RenderPass<'pass>) {
        self.bg_pipeline.render(rpass);
        self.quad_pipeline.render(rpass);
        self.text_renderer.render(rpass);
        self.nuon_renderer.render(rpass);
    }

    fn window_event(&mut self, ctx: &mut Context, event: &WindowEvent) {
        if let WindowEvent::MouseWheel { delta, .. } = event {
            match delta {
                winit::event::MouseScrollDelta::LineDelta(_, y) => {
                    let y = y * 60.0;
                    self.settings_scroll.update(y);
                    self.tracks_scroll.update(y);
                    self.library_scroll.update(y);
                }
                winit::event::MouseScrollDelta::PixelDelta(position) => {
                    self.settings_scroll.update(position.y as f32);
                    self.tracks_scroll.update(position.y as f32);
                    self.library_scroll.update(position.y as f32);
                }
            }
        }

        if event.cursor_moved() {
            self.nuon.mouse_move(
                ctx.window_state.cursor_logical_position.x,
                ctx.window_state.cursor_logical_position.y,
            );
        } else if event.left_mouse_pressed() {
            self.nuon.mouse_down();
        } else if event.left_mouse_released() {
            self.nuon.mouse_up();
        } else if event.back_mouse_pressed() {
            if *self.state.current() == Page::Metadata {
                self.metadata_editor = None;
            }
            self.state.go_back();
        }

        match self.state.current() {
            Page::Exit => {
                if event.key_pressed(Key::Named(NamedKey::Enter)) {
                    ctx.proxy.send_event(NeothesiaEvent::Exit).unwrap();
                }

                if event.key_pressed(Key::Named(NamedKey::Escape)) {
                    self.state.go_back();
                }
            }
            Page::Main => {
                if event.key_pressed(Key::Named(NamedKey::Tab)) {
                    self.futures.push(open_midi_file_picker(&mut self.state));
                }

                if event.key_pressed(Key::Named(NamedKey::Enter)) {
                    state::play(&self.state, ctx)
                }

                if event.key_pressed(Key::Named(NamedKey::Escape)) {
                    self.state.go_back();
                }

                if event.key_pressed(Key::Character("s")) {
                    self.state.go_to(Page::Settings);
                }

                if event.key_pressed(Key::Character("t")) {
                    self.state.go_to(Page::TrackSelection);
                }

                if event.key_pressed(Key::Character("f")) {
                    state::freeplay(&self.state, ctx);
                }

                if event.key_pressed(Key::Character("l")) {
                    self.state.go_to(Page::Library);
                }
            }
            Page::Settings => {
                if event.key_pressed(Key::Named(NamedKey::Escape)) {
                    self.state.go_back();
                }
            }
            Page::TrackSelection => {
                if event.key_pressed(Key::Named(NamedKey::Enter)) {
                    state::play(&self.state, ctx);
                }

                if event.key_pressed(Key::Named(NamedKey::Escape)) {
                    self.state.go_back();
                }
            }
            Page::Library => {
                if event.key_pressed(Key::Named(NamedKey::Escape)) {
                    if self.state.library_query.is_empty() {
                        self.state.go_back();
                    } else {
                        self.state.library_query.clear();
                    }
                } else if event.key_pressed(Key::Named(NamedKey::Backspace)) {
                    self.state.library_query.pop();
                } else if !ctx.window_state.modifiers_state.control_key()
                    && !ctx.window_state.modifiers_state.alt_key()
                    && let WindowEvent::KeyboardInput { event, .. } = event
                    && event.state.is_pressed()
                    && !event.repeat
                    && let Key::Character(text) = &event.logical_key
                {
                    self.state.library_query.push_str(text);
                }
            }
            Page::Metadata => {
                if event.key_pressed(Key::Named(NamedKey::Escape)) {
                    self.metadata_editor = None;
                    self.state.go_back();
                    return;
                }

                let save_shortcut = ctx.window_state.modifiers_state.control_key()
                    && event.key_pressed(Key::Character("s"));
                if save_shortcut {
                    self.save_metadata_editor();
                    return;
                }

                let Some(editor) = self.metadata_editor.as_mut() else {
                    self.state.go_back();
                    return;
                };
                if event.key_pressed(Key::Named(NamedKey::Tab)) {
                    let offset = if ctx.window_state.modifiers_state.shift_key() {
                        -1
                    } else {
                        1
                    };
                    editor.select_relative(offset);
                    editor.message = None;
                } else if event.key_pressed(Key::Named(NamedKey::ArrowUp)) {
                    editor.select_relative(-1);
                    editor.message = None;
                } else if event.key_pressed(Key::Named(NamedKey::ArrowDown)) {
                    editor.select_relative(1);
                    editor.message = None;
                } else if event.key_pressed(Key::Named(NamedKey::Backspace)) {
                    editor.fields[editor.active].pop();
                    editor.message = None;
                } else if event.key_pressed(Key::Named(NamedKey::Delete)) {
                    editor.fields[editor.active].clear();
                    editor.message = None;
                } else if event.key_pressed(Key::Named(NamedKey::Enter)) {
                    if editor.active + 1 == editor.fields.len() {
                        self.save_metadata_editor();
                    } else {
                        editor.select_relative(1);
                    }
                } else if !ctx.window_state.modifiers_state.control_key()
                    && !ctx.window_state.modifiers_state.alt_key()
                    && let WindowEvent::KeyboardInput { event, .. } = event
                    && event.state.is_pressed()
                    && let Key::Character(text) = &event.logical_key
                {
                    editor.fields[editor.active].push_str(text);
                    editor.message = None;
                }
            }
            Page::Exercises => {
                if event.key_pressed(Key::Named(NamedKey::Enter)) {
                    self.start_exercise(ctx);
                }
                if event.key_pressed(Key::Named(NamedKey::Escape)) {
                    self.state.go_back();
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    fn debug_semantic_action(&mut self, ctx: &mut Context, id: &str) -> bool {
        match id {
            super::playing_scene::practice_ui_ids::MENU_EXERCISES
                if *self.state.current() == Page::Main =>
            {
                self.state.go_to(Page::Exercises);
                true
            }
            super::playing_scene::practice_ui_ids::MENU_LIBRARY
                if *self.state.current() == Page::Main =>
            {
                self.state.go_to(Page::Library);
                true
            }
            super::playing_scene::practice_ui_ids::LIBRARY_OPEN_RECENT_EXERCISE
                if *self.state.current() == Page::Library =>
            {
                let spec = ctx
                    .practice_history
                    .recent_songs(usize::MAX)
                    .into_iter()
                    .find_map(|song| song.exercise_spec);
                spec.is_some_and(|spec| self.open_exercise(ctx, spec))
            }
            super::playing_scene::practice_ui_ids::EXERCISE_START
                if *self.state.current() == Page::Exercises =>
            {
                self.start_exercise(ctx)
            }
            id if *self.state.current() == Page::Exercises => {
                self.debug_exercise_preset_action(ctx, id) || self.debug_adjust_exercise(id)
            }
            super::playing_scene::practice_ui_ids::MENU_START if self.state.song().is_some() => {
                state::play(&self.state, ctx);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LibraryProvenance, LibraryRow, MetadataEditor, ReviewReason, ReviewStatus, SongMetadata,
        format_review_status, library_row_matches, metadata_credit, provenance_credit,
        provenance_summary, truncate_menu_label,
    };

    #[test]
    fn library_titles_are_shortened_without_splitting_unicode() {
        assert_eq!(truncate_menu_label("夜に駆ける Piano", 8), "夜に駆ける P…");
        assert_eq!(truncate_menu_label("Clair de Lune", 30), "Clair de Lune");
    }

    #[test]
    fn library_search_requires_every_term_across_metadata() {
        let song = LibraryRow {
            content_id: "id".into(),
            display_name: "Clair de Lune".into(),
            metadata: SongMetadata {
                artist: Some("Walter Gieseking".into()),
                composer: Some("Claude Debussy".into()),
                tags: vec!["Impressionism".into()],
                ..Default::default()
            },
            provenance: Some(LibraryProvenance {
                category: "Classical/Impressionism".into(),
                title: "Clair de Lune".into(),
                composer: "Claude Debussy".into(),
                license: "CC BY-SA 4.0".into(),
                source: "Teaching Corpus".into(),
                source_url: "https://example.test/clair".into(),
            }),
            source_path: Some("D:/Piano/Suite bergamasque/Track 3.mid".into()),
            exercise_spec: None,
            session_count: 0,
            latest_accuracy: None,
            last_used_unix_ms: 0,
            favorite: false,
            queue_position: None,
            recommended_measures: None,
            review: None,
        };
        assert!(library_row_matches("debussy lune", &song));
        assert!(library_row_matches("gieseking impressionism", &song));
        assert!(library_row_matches("suite track", &song));
        assert!(library_row_matches("impressionism teaching by-sa", &song));
        assert!(!library_row_matches("debussy moonlight", &song));
        assert_eq!(
            metadata_credit(&song.metadata).as_deref(),
            Some("Walter Gieseking")
        );
    }

    #[test]
    fn metadata_editor_round_trips_fields_and_normalizes_tags_on_save() {
        let mut editor = MetadataEditor::new(
            "content".into(),
            "D:/Piano/Song.mid".into(),
            SongMetadata {
                title: Some("Song".into()),
                tags: vec!["etude".into(), "romantic".into()],
                ..Default::default()
            },
            None,
        );
        assert_eq!(editor.fields[0], "Song");
        assert_eq!(editor.fields[5], "etude, romantic");

        editor.fields[1] = " Performer ".into();
        editor.fields[2] = "Composer".into();
        editor.fields[5] = " etude, , technique ".into();
        editor.fields[6] = " ".into();
        let metadata = editor.metadata();
        assert_eq!(metadata.artist.as_deref(), Some("Performer"));
        assert_eq!(metadata.composer.as_deref(), Some("Composer"));
        assert_eq!(metadata.tags, ["etude", "technique"]);
        assert_eq!(metadata.notes, None);
    }

    #[test]
    fn metadata_editor_navigation_stays_inside_the_form() {
        let mut editor = MetadataEditor::new(
            "content".into(),
            "D:/Piano/Song.mid".into(),
            SongMetadata::default(),
            None,
        );
        editor.select_relative(-1);
        assert_eq!(editor.active, 0);
        editor.select_relative(20);
        assert_eq!(editor.active, 6);
        editor.select_relative(-2);
        assert_eq!(editor.active, 4);
    }

    #[test]
    fn provenance_summary_exposes_category_source_license_and_fallback_credit() {
        let provenance = LibraryProvenance {
            category: "Teaching/Burgmuller".into(),
            title: "Arabesque".into(),
            composer: "Friedrich Burgmuller".into(),
            license: "Public Domain".into(),
            source: "Mutopia Project".into(),
            source_url: "https://example.test/arabesque".into(),
        };
        assert_eq!(
            provenance_summary(&provenance),
            "Teaching/Burgmuller · Mutopia Project · Public Domain"
        );
        assert_eq!(
            provenance_credit(&provenance).as_deref(),
            Some("Friedrich Burgmuller")
        );
    }

    #[test]
    fn review_labels_explain_due_and_waiting_states() {
        let waiting = ReviewStatus {
            due_at_unix_ms: 1,
            is_due: false,
            interval_days: 3,
            days_until_due: 2,
            mastery_streak: 2,
            latest_accuracy: Some(0.95),
            reason: ReviewReason::Consolidating,
        };
        assert_eq!(
            format_review_status(waiting),
            " · review in 2d (2 mastered)"
        );
        assert_eq!(
            format_review_status(ReviewStatus {
                is_due: true,
                reason: ReviewReason::NeedsReinforcement,
                ..waiting
            }),
            " · due: reinforce"
        );
    }
}
