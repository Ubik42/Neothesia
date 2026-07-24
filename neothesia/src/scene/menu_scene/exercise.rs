use neothesia_core::exercise::{
    ExerciseDirection, ExerciseHands, ExerciseMinorForm, ExercisePattern, ExercisePlan,
    ExerciseSpec, ExerciseTonality,
};
use piano_layout::KeyboardRange;

use crate::{context::Context, song::Song};

use super::super::playing_scene::practice_ui_ids;
use super::{MenuScene, neo_btn, state};

const CARD_W: f32 = 310.0;
const CARD_H: f32 = 58.0;
const CARD_GAP: f32 = 12.0;
const TEMPOS: &[u16] = &[
    30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 140, 160, 180, 200,
];
const REPETITIONS: &[u8] = &[1, 2, 4, 8];

impl MenuScene {
    pub(super) fn exercise_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;
        let grid_w = CARD_W * 2.0 + CARD_GAP;
        let grid_x = nuon::center_x(win_w, grid_w);
        let recent_specs = ctx.config.recent_exercise_specs();
        let favourite_specs = ctx.config.favourite_exercise_specs();
        let spec = &mut self.state.exercise_spec;

        nuon::label()
            .text("Technique Studio")
            .size(win_w, 52.0)
            .y(28.0)
            .font_size(34.0)
            .text_justify(nuon::TextJustify::Center)
            .build(ui);
        nuon::label()
            .text("Build a focused exercise, then practise it with the full learning player.")
            .size(win_w, 28.0)
            .y(76.0)
            .font_size(16.0)
            .color(nuon::Color::new_u8(205, 205, 220, 1.0))
            .text_justify(nuon::TextJustify::Center)
            .build(ui);

        nuon::translate().x(grid_x).y(122.0).build(ui, |ui| {
            apply_selection(
                spec,
                ExerciseField::Key,
                selector_card(
                    ui,
                    "Key",
                    tonic_name(spec.tonic),
                    practice_ui_ids::EXERCISE_KEY_PREVIOUS,
                    practice_ui_ids::EXERCISE_KEY_NEXT,
                ),
            );
            nuon::translate().x(CARD_W + CARD_GAP).add_to_current(ui);
            apply_selection(
                spec,
                ExerciseField::Tonality,
                selector_card(
                    ui,
                    "Tonality",
                    tonality_name(spec.tonality, spec.minor_form),
                    practice_ui_ids::EXERCISE_TONALITY_PREVIOUS,
                    practice_ui_ids::EXERCISE_TONALITY_NEXT,
                ),
            );
        });

        nuon::translate()
            .x(grid_x)
            .y(122.0 + CARD_H + CARD_GAP)
            .build(ui, |ui| {
                apply_selection(
                    spec,
                    ExerciseField::Pattern,
                    selector_card(
                        ui,
                        "Pattern",
                        pattern_name(spec.pattern),
                        practice_ui_ids::EXERCISE_PATTERN_PREVIOUS,
                        practice_ui_ids::EXERCISE_PATTERN_NEXT,
                    ),
                );
                nuon::translate().x(CARD_W + CARD_GAP).add_to_current(ui);
                apply_selection(
                    spec,
                    ExerciseField::Direction,
                    selector_card(
                        ui,
                        "Direction",
                        direction_name(spec.direction),
                        practice_ui_ids::EXERCISE_DIRECTION_PREVIOUS,
                        practice_ui_ids::EXERCISE_DIRECTION_NEXT,
                    ),
                );
            });

        nuon::translate()
            .x(grid_x)
            .y(122.0 + (CARD_H + CARD_GAP) * 2.0)
            .build(ui, |ui| {
                apply_selection(
                    spec,
                    ExerciseField::Hands,
                    selector_card(
                        ui,
                        "Hands",
                        hands_name(spec.hands),
                        practice_ui_ids::EXERCISE_HANDS_PREVIOUS,
                        practice_ui_ids::EXERCISE_HANDS_NEXT,
                    ),
                );
                nuon::translate().x(CARD_W + CARD_GAP).add_to_current(ui);
                let octaves = format!("{} octave{}", spec.octaves, plural(spec.octaves));
                apply_selection(
                    spec,
                    ExerciseField::Octaves,
                    selector_card(
                        ui,
                        "Range",
                        &octaves,
                        practice_ui_ids::EXERCISE_OCTAVES_PREVIOUS,
                        practice_ui_ids::EXERCISE_OCTAVES_NEXT,
                    ),
                );
            });

