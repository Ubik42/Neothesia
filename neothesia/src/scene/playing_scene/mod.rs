use midi_file::midly::MidiMessage;
use neothesia_core::practice::{
    AdaptiveTempoDecision, AdaptiveTempoReason, AttemptSummary, PracticeHands, PracticePart,
};
use neothesia_core::practice_history::{
    PracticeHistoryOverview, PracticeSession, PracticeSessionKind, SongPracticeSetup,
};
use neothesia_core::render::{
    GlowRenderer, GuidelineRenderer, NoteLabels, QuadRenderer, TextRenderer,
};
use std::time::Duration;
use winit::{
    event::WindowEvent,
    keyboard::{Key, NamedKey},
};

use self::top_bar::TopBar;

use super::{NuonRenderer, Scene};
use crate::{
    NeothesiaEvent, context::Context, render::WaterfallRenderer, scene::MouseToMidiEventState,
    song::Song, utils::window::WinitEvent,
};

mod keyboard;
pub use keyboard::Keyboard;

pub(crate) mod midi_player;
use midi_player::MidiPlayer;

mod rewind_controller;
use rewind_controller::RewindController;

mod toast_manager;
use toast_manager::ToastManager;

mod animation;
mod top_bar;

pub struct PlayingScene {
    keyboard: Keyboard,
    waterfall: WaterfallRenderer,
    guidelines: GuidelineRenderer,
    text_renderer: TextRenderer,
    nuon_renderer: NuonRenderer,

    note_labels: Option<NoteLabels>,

    player: MidiPlayer,
    rewind_controller: RewindController,
    quad_renderer_bg: QuadRenderer,
    quad_renderer_fg: QuadRenderer,
    glow: Option<GlowRenderer>,
    toast_manager: ToastManager,

    nuon: nuon::Ui,
    mouse_to_midi_state: MouseToMidiEventState,

    top_bar: TopBar,
    completion: Option<AttemptSummary>,
    saved_session_count: Option<usize>,
    completion_view: CompletionView,
}

impl PlayingScene {
    pub fn new(ctx: &mut Context, song: Song) -> Self {
        let saved_setup = ctx.practice_history.setup(&song.file.content_id).cloned();
        if let Some(setup) = &saved_setup
            && setup.speed.is_finite()
            && setup.speed >= 0.0
        {
            ctx.config.set_speed_multiplier(setup.speed);
        }
        let keyboard = Keyboard::new(ctx, song.config.clone());

        let keyboard_layout = keyboard.layout();

        let guidelines = GuidelineRenderer::new(
            keyboard_layout.clone(),
            *keyboard.pos(),
            ctx.config.vertical_guidelines(),
            ctx.config.horizontal_guidelines(),
            ctx.config.beat_guidelines(),
            song.file.measures.clone(),
            song.file.beats.clone(),
        );

        let hidden_tracks: Vec<usize> = song
            .config
            .tracks
            .iter()
            .filter(|t| !t.visible)
            .map(|t| t.track_id)
            .collect();

        let mut waterfall = WaterfallRenderer::new(
            &ctx.gpu,
            &song.file.tracks,
            &hidden_tracks,
            &ctx.config,
            &ctx.transform,
            keyboard_layout.clone(),
        );

        let text_renderer = ctx.text_renderer_factory.new_renderer();

        let note_labels = ctx.config.note_labels().then_some(NoteLabels::new(
            *keyboard.pos(),
            waterfall.notes(),
            ctx.text_renderer_factory.new_renderer(),
        ));

        let player = MidiPlayer::new(
            ctx.output_manager.connection().clone(),
            song,
            keyboard_layout.range.clone(),
            ctx.config.separate_channels(),
            ctx.config.wait_for_notes(),
        );
        waterfall.update(player.time_without_lead_in());

        let quad_renderer_bg = ctx.quad_renderer_factory.new_renderer();
        let quad_renderer_fg = ctx.quad_renderer_factory.new_renderer();

        let glow = ctx.config.glow().then_some(GlowRenderer::new(
            &ctx.gpu,
            &ctx.transform,
            keyboard.layout(),
        ));

        let mut scene = Self {
            keyboard,
            guidelines,
            note_labels,
            text_renderer,
            nuon_renderer: NuonRenderer::new(ctx),

            waterfall,
            player,
            rewind_controller: RewindController::new(),
            quad_renderer_bg,
            quad_renderer_fg,
            glow,
            toast_manager: ToastManager::default(),

            nuon: nuon::Ui::new(),
            mouse_to_midi_state: MouseToMidiEventState::default(),

            top_bar: TopBar::new(),
            completion: None,
            saved_session_count: None,
            completion_view: CompletionView::Current,
        };
        if let Some(loop_setup) = saved_setup.and_then(|setup| setup.loop_setup) {
            top_bar::restore_loop_setup(&mut scene, loop_setup);
        }
        scene
    }

