use std::time::{Duration, Instant};

use crate::{NeothesiaEvent, context::Context, icons};
use neothesia_core::practice::{
    AdaptiveTempoCoach, AdaptiveTempoDecision, AdaptiveTempoRules, AttemptHistory, AttemptSummary,
};

use super::{
    PlayingScene,
    animation::{Animated, Easing},
};

pub struct TopBar {
    pub topbar_expand_animation: Animated<bool, Instant>,
    is_expanded: bool,

    settings_animation: Animated<bool, Instant>,

    settings_active: bool,

    looper_active: bool,
    loop_start: Duration,
    loop_end: Duration,
    count_in: Option<CountIn>,
    attempts: AttemptHistory,
    tempo_coach: AdaptiveTempoCoach,
}

#[derive(Debug, Clone, Copy)]
struct CountIn {
    total: Duration,
    remaining: Duration,
}

impl CountIn {
    fn new(total: Duration) -> Self {
        Self {
            total,
            remaining: total,
        }
    }

    fn update(&mut self, delta: Duration) -> bool {
        self.remaining = self.remaining.saturating_sub(delta);
        self.remaining.is_zero()
    }

    fn number(self) -> u8 {
        if self.total.is_zero() {
            return 1;
        }
        ((self.remaining.as_secs_f32() / self.total.as_secs_f32() * 4.0).ceil() as u8).clamp(1, 4)
    }
}

impl TopBar {
    pub fn new() -> Self {
        Self {
            topbar_expand_animation: Animated::new(false)
                .duration(1000.)
                .easing(Easing::EaseOutExpo)
                .delay(30.0),
            settings_animation: Animated::new(false)
                .duration(1000.)
                .easing(Easing::EaseOutExpo)
                .delay(30.0),

            is_expanded: false,
            settings_active: false,

            looper_active: false,
            loop_start: Duration::ZERO,
            loop_end: Duration::ZERO,
            count_in: None,
            attempts: AttemptHistory::default(),
            tempo_coach: AdaptiveTempoCoach::default(),
        }
    }

    pub fn is_looper_active(&self) -> bool {
        self.looper_active
    }

    pub fn loop_start_timestamp(&self) -> Duration {
        self.loop_start
    }

    pub fn loop_end_timestamp(&self) -> Duration {
        self.loop_end
    }

    pub fn is_counting_in(&self) -> bool {
        self.count_in.is_some()
    }

    pub fn update_count_in(&mut self, delta: Duration) -> bool {
        let Some(count_in) = self.count_in.as_mut() else {
            return false;
        };
        if count_in.update(delta) {
            self.count_in = None;
            return true;
        }
        false
    }

    pub fn start_count_in(&mut self, duration: Duration) {
        self.count_in = Some(CountIn::new(duration));
    }

    pub fn cancel_count_in(&mut self) {
        self.count_in = None;
    }

    pub fn record_attempt(
        &mut self,
        summary: AttemptSummary,
        current_speed: f32,
        rules: AdaptiveTempoRules,
        coach_enabled: bool,
    ) -> Option<AdaptiveTempoDecision> {
        let decision =
            coach_enabled.then(|| self.tempo_coach.evaluate(&summary, current_speed, rules));
        if !coach_enabled {
            self.tempo_coach.reset();
        }
        self.attempts.record(summary);
        decision
    }

    pub fn reset_tempo_coach(&mut self) {
        self.tempo_coach.reset();
    }

    pub fn count_in_duration(&self, player: &super::midi_player::MidiPlayer) -> Duration {
        count_in_duration(&measure_boundaries(player), self.loop_start)
    }

    pub fn loop_measure_range(&self, player: &super::midi_player::MidiPlayer) -> (usize, usize) {
        loop_measure_range(player, self.loop_start, self.loop_end)
    }

    #[profiling::function]
    pub fn update(scene: &mut PlayingScene, ctx: &mut Context) {
        let PlayingScene { top_bar, .. } = scene;

        let window_state = &ctx.window_state;

        let h = 75.0;
        let is_hovered = window_state.cursor_logical_position.y < h * 1.7;

        top_bar.is_expanded = is_hovered;
        top_bar.is_expanded |= top_bar.settings_active;

        top_bar
            .topbar_expand_animation
            .transition(top_bar.is_expanded, ctx.frame_timestamp);
        top_bar
            .settings_animation
            .transition(top_bar.settings_active, ctx.frame_timestamp);

        Self::ui(scene, ctx);
    }

