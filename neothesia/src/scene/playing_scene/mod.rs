use midi_file::midly::MidiMessage;
use neothesia_core::fingering::{
    FingerSuggestion, FingeringHand, FingeringNote, suggest_fingerings_with_profile,
};
use neothesia_core::library::{FingerHint, save_song_fingerings};
use neothesia_core::practice::{
    AdaptiveTempoDecision, AdaptiveTempoReason, AttemptSummary, ExpressionSummary, PracticeHands,
    PracticePart, TimingCalibrationStatus, timing_calibration_status,
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

pub(crate) mod practice_ui_ids {
    pub const MENU_START: &str = "practice.menu.start";
    pub const MENU_EXERCISES: &str = "practice.menu.exercises";
    pub const MENU_LIBRARY: &str = "practice.menu.library";
    #[cfg(any(debug_assertions, test))]
    pub const LIBRARY_OPEN_RECENT_EXERCISE: &str = "practice.library.open-recent-exercise";
    pub const EXERCISE_START: &str = "practice.exercise.start";
    pub const EXERCISE_KEY_PREVIOUS: &str = "practice.exercise.key.previous";
    pub const EXERCISE_KEY_NEXT: &str = "practice.exercise.key.next";
    pub const EXERCISE_TONALITY_PREVIOUS: &str = "practice.exercise.tonality.previous";
    pub const EXERCISE_TONALITY_NEXT: &str = "practice.exercise.tonality.next";
    pub const EXERCISE_PATTERN_PREVIOUS: &str = "practice.exercise.pattern.previous";
    pub const EXERCISE_PATTERN_NEXT: &str = "practice.exercise.pattern.next";
    pub const EXERCISE_DIRECTION_PREVIOUS: &str = "practice.exercise.direction.previous";
    pub const EXERCISE_DIRECTION_NEXT: &str = "practice.exercise.direction.next";
    pub const EXERCISE_HANDS_PREVIOUS: &str = "practice.exercise.hands.previous";
    pub const EXERCISE_HANDS_NEXT: &str = "practice.exercise.hands.next";
    pub const EXERCISE_OCTAVES_PREVIOUS: &str = "practice.exercise.octaves.previous";
    pub const EXERCISE_OCTAVES_NEXT: &str = "practice.exercise.octaves.next";
    pub const EXERCISE_TEMPO_PREVIOUS: &str = "practice.exercise.tempo.previous";
    pub const EXERCISE_TEMPO_NEXT: &str = "practice.exercise.tempo.next";
    pub const EXERCISE_REPETITIONS_PREVIOUS: &str = "practice.exercise.repetitions.previous";
    pub const EXERCISE_REPETITIONS_NEXT: &str = "practice.exercise.repetitions.next";
    pub const EXERCISE_RECENT_PREVIOUS: &str = "practice.exercise.recent.previous";
    pub const EXERCISE_RECENT_NEXT: &str = "practice.exercise.recent.next";
    pub const EXERCISE_FAVOURITE_PREVIOUS: &str = "practice.exercise.favourite.previous";
    pub const EXERCISE_FAVOURITE_NEXT: &str = "practice.exercise.favourite.next";
    pub const EXERCISE_FAVOURITE_TOGGLE: &str = "practice.exercise.favourite.toggle";
    pub const PLAYER_BACK: &str = "practice.player.back";
    pub const PLAYER_WAIT: &str = "practice.player.wait";
    pub const PLAYER_COACH: &str = "practice.player.coach";
    pub const PLAYER_HANDS: &str = "practice.player.hands";
    pub const PLAYER_LOOP: &str = "practice.player.loop";
    pub const PLAYER_FINGERINGS: &str = "practice.player.fingerings";
    #[cfg(feature = "score-verovio")]
    pub const PLAYER_SCORE: &str = "practice.player.score";
    #[cfg(feature = "score-verovio")]
    pub const PLAYER_SCORE_ZOOM_OUT: &str = "practice.player.score.zoom-out";
    #[cfg(feature = "score-verovio")]
    pub const PLAYER_SCORE_ZOOM_IN: &str = "practice.player.score.zoom-in";
    pub const PLAYER_FINGERING_EDITOR: &str = "practice.player.fingering-editor";
    #[cfg(any(debug_assertions, test))]
    pub const PLAYER_FINGERING_NEXT: &str = "practice.player.fingering-next";
    #[cfg(any(debug_assertions, test))]
    pub const PLAYER_FINGERING_ASSIGN_1: &str = "practice.player.fingering-assign-1";
    #[cfg(any(debug_assertions, test))]
    pub const PLAYER_FINGERING_CLEAR: &str = "practice.player.fingering-clear";
    #[cfg(any(debug_assertions, test))]
    pub const PLAYER_FINGERING_SUGGEST: &str = "practice.player.fingering-suggest";
    #[cfg(any(debug_assertions, test))]
    pub const PLAYER_FINGERING_ACCEPT: &str = "practice.player.fingering-accept";
    #[cfg(debug_assertions)]
    pub const PLAYER_RESTART: &str = "practice.player.restart";
    pub const COMPLETION_OVERVIEW: &str = "practice.completion.tab.overview";
    pub const COMPLETION_TECHNIQUE: &str = "practice.completion.tab.technique";
    pub const COMPLETION_HISTORY: &str = "practice.completion.tab.history";
    pub const COMPLETION_CALIBRATE: &str = "practice.completion.calibrate";
    pub const COMPLETION_NOTES_LOOP: &str = "practice.completion.notes-loop";
    pub const COMPLETION_RHYTHM_LOOP: &str = "practice.completion.rhythm-loop";
    pub const COMPLETION_RETRY: &str = "practice.completion.retry";
    pub const COMPLETION_BACK: &str = "practice.completion.back";

    #[cfg(test)]
    pub const ALL: &[&str] = &[
        MENU_START,
        MENU_EXERCISES,
        MENU_LIBRARY,
        LIBRARY_OPEN_RECENT_EXERCISE,
        EXERCISE_START,
        EXERCISE_KEY_PREVIOUS,
        EXERCISE_KEY_NEXT,
        EXERCISE_TONALITY_PREVIOUS,
        EXERCISE_TONALITY_NEXT,
        EXERCISE_PATTERN_PREVIOUS,
        EXERCISE_PATTERN_NEXT,
        EXERCISE_DIRECTION_PREVIOUS,
        EXERCISE_DIRECTION_NEXT,
        EXERCISE_HANDS_PREVIOUS,
        EXERCISE_HANDS_NEXT,
        EXERCISE_OCTAVES_PREVIOUS,
        EXERCISE_OCTAVES_NEXT,
        EXERCISE_TEMPO_PREVIOUS,
        EXERCISE_TEMPO_NEXT,
        EXERCISE_REPETITIONS_PREVIOUS,
        EXERCISE_REPETITIONS_NEXT,
        EXERCISE_RECENT_PREVIOUS,
        EXERCISE_RECENT_NEXT,
        EXERCISE_FAVOURITE_PREVIOUS,
        EXERCISE_FAVOURITE_NEXT,
        EXERCISE_FAVOURITE_TOGGLE,
        PLAYER_BACK,
        PLAYER_WAIT,
        PLAYER_COACH,
        PLAYER_HANDS,
        PLAYER_LOOP,
        PLAYER_FINGERINGS,
        #[cfg(feature = "score-verovio")]
        PLAYER_SCORE,
        #[cfg(feature = "score-verovio")]
        PLAYER_SCORE_ZOOM_OUT,
        #[cfg(feature = "score-verovio")]
        PLAYER_SCORE_ZOOM_IN,
        PLAYER_FINGERING_EDITOR,
        PLAYER_FINGERING_NEXT,
        PLAYER_FINGERING_ASSIGN_1,
        PLAYER_FINGERING_CLEAR,
        PLAYER_FINGERING_SUGGEST,
        PLAYER_FINGERING_ACCEPT,
        PLAYER_RESTART,
        COMPLETION_OVERVIEW,
        COMPLETION_TECHNIQUE,
        COMPLETION_HISTORY,
        COMPLETION_CALIBRATE,
        COMPLETION_NOTES_LOOP,
        COMPLETION_RHYTHM_LOOP,
        COMPLETION_RETRY,
        COMPLETION_BACK,
    ];
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum DebugPracticeAction {
    Back,
    ToggleWait,
    ToggleCoach,
    CycleHands,
    ToggleLoop,
    ToggleFingerings,
    #[cfg(feature = "score-verovio")]
    ToggleScore,
    #[cfg(feature = "score-verovio")]
    ScoreZoomOut,
    #[cfg(feature = "score-verovio")]
    ScoreZoomIn,
    ToggleFingeringEditor,
    NextFingeringTarget,
    AssignFingerOne,
    ClearFinger,
    SuggestFinger,
    AcceptFingerSuggestion,
    Restart,
    ShowOverview,
    ShowTechnique,
    ShowHistory,
    Retry,
}

#[cfg(debug_assertions)]
impl DebugPracticeAction {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            practice_ui_ids::PLAYER_BACK | practice_ui_ids::COMPLETION_BACK => Some(Self::Back),
            practice_ui_ids::PLAYER_WAIT => Some(Self::ToggleWait),
            practice_ui_ids::PLAYER_COACH => Some(Self::ToggleCoach),
            practice_ui_ids::PLAYER_HANDS => Some(Self::CycleHands),
            practice_ui_ids::PLAYER_LOOP => Some(Self::ToggleLoop),
            practice_ui_ids::PLAYER_FINGERINGS => Some(Self::ToggleFingerings),
            #[cfg(feature = "score-verovio")]
            practice_ui_ids::PLAYER_SCORE => Some(Self::ToggleScore),
            #[cfg(feature = "score-verovio")]
            practice_ui_ids::PLAYER_SCORE_ZOOM_OUT => Some(Self::ScoreZoomOut),
            #[cfg(feature = "score-verovio")]
            practice_ui_ids::PLAYER_SCORE_ZOOM_IN => Some(Self::ScoreZoomIn),
            practice_ui_ids::PLAYER_FINGERING_EDITOR => Some(Self::ToggleFingeringEditor),
            practice_ui_ids::PLAYER_FINGERING_NEXT => Some(Self::NextFingeringTarget),
            practice_ui_ids::PLAYER_FINGERING_ASSIGN_1 => Some(Self::AssignFingerOne),
            practice_ui_ids::PLAYER_FINGERING_CLEAR => Some(Self::ClearFinger),
            practice_ui_ids::PLAYER_FINGERING_SUGGEST => Some(Self::SuggestFinger),
            practice_ui_ids::PLAYER_FINGERING_ACCEPT => Some(Self::AcceptFingerSuggestion),
            practice_ui_ids::PLAYER_RESTART => Some(Self::Restart),
            practice_ui_ids::COMPLETION_OVERVIEW => Some(Self::ShowOverview),
            practice_ui_ids::COMPLETION_TECHNIQUE => Some(Self::ShowTechnique),
            practice_ui_ids::COMPLETION_HISTORY => Some(Self::ShowHistory),
            practice_ui_ids::COMPLETION_RETRY => Some(Self::Retry),
            _ => None,
        }
    }
}
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct FingeringTarget {
    track_id: usize,
    note_index: usize,
    start: Duration,
    pitch: u8,
    channel: u8,
}