        nuon::translate()
            .x(grid_x)
            .y(122.0 + (CARD_H + CARD_GAP) * 3.0)
            .build(ui, |ui| {
                let tempo = format!("{} BPM", spec.tempo_bpm);
                apply_selection(
                    spec,
                    ExerciseField::Tempo,
                    selector_card(
                        ui,
                        "Tempo",
                        &tempo,
                        practice_ui_ids::EXERCISE_TEMPO_PREVIOUS,
                        practice_ui_ids::EXERCISE_TEMPO_NEXT,
                    ),
                );
                nuon::translate().x(CARD_W + CARD_GAP).add_to_current(ui);
                let repetitions = format!("{}×", spec.repetitions);
                apply_selection(
                    spec,
                    ExerciseField::Repetitions,
                    selector_card(
                        ui,
                        "Repeat",
                        &repetitions,
                        practice_ui_ids::EXERCISE_REPETITIONS_PREVIOUS,
                        practice_ui_ids::EXERCISE_REPETITIONS_NEXT,
                    ),
                );
            });

        nuon::translate()
            .x(grid_x)
            .y(122.0 + (CARD_H + CARD_GAP) * 4.0)
            .build(ui, |ui| {
                let recent_name = preset_name(&recent_specs, *spec, "No recent exercises");
                apply_preset_selection(
                    spec,
                    &recent_specs,
                    selector_card(
                        ui,
                        "Recent",
                        &recent_name,
                        practice_ui_ids::EXERCISE_RECENT_PREVIOUS,
                        practice_ui_ids::EXERCISE_RECENT_NEXT,
                    ),
                );
                nuon::translate().x(CARD_W + CARD_GAP).add_to_current(ui);
                let favourite_name = preset_name(&favourite_specs, *spec, "No favourite presets");
                apply_preset_selection(
                    spec,
                    &favourite_specs,
                    selector_card(
                        ui,
                        "Favourites",
                        &favourite_name,
                        practice_ui_ids::EXERCISE_FAVOURITE_PREVIOUS,
                        practice_ui_ids::EXERCISE_FAVOURITE_NEXT,
                    ),
                );
            });

        let preview_y = 122.0 + (CARD_H + CARD_GAP) * 5.0 + 9.0;
        if let Ok(plan) = ExercisePlan::generate(
            self.state.exercise_spec,
            &KeyboardRange::new(ctx.config.piano_range()),
        ) {
            nuon::label()
                .text(plan.display_name())
                .size(win_w, 35.0)
                .y(preview_y)
                .font_size(20.0)
                .text_justify(nuon::TextJustify::Center)
                .build(ui);
        }

        if let Some(message) = &self.state.exercise_message {
            nuon::label()
                .text(message)
                .size(win_w, 28.0)
                .y(preview_y + 32.0)
                .font_size(14.0)
                .color(nuon::Color::new_u8(255, 150, 150, 1.0))
                .text_justify(nuon::TextJustify::Center)
                .build(ui);
        }

        let start_w = 340.0;
        let favourite_w = 210.0;
        let action_gap = 12.0;
        let action_x = nuon::center_x(win_w, start_w + favourite_w + action_gap);
        if neo_btn()
            .id(super::super::playing_scene::practice_ui_ids::EXERCISE_START)
            .size(start_w, 62.0)
            .label("Start Exercise")
            .build_at(ui, action_x, preview_y + 67.0)
        {
            self.start_exercise(ctx);
        }
        let is_favourite = ctx.config.is_favourite_exercise(self.state.exercise_spec);
        let favourite_label = if is_favourite {
            "★ Remove favourite"
        } else {
            "☆ Save favourite"
        };
        if neo_btn()
            .id(practice_ui_ids::EXERCISE_FAVOURITE_TOGGLE)
            .size(favourite_w, 62.0)
            .label(favourite_label)
            .build_at(ui, action_x + start_w + action_gap, preview_y + 67.0)
        {
            ctx.config
                .toggle_favourite_exercise(self.state.exercise_spec);
            ctx.config.save();
        }