    #[profiling::function]
    pub fn ui(this: &mut PlayingScene, ctx: &mut Context) {
        let mut ui = std::mem::replace(&mut this.nuon, nuon::Ui::new());

        nuon::translate()
            .y(this.top_bar.topbar_expand_animation.animate_bool(
                -75.0 + 5.0,
                0.0,
                ctx.frame_timestamp,
            ))
            .build(&mut ui, |ui| {
                Self::panel(this, ctx, ui);
            });

        if nuon::button()
            .x(ctx.window_state.logical_size.width - 178.0)
            .size(80.0, 30.0)
            .label("PANIC  F12")
            .color([143, 48, 61])
            .hover_color([178, 58, 72])
            .preseed_color([198, 68, 82])
            .border_radius([5.0; 4])
            .build(&mut ui)
        {
            this.emergency_panic();
        }

        if let Some(count_in) = this.top_bar.count_in {
            let win_w = ctx.window_state.logical_size.width;
            let win_h = ctx.window_state.logical_size.height;
            let card_w = 220.0;
            let card_h = 170.0;
            nuon::translate()
                .x(nuon::center_x(win_w, card_w))
                .y(nuon::center_y(win_h, card_h) - 40.0)
                .build(&mut ui, |ui| {
                    nuon::quad()
                        .size(card_w, card_h)
                        .color([25, 22, 32, 235])
                        .border_radius([18.0; 4])
                        .build(ui);
                    nuon::label()
                        .y(20.0)
                        .size(card_w, 34.0)
                        .font_size(18.0)
                        .text("Get ready")
                        .build(ui);
                    nuon::label()
                        .y(52.0)
                        .size(card_w, 96.0)
                        .font_size(72.0)
                        .bold(true)
                        .text(count_in.number().to_string())
                        .build(ui);
                });
        }

        this.nuon = ui;
    }

    fn panel(this: &mut PlayingScene, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;

        nuon::quad()
            .size(win_w, 30.0 + 45.0)
            .color([37, 35, 42])
            .build(ui);

        Self::panel_left(this, ctx, ui);
        Self::panel_center(this, ctx, ui);
        Self::panel_right(this, ctx, ui);

        // ProggressBar
        nuon::translate().y(30.0).build(ui, |ui| {
            Self::proggress_bar(this, ctx, ui);
        });
    }

    fn button() -> nuon::Button {
        nuon::button().size(30.0, 30.0).border_radius([5.0; 4])
    }

    fn panel_left(this: &mut PlayingScene, ctx: &mut Context, ui: &mut nuon::Ui) {
        if Self::button().icon(icons::left_arrow_icon()).build(ui) {
            ctx.proxy
                .send_event(NeothesiaEvent::MainMenu(Some(this.player.song().clone())))
                .ok();
        }

        let wait_for_notes = this.player.wait_for_notes();
        if nuon::button()
            .x(38.0)
            .size(110.0, 30.0)
            .label(if wait_for_notes {
                "Wait: ON"
            } else {
                "Wait: OFF"
            })
            .color(if wait_for_notes {
                [56, 145, 255]
            } else {
                [74, 68, 88]
            })
            .hover_color([87, 155, 255])
            .preseed_color([97, 165, 255])
            .border_radius([5.0; 4])
            .build(ui)
        {
            let enabled = !wait_for_notes;
            this.player.set_wait_for_notes(enabled);
            ctx.config.set_wait_for_notes(enabled);
        }

        let coach_enabled = ctx.config.adaptive_tempo();
        if nuon::button()
            .x(156.0)
            .size(100.0, 30.0)
            .label(if coach_enabled {
                "Coach: ON"
            } else {
                "Coach: OFF"
            })
            .color(if coach_enabled {
                [63, 156, 112]
            } else {
                [74, 68, 88]
            })
            .hover_color([78, 176, 132])
            .preseed_color([88, 186, 142])
            .border_radius([5.0; 4])
            .build(ui)
        {
            let enabled = !coach_enabled;
            ctx.config.set_adaptive_tempo(enabled);
            this.top_bar.reset_tempo_coach();
            this.toast_manager.toast(if enabled {
                "Tempo Coach ON: use loop practice for guided speed changes"
            } else {
                "Tempo Coach OFF: speed stays under manual control"
            });
        }

        let snapshot = this.player.practice_snapshot();
        let status = if this.top_bar.looper_active {
            format!(
                "Take {}   Last {}   Best {}",
                this.top_bar.attempts.current_attempt(),
                attempt_accuracy(this.top_bar.attempts.last()),
                attempt_accuracy(this.top_bar.attempts.best())
            )
        } else {
            format!(
                "Hit {}   Wrong {}   Missed {}   Need {}",
                snapshot.matched_notes,
                snapshot.wrong_notes,
                snapshot.missed_notes,
                snapshot.required_notes
            )
        };
        nuon::label()
            .x(264.0)
            .size(220.0, 30.0)
            .font_size(14.0)
            .text(status)
            .text_justify(nuon::TextJustify::Center)
            .build(ui);
    }