    fn update_glow(&mut self, delta: Duration) {
        let Some(glow) = &mut self.glow else {
            return;
        };

        glow.clear();

        let keys = &self.keyboard.layout().keys;
        let states = self.keyboard.key_states();

        for (key, state) in keys.iter().zip(states) {
            let Some(color) = state.pressed_by_file() else {
                continue;
            };

            glow.push(
                key.id(),
                *color,
                key.x(),
                self.keyboard.pos().y,
                key.width(),
                delta,
            );
        }
    }

    fn emergency_panic(&mut self) {
        self.player.emergency_stop();
        self.keyboard.reset_notes();
        self.mouse_to_midi_state.reset();
        self.top_bar.cancel_count_in();
        self.toast_manager
            .toast("PANIC: output silenced and playback paused");
    }

    fn cycle_practice_hands(&mut self, ctx: &mut Context) {
        let Some(current) = self.player.practice_hands() else {
            self.toast_manager
                .toast("Hand switch unavailable: assign left/right tracks first");
            return;
        };
        let next = current.next();
        if !self.player.set_practice_hands(next) {
            return;
        }

        self.keyboard.reset_notes();
        self.top_bar.reset_tempo_coach();
        if self.top_bar.is_looper_active() {
            top_bar::restart_loop_take(self);
        } else {
            self.player.restart_practice();
        }
        self.toast_manager.toast(match next {
            PracticeHands::Both => "Practice: both hands",
            PracticeHands::Right => "Practice: right hand · left hand plays automatically",
            PracticeHands::Left => "Practice: left hand · right hand plays automatically",
            PracticeHands::Custom | PracticeHands::Unspecified => unreachable!(),
        });
        self.save_practice_setup(ctx);
    }

    fn save_practice_setup(&mut self, ctx: &mut Context) {
        let setup = SongPracticeSetup {
            tracks: self.player.song().config.practice_track_setup(),
            speed: ctx.config.speed_multiplier(),
            loop_setup: self.top_bar.practice_loop_setup(&self.player),
        };
        if let Err(error) = ctx.practice_history.save_setup(
            &self.player.song().file.content_id,
            &self.player.song().file.name,
            setup,
        ) {
            self.toast_manager
                .toast(format!("Could not save practice setup: {error}"));
        }
    }

    #[profiling::function]
    fn update_midi_player(&mut self, ctx: &mut Context, delta: Duration) -> f32 {
        if self.top_bar.update_count_in(delta) {
            self.player.resume();
        }

        if self.top_bar.is_counting_in() {
            return self.player.time_without_lead_in() + ctx.config.animation_offset();
        }

        self.player.tick_practice_clock(delta);

        if self.top_bar.is_looper_active()
            && self.player.time() >= self.top_bar.loop_end_timestamp()
        {
            let summary = self.player.finish_practice();
            let attempted_speed = ctx.config.speed_multiplier();
            let (start_measure, end_measure) = self.top_bar.loop_measure_range(&self.player);
            if let Err(error) = persist_practice_session(
                ctx,
                &self.player,
                PracticeSessionKind::Loop {
                    start_measure,
                    end_measure,
                },
                attempted_speed,
                summary.clone(),
            ) {
                self.toast_manager
                    .toast(format!("Could not save practice history: {error}"));
            }
            let coach_decision = self.top_bar.record_attempt(
                summary,
                attempted_speed,
                ctx.config.adaptive_tempo_rules(),
                ctx.config.adaptive_tempo(),
            );
            if let Some(decision) = coach_decision {
                ctx.config.set_speed_multiplier(decision.speed);
                self.toast_manager.toast(tempo_coach_message(decision));
                self.save_practice_setup(ctx);
            }
            self.player.reset_practice_attempt();
            self.player.set_time(self.top_bar.loop_start_timestamp());
            self.keyboard.reset_notes();
            self.player.pause();
            let count_in = self.top_bar.count_in_duration(&self.player);
            self.top_bar.start_count_in(count_in);
            return self.player.time_without_lead_in() + ctx.config.animation_offset();
        }

        if self.player.should_advance() {
            let delta = scale_playback_delta(delta, ctx.config.speed_multiplier());
            let midi_events = self.player.update(delta);
            self.keyboard.file_midi_events(&ctx.config, &midi_events);
        }

        self.player.time_without_lead_in() + ctx.config.animation_offset()
    }