#[derive(Debug, Clone)]
struct FingeringPreview {
    selected: usize,
    assignments: Vec<(FingeringTarget, FingerSuggestion)>,
}

#[derive(Debug, Clone)]
struct FingeringEditor {
    targets: Vec<FingeringTarget>,
    selected: usize,
    suggestion: Option<FingeringPreview>,
}

#[cfg(feature = "score-verovio")]
#[derive(Debug, Clone, Copy)]
struct ScorePageTexture {
    page_index: usize,
    image: neothesia_core::render::ImageIdentifier,
    width: u32,
    height: u32,
}

impl FingeringEditor {
    fn new(song: &Song, score_time: Duration) -> Option<Self> {
        let visible_tracks: std::collections::HashSet<_> = song
            .config
            .tracks
            .iter()
            .filter(|track| track.visible)
            .map(|track| track.track_id)
            .collect();
        let mut targets: Vec<_> = song
            .file
            .tracks
            .iter()
            .filter(|track| visible_tracks.contains(&track.track_id))
            .flat_map(|track| {
                track
                    .notes
                    .iter()
                    .enumerate()
                    .filter(|(_, note)| note.channel != 9)
                    .map(|(note_index, note)| FingeringTarget {
                        track_id: track.track_id,
                        note_index,
                        start: note.start,
                        pitch: note.note,
                        channel: note.channel,
                    })
            })
            .collect();
        targets.sort_by_key(|target| {
            (
                target.start,
                target.pitch,
                target.track_id,
                target.note_index,
            )
        });
        if targets.is_empty() {
            return None;
        }
        let selected = targets
            .partition_point(|target| target.start < score_time)
            .min(targets.len() - 1);
        Some(Self {
            targets,
            selected,
            suggestion: None,
        })
    }

    fn target(&self) -> FingeringTarget {
        self.targets[self.selected]
    }