    fn panel_center(_this: &mut PlayingScene, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let pill_w = 45.0 * 2.0;

        nuon::translate()
            .x(win_w / 2.0 - pill_w / 2.0)
            .y(5.0)
            .build(ui, |ui| {
                if nuon::button()
                    .size(45.0, 20.0)
                    .color([67, 67, 67])
                    .hover_color([87, 87, 87])
                    .preseed_color([97, 97, 97])
                    .border_radius([10.0, 0.0, 0.0, 10.0])
                    .icon(icons::minus_icon())
                    .text_justify(nuon::TextJustify::Left)
                    .build(ui)
                {
                    ctx.config
                        .set_speed_multiplier(ctx.config.speed_multiplier() - 0.1);
                    _this.top_bar.reset_tempo_coach();
                }

                nuon::label()
                    .text(format!(
                        "{}%",
                        (ctx.config.speed_multiplier() * 100.0).round()
                    ))
                    .bold(true)
                    .size(45.0 * 2.0, 20.0)
                    .build(ui);

                if nuon::button()
                    .size(45.0, 20.0)
                    .x(45.0)
                    .color([67, 67, 67])
                    .hover_color([87, 87, 87])
                    .preseed_color([97, 97, 97])
                    .border_radius([0.0, 10.0, 10.0, 0.0])
                    .icon(icons::plus_icon())
                    .text_justify(nuon::TextJustify::Right)
                    .build(ui)
                {
                    ctx.config
                        .set_speed_multiplier(ctx.config.speed_multiplier() + 0.1);
                    _this.top_bar.reset_tempo_coach();
                }
            });
    }

    fn panel_right(this: &mut PlayingScene, ctx: &mut Context, ui: &mut nuon::Ui) {
        nuon::translate()
            .x(ctx.window_state.logical_size.width)
            .build(ui, |ui| {
                nuon::translate().x(-30.0).add_to_current(ui);

                if Self::button()
                    .icon(if this.top_bar.settings_active {
                        icons::gear_fill_icon()
                    } else {
                        icons::gear_icon()
                    })
                    .build(ui)
                {
                    this.top_bar.settings_active = !this.top_bar.settings_active;
                }

                nuon::translate().x(-30.0).add_to_current(ui);

                if Self::button().icon(icons::repeat_icon()).build(ui) {
                    if this.top_bar.looper_active {
                        let was_counting_in = this.top_bar.count_in.take().is_some();
                        this.top_bar.looper_active = false;
                        this.top_bar.attempts.clear();
                        this.top_bar.reset_tempo_coach();
                        this.player.reset_practice_attempt();
                        if was_counting_in {
                            this.player.resume();
                        }
                    } else {
                        this.top_bar.looper_active = true;
                        if this.top_bar.loop_start.is_zero() && this.top_bar.loop_end.is_zero() {
                            (this.top_bar.loop_start, this.top_bar.loop_end) =
                                default_loop_range(this);
                        }
                        begin_loop_take(this, true);
                    }
                }

                nuon::translate().x(-30.0).add_to_current(ui);

                if Self::button()
                    .icon(if this.player.is_paused() {
                        icons::play_icon()
                    } else {
                        icons::pause_icon()
                    })
                    .build(ui)
                {
                    this.player.pause_resume();
                }
            });
    }