        if neo_btn()
            .size(96.0, 54.0)
            .label("Back")
            .build_at(ui, 16.0, win_h - 70.0)
        {
            self.state.go_back();
        }
    }

    pub(super) fn start_exercise(&mut self, ctx: &mut Context) -> bool {
        self.open_exercise(ctx, self.state.exercise_spec)
    }

    pub(super) fn open_exercise(&mut self, ctx: &mut Context, spec: ExerciseSpec) -> bool {
        let range = KeyboardRange::new(ctx.config.piano_range());
        let result = ExercisePlan::generate(spec, &range)
            .map_err(|error| error.to_string())
            .and_then(|plan| Song::from_exercise(&plan));
        match result {
            Ok(mut song) => {
                song.apply_saved_setup(ctx);
                self.state.exercise_message = None;
                self.state.song = Some(song);
                self.state.exercise_spec = spec;
                ctx.config.set_last_exercise_spec(spec);
                ctx.config.remember_exercise_spec(spec);
                state::play(&self.state, ctx);
                true
            }
            Err(error) => {
                self.state.exercise_message = Some(error);
                false
            }
        }
    }

    #[cfg(debug_assertions)]
    pub(super) fn debug_adjust_exercise(&mut self, id: &str) -> bool {
        let (field, delta) = match id {
            practice_ui_ids::EXERCISE_KEY_PREVIOUS => {
                (ExerciseField::Key, SelectionDelta::Previous)
            }
            practice_ui_ids::EXERCISE_KEY_NEXT => (ExerciseField::Key, SelectionDelta::Next),
            practice_ui_ids::EXERCISE_TONALITY_PREVIOUS => {
                (ExerciseField::Tonality, SelectionDelta::Previous)
            }
            practice_ui_ids::EXERCISE_TONALITY_NEXT => {
                (ExerciseField::Tonality, SelectionDelta::Next)
            }
            practice_ui_ids::EXERCISE_PATTERN_PREVIOUS => {
                (ExerciseField::Pattern, SelectionDelta::Previous)
            }
            practice_ui_ids::EXERCISE_PATTERN_NEXT => {
                (ExerciseField::Pattern, SelectionDelta::Next)
            }
            practice_ui_ids::EXERCISE_DIRECTION_PREVIOUS => {
                (ExerciseField::Direction, SelectionDelta::Previous)
            }
            practice_ui_ids::EXERCISE_DIRECTION_NEXT => {
                (ExerciseField::Direction, SelectionDelta::Next)
            }
            practice_ui_ids::EXERCISE_HANDS_PREVIOUS => {
                (ExerciseField::Hands, SelectionDelta::Previous)
            }
            practice_ui_ids::EXERCISE_HANDS_NEXT => (ExerciseField::Hands, SelectionDelta::Next),
            practice_ui_ids::EXERCISE_OCTAVES_PREVIOUS => {
                (ExerciseField::Octaves, SelectionDelta::Previous)
            }
            practice_ui_ids::EXERCISE_OCTAVES_NEXT => {
                (ExerciseField::Octaves, SelectionDelta::Next)
            }
            practice_ui_ids::EXERCISE_TEMPO_PREVIOUS => {
                (ExerciseField::Tempo, SelectionDelta::Previous)
            }
            practice_ui_ids::EXERCISE_TEMPO_NEXT => (ExerciseField::Tempo, SelectionDelta::Next),
            practice_ui_ids::EXERCISE_REPETITIONS_PREVIOUS => {
                (ExerciseField::Repetitions, SelectionDelta::Previous)
            }
            practice_ui_ids::EXERCISE_REPETITIONS_NEXT => {
                (ExerciseField::Repetitions, SelectionDelta::Next)
            }
            _ => return false,
        };
        apply_selection(&mut self.state.exercise_spec, field, delta);
        true
    }

    #[cfg(debug_assertions)]
    pub(super) fn debug_exercise_preset_action(&mut self, ctx: &mut Context, id: &str) -> bool {
        let delta = if id.ends_with(".previous") {
            SelectionDelta::Previous
        } else if id.ends_with(".next") {
            SelectionDelta::Next
        } else {
            SelectionDelta::None
        };
        match id {
            practice_ui_ids::EXERCISE_RECENT_PREVIOUS | practice_ui_ids::EXERCISE_RECENT_NEXT => {
                let specs = ctx.config.recent_exercise_specs();
                apply_preset_selection(&mut self.state.exercise_spec, &specs, delta);
                !specs.is_empty()
            }
            practice_ui_ids::EXERCISE_FAVOURITE_PREVIOUS
            | practice_ui_ids::EXERCISE_FAVOURITE_NEXT => {
                let specs = ctx.config.favourite_exercise_specs();
                apply_preset_selection(&mut self.state.exercise_spec, &specs, delta);
                !specs.is_empty()
            }
            practice_ui_ids::EXERCISE_FAVOURITE_TOGGLE => {
                let changed = ctx
                    .config
                    .toggle_favourite_exercise(self.state.exercise_spec);
                if changed {
                    ctx.config.save();
                }
                changed
            }
            _ => false,
        }
    }
}

