use std::time::Duration;

use neothesia_core::render::{NoteLabels, WaterfallRenderer};

use crate::{
    NeothesiaEvent,
    context::Context,
    icons,
    scene::{
        freeplay::{FreeplayScene, on_async},
        playing_scene::{Keyboard, midi_player::MidiPlayer},
    },
    song::Song,
};

pub use neothesia_core::recorder::{FreeplayRecorder, RecorderError, RecorderStatus};

pub struct Preview {
    player: MidiPlayer,
    waterfall: WaterfallRenderer,
    note_labels: Option<NoteLabels>,
}

impl Preview {
    fn new(keyboard: &Keyboard, song: Song, ctx: &Context) -> Self {
        let hidden_tracks: Vec<usize> = song
            .config
            .tracks
            .iter()
            .filter(|track| !track.visible)
            .map(|track| track.track_id)
            .collect();

        let mut waterfall = WaterfallRenderer::new(
            &ctx.gpu,
            &song.file.tracks,
            &hidden_tracks,
            &ctx.config,
            &ctx.transform,
            keyboard.layout().clone(),
        );

        let note_labels = ctx.config.note_labels().then_some(NoteLabels::new(
            *keyboard.pos(),
            waterfall.notes(),
            ctx.text_renderer_factory.new_renderer(),
        ));

        let mut player = MidiPlayer::new_with_lead_in(
            ctx.output_manager.connection().clone(),
            song,
            keyboard.layout().range.clone(),
            ctx.config.separate_channels(),
            false,
            Duration::ZERO,
        );
        player.pause();
        waterfall.update(player.time_without_lead_in() + ctx.config.animation_offset());

        Self {
            player,
            waterfall,
            note_labels,
        }
    }

    pub fn resize(&mut self, keyboard: &Keyboard, ctx: &mut Context) {
        self.waterfall
            .resize(&ctx.config, keyboard.layout().clone());

        if let Some(note_labels) = self.note_labels.as_mut() {
            note_labels.set_pos(*keyboard.pos());
        }
    }

    pub fn emergency_stop(&mut self) {
        self.player.emergency_stop();
    }

    pub fn update(&mut self, keyboard: &mut Keyboard, ctx: &mut Context, delta: Duration) {
        let midi_events = self.player.update(delta);
        keyboard.file_midi_events(&ctx.config, &midi_events);

        if self.player.is_finished() && !self.player.is_paused() {
            self.player.pause();
        }

        let time = self.player.time_without_lead_in() + ctx.config.animation_offset();

        self.waterfall.update(time);

        if let Some(note_labels) = self.note_labels.as_mut() {
            note_labels.update(
                ctx.window_state.physical_size,
                ctx.window_state.scale_factor as f32,
                keyboard.renderer(),
                ctx.config.animation_speed(),
                time,
            );
        }
    }

    pub fn render<'pass>(&'pass mut self, rpass: &mut wgpu_jumpstart::RenderPass<'pass>) {
        self.waterfall.render(rpass);
        if let Some(note_labels) = self.note_labels.as_mut() {
            note_labels.render(rpass);
        }
    }
}