    fn proggress_bar(this: &mut PlayingScene, ctx: &mut Context, ui: &mut nuon::Ui) {
        let h = 45.0;
        let w = ctx.window_state.logical_size.width;

        let render_looper = Self::proggress_bar_looper(this, ctx, ui, w, h);

        Self::proggress_bar_bg(this, ctx, ui, w, h);

        render_looper(ui);
    }

    fn proggress_bar_bg(
        this: &mut PlayingScene,
        ctx: &mut Context,
        ui: &mut nuon::Ui,
        w: f32,
        h: f32,
    ) {
        let progress_w = w * this.player.percentage();

        match nuon::click_area("ProggressBar").size(w, h).build(ui) {
            nuon::ClickAreaEvent::PressStart => {
                if !this.rewind_controller.is_rewinding() {
                    this.rewind_controller.start_mouse_rewind(&mut this.player);

                    let x = ctx.window_state.cursor_logical_position.x;
                    let w = ctx.window_state.logical_size.width;

                    let p = x / w;
                    this.player.set_percentage_time(p);
                    this.keyboard.reset_notes();
                }
            }
            nuon::ClickAreaEvent::PressEnd { .. } => {
                this.rewind_controller.stop_rewind(&mut this.player);
            }
            nuon::ClickAreaEvent::Idle { .. } => {}
        }

        nuon::quad()
            .size(progress_w, h)
            .color([56, 145, 255])
            .build(ui);

        for m in this.player.song().file.measures.iter() {
            let length = this.player.length().as_secs_f32();
            let start = this.player.leed_in().as_secs_f32() / length;
            let measure = m.as_secs_f32() / length;

            let x = (start + measure) * w;

            let light_measure = nuon::Color::new(1.0, 1.0, 1.0, 0.5);
            let dark_measure = nuon::Color::new(0.4, 0.4, 0.4, 1.0);

            let color = if x < progress_w {
                light_measure
            } else {
                dark_measure
            };

            nuon::quad().x(x).size(1.0, h).color(color).build(ui);
        }
    }