#[derive(Clone, Copy)]
enum ExerciseField {
    Key,
    Tonality,
    Pattern,
    Direction,
    Hands,
    Octaves,
    Tempo,
    Repetitions,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SelectionDelta {
    Previous,
    None,
    Next,
}

fn apply_selection(spec: &mut ExerciseSpec, field: ExerciseField, delta: SelectionDelta) {
    match (field, delta) {
        (_, SelectionDelta::None) => {}
        (ExerciseField::Key, SelectionDelta::Previous) => spec.tonic = (spec.tonic + 11) % 12,
        (ExerciseField::Key, SelectionDelta::Next) => spec.tonic = (spec.tonic + 1) % 12,
        (ExerciseField::Tonality, SelectionDelta::Previous) => {
            (spec.tonality, spec.minor_form) = previous_tonality(spec.tonality, spec.minor_form);
        }
        (ExerciseField::Tonality, SelectionDelta::Next) => {
            (spec.tonality, spec.minor_form) = next_tonality(spec.tonality, spec.minor_form);
        }
        (ExerciseField::Pattern, SelectionDelta::Previous) => {
            spec.pattern = previous_pattern(spec.pattern);
            if spec.pattern != ExercisePattern::Scale {
                spec.minor_form = ExerciseMinorForm::Natural;
            }
        }
        (ExerciseField::Pattern, SelectionDelta::Next) => {
            spec.pattern = next_pattern(spec.pattern);
            if spec.pattern != ExercisePattern::Scale {
                spec.minor_form = ExerciseMinorForm::Natural;
            }
        }
        (ExerciseField::Direction, SelectionDelta::Previous) => {
            spec.direction = previous_direction(spec.direction);
        }
        (ExerciseField::Direction, SelectionDelta::Next) => {
            spec.direction = next_direction(spec.direction);
        }
        (ExerciseField::Hands, SelectionDelta::Previous) => {
            spec.hands = previous_hands(spec.hands);
        }
        (ExerciseField::Hands, SelectionDelta::Next) => {
            spec.hands = next_hands(spec.hands);
        }
        (ExerciseField::Octaves, SelectionDelta::Previous) => {
            spec.octaves = if spec.octaves == 1 {
                3
            } else {
                spec.octaves - 1
            };
        }
        (ExerciseField::Octaves, SelectionDelta::Next) => {
            spec.octaves = if spec.octaves == 3 {
                1
            } else {
                spec.octaves + 1
            };
        }
        (ExerciseField::Tempo, SelectionDelta::Previous) => {
            spec.tempo_bpm = previous_tempo(spec.tempo_bpm);
        }
        (ExerciseField::Tempo, SelectionDelta::Next) => {
            spec.tempo_bpm = next_tempo(spec.tempo_bpm);
        }
        (ExerciseField::Repetitions, SelectionDelta::Previous) => {
            spec.repetitions = previous_repetitions(spec.repetitions);
        }
        (ExerciseField::Repetitions, SelectionDelta::Next) => {
            spec.repetitions = next_repetitions(spec.repetitions);
        }
    }
}

fn apply_preset_selection(
    spec: &mut ExerciseSpec,
    presets: &[ExerciseSpec],
    delta: SelectionDelta,
) {
    if presets.is_empty() || delta == SelectionDelta::None {
        return;
    }
    let current = presets.iter().position(|preset| preset == spec);
    let index = match (current, delta) {
        (Some(0), SelectionDelta::Previous) => presets.len() - 1,
        (Some(index), SelectionDelta::Previous) => index - 1,
        (Some(index), SelectionDelta::Next) => (index + 1) % presets.len(),
        (None, _) => 0,
        (_, SelectionDelta::None) => unreachable!(),
    };
    *spec = presets[index];
}

fn preset_name(presets: &[ExerciseSpec], current: ExerciseSpec, empty: &str) -> String {
    if presets.is_empty() {
        return empty.to_owned();
    }
    let selected = presets
        .iter()
        .position(|preset| *preset == current)
        .unwrap_or(0);
    let spec = presets[selected];
    format!(
        "{}/{} · {} {}",
        selected + 1,
        presets.len(),
        tonic_name(spec.tonic),
        pattern_name(spec.pattern)
    )
}

fn selector_card(
    ui: &mut nuon::Ui,
    label: &str,
    value: &str,
    previous_id: &'static str,
    next_id: &'static str,
) -> SelectionDelta {
    let previous = neo_btn()
        .id(previous_id)
        .size(48.0, CARD_H)
        .label("<")
        .build(ui);
    nuon::quad()
        .x(52.0)
        .size(CARD_W - 104.0, CARD_H)
        .color(nuon::Color::new_u8(17, 17, 17, 0.6))
        .border_radius([7.0; 4])
        .build(ui);
    nuon::label()
        .x(52.0)
        .size(CARD_W - 104.0, CARD_H)
        .text(format!("{label}  ·  {value}"))
        .font_size(18.0)
        .text_justify(nuon::TextJustify::Center)
        .build(ui);
    let mut next = false;
    nuon::translate().x(CARD_W - 48.0).build(ui, |ui| {
        next = neo_btn()
            .id(next_id)
            .size(48.0, CARD_H)
            .label(">")
            .build(ui);
    });
    if previous {
        SelectionDelta::Previous
    } else if next {
        SelectionDelta::Next
    } else {
        SelectionDelta::None
    }
}

fn tonic_name(tonic: u8) -> &'static str {
    [
        "C", "C♯", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", "B",
    ][tonic as usize]
}

