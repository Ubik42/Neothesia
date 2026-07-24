use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::exercise::ExerciseSpec;

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Model {
    #[serde(default)]
    pub waterfall: WaterfallConfig,
    #[serde(default)]
    pub playback: PlaybackConfig,
    #[serde(default)]
    pub history: History,
    #[serde(default)]
    pub synth: SynthConfig,
    #[serde(default)]
    pub keyboard_layout: LayoutConfig,
    #[serde(default)]
    pub devices: DevicesConfig,
    #[serde(default)]
    pub appearance: AppearanceConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WaterfallConfigV1 {
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f32,

    #[serde(default = "default_animation_offset")]
    pub animation_offset: f32,

    #[serde(default = "default_note_labels")]
    pub note_labels: bool,
}

#[derive(Serialize, Deserialize)]
pub enum WaterfallConfig {
    V1(WaterfallConfigV1),
}

impl Default for WaterfallConfig {
    fn default() -> Self {
        Self::V1(WaterfallConfigV1 {
            animation_speed: default_animation_speed(),
            animation_offset: default_animation_offset(),
            note_labels: default_note_labels(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlaybackConfigV1 {
    #[serde(default = "default_speed_multiplier")]
    pub speed_multiplier: f32,

    #[serde(default = "default_wait_for_notes")]
    pub wait_for_notes: bool,

    #[serde(default = "default_adaptive_tempo")]
    pub adaptive_tempo: bool,

    #[serde(default = "default_adaptive_tempo_mastery")]
    pub adaptive_tempo_mastery: f32,

    #[serde(default = "default_adaptive_tempo_min")]
    pub adaptive_tempo_min: f32,

    #[serde(default = "default_adaptive_tempo_max")]
    pub adaptive_tempo_max: f32,

    #[serde(default = "default_expression_feedback")]
    pub expression_feedback: bool,

    #[serde(default = "default_input_latency_ms")]
    pub input_latency_ms: i32,
}

#[derive(Serialize, Deserialize)]
pub enum PlaybackConfig {
    V1(PlaybackConfigV1),
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self::V1(PlaybackConfigV1 {
            speed_multiplier: default_speed_multiplier(),
            wait_for_notes: default_wait_for_notes(),
            adaptive_tempo: default_adaptive_tempo(),
            adaptive_tempo_mastery: default_adaptive_tempo_mastery(),
            adaptive_tempo_min: default_adaptive_tempo_min(),
            adaptive_tempo_max: default_adaptive_tempo_max(),
            expression_feedback: default_expression_feedback(),
            input_latency_ms: default_input_latency_ms(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryV1 {
    pub last_opened_song: Option<PathBuf>,
    #[serde(default)]
    pub watched_folders: Vec<PathBuf>,
    #[serde(default)]
    pub last_exercise_spec: ExerciseSpec,
    #[serde(default)]
    pub recent_exercise_specs: Vec<ExerciseSpec>,
    #[serde(default)]
    pub favourite_exercise_specs: Vec<ExerciseSpec>,
}

#[derive(Serialize, Deserialize)]
pub enum History {
    V1(HistoryV1),
}

impl Default for History {
    fn default() -> Self {
        Self::V1(HistoryV1 {
            last_opened_song: None,
            watched_folders: Vec::new(),
            last_exercise_spec: ExerciseSpec::default(),
            recent_exercise_specs: Vec::new(),
            favourite_exercise_specs: Vec::new(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SynthConfigV1 {
    pub soundfont_path: Option<PathBuf>,
    #[serde(default = "default_audio_gain")]
    pub audio_gain: f32,
}

#[derive(Serialize, Deserialize)]
pub enum SynthConfig {
    V1(SynthConfigV1),
}

impl Default for SynthConfig {
    fn default() -> Self {
        Self::V1(SynthConfigV1 {
            soundfont_path: None,
            audio_gain: default_audio_gain(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LayoutConfigV1 {
    #[serde(default = "default_piano_range")]
    pub range: (u8, u8),
}

#[derive(Serialize, Deserialize)]
pub enum LayoutConfig {
    V1(LayoutConfigV1),
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self::V1(LayoutConfigV1 {
            range: default_piano_range(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DevicesConfigV1 {
    #[serde(default = "default_output")]
    pub output: Option<String>,
    pub input: Option<String>,

    #[serde(default = "default_separate_channels")]
    pub separate_channels: bool,
}

#[derive(Serialize, Deserialize)]
pub enum DevicesConfig {
    V1(DevicesConfigV1),
}

impl Default for DevicesConfig {
    fn default() -> Self {
        Self::V1(DevicesConfigV1 {
            output: default_output(),
            input: None,
            separate_channels: default_separate_channels(),
        })
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ColorSchemaV1 {
    pub base: (u8, u8, u8),
    pub dark: (u8, u8, u8),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppearanceConfigV1 {
    #[serde(default = "default_color_schema")]
    pub color_schema: Vec<ColorSchemaV1>,

    #[serde(default)]
    pub background_color: (u8, u8, u8),

    #[serde(default = "default_vertical_guidelines")]
    pub vertical_guidelines: bool,

    #[serde(default = "default_horizontal_guidelines")]
    pub horizontal_guidelines: bool,

    #[serde(default = "default_beat_guidelines")]
    pub beat_guidelines: bool,

    #[serde(default = "default_measure_numbers")]
    pub measure_numbers: bool,

    #[serde(default = "default_glow")]
    pub glow: bool,
}

#[derive(Serialize, Deserialize)]
pub enum AppearanceConfig {
    V1(AppearanceConfigV1),
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self::V1(AppearanceConfigV1 {
            color_schema: default_color_schema(),
            background_color: Default::default(),
            vertical_guidelines: default_vertical_guidelines(),
            horizontal_guidelines: default_horizontal_guidelines(),
            beat_guidelines: default_beat_guidelines(),
            measure_numbers: default_measure_numbers(),
            glow: default_glow(),
        })
    }
}

fn default_piano_range() -> (u8, u8) {
    (21, 108)
}

fn default_speed_multiplier() -> f32 {
    1.0
}

fn default_wait_for_notes() -> bool {
    true
}

fn default_adaptive_tempo() -> bool {
    false
}

fn default_adaptive_tempo_mastery() -> f32 {
    0.9
}

fn default_adaptive_tempo_min() -> f32 {
    0.5
}

fn default_adaptive_tempo_max() -> f32 {
    1.0
}

fn default_expression_feedback() -> bool {
    true
}

fn default_input_latency_ms() -> i32 {
    0
}

fn default_animation_speed() -> f32 {
    400.0
}

fn default_animation_offset() -> f32 {
    0.0
}

fn default_note_labels() -> bool {
    false
}

fn default_audio_gain() -> f32 {
    0.2
}

fn default_vertical_guidelines() -> bool {
    true
}

fn default_horizontal_guidelines() -> bool {
    true
}

fn default_beat_guidelines() -> bool {
    false
}

fn default_measure_numbers() -> bool {
    true
}

fn default_glow() -> bool {
    true
}

fn default_separate_channels() -> bool {
    false
}

fn default_color_schema() -> Vec<ColorSchemaV1> {
    vec![
        ColorSchemaV1 {
            base: (210, 89, 222),
            dark: (125, 69, 134),
        },
        ColorSchemaV1 {
            base: (93, 188, 255),
            dark: (48, 124, 255),
        },
        ColorSchemaV1 {
            base: (255, 126, 51),
            dark: (192, 73, 0),
        },
        ColorSchemaV1 {
            base: (51, 255, 102),
            dark: (0, 168, 2),
        },
        ColorSchemaV1 {
            base: (255, 51, 129),
            dark: (48, 124, 255),
        },
        ColorSchemaV1 {
            base: (210, 89, 222),
            dark: (125, 69, 134),
        },
    ]
}

fn default_output() -> Option<String> {
    Some("Buildin Synth".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn practice_and_timeline_defaults_are_user_focused() {
        let PlaybackConfig::V1(playback) = PlaybackConfig::default();
        assert!(playback.wait_for_notes);
        assert!(!playback.adaptive_tempo);
        assert_eq!(playback.adaptive_tempo_mastery, 0.9);
        assert_eq!(playback.adaptive_tempo_min, 0.5);
        assert_eq!(playback.adaptive_tempo_max, 1.0);
        assert!(playback.expression_feedback);
        assert_eq!(playback.input_latency_ms, 0);

        let DevicesConfig::V1(devices) = DevicesConfig::default();
        assert_eq!(devices.output.as_deref(), Some("Buildin Synth"));

        let AppearanceConfig::V1(appearance) = AppearanceConfig::default();
        assert!(appearance.measure_numbers);
        assert!(!appearance.beat_guidelines);
    }

    #[test]
    fn playback_saved_before_latency_compensation_defaults_to_zero() {
        let playback: PlaybackConfigV1 = ron::from_str(
            r#"(
                speed_multiplier: 1.0,
                wait_for_notes: true,
                adaptive_tempo: false,
                adaptive_tempo_mastery: 0.9,
                adaptive_tempo_min: 0.5,
                adaptive_tempo_max: 1.0,
                expression_feedback: true,
            )"#,
        )
        .unwrap();

        assert_eq!(playback.input_latency_ms, 0);
    }
}
