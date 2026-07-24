use std::path::PathBuf;

mod model;

pub use model::ColorSchemaV1;
use model::{
    AppearanceConfig, AppearanceConfigV1, DevicesConfig, DevicesConfigV1, History, HistoryV1,
    LayoutConfig, LayoutConfigV1, Model, PlaybackConfig, PlaybackConfigV1, SynthConfig,
    SynthConfigV1, WaterfallConfig, WaterfallConfigV1,
};

const RECENT_EXERCISE_LIMIT: usize = 8;
const FAVOURITE_EXERCISE_LIMIT: usize = 12;

fn ron_options() -> ron::Options {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES)
}

impl Model {
    fn load() -> Self {
        let config: Option<Self> = if let Some(path) = crate::utils::resources::settings_ron() {
            if let Ok(file) = std::fs::read_to_string(path) {
                match ron_options().from_str(&file) {
                    Ok(config) => Some(config),
                    Err(err) => {
                        log::error!("{err:#?}");
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        config.unwrap_or_default()
    }

    fn from_config(config: Config) -> Self {
        let Config {
            playback,
            waterfall,
            devices,
            history,
            synth,
            keyboard_layout,
            appearance,
        } = config;

        Self {
            waterfall: WaterfallConfig::V1(waterfall),
            playback: PlaybackConfig::V1(playback),
            history: History::V1(history),
            synth: SynthConfig::V1(synth),
            keyboard_layout: LayoutConfig::V1(keyboard_layout),
            devices: DevicesConfig::V1(devices),
            appearance: AppearanceConfig::V1(appearance),
        }
    }

    fn build(self) -> Config {
        Config {
            playback: match self.playback {
                PlaybackConfig::V1(v) => v,
            },
            waterfall: match self.waterfall {
                WaterfallConfig::V1(v) => v,
            },
            appearance: match self.appearance {
                AppearanceConfig::V1(v) => v,
            },
            devices: match self.devices {
                DevicesConfig::V1(v) => v,
            },
            synth: match self.synth {
                SynthConfig::V1(v) => v,
            },
            history: match self.history {
                History::V1(v) => v,
            },
            keyboard_layout: match self.keyboard_layout {
                LayoutConfig::V1(v) => v,
            },
        }
    }
}

#[derive(Clone)]
pub struct Config {
    playback: PlaybackConfigV1,
    waterfall: WaterfallConfigV1,
    appearance: AppearanceConfigV1,
    devices: DevicesConfigV1,
    synth: SynthConfigV1,
    history: HistoryV1,
    keyboard_layout: LayoutConfigV1,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Self {
        Model::load().build()
    }

    pub fn piano_range(&self) -> std::ops::RangeInclusive<u8> {
        self.keyboard_layout.range.0..=self.keyboard_layout.range.1
    }

    pub fn set_piano_range_start(&mut self, start: u8) {
        self.keyboard_layout.range.0 = start;
    }

    pub fn set_piano_range_end(&mut self, start: u8) {
        self.keyboard_layout.range.1 = start.min(127);
    }

    pub fn set_separate_channels(&mut self, separate_channels: bool) {
        self.devices.separate_channels = separate_channels;
    }

    pub fn separate_channels(&self) -> bool {
        self.devices.separate_channels
    }

    pub fn vertical_guidelines(&self) -> bool {
        self.appearance.vertical_guidelines
    }

    pub fn horizontal_guidelines(&self) -> bool {
        self.appearance.horizontal_guidelines
    }

    pub fn set_vertical_guidelines(&mut self, vertical_guidelines: bool) {
        self.appearance.vertical_guidelines = vertical_guidelines;
    }

    pub fn set_horizontal_guidelines(&mut self, horizontal_guidelines: bool) {
        self.appearance.horizontal_guidelines = horizontal_guidelines;
    }

    pub fn beat_guidelines(&self) -> bool {
        self.appearance.beat_guidelines
    }

    pub fn set_beat_guidelines(&mut self, beat_guidelines: bool) {
        self.appearance.beat_guidelines = beat_guidelines;
    }

    pub fn measure_numbers(&self) -> bool {
        self.appearance.measure_numbers
    }

    pub fn set_measure_numbers(&mut self, measure_numbers: bool) {
        self.appearance.measure_numbers = measure_numbers;
    }

    pub fn exercise_fingerings(&self) -> bool {
        self.appearance.exercise_fingerings
    }

    pub fn set_exercise_fingerings(&mut self, show: bool) {
        self.appearance.exercise_fingerings = show;
    }

    pub fn glow(&self) -> bool {
        self.appearance.glow
    }

    pub fn set_glow(&mut self, glow: bool) {
        self.appearance.glow = glow;
    }

    pub fn last_opened_song(&self) -> Option<&PathBuf> {
        self.history.last_opened_song.as_ref()
    }

    pub fn set_last_opened_song(&mut self, last_opened_song: Option<PathBuf>) {
        self.history.last_opened_song = last_opened_song;
    }

    pub fn last_exercise_spec(&self) -> crate::exercise::ExerciseSpec {
        let spec = self.history.last_exercise_spec;
        if spec.validate().is_ok() {
            spec
        } else {
            crate::exercise::ExerciseSpec::default()
        }
    }

    pub fn set_last_exercise_spec(&mut self, spec: crate::exercise::ExerciseSpec) {
        self.history.last_exercise_spec = spec;
    }

    pub fn recent_exercise_specs(&self) -> Vec<crate::exercise::ExerciseSpec> {
        valid_unique_exercise_specs(&self.history.recent_exercise_specs, RECENT_EXERCISE_LIMIT)
    }

    pub fn remember_exercise_spec(&mut self, spec: crate::exercise::ExerciseSpec) -> bool {
        if spec.validate().is_err() {
            return false;
        }
        let before = self.history.recent_exercise_specs.clone();
        self.history
            .recent_exercise_specs
            .retain(|existing| *existing != spec);
        self.history.recent_exercise_specs.insert(0, spec);
        self.history
            .recent_exercise_specs
            .truncate(RECENT_EXERCISE_LIMIT);
        self.history.recent_exercise_specs != before
    }

    pub fn favourite_exercise_specs(&self) -> Vec<crate::exercise::ExerciseSpec> {
        valid_unique_exercise_specs(
            &self.history.favourite_exercise_specs,
            FAVOURITE_EXERCISE_LIMIT,
        )
    }

    pub fn is_favourite_exercise(&self, spec: crate::exercise::ExerciseSpec) -> bool {
        self.favourite_exercise_specs().contains(&spec)
    }

    pub fn toggle_favourite_exercise(&mut self, spec: crate::exercise::ExerciseSpec) -> bool {
        if spec.validate().is_err() {
            return false;
        }
        if let Some(index) = self
            .history
            .favourite_exercise_specs
            .iter()
            .position(|existing| *existing == spec)
        {
            self.history.favourite_exercise_specs.remove(index);
        } else {
            self.history.favourite_exercise_specs.insert(0, spec);
            self.history
                .favourite_exercise_specs
                .truncate(FAVOURITE_EXERCISE_LIMIT);
        }
        true
    }

    pub fn watched_folders(&self) -> &[PathBuf] {
        &self.history.watched_folders
    }

    pub fn add_watched_folder(&mut self, folder: PathBuf) -> bool {
        if self
            .history
            .watched_folders
            .iter()
            .any(|existing| paths_equal(existing, &folder))
        {
            return false;
        }
        self.history.watched_folders.push(folder);
        true
    }

    pub fn remove_watched_folder(&mut self, folder: &std::path::Path) -> bool {
        let old_len = self.history.watched_folders.len();
        self.history
            .watched_folders
            .retain(|existing| !paths_equal(existing, folder));
        self.history.watched_folders.len() != old_len
    }

    pub fn soundfont_path(&self) -> Option<&PathBuf> {
        self.synth.soundfont_path.as_ref()
    }

    pub fn set_soundfont_path(&mut self, soundfont_path: Option<PathBuf>) {
        self.synth.soundfont_path = soundfont_path;
    }

    pub fn output(&self) -> Option<&str> {
        self.devices.output.as_deref()
    }

    pub fn set_output(&mut self, output: Option<String>) {
        self.devices.output = output;
    }

    pub fn input(&self) -> Option<&str> {
        self.devices.input.as_deref()
    }

    pub fn set_input<D: std::fmt::Display>(&mut self, v: Option<D>) {
        self.devices.input = v.map(|v| v.to_string());
    }

    pub fn background_color(&self) -> (u8, u8, u8) {
        self.appearance.background_color
    }

    pub fn set_background_color(&mut self, background_color: (u8, u8, u8)) {
        self.appearance.background_color = background_color;
    }

    pub fn color_schema(&self) -> &[ColorSchemaV1] {
        &self.appearance.color_schema
    }

    pub fn set_color_schema(&mut self, color_schema: Vec<ColorSchemaV1>) {
        self.appearance.color_schema = color_schema;
    }

    pub fn audio_gain(&self) -> f32 {
        self.synth.audio_gain
    }

    pub fn set_audio_gain(&mut self, gain: f32) {
        self.synth.audio_gain = gain.max(0.0);
    }

    pub fn animation_offset(&self) -> f32 {
        self.waterfall.animation_offset
    }

    pub fn set_animation_offset(&mut self, offset: f32) {
        self.waterfall.animation_offset = offset;
    }

    pub fn animation_speed(&self) -> f32 {
        self.waterfall.animation_speed
    }

    pub fn set_animation_speed(&mut self, speed: f32) {
        if speed == 0.0 {
            // 0.0 is invalid speed, let's skip it and negate
            self.waterfall.animation_speed = -self.waterfall.animation_speed;
        } else {
            self.waterfall.animation_speed = speed;
        }
    }

    pub fn set_note_labels(&mut self, show: bool) {
        self.waterfall.note_labels = show;
    }

    pub fn note_labels(&self) -> bool {
        self.waterfall.note_labels
    }

    pub fn speed_multiplier(&self) -> f32 {
        self.playback.speed_multiplier
    }

    pub fn set_speed_multiplier(&mut self, speed_multiplier: f32) {
        self.playback.speed_multiplier = if speed_multiplier.is_finite() {
            speed_multiplier.max(0.0)
        } else {
            1.0
        };
    }

    pub fn wait_for_notes(&self) -> bool {
        self.playback.wait_for_notes
    }

    pub fn set_wait_for_notes(&mut self, wait_for_notes: bool) {
        self.playback.wait_for_notes = wait_for_notes;
    }

    pub fn adaptive_tempo(&self) -> bool {
        self.playback.adaptive_tempo
    }

    pub fn set_adaptive_tempo(&mut self, enabled: bool) {
        self.playback.adaptive_tempo = enabled;
    }

    pub fn expression_feedback(&self) -> bool {
        self.playback.expression_feedback
    }

    pub fn set_expression_feedback(&mut self, enabled: bool) {
        self.playback.expression_feedback = enabled;
    }

    pub fn input_latency_ms(&self) -> i32 {
        self.playback.input_latency_ms
    }

    pub fn set_input_latency_ms(&mut self, milliseconds: i32) {
        self.playback.input_latency_ms = milliseconds.clamp(-250, 250);
    }

    pub fn hand_span_profile(&self) -> crate::fingering::HandSpanProfile {
        self.playback.hand_span_profile
    }

    pub fn set_hand_span_profile(&mut self, profile: crate::fingering::HandSpanProfile) {
        self.playback.hand_span_profile = profile;
        self.playback.right_hand_span_profile = None;
        self.playback.left_hand_span_profile = None;
    }

    pub fn hand_span_profile_for(
        &self,
        hand: crate::fingering::FingeringHand,
    ) -> crate::fingering::HandSpanProfile {
        match hand {
            crate::fingering::FingeringHand::Right => self.playback.right_hand_span_profile,
            crate::fingering::FingeringHand::Left => self.playback.left_hand_span_profile,
        }
        .unwrap_or(self.playback.hand_span_profile)
    }

    pub fn set_hand_span_profile_for(
        &mut self,
        hand: crate::fingering::FingeringHand,
        profile: crate::fingering::HandSpanProfile,
    ) {
        match hand {
            crate::fingering::FingeringHand::Right => {
                self.playback.right_hand_span_profile = Some(profile);
            }
            crate::fingering::FingeringHand::Left => {
                self.playback.left_hand_span_profile = Some(profile);
            }
        }
    }

    pub fn adaptive_tempo_mastery(&self) -> f32 {
        self.playback.adaptive_tempo_mastery
    }

    pub fn set_adaptive_tempo_mastery(&mut self, mastery: f32) {
        self.playback.adaptive_tempo_mastery = (mastery.clamp(0.7, 1.0) * 100.0).round() / 100.0;
    }

    pub fn adaptive_tempo_min(&self) -> f32 {
        self.playback.adaptive_tempo_min
    }

    pub fn set_adaptive_tempo_min(&mut self, min: f32) {
        self.playback.adaptive_tempo_min =
            (min.clamp(0.25, self.playback.adaptive_tempo_max) * 100.0).round() / 100.0;
    }

    pub fn adaptive_tempo_max(&self) -> f32 {
        self.playback.adaptive_tempo_max
    }

    pub fn set_adaptive_tempo_max(&mut self, max: f32) {
        self.playback.adaptive_tempo_max =
            (max.clamp(self.playback.adaptive_tempo_min, 2.0) * 100.0).round() / 100.0;
    }

    pub fn adaptive_tempo_rules(&self) -> crate::practice::AdaptiveTempoRules {
        crate::practice::AdaptiveTempoRules {
            mastery_accuracy: self.adaptive_tempo_mastery(),
            min_speed: self.adaptive_tempo_min(),
            max_speed: self.adaptive_tempo_max(),
            ..crate::practice::AdaptiveTempoRules::default()
        }
    }

    pub fn save(&self) {
        let res = ron_options().to_string_pretty(
            &Model::from_config(self.clone()),
            ron::ser::PrettyConfig::default(),
        );

        if let Ok(s) = res
            && let Some(path) = crate::utils::resources::settings_ron()
        {
            std::fs::create_dir_all(path.parent().unwrap()).ok();
            std::fs::write(path, s).ok();
        }
    }
}

fn paths_equal(left: &std::path::Path, right: &std::path::Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    if cfg!(target_os = "windows") {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

fn valid_unique_exercise_specs(
    specs: &[crate::exercise::ExerciseSpec],
    limit: usize,
) -> Vec<crate::exercise::ExerciseSpec> {
    let mut valid = Vec::new();
    for spec in specs {
        if spec.validate().is_ok() && !valid.contains(spec) {
            valid.push(*spec);
            if valid.len() == limit {
                break;
            }
        }
    }
    valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watched_folders_are_unique_and_removable() {
        let mut config = Model::default().build();
        let folder = PathBuf::from("D:/Music/MIDI");

        assert!(config.add_watched_folder(folder.clone()));
        assert!(!config.add_watched_folder(folder.clone()));
        assert_eq!(config.watched_folders(), std::slice::from_ref(&folder));
        assert!(config.remove_watched_folder(&folder));
        assert!(config.watched_folders().is_empty());
    }

    #[test]
    fn legacy_history_config_defaults_to_no_watched_folders() {
        let history: HistoryV1 = ron::from_str("(last_opened_song:None)").unwrap();
        assert!(history.watched_folders.is_empty());
        assert!(history.recent_exercise_specs.is_empty());
        assert!(history.favourite_exercise_specs.is_empty());
        assert_eq!(
            history.last_exercise_spec,
            crate::exercise::ExerciseSpec::default()
        );
    }

    #[test]
    fn exercise_spec_round_trips_through_config_model() {
        let mut config = Model::default().build();
        let spec = crate::exercise::ExerciseSpec {
            tonic: 9,
            tonality: crate::exercise::ExerciseTonality::Minor,
            minor_form: crate::exercise::ExerciseMinorForm::Harmonic,
            pattern: crate::exercise::ExercisePattern::Scale,
            direction: crate::exercise::ExerciseDirection::Descending,
            hands: crate::exercise::ExerciseHands::Left,
            octaves: 2,
            repetitions: 4,
            tempo_bpm: 80,
        };
        config.set_last_exercise_spec(spec);
        let serialized = ron_options()
            .to_string(&Model::from_config(config))
            .unwrap();
        let rebuilt: Model = ron_options().from_str(&serialized).unwrap();
        let rebuilt = rebuilt.build();

        assert_eq!(rebuilt.last_exercise_spec(), spec);
    }

    #[test]
    fn invalid_persisted_exercise_spec_falls_back_safely() {
        let mut config = Model::default().build();
        config.history.last_exercise_spec.tonic = 99;

        assert_eq!(
            config.last_exercise_spec(),
            crate::exercise::ExerciseSpec::default()
        );
    }

    #[test]
    fn exercise_fingering_visibility_round_trips() {
        let mut config = Model::default().build();
        config.set_exercise_fingerings(false);
        let serialized = ron_options()
            .to_string(&Model::from_config(config))
            .unwrap();
        let rebuilt: Model = ron_options().from_str(&serialized).unwrap();

        assert!(!rebuilt.build().exercise_fingerings());
    }

    #[test]
    fn recent_exercises_are_valid_unique_newest_first_and_bounded() {
        let mut config = Model::default().build();
        for tonic in 0..12 {
            assert!(
                config.remember_exercise_spec(crate::exercise::ExerciseSpec {
                    tonic,
                    ..Default::default()
                })
            );
        }
        assert_eq!(
            config
                .recent_exercise_specs()
                .iter()
                .map(|spec| spec.tonic)
                .collect::<Vec<_>>(),
            vec![11, 10, 9, 8, 7, 6, 5, 4]
        );
        assert!(
            config.remember_exercise_spec(crate::exercise::ExerciseSpec {
                tonic: 8,
                ..Default::default()
            })
        );
        assert_eq!(config.recent_exercise_specs()[0].tonic, 8);

        config.history.recent_exercise_specs.insert(
            0,
            crate::exercise::ExerciseSpec {
                tonic: 99,
                ..Default::default()
            },
        );
        assert!(
            config
                .recent_exercise_specs()
                .iter()
                .all(|spec| spec.validate().is_ok())
        );
    }

    #[test]
    fn favourite_exercises_toggle_and_survive_config_round_trip() {
        let mut config = Model::default().build();
        let favourite = crate::exercise::ExerciseSpec {
            tonic: 6,
            tempo_bpm: 80,
            ..Default::default()
        };

        assert!(config.toggle_favourite_exercise(favourite));
        assert!(config.is_favourite_exercise(favourite));
        assert!(config.toggle_favourite_exercise(favourite));
        assert!(!config.is_favourite_exercise(favourite));
        assert!(config.toggle_favourite_exercise(favourite));

        let serialized = ron_options()
            .to_string(&Model::from_config(config))
            .unwrap();
        let rebuilt: Model = ron_options().from_str(&serialized).unwrap();
        assert_eq!(rebuilt.build().favourite_exercise_specs(), vec![favourite]);
    }

    #[test]
    fn input_latency_is_bounded_to_a_safe_adjustment_range() {
        let mut config = Model::default().build();
        config.set_input_latency_ms(500);
        assert_eq!(config.input_latency_ms(), 250);
        config.set_input_latency_ms(-500);
        assert_eq!(config.input_latency_ms(), -250);
    }

    #[test]
    fn hand_span_profiles_migrate_then_diverge_per_hand() {
        let mut config = Model::default().build();
        config.set_hand_span_profile(crate::fingering::HandSpanProfile::Compact);
        assert_eq!(
            config.hand_span_profile_for(crate::fingering::FingeringHand::Right),
            crate::fingering::HandSpanProfile::Compact
        );
        assert_eq!(
            config.hand_span_profile_for(crate::fingering::FingeringHand::Left),
            crate::fingering::HandSpanProfile::Compact
        );

        config.set_hand_span_profile_for(
            crate::fingering::FingeringHand::Right,
            crate::fingering::HandSpanProfile::Large,
        );
        assert_eq!(
            config.hand_span_profile_for(crate::fingering::FingeringHand::Right),
            crate::fingering::HandSpanProfile::Large
        );
        assert_eq!(
            config.hand_span_profile_for(crate::fingering::FingeringHand::Left),
            crate::fingering::HandSpanProfile::Compact
        );

        let serialized = ron_options()
            .to_string(&Model::from_config(config))
            .unwrap();
        let rebuilt: Model = ron_options().from_str(&serialized).unwrap();
        let rebuilt = rebuilt.build();
        assert_eq!(
            rebuilt.hand_span_profile_for(crate::fingering::FingeringHand::Right),
            crate::fingering::HandSpanProfile::Large
        );
        assert_eq!(
            rebuilt.hand_span_profile_for(crate::fingering::FingeringHand::Left),
            crate::fingering::HandSpanProfile::Compact
        );
    }
}