fn tonality_name(tonality: ExerciseTonality, minor_form: ExerciseMinorForm) -> &'static str {
    match (tonality, minor_form) {
        (ExerciseTonality::Major, _) => "Major",
        (ExerciseTonality::Minor, ExerciseMinorForm::Natural) => "Natural minor",
        (ExerciseTonality::Minor, ExerciseMinorForm::Harmonic) => "Harmonic minor",
        (ExerciseTonality::Minor, ExerciseMinorForm::Melodic) => "Melodic minor",
    }
}

fn next_tonality(
    tonality: ExerciseTonality,
    minor_form: ExerciseMinorForm,
) -> (ExerciseTonality, ExerciseMinorForm) {
    match (tonality, minor_form) {
        (ExerciseTonality::Major, _) => (ExerciseTonality::Minor, ExerciseMinorForm::Natural),
        (ExerciseTonality::Minor, ExerciseMinorForm::Natural) => {
            (ExerciseTonality::Minor, ExerciseMinorForm::Harmonic)
        }
        (ExerciseTonality::Minor, ExerciseMinorForm::Harmonic) => {
            (ExerciseTonality::Minor, ExerciseMinorForm::Melodic)
        }
        (ExerciseTonality::Minor, ExerciseMinorForm::Melodic) => {
            (ExerciseTonality::Major, ExerciseMinorForm::Natural)
        }
    }
}