pub fn update_preview_ui(scene: &mut FreeplayScene, ctx: &mut Context) {
    let top_bar_height = 30.0;

    let width = ctx.window_state.logical_size.width;

    let available = scene.preview.is_some();

    let is_paused = scene
        .preview
        .as_ref()
        .map(|s| s.player.is_paused())
        .unwrap_or(true);

    let status_label = if scene.recorder.is_recording() {
        format!("Recording {:.1}s", scene.recorder.duration().as_secs_f32())
    } else {
        scene.recorder_status.to_string()
    };

    enum Msg {
        TogglePlay,
        Seek,
        GoBack,
        Record,
        Save,
        None,
    }

    let mut msg = Msg::None;

    nuon::translate().build(&mut scene.nuon, |ui| {
        nuon::quad()
            .size(width, top_bar_height)
            .color([37, 35, 42])
            .build(ui);

        nuon::translate().build(ui, |ui| {
            if nuon::button()
                .size(30.0, 30.0)
                .border_radius([5.0; 4])
                .icon(icons::left_arrow_icon())
                .build(ui)
            {
                msg = Msg::GoBack;
            }
            nuon::translate().x(30.0).add_to_current(ui);
        });

        nuon::label()
            .size(width, 30.0)
            .text(&status_label)
            .text_justify(nuon::TextJustify::Center)
            .build(ui);

        nuon::translate().x(width).build(ui, |ui| {
            nuon::translate().x(-30.0).add_to_current(ui);

            if nuon::button()
                .size(30.0, 30.0)
                .border_radius([5.0; 4])
                .icon(if is_paused {
                    icons::play_icon()
                } else {
                    icons::pause_icon()
                })
                .font_color(if available {
                    [255, 255, 255, 255]
                } else {
                    [255, 255, 255, 100]
                })
                .build(ui)
                && available
            {
                msg = Msg::TogglePlay;
            }

            nuon::translate().x(-30.0).add_to_current(ui);

            if nuon::button()
                .size(30.0, 30.0)
                .border_radius([5.0; 4])
                .icon(icons::save_icon())
                .font_color(if available {
                    [255, 255, 255, 255]
                } else {
                    [255, 255, 255, 100]
                })
                .build(ui)
                && available
            {
                msg = Msg::Save;
            }

            nuon::translate().x(-30.0).add_to_current(ui);

            if nuon::button()
                .size(30.0, 30.0)
                .border_radius([5.0; 4])
                .icon(if scene.recorder.is_recording() {
                    icons::record_stop_icon()
                } else {
                    icons::record_icon()
                })
                .color(if scene.recorder.is_recording() {
                    [208, 18, 0, 255]
                } else {
                    [0, 0, 0, 0]
                })
                .hover_color(if scene.recorder.is_recording() {
                    [165, 47, 47]
                } else {
                    [97, 97, 97]
                })
                .preseed_color(if scene.recorder.is_recording() {
                    [145, 37, 37]
                } else {
                    [87, 87, 87]
                })
                .build(ui)
            {
                msg = Msg::Record;
            }
        });

        if let Some(state) = scene.preview.as_ref() {
            let length = state.player.length();
            let progress = state.player.percentage();
            let measures = &state.player.song().file.measures;

            nuon::translate().y(30.0).build(ui, |ui| {
                let event = nuon::click_area("FreeplayPreviewProgress")
                    .size(width, 45.0)
                    .build(ui);

                if event.is_pressed() {
                    msg = Msg::Seek;
                }

                nuon::quad().size(width, 45.0).color([37, 35, 42]).build(ui);
                nuon::quad()
                    .size(width * progress, 45.0)
                    .color([56, 145, 255])
                    .build(ui);

                if !length.is_zero() {
                    for measure in measures.iter() {
                        let x = (measure.as_secs_f32() / length.as_secs_f32()) * width;
                        nuon::quad()
                            .x(x)
                            .size(1.0, 45.0)
                            .color(if x < width * progress {
                                [255, 255, 255, 127]
                            } else {
                                [102, 102, 102, 255]
                            })
                            .build(ui);
                    }
                }
            });
        }

        // H-separator
        nuon::quad()
            .y(top_bar_height)
            .size(width, 1.0)
            .color([57, 55, 62])
            .build(ui);
    });

    match msg {
        Msg::TogglePlay => {
            toggle_preview_playback(scene);
        }
        Msg::Seek => {
            seek_preview_to_cursor(scene, ctx);
        }
        Msg::GoBack => {
            ctx.proxy
                .send_event(NeothesiaEvent::MainMenu(scene.song.clone()))
                .ok();
        }
        Msg::Record => {
            handle_record_click(scene, ctx);
        }
        Msg::Save => {
            handle_save_click(scene, ctx);
        }
        Msg::None => {}
    }
}

fn handle_record_click(scene: &mut FreeplayScene, ctx: &Context) {
    if scene.recorder.is_recording() {
        scene.recorder_status = match stop_recording(scene, ctx) {
            Ok(()) => RecorderStatus::RecordingFinished(scene.recorder.duration()),
            Err(err) => RecorderStatus::Error(err),
        };
        return;
    }

    scene.keyboard.set_song_config(Default::default());
    scene.keyboard.reset_notes();

    scene.preview = None;
    scene.recorder_status = RecorderStatus::default();
    scene.recorder.start();
}

fn handle_save_click(scene: &mut FreeplayScene, ctx: &Context) {
    let mut dialog = rfd::AsyncFileDialog::new()
        .add_filter("midi", &["mid", "midi"])
        .set_file_name("freeplay-recording.mid");

    // TODO: `last_opened_song` is wrong in this context
    if let Some(path) = ctx.config.last_opened_song().and_then(|path| path.parent()) {
        dialog = dialog.set_directory(path);
    }

    let smf = match scene.recorder.as_smf() {
        Ok(smf) => smf.clone(),
        Err(err) => {
            scene.recorder_status = RecorderStatus::Error(err);
            return;
        }
    };

    scene
        .futures
        .push(on_async(dialog.save_file(), move |file, state, _ctx| {
            let Some(file) = file else {
                return;
            };

            match smf.save(file.path()) {
                Ok(()) => {
                    state.recorder_status = RecorderStatus::Saved(file.path().to_owned());
                }
                Err(_) => {
                    state.recorder_status = RecorderStatus::Error(RecorderError::Write);
                }
            }
        }));
}

fn stop_recording(scene: &mut FreeplayScene, ctx: &Context) -> Result<(), RecorderError> {
    let smf = scene.recorder.stop()?;

    let midi = midi_file::MidiFile::from_smf("freeplay-recording.mid", smf)
        .map_err(RecorderError::MidiFileParse)?;
    let song = Song::new(midi);

    scene.keyboard.set_song_config(song.config.clone());
    scene.keyboard.reset_notes();

    scene.preview = Some(Preview::new(&scene.keyboard, song, ctx));

    Ok(())
}

fn seek_preview_to_cursor(scene: &mut FreeplayScene, ctx: &Context) {
    let Some(player) = scene.preview.as_mut().map(|state| &mut state.player) else {
        return;
    };

    let width = ctx.window_state.logical_size.width.max(1.0);
    let percentage = (ctx.window_state.cursor_logical_position.x / width).clamp(0.0, 1.0);

    player.set_percentage_time(percentage);
    scene.keyboard.reset_notes();
}

pub fn toggle_preview_playback(scene: &mut FreeplayScene) {
    let Some(preview) = scene.preview.as_mut() else {
        return;
    };

    preview.player.pause_resume();
}