    fn queue_measure_numbers(&mut self, ctx: &Context, time: f32) {
        if !ctx.config.horizontal_guidelines() || !ctx.config.measure_numbers() {
            return;
        }

        let animation_speed = ctx.config.animation_speed() / ctx.window_state.scale_factor as f32;

        for (index, measure) in self
            .player
            .song()
            .file
            .measures
            .iter()
            .enumerate()
            .skip_while(|(_, measure)| measure.as_secs_f32() < time)
        {
            let y = self.keyboard.pos().y - (measure.as_secs_f32() - time) * animation_speed;
            if y < 0.0 {
                break;
            }

            let label = TextRenderer::gen_buffer_bold(14.0, &(index + 1).to_string());
            self.text_renderer
                .queue_buffer(8.0, (y - 18.0).max(0.0), label);
        }
    }

    fn completion_ui(&mut self, ctx: &mut Context) {
        let Some(summary) = self.completion.clone() else {
            return;
        };

        let mut weak_measures = summary.measures.clone();
        weak_measures.sort_by(|left, right| {
            left.breakdown
                .accuracy()
                .unwrap_or(1.0)
                .total_cmp(&right.breakdown.accuracy().unwrap_or(1.0))
                .then_with(|| left.measure.cmp(&right.measure))
        });
        let review = weak_measures
            .iter()
            .filter(|item| item.breakdown.accuracy().unwrap_or(1.0) < 0.999)
            .take(4)
            .map(|item| format!("{} ({}%)", item.measure, percent(item.breakdown.accuracy())))
            .collect::<Vec<_>>();
        let review = if review.is_empty() {
            "No weak measures detected".to_owned()
        } else {
            format!("Review measures {}", review.join(", "))
        };

        let part_accuracy = |part| {
            summary
                .parts
                .iter()
                .find(|item| item.part == part)
                .and_then(|item| item.breakdown.accuracy())
        };
        let recommendation = ctx
            .practice_history
            .song(&self.player.song().file.content_id)
            .and_then(|history| history.recommended_passage())
            .map(|mut recommendation| {
                let measure_count = self.player.song().file.measures.len().max(1);
                recommendation.start_measure = recommendation.start_measure.clamp(1, measure_count);
                recommendation.end_measure = recommendation
                    .end_measure
                    .clamp(recommendation.start_measure, measure_count);
                recommendation
            });
        let history_overview = ctx
            .practice_history
            .song(&self.player.song().file.content_id)
            .map(|history| history.overview(4, 4));

        let mut action = None;
        let mut requested_view = None;
        let completion_view = self.completion_view;
        let mut ui = std::mem::replace(&mut self.nuon, nuon::Ui::new());
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;
        let panel_w = (win_w - 40.0).clamp(480.0, 720.0);
        let panel_h = (win_h - 40.0).clamp(590.0, 620.0);
        let panel_x = nuon::center_x(win_w, panel_w);
        let panel_y = nuon::center_y(win_h, panel_h);

        nuon::layer().overlay(true).build(&mut ui, |ui| {
            nuon::quad()
                .size(win_w, win_h)
                .color([8, 7, 12, 220])
                .build(ui);

            nuon::translate().x(panel_x).y(panel_y).build(ui, |ui| {
                nuon::quad()
                    .size(panel_w, panel_h)
                    .color([37, 35, 48])
                    .border_radius([16.0; 4])
                    .build(ui);

                nuon::label()
                    .x(28.0)
                    .y(24.0)
                    .size(panel_w - 256.0, 44.0)
                    .font_size(30.0)
                    .bold(true)
                    .text("Practice complete")
                    .build(ui);

                let tab_gap = 8.0;
                let tab_w = 92.0;
                let tab_x = panel_w - 28.0 - tab_w * 2.0 - tab_gap;
                if nuon::button()
                    .x(tab_x)
                    .y(26.0)
                    .size(tab_w, 34.0)
                    .label("This take")
                    .color(if completion_view == CompletionView::Current {
                        [56, 145, 255]
                    } else {
                        [61, 57, 73]
                    })
                    .hover_color([87, 165, 255])
                    .preseed_color([97, 175, 255])
                    .border_radius([8.0; 4])
                    .build(ui)
                {
                    requested_view = Some(CompletionView::Current);
                }
                if nuon::button()
                    .x(tab_x + tab_w + tab_gap)
                    .y(26.0)
                    .size(tab_w, 34.0)
                    .label("History")
                    .color(if completion_view == CompletionView::History {
                        [56, 145, 255]
                    } else {
                        [61, 57, 73]
                    })
                    .hover_color([87, 165, 255])
                    .preseed_color([97, 175, 255])
                    .border_radius([8.0; 4])
                    .build(ui)
                {
                    requested_view = Some(CompletionView::History);
                }

                if completion_view == CompletionView::Current {
                    nuon::label()
                        .x(28.0)
                        .y(78.0)
                        .size(panel_w - 56.0, 54.0)
                        .font_size(38.0)
                        .bold(true)
                        .text(format!("{}% accuracy", percent(summary.overall.accuracy())))
                        .build(ui);

                    nuon::label()
                        .x(28.0)
                        .y(140.0)
                        .size(panel_w - 56.0, 34.0)
                        .font_size(17.0)
                        .text(format!(
                            "Hit {}    On time {}    Early {}    Late {}",
                            summary.overall.matched_notes,
                            summary.overall.on_time_notes,
                            summary.overall.early_notes,
                            summary.overall.late_notes
                        ))
                        .build(ui);

                    nuon::label()
                        .x(28.0)
                        .y(178.0)
                        .size(panel_w - 56.0, 34.0)
                        .font_size(17.0)
                        .text(format!(
                            "Wrong {}    Missed {}",
                            summary.overall.wrong_notes, summary.overall.missed_notes
                        ))
                        .build(ui);

                    nuon::label()
                        .x(28.0)
                        .y(226.0)
                        .size(panel_w - 56.0, 34.0)
                        .font_size(17.0)
                        .text(format!(
                            "Right hand {}    Left hand {}",
                            format_accuracy(part_accuracy(PracticePart::RightHand)),
                            format_accuracy(part_accuracy(PracticePart::LeftHand))
                        ))
                        .build(ui);

                    nuon::quad()
                        .x(28.0)
                        .y(278.0)
                        .size(panel_w - 56.0, 1.0)
                        .color([83, 78, 98])
                        .build(ui);

                    nuon::label()
                        .x(28.0)
                        .y(294.0)
                        .size(panel_w - 56.0, 44.0)
                        .font_size(17.0)
                        .text(review)
                        .build(ui);

                    if let Some(session_count) = self.saved_session_count {
                        nuon::label()
                            .x(28.0)
                            .y(338.0)
                            .size(panel_w - 56.0, 28.0)
                            .font_size(14.0)
                            .color([143, 205, 171])
                            .text(format!(
                                "Saved locally  ·  {session_count} session{} for this MIDI",
                                if session_count == 1 { "" } else { "s" }
                            ))
                            .build(ui);
                    }
                } else {
                    render_history_overview(ui, panel_w, history_overview.as_ref());
                }

                let button_gap = 12.0;
                let button_y = panel_h - 68.0;
                let button_w = (panel_w - 56.0 - button_gap) / 2.0;

                if let Some(recommendation) = recommendation {
                    if completion_view == CompletionView::Current {
                        nuon::label()
                            .x(28.0)
                            .y(366.0)
                            .size(panel_w - 56.0, 30.0)
                            .font_size(14.0)
                            .color([255, 205, 124])
                            .text(format!(
                                "Suggested: measures {}–{}  ·  {}% across {} notes / {} takes",
                                recommendation.start_measure,
                                recommendation.end_measure,
                                percent(Some(recommendation.weakest_accuracy)),
                                recommendation.judged_notes,
                                recommendation.attempts
                            ))
                            .build(ui);
                    }

                    if nuon::button()
                        .x(28.0)
                        .y(button_y - 56.0)
                        .size(panel_w - 56.0, 44.0)
                        .label(format!(
                            "Practice suggested measures {}–{}",
                            recommendation.start_measure, recommendation.end_measure
                        ))
                        .color([232, 144, 57])
                        .hover_color([245, 163, 82])
                        .preseed_color([255, 177, 96])
                        .border_radius([8.0; 4])
                        .build(ui)
                    {
                        action = Some(CompletionAction::PracticeWeak {
                            start_measure: recommendation.start_measure,
                            end_measure: recommendation.end_measure,
                        });
                    }
                }

                if nuon::button()
                    .x(28.0)
                    .y(button_y)
                    .size(button_w, 44.0)
                    .label("Practice again")
                    .color([56, 145, 255])
                    .hover_color([87, 165, 255])
                    .preseed_color([97, 175, 255])
                    .border_radius([8.0; 4])
                    .build(ui)
                {
                    action = Some(CompletionAction::Retry);
                }

                if nuon::button()
                    .x(28.0 + button_w + button_gap)
                    .y(button_y)
                    .size(button_w, 44.0)
                    .label("Back to songs")
                    .color([74, 68, 88])
                    .hover_color([94, 88, 108])
                    .preseed_color([104, 98, 118])
                    .border_radius([8.0; 4])
                    .build(ui)
                {
                    action = Some(CompletionAction::Back);
                }
            });
        });
        self.nuon = ui;
        if let Some(view) = requested_view {
            self.completion_view = view;
        }

        match action {
            Some(CompletionAction::PracticeWeak {
                start_measure,
                end_measure,
            }) => {
                if top_bar::begin_measure_loop(self, start_measure, end_measure) {
                    self.completion = None;
                    self.saved_session_count = None;
                    self.completion_view = CompletionView::Current;
                }
            }
            Some(CompletionAction::Retry) => {
                self.player.restart_practice();
                self.keyboard.reset_notes();
                self.completion = None;
                self.saved_session_count = None;
                self.completion_view = CompletionView::Current;
            }
            Some(CompletionAction::Back) => {
                ctx.proxy
                    .send_event(NeothesiaEvent::MainMenu(Some(self.player.song().clone())))
                    .ok();
            }
            None => {}
        }
    }

