use neothesia_core::practice_history::PracticeTrackSetup;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SessionSettings {
    pub tracks: Vec<PracticeTrackSetup>,
    pub parts: BTreeMap<usize, neothesia_core::practice::PracticePart>,
    pub mode: Option<String>,
    pub count_in: u8,
    pub metronome: bool,
    pub latency: i32,
    pub rounds: usize,
    pub hands: String,
    pub adaptive: bool,
}
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Default, Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Preferences {
    pub path: Option<PathBuf>,
    pub exercise: Option<neothesia_core::exercise::ExerciseSpec>,
    pub exercise_layouts:BTreeMap<String,neothesia_core::exercise::ExerciseHands>,
    pub title: String,
    pub speed: Option<f64>,
    pub wait: Option<bool>,
    pub hands: Option<String>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub passage: Option<Passage>,
    pub volume: Option<f64>,
    pub songs: BTreeMap<String, SessionSettings>,
    pub track_appearances:BTreeMap<String,BTreeMap<usize,crate::track_appearance::TrackAppearance>>,
    pub track_sounds: BTreeMap<String, BTreeMap<usize, crate::track_sound::TrackSound>>,
    pub input_configured: bool,
    pub calibrations: BTreeMap<String, i32>,
    pub passages: BTreeMap<String, Vec<PassagePreset>>,
    pub exercise_presets: Vec<ExercisePreset>,
    pub ladder_presets: BTreeMap<String,Vec<LadderPreset>>,
    pub ladder_runs: BTreeMap<String,LadderRun>,
    pub routines:Vec<crate::routine_commands::PracticeRoutine>,
    pub routine_runs:BTreeMap<String,crate::routine_commands::RoutineRun>,
    pub score_versions: BTreeMap<String, Vec<ScoreVersion>>,
    pub hand_span_defaults: BTreeMap<
        neothesia_core::practice::PracticePart,
        neothesia_core::fingering::HandSpanProfile,
    >,
    pub hand_span_profiles:
        BTreeMap<String, BTreeMap<String, neothesia_core::fingering::HandSpanProfile>>,
    pub fingering_profiles:
        BTreeMap<String, BTreeMap<usize, neothesia_core::fingering::HandSpanProfile>>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreVersion {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub content_id: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassagePreset {
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub precise_range: Option<crate::tick_ranges::TickRange>,
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub score_range: Option<crate::score_ranges::ScoreRange>,
    #[serde(default)]
    pub grid: Option<String>,
    pub id: String,
    pub name: String,
    pub start: usize,
    pub end: usize,
    pub notes: String,
    pub speed: f64,
    pub settings: SessionSettings,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExercisePreset {
    pub id: String,
    pub name: String,
    pub spec: neothesia_core::exercise::ExerciseSpec,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Passage {
    pub start: f64,
    pub end: f64,
}

impl Preferences {
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| format!("练习设置读取失败：{e}"))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("练习设置读取失败：{e}")),
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let save = || -> Result<(), Box<dyn std::error::Error>> {
            std::fs::create_dir_all(path.parent().ok_or("设置目录无效")?)?;
            let temporary = path.with_extension("json.tmp");
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(&serde_json::to_vec_pretty(self)?)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(temporary, path)?;
            Ok(())
        };
        save().map_err(|e| format!("练习设置保存失败：{e}"))
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct LadderPreset {
    pub id:String,
    pub name:String,
    pub start:usize,
    pub end:usize,
    pub grid:String,
    pub scope:String,
    pub exercise_spec:Option<neothesia_core::exercise::ExerciseSpec>,
    pub settings:SessionSettings,
    pub plan:neothesia_core::speed_ladder::SpeedLadderPlan,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct LadderRun {
    pub preset:LadderPreset,
    pub progress:neothesia_core::speed_ladder::SpeedLadderProgress,
    pub active:bool,
}