fn previous_tonality(
    tonality: ExerciseTonality,
    minor_form: ExerciseMinorForm,
) -> (ExerciseTonality, ExerciseMinorForm) {
    match (tonality, minor_form) {
        (ExerciseTonality::Major, _) => (ExerciseTonality::Minor, ExerciseMinorForm::Melodic),
        (ExerciseTonality::Minor, ExerciseMinorForm::Natural) => {
            (ExerciseTonality::Major, ExerciseMinorForm::Natural)
        }
        (ExerciseTonality::Minor, ExerciseMinorForm::Harmonic) => {
            (ExerciseTonality::Minor, ExerciseMinorForm::Natural)
        }
        (ExerciseTonality::Minor, ExerciseMinorForm::Melodic) => {
            (ExerciseTonality::Minor, ExerciseMinorForm::Harmonic)
        }
    }
}

fn pattern_name(pattern: ExercisePattern) -> &'static str {
    match pattern {
        ExercisePattern::Scale => "Scale",
        ExercisePattern::Arpeggio => "Arpeggio",
        ExercisePattern::PrimaryChords => "Primary chords",
    }
}

fn direction_name(direction: ExerciseDirection) -> &'static str {
    match direction {
        ExerciseDirection::Ascending => "Ascending",
        ExerciseDirection::Descending => "Descending",
        ExerciseDirection::UpAndDown => "Up and down",
    }
}

fn hands_name(hands: ExerciseHands) -> &'static str {
    match hands {
        ExerciseHands::Right => "Right",
        ExerciseHands::Left => "Left",
        ExerciseHands::Both => "Both",
    }
}

fn next_pattern(pattern: ExercisePattern) -> ExercisePattern {
    match pattern {
        ExercisePattern::Scale => ExercisePattern::Arpeggio,
        ExercisePattern::Arpeggio => ExercisePattern::PrimaryChords,
        ExercisePattern::PrimaryChords => ExercisePattern::Scale,
    }
}

fn previous_pattern(pattern: ExercisePattern) -> ExercisePattern {
    match pattern {
        ExercisePattern::Scale => ExercisePattern::PrimaryChords,
        ExercisePattern::Arpeggio => ExercisePattern::Scale,
        ExercisePattern::PrimaryChords => ExercisePattern::Arpeggio,
    }
}

fn next_direction(direction: ExerciseDirection) -> ExerciseDirection {
    match direction {
        ExerciseDirection::Ascending => ExerciseDirection::Descending,
        ExerciseDirection::Descending => ExerciseDirection::UpAndDown,
        ExerciseDirection::UpAndDown => ExerciseDirection::Ascending,
    }
}

fn previous_direction(direction: ExerciseDirection) -> ExerciseDirection {
    match direction {
        ExerciseDirection::Ascending => ExerciseDirection::UpAndDown,
        ExerciseDirection::Descending => ExerciseDirection::Ascending,
        ExerciseDirection::UpAndDown => ExerciseDirection::Descending,
    }
}

fn next_hands(hands: ExerciseHands) -> ExerciseHands {
    match hands {
        ExerciseHands::Right => ExerciseHands::Left,
        ExerciseHands::Left => ExerciseHands::Both,
        ExerciseHands::Both => ExerciseHands::Right,
    }
}

fn previous_hands(hands: ExerciseHands) -> ExerciseHands {
    match hands {
        ExerciseHands::Right => ExerciseHands::Both,
        ExerciseHands::Left => ExerciseHands::Right,
        ExerciseHands::Both => ExerciseHands::Left,
    }
}

fn next_tempo(tempo: u16) -> u16 {
    TEMPOS
        .iter()
        .copied()
        .find(|candidate| *candidate > tempo)
        .unwrap_or(TEMPOS[0])
}

fn previous_tempo(tempo: u16) -> u16 {
    TEMPOS
        .iter()
        .copied()
        .rev()
        .find(|candidate| *candidate < tempo)
        .unwrap_or(*TEMPOS.last().unwrap())
}

fn next_repetitions(repetitions: u8) -> u8 {
    REPETITIONS
        .iter()
        .copied()
        .find(|candidate| *candidate > repetitions)
        .unwrap_or(REPETITIONS[0])
}

fn previous_repetitions(repetitions: u8) -> u8 {
    REPETITIONS
        .iter()
        .copied()
        .rev()
        .find(|candidate| *candidate < repetitions)
        .unwrap_or(*REPETITIONS.last().unwrap())
}