    #[profiling::function]
    fn resize(&mut self, ctx: &mut Context) {
        self.keyboard.resize(ctx);

        self.guidelines.set_layout(self.keyboard.layout().clone());
        self.guidelines.set_pos(*self.keyboard.pos());
        if let Some(note_labels) = self.note_labels.as_mut() {
            note_labels.set_pos(*self.keyboard.pos());
        }

        self.waterfall
            .resize(&ctx.config, self.keyboard.layout().clone());
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum CompletionView {
    Current,
    History,
}

#[derive(Debug, Clone, Copy)]
enum CompletionAction {
    PracticeWeak {
        start_measure: usize,
        end_measure: usize,
    },
    Retry,
    Back,
}

fn render_history_overview(
    ui: &mut nuon::Ui,
    panel_w: f32,
    overview: Option<&PracticeHistoryOverview>,
) {
    let Some(overview) = overview else {
        nuon::label()
            .x(28.0)
            .y(106.0)
            .size(panel_w - 56.0, 40.0)
            .font_size(20.0)
            .text("No saved practice history for this MIDI yet")
            .build(ui);
        return;
    };

    nuon::label()
        .x(28.0)
        .y(88.0)
        .size(panel_w - 56.0, 42.0)
        .font_size(30.0)
        .bold(true)
        .text(format!(
            "{} saved practice session{}",
            overview.total_sessions,
            if overview.total_sessions == 1 {
                ""
            } else {
                "s"
            }
        ))
        .build(ui);

    nuon::label()
        .x(28.0)
        .y(132.0)
        .size(panel_w - 56.0, 28.0)
        .font_size(16.0)
        .color([143, 205, 171])
        .text(format!(
            "{} trend  ·  {} comparable attempt{}",
            overview
                .trend_kind
                .zip(overview.trend_hands)
                .map(|(kind, hands)| format_practice_scope(kind, hands))
                .unwrap_or_else(|| "Practice".to_owned()),
            overview.trend_attempts,
            if overview.trend_attempts == 1 {
                ""
            } else {
                "s"
            },
        ))
        .build(ui);

    nuon::label()
        .x(28.0)
        .y(160.0)
        .size(panel_w - 56.0, 28.0)
        .font_size(15.0)
        .text(format!(
            "{}  ·  {}",
            format_trend("accuracy", overview.accuracy_delta),
            format_trend("speed", overview.speed_delta)
        ))
        .build(ui);

    nuon::quad()
        .x(28.0)
        .y(196.0)
        .size(panel_w - 56.0, 1.0)
        .color([83, 78, 98])
        .build(ui);

    nuon::label()
        .x(28.0)
        .y(208.0)
        .size(panel_w - 56.0, 30.0)
        .font_size(15.0)
        .bold(true)
        .text("Recent attempts  ·  oldest to newest")
        .build(ui);

    for (index, session) in overview.recent.iter().enumerate() {
        let accuracy = session
            .accuracy
            .map(|value| format!("{}%", percent(Some(value))))
            .unwrap_or_else(|| "--".to_owned());
        nuon::label()
            .x(28.0)
            .y(240.0 + index as f32 * 28.0)
            .size(panel_w - 56.0, 26.0)
            .font_size(15.0)
            .color(if index + 1 == overview.recent.len() {
                [235, 238, 245]
            } else {
                [178, 175, 190]
            })
            .text(format!(
                "{}. {}  ·  {} accuracy  ·  {} speed",
                index + 1,
                format_practice_scope(session.kind, session.hands),
                accuracy,
                format_speed(session.speed)
            ))
            .build(ui);
    }

    nuon::label()
        .x(28.0)
        .y(352.0)
        .size(panel_w - 56.0, 30.0)
        .font_size(15.0)
        .bold(true)
        .text("Persistent weak measures")
        .build(ui);

    if overview.weak_measures.is_empty() {
        nuon::label()
            .x(28.0)
            .y(386.0)
            .size(panel_w - 56.0, 28.0)
            .font_size(15.0)
            .color([143, 205, 171])
            .text("No weak measures detected")
            .build(ui);
    } else {
        let column_w = (panel_w - 68.0) / 2.0;
        for (index, measure) in overview.weak_measures.iter().enumerate() {
            nuon::label()
                .x(28.0 + (index % 2) as f32 * (column_w + 12.0))
                .y(386.0 + (index / 2) as f32 * 28.0)
                .size(column_w, 26.0)
                .font_size(15.0)
                .color([255, 205, 124])
                .text(format!(
                    "#{}  Measure {}  ·  {}%  ·  {} takes",
                    index + 1,
                    measure.measure,
                    percent(Some(measure.accuracy)),
                    measure.attempts
                ))
                .build(ui);
        }
    }
}

fn format_session_kind(kind: PracticeSessionKind) -> String {
    match kind {
        PracticeSessionKind::WholeSong => "Whole song".to_owned(),
        PracticeSessionKind::Loop {
            start_measure,
            end_measure,
        } if start_measure == end_measure => format!("Measure {start_measure}"),
        PracticeSessionKind::Loop {
            start_measure,
            end_measure,
        } => format!("Measures {start_measure}-{end_measure}"),
    }
}

fn format_practice_scope(kind: PracticeSessionKind, hands: PracticeHands) -> String {
    let kind = format_session_kind(kind);
    match hands {
        PracticeHands::Both => format!("{kind} · Both hands"),
        PracticeHands::Right => format!("{kind} · Right hand"),
        PracticeHands::Left => format!("{kind} · Left hand"),
        PracticeHands::Custom => format!("{kind} · Custom parts"),
        PracticeHands::Unspecified => kind,
    }
}

fn format_trend(label: &str, delta: Option<f32>) -> String {
    let Some(delta) = delta else {
        return format!("{label}: need 2 attempts");
    };
    let points = (delta * 100.0).round() as i32;
    if points > 0 {
        format!("{label}: +{points} pts")
    } else {
        format!("{label}: {points} pts")
    }
}

fn format_speed(speed: f32) -> String {
    if speed.is_finite() {
        format!("{}%", (speed * 100.0).round() as i32)
    } else {
        "--".to_owned()
    }
}

fn percent(accuracy: Option<f32>) -> u32 {
    (accuracy.unwrap_or(0.0) * 100.0).round() as u32
}

fn format_accuracy(accuracy: Option<f32>) -> String {
    accuracy
        .map(|accuracy| format!("{}%", percent(Some(accuracy))))
        .unwrap_or_else(|| "--".to_owned())
}

fn persist_practice_session(
    ctx: &mut Context,
    player: &MidiPlayer,
    kind: PracticeSessionKind,
    speed: f32,
    summary: AttemptSummary,
) -> Result<usize, neothesia_core::practice_history::PracticeHistoryError> {
    let song_id = player.song().file.content_id.clone();
    let display_name = player.song().file.name.clone();
    ctx.practice_history.record_session(
        &song_id,
        &display_name,
        PracticeSession::new(
            kind,
            player
                .practice_hands()
                .unwrap_or(PracticeHands::Unspecified),
            speed,
            summary,
        ),
    )?;
    Ok(ctx.practice_history.session_count(&song_id))
}

fn scale_playback_delta(delta: Duration, speed: f32) -> Duration {
    let speed = if speed.is_finite() {
        speed.max(0.0) as f64
    } else {
        1.0
    };
    let speed = (speed * 10_000.0).round() / 10_000.0;
    Duration::from_secs_f64(delta.as_secs_f64() * speed)
}

fn tempo_coach_message(decision: AdaptiveTempoDecision) -> String {
    let speed = |value: f32| (value * 100.0).round() as u32;
    match decision.reason {
        AdaptiveTempoReason::NoJudgedNotes => {
            "Tempo Coach: no played notes yet, holding speed".to_owned()
        }
        AdaptiveTempoReason::BuildingMastery {
            completed,
            required,
        } => format!(
            "Tempo Coach: mastered take {completed}/{required}, hold {}%",
            speed(decision.speed)
        ),
        AdaptiveTempoReason::Mastered => format!(
            "Tempo Coach: two mastered takes, {}% -> {}%",
            speed(decision.previous_speed),
            speed(decision.speed)
        ),
        AdaptiveTempoReason::NeedsAccuracy => format!(
            "Tempo Coach: accuracy below target, {}% -> {}%",
            speed(decision.previous_speed),
            speed(decision.speed)
        ),
        AdaptiveTempoReason::KeepPractising => format!(
            "Tempo Coach: building consistency, hold {}%",
            speed(decision.speed)
        ),
        AdaptiveTempoReason::AtMaximum => format!(
            "Tempo Coach: mastery reached at the {}% maximum",
            speed(decision.speed)
        ),
        AdaptiveTempoReason::AtMinimum => format!(
            "Tempo Coach: accuracy needs work, hold at {}% minimum",
            speed(decision.speed)
        ),
    }
}

impl Scene for PlayingScene {
    #[profiling::function]
    fn update(&mut self, ctx: &mut Context, delta: Duration) {
        self.quad_renderer_bg.clear();
        self.quad_renderer_fg.clear();

        self.rewind_controller.update(&mut self.player, ctx, delta);
        self.toast_manager.update(&mut self.text_renderer);

        let time = self.update_midi_player(ctx, delta);
        self.waterfall.update(time);
        self.guidelines.update(
            &mut self.quad_renderer_bg,
            ctx.config.animation_speed(),
            ctx.window_state.scale_factor as f32,
            time,
            ctx.window_state.logical_size,
        );
        self.queue_measure_numbers(ctx, time);
        self.keyboard
            .update(&mut self.quad_renderer_fg, &mut self.text_renderer);
        if let Some(note_labels) = self.note_labels.as_mut() {
            note_labels.update(
                ctx.window_state.physical_size,
                ctx.window_state.scale_factor as f32,
                self.keyboard.renderer(),
                ctx.config.animation_speed(),
                time,
            );
        }

        self.update_glow(delta);

        if self.completion.is_none() && self.player.is_finished() && !self.player.is_paused() {
            let summary = self.player.finish_practice();
            let speed = ctx.config.speed_multiplier();
            self.saved_session_count = match persist_practice_session(
                ctx,
                &self.player,
                PracticeSessionKind::WholeSong,
                speed,
                summary.clone(),
            ) {
                Ok(count) => Some(count),
                Err(error) => {
                    self.toast_manager
                        .toast(format!("Could not save practice history: {error}"));
                    None
                }
            };
            self.player.pause();
            self.completion = Some(summary);
        }

        TopBar::update(self, ctx);
        self.completion_ui(ctx);

        super::render_nuon(&mut self.nuon, &mut self.nuon_renderer, ctx);

        self.quad_renderer_bg.prepare();
        self.quad_renderer_fg.prepare();

        if let Some(glow) = &mut self.glow {
            glow.prepare();
        }

        #[cfg(debug_assertions)]
        self.text_renderer.queue_fps(
            ctx.fps_ticker.avg(),
            self.top_bar
                .topbar_expand_animation
                .animate_bool(5.0, 80.0, ctx.frame_timestamp),
        );
        self.text_renderer.update(
            ctx.window_state.physical_size,
            ctx.window_state.scale_factor as f32,
        );
    }

    #[profiling::function]
    fn render<'pass>(&'pass mut self, rpass: &mut wgpu_jumpstart::RenderPass<'pass>) {
        self.quad_renderer_bg.render(rpass);
        self.waterfall.render(rpass);
        if let Some(note_labels) = self.note_labels.as_mut() {
            note_labels.render(rpass);
        }
        self.quad_renderer_fg.render(rpass);
        if let Some(glow) = &self.glow {
            glow.render(rpass);
        }
        self.text_renderer.render(rpass);

        self.nuon_renderer.render(rpass);
    }

    fn window_event(&mut self, ctx: &mut Context, event: &WindowEvent) {
        if self.completion.is_some() {
            if event.back_mouse_pressed() || event.key_released(Key::Named(NamedKey::Escape)) {
                ctx.proxy
                    .send_event(NeothesiaEvent::MainMenu(Some(self.player.song().clone())))
                    .ok();
            }
            if event.window_resized() || event.scale_factor_changed() {
                self.resize(ctx)
            }
            super::handle_nuon_window_event(&mut self.nuon, event, ctx);
            return;
        }

        self.rewind_controller
            .handle_window_event(ctx, event, &mut self.player);

        if self.rewind_controller.is_rewinding() {
            self.keyboard.reset_notes();
        }

        if event.back_mouse_pressed() || event.key_released(Key::Named(NamedKey::Escape)) {
            ctx.proxy
                .send_event(NeothesiaEvent::MainMenu(Some(self.player.song().clone())))
                .ok();
        }

        if event.key_released(Key::Named(NamedKey::Space)) {
            self.player.pause_resume();
        }

        let speed_before = ctx.config.speed_multiplier();
        handle_settings_input(ctx, &mut self.toast_manager, &mut self.waterfall, event);
        if ctx.config.speed_multiplier() != speed_before {
            self.top_bar.reset_tempo_coach();
            self.save_practice_setup(ctx);
        }
        super::handle_pc_keyboard_to_midi_event(ctx, event);
        super::handle_mouse_to_midi_event(
            &mut self.keyboard,
            &mut self.mouse_to_midi_state,
            ctx,
            event,
        );

        if event.window_resized() || event.scale_factor_changed() {
            self.resize(ctx)
        }

        super::handle_nuon_window_event(&mut self.nuon, event, ctx);
    }

    fn midi_event(&mut self, _ctx: &mut Context, channel: u8, message: &MidiMessage) {
        self.player.user_midi_event(channel, message);
        self.keyboard.user_midi_event(message);
    }

    fn emergency_stop(&mut self, _ctx: &mut Context) {
        self.emergency_panic();
    }
}

fn handle_settings_input(
    ctx: &mut Context,
    toast_manager: &mut ToastManager,
    waterfall: &mut WaterfallRenderer,
    event: &WindowEvent,
) {
    if event.key_released(Key::Named(NamedKey::ArrowUp))
        || event.key_released(Key::Named(NamedKey::ArrowDown))
    {
        let amount = if ctx.window_state.modifiers_state.shift_key() {
            0.5
        } else {
            0.1
        };

        if event.key_released(Key::Named(NamedKey::ArrowUp)) {
            ctx.config
                .set_speed_multiplier(ctx.config.speed_multiplier() + amount);
        } else {
            ctx.config
                .set_speed_multiplier(ctx.config.speed_multiplier() - amount);
        }

        toast_manager.speed_toast(ctx.config.speed_multiplier());
        return;
    }

    if event.key_released(Key::Named(NamedKey::PageUp))
        || event.key_released(Key::Named(NamedKey::PageDown))
    {
        let amount = if ctx.window_state.modifiers_state.shift_key() {
            500.0
        } else {
            100.0
        };

        if event.key_released(Key::Named(NamedKey::PageUp)) {
            ctx.config
                .set_animation_speed(ctx.config.animation_speed() + amount);
        } else {
            ctx.config
                .set_animation_speed(ctx.config.animation_speed() - amount);
        }

        waterfall
            .pipeline()
            .set_speed(&ctx.gpu.queue, ctx.config.animation_speed());
        toast_manager.animation_speed_toast(ctx.config.animation_speed());
        return;
    }

    if let Some(ch @ ("_" | "-" | "+" | "=")) = event.character_released() {
        let amount = if ctx.window_state.modifiers_state.shift_key() {
            0.1
        } else {
            0.01
        };

        if matches!(ch, "-" | "_") {
            ctx.config
                .set_animation_offset(ctx.config.animation_offset() - amount);
        } else {
            ctx.config
                .set_animation_offset(ctx.config.animation_offset() + amount);
        }

        toast_manager.offset_toast(ctx.config.animation_offset());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_delta_preserves_five_percent_speed_steps() {
        assert_eq!(
            scale_playback_delta(Duration::from_secs(1), 0.75),
            Duration::from_millis(750)
        );
        assert_eq!(
            scale_playback_delta(Duration::from_secs(1), 1.05),
            Duration::from_millis(1_050)
        );
    }
}