    fn proggress_bar_looper<'a>(
        this: &mut PlayingScene,
        ctx: &mut Context,
        ui: &mut nuon::Ui,
        w: f32,
        h: f32,
    ) -> impl FnOnce(&mut nuon::Ui) + 'a {
        let loop_start_time = this.top_bar.loop_start;
        let loop_start = this.player.time_to_percentage(&loop_start_time) * w;

        let loop_end_time = this.top_bar.loop_end;
        let loop_end = this.player.time_to_percentage(&loop_end_time) * w;

        let loop_h = h + 10.0;

        let looper_active = this.top_bar.looper_active;
        let measure_label = loop_measure_label(this);

        let (loop_start_ev, loop_end_ev) = if looper_active {
            let loop_start_ev = nuon::click_area("LooperStart")
                .x(loop_start)
                .width(5.0)
                .height(loop_h)
                .build(ui);
            let loop_end_ev = nuon::click_area("LooperEnd")
                .x(loop_end)
                .width(5.0)
                .height(loop_h)
                .build(ui);
            (loop_start_ev, loop_end_ev)
        } else {
            (nuon::ClickAreaEvent::null(), nuon::ClickAreaEvent::null())
        };

        if loop_start_ev.is_pressed() {
            let x = ctx.window_state.cursor_logical_position.x;
            let w = ctx.window_state.logical_size.width;
            let p = x / w;

            if p * w < loop_end - 10.0 {
                let candidate = this.player.percentage_to_time(p);
                let snapped = snap_to_measure(this, candidate);
                if snapped < this.top_bar.loop_end {
                    this.top_bar.loop_start = snapped;
                }
            }
        }

        if loop_end_ev.is_pressed() {
            let x = ctx.window_state.cursor_logical_position.x;
            let w = ctx.window_state.logical_size.width;
            let p = x / w;

            if p * w > loop_start + 10.0 {
                let candidate = this.player.percentage_to_time(p);
                let snapped = snap_to_measure(this, candidate);
                if snapped > this.top_bar.loop_start {
                    this.top_bar.loop_end = snapped;
                }
            }
        }

        if looper_active
            && (matches!(loop_start_ev, nuon::ClickAreaEvent::PressEnd { .. })
                || matches!(loop_end_ev, nuon::ClickAreaEvent::PressEnd { .. }))
        {
            begin_loop_take(this, true);
        }

        // render
        move |ui| {
            if !looper_active {
                return;
            }

            let color = [255, 56, 187];
            let white = [255; 3];

            nuon::quad()
                .x(loop_start)
                .width(loop_end - loop_start)
                .height(loop_h)
                .color([255, 56, 187, 90])
                .build(ui);

            if loop_end - loop_start > 100.0 {
                nuon::label()
                    .x(loop_start + 8.0)
                    .y(8.0)
                    .size(loop_end - loop_start - 16.0, 24.0)
                    .font_size(13.0)
                    .bold(true)
                    .color([255, 220, 244])
                    .text(measure_label)
                    .build(ui);
            }

            nuon::quad()
                .x(loop_start)
                .width(5.0)
                .height(loop_h)
                .color(
                    if loop_start_ev.is_hovered() || loop_start_ev.is_pressed() {
                        white
                    } else {
                        color
                    },
                )
                .build(ui);

            nuon::quad()
                .x(loop_end)
                .width(5.0)
                .height(loop_h)
                .color(if loop_end_ev.is_hovered() || loop_end_ev.is_pressed() {
                    white
                } else {
                    color
                })
                .build(ui);
        }
    }
}

fn begin_loop_take(scene: &mut PlayingScene, clear_history: bool) {
    if clear_history {
        scene.top_bar.attempts.clear();
        scene.top_bar.reset_tempo_coach();
    }
    let boundaries = measure_boundaries(&scene.player);
    let count_in = count_in_duration(&boundaries, scene.top_bar.loop_start);

    scene.player.reset_practice_attempt();
    scene.player.set_time(scene.top_bar.loop_start);
    scene.keyboard.reset_notes();
    scene.player.pause();
    scene.top_bar.start_count_in(count_in);
}

pub(super) fn begin_measure_loop(
    scene: &mut PlayingScene,
    start_measure: usize,
    end_measure: usize,
) -> bool {
    let boundaries = measure_boundaries(&scene.player);
    let Some((start, end)) = measure_range_from_boundaries(&boundaries, start_measure, end_measure)
    else {
        return false;
    };
    scene.top_bar.loop_start = start;
    scene.top_bar.loop_end = end;
    scene.top_bar.looper_active = true;
    begin_loop_take(scene, true);
    true
}

fn measure_range_from_boundaries(
    boundaries: &[Duration],
    start_measure: usize,
    end_measure: usize,
) -> Option<(Duration, Duration)> {
    let measure_count = boundaries.len().checked_sub(1)?;
    if measure_count == 0 {
        return None;
    }
    let start_measure = start_measure.clamp(1, measure_count);
    let end_measure = end_measure.clamp(start_measure, measure_count);
    Some((boundaries[start_measure - 1], boundaries[end_measure]))
}

fn default_loop_range(scene: &PlayingScene) -> (Duration, Duration) {
    let boundaries = measure_boundaries(&scene.player);
    if boundaries.len() < 2 {
        return (Duration::ZERO, scene.player.length());
    }

    let current = scene.player.time();
    let start_index = boundaries
        .partition_point(|boundary| *boundary <= current)
        .saturating_sub(1)
        .min(boundaries.len() - 2);
    let end_index = (start_index + 2).min(boundaries.len() - 1);

    (boundaries[start_index], boundaries[end_index])
}

