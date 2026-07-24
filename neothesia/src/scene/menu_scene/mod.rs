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
use neothesia_core::practice_history::{ReviewReason, ReviewStatus};
use neothesia_core::render::{BgPipeline, ImageIdentifier, QuadRenderer, TextRenderer};

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

                        if neo_btn().size(w, h).label("Practice Library").build(ui) {
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
            if library_text_matches(&query, &song.display_name, song.source_path.as_deref()) {
                rows.insert(
                    song.content_id.clone(),
                    LibraryRow {
                        content_id: song.content_id,
                        display_name: song.display_name,
                        source_path: song.source_path,
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
        }
        if let Some(index) = &self.state.library_index {
            for song in index.search(&query) {
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
                    })
                    .or_insert_with(|| LibraryRow {
                        content_id: song.content_id.clone(),
                        display_name: song.display_name.clone(),
                        source_path: available_path,
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
                            let accuracy = song
                                .latest_accuracy
                                .map(|value| format!(" · latest {}%", (value * 100.0).round()))
                                .unwrap_or_default();
                            let action = if path_available { "Open" } else { "Locate" };
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
                            let label = format!(
                                "{}  ·  {} session{}{}{}{}  ·  {}",
                                truncate_menu_label(&song.display_name, 54),
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
                            let open_width = row_w - 142.0 - reorder_width;
                            if nuon::button()
                                .id(nuon::Id::hash_with(|hasher| {
                                    "library-song".hash(hasher);
                                    song.content_id.hash(hasher);
                                }))
                                .x(row_x)
                                .y(index as f32 * 62.0)
                                .size(open_width, 52.0)
                                .label(label)
                                .color(if path_available {
                                    [48, 91, 82]
                                } else {
                                    [91, 72, 52]
                                })
                                .hover_color(if path_available {
                                    [57, 112, 99]
                                } else {
                                    [112, 88, 60]
                                })
                                .preseed_color([109, 78, 164])
                                .border_radius([7.0; 4])
                                .build(ui)
                            {
                                if let Some(path) = song.source_path.clone().filter(|p| p.is_file())
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
                                    "library-favorite".hash(hasher);
                                    song.content_id.hash(hasher);
                                }))
                                .x(row_x + open_width + 6.0)
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
                                .x(row_x + open_width + 72.0)
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
                                    [(148.0, -1, "↑"), (192.0, 1, "↓")]
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
                state.library_index = Some(index);
                state.library_scanning = false;
                state.library_message = Some(format!(
                    "Indexed {unique} unique piece{} from {seen} MIDI file{}{}.",
                    if unique == 1 { "" } else { "s" },
                    if seen == 1 { "" } else { "s" },
                    if unreadable == 0 {
                        String::new()
                    } else {
                        format!("; skipped {unreadable} unreadable")
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
    source_path: Option<PathBuf>,
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

async fn choose_library_folder() -> Option<PathBuf> {
    let folder = rfd::AsyncFileDialog::new()
        .set_title("Add Piano MIDI Folder")
        .pick_folder()
        .await?;
    Some(folder.path().to_path_buf())
}

fn library_text_matches(query: &str, name: &str, path: Option<&std::path::Path>) -> bool {
    let terms: Vec<_> = query
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|term| !term.is_empty())
        .collect();
    if terms.is_empty() {
        return true;
    }
    let searchable = format!(
        "{} {}",
        name.to_lowercase(),
        path.map(|path| path.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    );
    terms.iter().all(|term| searchable.contains(term))
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
            super::playing_scene::practice_ui_ids::EXERCISE_START
                if *self.state.current() == Page::Exercises =>
            {
                self.start_exercise(ctx)
            }
            id if *self.state.current() == Page::Exercises => self.debug_adjust_exercise(id),
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
        ReviewReason, ReviewStatus, format_review_status, library_text_matches, truncate_menu_label,
    };

    #[test]
    fn library_titles_are_shortened_without_splitting_unicode() {
        assert_eq!(truncate_menu_label("夜に駆ける Piano", 8), "夜に駆ける P…");
        assert_eq!(truncate_menu_label("Clair de Lune", 30), "Clair de Lune");
    }

    #[test]
    fn library_search_requires_every_term() {
        let path = std::path::Path::new("D:/Piano/Debussy/Clair de Lune.mid");
        assert!(library_text_matches(
            "debussy lune",
            "Clair de Lune.mid",
            Some(path)
        ));
        assert!(!library_text_matches(
            "debussy moonlight",
            "Clair de Lune.mid",
            Some(path)
        ));
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