    fn move_relative(&mut self, offset: isize) -> bool {
        let next = self
            .selected
            .saturating_add_signed(offset)
            .min(self.targets.len() - 1);
        let changed = next != self.selected;
        self.selected = next;
        if changed {
            self.suggestion = None;
        }
        changed
    }
}

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
    fingering_editor: Option<FingeringEditor>,
    #[cfg(feature = "score-verovio")]
    score_artifact: Option<crate::score_renderer_worker::RenderedScoreArtifact>,
    #[cfg(feature = "score-verovio")]
    score_render_generation: Option<crate::score_renderer_worker::ScoreRenderGeneration>,
    #[cfg(feature = "score-verovio")]
    score_pages: neothesia_core::score_view::ScorePageCache<
        crate::score_renderer_worker::RasterizedScorePage,
    >,
    #[cfg(feature = "score-verovio")]
    score_texture: Option<ScorePageTexture>,
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

        let has_fingerings = !song.fingerings.is_empty();
        let note_labels = (ctx.config.note_labels() || has_fingerings).then_some(
            NoteLabels::with_fingering_guidance(
                *keyboard.pos(),
                waterfall.notes(),
                ctx.text_renderer_factory.new_renderer(),
                ctx.config.note_labels(),
                song.fingerings.clone(),
                song.fingering_crossings.clone(),
                ctx.config.exercise_fingerings(),
            ),
        );

        let mut player = MidiPlayer::new(
            ctx.output_manager.connection().clone(),
            song,
            keyboard_layout.range.clone(),
            ctx.config.separate_channels(),
            ctx.config.wait_for_notes(),
        );
        player.set_input_latency_ms(ctx.config.input_latency_ms());
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
            completion_view: CompletionView::Overview,
            fingering_editor: None,
            #[cfg(feature = "score-verovio")]
            score_artifact: None,
            #[cfg(feature = "score-verovio")]
            score_render_generation: None,
            #[cfg(feature = "score-verovio")]
            score_pages: Default::default(),
            #[cfg(feature = "score-verovio")]
            score_texture: None,
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

    fn toggle_wait_for_notes(&mut self, ctx: &mut Context) {
        let enabled = !self.player.wait_for_notes();
        self.player.set_wait_for_notes(enabled);
        ctx.config.set_wait_for_notes(enabled);
    }

    fn toggle_tempo_coach(&mut self, ctx: &mut Context) {
        let enabled = !ctx.config.adaptive_tempo();
        ctx.config.set_adaptive_tempo(enabled);
        self.top_bar.reset_tempo_coach();
        self.toast_manager.toast(if enabled {
            "Tempo Coach ON: use loop practice for guided speed changes"
        } else {
            "Tempo Coach OFF: speed stays under manual control"
        });
    }

    fn toggle_fingerings(&mut self, ctx: &mut Context) -> bool {
        let Some(labels) = self.note_labels.as_mut() else {
            return false;
        };
        if !labels.toggle_fingerings() {
            return false;
        }
        let enabled = labels.fingerings_enabled();
        ctx.config.set_exercise_fingerings(enabled);
        ctx.config.save();
        self.toast_manager.toast(if enabled {
            "Reviewed fingering ON"
        } else {
            "Reviewed fingering OFF"
        });
        true
    }

    fn can_edit_fingerings(&self) -> bool {
        self.player.song().file.source_path.is_some() && self.player.song().exercise_spec.is_none()
    }

    fn fingering_editor_active(&self) -> bool {
        self.fingering_editor.is_some()
    }

    fn toggle_fingering_editor(&mut self, ctx: &mut Context) -> bool {
        if self.fingering_editor.take().is_some() {
            if let Some(labels) = self.note_labels.as_mut() {
                labels.clear_fingering_selection();
            }
            self.toast_manager
                .toast("Finger edit closed · saved hints remain visible");
            return true;
        }
        if !self.can_edit_fingerings() {
            self.toast_manager
                .toast("Export this exercise to MIDI before adding manual finger hints");
            return false;
        }
        let score_time = self.player.time().saturating_sub(*self.player.leed_in());
        let Some(editor) = FingeringEditor::new(self.player.song(), score_time) else {
            self.toast_manager
                .toast("No visible piano notes to annotate");
            return false;
        };
        self.player.pause();
        self.fingering_editor = Some(editor);
        self.ensure_fingering_labels(ctx);
        ctx.config.set_exercise_fingerings(true);
        ctx.config.save();
        self.seek_to_fingering_target();
        true
    }

    fn ensure_fingering_labels(&mut self, ctx: &mut Context) {
        if self.note_labels.is_some() {
            return;
        }
        self.note_labels = Some(NoteLabels::with_fingering_guidance(
            *self.keyboard.pos(),
            self.waterfall.notes(),
            ctx.text_renderer_factory.new_renderer(),
            ctx.config.note_labels(),
            self.player.song().fingerings.clone(),
            self.player.song().fingering_crossings.clone(),
            true,
        ));
    }

    fn move_fingering_target(&mut self, offset: isize) -> bool {
        let Some(editor) = self.fingering_editor.as_mut() else {
            return false;
        };
        editor.move_relative(offset);
        self.seek_to_fingering_target();
        true
    }

    fn seek_to_fingering_target(&mut self) {
        let Some(target) = self.fingering_editor.as_ref().map(FingeringEditor::target) else {
            return;
        };
        self.player.set_time(target.start + *self.player.leed_in());
        self.player.pause();
        self.refresh_fingering_selection(target);
        self.toast_fingering_target(target);
    }

    fn refresh_fingering_selection(&mut self, target: FingeringTarget) {
        let selected_key = fingering_key(target);
        let previews = self
            .fingering_editor
            .as_ref()
            .and_then(|editor| {
                editor
                    .suggestion
                    .as_ref()
                    .filter(|preview| preview.selected == editor.selected)
            })
            .map(|preview| {
                preview
                    .assignments
                    .iter()
                    .map(|(target, suggestion)| (fingering_key(*target), suggestion.finger))
                    .collect::<Vec<_>>()
            });
        if let Some(labels) = self.note_labels.as_mut() {
            if let Some(previews) = previews {
                labels.set_fingering_preview(selected_key, previews);
            } else {
                labels.set_fingering_selection(selected_key, None);
            }
        }
    }

    fn toast_fingering_target(&mut self, target: FingeringTarget) {
        let song = self.player.song();
        let measure = song
            .file
            .measures
            .partition_point(|start| *start <= target.start)
            .max(1);
        let part = song
            .config
            .tracks
            .iter()
            .find(|track| track.track_id == target.track_id)
            .map(|track| match track.practice_part {
                PracticePart::RightHand => "RH",
                PracticePart::LeftHand => "LH",
                PracticePart::Other => "Part",
            })
            .unwrap_or("Part");
        let pitch = format_midi_pitch(target.pitch);
        let current = song
            .fingerings
            .get(&(target.start, target.pitch, target.channel, target.track_id))
            .map(|finger| format!(" · current {finger}"))
            .unwrap_or_default();
        self.toast_manager.toast(format!(
            "FINGER EDIT · {part} M{measure} {pitch}{current} · ←/→ select · 1–5 assign · G suggest"
        ));
    }

    fn set_selected_finger(&mut self, finger: Option<u8>) -> bool {
        let Some(target) = self.fingering_editor.as_ref().map(FingeringEditor::target) else {
            return false;
        };
        let song = self.player.song();
        let Some(path) = song.file.source_path.clone() else {
            return false;
        };
        let hints = replace_finger_hint(song.manual_fingering_hints.clone(), target, finger);

        if let Err(error) = save_song_fingerings(&path, &song.file.content_id, hints.clone()) {
            self.toast_manager
                .toast(format!("Could not save finger hint: {error}"));
            return false;
        }

        let key = fingering_key(target);
        let song = self.player.song_mut();
        song.manual_fingering_hints = hints;
        if let Some(finger) = finger {
            song.fingerings.insert(key, finger);
        } else {
            song.fingerings.remove(&key);
            song.fingering_crossings.remove(&key);
        }
        if let Some(labels) = self.note_labels.as_mut() {
            labels.set_fingering(key, finger);
        }
        if let Some(editor) = self.fingering_editor.as_mut() {
            editor.suggestion = None;
        }

        if finger.is_some() {
            if let Some(editor) = self.fingering_editor.as_mut() {
                editor.move_relative(1);
            }
            self.seek_to_fingering_target();
        } else {
            self.refresh_fingering_selection(target);
            self.toast_fingering_target(target);
        }
        true
    }

    fn request_fingering_suggestion(&mut self, ctx: &Context) -> bool {
        let Some(target) = self.fingering_editor.as_ref().map(FingeringEditor::target) else {
            return false;
        };
        let song = self.player.song();
        let hand = song
            .config
            .tracks
            .iter()
            .find(|track| track.track_id == target.track_id)
            .and_then(|track| match track.practice_part {
                PracticePart::RightHand => Some(FingeringHand::Right),
                PracticePart::LeftHand => Some(FingeringHand::Left),
                PracticePart::Other => None,
            });
        let Some(hand) = hand else {
            self.toast_manager
                .toast("Suggestion unavailable: mark this track as left or right hand first");
            return false;
        };
        let Some(track) = song
            .file
            .tracks
            .iter()
            .find(|track| track.track_id == target.track_id)
        else {
            return false;
        };
        let anchors: std::collections::HashMap<_, _> = song
            .manual_fingering_hints
            .iter()
            .filter(|hint| hint.track_id == target.track_id)
            .map(|hint| (hint.note_index, hint.finger))
            .collect();
        let notes: Vec<_> = track
            .notes
            .iter()
            .enumerate()
            .map(|(note_index, note)| FingeringNote {
                pitch: note.note,
                onset: note.start,
                end: note.end,
                anchored_finger: (note_index != target.note_index)
                    .then(|| anchors.get(&note_index).copied())
                    .flatten(),
            })
            .collect();
        let suggestions =
            suggest_fingerings_with_profile(&notes, hand, ctx.config.hand_span_profile_for(hand));
        let Some(suggestion) = suggestions.get(target.note_index).copied().flatten() else {
            self.toast_manager
                .toast("No suggestion: unsupported chord size, duplicate pitch or anchor conflict");
            return false;
        };
        let onset = track.notes[target.note_index].start;
        let mut assignments: Vec<_> = track
            .notes
            .iter()
            .enumerate()
            .filter(|(_, note)| note.start == onset)
            .filter_map(|(note_index, note)| {
                Some((
                    FingeringTarget {
                        track_id: target.track_id,
                        note_index,
                        start: note.start,
                        pitch: note.note,
                        channel: note.channel,
                    },
                    suggestions.get(note_index).copied().flatten()?,
                ))
            })
            .collect();
        if assignments.is_empty()
            || assignments.len()
                != track
                    .notes
                    .iter()
                    .filter(|note| note.start == onset)
                    .count()
        {
            self.toast_manager
                .toast("No suggestion: the chord could not form one complete hand shape");
            return false;
        }
        assignments.sort_by_key(|(target, _)| (target.pitch, target.note_index));
        let selected = self.fingering_editor.as_ref().unwrap().selected;
        self.fingering_editor.as_mut().unwrap().suggestion = Some(FingeringPreview {
            selected,
            assignments: assignments.clone(),
        });
        self.refresh_fingering_selection(target);
        let shape = assignments
            .iter()
            .map(|(_, suggestion)| suggestion.finger.to_string())
            .collect::<Vec<_>>()
            .join("–");
        self.toast_manager.toast(if assignments.len() > 1 {
            format!(
                "SUGGEST CHORD {shape} · {}% · {} · Enter accepts all {}",
                suggestion.confidence_percent,
                suggestion.reason.explanation(hand),
                assignments.len()
            )
        } else {
            format!(
                "SUGGEST {shape} · {}% · {} · Enter accepts",
                suggestion.confidence_percent,
                suggestion.reason.explanation(hand)
            )
        });
        true
    }

    fn accept_fingering_suggestion(&mut self) -> bool {
        let Some(editor) = self.fingering_editor.as_ref() else {
            return false;
        };
        let Some(preview) = editor.suggestion.as_ref() else {
            self.toast_manager
                .toast("Press G to preview a suggestion before accepting it");
            return false;
        };
        if preview.selected != editor.selected {
            return false;
        }
        let assignments = preview.assignments.clone();
        self.set_suggested_fingers(&assignments)
    }

    #[cfg(debug_assertions)]
    fn pending_fingering_suggestion(&self) -> Option<FingerSuggestion> {
        let editor = self.fingering_editor.as_ref()?;
        let preview = editor.suggestion.as_ref()?;
        if preview.selected != editor.selected {
            return None;
        }
        let target = editor.target();
        preview
            .assignments
            .iter()
            .find_map(|(assignment, suggestion)| (*assignment == target).then_some(*suggestion))
    }

    #[cfg(debug_assertions)]
    fn pending_fingering_suggestion_count(&self) -> usize {
        let Some(editor) = self.fingering_editor.as_ref() else {
            return 0;
        };
        editor
            .suggestion
            .as_ref()
            .filter(|preview| preview.selected == editor.selected)
            .map_or(0, |preview| preview.assignments.len())
    }

    fn set_suggested_fingers(
        &mut self,
        assignments: &[(FingeringTarget, FingerSuggestion)],
    ) -> bool {
        if assignments.is_empty() {
            return false;
        }
        let song = self.player.song();
        let Some(path) = song.file.source_path.clone() else {
            return false;
        };
        let mut hints = song.manual_fingering_hints.clone();
        for (target, suggestion) in assignments {
            hints = replace_finger_hint(hints, *target, Some(suggestion.finger));
        }
        if let Err(error) = save_song_fingerings(&path, &song.file.content_id, hints.clone()) {
            self.toast_manager
                .toast(format!("Could not save finger hints: {error}"));
            return false;
        }

        let song = self.player.song_mut();
        song.manual_fingering_hints = hints;
        for (target, suggestion) in assignments {
            let key = fingering_key(*target);
            song.fingerings.insert(key, suggestion.finger);
            if let Some(labels) = self.note_labels.as_mut() {
                labels.set_fingering(key, Some(suggestion.finger));
            }
        }
        if let Some(editor) = self.fingering_editor.as_mut() {
            let last = editor
                .targets
                .iter()
                .rposition(|target| {
                    assignments
                        .iter()
                        .any(|(assignment, _)| assignment == target)
                })
                .unwrap_or(editor.selected);
            editor.selected = (last + 1).min(editor.targets.len() - 1);
            editor.suggestion = None;
        }
        self.seek_to_fingering_target();
        true
    }

    fn fingering_state(&self) -> (bool, bool) {
        self.note_labels
            .as_ref()
            .map(|labels| (labels.has_fingerings(), labels.fingerings_enabled()))
            .unwrap_or((false, false))
    }

    fn retry_practice(&mut self) {
        self.player.restart_practice();
        self.keyboard.reset_notes();
        self.completion = None;
        self.saved_session_count = None;
        self.completion_view = CompletionView::Overview;
    }

    fn restart_practice_scope(&mut self) {
        if self.top_bar.is_looper_active() {
            top_bar::restart_loop_take(self);
        } else {
            self.retry_practice();
        }
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
            source_path: self.player.song().file.source_path.clone(),
            exercise_spec: self.player.song().exercise_spec,
            last_used_unix_ms: 0,
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
        let rhythm_recommendation = ctx
            .practice_history
            .song(&self.player.song().file.content_id)
            .and_then(|history| history.recommended_rhythm_passage())
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
        let calibration =
            match timing_calibration_status(summary.timing, self.player.input_latency_ms()) {
                TimingCalibrationStatus::Suggested(suggestion) => Some(suggestion),
                _ => None,
            };

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
                    .size(panel_w - 324.0, 40.0)
                    .font_size(24.0)
                    .bold(true)
                    .text(if panel_w < 600.0 {
                        "Complete"
                    } else {
                        "Practice complete"
                    })
                    .build(ui);

                let (tab_x, tab_w, tab_gap) = completion_tab_layout(panel_w);
                if nuon::button()
                    .id(practice_ui_ids::COMPLETION_OVERVIEW)
                    .x(tab_x)
                    .y(26.0)
                    .size(tab_w, 34.0)
                    .label("Overview")
                    .color(if completion_view == CompletionView::Overview {
                        [56, 145, 255]
                    } else {
                        [61, 57, 73]
                    })
                    .hover_color([87, 165, 255])
                    .preseed_color([97, 175, 255])
                    .border_radius([8.0; 4])
                    .build(ui)
                {
                    requested_view = Some(CompletionView::Overview);
                }
                if nuon::button()
                    .id(practice_ui_ids::COMPLETION_TECHNIQUE)
                    .x(tab_x + tab_w + tab_gap)
                    .y(26.0)
                    .size(tab_w, 34.0)
                    .label("Technique")
                    .color(if completion_view == CompletionView::Technique {
                        [132, 96, 191]
                    } else {
                        [61, 57, 73]
                    })
                    .hover_color([151, 112, 215])
                    .preseed_color([163, 124, 227])
                    .border_radius([8.0; 4])
                    .build(ui)
                {
                    requested_view = Some(CompletionView::Technique);
                }
                if nuon::button()
                    .id(practice_ui_ids::COMPLETION_HISTORY)
                    .x(tab_x + (tab_w + tab_gap) * 2.0)
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

                if completion_view == CompletionView::Overview {
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
                        .y(138.0)
                        .size(panel_w - 56.0, 28.0)
                        .font_size(16.0)
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
                        .y(176.0)
                        .size(panel_w - 56.0, 30.0)
                        .font_size(17.0)
                        .text(format!(
                            "Wrong {}    Missed {}",
                            summary.overall.wrong_notes, summary.overall.missed_notes
                        ))
                        .build(ui);

                    nuon::label()
                        .x(28.0)
                        .y(210.0)
                        .size(panel_w - 56.0, 24.0)
                        .font_size(16.0)
                        .text(format!(
                            "Right hand {}    Left hand {}",
                            format_accuracy(part_accuracy(PracticePart::RightHand)),
                            format_accuracy(part_accuracy(PracticePart::LeftHand))
                        ))
                        .build(ui);

                    nuon::quad()
                        .x(28.0)
                        .y(254.0)
                        .size(panel_w - 56.0, 1.0)
                        .color([83, 78, 98])
                        .build(ui);

                    nuon::label()
                        .x(28.0)
                        .y(272.0)
                        .size(panel_w - 56.0, 44.0)
                        .font_size(17.0)
                        .text(review)
                        .build(ui);

                    if let Some(session_count) = self.saved_session_count {
                        nuon::label()
                            .x(28.0)
                            .y(320.0)
                            .size(panel_w - 56.0, 28.0)
                            .font_size(14.0)
                            .color([143, 205, 171])
                            .text(format!(
                                "Saved locally  ·  {session_count} session{} for this MIDI",
                                if session_count == 1 { "" } else { "s" }
                            ))
                            .build(ui);
                    }
                } else if completion_view == CompletionView::Technique {
                    nuon::label()
                        .x(28.0)
                        .y(88.0)
                        .size(panel_w - 56.0, 38.0)
                        .font_size(26.0)
                        .bold(true)
                        .text("Timing & coordination")
                        .build(ui);

                    nuon::label()
                        .x(28.0)
                        .y(132.0)
                        .size(
                            if calibration.is_some() {
                                panel_w - 244.0
                            } else {
                                panel_w - 56.0
                            },
                            24.0,
                        )
                        .font_size(14.0)
                        .color([184, 178, 205])
                        .text(format_timing_profile(
                            summary.timing,
                            self.player.input_latency_ms(),
                        ))
                        .build(ui);

                    if let Some(suggestion) = calibration
                        && nuon::button()
                            .id(practice_ui_ids::COMPLETION_CALIBRATE)
                            .x(panel_w - 204.0)
                            .y(128.0)
                            .size(176.0, 30.0)
                            .label(format!("Apply {:+} ms & retry", suggestion.suggested_ms))
                            .color([132, 96, 191])
                            .hover_color([151, 112, 215])
                            .preseed_color([163, 124, 227])
                            .border_radius([7.0; 4])
                            .build(ui)
                    {
                        action = Some(CompletionAction::ApplyCalibration {
                            suggested_ms: suggestion.suggested_ms,
                        });
                    }

                    nuon::label()
                        .x(28.0)
                        .y(170.0)
                        .size(panel_w - 56.0, 24.0)
                        .font_size(14.0)
                        .color([184, 178, 205])
                        .text(format_hand_timing(&summary.parts))
                        .build(ui);

                    nuon::label()
                        .x(28.0)
                        .y(202.0)
                        .size(panel_w - 56.0, 24.0)
                        .font_size(14.0)
                        .color([184, 178, 205])
                        .text(
                            format_exercise_passes(&summary.exercise_passes)
                                .unwrap_or_else(|| format_chord_profile(summary.chords)),
                        )
                        .build(ui);

                    nuon::quad()
                        .x(28.0)
                        .y(238.0)
                        .size(panel_w - 56.0, 1.0)
                        .color([83, 78, 98])
                        .build(ui);

                    nuon::label()
                        .x(28.0)
                        .y(256.0)
                        .size(panel_w - 56.0, 34.0)
                        .font_size(22.0)
                        .bold(true)
                        .text("Expression evidence")
                        .build(ui);

                    if ctx.config.expression_feedback() {
                        let (dynamics, contour, pedal, pedal_timing, articulation) =
                            format_expression_summary(summary.expression);
                        for (index, text) in [dynamics, contour, pedal, pedal_timing, articulation]
                            .into_iter()
                            .enumerate()
                        {
                            nuon::label()
                                .x(28.0)
                                .y(296.0 + index as f32 * 30.0)
                                .size(panel_w - 56.0, 28.0)
                                .font_size(13.5)
                                .color([184, 178, 205])
                                .text(text)
                                .build(ui);
                        }
                    } else {
                        nuon::label()
                            .x(28.0)
                            .y(300.0)
                            .size(panel_w - 56.0, 28.0)
                            .font_size(14.0)
                            .color([143, 139, 156])
                            .text("Expression Summary is disabled in Settings")
                            .build(ui);
                    }
                } else {
                    render_history_overview(ui, panel_w, history_overview.as_ref());
                }

                let button_gap = 12.0;
                let button_y = panel_h - 68.0;
                let button_w = (panel_w - 56.0 - button_gap) / 2.0;
                let has_note_recommendation = recommendation.is_some();

                if completion_view == CompletionView::Overview
                    && let Some(recommendation) = recommendation
                    && nuon::button()
                        .id(practice_ui_ids::COMPLETION_NOTES_LOOP)
                        .x(28.0)
                        .y(button_y - 56.0)
                        .size(
                            if rhythm_recommendation.is_some() {
                                button_w
                            } else {
                                panel_w - 56.0
                            },
                            44.0,
                        )
                        .label(format!(
                            "Notes m.{}–{} · {}% / {} takes",
                            recommendation.start_measure,
                            recommendation.end_measure,
                            percent(Some(recommendation.weakest_accuracy)),
                            recommendation.attempts,
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

                if completion_view == CompletionView::Overview
                    && let Some(recommendation) = rhythm_recommendation
                    && nuon::button()
                        .id(practice_ui_ids::COMPLETION_RHYTHM_LOOP)
                        .x(if has_note_recommendation {
                            28.0 + button_w + button_gap
                        } else {
                            28.0
                        })
                        .y(button_y - 56.0)
                        .size(
                            if has_note_recommendation {
                                button_w
                            } else {
                                panel_w - 56.0
                            },
                            44.0,
                        )
                        .label(format!(
                            "Rhythm m.{}–{} · {}ms spread",
                            recommendation.start_measure,
                            recommendation.end_measure,
                            recommendation.median_deviation_ms,
                        ))
                        .color([132, 96, 191])
                        .hover_color([151, 112, 215])
                        .preseed_color([163, 124, 227])
                        .border_radius([8.0; 4])
                        .build(ui)
                {
                    action = Some(CompletionAction::PracticeRhythm {
                        start_measure: recommendation.start_measure,
                        end_measure: recommendation.end_measure,
                    });
                }

                if nuon::button()
                    .id(practice_ui_ids::COMPLETION_RETRY)
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
                    .id(practice_ui_ids::COMPLETION_BACK)
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
                    self.completion_view = CompletionView::Overview;
                }
            }
            Some(CompletionAction::ApplyCalibration { suggested_ms }) => {
                ctx.config.set_input_latency_ms(suggested_ms);
                self.player.set_input_latency_ms(suggested_ms);
                self.player.restart_practice();
                self.keyboard.reset_notes();
                self.completion = None;
                self.saved_session_count = None;
                self.completion_view = CompletionView::Overview;
                self.toast_manager.toast(format!(
                    "Timing offset saved at {suggested_ms:+} ms · verify with this take"
                ));
            }
            Some(CompletionAction::PracticeRhythm {
                start_measure,
                end_measure,
            }) => {
                if top_bar::begin_measure_loop(self, start_measure, end_measure) {
                    self.completion = None;
                    self.saved_session_count = None;
                    self.completion_view = CompletionView::Overview;
                    self.toast_manager.toast(format!(
                        "Rhythm focus: measures {start_measure}–{end_measure}"
                    ));
                }
            }
            Some(CompletionAction::Retry) => {
                self.retry_practice();
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
    Overview,
    Technique,
    History,
}

impl CompletionView {
    #[cfg(debug_assertions)]
    fn label(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Technique => "technique",
            Self::History => "history",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Overview => Self::Technique,
            Self::Technique => Self::History,
            Self::History => Self::Overview,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Overview => Self::History,
            Self::Technique => Self::Overview,
            Self::History => Self::Technique,
        }
    }
}