fn measure_boundaries(player: &super::midi_player::MidiPlayer) -> Vec<Duration> {
    let lead_in = *player.leed_in();
    let length = player.length();
    let mut boundaries: Vec<_> = player
        .song()
        .file
        .measures
        .iter()
        .map(|measure| lead_in + *measure)
        .filter(|boundary| *boundary <= length)
        .collect();

    if boundaries.first().is_none_or(|first| *first > lead_in) {
        boundaries.insert(0, lead_in);
    }
    if boundaries.last().is_none_or(|last| *last < length) {
        boundaries.push(length);
    }
    boundaries.dedup();
    boundaries
}

fn snap_to_measure(scene: &PlayingScene, time: Duration) -> Duration {
    snap_duration(&measure_boundaries(&scene.player), time)
}

fn loop_measure_label(scene: &PlayingScene) -> String {
    let (first, last) = scene.top_bar.loop_measure_range(&scene.player);

    if first == last {
        format!("Measure {first}")
    } else {
        format!("Measures {first}-{last}")
    }
}

fn loop_measure_range(
    player: &super::midi_player::MidiPlayer,
    start: Duration,
    end: Duration,
) -> (usize, usize) {
    let measures = &player.song().file.measures;
    let lead_in = *player.leed_in();
    let score_start = start.saturating_sub(lead_in);
    let score_end = end.saturating_sub(lead_in);
    let first = measures
        .partition_point(|measure| *measure <= score_start)
        .max(1);
    let last = measures
        .partition_point(|measure| *measure < score_end)
        .max(first);
    (first, last)
}

fn snap_duration(boundaries: &[Duration], time: Duration) -> Duration {
    boundaries
        .iter()
        .min_by_key(|boundary| boundary.abs_diff(time))
        .copied()
        .unwrap_or(time)
}

fn count_in_duration(boundaries: &[Duration], start: Duration) -> Duration {
    let start_index = boundaries
        .partition_point(|boundary| *boundary <= start)
        .saturating_sub(1);
    boundaries
        .get(start_index + 1)
        .map(|next| next.saturating_sub(start))
        .filter(|duration| !duration.is_zero())
        .unwrap_or(Duration::from_secs(2))
        .clamp(Duration::from_secs(1), Duration::from_secs(6))
}

fn attempt_accuracy(summary: Option<&AttemptSummary>) -> String {
    summary
        .and_then(|summary| summary.overall.accuracy())
        .map(|accuracy| format!("{}%", (accuracy * 100.0).round() as u32))
        .unwrap_or_else(|| "--".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping_chooses_the_nearest_measure_boundary() {
        let boundaries = [
            Duration::from_secs(3),
            Duration::from_secs(5),
            Duration::from_secs(7),
        ];

        assert_eq!(
            snap_duration(&boundaries, Duration::from_millis(5_700)),
            Duration::from_secs(5)
        );
        assert_eq!(
            snap_duration(&boundaries, Duration::from_millis(6_300)),
            Duration::from_secs(7)
        );
    }

    #[test]
    fn count_in_uses_one_measure_and_has_safe_limits() {
        let regular = [Duration::ZERO, Duration::from_secs(2)];
        assert_eq!(
            count_in_duration(&regular, Duration::ZERO),
            Duration::from_secs(2)
        );

        let very_slow = [Duration::ZERO, Duration::from_secs(12)];
        assert_eq!(
            count_in_duration(&very_slow, Duration::ZERO),
            Duration::from_secs(6)
        );
    }

    #[test]
    fn count_in_advances_from_four_to_one() {
        let mut count_in = CountIn::new(Duration::from_secs(4));
        assert_eq!(count_in.number(), 4);
        assert!(!count_in.update(Duration::from_millis(2_100)));
        assert_eq!(count_in.number(), 2);
        assert!(count_in.update(Duration::from_secs(2)));
        assert_eq!(count_in.number(), 1);
    }

    #[test]
    fn recommended_measure_range_maps_to_inclusive_boundaries() {
        let boundaries = [
            Duration::from_secs(2),
            Duration::from_secs(4),
            Duration::from_secs(6),
            Duration::from_secs(8),
        ];

        assert_eq!(
            measure_range_from_boundaries(&boundaries, 2, 3),
            Some((Duration::from_secs(4), Duration::from_secs(8)))
        );
    }
}