fn plural(value: u8) -> &'static str {
    if value == 1 { "" } else { "s" }
}

trait BuildAt {
    fn build_at(self, ui: &mut nuon::Ui, x: f32, y: f32) -> bool;
}

impl BuildAt for super::neo_btn::NeoBtn {
    fn build_at(self, ui: &mut nuon::Ui, x: f32, y: f32) -> bool {
        let mut clicked = false;
        nuon::translate().x(x).y(y).build(ui, |ui| {
            clicked = self.build(ui);
        });
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exercise_choices_cycle_without_invalid_states() {
        let mut spec = ExerciseSpec::default();
        apply_selection(&mut spec, ExerciseField::Key, SelectionDelta::Previous);
        assert_eq!(spec.tonic, 11);
        apply_selection(&mut spec, ExerciseField::Key, SelectionDelta::Next);
        assert_eq!(spec.tonic, 0);
        apply_selection(&mut spec, ExerciseField::Octaves, SelectionDelta::Previous);
        assert_eq!(spec.octaves, 3);

        assert_eq!(
            next_pattern(ExercisePattern::PrimaryChords),
            ExercisePattern::Scale
        );
        assert_eq!(
            next_direction(ExerciseDirection::UpAndDown),
            ExerciseDirection::Ascending
        );
        assert_eq!(next_hands(ExerciseHands::Both), ExerciseHands::Right);
        assert_eq!(next_tempo(60), 70);
        assert_eq!(next_tempo(200), 30);
        assert_eq!(
            previous_pattern(ExercisePattern::Scale),
            ExercisePattern::PrimaryChords
        );
        assert_eq!(
            previous_direction(ExerciseDirection::Ascending),
            ExerciseDirection::UpAndDown
        );
        assert_eq!(previous_hands(ExerciseHands::Right), ExerciseHands::Both);
        assert_eq!(previous_tempo(60), 50);
        assert_eq!(previous_tempo(30), 200);
        assert_eq!(next_repetitions(1), 2);
        assert_eq!(next_repetitions(8), 1);
        assert_eq!(previous_repetitions(1), 8);
        assert_eq!(previous_repetitions(4), 2);
        assert_eq!(
            next_tonality(ExerciseTonality::Minor, ExerciseMinorForm::Harmonic),
            (ExerciseTonality::Minor, ExerciseMinorForm::Melodic)
        );
        assert_eq!(
            previous_tonality(ExerciseTonality::Major, ExerciseMinorForm::Natural),
            (ExerciseTonality::Minor, ExerciseMinorForm::Melodic)
        );

        spec.tonality = ExerciseTonality::Minor;
        spec.minor_form = ExerciseMinorForm::Melodic;
        spec.pattern = ExercisePattern::Scale;
        apply_selection(&mut spec, ExerciseField::Pattern, SelectionDelta::Next);
        assert_eq!(spec.pattern, ExercisePattern::Arpeggio);
        assert_eq!(spec.minor_form, ExerciseMinorForm::Natural);
    }

    #[test]
    fn preset_choices_restore_complete_specs_and_wrap() {
        let first = ExerciseSpec {
            tonic: 2,
            tempo_bpm: 70,
            ..Default::default()
        };
        let second = ExerciseSpec {
            tonic: 9,
            hands: ExerciseHands::Left,
            repetitions: 4,
            tempo_bpm: 90,
            ..Default::default()
        };
        let presets = [first, second];
        let mut selected = ExerciseSpec::default();

        apply_preset_selection(&mut selected, &presets, SelectionDelta::Next);
        assert_eq!(selected, first);
        apply_preset_selection(&mut selected, &presets, SelectionDelta::Previous);
        assert_eq!(selected, second);
        apply_preset_selection(&mut selected, &presets, SelectionDelta::Next);
        assert_eq!(selected, first);
        assert_eq!(preset_name(&presets, selected, "empty"), "1/2 · D Scale");

        apply_preset_selection(&mut selected, &[], SelectionDelta::Next);
        assert_eq!(selected, first);
        assert_eq!(preset_name(&[], selected, "empty"), "empty");
    }
}