fn completion_tab_layout(panel_width: f32) -> (f32, f32, f32) {
    let gap = 8.0;
    let width = 84.0;
    let x = panel_width - 28.0 - width * 3.0 - gap * 2.0;
    (x, width, gap)
}

#[derive(Debug, Clone, Copy)]
enum CompletionAction {
    PracticeWeak {
        start_measure: usize,
        end_measure: usize,
    },
    ApplyCalibration {
        suggested_ms: i32,
    },
    PracticeRhythm {
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
            format_tempo_or_speed_trend(overview.tempo_bpm_delta, overview.speed_delta)
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
                "{}. {}  ·  {} accuracy  ·  {}",
                index + 1,
                format_practice_scope(session.kind, session.hands),
                accuracy,
                format_attempt_tempo(session.effective_tempo_bpm, session.speed)
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

fn format_exercise_passes(
    passes: &[neothesia_core::practice::ExercisePassSummary],
) -> Option<String> {
    let accuracies: Vec<_> = passes
        .iter()
        .filter_map(|pass| pass.breakdown.accuracy())
        .collect();
    if accuracies.len() < 2 {
        return None;
    }
    let first = accuracies[0];
    let last = *accuracies.last().unwrap();
    let minimum = accuracies.iter().copied().fold(f32::INFINITY, f32::min);
    let maximum = accuracies.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let trend = if last - first >= 0.05 {
        "improved across passes"
    } else if first - last >= 0.05 {
        "accuracy fell in later passes"
    } else if maximum - minimum <= 0.05 {
        "held steady"
    } else {
        "varied between passes"
    };
    let sequence = passes
        .iter()
        .map(|pass| format_accuracy(pass.breakdown.accuracy()))
        .collect::<Vec<_>>()
        .join(" → ");
    let timing = passes
        .first()
        .and_then(|pass| pass.timing.median_deviation_ms)
        .zip(
            passes
                .last()
                .and_then(|pass| pass.timing.median_deviation_ms),
        )
        .map(|(first, last)| format!(" · timing spread {first}→{last}ms"))
        .unwrap_or_default();
    Some(format!("Pass accuracy: {sequence} · {trend}{timing}"))
}

fn format_attempt_tempo(effective_tempo_bpm: Option<u16>, speed: f32) -> String {
    match effective_tempo_bpm {
        Some(tempo) => format!("{tempo} BPM · {} speed", format_speed(speed)),
        None => format!("{} speed", format_speed(speed)),
    }
}

fn format_tempo_or_speed_trend(tempo_delta: Option<i32>, speed_delta: Option<f32>) -> String {
    match tempo_delta {
        Some(delta) if delta > 0 => format!("tempo: +{delta} BPM"),
        Some(delta) => format!("tempo: {delta} BPM"),
        None => format_trend("speed", speed_delta),
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

fn format_timing_profile(
    timing: neothesia_core::practice::TimingSummary,
    current_calibration_ms: i32,
) -> String {
    if !timing.has_profile() {
        return format!(
            "Timing profile: need 8 matched notes · {} captured",
            timing.matched_samples
        );
    }

    let offset = timing.median_offset_ms.unwrap_or(0);
    let bias = if offset < 0 {
        format!("{} ms early", offset.unsigned_abs())
    } else if offset > 0 {
        format!("{} ms late", offset)
    } else {
        "centered".to_owned()
    };
    let mut profile = format!(
        "Timing profile: median {bias} · typical spread {} ms",
        timing.median_deviation_ms.unwrap_or(0)
    );
    match timing_calibration_status(timing, current_calibration_ms) {
        TimingCalibrationStatus::InsufficientSamples { required, .. } => {
            profile.push_str(&format!(" · calibration needs {required}"));
        }
        TimingCalibrationStatus::Unstable { .. } => {
            profile.push_str(" · too variable to calibrate");
        }
        TimingCalibrationStatus::Centered => {
            profile.push_str(" · calibration looks centered");
        }
        TimingCalibrationStatus::AtLimit => {
            profile.push_str(" · calibration is at its limit");
        }
        TimingCalibrationStatus::Suggested(_) => {}
    }
    profile
}

fn format_hand_timing(parts: &[neothesia_core::practice::PartSummary]) -> String {
    let describe = |part| {
        let timing = parts
            .iter()
            .find(|summary| summary.part == part)
            .map(|summary| summary.timing)
            .unwrap_or_default();
        if !timing.has_profile() {
            return format!(
                "need {} more",
                8usize.saturating_sub(timing.matched_samples)
            );
        }
        let offset = timing.median_offset_ms.unwrap_or(0);
        let bias = if offset < 0 {
            format!("{}ms early", offset.unsigned_abs())
        } else if offset > 0 {
            format!("{offset}ms late")
        } else {
            "centered".to_owned()
        };
        format!(
            "{bias}, spread {}ms",
            timing.median_deviation_ms.unwrap_or(0)
        )
    };

    format!(
        "Hand timing · right: {} · left: {}",
        describe(PracticePart::RightHand),
        describe(PracticePart::LeftHand)
    )
}

fn format_chord_profile(chords: neothesia_core::practice::ChordSummary) -> String {
    if chords.eligible_chords == 0 {
        return "Chord attacks: no exact-onset block chords in this take".to_owned();
    }
    if !chords.has_profile() {
        return format!(
            "Chord attacks: need 4 complete chords · {} complete / {} incomplete",
            chords.complete_chords, chords.incomplete_chords
        );
    }
    format!(
        "Chord attacks (descriptive): median span {}ms · max {}ms · {}/{} complete",
        chords.median_attack_span_ms.unwrap_or(0),
        chords.maximum_attack_span_ms.unwrap_or(0),
        chords.complete_chords,
        chords.eligible_chords,
    )
}

fn format_expression_summary(
    expression: ExpressionSummary,
) -> (String, String, String, String, String) {
    let velocity = expression.velocity;
    let dynamics = if expression.has_velocity_evidence() {
        format!(
            "Dynamics (descriptive): played {}–{} · score {}–{} · avg gap {}",
            velocity.played_min.unwrap_or(0),
            velocity.played_max.unwrap_or(0),
            velocity.target_min.unwrap_or(0),
            velocity.target_max.unwrap_or(0),
            velocity.mean_abs_difference.unwrap_or(0),
        )
    } else {
        format!(
            "Dynamics: need 4 matched notes · {} captured",
            velocity.matched_samples
        )
    };
    let contour = if expression.has_velocity_contour_evidence() {
        format!(
            "Dynamics contour (descriptive): {}/{} followed · {} flat · {} opposite",
            velocity.contour_aligned,
            velocity.contour_steps,
            velocity.contour_flat,
            velocity.contour_inverted,
        )
    } else {
        format!(
            "Dynamics contour: need 6 shaped steps · {} captured",
            velocity.contour_steps
        )
    };

    let pedal_summary = expression.pedal;
    let continuous = if pedal_summary.user_continuous_samples != 0
        || pedal_summary.target_continuous_samples != 0
    {
        " · continuous values seen"
    } else {
        ""
    };
    let pedal = match (pedal_summary.target_present, pedal_summary.user_used) {
        (true, true) => format!(
            "Pedal (counts only): user {} changes · score {}{continuous}",
            pedal_summary.user_changes, pedal_summary.target_changes
        ),
        (true, false) => "Pedal: score contains sustain data · no user use captured".to_owned(),
        (false, true) => format!(
            "Pedal: {} user changes captured · score has no pedal reference",
            pedal_summary.user_changes
        ),
        (false, false) => "Pedal: no sustain evidence in this take".to_owned(),
    };
    let pedal_timing = if !pedal_summary.target_present || !pedal_summary.user_used {
        "Pedal timing: needs both score and user sustain".to_owned()
    } else if pedal_summary.timing_samples < 4 {
        format!(
            "Pedal timing: need 4 paired down/up moves · {} captured",
            pedal_summary.timing_samples
        )
    } else if pedal_summary.user_transitions != pedal_summary.target_transitions {
        format!(
            "Pedal timing: transition mismatch · user {} / score {}",
            pedal_summary.user_transitions, pedal_summary.target_transitions
        )
    } else if !expression.has_pedal_timing_evidence() {
        format!(
            "Pedal timing: spread {}ms is too variable for a stable profile",
            pedal_summary.median_deviation_ms.unwrap_or(0)
        )
    } else {
        let offset = pedal_summary.median_offset_ms.unwrap_or(0);
        let bias = if offset < 0 {
            format!("{}ms early", offset.unsigned_abs())
        } else if offset > 0 {
            format!("{offset}ms late")
        } else {
            "centered".to_owned()
        };
        format!(
            "Pedal timing (descriptive): median {bias} · spread {}ms · {} pairs",
            pedal_summary.median_deviation_ms.unwrap_or(0),
            pedal_summary.timing_samples,
        )
    };

    let articulation = expression.articulation;
    let articulation = if expression.has_articulation_evidence() {
        format!(
            "Key hold vs score: median {}% · <75% {} / 75–125% {} / >125% {}",
            articulation.median_duration_ratio_percent.unwrap_or(0),
            articulation.shorter_count,
            articulation.similar_count,
            articulation.longer_count,
        )
    } else {
        format!(
            "Key hold: need 4 completed notes · {} captured (pedal excluded)",
            articulation.matched_samples
        )
    };

    (dynamics, contour, pedal, pedal_timing, articulation)
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
        )
        .with_effective_tempo_bpm(player.song().effective_tempo_bpm(speed)),
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

fn format_midi_pitch(pitch: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
    ];
    format!(
        "{}{}",
        NAMES[usize::from(pitch % 12)],
        i16::from(pitch) / 12 - 1
    )
}

fn fingering_key(target: FingeringTarget) -> (Duration, u8, u8, usize) {
    (target.start, target.pitch, target.channel, target.track_id)
}

fn replace_finger_hint(
    mut hints: Vec<FingerHint>,
    target: FingeringTarget,
    finger: Option<u8>,
) -> Vec<FingerHint> {
    hints
        .retain(|hint| !(hint.track_id == target.track_id && hint.note_index == target.note_index));
    if let Some(finger) = finger.filter(|finger| (1..=5).contains(finger)) {
        hints.push(FingerHint {
            track_id: target.track_id,
            note_index: target.note_index,
            finger,
        });
    }
    hints.sort_by_key(|hint| (hint.track_id, hint.note_index));
    hints
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

#[cfg(feature = "score-verovio")]
impl PlayingScene {
    fn score_available(&self) -> bool {
        self.score_artifact.is_some()
    }

    fn score_visible(&self, ctx: &Context) -> bool {
        self.score_available() && ctx.config.score_visible()
    }

    fn toggle_score_visibility(&mut self, ctx: &mut Context) -> bool {
        if !self.score_available() {
            return false;
        }
        let visible = !ctx.config.score_visible();
        ctx.config.set_score_visible(visible);
        ctx.config.save();
        if visible {
            self.upload_focused_score_page(ctx);
        } else if let Some(previous) = self.score_texture.take() {
            self.nuon_renderer.remove_image(previous.image);
        }
        self.toast_manager
            .toast(if visible { "Score ON" } else { "Score OFF" });
        true
    }

    fn adjust_score_zoom(&mut self, ctx: &mut Context, delta: i8) -> bool {
        if !self.score_available() {
            return false;
        }
        let current = ctx.config.score_zoom_percent();
        let next = (i16::from(current) + i16::from(delta)).clamp(
            i16::from(neothesia_core::config::SCORE_ZOOM_MIN_PERCENT),
            i16::from(neothesia_core::config::SCORE_ZOOM_MAX_PERCENT),
        ) as u8;
        if next != current {
            ctx.config.set_score_zoom_percent(next);
            ctx.config.save();
            self.toast_manager.toast(format!("Score size: {next}%"));
        } else {
            self.toast_manager
                .toast(format!("Score size limit: {current}%"));
        }
        true
    }

    fn follow_score_playback(&mut self, ctx: &mut Context) -> bool {
        let score_time = self.player.time().saturating_sub(*self.player.leed_in());
        let target_page = self
            .score_artifact
            .as_ref()
            .and_then(|artifact| artifact.synchronization.as_ref())
            .and_then(|synchronization| score_focus_page(synchronization, score_time));
        let Some(target_page) = target_page else {
            return false;
        };
        if self.score_pages.focus() == Some(target_page) {
            return false;
        }
        let Ok(requests) = self.score_pages.focus_page(target_page) else {
            return false;
        };
        if let Some(previous) = self.score_texture.take() {
            self.nuon_renderer.remove_image(previous.image);
        }
        if ctx.config.score_visible() {
            self.upload_focused_score_page(ctx);
        }
        if !requests.is_empty()
            && let (Some(artifact), Some(generation)) =
                (&self.score_artifact, self.score_render_generation)
        {
            crate::score_renderer_worker::start_page_loads(
                artifact,
                generation,
                requests,
                ctx.proxy.clone(),
            );
        }
        true
    }

    fn active_score_bounds(
        &self,
        page_index: usize,
    ) -> Vec<crate::score_renderer_worker::ScoreElementBounds> {
        let score_time = self.player.time().saturating_sub(*self.player.leed_in());
        let Some(synchronization) = self
            .score_artifact
            .as_ref()
            .and_then(|artifact| artifact.synchronization.as_ref())
        else {
            return Vec::new();
        };
        let Some(page) = self.score_pages.page(page_index) else {
            return Vec::new();
        };
        synchronization
            .timeline
            .frame_at(score_time, &synchronization.index)
            .active
            .into_iter()
            .filter(|highlight| highlight.page_index == page_index)
            .filter_map(|highlight| page.element_bounds.get(&highlight.renderer_id).copied())
            .collect()
    }

    #[cfg(debug_assertions)]
    fn active_score_highlight_count(&self) -> usize {
        let score_time = self.player.time().saturating_sub(*self.player.leed_in());
        self.score_artifact
            .as_ref()
            .and_then(|artifact| artifact.synchronization.as_ref())
            .map_or(0, |synchronization| {
                synchronization
                    .timeline
                    .frame_at(score_time, &synchronization.index)
                    .active
                    .len()
            })
    }

    fn upload_focused_score_page(&mut self, ctx: &mut Context) -> bool {
        let Some(page_index) = self.score_pages.focus() else {
            return false;
        };
        if !score_page_needs_upload(
            Some(page_index),
            self.score_texture.map(|texture| texture.page_index),
        ) {
            return false;
        }
        let Some(page) = self.score_pages.page(page_index) else {
            return false;
        };
        let Some(image) = neothesia_core::render::Image::from_rgba(
            &ctx.gpu.device,
            &ctx.gpu.queue,
            page.rgba.clone(),
            page.width,
            page.height,
        ) else {
            log::warn!("Focused score page has invalid RGBA dimensions");
            return false;
        };
        if let Some(previous) = self.score_texture.take() {
            self.nuon_renderer.remove_image(previous.image);
        }
        let texture = ScorePageTexture {
            page_index,
            image: self.nuon_renderer.add_image(image),
            width: page.width,
            height: page.height,
        };
        self.score_texture = Some(texture);
        true
    }

    fn score_page_ui(&mut self, ctx: &mut Context) {
        let Some(texture) = self.score_texture else {
            return;
        };
        let active_bounds = self.active_score_bounds(texture.page_index);
        let viewport = ctx
            .window_state
            .physical_size
            .to_logical::<f32>(ctx.window_state.scale_factor);
        let (x, y, width, height) = score_page_layout(
            viewport.width,
            viewport.height,
            texture.width,
            texture.height,
            ctx.config.score_zoom_percent(),
        );
        let mut ui = std::mem::replace(&mut self.nuon, nuon::Ui::new());
        nuon::quad()
            .pos(x - 8.0, y - 32.0)
            .size(width + 16.0, height + 40.0)
            .color([24, 22, 31, 238])
            .border_radius([8.0; 4])
            .build(&mut ui);
        nuon::label()
            .pos(x, y - 27.0)
            .size((width - 112.0).max(1.0), 20.0)
            .font_size(13.0)
            .color([205, 202, 216])
            .text(format!(
                "Score  ·  page {} of {}  ·  size {}%",
                texture.page_index + 1,
                self.score_artifact
                    .as_ref()
                    .map_or(1, |artifact| artifact.manifest.page_count),
                ctx.config.score_zoom_percent(),
            ))
            .build(&mut ui);
        let zoom = ctx.config.score_zoom_percent();
        let zoom_out_color = if zoom == neothesia_core::config::SCORE_ZOOM_MIN_PERCENT {
            [54, 51, 63]
        } else {
            [74, 68, 88]
        };
        if nuon::button()
            .id(practice_ui_ids::PLAYER_SCORE_ZOOM_OUT)
            .pos(x + width - 66.0, y - 29.0)
            .size(30.0, 24.0)
            .label("-")
            .color(zoom_out_color)
            .hover_color([67, 136, 199])
            .preseed_color([77, 146, 209])
            .border_radius([4.0; 4])
            .build(&mut ui)
        {
            self.adjust_score_zoom(
                ctx,
                -(neothesia_core::config::SCORE_ZOOM_STEP_PERCENT as i8),
            );
        }
        let zoom_in_color = if zoom == neothesia_core::config::SCORE_ZOOM_MAX_PERCENT {
            [54, 51, 63]
        } else {
            [74, 68, 88]
        };
        if nuon::button()
            .id(practice_ui_ids::PLAYER_SCORE_ZOOM_IN)
            .pos(x + width - 30.0, y - 29.0)
            .size(30.0, 24.0)
            .label("+")
            .color(zoom_in_color)
            .hover_color([67, 136, 199])
            .preseed_color([77, 146, 209])
            .border_radius([4.0; 4])
            .build(&mut ui)
        {
            self.adjust_score_zoom(ctx, neothesia_core::config::SCORE_ZOOM_STEP_PERCENT as i8);
        }
        nuon::image(texture.image)
            .pos(x, y)
            .size(width, height)
            .border_radius([4.0; 4])
            .build(&mut ui);
        for bounds in active_bounds {
            let (highlight_x, highlight_y, highlight_width, highlight_height) =
                score_highlight_layout(x, y, width, height, texture.width, texture.height, bounds);
            score_highlight_ui(
                &mut ui,
                highlight_x,
                highlight_y,
                highlight_width,
                highlight_height,
            );
        }
        self.nuon = ui;
    }
}

#[cfg(feature = "score-verovio")]
fn score_highlight_ui(ui: &mut nuon::Ui, x: f32, y: f32, width: f32, height: f32) {
    const EDGE: f32 = 2.0;
    nuon::quad()
        .pos(x, y)
        .size(width, height)
        .color([45, 126, 210, 48])
        .border_radius([3.0; 4])
        .build(ui);
    for (edge_x, edge_y, edge_width, edge_height) in [
        (x, y, width, EDGE),
        (x, y + height - EDGE, width, EDGE),
        (x, y, EDGE, height),
        (x + width - EDGE, y, EDGE, height),
    ] {
        nuon::quad()
            .pos(edge_x, edge_y)
            .size(edge_width, edge_height)
            .color([34, 105, 184, 230])
            .border_radius([1.0; 4])
            .build(ui);
    }
}

#[cfg(feature = "score-verovio")]
#[allow(clippy::too_many_arguments)]
fn score_highlight_layout(
    page_x: f32,
    page_y: f32,
    page_width: f32,
    page_height: f32,
    source_width: u32,
    source_height: u32,
    bounds: crate::score_renderer_worker::ScoreElementBounds,
) -> (f32, f32, f32, f32) {
    let scale_x = page_width / source_width.max(1) as f32;
    let scale_y = page_height / source_height.max(1) as f32;
    let padding = 3.0;
    let left = (page_x + bounds.x as f32 * scale_x - padding).max(page_x);
    let top = (page_y + bounds.y as f32 * scale_y - padding).max(page_y);
    let right =
        (page_x + (bounds.x + bounds.width) as f32 * scale_x + padding).min(page_x + page_width);
    let bottom =
        (page_y + (bounds.y + bounds.height) as f32 * scale_y + padding).min(page_y + page_height);
    (left, top, (right - left).max(4.0), (bottom - top).max(4.0))
}

#[cfg(feature = "score-verovio")]
fn score_focus_page(
    synchronization: &crate::score_renderer_worker::ScoreSynchronization,
    score_time: Duration,
) -> Option<usize> {
    synchronization
        .timeline
        .frame_at(score_time, &synchronization.index)
        .focus
        .map(|focus| focus.page_index)
}

#[cfg(feature = "score-verovio")]
fn score_page_layout(
    window_width: f32,
    window_height: f32,
    page_width: u32,
    page_height: u32,
    zoom_percent: u8,
) -> (f32, f32, f32, f32) {
    const PAGE_TOP: f32 = 54.0;
    const KEYBOARD_RESERVE: f32 = 160.0;
    let available_width = (window_width - 48.0).max(1.0);
    let zoom_percent = zoom_percent.clamp(
        neothesia_core::config::SCORE_ZOOM_MIN_PERCENT,
        neothesia_core::config::SCORE_ZOOM_MAX_PERCENT,
    );
    let available_height = (window_height * f32::from(zoom_percent) / 100.0)
        .min((window_height - PAGE_TOP - KEYBOARD_RESERVE).max(1.0))
        .max(1.0);
    let scale = (available_width / page_width.max(1) as f32)
        .min(available_height / page_height.max(1) as f32);
    let width = page_width.max(1) as f32 * scale;
    let height = page_height.max(1) as f32 * scale;
    (nuon::center_x(window_width, width), PAGE_TOP, width, height)
}

#[cfg(feature = "score-verovio")]
fn score_page_needs_upload(focused_page: Option<usize>, texture_page: Option<usize>) -> bool {
    focused_page.is_some() && focused_page != texture_page
}

impl Scene for PlayingScene {
    #[cfg(feature = "score-verovio")]
    fn score_artifact_ready(
        &mut self,
        ctx: &mut Context,
        generation: crate::score_renderer_worker::ScoreRenderGeneration,
        artifact: crate::score_renderer_worker::RenderedScoreArtifact,
    ) -> bool {
        if let Some(previous) = self.score_texture.take() {
            self.nuon_renderer.remove_image(previous.image);
        }
        if self
            .score_pages
            .begin_document(artifact.manifest.page_count)
            .is_err()
        {
            return false;
        }
        let Ok(requests) = self.score_pages.focus_page(0) else {
            return false;
        };
        self.score_render_generation = Some(generation);
        self.score_artifact = Some(artifact);
        crate::score_renderer_worker::start_page_loads(
            self.score_artifact.as_ref().unwrap(),
            generation,
            requests,
            ctx.proxy.clone(),
        );
        true
    }

    #[cfg(feature = "score-verovio")]
    fn score_page_ready(
        &mut self,
        ctx: &mut Context,
        generation: crate::score_renderer_worker::ScoreRenderGeneration,
        request: neothesia_core::score_view::ScorePageRequest,
        result: Result<crate::score_renderer_worker::RasterizedScorePage, String>,
    ) -> bool {
        if self.score_render_generation != Some(generation) {
            return false;
        }
        let disposition = match result {
            Ok(page) => self.score_pages.complete(request, page),
            Err(error) => {
                log::warn!("Score page {} failed to load: {error}", request.page_index);
                self.score_pages.fail(request)
            }
        };
        if disposition != neothesia_core::score_view::PageLoadDisposition::Accepted {
            return false;
        }
        if ctx.config.score_visible() && self.score_pages.focus() == Some(request.page_index) {
            self.upload_focused_score_page(ctx);
        }
        true
    }

    #[profiling::function]
    fn update(&mut self, ctx: &mut Context, delta: Duration) {
        self.quad_renderer_bg.clear();
        self.quad_renderer_fg.clear();

        self.rewind_controller.update(&mut self.player, ctx, delta);
        self.toast_manager.update(&mut self.text_renderer);

        let time = self.update_midi_player(ctx, delta);
        #[cfg(feature = "score-verovio")]
        self.follow_score_playback(ctx);
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

        #[cfg(feature = "score-verovio")]
        self.score_page_ui(ctx);
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
            if event.key_released(Key::Named(NamedKey::ArrowRight)) {
                self.completion_view = self.completion_view.next();
            }
            if event.key_released(Key::Named(NamedKey::ArrowLeft)) {
                self.completion_view = self.completion_view.previous();
            }
            if event.window_resized() || event.scale_factor_changed() {
                self.resize(ctx)
            }
            super::handle_nuon_window_event(&mut self.nuon, event, ctx);
            return;
        }

        if ctx.window_state.modifiers_state.control_key() && event.key_released(Key::Character("i"))
        {
            self.toggle_fingering_editor(ctx);
            return;
        }
        if self.fingering_editor_active() {
            if event.key_released(Key::Named(NamedKey::Escape)) {
                self.toggle_fingering_editor(ctx);
                return;
            }
            if event.key_released(Key::Named(NamedKey::ArrowLeft)) {
                self.move_fingering_target(-1);
                return;
            }
            if event.key_released(Key::Named(NamedKey::ArrowRight)) {
                self.move_fingering_target(1);
                return;
            }
            if event.key_released(Key::Named(NamedKey::Delete))
                || event.key_released(Key::Named(NamedKey::Backspace))
            {
                self.set_selected_finger(None);
                return;
            }
            if event.key_released(Key::Character("g")) {
                self.request_fingering_suggestion(ctx);
                return;
            }
            if event.key_released(Key::Named(NamedKey::Enter)) {
                self.accept_fingering_suggestion();
                return;
            }
            for (key, finger) in [("1", 1), ("2", 2), ("3", 3), ("4", 4), ("5", 5)] {
                if event.key_released(Key::Character(key)) {
                    self.set_selected_finger(Some(finger));
                    return;
                }
            }
            if event.key_released(Key::Named(NamedKey::Space)) {
                self.toast_manager
                    .toast("Close Finger edit before resuming practice");
                return;
            }
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
        if event.key_released(Key::Character("r")) {
            self.restart_practice_scope();
            self.toast_manager.toast("Practice restarted");
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

    #[cfg(debug_assertions)]
    fn debug_semantic_action(&mut self, ctx: &mut Context, id: &str) -> bool {
        let Some(action) = DebugPracticeAction::from_id(id) else {
            return false;
        };

        match action {
            DebugPracticeAction::ToggleWait => self.toggle_wait_for_notes(ctx),
            DebugPracticeAction::ToggleCoach => self.toggle_tempo_coach(ctx),
            DebugPracticeAction::CycleHands => self.cycle_practice_hands(ctx),
            DebugPracticeAction::ToggleLoop => top_bar::toggle_loop(self, ctx),
            DebugPracticeAction::ToggleFingerings => {
                if !self.toggle_fingerings(ctx) {
                    return false;
                }
            }
            #[cfg(feature = "score-verovio")]
            DebugPracticeAction::ToggleScore => {
                if !self.toggle_score_visibility(ctx) {
                    return false;
                }
            }
            #[cfg(feature = "score-verovio")]
            DebugPracticeAction::ScoreZoomOut => {
                if !self.adjust_score_zoom(
                    ctx,
                    -(neothesia_core::config::SCORE_ZOOM_STEP_PERCENT as i8),
                ) {
                    return false;
                }
            }
            #[cfg(feature = "score-verovio")]
            DebugPracticeAction::ScoreZoomIn => {
                if !self
                    .adjust_score_zoom(ctx, neothesia_core::config::SCORE_ZOOM_STEP_PERCENT as i8)
                {
                    return false;
                }
            }
            DebugPracticeAction::ToggleFingeringEditor => {
                if !self.toggle_fingering_editor(ctx) {
                    return false;
                }
            }
            DebugPracticeAction::NextFingeringTarget => {
                if !self.move_fingering_target(1) {
                    return false;
                }
            }
            DebugPracticeAction::AssignFingerOne => {
                if !self.set_selected_finger(Some(1)) {
                    return false;
                }
            }
            DebugPracticeAction::ClearFinger => {
                if !self.set_selected_finger(None) {
                    return false;
                }
            }
            DebugPracticeAction::SuggestFinger => {
                if !self.request_fingering_suggestion(ctx) {
                    return false;
                }
            }
            DebugPracticeAction::AcceptFingerSuggestion => {
                if !self.accept_fingering_suggestion() {
                    return false;
                }
            }
            DebugPracticeAction::Restart => self.restart_practice_scope(),
            DebugPracticeAction::Back => {
                ctx.proxy
                    .send_event(NeothesiaEvent::MainMenu(Some(self.player.song().clone())))
                    .ok();
            }
            DebugPracticeAction::ShowOverview if self.completion.is_some() => {
                self.completion_view = CompletionView::Overview;
            }
            DebugPracticeAction::ShowTechnique if self.completion.is_some() => {
                self.completion_view = CompletionView::Technique;
            }
            DebugPracticeAction::ShowHistory if self.completion.is_some() => {
                self.completion_view = CompletionView::History;
            }
            DebugPracticeAction::Retry if self.completion.is_some() => {
                self.retry_practice();
            }
            _ => return false,
        }
        true
    }

    #[cfg(debug_assertions)]
    fn debug_practice_snapshot(&self, ctx: &Context) -> Option<super::DebugPracticeSnapshot> {
        let snapshot = self.player.practice_snapshot();
        #[cfg(feature = "score-verovio")]
        let score_state = (
            self.score_artifact.is_some(),
            self.score_artifact
                .as_ref()
                .is_some_and(|artifact| artifact.synchronization.is_some()),
            self.score_artifact
                .as_ref()
                .and_then(|artifact| artifact.synchronization.as_ref())
                .map_or(0, |synchronization| synchronization.index.len()),
            self.active_score_highlight_count(),
            self.score_texture.map_or(0, |texture| {
                self.active_score_bounds(texture.page_index).len()
            }),
            self.score_pages.cached_pages().count(),
            self.score_pages.focus(),
            self.score_texture.map(|texture| texture.page_index),
            self.score_texture.map(|texture| texture.width as usize),
            self.score_texture.map(|texture| texture.height as usize),
            self.score_visible(ctx),
        );
        #[cfg(not(feature = "score-verovio"))]
        let score_state = (false, false, 0, 0, 0, 0, None, None, None, None, false);
        let loop_range = self
            .top_bar
            .is_looper_active()
            .then(|| self.top_bar.loop_measure_range(&self.player));
        Some(super::DebugPracticeSnapshot {
            wait_for_notes: self.player.wait_for_notes(),
            adaptive_tempo: ctx.config.adaptive_tempo(),
            hands: self.player.practice_hands(),
            loop_active: self.top_bar.is_looper_active(),
            loop_start_measure: loop_range.map(|range| range.0),
            loop_end_measure: loop_range.map(|range| range.1),
            counting_in: self.top_bar.is_counting_in(),
            paused: self.player.is_paused(),
            completion_tab: self
                .completion
                .as_ref()
                .map(|_| self.completion_view.label()),
            matched_notes: snapshot.matched_notes,
            wrong_notes: snapshot.wrong_notes,
            missed_notes: snapshot.missed_notes,
            required_notes: snapshot.required_notes,
            required_note_pitches: self.player.required_note_pitches(),
            input_latency_ms: self.player.input_latency_ms(),
            fingerings_available: self.fingering_state().0,
            fingerings_enabled: self.fingering_state().1,
            fingering_count: self.player.song().fingerings.len(),
            manual_fingering_count: self.player.song().manual_fingering_hints.len(),
            fingering_crossing_count: self
                .note_labels
                .as_ref()
                .map_or(0, NoteLabels::fingering_crossing_count),
            fingering_editor_active: self.fingering_editor_active(),
            suggested_finger: self
                .pending_fingering_suggestion()
                .map(|suggestion| usize::from(suggestion.finger)),
            suggested_fingering_count: self.pending_fingering_suggestion_count(),
            suggestion_confidence_percent: self
                .pending_fingering_suggestion()
                .map(|suggestion| usize::from(suggestion.confidence_percent)),
            score_artifact_ready: score_state.0,
            score_synchronization_ready: score_state.1,
            score_synchronized_notes: score_state.2,
            score_active_highlights: score_state.3,
            score_visible_highlights: score_state.4,
            score_cached_pages: score_state.5,
            score_focused_page: score_state.6,
            score_texture_page: score_state.7,
            score_texture_width: score_state.8,
            score_texture_height: score_state.9,
            score_visible: score_state.10,
            score_zoom_percent: ctx.config.score_zoom_percent(),
        })
    }

    #[cfg(debug_assertions)]
    fn debug_midi_event(&mut self, ctx: &mut Context, channel: u8, message: &MidiMessage) -> bool {
        self.midi_event(ctx, channel, message);
        true
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
    use std::collections::HashSet;

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

    #[test]
    fn expression_copy_stays_descriptive_when_reference_is_incomplete() {
        let (dynamics, contour, pedal, pedal_timing, articulation) =
            format_expression_summary(ExpressionSummary {
                velocity: neothesia_core::practice::VelocitySummary {
                    matched_samples: 4,
                    mean_abs_difference: Some(8),
                    played_min: Some(30),
                    played_max: Some(100),
                    target_min: Some(40),
                    target_max: Some(96),
                    ..Default::default()
                },
                pedal: neothesia_core::practice::PedalSummary {
                    user_changes: 2,
                    user_used: true,
                    ..Default::default()
                },
                articulation: Default::default(),
            });

        assert!(dynamics.contains("descriptive"));
        assert!(dynamics.contains("avg gap 8"));
        assert!(contour.contains("need 6 shaped steps"));
        assert_eq!(
            pedal,
            "Pedal: 2 user changes captured · score has no pedal reference"
        );
        assert_eq!(
            pedal_timing,
            "Pedal timing: needs both score and user sustain"
        );
        assert!(articulation.contains("pedal excluded"));
    }

    #[test]
    fn expression_copy_requires_stable_pedal_and_contour_evidence() {
        let (_, contour, _, pedal_timing, _) = format_expression_summary(ExpressionSummary {
            velocity: neothesia_core::practice::VelocitySummary {
                matched_samples: 7,
                contour_steps: 6,
                contour_aligned: 5,
                contour_flat: 1,
                ..Default::default()
            },
            pedal: neothesia_core::practice::PedalSummary {
                user_used: true,
                target_present: true,
                user_transitions: 4,
                target_transitions: 4,
                timing_samples: 4,
                median_offset_ms: Some(40),
                median_deviation_ms: Some(10),
                ..Default::default()
            },
            articulation: Default::default(),
        });

        assert_eq!(
            contour,
            "Dynamics contour (descriptive): 5/6 followed · 1 flat · 0 opposite"
        );
        assert_eq!(
            pedal_timing,
            "Pedal timing (descriptive): median 40ms late · spread 10ms · 4 pairs"
        );

        let (_, _, _, mismatched, _) = format_expression_summary(ExpressionSummary {
            pedal: neothesia_core::practice::PedalSummary {
                user_used: true,
                target_present: true,
                user_transitions: 5,
                target_transitions: 4,
                timing_samples: 4,
                median_offset_ms: Some(40),
                median_deviation_ms: Some(10),
                ..Default::default()
            },
            ..Default::default()
        });
        assert!(mismatched.contains("transition mismatch"));
    }

    #[test]
    fn timing_profile_copy_explains_signed_bias_and_evidence_threshold() {
        assert_eq!(
            format_timing_profile(
                neothesia_core::practice::TimingSummary {
                    matched_samples: 12,
                    median_offset_ms: Some(-18),
                    median_deviation_ms: Some(9),
                },
                0,
            ),
            "Timing profile: median 18 ms early · typical spread 9 ms · calibration needs 24"
        );
        assert!(format_timing_profile(Default::default(), 0).contains("need 8 matched notes"));
    }

    #[test]
    fn exercise_history_prefers_real_bpm_over_multiplier_only_copy() {
        assert_eq!(format_attempt_tempo(Some(84), 1.2), "84 BPM · 120% speed");
        assert_eq!(format_attempt_tempo(None, 0.8), "80% speed");
        assert_eq!(
            format_tempo_or_speed_trend(Some(24), Some(0.4)),
            "tempo: +24 BPM"
        );
        assert_eq!(
            format_tempo_or_speed_trend(None, Some(0.1)),
            "speed: +10 pts"
        );
    }

    #[test]
    fn repeated_exercise_copy_reports_improvement_without_claiming_a_cause() {
        use neothesia_core::practice::{ExercisePassSummary, PracticeBreakdown, TimingSummary};

        let pass = |pass, matched, missed, spread| ExercisePassSummary {
            pass,
            breakdown: PracticeBreakdown {
                target_notes: matched + missed,
                matched_notes: matched,
                missed_notes: missed,
                ..Default::default()
            },
            timing: TimingSummary {
                matched_samples: matched,
                median_offset_ms: Some(0),
                median_deviation_ms: Some(spread),
            },
        };
        let copy = format_exercise_passes(&[pass(1, 8, 2, 24), pass(2, 10, 0, 12)]).unwrap();

        assert_eq!(
            copy,
            "Pass accuracy: 80% → 100% · improved across passes · timing spread 24→12ms"
        );
        assert!(!copy.contains("fatigue"));
        assert_eq!(format_exercise_passes(&[pass(1, 8, 2, 24)]), None);
    }

    #[test]
    fn hand_timing_copy_keeps_each_hand_evidence_separate() {
        let part = |part, samples, offset, deviation| neothesia_core::practice::PartSummary {
            part,
            breakdown: Default::default(),
            timing: neothesia_core::practice::TimingSummary {
                matched_samples: samples,
                median_offset_ms: Some(offset),
                median_deviation_ms: Some(deviation),
            },
        };
        let copy = format_hand_timing(&[
            part(PracticePart::RightHand, 8, 14, 9),
            part(PracticePart::LeftHand, 6, -20, 11),
        ]);

        assert!(copy.contains("right: 14ms late, spread 9ms"));
        assert!(copy.contains("left: need 2 more"));
    }

    #[test]
    fn chord_copy_requires_complete_exact_onset_evidence() {
        assert!(format_chord_profile(Default::default()).contains("no exact-onset"));
        assert!(
            format_chord_profile(neothesia_core::practice::ChordSummary {
                eligible_chords: 4,
                complete_chords: 3,
                incomplete_chords: 1,
                ..Default::default()
            })
            .contains("need 4 complete")
        );
        assert!(
            format_chord_profile(neothesia_core::practice::ChordSummary {
                eligible_chords: 5,
                complete_chords: 4,
                incomplete_chords: 1,
                median_attack_span_ms: Some(28),
                maximum_attack_span_ms: Some(61),
            })
            .contains("median span 28ms")
        );
    }

    #[test]
    fn completion_tabs_fit_the_minimum_panel_width() {
        let panel_width = 480.0;
        let (x, width, gap) = completion_tab_layout(panel_width);
        let right = x + width * 3.0 + gap * 2.0;

        assert!(x >= 28.0);
        assert_eq!(right, panel_width - 28.0);
    }

    #[test]
    fn completion_tabs_cycle_in_both_keyboard_directions() {
        assert_eq!(CompletionView::Overview.next(), CompletionView::Technique);
        assert_eq!(CompletionView::Technique.next(), CompletionView::History);
        assert_eq!(CompletionView::History.next(), CompletionView::Overview);
        assert_eq!(CompletionView::Overview.previous(), CompletionView::History);
    }

    #[cfg(feature = "score-verovio")]
    #[test]
    fn score_page_layout_stays_inside_the_minimum_window() {
        for zoom in [0, 40, 52, 72, u8::MAX] {
            let (x, y, width, height) = score_page_layout(670.0, 620.0, 1_600, 2_263, zoom);
            assert!(x >= 24.0);
            assert!(y >= 32.0);
            assert!(x + width <= 670.0 - 24.0);
            assert!(y + height <= 620.0 - 160.0);
        }
    }

    #[cfg(feature = "score-verovio")]
    #[test]
    fn score_page_layout_grows_monotonically_and_clamps_invalid_zoom() {
        let layout = |zoom| score_page_layout(1_620.0, 1_138.0, 840, 1_188, zoom);
        let below_minimum = layout(0);
        let minimum = layout(neothesia_core::config::SCORE_ZOOM_MIN_PERCENT);
        let default = layout(62);
        let maximum = layout(neothesia_core::config::SCORE_ZOOM_MAX_PERCENT);
        let above_maximum = layout(u8::MAX);

        assert_eq!(below_minimum, minimum);
        assert!(minimum.3 < default.3);
        assert!(default.3 < maximum.3);
        assert_eq!(maximum, above_maximum);
    }

    #[cfg(feature = "score-verovio")]
    #[test]
    fn score_highlight_layout_scales_and_clips_to_the_visible_page() {
        let bounds = crate::score_renderer_worker::ScoreElementBounds {
            x: 0,
            y: 40,
            width: 12,
            height: 8,
        };
        let (x, y, width, height) =
            score_highlight_layout(100.0, 50.0, 420.0, 594.0, 840, 1188, bounds);

        assert_eq!(x, 100.0, "left padding clips to the page edge");
        assert_eq!(y, 67.0);
        assert_eq!(width, 9.0);
        assert_eq!(height, 10.0);
        assert!(x + width <= 520.0);
        assert!(y + height <= 644.0);
    }

    #[cfg(feature = "score-verovio")]
    #[test]
    fn only_a_missing_focused_page_needs_gpu_upload() {
        assert!(score_page_needs_upload(Some(0), None));
        assert!(!score_page_needs_upload(Some(0), Some(0)));
        assert!(!score_page_needs_upload(None, Some(0)));
        assert!(!score_page_needs_upload(Some(1), Some(1)));
        assert!(score_page_needs_upload(Some(2), Some(1)));
    }

    #[cfg(feature = "score-verovio")]
    #[test]
    fn semantic_playback_focus_follows_pages_and_holds_during_rests() {
        use neothesia_core::{
            musicxml::{ScoreEventId, ScoreEventKind},
            score_alignment::MidiNoteId,
            score_playback::ScoreEventOccurrenceId,
            score_view::{
                RenderedScoreElement, ScoreHighlightCue, ScoreHighlightTimeline, ScoreRenderIndex,
            },
        };

        let source = |ordinal| ScoreEventId {
            part_id: "P1".into(),
            measure_ordinal: ordinal,
            kind: ScoreEventKind::Note,
            ordinal: 0,
        };
        let first = source(0);
        let second = source(8);
        let synchronization = crate::score_renderer_worker::ScoreSynchronization {
            timeline: ScoreHighlightTimeline {
                cues: vec![
                    ScoreHighlightCue {
                        score_id: ScoreEventOccurrenceId {
                            source_id: first.clone(),
                            measure_occurrence_ordinal: 0,
                        },
                        midi_id: MidiNoteId {
                            track_id: 0,
                            note_index: 0,
                        },
                        start: Duration::from_secs(1),
                        end: Duration::from_secs(2),
                    },
                    ScoreHighlightCue {
                        score_id: ScoreEventOccurrenceId {
                            source_id: second.clone(),
                            measure_occurrence_ordinal: 0,
                        },
                        midi_id: MidiNoteId {
                            track_id: 0,
                            note_index: 1,
                        },
                        start: Duration::from_secs(5),
                        end: Duration::from_secs(6),
                    },
                ],
                missing_performance_notes: Vec::new(),
            },
            index: ScoreRenderIndex::new(
                2,
                [
                    RenderedScoreElement {
                        source_id: first,
                        renderer_id: "note-page-1".into(),
                        page_index: 0,
                    },
                    RenderedScoreElement {
                        source_id: second,
                        renderer_id: "note-page-2".into(),
                        page_index: 1,
                    },
                ],
            )
            .unwrap(),
        };

        assert_eq!(score_focus_page(&synchronization, Duration::ZERO), None);
        assert_eq!(
            score_focus_page(&synchronization, Duration::from_secs(1)),
            Some(0)
        );
        assert_eq!(
            score_focus_page(&synchronization, Duration::from_secs(4)),
            Some(0),
            "rests retain the last started score page"
        );
        assert_eq!(
            score_focus_page(&synchronization, Duration::from_secs(5)),
            Some(1)
        );
    }

    #[test]
    fn fingering_editor_uses_stable_score_order_and_bounded_navigation() {
        let song = Song::new(midi_file::MidiFile::new("../test.mid").unwrap());
        let mut editor = FingeringEditor::new(&song, Duration::ZERO).unwrap();
        assert!(editor.targets.windows(2).all(|pair| {
            (
                pair[0].start,
                pair[0].pitch,
                pair[0].track_id,
                pair[0].note_index,
            ) <= (
                pair[1].start,
                pair[1].pitch,
                pair[1].track_id,
                pair[1].note_index,
            )
        }));
        editor.move_relative(-1);
        assert_eq!(editor.selected, 0);
        editor.move_relative(isize::MAX);
        assert_eq!(editor.selected, editor.targets.len() - 1);

        let at_end = FingeringEditor::new(&song, Duration::MAX).unwrap();
        assert_eq!(at_end.selected, at_end.targets.len() - 1);
    }

    #[test]
    fn midi_pitch_labels_use_piano_octave_names() {
        assert_eq!(format_midi_pitch(21), "A0");
        assert_eq!(format_midi_pitch(60), "C4");
        assert_eq!(format_midi_pitch(61), "C♯4");
        assert_eq!(format_midi_pitch(108), "C8");
    }

    #[test]
    fn manual_finger_edit_replaces_or_clears_only_the_selected_note() {
        let target = FingeringTarget {
            track_id: 2,
            note_index: 7,
            start: Duration::from_secs(1),
            pitch: 64,
            channel: 1,
        };
        let other = FingerHint {
            track_id: 1,
            note_index: 3,
            finger: 2,
        };
        let existing = FingerHint {
            track_id: target.track_id,
            note_index: target.note_index,
            finger: 4,
        };

        assert_eq!(
            replace_finger_hint(vec![existing, other], target, Some(5)),
            [
                other,
                FingerHint {
                    finger: 5,
                    ..existing
                }
            ]
        );
        assert_eq!(
            replace_finger_hint(vec![existing, other], target, None),
            [other]
        );
    }

    #[test]
    fn practice_ui_action_ids_are_stable_unique_and_namespaced() {
        let unique: HashSet<_> = practice_ui_ids::ALL.iter().copied().collect();

        assert_eq!(unique.len(), practice_ui_ids::ALL.len());
        assert!(
            practice_ui_ids::ALL
                .iter()
                .all(|id| id.starts_with("practice."))
        );
    }

    #[test]
    fn debug_practice_actions_have_an_explicit_supported_boundary() {
        let supported = [
            (practice_ui_ids::PLAYER_BACK, DebugPracticeAction::Back),
            (
                practice_ui_ids::PLAYER_WAIT,
                DebugPracticeAction::ToggleWait,
            ),
            (
                practice_ui_ids::PLAYER_COACH,
                DebugPracticeAction::ToggleCoach,
            ),
            (
                practice_ui_ids::PLAYER_HANDS,
                DebugPracticeAction::CycleHands,
            ),
            (
                practice_ui_ids::PLAYER_LOOP,
                DebugPracticeAction::ToggleLoop,
            ),
            (
                practice_ui_ids::PLAYER_FINGERINGS,
                DebugPracticeAction::ToggleFingerings,
            ),
            #[cfg(feature = "score-verovio")]
            (
                practice_ui_ids::PLAYER_SCORE,
                DebugPracticeAction::ToggleScore,
            ),
            #[cfg(feature = "score-verovio")]
            (
                practice_ui_ids::PLAYER_SCORE_ZOOM_OUT,
                DebugPracticeAction::ScoreZoomOut,
            ),
            #[cfg(feature = "score-verovio")]
            (
                practice_ui_ids::PLAYER_SCORE_ZOOM_IN,
                DebugPracticeAction::ScoreZoomIn,
            ),
            (
                practice_ui_ids::PLAYER_FINGERING_EDITOR,
                DebugPracticeAction::ToggleFingeringEditor,
            ),
            (
                practice_ui_ids::PLAYER_FINGERING_NEXT,
                DebugPracticeAction::NextFingeringTarget,
            ),
            (
                practice_ui_ids::PLAYER_FINGERING_ASSIGN_1,
                DebugPracticeAction::AssignFingerOne,
            ),
            (
                practice_ui_ids::PLAYER_FINGERING_CLEAR,
                DebugPracticeAction::ClearFinger,
            ),
            (
                practice_ui_ids::PLAYER_FINGERING_SUGGEST,
                DebugPracticeAction::SuggestFinger,
            ),
            (
                practice_ui_ids::PLAYER_FINGERING_ACCEPT,
                DebugPracticeAction::AcceptFingerSuggestion,
            ),
            (
                practice_ui_ids::PLAYER_RESTART,
                DebugPracticeAction::Restart,
            ),
            (
                practice_ui_ids::COMPLETION_OVERVIEW,
                DebugPracticeAction::ShowOverview,
            ),
            (
                practice_ui_ids::COMPLETION_TECHNIQUE,
                DebugPracticeAction::ShowTechnique,
            ),
            (
                practice_ui_ids::COMPLETION_HISTORY,
                DebugPracticeAction::ShowHistory,
            ),
            (
                practice_ui_ids::COMPLETION_RETRY,
                DebugPracticeAction::Retry,
            ),
            (practice_ui_ids::COMPLETION_BACK, DebugPracticeAction::Back),
        ];

        for (id, expected) in supported {
            assert_eq!(DebugPracticeAction::from_id(id), Some(expected));
        }
        assert_eq!(
            DebugPracticeAction::from_id(practice_ui_ids::COMPLETION_CALIBRATE),
            None
        );
        assert_eq!(DebugPracticeAction::from_id("practice.unknown"), None);
    }
}
