mod finger_pair;
mod finger_pair_interactions;
mod performance_archive;
mod replay_clips;
mod history_problem_plan;
mod history_problem_cuts;
mod practice_conditions;
mod history_multi_plan;
mod metadata_history;
mod metadata_templates;
pub mod file_locations;
mod import_storage;
mod library_metadata;
mod audio;
mod track_sound;
mod track_appearance;
mod expression_goal;
mod expression_review;
mod document;
mod fingerings;
mod finger_practice;
mod paper_practice;
mod held_actions;
mod hands;
mod history_commands;
mod weak_practice;
mod ladder_commands;
mod routine_commands;
mod library_index;
mod library_scores;
pub mod library_workspace;
mod notation;
pub mod score_ranges;
mod tick_ranges;
pub mod score_export;
pub mod score_sources;
mod finger_demo;
mod held_demo;
mod song_learning;
mod package_commands;
mod piece_package;
mod preferences;
use neothesia_core::{
    exercise::{ExercisePlan, ExerciseSpec},
    practice::{AdaptiveTempoCoach, AdaptiveTempoRules, AttemptSummary},
    song_config::{PlayerConfig, SongConfig},
};
use preferences::{Passage, Preferences};
use std::collections::HashMap;
pub mod library;
pub mod library_monitor;
pub mod library_repairs;
pub mod score_attachments;
pub mod practice_backup;

use midi_file::{MidiFile, MidiNote};
use neothesia_core::{
    piano_layout::KeyboardRange,
    practice::{PracticeHands, PracticeMatcher, PracticePart, PracticeSnapshot, PracticeTarget},
    practice_history::{
        PracticeHistoryStore, PracticeSession, PracticeSessionKind, SongPracticeSetup,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub pitch: u8,
    pub start: f64,
    pub duration: f64,
    pub velocity: u8,
    pub track: usize,
    pub tick: f64,
    pub end_tick: f64,
    pub finger: Option<u8>,
    pub index: usize,
    pub part: String,
    pub manual_hand: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedSong {
    pub title: String,
    pub content_id: String,
    pub source_path: PathBuf,
    pub duration: f64,
    pub notes: Vec<Note>,
    pub measures: Vec<document::Bar>,
    pub tempo: Vec<document::Tempo>,
    pub ppq: u16,
    pub tracks: Vec<document::Track>,
    pub generated: bool,
    pub has_score: bool,
    pub score_revision: u64,
    pub meter_correction: Option<neothesia_core::library::MeterCorrection>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub status: String,
    pub position: f64,
    pub duration: f64,
    pub practice_ms:u64,
    pub speed: f64,
    pub wait: bool,
    pub hands: String,
    pub required: Vec<u8>,
    pub pressed: Vec<u8>,
    pub score: PracticeSnapshot,
    pub input: Option<String>,
    pub output: String,
    pub error: Option<String>,
    pub saved: bool,
    pub passage: Option<Passage>,
    pub repetitions: usize,
    pub ladder: Option<preferences::LadderRun>,
    pub routine:Option<serde_json::Value>,
    pub volume: f64,
    pub mode: String,
    pub measure: usize,
    pub beat: f64,
    pub tick: f64,
    pub bpm: f64,
    pub count_in: u8,
    pub count_remaining: f64,
    pub metronome: bool,
    pub latency: i32,
    pub adaptive: bool,
    pub summary: Option<AttemptSummary>,
    pub recording: bool,
    pub record_duration: f64,
    pub input_connected: bool,
    pub midi_events: u64,
    pub pedal: u8,
    pub rounds: usize,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            status: "idle".into(),
            position: 0.,
            duration: 0.,
            practice_ms:0,
            speed: 1.,
            wait: true,
            hands: "both".into(),
            required: vec![],
            pressed: vec![],
            score: PracticeSnapshot::default(),
            input: None,
            output: "内置钢琴".into(),
            error: None,
            saved: false,
            passage: None,
            repetitions: 0,
            ladder: None,
            routine:None,
            volume: 0.8,
            mode: "wait".into(),
            measure: 1,
            beat: 1.,
            tick: 0.,
            bpm: 120.,
            count_in: 1,
            count_remaining: 0.,
            metronome: false,
            latency: 0,
            adaptive: false,
            summary: None,
            recording: false,
            record_duration: 0.,
            input_connected: false,
            midi_events: 0,
            pedal: 0,
            rounds: 0,
        }
    }
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Command {
    MetadataTemplates,
    UpdateMetadataTemplate {name:String,rules:Option<metadata_templates::Rules>,expected:u64},
    FileLocations,
    UpdateFileLocation {kind:String,path:Option<PathBuf>,clear:bool},
    TrackAppearance {id:usize,appearance:Option<track_appearance::TrackAppearance>},
    TrackSound { id:usize, sound:Option<track_sound::TrackSound> },
    Mode {
        value: String,
    },
    Track {
        id: usize,
        mode: String,
        part: String,
        visible: bool,
    },
    MeasureLoop {
        start: usize,
        end: usize,
        enabled: bool,
    },
    SeekMeasure {
        measure: usize,
    },
    SeekBeat {
        measure: usize,
        beat: f64,
    },
    StepBeat {
        direction: i8,
    },
    CountIn {
        bars: u8,
    },
    Rounds {
        count: usize,
    },
    Calibration,
    Metronome {
        enabled: bool,
    },
    Latency {
        milliseconds: i32,
    },
    Adaptive {
        enabled: bool,
    },
    Generate {
        spec: ExerciseSpec,
    },
    Feedback,
    Panic,
    Recording {
        active: bool,
    },
    ExportMidi,
    PracticeBackupSnapshot{selection:practice_backup::Selection},
    ExportPracticeBackup{selection:practice_backup::Selection},
    InspectPracticeBackup{backup:practice_backup::Backup},
    ApplyPracticeBackup{#[serde(deserialize_with="practice_backup::deserialize_command_backup")] backup:practice_backup::Backup,selection:practice_backup::Selection,policy:String},
    ImportStagedPracticeBackup{token:String,selection:practice_backup::Selection,policy:String},
    ExportPackage,
    ExportPackageFile,
    ImportStagedPackage { token: String, policy: String },
    InspectPackage {
        bytes: Vec<u8>,
    },
    ImportPackage {
        bytes: Vec<u8>,
        policy: String,
    },
    VstEditor,
    Score,
    ScoreSources { #[serde(default)] content_id: Option<String> },
    PreviewScoreSource {id:String,fingerprint:String},
    LinkScoreSource {midi_path:PathBuf,content_id:String,version_id:String,path:PathBuf,default_bpm:u16},
    AcknowledgeScoreSource {id:String,fingerprint:String},
    UnlinkScoreSource {id:String,path:PathBuf},
    ApplyScoreSource {id:String,fingerprint:String,mode:String},
    ImportScoreOriginal {path:PathBuf,default_bpm:u16},
    ScoreVersions,
    LibraryScores {path:PathBuf,content_id:String},
    CheckLibraryScoreImport {path:PathBuf,content_id:String,bytes:Vec<u8>,kind:String,#[serde(default)]target:Option<String>},
    UpdateLibraryPaper{path:PathBuf,content_id:String,id:String,action:String,page:usize,direction:i32},
    AddLibraryScore{path:PathBuf,content_id:String,name:String,bytes:Vec<u8>,kind:String,#[serde(default)]target:Option<String>,#[serde(default)]page:usize},
    ManageLibraryScore {path:PathBuf,content_id:String,kind:String,id:String,action:String,#[serde(default)] name:Option<String>},
    OpenLibraryScore {path:PathBuf,content_id:String,kind:String,id:String},
    ScoreAttachments{content_id:String},
    LinkPaperSource{content_id:String,book:String,asset:String,path:Option<PathBuf>,baseline:Option<String>},
    PaperSourceUpdates,
    InspectPaperSources{content_id:String,book:String},
    PreviewPaperSource{content_id:String,book:String,asset:String,fingerprint:String},
    ApplyPaperSource{content_id:String,book:String,asset:String,fingerprint:String,baseline:String,action:String},
    PreviewPaperTransfer{content_id:String,source:String,target:String},
    ApplyPaperTransfer{content_id:String,request:score_attachments::PaperTransfer},
    PaperPracticeContext,
    EditPaperInk{content_id:String,book:String,asset_id:String,page:usize,expected:Vec<score_attachments::Annotation>,strokes:Vec<score_attachments::Annotation>},
    SeekPaperMeasure{content_id:String,id:String,measure:usize},
    PreviewPaperPractice{content_id:String,book:String,measure:usize},
    SavePaperPractice{request:paper_practice::Request},
    EditPaperAnnotation{content_id:String,book:String,#[serde(default)] id:Option<String>,draft:Option<score_attachments::AnnotationDraft>,#[serde(default)] expected:Option<score_attachments::Annotation>},
    SetPaperMapping{content_id:String,id:String,mapping:Option<score_attachments::Mapping>},
    AddScoreAttachment{content_id:String,name:String,bytes:Vec<u8>,#[serde(default)] append:Option<String>},
    UpdateScoreAttachment{content_id:String,#[serde(default)] id:Option<String>,action:String,#[serde(default)] name:Option<String>,#[serde(default)] view:Option<score_attachments::View>,#[serde(default)] page:usize,#[serde(default)] direction:i32},
    ActivateScore {
        id: String,
        content_id: String,
    },
    DetachScore {
        content_id: String,
    },
    RemoveScoreVersion {
        id: String,
        content_id: String,
    },
    RenameScoreVersion {
        id: String,
        content_id: String,
        name: String,
    },
    PairScore {
        bytes: Vec<u8>,
        name: String,
        content_id: String,
    },
    ExportAnnotatedScore {
        content_id: String,
        score_revision: u64,
        options: score_export::Options,
        #[serde(default)]
        download: bool,
        #[serde(default)]
        fingerprint: Option<String>,
    },
    ScoreMap {
        #[serde(default)]
        score_revision: Option<u64>,
        midi: Vec<u8>,
        notes: Vec<notation::RenderNote>,
        content_id: String,
    },
    Finger {
        track: usize,
        index: usize,
        finger: Option<u8>,
    },
    StartFingerDemo {request:fingerings::PlanRequest,fingerprint:String,rate:f64},
    HeldFingerDemoContext {content_id:String,track:usize,index:usize,rate:f64},
    StartHeldFingerDemo {content_id:String,track:usize,index:usize,baseline:String,rate:f64},
    FingerDemoState,
    StopFingerDemo {#[serde(default)] id:Option<String>},
    SuggestFingerPlans {request:fingerings::PlanRequest},
    SuggestHeldFingerPlans {request:fingerings::PlanRequest},
    AcceptHeldFingerPlan {request:fingerings::PlanRequest,fingerprint:String,edits:Vec<fingerings::Edit>,actions:Vec<held_actions::ActionInput>},
    ClearHeldFingerActions {content_id:String},
    ClearNoteHeldFingerActions {content_id:String,track:usize,index:usize},
    ReviewHeldActionEdit {request:held_actions::ActionEdit},
    SaveHeldActionEdit {request:held_actions::ActionEdit,review:String},
    FingerPlanContext {request:fingerings::PlanRequest},
    AcceptFingerPlan {request:fingerings::PlanRequest,fingerprint:String,edits:Vec<fingerings::Edit>},
    FingerDemoControl{id:String,action:String,position:Option<f64>},
    StartPairFingerDemo{parts:Vec<finger_pair::PairPart>,proof:String,acknowledged:bool,rate:f64},
    SavePairFingerPractice{parts:Vec<finger_pair::PairPart>,proof:String,acknowledged:bool,ranges:Vec<finger_practice::PracticeRange>},
    FingerPair{parts:Vec<finger_pair::PairPart>,proof:Option<String>,acknowledged:bool},
    ReviewFingerAcceptance {request:fingerings::PlanRequest,fingerprint:String,edits:Vec<fingerings::Edit>},
    SaveFingerPractice {request:fingerings::PlanRequest,fingerprint:String,ranges:Vec<finger_practice::PracticeRange>},
    SuggestFingers {
        #[serde(default)]
        hand: Option<PracticePart>,
        track: usize,
        index: usize,
        #[serde(default)]
        profile: neothesia_core::fingering::HandSpanProfile,
        #[serde(default)]
        range: Option<(usize, usize)>,
    },
    ApplyFingers {
        content_id: String,
        hints: Vec<neothesia_core::library::FingerHint>,
    },
    SuggestHands {
        track: usize,
        range: (usize, usize),
        reference: u8,
    },
    ApplyHands {
        content_id: String,
        hints: Vec<neothesia_core::library::NoteHandHint>,
    },
    NoteHand {
        track: usize,
        index: usize,
        part: Option<PracticePart>,
    },
    UndoHands,
    Meter {
        value: Option<neothesia_core::library::MeterCorrection>,
    },
    UndoFingers,
    EditFingersFor{content_id:String,edits:Vec<fingerings::Edit>},
    UndoFingersFor{content_id:String},
    FingerProfiles,
    HandSpanDefault {
        hand: PracticePart,
        profile: neothesia_core::fingering::HandSpanProfile,
    },
    Passages,
    PreviewScoreRangeTrim {content_id:String,score_revision:u64,first:u32,last:u32,start_trim:u64,end_trim:u64},
    PreviewScoreRange { content_id: String, score_revision: u64, first: u32, last: u32 },
    ApplyScoreRange { content_id: String, score_revision: u64, range: score_ranges::ScoreRange },
    SaveScorePassage { content_id: String, score_revision: u64, id: Option<String>, name: String, notes: String, range: score_ranges::ScoreRange },
    PreviewTickRange {content_id:String,grid:String,start:tick_ranges::BeatPoint,end:tick_ranges::BeatPoint},
    ApplyTickRange {range:tick_ranges::TickRange},
    SaveTickPassage {id:Option<String>,name:String,notes:String,range:tick_ranges::TickRange},
    SavePassage {
        id: Option<String>,
        name: String,
        start: usize,
        end: usize,
        notes: String,
    },
    UpdatePassageDetails { content_id: String, id: String, name: String, notes: String },
    OpenPassage {
        id: String,
    },
    RemovePassage {
        id: String,
    },
    ExercisePresets,
    SaveExercisePreset {
        name: String,
        spec: ExerciseSpec,
    },
    RemoveExercisePreset {
        id: String,
    },
    LibraryWorkspace,
    SaveSongLearning {path:PathBuf,content_id:String,value:Option<song_learning::Draft>,#[serde(default)]expected:Option<song_learning::Learning>},
    InspectSong {
        path: PathBuf,
        #[serde(default)]
        force: bool,
    },
    InspectSongs {
        paths: Vec<PathBuf>,
        #[serde(default)]
        force: bool,
    },
    LibraryGroup {
        id: Option<String>,
        name: String,
        parent: Option<String>,
    },
    RemoveLibraryGroup {
        id: String,
    },
    AssignLibrary {
        paths: Vec<PathBuf>,
        group: Option<String>,
        rating: Option<u8>,
    },
    UnassignLibrary {
        paths: Vec<PathBuf>,
        group: String,
    },
    LibraryMetadataHistory {path:PathBuf,content_id:String},
    PreviewMetadataRestore {path:PathBuf,content_id:String,id:String},
    RestoreLibraryMetadata {path:PathBuf,content_id:String,id:String,fields:Vec<String>,expected:neothesia_core::library::SongMetadata},
    PreviewLibraryMetadata {path:PathBuf,content_id:Option<String>,rules:metadata_templates::Rules},
    ReadLibraryMetadata {path:PathBuf,content_id:Option<String>},
    SaveLibraryMetadata {path:PathBuf,content_id:String,value:neothesia_core::library::SongMetadata,expected:neothesia_core::library::SongMetadata},
    UpdateSongMetadata {
        path: PathBuf,
        value: neothesia_core::library::SongMetadata,
    },
    LibraryRepairs {#[serde(default)] query:String,#[serde(default)] kind:String,#[serde(default)] include_resolved:bool,#[serde(default)] offset:usize,#[serde(default="library_repairs::page_size")] limit:usize},
    RelinkLibrary {items:Vec<library_repairs::RelinkItem>},
    SetLibraryPrimary {content_id:String,path:Option<PathBuf>},
    RestoreLibraryPath {content_id:String,path:PathBuf},
    IgnoreLibraryPath {content_id:String,path:PathBuf},
    LibraryMonitorStatus,
    SetLibraryMonitor { enabled: bool },
    LibraryFolders,
    RefreshLibrary,
    IndexLibrary {
        paths: Vec<PathBuf>,
        force: bool,
    },
    IndexStatus,
    PauseIndex,
    ResumeIndex,
    CancelIndex,
    RemoveLibraryFolder {
        path: PathBuf,
    },
    Metadata,
    SetMetadata {
        value: neothesia_core::library::SongMetadata,
    },
    Load {
        path: PathBuf,
        title: String,
    },
    ImportScore {
        bytes: Vec<u8>,
        name: String,
        default_bpm: u16,
    },
    ImportMidi {
        bytes: Vec<u8>,
        name: String,
    },
    CurrentSong,
    Routines{day:String},
    SaveRoutine{id:Option<String>,day:Option<String>,name:String,notes:String,copy_of:Option<String>},
    SaveRoutineSchedule{id:String,schedule:Option<routine_commands::RoutineSchedule>},
    RemoveRoutine{id:String},
    PassageCatalog{content_id:Option<String>,query:String,offset:usize},
    ComposePassages{routine_id:String,day:Option<String>,content_id:String,entries:Vec<routine_commands::PassageSequenceEntry>,goal:routine_commands::RoutineGoal,proof:Option<String>},
    SaveRoutineItem{routine_id:String,day:Option<String>,id:Option<String>,source:Option<String>,source_id:Option<String>,title:String,notes:String,goal:routine_commands::RoutineGoal},
    MoveRoutineItem{routine_id:String,day:Option<String>,item_id:String,direction:i8},
    RemoveRoutineItem{routine_id:String,day:Option<String>,item_id:String},
    OpenRoutineItem{routine_id:String,day:String,item_id:String,resume:bool},
    StopRoutine,
    SkipRoutineItem{routine_id:String,day:String,item_id:String,reason:String},
    ResetRoutineItem{routine_id:String,day:String,item_id:String},
    LadderPresets,
    SaveLadder {id:Option<String>,name:String,start:usize,end:usize,plan:neothesia_core::speed_ladder::SpeedLadderPlan},
    RemoveLadder {id:String},
    StartLadder {id:String,resume:bool},
    StopLadder,
    WeakPractice,
    AcceptWeakPractice {ids:Vec<String>,routine_id:Option<String>,day:Option<String>,name:String},
    History,
    HistoryQuery {
        #[serde(default)] query: String,
        #[serde(default)] mode: String,
        #[serde(default)] hands: String,
        #[serde(default)] range: String,
        #[serde(default)] days: u32,
        #[serde(default)] offset: usize,
        #[serde(default)] limit: usize,
    },
    AnnotateHistory {content_id:String,id:String,note:String,teacher:String,next:String,expected:Option<neothesia_core::practice_history::PracticeAnnotation>},
    PracticeTime {content_id:String,from:u64,to:u64},
    HistoryPerformance {content_id:String,id:String},
    StartHistoryReplay {content_id:String,id:String,source:String,rate:f64},
    HistoryReplayControl {id:String,action:String,#[serde(default)] position:Option<f64>,#[serde(default)] rate:Option<f64>},
    HistoryReplayLoop {id:String,start:f64,end:f64,enabled:bool},
    HistoryReplayClips {content_id:String,id:String},
    SaveHistoryReplayClip {content_id:String,id:String,clip_id:Option<String>,name:String,start:f64,end:f64,notes:String,expected:String},
    DeleteHistoryReplayClip {content_id:String,id:String,clip_id:String,expected:String},
    HistoryDetail { content_id: String, id: String },
    HistoryOpen { content_id: String, id: String, restore: bool },
    HistoryProblemNotes {content_id:String,id:String,offset:usize},
    PreviewProblemCuts {single:Option<history_problem_plan::ProblemPlanSpec>,multi:Option<history_multi_plan::MultiProblemSpec>,expected:String,edits:Vec<history_problem_plan::ProblemPlanEdit>},
    PreviewMultiProblemPlan {spec:history_multi_plan::MultiProblemSpec},
    SaveMultiProblemPlan {spec:history_multi_plan::MultiProblemSpec,expected:String,name:String,notes:String,goal:routine_commands::RoutineGoal,edits:Vec<history_problem_plan::ProblemPlanEdit>},
    PreviewHistoryProblemPlan {spec:history_problem_plan::ProblemPlanSpec},
    SaveHistoryProblemPlan {spec:history_problem_plan::ProblemPlanSpec,expected:String,name:String,notes:String,goal:routine_commands::RoutineGoal,edits:Vec<history_problem_plan::ProblemPlanEdit>},
    HistoryPracticeRange { content_id:String,id:String,reference:usize,before:u8,after:u8 },
    HistoryScoreReview {content_id:String,id:String},
    SetLibraryCollection {path:PathBuf,content_id:String,favorite:Option<bool>,queued:Option<bool>},
    Collection,
    Favorite {
        enabled: bool,
    },
    Queue {
        enabled: bool,
    },
    MoveQueue {
        direction: isize,
    },
    Volume {
        value: f64,
    },
    OpenRecent {
        content_id: String,
    },
    Loop {
        start: f64,
        end: f64,
        enabled: bool,
    },
    Play,
    Pause,
    Restart,
    Save,
    Seek {
        position: f64,
    },
    Speed {
        value: f64,
    },
    Wait {
        enabled: bool,
    },
    Hands {
        value: String,
    },
    Input {
        name: Option<String>,
    },
    Output {
        name: Option<String>,
    },
    Note {
        pitch: u8,
        active: bool,
        velocity: u8,
    },
}
#[derive(Clone)]
pub struct Engine {
    tx: mpsc::Sender<(Command, mpsc::SyncSender<Result<serde_json::Value, String>>)>,
    state: Arc<Mutex<Snapshot>>,
    indexer: library_index::Indexer,
    data: PathBuf,
}
impl Engine {
    pub fn start(data: PathBuf, soundfont: PathBuf, silent: bool) -> Self {
        let indexer = library_index::Indexer::start(data.clone());
        let engine_data = data.clone();
        let (tx, rx) =
            mpsc::channel::<(Command, mpsc::SyncSender<Result<serde_json::Value, String>>)>();
        let state = Arc::new(Mutex::new(Snapshot::default()));
        let shared = state.clone();
        thread::spawn(move || {
            let mut player = Player::new(data, soundfont, silent);
            player.restore();
            if let Ok(mut snapshot) = shared.lock() {
                *snapshot = player.snapshot();
            }
            let mut last = Instant::now();
            let mut device_check = Instant::now();
            loop {
                match rx.recv_timeout(Duration::from_millis(2)) {
                    Ok((command, reply)) => {
                        if matches!(
                            command,
                            Command::ImportScore { .. } | Command::ImportScoreOriginal { .. }
                                | Command::ImportMidi { .. }
                                | Command::ImportPackage { .. }
                                | Command::ExportPackage | Command::ExportPackageFile
                        ) && !player.recorder.is_recording()
                        {
                            player.panic();
                            player.state.status = "paused".into();
                            if let Ok(mut snapshot) = shared.lock() {
                                *snapshot = player.snapshot();
                            }
                        }
                        let result = player.command(command);
                        if let Ok(mut snapshot) = shared.lock() {
                            *snapshot = player.snapshot();
                        }
                        let _ = reply.send(result);
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        player.panic();
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                let now = Instant::now();
                player.tick(now.duration_since(last));
                last = now;
                while let Ok(bytes) = player.input_rx.try_recv() {
                    player.midi(&bytes);
                }
                if device_check.elapsed() > Duration::from_secs(1) {
                    player.check_input();
                    device_check = now;
                }
                if let Ok(mut snapshot) = shared.lock() {
                    *snapshot = player.snapshot();
                }
            }
        });
        Self {
            tx,
            state,
            indexer,
            data: engine_data,
        }
    }
    pub fn stage_package(&self,bytes:Vec<u8>)->Result<serde_json::Value,String> {
        let token=blake3::hash(&bytes).to_hex().to_string();
        let mut summary=self.command(Command::InspectPackage{bytes:bytes.clone()})?;
        let dir=self.data.join("package-transfers");std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
        std::fs::write(dir.join(format!("{token}.neopiece")),bytes).map_err(|e|e.to_string())?;
        summary["token"]=serde_json::json!(token);Ok(summary)
    }
    pub fn stage_practice_backup(&self,bytes:Vec<u8>)->Result<serde_json::Value,String>{
       let backup=practice_backup::Backup::decode(&bytes)?;let token=blake3::hash(&bytes).to_hex().to_string();
       let mut preview=self.command(Command::InspectPracticeBackup{backup})?;
       let dir=self.data.join("practice-transfers");std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;std::fs::write(dir.join(format!("{token}.neopractice")),bytes).map_err(|e|e.to_string())?;
       preview["token"]=serde_json::json!(token);Ok(preview)
    }
    pub fn command(&self, command: Command) -> Result<serde_json::Value, String> {
        if let Command::SuggestHeldFingerPlans{request}=command {let prepared=self.command(Command::FingerPlanContext{request})?;return fingerings::held_plans(serde_json::from_value(prepared).map_err(|e|e.to_string())?);}
        if let Command::PreviewPaperTransfer{content_id,source,target}=command{return score_attachments::preview_transfer(&self.data,&content_id,&source,&target);}
        if let Command::ApplyPaperTransfer{content_id,request}=command{return score_attachments::apply_transfer(&self.data,&content_id,request);}
        if let Command::LinkPaperSource{content_id,book,asset,path,baseline}=command{return score_attachments::link_source(&self.data,&content_id,&book,&asset,path,baseline.as_deref());}
        if let Command::PaperSourceUpdates=command{return score_attachments::source_monitor::snapshot(&self.data);}
        if let Command::InspectPaperSources{content_id,book}=command{return score_attachments::inspect_sources(&self.data,&content_id,&book);}
        if let Command::PreviewPaperSource{content_id,book,asset,fingerprint}=command{return score_attachments::preview_source(&self.data,&content_id,&book,&asset,&fingerprint);}
        if let Command::ApplyPaperSource{content_id,book,asset,fingerprint,baseline,action}=command{return score_attachments::apply_source(&self.data,&content_id,&book,&asset,&fingerprint,&baseline,&action);}
        if matches!(command,Command::MetadataTemplates){return metadata_templates::inspect(&self.data);}
        if let Command::UpdateMetadataTemplate{name,rules,expected}=command{return metadata_templates::update(&self.data,name,rules,expected);}
        if matches!(command,Command::FileLocations){return file_locations::inspect(&self.data);}
        if let Command::UpdateFileLocation{kind,path,clear}=command{return file_locations::update(&self.data,&kind,path,clear);}
        if let Command::ScoreSources{content_id}=command{return score_sources::inspect(&self.data,content_id.as_deref());}
        if let Command::PreviewScoreSource{id,fingerprint}=command{return score_sources::preview(&self.data,&id,&fingerprint);}
        if let Command::ExportPracticeBackup{selection}=command {let snapshot=self.command(Command::PracticeBackupSnapshot{selection})?;let backup:practice_backup::Backup=serde_json::from_value(snapshot).map_err(|e|e.to_string())?;let bytes=backup.encode()?;let dir=self.data.join("practice-transfers");std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;let path=dir.join(format!("{}.neopractice",blake3::hash(&bytes)));std::fs::write(&path,bytes).map_err(|e|e.to_string())?;return Ok(serde_json::json!({"path":path,"summary":backup.summary()}));}
        if let Command::ImportStagedPracticeBackup{token,selection,policy}=command {if token.len()!=64||!token.bytes().all(|c|c.is_ascii_hexdigit()){return Err("备份预览已失效，请重新选择文件".into());}let bytes=std::fs::read(self.data.join("practice-transfers").join(format!("{token}.neopractice"))).map_err(|e|e.to_string())?;if blake3::hash(&bytes).to_hex().as_str()!=token{return Err("备份缓存校验失败".into());}let backup=practice_backup::Backup::decode(&bytes)?;return self.command(Command::ApplyPracticeBackup{backup,selection,policy});}
        if let Command::InspectPracticeBackup{backup}|Command::ApplyPracticeBackup{backup,..}=&command{backup.validate()?;}
        if let Command::SuggestFingerPlans{request}=command {let prepared=self.command(Command::FingerPlanContext{request})?;return fingerings::plans(serde_json::from_value(prepared).map_err(|e|e.to_string())?);}
        if let Command::UpdateLibraryPaper{path,content_id,id,action,page,direction}=command {library_scores::location(&self.data,&path,&content_id)?;if !["movePage","removePage"].contains(&action.as_str()){return Err("谱页整理操作无效".into());}return score_attachments::update(&self.data,&content_id,Some(&id),&action,None,None,page,direction);}
        if let Command::AddLibraryScore{path,content_id,name,bytes,kind,target,page}=&command {
           library_scores::location(&self.data,path,content_id)?;
           if kind=="notation" {if bytes.len()>4_000_000{return Err("演奏乐谱超过 4 MB".into());}let score=neothesia_core::musicxml::import_musicxml_document(bytes).map_err(|e|e.to_string())?;neothesia_core::score_performance::normalized(&score)?;}
           else if kind=="paper"||kind=="append" {return score_attachments::add(&self.data,content_id,name,bytes.clone(),if kind=="append"{Some(target.as_deref().ok_or("未指定图片谱面")?)}else{None});}
           else if kind=="repairPaper"{return score_attachments::repair(&self.data,content_id,target.as_deref().ok_or("未指定待修复谱面")?,*page,bytes.clone());}
           else{return Err("谱面导入类型无效".into());}
        }
        if let Command::CheckLibraryScoreImport{path,content_id,bytes,kind,target}=&command {return library_scores::check_import(&self.data,path,content_id,bytes,kind,target.as_deref());}
        if let Command::LibraryScores{path,content_id}=&command {return library_scores::inspect(&self.data,path,content_id);}
        let attachment_id=match &command{Command::UpdateScoreAttachment{action,..} if action=="view"=>None,Command::ScoreAttachments{content_id}|Command::AddScoreAttachment{content_id,..}|Command::UpdateScoreAttachment{content_id,..}=>Some(content_id),_=>None};
        if let Some(id)=attachment_id{let current=self.command(Command::CurrentSong)?;if current["contentId"].as_str()!=Some(id){return Err("曲目已切换，请重新打开谱面管理".into());}}
        match command{
          Command::ScoreAttachments{content_id}=>return score_attachments::list(&self.data,&content_id),
          Command::AddScoreAttachment{content_id,name,bytes,append}=>return score_attachments::add(&self.data,&content_id,&name,bytes,append.as_deref()),
          Command::UpdateScoreAttachment{content_id,id,action,name,view,page,direction}=>return score_attachments::update(&self.data,&content_id,id.as_deref(),&action,name.as_deref(),view,page,direction),
          _=>{}
        }
        match &command {
          Command::LibraryRepairs{query,kind,include_resolved,offset,limit}=>{let known=self.command(Command::Collection)?;return library_repairs::query(&self.data,&known,query,kind,*include_resolved,*offset,*limit);},
          Command::RelinkLibrary{..}|Command::SetLibraryPrimary{..}|Command::RestoreLibraryPath{..}|Command::IgnoreLibraryPath{..}=>{}
          _=>{}
        }
        match command {
          Command::RelinkLibrary{items}=>return library_repairs::relink(&self.data,items),
          Command::SetLibraryPrimary{content_id,path}=>return library_repairs::set_primary(&self.data,&content_id,path),
          Command::IgnoreLibraryPath{content_id,path}=>return library_repairs::ignore_path(&self.data,&content_id,&path),
          Command::RestoreLibraryPath{content_id,path}=>return library_repairs::restore_path(&self.data,&content_id,&path),
          _=>{}
        }
        if let Command::ImportStagedPackage { token, policy } = command {
            if token.len()!=64 || !token.bytes().all(|c|c.is_ascii_hexdigit()) {return Err("曲目包预览已失效，请重新选择文件".into());}
            let bytes=std::fs::read(self.data.join("package-transfers").join(format!("{token}.neopiece"))).map_err(|e|e.to_string())?;
            if blake3::hash(&bytes).to_hex().as_str()!=token {return Err("曲目包缓存校验失败，请重新选择".into());}
            return self.command(Command::ImportPackage{bytes,policy});
        }
        if let Command::InspectPackage { bytes } = command {
            let package = piece_package::Package::decode(&bytes)?;
            let mut result = package.summary();
            let index = library_workspace::Workspace::load(&self.data)?;
            let prefs = Preferences::load(&self.data.join("web-preferences.json"))?;
            result["existing"] = serde_json::json!(
                index
                    .entries
                    .values()
                    .any(|e| e.content_id.as_ref() == Some(&package.manifest.content_id))
                    || prefs.songs.contains_key(&package.manifest.content_id)
            );
            return Ok(result);
        }
        if matches!(
            command,
            Command::IndexLibrary { .. }
                | Command::IndexStatus
                | Command::PauseIndex
                | Command::ResumeIndex
                | Command::CancelIndex
        ) {
            return self.indexer.command(command);
        }
        if library_workspace::accepts(&command) {
            return library_workspace::execute(command, &self.data);
        }
        let timeout = if matches!(
            command,
            Command::ImportScore { .. }
                | Command::ImportMidi { .. }
                | Command::ImportPackage { .. }
                | Command::ExportPackage | Command::ExportPackageFile | Command::PracticeBackupSnapshot{..} | Command::InspectPracticeBackup{..} | Command::ApplyPracticeBackup{..}
                | Command::ManageLibraryScore{..} | Command::OpenLibraryScore{..}
        ) {
            Duration::from_secs(120)
        } else {
            Duration::from_secs(15)
        };
        let (reply, rx) = mpsc::sync_channel(1);
        self.tx
            .send((command, reply))
            .map_err(|_| "音乐引擎已关闭".to_string())?;
        rx.recv_timeout(timeout)
            .map_err(|_| "音乐引擎响应超时".to_string())?
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

pub fn is_timed_mode(mode: &str) -> bool {
    matches!(mode, "flow" | "recital" | "memory")
}
fn is_recital_mode(mode: &str) -> bool {
    matches!(mode, "recital" | "memory")
}

pub fn devices() -> serde_json::Value {
    let inputs = midir::MidiInput::new("Neothesia devices")
        .map(|m| {
            m.ports()
                .iter()
                .filter_map(|p| m.port_name(p).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let outputs = midir::MidiOutput::new("Neothesia devices")
        .map(|m| {
            m.ports()
                .iter()
                .filter_map(|p| m.port_name(p).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    serde_json::json!({ "inputs": inputs, "outputs": outputs,"vst3":neothesia_core::vst3_backend::discover_standard_plugins() })
}

#[derive(Clone)]
struct PracticeNote {
    inner: MidiNote,
    index: usize,
}
impl std::ops::Deref for PracticeNote {
    type Target = MidiNote;
    fn deref(&self) -> &MidiNote {
        &self.inner
    }
}
struct Player {
    state: Snapshot,
    file: Option<MidiFile>,
    notes: Vec<PracticeNote>,
    note_hands: HashMap<(usize, usize), PracticePart>,
    hand_undo: Vec<Vec<neothesia_core::library::NoteHandHint>>,
    event_notes: HashMap<(usize, u8, u8, Duration, bool), usize>,
    original_grid: Option<midi_file::musical_time::MusicalTime>,
    meter_correction: Option<neothesia_core::library::MeterCorrection>,
    note_cursor: usize,
    events: Vec<midi_file::MidiEvent>,
    event_cursor: usize,
    matcher: PracticeMatcher,
    session_clock: Duration,
    history: PracticeHistoryStore,
    title: String,
    input_connection: Option<midir::MidiInputConnection<()>>,
    input_tx: mpsc::SyncSender<Vec<u8>>,
    input_rx: mpsc::Receiver<Vec<u8>>,
    input_overflow: Arc<AtomicBool>,
    output_connection: Option<midir::MidiOutputConnection>,
    audio: Option<audio::Audio>,
    soundfont: PathBuf,
    silent: bool,
    preferences: Preferences,
    preferences_path: PathBuf,
    restoring: bool,
    track_routing: track_sound::Routing,
    config: SongConfig,
    fingers: HashMap<(usize, Duration, u8), u8>,
    exercise: Option<ExerciseSpec>,
    previous_summary: Option<AttemptSummary>,
    coach: AdaptiveTempoCoach,
    ladder: Option<preferences::LadderRun>,
    routine:Option<routine_commands::ActiveRoutine>,
    last_click: Option<(usize, usize)>,
    finger_demo: Option<finger_demo::Demo>,
    recorder: neothesia_core::recorder::FreeplayRecorder,
    performance_capture:performance_archive::Capture,
    data: PathBuf,
    notation: Option<notation::ScoreAsset>,
    score_revision: u64,
    vst: Option<neothesia_core::vst3_backend::Vst3OutputConnection>,
    end_hold: f64,
    previous_mode: Option<String>,
    finger_undo: Vec<(Vec<neothesia_core::library::FingerHint>,Vec<neothesia_core::library::FingerAction>)>,
}
impl Player {
    fn new(data: PathBuf, soundfont: PathBuf, silent: bool) -> Self {
        let (input_tx, input_rx) = mpsc::sync_channel(4096);
        let preferences_path = data.join("web-preferences.json");
        let recovery_error = practice_backup::recover(&data).err();
        let loaded = Preferences::load(&preferences_path);
        let settings_error = recovery_error.or_else(||loaded.as_ref().err().cloned());
        Self {
            state: Snapshot {
                error: settings_error,
                ..Snapshot::default()
            },
            file: None,
            notes: vec![],
            note_hands: HashMap::new(),
            hand_undo: vec![],
            event_notes: HashMap::new(),
            original_grid: None,
            meter_correction: None,
            note_cursor: 0,
            events: vec![],
            event_cursor: 0,
            matcher: PracticeMatcher::new(KeyboardRange::new(0..=127)),
            session_clock: Duration::ZERO,
            history: PracticeHistoryStore::load(data.join("web-practice-history.ron")),
            title: String::new(),
            input_connection: None,
            input_tx,
            input_rx,
            input_overflow: Arc::new(AtomicBool::new(false)),
            output_connection: None,
            audio: None,
            soundfont,
            silent,
            preferences: loaded.unwrap_or_default(),
            preferences_path,
            restoring: false,
            track_routing: track_sound::Routing::default(),
            recorder: neothesia_core::recorder::FreeplayRecorder::default(),
            performance_capture:Default::default(),
            data: data.clone(),
            notation: None,
            score_revision: 0,
            vst: None,
            end_hold: 0.,
            previous_mode: None,
            finger_undo: Vec::new(),
            config: SongConfig::default(),
            fingers: HashMap::new(),
            exercise: None,
            previous_summary: None,
            coach: AdaptiveTempoCoach::default(),
            ladder: None,
            routine:None,
            last_click: None,
            finger_demo: None,
        }
    }
    fn annotation_path(&self, file: &MidiFile) -> Result<PathBuf, String> {
        if let Some(path) = &file.source_path {
            return Ok(path.clone());
        }
        let root = self.data.join("song-assets");
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let spec=self.exercise.ok_or("曲目缺少可保存的内容")?;
        let canonical=ExerciseSpec{hands:neothesia_core::exercise::ExerciseHands::Both,repetitions:1,tempo_bpm:120,minor_form:if spec.tonality==neothesia_core::exercise::ExerciseTonality::Major{neothesia_core::exercise::ExerciseMinorForm::Natural}else{spec.minor_form},..spec};
        let smf=ExercisePlan::generate(canonical,&KeyboardRange::new(21..=108)).map_err(|e|e.to_string())?.to_smf().map_err(|e|e.to_string())?;
        let mut bytes=Vec::new();smf.write_std(&mut bytes).map_err(|e|e.to_string())?;
        let bytes_id=blake3::hash(&bytes).to_hex().to_string();
        Ok(import_storage::store_with_identity(&root,&file.content_id,&bytes_id,&file.content_id,&bytes)?.path)

    }
    fn saved_hints(&self) -> Result<Vec<neothesia_core::library::FingerHint>, String> {
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        let path = self.annotation_path(file)?;
        match neothesia_core::library::load_song_sidecar(&path, &file.content_id) {
            Ok(s) => Ok(s.fingerings),
            Err(neothesia_core::library::MetadataError::Read(e))
                if e.kind() == std::io::ErrorKind::NotFound =>
            {
                Ok(vec![])
            }
            Err(e) => Err(e.to_string()),
        }
    }
    fn write_hints(&mut self,hints:Vec<neothesia_core::library::FingerHint>,remember:bool)->Result<(),String>{
        self.write_schedule(hints,self.saved_actions()?,remember)
    }
    fn write_schedule(
        &mut self,
        hints: Vec<neothesia_core::library::FingerHint>,
        actions:Vec<neothesia_core::library::FingerAction>,
        remember: bool,
    ) -> Result<(), String> {
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        for h in &hints {
            if !(1..=5).contains(&h.finger)
                || !file
                    .tracks
                    .iter()
                    .any(|t| t.track_id == h.track_id && h.note_index < t.notes.len())
            {
                return Err("指法包含无效的音符或手指编号，未保存任何修改".into());
            }
        }
        let previous = (self.saved_hints()?,self.saved_actions()?);
        let path = self.annotation_path(file)?;
        let mut sidecar=match neothesia_core::library::load_song_sidecar(&path,&file.content_id){Ok(s)=>s,Err(neothesia_core::library::MetadataError::Read(e)) if e.kind()==std::io::ErrorKind::NotFound=>Default::default(),Err(e)=>return Err(e.to_string())};
        sidecar.fingerings=hints.clone();sidecar.finger_actions=actions;
        neothesia_core::library::save_song_sidecar(&path,&file.content_id,sidecar).map_err(|e|e.to_string())?;
        if remember {
            self.finger_undo.push(previous);
            if self.finger_undo.len() > 30 {
                self.finger_undo.remove(0);
            }
        }
        self.fingers.clear();
        if let Some(spec) = self.exercise {
            let plan = ExercisePlan::generate(spec, &KeyboardRange::new(21..=108))
                .map_err(|e| e.to_string())?;
            if let Some(fs) = plan.fingerings() {
                for t in file.tracks.iter() {
                    if let Some(first) = t.notes.first() {
                        let fingers = if first.channel == 0 {
                            &fs.right
                        } else {
                            &fs.left
                        };
                        for (n, finger) in t.notes.iter().zip(fingers) {
                            self.fingers.insert((t.track_id, n.start, n.note), *finger);
                        }
                    }
                }
            }
        }
        for h in hints {
            let n = &file
                .tracks
                .iter()
                .find(|t| t.track_id == h.track_id)
                .unwrap()
                .notes[h.note_index];
            self.fingers.insert((h.track_id, n.start, n.note), h.finger);
        }
        Ok(())
    }
    fn begin_play(&mut self) {
        if self.state.count_in > 0 && self.session_clock == Duration::ZERO {
            let snap = self.snapshot();
            let f = self.file.as_ref().unwrap();
            let m = f.musical_time.measure_at(snap.tick).unwrap();
            self.state.count_remaining = f64::from(self.state.count_in) * f64::from(m.numerator)
                / if m.denominator == 8 && m.numerator >= 6 && m.numerator % 3 == 0 {
                    3.
                } else {
                    1.
                };
            self.state.status = "countIn".into();
            if let Some(a) = &self.audio {
                a.click(true)
            }
        } else {
            self.state.status = "playing".into();
        }
    }
    fn install(
        &mut self,
        mut file: MidiFile,
        title: String,
        plan: Option<ExercisePlan>,
    ) -> Result<(), String> {
        let previous_hands=plan.as_ref().and_then(|_|self.preferences.exercise_layouts.get(&file.content_id).copied().or_else(||self.preferences.exercise.and_then(|spec|ExercisePlan::generate(spec,&KeyboardRange::new(21..=108)).ok().filter(|old|old.practice_id()==file.content_id).map(|old|old.spec.hands))));
        let mut notes: Vec<_> = file
            .tracks
            .iter()
            .filter(|t| !t.has_drums || t.has_other_than_drums)
            .flat_map(|t| {
                t.notes.iter().enumerate().map(|(index, n)| PracticeNote {
                    inner: n.clone(),
                    index,
                })
            })
            .collect();
        notes.sort_by_key(|n| (n.start, n.track_id, n.note));
        if notes.is_empty() {
            return Err("这首 MIDI 没有可练习的钢琴音符".into());
        }
        self.panic();
        self.original_grid = Some(file.musical_time.clone());
        self.meter_correction = None;
        self.config = SongConfig::new(&file.tracks);
        self.state.mode = "wait".into();
        self.state.wait = true;
        self.state.hands = "both".into();
        self.state.speed = 1.;
        self.state.count_in = 1;
        self.state.metronome = false;
        self.state.rounds = 0;
        self.state.adaptive = false;

        self.fingers.clear();
        self.finger_undo.clear();
        self.note_hands.clear();
        self.hand_undo.clear();
        self.event_notes.clear();
        self.exercise = plan.as_ref().map(|p| p.spec);
        if let Some(plan) = plan {
            let fingers = plan.fingerings();
            for t in file.tracks.iter() {
                if let Some(config)=self.config.tracks.iter_mut().find(|config|config.track_id==t.track_id){
                    if t.track_id==1{config.practice_part=PracticePart::RightHand;}else if t.track_id==2{config.practice_part=PracticePart::LeftHand;}
                }

                if let Some(n) = t.notes.first() {
                    if let Some(c) = self
                        .config
                        .tracks
                        .iter_mut()
                        .find(|c| c.track_id == t.track_id)
                    {
                        c.practice_part = if n.channel == 0 {
                            PracticePart::RightHand
                        } else {
                            PracticePart::LeftHand
                        };
                    }
                    if let Some(f) = &fingers {
                        let fs = if n.channel == 0 { &f.right } else { &f.left };
                        for (n, f) in t.notes.iter().zip(fs) {
                            self.fingers.insert((t.track_id, n.start, n.note), *f);
                        }
                    }
                }
            }
        }
        self.notation = None;
        self.score_revision = self.score_revision.wrapping_add(1);
        {
            let path = self.annotation_path(&file)?;
            match neothesia_core::library::load_song_sidecar(&path, &file.content_id) {
                Ok(sidecar) => {
                    if let Some(meter) = sidecar.meter {
                        match Self::corrected_grid(&file, meter) {
                            Ok(grid) => {
                                file.measures = grid.bar_times(&file.tempo_track).into();
                                file.beats = grid.beat_times(&file.tempo_track).into();
                                file.musical_time = grid;
                                self.meter_correction = Some(meter);
                            }
                            Err(e) => self.state.error = Some(format!("小节修正无法恢复：{e}")),
                        }
                    }
                    for (track, part) in sidecar.track_parts {
                        if let Some(t) = self.config.tracks.iter_mut().find(|t| t.track_id == track)
                        {
                            t.practice_part = part;
                        }
                    }
                    for h in sidecar.hands {
                        if file
                            .tracks
                            .iter()
                            .find(|t| t.track_id == h.track_id)
                            .and_then(|t| t.notes.get(h.note_index))
                            .is_some()
                        {
                            self.note_hands.insert((h.track_id, h.note_index), h.part);
                        }
                    }
                    for h in sidecar.fingerings {
                        if let Some(n) = file
                            .tracks
                            .iter()
                            .find(|t| t.track_id == h.track_id)
                            .and_then(|t| t.notes.get(h.note_index))
                        {
                            self.fingers.insert((h.track_id, n.start, n.note), h.finger);
                        }
                    }
                    if let Some(association) = sidecar.score {
                        match neothesia_core::library::verify_score_association(&path, &association)
                        {
                            Ok(_) => {
                                let score_path = neothesia_core::library::resolve_score_path(
                                    &path,
                                    &association,
                                );
                                match std::fs::read(&score_path)
                                    .map_err(|e| e.to_string())
                                    .and_then(|bytes| {
                                        notation::ScoreAsset::new(
                                            bytes,
                                            self.preferences
                                                .score_versions
                                                .get(&file.content_id)
                                                .and_then(|rows| {
                                                    rows.iter().find(|v| v.path == score_path)
                                                })
                                                .map(|v| v.name.clone())
                                                .unwrap_or_else(|| {
                                                    score_path
                                                        .file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy()
                                                        .into()
                                                }),
                                            &file,
                                        )
                                    }) {
                                    Ok(s) => self.notation = Some(s),
                                    Err(e) => self.state.error = Some(format!("乐谱无法恢复：{e}")),
                                }
                            }
                            Err(e) => self.state.error = Some(format!("乐谱已更改或丢失：{e}")),
                        }
                    }
                }
                Err(neothesia_core::library::MetadataError::Read(e))
                    if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => self.state.error = Some(format!("曲目附加信息读取失败：{e}")),
            }
        }
        self.title = neothesia_core::library::load_song_metadata(
            &self.annotation_path(&file)?,
            &file.content_id,
        )
        .ok()
        .and_then(|m| m.title)
        .unwrap_or(title);
        self.state.duration = notes
            .iter()
            .map(|n| n.end.as_secs_f64())
            .fold(file.duration.as_secs_f64(), f64::max);
        for n in &notes {
            self.event_notes
                .insert((n.track_id, n.channel, n.note, n.start, true), n.index);
            self.event_notes
                .insert((n.track_id, n.channel, n.note, n.end, false), n.index);
        }
        self.notes = notes;
        self.events = file
            .tracks
            .iter()
            .flat_map(|t| t.events.iter().cloned())
            .collect();
        self.events.sort_by_key(|e| e.timestamp);
        if let Some(setup) = self.history.setup(&file.content_id).cloned() {
            self.config.apply_practice_setup(&setup);
            if (0.25..=2.).contains(&setup.speed) {
                self.state.speed = f64::from(setup.speed)
            }
        }
        self.state.passage = None;
        self.ladder = None;
        self.state.repetitions = 0;
        self.previous_summary = None;
        self.previous_mode = None;
        if let Some(l) = self
            .history
            .setup(&file.content_id)
            .and_then(|s| s.loop_setup)
            .filter(|l| l.enabled)
        {
            if let (Some(a), Some(b)) = (
                file.musical_time
                    .measures
                    .get(l.start_measure.saturating_sub(1)),
                file.musical_time
                    .measures
                    .get(l.end_measure.saturating_sub(1)),
            ) {
                self.state.passage = Some(Passage {
                    start: file
                        .tempo_track
                        .pulses_to_duration(a.start_tick)
                        .as_secs_f64(),
                    end: file
                        .tempo_track
                        .pulses_to_duration(b.end_tick)
                        .as_secs_f64()
                        .min(self.state.duration),
                });
            }
        }

        if let Some(settings) = self.preferences.songs.get(&file.content_id) {
            self.config.apply_practice_setup(&SongPracticeSetup {
                tracks: settings.tracks.clone(),
                ..SongPracticeSetup::default()
            });
            self.state.mode = settings
                .mode
                .clone()
                .filter(|mode| {
                    ["wait", "flow", "listen", "recital", "memory"].contains(&mode.as_str())
                })
                .unwrap_or_else(|| "wait".into());
            self.state.wait = self.state.mode == "wait";
            self.state.count_in = settings.count_in;
            self.state.metronome = settings.metronome;
            self.state.latency = settings.latency;
            self.state.rounds = settings.rounds;
            self.state.adaptive = settings.adaptive;
            if ["both", "left", "right", "custom"].contains(&settings.hands.as_str()) {
                self.state.hands = settings.hands.clone();
            }
            for t in &mut self.config.tracks {
                if let Some(part) = settings.parts.get(&t.track_id) {
                    t.practice_part = *part
                }
            }
        }
        if let Some(spec)=self.exercise{
            let mut scope_changed=false;
            for track in file.tracks.iter().filter(|track|!track.notes.is_empty()){
                let channel=track.notes[0].channel;
                let was_selected=previous_hands.is_some_and(|hands|hands==neothesia_core::exercise::ExerciseHands::Both||(hands==neothesia_core::exercise::ExerciseHands::Right&&channel==0)||(hands==neothesia_core::exercise::ExerciseHands::Left&&channel==1));
                if !was_selected{if let Some(config)=self.config.tracks.iter_mut().find(|config|config.track_id==track.track_id){config.player=PlayerConfig::Human;config.visible=true;config.practice_part=if channel==0{PracticePart::RightHand}else{PracticePart::LeftHand};scope_changed=true;}}
            }
            if scope_changed&&previous_hands!=Some(spec.hands){self.state.hands="both".into();}
        }
        if is_recital_mode(&self.state.mode) {
            self.state.passage = None;
            self.state.rounds = 1;
            self.state.adaptive = false;
        }
        self.file = Some(file);
        self.reset(self.state.passage.as_ref().map_or(0., |p| p.start));
        self.persist()?;
        Ok(())
    }
    fn restore(&mut self) {
        self.restoring = true;
        let prefs = self.preferences.clone();
        let mut errors = Vec::new();
        if let Some(spec) = prefs.exercise {
            if let Err(e) = self.command(Command::Generate { spec }) {
                errors.push(e)
            }
        } else if let Some(path) = prefs.path.clone() {
            if let Err(e) = self.command(Command::Load {
                path,
                title: prefs.title.clone(),
            }) {
                errors.push(format!("上次曲目无法恢复：{e}"));
            }
        }
        if let Some(speed) = prefs
            .speed
            .filter(|v| v.is_finite() && (0.25..=2.).contains(v))
        {
            self.state.speed = speed;
        }
        if self.preferences.songs.is_empty() {
            self.state.wait = prefs.wait.unwrap_or(true);
            self.state.mode = if self.state.wait { "wait" } else { "listen" }.into();
        }
        self.state.volume = prefs
            .volume
            .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
            .unwrap_or(0.8);
        if let Some(hands) = prefs
            .hands
            .clone()
            .filter(|h| ["both", "left", "right"].contains(&h.as_str()))
        {
            self.state.hands = hands;
        }
        if let Some(passage) = prefs.passage.clone() {
            if !is_recital_mode(&self.state.mode)
                && self.file.is_some()
                && passage.start.is_finite()
                && passage.end.is_finite()
                && passage.start >= 0.
                && passage.end <= self.state.duration
                && passage.end > passage.start
            {
                self.state.passage = Some(passage.clone());
                self.reset(passage.start);
            }
        }
        if let Some(input) = prefs.input.clone() {
            if let Err(e) = self.connect_input(Some(input.clone())) {
                self.state.input = Some(input);
                errors.push(e);
            }
        }
        if let Some(output) = prefs.output.clone() {
            if let Err(e) = self.command(Command::Output { name: Some(output) }) {
                errors.push(format!("上次音源无法连接，暂用内置钢琴：{e}"));
            }
        }
        self.preferences = prefs;
        if !self.silent && !self.preferences.input_configured && self.state.input.is_none() {
            if let Some(name) = devices()["inputs"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
            {
                let name = name.to_string();
                match self.connect_input(Some(name.clone())) {
                    Ok(_) => {
                        self.preferences.input = Some(name);
                    }
                    Err(e) => errors.push(e),
                }
            }
        }
        self.restoring = false;
        self.restore_latency();
        if !errors.is_empty() {
            self.state.error = Some(errors.join("；"));
        }
        // Never start transport automatically after opening the application.
    }
    fn persist(&mut self) -> Result<(), String> {
        if self.restoring {
            return Ok(());
        }
        self.preferences.speed = Some(self.state.speed);
        self.preferences.wait = Some(self.state.wait);
        self.preferences.hands = Some(self.state.hands.clone());
        self.preferences.passage = self.state.passage.clone();
        self.preferences.volume = Some(self.state.volume);
        if let Some(file) = &self.file {
            self.preferences.path = file.source_path.clone();
            self.preferences.title = self.title.clone();
        }
        if let Some(file) = &self.file {
            if let Some(run)=&self.ladder {self.preferences.ladder_runs.insert(format!("{}:{}",file.content_id,run.preset.id),run.clone());}
            self.preferences.exercise = self.exercise;
            if let Some(spec)=self.exercise{self.preferences.exercise_layouts.insert(file.content_id.clone(),spec.hands);}
            self.preferences.songs.insert(
                file.content_id.clone(),
                preferences::SessionSettings {
                    tracks: self.config.practice_track_setup(),
                    parts: self
                        .config
                        .tracks
                        .iter()
                        .map(|t| (t.track_id, t.practice_part))
                        .collect(),
                    mode: Some(self.state.mode.clone()),
                    count_in: self.state.count_in,
                    metronome: self.state.metronome,
                    latency: self.state.latency,
                    rounds: self.state.rounds,
                    hands: self.state.hands.clone(),
                    adaptive: self.state.adaptive,
                },
            );
            self.history
                .save_setup(
                    &file.content_id,
                    &self.title,
                    SongPracticeSetup {
                        source_path: file.source_path.clone(),
                        speed: self.state.speed as f32,
                        tracks: self.config.practice_track_setup(),
                        exercise_spec: self.exercise,
                        loop_setup: self.state.passage.as_ref().map(|p| {
                            neothesia_core::practice_history::PracticeLoopSetup {
                                enabled: true,
                                start_measure: file
                                    .measures
                                    .partition_point(|m| m.as_secs_f64() <= p.start)
                                    .max(1),
                                end_measure: file
                                    .measures
                                    .partition_point(|m| m.as_secs_f64() < p.end)
                                    .max(1),
                            }
                        }),
                        ..SongPracticeSetup::default()
                    },
                )
                .map_err(|e| e.to_string())?;
        }
        self.preferences.save(&self.preferences_path)
    }
    fn remember_active_score(&mut self) -> Result<(), String> {
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        let midi = self.annotation_path(file)?;
        let Ok(sidecar) = neothesia_core::library::load_song_sidecar(&midi, &file.content_id)
        else {
            return Ok(());
        };
        if let Some(association) = sidecar.score {
            let path = neothesia_core::library::resolve_score_path(&midi, &association);
            let entries = self
                .preferences
                .score_versions
                .entry(file.content_id.clone())
                .or_default();
            if !entries
                .iter()
                .any(|v| v.path == path && v.content_id == association.content_id)
            {
                if entries.len() >= 100 {
                    return Err("这首曲目已有 100 份谱面，请先移除不需要的版本".into());
                }
                let id = format!(
                    "score-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| e.to_string())?
                        .as_nanos()
                );
                entries.push(preferences::ScoreVersion {
                    id,
                    name: self
                        .notation
                        .as_ref()
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| {
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into()
                        }),
                    path,
                    content_id: association.content_id,
                });
                self.preferences.save(&self.preferences_path)?;
            }
        }
        Ok(())
    }
    fn score_content_guard(&self, id: &str) -> Result<(), String> {
        if self.file.as_ref().is_none_or(|f| f.content_id != id) {
            Err("曲目已切换，请重新打开谱面管理".into())
        } else {
            Ok(())
        }
    }
    fn corrected_grid(
        file: &MidiFile,
        meter: neothesia_core::library::MeterCorrection,
    ) -> Result<midi_file::musical_time::MusicalTime, String> {
        let end = file
            .tracks
            .iter()
            .flat_map(|t| t.notes.iter())
            .map(|n| {
                file.tempo_track
                    .seconds_to_pulses(n.end.as_secs_f64())
                    .round() as u64
            })
            .max()
            .unwrap_or(1);
        midi_file::musical_time::MusicalTime::corrected(
            file.musical_time.ppq,
            end,
            meter.numerator,
            meter.denominator,
            meter.pickup_ticks,
        )
    }
    fn grid_signature(&self)->String {self.file.as_ref().map(practice_conditions::grid).unwrap_or_else(||"grid-v1-cbf29ce484222325".into())}
    fn practice_scope(&self)->String {practice_conditions::scope(self.file.as_ref(),&self.notes,&self.config,&self.note_hands,&self.state.hands)}
    fn note_player(&self, track: usize, index: usize) -> PlayerConfig {
        let base = self
            .config
            .tracks
            .iter()
            .find(|t| t.track_id == track)
            .map_or(PlayerConfig::Mute, |t| t.player);
        if base != PlayerConfig::Human {
            return base;
        }
        let wanted = match self.state.hands.as_str() {
            "left" => Some(PracticePart::LeftHand),
            "right" => Some(PracticePart::RightHand),
            _ => None,
        };
        if wanted.is_some_and(|h| hands::part(&self.config, &self.note_hands, track, index) != h) {
            PlayerConfig::Auto
        } else {
            base
        }
    }
    fn set_hand_scope(&mut self, value: &str) -> Result<(), String> {
        let wanted = match value {
            "left" => Some(PracticePart::LeftHand),
            "right" => Some(PracticePart::RightHand),
            "both" => None,
            _ => return Err("练习声部无效".into()),
        };
        if wanted.is_some_and(|h| {
            !self
                .notes
                .iter()
                .any(|n| hands::part(&self.config, &self.note_hands, n.track_id, n.index) == h)
        }) {
            return Err("这首曲目还没有指定这一只手，请打开逐音分手".into());
        }
        let human_tracks: std::collections::HashSet<_> = self
            .notes
            .iter()
            .filter(|n| {
                wanted.is_none_or(|h| {
                    hands::part(&self.config, &self.note_hands, n.track_id, n.index) == h
                })
            })
            .map(|n| n.track_id)
            .collect();
        for t in &mut self.config.tracks {
            if self.notes.iter().any(|n| n.track_id == t.track_id) {
                t.player = if human_tracks.contains(&t.track_id) {
                    PlayerConfig::Human
                } else {
                    PlayerConfig::Auto
                };
            }
        }
        self.state.hands = value.into();
        Ok(())
    }
    fn write_hands(
        &mut self,
        hints: Vec<neothesia_core::library::NoteHandHint>,
        remember: bool,
    ) -> Result<(), String> {
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        for h in &hints {
            if file
                .tracks
                .iter()
                .find(|t| t.track_id == h.track_id)
                .and_then(|t| t.notes.get(h.note_index))
                .is_none()
            {
                return Err("音符不存在，分手信息未保存".into());
            }
        }
        neothesia_core::library::save_song_hands(
            &self.annotation_path(file)?,
            &file.content_id,
            hints.clone(),
        )
        .map_err(|e| e.to_string())?;
        if remember {
            self.hand_undo.push(hands::hints(&self.note_hands));
            if self.hand_undo.len() > 30 {
                self.hand_undo.remove(0);
            }
        }
        self.note_hands = hints
            .into_iter()
            .map(|h| ((h.track_id, h.note_index), h.part))
            .collect();
        let scope = self.state.hands.clone();
        if scope != "custom" && self.set_hand_scope(&scope).is_err() {
            self.set_hand_scope("both")?;
        }
        self.previous_summary = None;
        self.previous_mode = None;
        self.coach = AdaptiveTempoCoach::default();
        self.state.status = "paused".into();
        self.reset(self.state.passage.as_ref().map_or(0., |p| p.start));
        Ok(())
    }
    fn selected(&self, track: usize) -> bool {
        self.config
            .tracks
            .iter()
            .any(|t| t.track_id == track && t.player == PlayerConfig::Human)
    }
    fn ensure_audio(&mut self) -> Result<(), String> {
        if !self.silent
            && self.audio.is_none()
            && (self.output_connection.is_none() || self.state.metronome || self.state.count_in > 0)
        {
            self.audio = Some(audio::Audio::open(&self.soundfont)?);
            self.audio.as_ref().unwrap().set_volume(self.state.volume);
            self.restore_track_sound();
        }
        Ok(())
    }
    fn panic(&mut self) {
        self.performance_capture.close(self.session_clock.as_secs_f64(),self.state.position);
        self.finger_demo = None;
        if let Some(vst) = &self.vst {
            vst.stop_all();
        }
        if let Some(audio) = &self.audio {
            audio.stop();
        }
        if let Some(out) = &mut self.output_connection {
            for channel in 0..16 {
                for (cc, value) in [(64, 0), (123, 0), (120, 0)] {
                    let _ = out.send(&[176 | channel, cc, value]);
                }
            }
        }
        self.state.pressed.clear();
        self.state.pedal = 0;
    }
    fn send(&mut self, bytes: &[u8]) {
        if let Some(vst) = &self.vst {
            if let Ok(midi_file::midly::live::LiveEvent::Midi { channel, message }) =
                midi_file::midly::live::LiveEvent::parse(bytes)
            {
                vst.midi_event(channel, message);
            }
        } else if let Some(out) = &mut self.output_connection {
            if let Err(e) = out.send(bytes) {
                self.state.error = Some(format!("MIDI 输出失败：{e}"));
                self.state.status = "paused".into();
            }
        } else if let Some(audio) = &self.audio {
            audio.send(bytes);
        }
    }
    fn save(&mut self) -> Result<(), String> {
        if self.state.saved {
            return Ok(());
        }
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        if self.matcher.snapshot().matched_notes
            + self.matcher.snapshot().wrong_notes
            + self.matcher.snapshot().missed_notes
            == 0
        {
            return Err("还没有演奏记录，无法保存结果".into());
        }
        let hands = match self.state.hands.as_str() {
            "left" => PracticeHands::Left,
            "right" => PracticeHands::Right,
            "custom" => PracticeHands::Custom,
            _ => PracticeHands::Both,
        };
        let session=PracticeSession::new(
                    self.state
                        .passage
                        .as_ref()
                        .map_or(PracticeSessionKind::WholeSong, |p| {
                            PracticeSessionKind::Loop {
                                start_measure: file
                                    .measures
                                    .partition_point(|m| m.as_secs_f64() <= p.start)
                                    .max(1),
                                end_measure: file
                                    .measures
                                    .partition_point(|m| m.as_secs_f64() < p.end)
                                    .max(1),
                            }
                        }),
                    hands,
                    self.state.speed as f32,
                    self.matcher.summary(),
                )
                .with_playing_ms(self.session_clock.as_millis().min(u128::from(u64::MAX)) as u64)
                .with_context(self.history_context())
                .with_scope(self.practice_scope())
                .with_mode(&self.state.mode)
                .with_effective_tempo_bpm(Some(
                    (self.snapshot().bpm * self.state.speed)
                        .round()
                        .clamp(1., 65535.) as u16,
                ));
        performance_archive::save(&self.data,&file.content_id,&session.stable_id(),&self.performance_capture,self.session_clock.as_secs_f64(),self.matcher.results())?;
        self.history.record_session(&file.content_id,&self.title,session).map_err(|e|e.to_string())?;
        self.previous_summary = Some(self.matcher.summary());
        self.previous_mode = Some(self.state.mode.clone());
        self.state.saved = true;
        Ok(())
    }
    fn reset(&mut self, position: f64) {
        self.panic();
        self.matcher.reset();
        self.performance_capture=Default::default();
        self.session_clock = Duration::ZERO;
        self.state.position = position;
        if self.state.metronome {
            let snap = self.snapshot();
            let click = (snap.measure, snap.beat.floor() as usize);
            if self.last_click != Some(click) {
                if let Some(a) = &self.audio {
                    a.click(click.1 == 1)
                }
                self.last_click = Some(click);
            }
        }
        self.state.saved = false;
        self.note_cursor = self
            .notes
            .partition_point(|n| n.start.as_secs_f64() < position);
        self.event_cursor = self
            .events
            .partition_point(|e| e.timestamp.as_secs_f64() < position);
        self.restore_track_sound();
        self.state.status = "ready".into();
        self.state.count_remaining = 0.;
        self.last_click = None;
        self.end_hold = 0.;
    }
    fn command(&mut self, command: Command) -> Result<serde_json::Value, String> {
        if self.finger_demo.is_some() && !matches!(&command, Command::StartFingerDemo{..}|Command::FingerDemoControl{..}|Command::StartHeldFingerDemo{..}|Command::StartHistoryReplay{..}|Command::HistoryReplayControl{..}|Command::HistoryReplayLoop{..}|Command::HistoryProblemNotes{..}|Command::PreviewProblemCuts{..}|Command::PreviewMultiProblemPlan{..}|Command::SaveMultiProblemPlan{..}|Command::PreviewHistoryProblemPlan{..}|Command::SaveHistoryProblemPlan{..}|Command::HistoryReplayClips{..}|Command::SaveHistoryReplayClip{..}|Command::DeleteHistoryReplayClip{..}|Command::HistoryPerformance{..}|Command::StopFingerDemo{..}|Command::FingerDemoState|Command::CurrentSong|Command::Volume{..}) {
            self.panic();
        }
        if self.routine.is_some() && matches!(command,
            Command::Mode{..}|Command::Wait{..}|Command::Speed{..}|Command::Hands{..}|Command::Track{..}
            |Command::Meter{..}|Command::ApplyHands{..}|Command::NoteHand{..}|Command::UndoHands
            |Command::MeasureLoop{..}|Command::Loop{..}|Command::ApplyScoreRange{..}|Command::Rounds{..}|Command::Adaptive{..}
            |Command::Seek{..}|Command::SeekMeasure{..}|Command::SeekBeat{..}|Command::StepBeat{..}
            |Command::CountIn{..}|Command::Metronome{..}|Command::Latency{..}|Command::Save|Command::Recording{active:true}) {
            return Err("正在练习计划项目，请先结束本项再修改条件".into());
        }
        if self.routine.is_some() && matches!(command,Command::Load{..}|Command::Generate{..}|Command::ImportScore{..}|Command::ImportMidi{..}|Command::ImportPackage{..}|Command::OpenRecent{..}|Command::HistoryOpen{..}|Command::OpenPassage{..}|Command::StartLadder{..}) {self.stop_routine()?;}
        match &command {
            Command::Routines{day}=>return self.routines(day),
            Command::SaveRoutine{id,day,name,notes,copy_of}=>return self.save_routine(id.clone(),day.clone(),name.clone(),notes.clone(),copy_of.clone()),
            Command::SaveRoutineSchedule{id,schedule}=>return self.save_routine_schedule(id,schedule.clone()),
            Command::RemoveRoutine{id}=>return self.remove_routine(id),
            Command::PassageCatalog{content_id,query,offset}=>return self.passage_catalog(content_id.as_deref(),query,*offset),
            Command::ComposePassages{routine_id,day,content_id,entries,goal,proof}=>return self.compose_passages(routine_id,day.as_deref(),content_id,entries,goal,proof.as_deref()),
            Command::SaveRoutineItem{routine_id,day,id,source,source_id,title,notes,goal}=>return self.save_routine_item(routine_id,day.as_deref(),id.clone(),source.as_deref(),source_id.as_deref(),title.clone(),notes.clone(),goal.clone()),
            Command::MoveRoutineItem{routine_id,day,item_id,direction}=>return self.move_routine_item(routine_id,day.as_deref(),item_id,*direction),
            Command::RemoveRoutineItem{routine_id,day,item_id}=>return self.remove_routine_item(routine_id,day.as_deref(),item_id),
            Command::OpenRoutineItem{routine_id,day,item_id,resume}=>return self.open_routine_item(routine_id,day,item_id,*resume),
            Command::PreviewTickRange{content_id,grid,start,end}=>return self.preview_tick_range(content_id,grid,start,end),
            Command::ApplyTickRange{range}=>return self.apply_tick_range(range),
            Command::SaveTickPassage{id,name,notes,range}=>return self.save_tick_passage(id.clone(),name.clone(),notes.clone(),range.clone()),
            Command::StopRoutine=>return self.stop_routine(),
            Command::SkipRoutineItem{routine_id,day,item_id,reason}=>return self.mark_routine_item(routine_id,day,item_id,false,reason),
            Command::ResetRoutineItem{routine_id,day,item_id}=>return self.mark_routine_item(routine_id,day,item_id,true,""),
            Command::StopLadder if self.routine.is_some()=>return self.stop_routine(),
            _=>{}
        }
        if self.ladder.as_ref().is_some_and(|r|r.active) && matches!(command,
            Command::Mode{..}|Command::Wait{..}|Command::Speed{..}|Command::Hands{..}|Command::Track{..}
            |Command::Meter{..}|Command::ApplyHands{..}|Command::NoteHand{..}|Command::UndoHands
            |Command::MeasureLoop{..}|Command::Loop{..}|Command::ApplyScoreRange{..}|Command::Rounds{..}|Command::Adaptive{..}
            |Command::OpenPassage{..}|Command::Seek{..}|Command::SeekMeasure{..}|Command::SeekBeat{..}|Command::StepBeat{..}
            |Command::CountIn{..}|Command::Metronome{..}|Command::Latency{..}|Command::Save|Command::Recording{active:true}) {
            return Err("速度阶梯进行中，请先结束阶梯再修改练习条件".into());
        }
        match &command {
            Command::LadderPresets=>return self.ladder_presets(),
            Command::SaveLadder{id,name,start,end,plan}=>return self.save_ladder(id.clone(),name,*start,*end,plan.clone()),
            Command::RemoveLadder{id}=>return self.remove_ladder(id),
            Command::StartLadder{id,resume}=>return self.start_ladder(id,*resume),
            Command::StopLadder=>return self.stop_ladder(),
            _=>{}
        }
        let workspace_lock = if matches!(
            command,
            Command::ImportScore { .. }
                | Command::ImportMidi { .. }
                | Command::ImportPackage { .. }
                | Command::ExportPackage | Command::ExportPackageFile
        ) {
            Some(library_workspace::lock_for(&self.data)?)
        } else {
            None
        };
        let _workspace_guard = workspace_lock
            .as_ref()
            .map(|lock| lock.lock().map_err(|_| "曲库文件正忙"))
            .transpose()?;
        if self.recorder.is_recording()
            && matches!(
                command,
                Command::ImportScore { .. }
                    | Command::ImportPackage { .. }
                    | Command::ImportMidi { .. }
                    | Command::Load { .. }
                    | Command::OpenRecent { .. }
                    | Command::HistoryOpen { .. }
                    | Command::HistoryPracticeRange { .. }
                    | Command::HistoryScoreReview { .. }
                    | Command::Generate { .. }
                    | Command::OpenPassage { .. }
                    | Command::Play
                    | Command::Restart
                    | Command::Seek { .. }
                    | Command::SeekBeat { .. }
                    | Command::StepBeat { .. }
                    | Command::SeekMeasure { .. }
                    | Command::MeasureLoop { .. }
                    | Command::Loop { .. }
                    | Command::ApplyScoreRange { .. }
                    | Command::Mode { .. }
                    | Command::Track { .. }
                    | Command::ApplyHands { .. }
                    | Command::NoteHand { .. }
                    | Command::Meter { .. }
                    | Command::UndoHands
                    | Command::Hands { .. }
            )
        {
            return Err("请先停止录音，再切换曲目或练习设置".into());
        }
        if is_recital_mode(&self.state.mode) {
            if matches!(
                command,
                Command::MeasureLoop { enabled: true, .. }
                    | Command::Loop { enabled: true, .. }
                    | Command::OpenPassage { .. }
                    | Command::SavePassage { .. }
                    | Command::SaveScorePassage { .. }
                    | Command::ApplyScoreRange { .. }
                    | Command::Adaptive { enabled: true }
            ) {
                return Err("完整演奏与背谱只演奏一遍；选段循环请切换到连续或等音模式".into());
            }
            if ["playing", "countIn", "paused"].contains(&self.state.status.as_str())
                && matches!(
                    command,
                    Command::Seek { .. }
                        | Command::SeekMeasure { .. }
                        | Command::SeekBeat { .. }
                        | Command::StepBeat { .. }
                        | Command::Track { .. }
                        | Command::Hands { .. }
                        | Command::Meter { .. }
                        | Command::NoteHand { .. }
                        | Command::ApplyHands { .. }
                        | Command::UndoHands
                        | Command::Speed { .. }
                        | Command::CountIn { .. }
                        | Command::Rounds { .. }
                )
            {
                return Err("本轮演奏已开始，请先重新开始，再修改范围、速度或声部".into());
            }
        }
        if library_workspace::accepts(&command) {
            return library_workspace::execute(command, &self.data);
        }
        if matches!(command, Command::Collection) {
            let index=library_workspace::Workspace::load(&self.data)?;
            let songs: Vec<_> = self.history.recent_songs(usize::MAX).into_iter().map(|s| serde_json::json!({
                "contentId": s.content_id, "title": s.display_name, "storedPath":s.source_path, "path": library::available_path(&index,&s.content_id,s.source_path.as_deref()),
                "favorite": s.favorite, "queuePosition": s.queue_position, "lastUsed": s.last_used_unix_ms,
                "lastPracticed":self.history.song(&s.content_id).and_then(|h|h.sessions.last()).map(|s|s.recorded_at_unix_ms), "sessions": s.session_count, "accuracy": s.latest_accuracy,"available":s.source_path.is_none()||library::available_path(&index,&s.content_id,s.source_path.as_deref()).is_some(),"recommended":s.recommended_measures,"review":s.review.map(|r|serde_json::json!({"due":r.is_due,"days":r.days_until_due,"streak":r.mastery_streak})), "generated":self.history.setup(&s.content_id).is_some_and(|s|s.exercise_spec.is_some())
            })).collect();
            return Ok(serde_json::json!({ "songs": songs }));
        }
        if let Command::OpenRecent { content_id } = &command {
            let record = self.history.song(content_id).ok_or("找不到该练习记录")?;
            if let Some(spec) = record.setup.as_ref().and_then(|s| s.exercise_spec) {
                return self.command(Command::Generate { spec });
            }
            let paths=record.setup.as_ref().and_then(|s|s.source_path.clone()).into_iter().chain(record.library.source_path.clone());
            let file=library::verified_file(&self.data,content_id,paths)?;
            let title=record.display_name.clone();
            self.install(file,title,None)?;
            self.persist()?;
            return self.command(Command::CurrentSong);
        }
        match &command {
            Command::WeakPractice=>return self.weak_practice(),
            Command::AcceptWeakPractice{ids,routine_id,day,name}=>return self.accept_weak_practice(ids,routine_id.as_deref(),day.as_deref(),name),
            Command::HistoryQuery { query, mode, hands, range, days, offset, limit } => {
                return self.history_query(query, mode, hands, range, *days, *offset, *limit);
            }
            Command::AnnotateHistory{content_id,id,note,teacher,next,expected}=>return self.annotate_history(content_id,id,note.clone(),teacher.clone(),next.clone(),expected.clone()),
            Command::SetLibraryCollection{path,content_id,favorite,queued} => return self.set_library_collection(path,content_id,*favorite,*queued),
            Command::PracticeTime {content_id,from,to} => return self.practice_time(content_id,*from,*to),
            Command::HistoryPerformance{content_id,id}=>return self.history_performance(content_id,id),
            Command::StartHistoryReplay{content_id,id,source,rate}=>return self.start_history_replay(content_id,id,source,*rate),
            Command::HistoryReplayControl{id,action,position,rate}=>return self.history_replay_control(id,action,*position,*rate),
            Command::HistoryReplayLoop{id,start,end,enabled}=>return self.history_replay_loop(id,*start,*end,*enabled),
            Command::HistoryReplayClips{content_id,id}=>return self.history_replay_clips(content_id,id),
            Command::SaveHistoryReplayClip{content_id,id,clip_id,name,start,end,notes,expected}=>return self.save_history_replay_clip(content_id,id,clip_id.as_deref(),name,*start,*end,notes,expected),
            Command::DeleteHistoryReplayClip{content_id,id,clip_id,expected}=>return self.delete_history_replay_clip(content_id,id,clip_id,expected),
            Command::HistoryDetail { content_id, id } => return self.history_detail(content_id, id),
            Command::HistoryOpen { content_id, id, restore } => return self.history_open(content_id, id, *restore),
            Command::HistoryProblemNotes{content_id,id,offset}=>return self.history_problem_notes(content_id,id,*offset),
            Command::PreviewProblemCuts{single,multi,expected,edits}=>{let built=match(single,multi){(Some(s),None)=>self.build_history_problem_plan(s)?,(None,Some(s))=>self.build_multi_problem_plan(s)?,_=>return Err("请选择唯一编排来源".into())};let b=self.apply_problem_cuts(built,expected,edits)?;return Ok(serde_json::json!({"rows":b.rows,"revision":b.revision}));}
            Command::PreviewMultiProblemPlan{spec}=>return self.preview_multi_problem_plan(spec),
            Command::SaveMultiProblemPlan{spec,expected,name,notes,goal,edits}=>return self.save_multi_problem_plan(spec,expected,name,notes,goal,edits),
            Command::PreviewHistoryProblemPlan{spec}=>return self.preview_history_problem_plan(spec),
            Command::SaveHistoryProblemPlan{spec,expected,name,notes,goal,edits}=>return self.save_history_problem_plan(spec,expected,name,notes,goal,edits),
            Command::HistoryPracticeRange {content_id,id,reference,before,after}=>return self.history_practice_range(content_id,id,*reference,*before,*after),
            Command::HistoryScoreReview {content_id,id}=>return self.history_score_review(content_id,id),
            Command::History => return self.history_query("", "", "", "", 0, 0, 200),
            _ => {}
        }
        let persist = !matches!(
            command,
            Command::Note { .. }
                | Command::Play
                | Command::Pause
                | Command::Panic
                | Command::Save
                | Command::Feedback
                | Command::ExportPackage | Command::ExportPackageFile
                | Command::InspectPackage { .. }
                | Command::ExportMidi
                | Command::Metadata
                | Command::Calibration
                | Command::Score
                | Command::ScoreMap { .. } | Command::ExportAnnotatedScore { .. }
                | Command::SuggestHands { .. }
                | Command::SuggestFingers { .. } | Command::FingerPlanContext { .. }
                | Command::Seek { .. }
        );
        let _legacy_persist = matches!(
            command,
            Command::Load { .. }
                | Command::Speed { .. }
                | Command::Wait { .. }
                | Command::Hands { .. }
                | Command::Input { .. }
                | Command::Output { .. }
                | Command::Loop { .. }
                | Command::Volume { .. }
        );
        if matches!(command, Command::CurrentSong) {
            let current_title = self
                .file
                .as_ref()
                .and_then(|f| {
                    self.annotation_path(f)
                        .ok()
                        .and_then(|path| {
                            neothesia_core::library::load_song_metadata(&path, &f.content_id).ok()
                        })
                        .and_then(|m| m.title)
                })
                .unwrap_or_else(|| self.title.clone());
            return match &self.file {
                Some(file) => serde_json::to_value(document::build(
                    file,
                    &current_title,
                    self.state.duration,
                    &self.config,
                    &self.fingers,
                    &self.note_hands,
                    self.meter_correction,
                    self.exercise.is_some(),
                    self.notation.is_some(),
                    self.score_revision,
                ))
                .map_err(|e| e.to_string()).map(|mut v|{v["fingerUndoAvailable"]=serde_json::json!(!self.finger_undo.is_empty());v["trackSounds"]=serde_json::json!(self.track_sound_settings());v["soundPresets"]=serde_json::json!(track_sound::presets(&self.soundfont));self.decorate_tracks(&mut v);self.decorate_finger_actions(&mut v);v}),
                None => Ok(serde_json::Value::Null),
            };
        }
        self.state.error = None;
        match command {
            Command::TrackAppearance{id,appearance} => return self.set_track_appearance(id,appearance),
            Command::TrackSound{id,sound} => return self.set_track_sound(id,sound),
            Command::ExportPackage | Command::ExportPackageFile => {
                self.command(Command::Pause)?;
                return self.export_piece_package(matches!(command,Command::ExportPackageFile));
            }
            Command::ImportStagedPackage { .. } => return Err("请先预览曲目包".into()),
            Command::InspectPackage { bytes } => {
                let package = piece_package::Package::decode(&bytes)?;
                let mut result = package.summary();
                result["existing"] = serde_json::json!(
                    self.history.song(&package.manifest.content_id).is_some()
                        || library_workspace::Workspace::load(&self.data)?
                            .entries
                            .values()
                            .any(|e| e.content_id.as_ref() == Some(&package.manifest.content_id))
                );
                return Ok(result);
            }
            Command::ImportPackage { bytes, policy } => {
                return self.import_piece_package(bytes, policy);
            }
            Command::ImportMidi { bytes, name } => {
                if bytes.is_empty() || bytes.len() > 32_000_000 {
                    return Err("MIDI 文件为空或超过 32 MB".into());
                }
                let smf = midi_file::midly::Smf::parse(&bytes).map_err(|e| e.to_string())?;
                MidiFile::from_smf(&name, &smf)?;
                let root = self.data.join("imports");
                std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
                let cid=blake3::hash(&bytes).to_hex().to_string();
                let stored=import_storage::store(&root,&cid,&cid,&bytes)?;
                let path=stored.path.clone();
                let title = PathBuf::from(name)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.command(Command::Load {
                    path: path.clone(),
                    title: title.clone(),
                })?;
                let mut index = library_workspace::Workspace::load(&self.data)?;
                let entry = index.inspect(path.clone(), false)?;
                // The cache filename is a content hash. Preserve the user's
                // import name in metadata so later library searches and score
                // pairing do not expose that hash as the work's title.
                // Existing titles, tags and personal annotations take priority.
                if entry.metadata.title.as_deref().is_none_or(|v| v.trim().is_empty()) && !title.trim().is_empty() {
                    let mut metadata = entry.metadata;
                    metadata.title = Some(title);
                    neothesia_core::library::save_song_metadata(&path, &cid, metadata).map_err(|e| e.to_string())?;
                    index.inspect(path, true)?;
                }
                index.save(&self.data)?;
                let mut result=self.command(Command::CurrentSong)?;
                result["importRecovery"]=serde_json::json!(stored.recovery);result["importWarnings"]=serde_json::json!(stored.warning().into_iter().collect::<Vec<_>>());
                return Ok(result);
            }
            Command::ImportScore {
                bytes,
                name,
                default_bpm,
            } => {
                if bytes.is_empty() || bytes.len() > 4_000_000 {
                    return Err("乐谱文件为空或超过 4 MB".into());
                }
                let score = neothesia_core::musicxml::import_musicxml_document(&bytes)
                    .map_err(|e| e.to_string())?;
                let performance = neothesia_core::score_performance::generate(&score, default_bpm)?;
                let file = MidiFile::from_smf(&name, &performance.smf)?;
                let title = score.title.clone().unwrap_or_else(|| {
                    PathBuf::from(&name)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into()
                });
                let root = self.data.join("imports");
                std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
                let mut midi_bytes=Vec::new();performance.smf.write_std(&mut midi_bytes).map_err(|e|e.to_string())?;
                let stored=import_storage::store(&root,&file.content_id,&file.content_id,&midi_bytes)?;
                let path=stored.path.clone();
                match neothesia_core::library::load_song_sidecar(&path, &file.content_id) {
                    Ok(_) => {}
                    Err(neothesia_core::library::MetadataError::Read(e))
                        if e.kind() == std::io::ErrorKind::NotFound =>
                    {
                        let mut track_parts = std::collections::BTreeMap::new();
                        let mut fingers = vec![];
                        for (i, staff) in performance.tracks.iter().enumerate() {
                            let hints: HashMap<_, _> = staff
                                .notes
                                .iter()
                                .map(|v| ((v.start, v.pitch), v.finger))
                                .collect();
                            let t = &file.tracks[i + 1];
                            track_parts.insert(t.track_id, staff.part);
                            for (index, n) in t.notes.iter().enumerate() {
                                let start = file
                                    .tempo_track
                                    .seconds_to_pulses(n.start.as_secs_f64())
                                    .round() as u64;
                                if let Some(finger) = hints.get(&(start, n.note)).copied().flatten()
                                {
                                    fingers.push(neothesia_core::library::FingerHint {
                                        track_id: t.track_id,
                                        note_index: index,
                                        finger,
                                    });
                                }
                            }
                        }
                        neothesia_core::library::save_song_sidecar(
                            &path,
                            &file.content_id,
                            neothesia_core::library::SongSidecar {
                                metadata: neothesia_core::library::SongMetadata {
                                    title: Some(title.clone()),
                                    composer: score.composer.clone(),
                                    tags: vec!["乐谱导入".into()],
                                    notes: (!performance.warnings.is_empty())
                                        .then(|| performance.warnings.join("；")),
                                    ..Default::default()
                                },
                                track_parts,
                                fingerings: fingers,
                                ..Default::default()
                            },
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    Err(e) => return Err(e.to_string()),
                }
                self.command(Command::Load {
                    path: path.clone(),
                    title,
                })?;
                self.command(Command::PairScore {
                    bytes,
                    name,
                    content_id: file.content_id,
                })?;
                let mut index = library_workspace::Workspace::load(&self.data)?;
                index.inspect(path, true)?;
                index.save(&self.data)?;
                let mut result = self.command(Command::CurrentSong)?;
                let mut warnings=performance.warnings;warnings.extend(stored.warning());
                result["importRecovery"]=serde_json::json!(stored.recovery);
                result["importWarnings"] = serde_json::json!(warnings);
                return Ok(result);
            }
            Command::Meter { value } => {
                let file = self.file.as_ref().ok_or("请先选择曲目")?;
                let grid = match value {
                    Some(meter) => Self::corrected_grid(file, meter)?,
                    None => self.original_grid.clone().ok_or("原始小节网格不存在")?,
                };
                neothesia_core::library::save_song_meter(
                    &self.annotation_path(file)?,
                    &file.content_id,
                    value,
                )
                .map_err(|e| e.to_string())?;
                self.panic();
                self.state.status = "paused".into();
                self.state.passage = None;
                self.previous_summary = None;
                self.previous_mode = None;
                let file = self.file.as_mut().unwrap();
                file.measures = grid.bar_times(&file.tempo_track).into();
                file.beats = grid.beat_times(&file.tempo_track).into();
                file.musical_time = grid;
                self.meter_correction = value;
                if let Some(asset) = self.notation.take() {
                    self.notation = Some(notation::ScoreAsset::new(asset.bytes, asset.name, file)?);
                }
                self.score_revision = self.score_revision.wrapping_add(1);
                self.coach = AdaptiveTempoCoach::default();
                self.reset(0.);
            }
            Command::SuggestHands {
                track,
                range,
                reference,
            } => {
                let file = self.file.as_ref().ok_or("请先选择曲目")?;
                let proposals = hands::propose(
                    file,
                    &self.config,
                    &self.note_hands,
                    track,
                    range,
                    reference,
                )?;
                return Ok(
                    serde_json::json!({"contentId":file.content_id,"proposals":proposals,"undoAvailable":!self.hand_undo.is_empty()}),
                );
            }
            Command::ApplyHands { content_id, hints } => {
                if self
                    .file
                    .as_ref()
                    .is_none_or(|f| f.content_id != content_id)
                {
                    return Err("曲目已切换，请重新审阅分手建议".into());
                }
                if hints.len() > 4096 {
                    return Err("一次最多接受 4096 个分手标注".into());
                }
                let mut merged = hands::hints(&self.note_hands);
                for h in hints {
                    merged.retain(|v| v.track_id != h.track_id || v.note_index != h.note_index);
                    merged.push(h);
                }
                self.write_hands(merged, true)?;
            }
            Command::NoteHand { track, index, part } => {
                let file = self.file.as_ref().ok_or("请先选择曲目")?;
                if file
                    .tracks
                    .iter()
                    .find(|t| t.track_id == track)
                    .and_then(|t| t.notes.get(index))
                    .is_none()
                {
                    return Err("音符不存在".into());
                }
                let mut hints = hands::hints(&self.note_hands);
                hints.retain(|v| v.track_id != track || v.note_index != index);
                if let Some(part) = part {
                    hints.push(neothesia_core::library::NoteHandHint {
                        track_id: track,
                        note_index: index,
                        part,
                    });
                }
                self.write_hands(hints, true)?;
            }
            Command::UndoHands => {
                let previous = self
                    .hand_undo
                    .last()
                    .cloned()
                    .ok_or("没有可撤销的分手修改")?;
                self.write_hands(previous, false)?;
                self.hand_undo.pop();
            }
            Command::HandSpanDefault { hand, profile } => {
                if hand == PracticePart::Other {
                    return Err("请指定左手或右手".into());
                }
                self.preferences.hand_span_defaults.insert(hand, profile);
            }
            Command::FingerProfiles => {
                let f = self.file.as_ref().ok_or("请先选择曲目")?;
                return Ok(
                    serde_json::json!({"profiles":self.preferences.fingering_profiles.get(&f.content_id),"profilesByHand":self.preferences.hand_span_profiles.get(&f.content_id),"defaults":self.preferences.hand_span_defaults}),
                );
            }
            Command::PreviewScoreRangeTrim{content_id,score_revision,first,last,start_trim,end_trim}=>{self.score_range_context(&content_id,score_revision)?;return self.preview_score_range_trim(first,last,start_trim,end_trim);}
            Command::PreviewScoreRange {content_id,score_revision,first,last} => {
                self.score_range_context(&content_id,score_revision)?;
                return self.preview_score_range(first,last);
            }
            Command::ApplyScoreRange {content_id,score_revision,range} => {
                self.score_range_context(&content_id,score_revision)?;
                return self.apply_score_range(&range);
            }
            Command::SaveScorePassage {content_id,score_revision,id,name,notes,range} => {
                self.score_range_context(&content_id,score_revision)?;
                return self.save_score_passage(id,name,notes,range);
            }
            Command::Passages => {
                let f = self.file.as_ref().ok_or("请先选择曲目")?;
                return Ok(
                    serde_json::json!({"contentId":f.content_id,"grid":self.grid_signature(),"passages":self.preferences.passages.get(&f.content_id).cloned().unwrap_or_default()}),
                );
            }
            Command::UpdatePassageDetails {content_id,id,name,notes} => {
                if self.file.as_ref().is_none_or(|f|f.content_id!=content_id){return Err("曲目已切换，请重新打开练习段落".into());}
                if name.trim().is_empty() || name.chars().count()>80 || notes.len()>8192{return Err("练习段名称或备注过长/为空".into());}
                let mut prefs=self.preferences.clone();
                let preset=prefs.passages.get_mut(&content_id).and_then(|p|p.iter_mut().find(|p|p.id==id)).ok_or("练习段已不存在，请刷新列表")?;
                preset.name=name.trim().into();preset.notes=notes;
                prefs.save(&self.preferences_path)?;self.preferences=prefs;
                return Ok(serde_json::json!({"saved":true}));
            }
            Command::SavePassage {
                id,
                name,
                start,
                end,
                notes,
            } => {
                let f = self.file.as_ref().ok_or("请先选择曲目")?;
                if id.as_ref().is_some_and(|id|self.preferences.passages.get(&f.content_id).is_some_and(|p|p.iter().any(|p|&p.id==id&&(p.score_range.is_some()||p.precise_range.is_some())))){return Err("精确练习段请重新审阅对应范围，或只更新名称与备注".into());}
                if name.trim().is_empty() || name.chars().count() > 80 {
                    return Err("练习段名称需要 1～80 个字".into());
                }
                if start == 0 || start > end || end > f.musical_time.measures.len() {
                    return Err("小节范围无效".into());
                }
                if notes.len() > 8192 {
                    return Err("练习备注过长".into());
                }
                let settings = preferences::SessionSettings {
                    tracks: self.config.practice_track_setup(),
                    parts: self
                        .config
                        .tracks
                        .iter()
                        .map(|t| (t.track_id, t.practice_part))
                        .collect(),
                    mode: Some(self.state.mode.clone()),
                    count_in: self.state.count_in,
                    metronome: self.state.metronome,
                    latency: self.state.latency,
                    rounds: self.state.rounds,
                    hands: self.state.hands.clone(),
                    adaptive: self.state.adaptive,
                };
                let id = id.unwrap_or_else(|| {
                    format!(
                        "passage-{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos()
                    )
                });
                let preset = preferences::PassagePreset {
                    grid: Some(self.grid_signature()),
                    score_range: None,
                    precise_range: None,
                    id: id.clone(),
                    name: name.trim().into(),
                    start,
                    end,
                    notes,
                    speed: self.state.speed,
                    settings,
                };
                let passages = self
                    .preferences
                    .passages
                    .entry(f.content_id.clone())
                    .or_default();
                if let Some(old) = passages.iter_mut().find(|p| p.id == id) {
                    *old = preset;
                } else {
                    if passages.len() >= 200 {
                        return Err("每首曲目最多保存 200 个练习段".into());
                    }
                    passages.push(preset);
                }
            }
            Command::RemovePassage { id } => {
                let f = self.file.as_ref().ok_or("请先选择曲目")?;
                if let Some(passages) = self.preferences.passages.get_mut(&f.content_id) {
                    passages.retain(|p| p.id != id);
                }
            }
            Command::OpenPassage { id } => {
                let f = self.file.as_ref().ok_or("请先选择曲目")?;
                let preset = self
                    .preferences
                    .passages
                    .get(&f.content_id)
                    .and_then(|ps| ps.iter().find(|p| p.id == id))
                    .cloned()
                    .ok_or("该练习段不属于当前曲目")?;
                if preset
                    .grid
                    .as_ref()
                    .is_some_and(|g| *g != self.grid_signature())
                {
                    return Err("小节网格已改变，请重新确认这个练习段的范围并保存".into());
                }
                if let Some(range)=&preset.score_range { self.validate_score_range(range)?; }
                if let Some(range)=&preset.precise_range { self.validate_tick_range(range)?; }
                self.panic();
                self.state.status = "paused".into();
                self.config.apply_practice_setup(&SongPracticeSetup {
                    tracks: preset.settings.tracks.clone(),
                    ..Default::default()
                });
                for t in &mut self.config.tracks {
                    if let Some(part) = preset.settings.parts.get(&t.track_id) {
                        t.practice_part = *part;
                    }
                }
                self.state.mode = preset.settings.mode.unwrap_or_else(|| "wait".into());
                self.state.wait = self.state.mode == "wait";
                self.state.hands = preset.settings.hands;
                self.state.count_in = preset.settings.count_in;
                self.state.metronome = preset.settings.metronome;
                self.state.rounds = preset.settings.rounds;
                self.state.adaptive = preset.settings.adaptive;
                self.state.speed = preset.speed;
                if let Some(range)=&preset.score_range {return self.apply_score_range(range);}
                if let Some(range)=&preset.precise_range {return self.apply_tick_range(range);}
                return self.command(Command::MeasureLoop {
                    start: preset.start,
                    end: preset.end,
                    enabled: true,
                });
            }
            Command::ExercisePresets => {
                return Ok(serde_json::json!({"presets":self.preferences.exercise_presets}));
            }
            Command::SaveExercisePreset { name, spec } => {
                if name.trim().is_empty() || name.chars().count() > 80 {
                    return Err("练习方案名称需要 1～80 个字".into());
                }
                let plan = ExercisePlan::generate(spec, &KeyboardRange::new(21..=108))
                    .map_err(|e| e.to_string())?;
                let id = plan.practice_id();
                if self.preferences.exercise_presets.len() >= 200
                    && !self.preferences.exercise_presets.iter().any(|p| p.id == id)
                {
                    return Err("最多保存 200 个练习方案".into());
                }
                self.preferences.exercise_presets.retain(|p| p.id != id);
                self.preferences
                    .exercise_presets
                    .push(preferences::ExercisePreset {
                        id,
                        name: name.trim().into(),
                        spec,
                    });
            }
            Command::RemoveExercisePreset { id } => {
                self.preferences.exercise_presets.retain(|p| p.id != id)
            }
            Command::EditPaperAnnotation{content_id,book,id,draft,expected}=>return score_attachments::edit_annotation(&self.data,&content_id,&book,id.as_deref(),draft,expected),
            Command::EditPaperInk{content_id,book,asset_id,page,expected,strokes}=>return score_attachments::edit_ink(&self.data,&content_id,&book,&asset_id,page,expected,strokes),
            Command::PaperPracticeContext=>return Ok(serde_json::json!({"contentId":self.file.as_ref().map(|f|&f.content_id),"grid":self.grid_signature(),"measures":self.file.as_ref().map_or(0,|f|f.musical_time.measures.len()),"measure":self.snapshot().measure})),
            Command::PreviewPaperPractice{content_id,book,measure}=>return self.preview_paper_practice(&content_id,&book,measure),
            Command::SavePaperPractice{request}=>return self.save_paper_practice(request),
            Command::SeekPaperMeasure{content_id,id,measure}=>{
                self.file.as_ref().filter(|f|f.content_id==content_id).ok_or("曲目已切换，请重新打开谱页")?;
                let papers=score_attachments::list(&self.data,&content_id)?;
                let map=papers["attachments"].as_array().and_then(|v|v.iter().find(|a|a["id"].as_str()==Some(&id))).and_then(|a|a.get("mapping")).ok_or("该谱面没有小节对应")?;
                let mapping:score_attachments::Mapping=serde_json::from_value(map.clone()).map_err(|_|"谱页小节对应不可用")?;
                if mapping.grid!=self.grid_signature()||!mapping.anchors.iter().any(|a|a.measure==measure){return Err("小节对应已改变，请重新确认谱页位置".into());}
                return self.command(Command::SeekMeasure{measure});
            }
            Command::SetPaperMapping{content_id,id,mapping}=>{
                let file=self.file.as_ref().filter(|f|f.content_id==content_id).ok_or("曲目已切换，请重新打开谱页小节对应")?;
                if let Some(m)=&mapping{if m.grid!=self.grid_signature()||m.anchors.iter().any(|a|a.measure>file.musical_time.measures.len()){return Err("小节网格或范围已改变，请重新确认对应".into());}}
                return score_attachments::set_mapping(&self.data,&content_id,&id,mapping);
            }
            Command::LibraryRepairs{..}
            | Command::RelinkLibrary{..}
            | Command::SetLibraryPrimary{..}
            | Command::ScoreAttachments{..}
            | Command::PaperSourceUpdates| Command::LinkPaperSource{..}|Command::InspectPaperSources{..}|Command::PreviewPaperSource{..}|Command::ApplyPaperSource{..}
            | Command::PreviewPaperTransfer{..}|Command::ApplyPaperTransfer{..}
            | Command::AddScoreAttachment{..}
            | Command::UpdateScoreAttachment{..}
            | Command::RestoreLibraryPath{..}
            | Command::IgnoreLibraryPath{..}
            | Command::LibraryMonitorStatus
            | Command::SetLibraryMonitor { .. }
            | Command::LibraryWorkspace
            | Command::SaveSongLearning{..}
            | Command::InspectSong { .. }
            | Command::InspectSongs { .. }
            | Command::LibraryGroup { .. }
            | Command::RemoveLibraryGroup { .. }
            | Command::AssignLibrary { .. }
            | Command::UnassignLibrary { .. }
            | Command::LibraryMetadataHistory { .. } | Command::PreviewMetadataRestore { .. } | Command::RestoreLibraryMetadata { .. }
            | Command::PreviewLibraryMetadata { .. }
            | Command::ReadLibraryMetadata { .. }
            | Command::SaveLibraryMetadata { .. }
            | Command::UpdateSongMetadata { .. } => unreachable!(),
            Command::LibraryFolders
            | Command::RefreshLibrary
            | Command::RemoveLibraryFolder { .. } => return Err("曲库文件夹管理由应用处理".into()),
            Command::Metadata => {
                let f = self.file.as_ref().ok_or("请先选择曲目")?;
                let metadata = neothesia_core::library::load_song_metadata(
                    &self.annotation_path(f)?,
                    &f.content_id,
                )
                .unwrap_or_default();
                return serde_json::to_value(metadata).map_err(|e| e.to_string());
            }
            Command::SetMetadata { value } => {
                let f = self.file.as_ref().ok_or("请先选择曲目")?;
                let path = self.annotation_path(f)?;
                neothesia_core::library::save_song_metadata(&path, &f.content_id, value.clone())
                    .map_err(|e| e.to_string())?;
                if let Some(title) = value.title.filter(|s| !s.trim().is_empty()) {
                    self.title = title;
                }
                self.persist()?;
            }
            Command::VstEditor => {
                self.vst
                    .as_ref()
                    .ok_or("请先选择 VST3 音源")?
                    .open_editor()
                    .map_err(|e| e.to_string())?;
            }
            Command::LibraryScores{..}=>return Err("请从曲库查看谱面资料".into()),
            Command::CheckLibraryScoreImport{..}=>return Err("请从独立谱面核对调用".into()),
            Command::ManageLibraryScore{path,content_id,kind,id,action,name}=>return self.manage_library_score(path,content_id,kind,id,action,name),
            Command::OpenLibraryScore{path,content_id,kind,id}=>return self.open_library_score(path,content_id,kind,id),
            Command::ScoreSources{..}|Command::PreviewScoreSource{..}=>return Err("请从独立线程检查原谱".into()),
            Command::LinkScoreSource{midi_path,content_id,version_id,path,default_bpm}=>return self.link_score_source(midi_path,content_id,version_id,path,default_bpm),
            Command::AcknowledgeScoreSource{id,fingerprint}=>return score_sources::acknowledge(&self.data,&id,&fingerprint),
            Command::UnlinkScoreSource{id,path}=>return score_sources::unlink(&self.data,&id,&path),
            Command::ApplyScoreSource{id,fingerprint,mode}=>return self.apply_score_source(id,fingerprint,mode),
            Command::ImportScoreOriginal{path,default_bpm}=>return self.import_score_original(path,default_bpm),
            Command::ScoreVersions => {
                self.remember_active_score()?;
                let f = self.file.as_ref().ok_or("请先选择曲目")?;
                let midi = self.annotation_path(f)?;
                let current = neothesia_core::library::load_song_sidecar(&midi, &f.content_id)
                    .ok()
                    .and_then(|s| s.score)
                    .map(|s| neothesia_core::library::resolve_score_path(&midi, &s));
                let rows:Vec<_>=self.preferences.score_versions.get(&f.content_id).into_iter().flatten().map(|v|serde_json::json!({"id":v.id,"name":v.name,"path":v.path,"active":current.as_ref()==Some(&v.path),"available":v.path.is_file()})).collect();
                return Ok(serde_json::json!({"versions":rows}));
            }
            Command::ActivateScore { id, content_id } => {
                self.score_content_guard(&content_id)?;
                let v = self
                    .preferences
                    .score_versions
                    .get(&content_id)
                    .and_then(|rows| rows.iter().find(|v| v.id == id))
                    .cloned()
                    .ok_or("谱面版本不存在")?;
                let f = self.file.as_ref().unwrap();
                let midi = self.annotation_path(f)?;
                let assoc = neothesia_core::library::ScoreAssociation {
                    path: v.path.clone(),
                    content_id: v.content_id,
                };
                if !neothesia_core::library::verify_score_association(&midi, &assoc)
                    .map_err(|e| e.to_string())?
                {
                    return Err("这份谱面文件已更改，请重新导入确认".into());
                }
                let score = notation::ScoreAsset::new(
                    std::fs::read(&v.path).map_err(|e| e.to_string())?,
                    v.name,
                    f,
                )?;
                neothesia_core::library::save_score_association(&midi, &content_id, &v.path)
                    .map_err(|e| e.to_string())?;
                self.panic();
                self.state.status = "paused".into();
                self.notation = Some(score);
                self.score_revision = self.score_revision.wrapping_add(1);
            }
            Command::DetachScore { content_id } => {
                self.score_content_guard(&content_id)?;
                self.remember_active_score()?;
                let f = self.file.as_ref().unwrap();
                neothesia_core::library::clear_score_association(
                    &self.annotation_path(f)?,
                    &content_id,
                )
                .map_err(|e| e.to_string())?;
                self.notation = None;
                self.score_revision = self.score_revision.wrapping_add(1);
            }
            Command::RemoveScoreVersion { id, content_id } => {
                self.score_content_guard(&content_id)?;
                let f = self.file.as_ref().unwrap();
                let midi = self.annotation_path(f)?;
                let current = neothesia_core::library::load_song_sidecar(&midi, &content_id)
                    .ok()
                    .and_then(|s| s.score)
                    .map(|s| neothesia_core::library::resolve_score_path(&midi, &s));
                let rows = self
                    .preferences
                    .score_versions
                    .get_mut(&content_id)
                    .ok_or("没有谱面版本")?;
                let v = rows.iter().find(|v| v.id == id).ok_or("谱面版本不存在")?;
                if current.as_ref() == Some(&v.path) {
                    return Err("请先解除当前谱面的关联，再移除版本".into());
                }
                rows.retain(|v| v.id != id);
                self.preferences.save(&self.preferences_path)?;
            }
            Command::RenameScoreVersion {
                id,
                content_id,
                name,
            } => {
                self.score_content_guard(&content_id)?;
                let name = name.trim();
                if name.is_empty() || name.chars().count() > 120 {
                    return Err("请输入 1 至 120 字的谱面名称".into());
                }
                let v = self
                    .preferences
                    .score_versions
                    .get_mut(&content_id)
                    .and_then(|rows| rows.iter_mut().find(|v| v.id == id))
                    .ok_or("谱面版本不存在")?;
                v.name = name.into();
                self.preferences.save(&self.preferences_path)?;
            }
            Command::Score => {
                let mut payload = self
                    .notation
                    .as_ref()
                    .map_or(serde_json::Value::Null, |s| s.payload(self.file.as_ref()));
                if let Some(f) = &self.file {
                    if !payload.is_null() {
                        payload["contentId"] = serde_json::json!(f.content_id);
                        payload["scoreRevision"] = serde_json::json!(self.score_revision)
                    }
                }
                return Ok(payload);
            }
            Command::PairScore {
                bytes,
                name,
                content_id,
            } => {
                self.score_content_guard(&content_id)?;
                self.remember_active_score()?;
                if self
                    .preferences
                    .score_versions
                    .get(&content_id)
                    .is_some_and(|v| v.len() >= 100 && !v.iter().any(|v|v.name==name && v.content_id==blake3::hash(&bytes).to_hex().as_str()))
                {
                    return Err("这首曲目已有 100 份谱面，请先移除不需要的版本".into());
                }
                self.panic();
                self.state.status = "paused".into();
                let f = self.file.as_ref().ok_or("请先打开 MIDI")?;
                if f.content_id != content_id {
                    return Err("曲目已切换，请重新配对乐谱".into());
                }
                let score = notation::ScoreAsset::new(bytes, name, f)?;
                let path = self.store_score_version_file(&score,&content_id)?;
                let f = self.file.as_ref().unwrap();
                {
                    let midi = self.annotation_path(f)?;
                    neothesia_core::library::save_score_association(&midi, &f.content_id, &path)
                        .map_err(|e| e.to_string())?;
                }
                self.notation = Some(score);
                self.score_revision = self.score_revision.wrapping_add(1);
                self.remember_active_score()?;
                return self.command(Command::Score);
            }
            Command::ExportAnnotatedScore {content_id,score_revision,options,download,fingerprint} => {
                if self.file.as_ref().is_none_or(|f|f.content_id!=content_id) || self.score_revision!=score_revision {return Err("曲目或谱面版本已切换，请重新打开导出窗口".into());}
                let mut actions=Vec::new();
                if options.substitutions {
                    let saved=self.saved_actions()?;let actual=self.command(Command::CurrentSong)?;
                    let rates:std::collections::BTreeSet<_>=saved.iter().map(|a|a.rate_milli).collect();
                    let validity:std::collections::BTreeMap<_,_>=rates.into_iter().map(|rate|(rate,self.check_action_groups(&actual,&saved,rate as f64/1000.))).collect();
                    actions=saved.iter().enumerate().map(|(i,a)|(a.clone(),validity[&a.rate_milli][i])).collect();
                }
                let file=self.file.as_ref().ok_or("请先打开曲目")?;
                let asset=self.notation.as_ref().ok_or("当前版本没有 MusicXML 乐谱")?;
                return score_export::export(asset,file,&self.fingers,&self.note_hands,&actions,&options,download,fingerprint.as_deref());
            }
            Command::ScoreMap {
                score_revision,
                midi,
                notes,
                content_id,
            } => {
                if score_revision.is_some_and(|r| r != self.score_revision) {
                    return Err("谱面版本已切换，请重新加载乐谱".into());
                }
                let f = self.file.as_ref().ok_or("请先打开 MIDI")?;
                if f.content_id != content_id {
                    return Err("曲目已切换，乐谱映射已取消".into());
                }
                return self
                    .notation
                    .as_ref()
                    .ok_or("尚未配对乐谱")?
                    .map(f, &midi, notes);
            }
            Command::UpdateLibraryPaper{..}=>return Err("谱页整理应由独立调用线程执行".into()),
            Command::ExportPracticeBackup{..}|Command::ImportStagedPracticeBackup{..}=>return Err("备份文件应由独立调用线程处理".into()),
            Command::PracticeBackupSnapshot{selection}=>return self.practice_backup_snapshot(&selection),
            Command::InspectPracticeBackup{backup}=>return Ok(self.inspect_practice_backup(&backup)),
            Command::ApplyPracticeBackup{backup,selection,policy}=>return self.apply_practice_backup(backup,selection,policy),
            Command::AddLibraryScore{path,content_id,name,bytes,kind,target,..}=>{if kind!="notation"{return Err("谱页操作应由独立调用线程执行".into());}return self.add_library_notation(path,content_id,name,bytes,target);}
            Command::EditFingersFor{content_id,edits}=>{
                let file=self.file.as_ref().ok_or("请先选择曲目")?;
                if file.content_id!=content_id{return Err("曲目已切换，指法修改未保存".into());}
                if edits.is_empty()||edits.len()>4096{return Err("请选择 1 至 4096 个音符".into());}
                let mut hints=self.saved_hints()?;
                let mut identities=std::collections::HashSet::new();
                for edit in edits{
                  if !identities.insert((edit.track_id,edit.note_index)){return Err("同一音符包含重复修改，未保存".into());}
                  if edit.finger.is_some_and(|f|!(1..=5).contains(&f))||!file.tracks.iter().any(|t|t.track_id==edit.track_id&&edit.note_index<t.notes.len()){return Err("指法包含无效音符或编号，未保存任何修改".into());}
                  hints.retain(|h|h.track_id!=edit.track_id||h.note_index!=edit.note_index);
                  if let Some(finger)=edit.finger{hints.push(neothesia_core::library::FingerHint{track_id:edit.track_id,note_index:edit.note_index,finger});}
                }
                self.write_hints(hints,true)?;
            }
            Command::UndoFingersFor{content_id}=>{
                if self.file.as_ref().is_none_or(|f|f.content_id!=content_id){return Err("曲目已切换，撤销未执行".into());}
                let previous=self.finger_undo.last().cloned().ok_or("没有可撤销的指法修改")?;
                self.write_schedule(previous.0,previous.1,false)?;self.finger_undo.pop();
            }
            Command::Finger {
                track,
                index,
                finger,
            } => {
                if finger.is_some_and(|f| !(1..=5).contains(&f)) {
                    return Err("指法应为 1 到 5".into());
                }
                let f = self.file.as_ref().ok_or("请先选择曲目")?;
                f.tracks
                    .iter()
                    .find(|t| t.track_id == track)
                    .and_then(|t| t.notes.get(index))
                    .ok_or("音符不存在")?;
                let mut hints = self.saved_hints()?;
                hints.retain(|h| h.track_id != track || h.note_index != index);
                if let Some(finger) = finger {
                    hints.push(neothesia_core::library::FingerHint {
                        track_id: track,
                        note_index: index,
                        finger,
                    });
                }
                self.write_hints(hints, true)?;
            }
            Command::StartFingerDemo{request,fingerprint,rate}=>return self.start_finger_demo(request,fingerprint,rate),
            Command::HeldFingerDemoContext{content_id,track,index,rate}=>return serde_json::to_value(self.prepare_held_demo(content_id,track,index,rate)?).map_err(|e|e.to_string()),
            Command::StartHeldFingerDemo{content_id,track,index,baseline,rate}=>return self.start_held_demo(content_id,track,index,baseline,rate),
            Command::FingerDemoState=>return Ok(self.finger_demo_state()),
            Command::StopFingerDemo{id}=>{if self.finger_demo.as_ref().is_some_and(|demo|id.as_ref().is_none_or(|id|&demo.id==id)){self.panic();}return Ok(self.finger_demo_state());},
            Command::SuggestFingerPlans{..}=>return Err("指法搜索应由独立调用线程执行".into()),
            Command::AcceptHeldFingerPlan{request,fingerprint,edits,actions}=>return self.accept_held_plan(request,fingerprint,edits,actions),
            Command::ClearHeldFingerActions{content_id}=>{if self.file.as_ref().is_none_or(|f|f.content_id!=content_id){return Err("曲目已切换，持音换指未清除".into());}self.write_schedule(self.saved_hints()?,vec![],true)?;},
            Command::ReviewHeldActionEdit{request}=>return self.review_action_edit(request),
            Command::SaveHeldActionEdit{request,review}=>return self.save_action_edit(request,review),
            Command::ClearNoteHeldFingerActions{content_id,track,index}=>{
              let file=self.file.as_ref().ok_or("请先选择曲目")?;
              if file.content_id!=content_id{return Err("曲目已切换，换指动作未清除".into());}
              if !file.tracks.iter().any(|t|t.track_id==track&&index<t.notes.len()){return Err("持音音符不存在，未清除动作".into());}
              let mut actions=self.saved_actions()?;let before=actions.len();actions.retain(|a|a.track_id!=track||a.note_index!=index);
              if actions.len()!=before{self.write_schedule(self.saved_hints()?,actions,true)?;}
            },
            Command::SuggestHeldFingerPlans{..}=>return Err("持音换指搜索应由独立调用线程执行".into()),
            Command::FingerPlanContext{request}=>{let file=self.file.as_ref().ok_or("请先选择曲目")?;return serde_json::to_value(fingerings::prepare(file,&self.config,&self.fingers,&self.note_hands,request)?).map_err(|e|e.to_string());}
            Command::AcceptFingerPlan{request,fingerprint,edits}=>{
              let file=self.file.as_ref().ok_or("请先选择曲目")?;let prepared=fingerings::prepare(file,&self.config,&self.fingers,&self.note_hands,request.clone())?;
              if prepared.fingerprint!=fingerprint{return Err("音符、分手或已有指法已变化，请重新生成方案".into());}
              let review=fingerings::review_acceptance(&prepared,&edits)?;
              if !review["valid"].as_bool().unwrap_or(false){return Err("勾选建议与保留指法存在同时按键冲突，请先审阅保存结果、调整选择或固定已有指法重新推荐".into());}
              if edits.iter().any(|e|!prepared.positions.iter().any(|n|n.target&&n.track==e.track_id&&n.index==e.note_index)||e.finger.is_none()){return Err("方案包含选段之外的修改，未保存".into());}
              let edited_tracks:std::collections::HashSet<_>=edits.iter().map(|e|e.track_id).collect();
              let result=self.command(Command::EditFingersFor{content_id:request.content_id.clone(),edits})?;
              let profiles=self.preferences.hand_span_profiles.entry(request.content_id).or_default();
              for track in edited_tracks {profiles.insert(format!("{}:{}",track,hands::label(request.hand)),request.profile);}
              self.preferences.save(&self.preferences_path)?;return Ok(result);
            }
            Command::FingerDemoControl{id,action,position}=>return self.finger_demo_control(&id,&action,position),
            Command::StartPairFingerDemo{parts,proof,acknowledged,rate}=>return self.start_pair_demo(&parts,&proof,acknowledged,rate),
            Command::SavePairFingerPractice{parts,proof,acknowledged,ranges}=>return self.save_pair_practice(&parts,&proof,acknowledged,ranges),
            Command::FingerPair{parts,proof,acknowledged}=>return self.finger_pair(&parts,proof.as_deref(),acknowledged),
            Command::ReviewFingerAcceptance{request,fingerprint,edits}=>{
              let file=self.file.as_ref().ok_or("请先选择曲目")?;
              let prepared=fingerings::prepare(file,&self.config,&self.fingers,&self.note_hands,request)?;
              if prepared.fingerprint!=fingerprint{return Err("音符、分手或已有指法已变化，请重新生成方案".into());}
              return fingerings::review_acceptance(&prepared,&edits);
            }
            Command::SaveFingerPractice{request,fingerprint,ranges}=>return self.save_finger_practice(request,fingerprint,ranges),
            Command::SuggestFingers {
                hand,
                track,
                index,
                profile,
                range,
            } => {
                let file = self.file.as_ref().ok_or("请先选择曲目")?;
                let proposals = fingerings::propose(
                    file,
                    &self.config,
                    &self.fingers,
                    &self.note_hands,
                    hand,
                    track,
                    index,
                    profile,
                    range,
                )?;
                let id = file.content_id.clone();
                self.preferences
                    .fingering_profiles
                    .entry(id.clone())
                    .or_default()
                    .insert(track, profile);
                let part = hand
                    .unwrap_or_else(|| hands::part(&self.config, &self.note_hands, track, index));
                self.preferences
                    .hand_span_profiles
                    .entry(id.clone())
                    .or_default()
                    .insert(format!("{track}:{}", hands::label(part)), profile);
                self.preferences.save(&self.preferences_path)?;
                return Ok(
                    serde_json::json!({"contentId":id,"proposals":proposals,"undoAvailable":!self.finger_undo.is_empty()}),
                );
            }
            Command::ApplyFingers { content_id, hints } => {
                if self
                    .file
                    .as_ref()
                    .is_none_or(|f| f.content_id != content_id)
                {
                    return Err("曲目已切换，请重新生成指法建议".into());
                }
                if hints.len() > 4096 {
                    return Err("一次最多接受 4096 个指法".into());
                }
                let mut merged = self.saved_hints()?;
                for h in hints {
                    merged.retain(|v| v.track_id != h.track_id || v.note_index != h.note_index);
                    merged.push(h);
                }
                self.write_hints(merged, true)?;
            }
            Command::UndoFingers => {
                let previous = self
                    .finger_undo
                    .last()
                    .cloned()
                    .ok_or("没有可撤销的指法修改")?;
                self.write_schedule(previous.0,previous.1, false)?;
                self.finger_undo.pop();
            }
            Command::Recording { active } => {
                self.panic();
                self.ensure_audio()?;
                if active {
                    self.matcher.reset();
                    self.performance_capture=Default::default();
                    self.recorder.start();
                    self.state.status = "freePlay".into();
                } else {
                    let smf = match self.recorder.stop() {
                        Ok(smf) => smf,
                        Err(error) => {
                            self.state.status = "ready".into();
                            return Err(error.to_string());
                        }
                    };
                    let directory = self.data.join("recordings");
                    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    let path = directory.join(format!("演奏-{timestamp}.mid"));
                    smf.save(&path).map_err(|e| e.to_string())?;
                    self.state.mode = "listen".into();
                    self.state.wait = false;
                    self.state.passage = None;
                    let result = self.command(Command::Load {
                        path,
                        title: "自由演奏录音".into(),
                    })?;
                    self.state.mode = "listen".into();
                    self.state.wait = false;
                    self.state.count_in = 0;
                    self.persist()?;
                    return Ok(result);
                }
            }
            Command::ExportMidi => {
                let f = self.file.as_ref().ok_or("请先打开曲目或录制演奏")?;
                let bytes = if let Some(path) = &f.source_path {
                    std::fs::read(path).map_err(|e| e.to_string())?
                } else {
                    let plan = ExercisePlan::generate(
                        self.exercise.ok_or("曲目无法导出")?,
                        &KeyboardRange::new(21..=108),
                    )
                    .map_err(|e| e.to_string())?;
                    let mut bytes = Vec::new();
                    plan.to_smf()?
                        .write_std(&mut bytes)
                        .map_err(|e| e.to_string())?;
                    bytes
                };
                return Ok(serde_json::json!({"bytes":bytes,"title":self.title}));
            }
            Command::Feedback => {
                let mut result = serde_json::to_value(
                    self.previous_summary
                        .clone()
                        .unwrap_or_else(|| self.matcher.summary()),
                )
                .map_err(|e| e.to_string())?;
                result["mode"] =
                    serde_json::json!(self.previous_mode.as_ref().unwrap_or(&self.state.mode));
                if let Some(history) = self
                    .file
                    .as_ref()
                    .and_then(|f| self.history.song(&f.content_id))
                {
                    let overview = if history.sessions.last().is_some_and(|s| {
                        s.scope
                            .as_ref()
                            .is_some_and(|scope| *scope != self.practice_scope())
                    }) {
                        history.overview(0, 0)
                    } else {
                        history.overview(20, 5)
                    };
                    result["trend"] = serde_json::json!({"attempts":overview.trend_attempts,"accuracyDelta":overview.accuracy_delta,"speedDelta":overview.speed_delta,"tempoDelta":overview.tempo_bpm_delta});
                }
                return Ok(result);
            }
            Command::Panic => {
                self.panic();
                self.state.status = "paused".into();
                self.state.count_remaining = 0.;
            }
            Command::Generate { spec } => {
                let plan = ExercisePlan::generate(spec, &KeyboardRange::new(21..=108))
                    .map_err(|e| e.to_string())?;
                let mut file = plan.to_midi_file()?;
                file.content_id = plan.practice_id();
                let title = format!(
                    "{}{} · {}",
                    [
                        "C", "C♯", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", "B"
                    ][spec.tonic as usize],
                    if matches!(
                        spec.tonality,
                        neothesia_core::exercise::ExerciseTonality::Major
                    ) {
                        "大调"
                    } else {
                        "小调"
                    },
                    match spec.pattern {
                        neothesia_core::exercise::ExercisePattern::Scale => "音阶",
                        neothesia_core::exercise::ExercisePattern::Arpeggio => "琶音",
                        _ => "主三和弦",
                    }
                );
                self.install(file, title, Some(plan))?;
                self.persist()?;
                return self.command(Command::CurrentSong);
            }
            Command::Mode { value } => {
                if !["wait", "flow", "listen", "recital", "memory"].contains(&value.as_str()) {
                    return Err("练习模式无效".into());
                }
                self.state.wait = value == "wait";
                let recital = is_recital_mode(&value);
                self.state.mode = value;
                if recital {
                    self.state.passage = None;
                    self.state.rounds = 1;
                    self.state.adaptive = false;
                }
                self.reset(if recital { 0. } else { self.state.position });
            }
            Command::Track {
                id,
                mode,
                part,
                visible,
            } => {
                let player = match mode.as_str() {
                    "human" => PlayerConfig::Human,
                    "auto" => PlayerConfig::Auto,
                    "mute" => PlayerConfig::Mute,
                    _ => return Err("音轨模式无效".into()),
                };
                let part = match part.as_str() {
                    "left" => PracticePart::LeftHand,
                    "right" => PracticePart::RightHand,
                    "other" => PracticePart::Other,
                    _ => return Err("声部无效".into()),
                };
                let track = self
                    .config
                    .tracks
                    .iter_mut()
                    .find(|t| t.track_id == id)
                    .ok_or("找不到音轨")?;
                track.player = player;
                track.practice_part = part;
                track.visible = visible;
                self.state.hands = "custom".into();
                self.previous_summary = None;
                self.previous_mode = None;
                self.coach = AdaptiveTempoCoach::default();
                self.reset(self.state.position);
            }
            Command::MeasureLoop {
                start,
                end,
                enabled,
            } => {
                let f = self.file.as_ref().ok_or("请先选择曲目")?;
                let first = f
                    .musical_time
                    .measures
                    .get(start.checked_sub(1).ok_or("小节从 1 开始")?)
                    .ok_or("起始小节超出曲目")?;
                let last = f
                    .musical_time
                    .measures
                    .get(end.checked_sub(1).ok_or("小节从 1 开始")?)
                    .ok_or("结束小节超出曲目")?;
                if end < start {
                    return Err("结束小节不能早于起始小节".into());
                }
                return self.command(Command::Loop {
                    start: f
                        .tempo_track
                        .pulses_to_duration(first.start_tick)
                        .as_secs_f64(),
                    end: f
                        .tempo_track
                        .pulses_to_duration(last.end_tick)
                        .as_secs_f64()
                        .min(self.state.duration),
                    enabled,
                });
            }
            Command::SeekMeasure { measure } => {
                let f = self.file.as_ref().ok_or("请先选择曲目")?;
                let m = f
                    .musical_time
                    .measures
                    .get(measure.checked_sub(1).ok_or("小节从 1 开始")?)
                    .ok_or("小节超出曲目")?;
                return self.command(Command::Seek {
                    position: f.tempo_track.pulses_to_duration(m.start_tick).as_secs_f64(),
                });
            }
            Command::SeekBeat { measure, beat } => {
                let file = self.file.as_ref().ok_or("请先选择曲目")?;
                let bar = file
                    .musical_time
                    .measures
                    .get(measure.checked_sub(1).ok_or("小节从 1 开始")?)
                    .ok_or("小节超出曲目")?;
                let group = if bar.denominator == 8 && bar.numerator >= 6 && bar.numerator % 3 == 0
                {
                    3.
                } else {
                    1.
                };
                let length =
                    f64::from(file.musical_time.ppq) * 4. / f64::from(bar.denominator) * group;
                let tick = bar.start_tick as f64 + (beat - 1.) * length;
                if !beat.is_finite() || beat < 1. || tick >= bar.end_tick as f64 {
                    return Err("拍位置超出当前小节；弱起只包含实际拍数".into());
                }
                let position = file
                    .tempo_track
                    .pulses_to_duration(tick.round() as u64)
                    .as_secs_f64();
                return self.command(Command::Seek { position });
            }
            Command::StepBeat { direction } => {
                if direction != 1 && direction != -1 {
                    return Err("逐拍方向无效".into());
                }
                let file = self.file.as_ref().ok_or("请先选择曲目")?;
                let mut ticks = Vec::new();
                for bar in file.musical_time.measures.iter() {
                    let group =
                        if bar.denominator == 8 && bar.numerator >= 6 && bar.numerator % 3 == 0 {
                            3
                        } else {
                            1
                        };
                    for b in (0..bar.numerator).step_by(group) {
                        let tick = bar.start_tick
                            + u64::from(b) * u64::from(file.musical_time.ppq) * 4
                                / u64::from(bar.denominator);
                        if tick >= bar.end_tick {
                            break;
                        }
                        let time = file.tempo_track.pulses_to_duration(tick).as_secs_f64();
                        if time < self.state.duration
                            && self
                                .state
                                .passage
                                .as_ref()
                                .is_none_or(|p| time >= p.start && time < p.end)
                        {
                            ticks.push(time);
                        }
                    }
                }
                let position = if direction > 0 {
                    ticks
                        .into_iter()
                        .find(|t| *t > self.state.position + 0.000001)
                } else {
                    ticks
                        .into_iter()
                        .rev()
                        .find(|t| *t < self.state.position - 0.000001)
                }
                .ok_or("已到当前范围的边界")?;
                return self.command(Command::Seek { position });
            }
            Command::Rounds { count } => {
                if count > 100 {
                    return Err("循环遍数不能超过 100".into());
                }
                self.state.rounds = count;
            }
            Command::Calibration => {
                use neothesia_core::practice::{
                    TimingCalibrationStatus as C, timing_calibration_status,
                };
                if !is_timed_mode(&self.state.mode)
                    && !self.previous_mode.as_deref().is_some_and(is_timed_mode)
                {
                    return Ok(
                        serde_json::json!({"message":"请先在连续模式下按节拍器演奏至少 24 个音符","suggested":null}),
                    );
                }
                let timing = self
                    .previous_summary
                    .as_ref()
                    .map_or_else(|| self.matcher.summary().timing, |s| s.timing);
                let (message, suggested) =
                    match timing_calibration_status(timing, self.state.latency) {
                        C::Suggested(s) => (
                            format!("参考偏移稳定，建议补偿 {} 毫秒", s.suggested_ms),
                            Some(s.suggested_ms),
                        ),
                        C::InsufficientSamples { captured, required } => (
                            format!("有效样本 {captured}/{required}，继续按节拍演奏"),
                            None,
                        ),
                        C::Unstable { deviation_ms, .. } => (
                            format!("当前节奏波动 {deviation_ms} 毫秒，暂不能判断设备延迟"),
                            None,
                        ),
                        C::Centered => ("当前偏移在正常范围内".into(), None),
                        C::AtLimit => ("补偿已达到范围上限".into(), None),
                    };
                return Ok(serde_json::json!({"message":message,"suggested":suggested}));
            }
            Command::CountIn { bars } => {
                if bars > 4 {
                    return Err("预备拍应为 0 到 4 小节".into());
                }
                self.state.count_in = bars;
            }
            Command::Metronome { enabled } => self.state.metronome = enabled,
            Command::Latency { milliseconds } => {
                if milliseconds.abs() > 250 {
                    return Err("延迟补偿应为 -250 到 250 毫秒".into());
                }
                self.state.latency = milliseconds;
                self.preferences.calibrations.insert(
                    format!(
                        "{}|{}",
                        self.state.input.as_deref().unwrap_or("computer"),
                        self.state.output
                    ),
                    milliseconds,
                );
            }
            Command::Adaptive { enabled } => self.state.adaptive = enabled,
            Command::Load { path, title } => {
                let file = MidiFile::new(&path)?;
                self.install(file, title, None)?;
                self.persist()?;
                return self.command(Command::CurrentSong);
            }
            Command::Play => {
                if self.recorder.is_recording() {
                    return Err("请先停止录音，再开始曲目练习".into());
                }
                if self.state.mode != "listen"
                    && !self
                        .notes
                        .iter()
                        .any(|n| self.note_player(n.track_id, n.index) == PlayerConfig::Human)
                {
                    return Err("请设置至少一条自己弹的音轨，或切换到聆听模式".into());
                }
                if self.file.is_none() {
                    return Err("请先选择曲目".into());
                }
                self.ensure_audio()?;
                if is_recital_mode(&self.state.mode) && self.state.status == "ready" {
                    self.state.passage = None;
                    self.reset(0.);
                }
                if self.state.status == "finished" {
                    self.reset(self.state.passage.as_ref().map_or(0., |p| p.start));
                }
                if let Some(p) = &self.state.passage {
                    if self.state.position < p.start || self.state.position >= p.end {
                        self.reset(p.start);
                    }
                }
                self.begin_play();
            }
            Command::IndexLibrary { .. }
            | Command::IndexStatus
            | Command::PauseIndex
            | Command::ResumeIndex
            | Command::CancelIndex => return Err("请通过后台索引服务执行此命令".into()),
            Command::CurrentSong
            | Command::PassageCatalog{..}|Command::ComposePassages{..}|Command::Routines{..}|Command::SaveRoutine{..}|Command::RemoveRoutine{..}|Command::SaveRoutineItem{..}
            |Command::MoveRoutineItem{..}|Command::RemoveRoutineItem{..}|Command::OpenRoutineItem{..}
            |Command::PreviewTickRange{..}|Command::ApplyTickRange{..}|Command::SaveTickPassage{..}
            |Command::SaveRoutineSchedule{..}|Command::StopRoutine|Command::SkipRoutineItem{..}|Command::ResetRoutineItem{..}
            | Command::LadderPresets
            | Command::SaveLadder{..}
            | Command::RemoveLadder{..}
            | Command::StartLadder{..}
            | Command::StopLadder
            | Command::WeakPractice | Command::AcceptWeakPractice{..}
            | Command::History
            | Command::HistoryQuery { .. }
            | Command::AnnotateHistory { .. }
            | Command::PracticeTime { .. }
            | Command::HistoryPerformance{..}|Command::StartHistoryReplay{..}|Command::HistoryReplayControl{..}|Command::HistoryReplayLoop{..}|Command::HistoryProblemNotes{..}|Command::PreviewProblemCuts{..}|Command::PreviewMultiProblemPlan{..}|Command::SaveMultiProblemPlan{..}|Command::PreviewHistoryProblemPlan{..}|Command::SaveHistoryProblemPlan{..}|Command::HistoryReplayClips{..}|Command::SaveHistoryReplayClip{..}|Command::DeleteHistoryReplayClip{..}
            | Command::HistoryDetail { .. }
            | Command::HistoryOpen { .. }
                    | Command::HistoryPracticeRange { .. }
                    | Command::HistoryScoreReview { .. }
            | Command::SetLibraryCollection { .. }
            | Command::Collection
            | Command::MetadataTemplates | Command::UpdateMetadataTemplate { .. }
            | Command::FileLocations | Command::UpdateFileLocation { .. }
            | Command::OpenRecent { .. } => unreachable!(),
            Command::Favorite { enabled } => {
                let file = self.file.as_ref().ok_or("请先选择曲目")?;
                self.history
                    .set_favorite(
                        &file.content_id,
                        &self.title,
                        file.source_path.clone(),
                        enabled,
                    )
                    .map_err(|e| e.to_string())?;
            }
            Command::Queue { enabled } => {
                let file = self.file.as_ref().ok_or("请先选择曲目")?;
                self.history
                    .set_queued(
                        &file.content_id,
                        &self.title,
                        file.source_path.clone(),
                        enabled,
                    )
                    .map_err(|e| e.to_string())?;
            }
            Command::MoveQueue { direction } => {
                if direction != -1 && direction != 1 {
                    return Err("队列移动方向无效".into());
                }
                let file = self.file.as_ref().ok_or("请先选择曲目")?;
                self.history
                    .move_in_queue(&file.content_id, direction)
                    .map_err(|e| e.to_string())?;
            }
            Command::Volume { value } => {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err("音量参数无效".into());
                }
                self.state.volume = value;
                if let Some(audio) = &self.audio {
                    audio.set_volume(value);
                }
            }
            Command::Loop {
                start,
                end,
                enabled,
            } => {
                if self.file.is_none() {
                    return Err("请先选择曲目".into());
                }
                if enabled
                    && (!start.is_finite()
                        || !end.is_finite()
                        || start < 0.
                        || end > self.state.duration
                        || end - start < 0.1)
                {
                    return Err("分段终点应在起点之后，且不超过曲目时长".into());
                }
                self.state.passage = enabled.then_some(Passage { start, end });
                self.state.repetitions = 0;
                self.reset(if enabled { start } else { 0. });
            }
            Command::Pause => {
                self.panic();
                self.state.status = "paused".into();
            }
            Command::Restart => {
                if let Some(run)=&mut self.ladder {if run.active {run.progress.success_streak=0;run.progress.failure_streak=0;run.progress.last_result="重新练习当前速度".into();}}
                self.state.repetitions = 0;
                self.reset(self.state.passage.as_ref().map_or(0., |p| p.start));
            }
            Command::Seek { position } => {
                if !position.is_finite() || position < 0. || position > self.state.duration {
                    return Err("播放位置无效".into());
                }
                if let Some(p) = &self.state.passage {
                    if position < p.start || position >= p.end {
                        return Err("播放位置应在当前分段范围内；可先关闭分段循环".into());
                    }
                }
                self.reset(position);
            }
            Command::Speed { value } => {
                if !value.is_finite() || !(0.25..=2.).contains(&value) {
                    return Err("速度应在 25% 到 200% 之间".into());
                }
                self.state.speed = value;
            }
            Command::Wait { enabled } => {
                if self.state.wait != enabled {
                    self.state.wait = enabled;
                    self.state.mode = if enabled { "wait" } else { "listen" }.into();
                    self.reset(self.state.position);
                }
            }
            Command::Hands { value } => {
                self.previous_summary = None;
                self.previous_mode = None;
                self.coach = AdaptiveTempoCoach::default();
                self.set_hand_scope(&value)?;
                self.reset(self.state.passage.as_ref().map_or(0., |p| p.start));
            }
            Command::Save => {
                if is_recital_mode(&self.state.mode) && self.state.status != "finished" {
                    return Err("完整演奏结束后自动保存结果".into());
                }
                self.save()?;
            }
            Command::Note {
                pitch,
                active,
                velocity,
            } => {
                if pitch > 127 || velocity > 127 {
                    return Err("MIDI 音符参数无效".into());
                }
                self.ensure_audio()?;
                self.midi(&[if active { 144 } else { 128 }, pitch, velocity]);
            }
            Command::Input { name } => {
                self.connect_input(name.clone())?;
                self.preferences.input = name;
                self.preferences.input_configured = true;
            }
            Command::Output { name } => {
                self.panic();
                let chosen = name.clone();
                if let Some(name) = name.as_ref().filter(|n| n.starts_with("vst3:")) {
                    let path = PathBuf::from(name.trim_start_matches("vst3:"));
                    if !neothesia_core::vst3_backend::discover_standard_plugins().contains(&path) {
                        return Err("音源不在已发现的 VST3 插件中".into());
                    }
                    let backend = neothesia_core::vst3_backend::Vst3Backend::new()
                        .map_err(|e| e.to_string())?;
                    let connection = backend
                        .new_output_connection(&path)
                        .map_err(|e| e.to_string())?;
                    self.vst = Some(connection);
                    self.output_connection = None;
                    self.audio = None;
                    self.state.output = format!(
                        "VST3 · {}",
                        path.file_stem().unwrap_or_default().to_string_lossy()
                    );
                } else if let Some(name) = name {
                    let output =
                        midir::MidiOutput::new("Neothesia playback").map_err(|e| e.to_string())?;
                    let port = output
                        .ports()
                        .into_iter()
                        .find(|p| output.port_name(p).ok().as_deref() == Some(&name))
                        .ok_or("找不到所选 MIDI 输出")?;
                    let connection = output
                        .connect(&port, "Neothesia playback")
                        .map_err(|e| e.to_string())?;
                    self.vst = None;
                    self.output_connection = Some(connection);
                    self.audio = None;
                    self.state.output = name;
                } else {
                    self.vst = None;
                    self.output_connection = None;
                    self.state.output = "内置钢琴".into();
                    self.ensure_audio()?;
                }
                self.preferences.output = chosen;
                self.restore_latency();
                self.reset(self.state.position);
            }
        }
        if persist {
            self.persist()?;
        }
        Ok(serde_json::json!({ "ok": true }))
    }
    fn connect_input(&mut self, name: Option<String>) -> Result<(), String> {
        if let Some(name) = &name {
            let mut input =
                midir::MidiInput::new("Neothesia keyboard").map_err(|e| e.to_string())?;
            input.ignore(midir::Ignore::None);
            let port = input
                .ports()
                .into_iter()
                .find(|p| input.port_name(p).ok().as_deref() == Some(name))
                .ok_or("找不到所选 MIDI 键盘")?;
            let tx = self.input_tx.clone();
            let overflow = self.input_overflow.clone();
            let connection = input
                .connect(
                    &port,
                    "Neothesia keyboard",
                    move |_, bytes, _| {
                        if tx.try_send(bytes.to_vec()).is_err() {
                            overflow.store(true, Ordering::Release);
                        }
                    },
                    (),
                )
                .map_err(|e| e.to_string())?;
            self.panic();
            self.input_connection = Some(connection);
            self.state.input = Some(name.clone());
        } else {
            self.panic();
            self.input_connection = None;
            self.state.input = None;
        }
        self.restore_latency();
        Ok(())
    }
    fn restore_latency(&mut self) {
        self.state.latency = self
            .preferences
            .calibrations
            .get(&format!(
                "{}|{}",
                self.state.input.as_deref().unwrap_or("computer"),
                self.state.output
            ))
            .copied()
            .unwrap_or(0);
    }
    fn check_input(&mut self) {
        if let Some(name) = self.state.input.clone() {
            let present = devices()["inputs"]
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(&name)));
            if !present && self.input_connection.is_some() {
                self.panic();
                self.input_connection = None;
                self.state.status = "paused".into();
                self.state.error = Some("MIDI 键盘已断开，播放已暂停；重新连接后可继续".into());
            }
            if present && self.input_connection.is_none() {
                match self.connect_input(Some(name)) {
                    Ok(_) => self.state.error = None,
                    Err(e) => self.state.error = Some(e),
                }
            }
        }
    }
    fn midi(&mut self, bytes: &[u8]) {
        if self.finger_demo.is_some() && bytes.len() >= 3 && matches!(bytes[0]&240,128|144|176) {
            // Ignore clock/active-sensing traffic; real keys or controls take over the sound.
            self.panic();
        }
        self.state.midi_events += 1;
        if bytes.len() < 3 {
            let mut routed=bytes.to_vec();
            if !self.track_routing.lanes.is_empty() && !routed.is_empty() && routed[0]<240 {routed[0]&=240;}
            self.send(&routed);
            return;
        }
        let input_time=if self.state.latency>=0{self.session_clock.saturating_sub(Duration::from_millis(self.state.latency as u64))}else{self.session_clock+Duration::from_millis((-self.state.latency) as u64)};
        let kind = bytes[0] & 240;
        if let Ok(midi_file::midly::live::LiveEvent::Midi { channel, message }) =
            midi_file::midly::live::LiveEvent::parse(bytes)
        {
            self.recorder.push_event(channel.as_int(), message);
        }
        let note = bytes[1];
        let velocity = bytes[2];
        let performance_input=if self.state.status=="playing" && self.state.mode!="listen" && ((kind==144||kind==128)||kind==176&&note==64){self.performance_capture.input(self.session_clock.as_secs_f64(),input_time.as_secs_f64(),self.state.position,[kind,note,bytes[2]])}else{None};
        if (kind == 144 || kind == 128) && note <= 127 && velocity <= 127 {
            let active = kind == 144 && velocity != 0;
            self.state.pressed.retain(|p| *p != note);
            if active {
                self.state.pressed.push(note);
            }
            if self.state.status == "playing" && self.state.mode != "listen" {
                self.matcher.user_note_with_input(
                    input_time,
                    note,
                    active,
                    velocity,
                    performance_input,
                );
                self.state.saved = false;
            }
        }
        if kind == 176 && note == 64 {
            self.state.pedal = velocity;
        }
        if kind == 176 && note == 64 && self.state.status == "playing" {
            self.matcher.user_pedal(input_time, velocity);
        }
        let mut routed=bytes.to_vec();
        if !self.track_routing.lanes.is_empty() && routed[0]<240 {routed[0]&=240;}
        self.send(&routed);
    }
    fn tick(&mut self, delta: Duration) {
        self.tick_finger_demo(delta);
        if let Some(vst) = &self.vst {
            vst.service_editor();
        }
        if self.input_overflow.swap(false, Ordering::AcqRel) {
            self.panic();
            self.state.status = "paused".into();
            self.state.error = Some("MIDI 输入过载，已停止发声并暂停播放".into());
            while self.input_rx.try_recv().is_ok() {}
        }
        if self.state.status == "countIn" {
            let snap = self.snapshot();
            let denominator = self
                .file
                .as_ref()
                .and_then(|f| f.musical_time.measure_at(snap.tick))
                .map_or(4, |m| m.denominator);
            let compound = self
                .file
                .as_ref()
                .and_then(|f| f.musical_time.measure_at(snap.tick))
                .is_some_and(|m| m.denominator == 8 && m.numerator >= 6 && m.numerator % 3 == 0);
            let bpm = snap.bpm * self.state.speed * f64::from(denominator)
                / 4.
                / if compound { 3. } else { 1. };
            let old = self.state.count_remaining.ceil() as usize;
            self.state.count_remaining =
                (self.state.count_remaining - delta.as_secs_f64() * bpm / 60.).max(0.);
            if self.state.count_remaining.ceil() as usize != old {
                if let Some(a) = &self.audio {
                    a.click(false)
                }
            }
            if self.state.count_remaining == 0. {
                self.state.status = "playing".into();
                self.last_click = None;
            }
            return;
        }
        if self.state.status != "playing" {
            return;
        }
        let capture_clock=self.session_clock.as_secs_f64();
        let capture_position=self.state.position;
        self.session_clock += delta;
        self.matcher.tick(self.session_clock);
        if self.state.wait && !self.matcher.are_required_notes_pressed() {
            if self.state.mode!="listen" && !self.state.recording {if let Some(file)=&self.file{self.performance_capture.advance(capture_clock,self.session_clock.as_secs_f64(),capture_position,capture_position,self.state.speed,&file.measures,self.state.duration);}}
            return;
        }
        let end = self
            .state
            .passage
            .as_ref()
            .map_or(self.state.duration, |p| p.end);
        let mut position = (self.state.position + delta.as_secs_f64() * self.state.speed).min(end);
        if self.state.wait {
            while self.note_cursor < self.notes.len()
                && self.note_player(
                    self.notes[self.note_cursor].track_id,
                    self.notes[self.note_cursor].index,
                ) != PlayerConfig::Human
            {
                self.note_cursor += 1;
            }
            if let Some(note) = self.notes.get(self.note_cursor) {
                position = position.min(note.start.as_secs_f64().max(self.state.position));
            }
        }
        if self.state.mode!="listen" && !self.state.recording {if let Some(file)=&self.file{self.performance_capture.advance(capture_clock,self.session_clock.as_secs_f64(),capture_position,position,self.state.speed,&file.measures,self.state.duration);}}
        self.state.position = position;
        while self.event_cursor < self.events.len()
            && self.events[self.event_cursor].timestamp.as_secs_f64() <= position + 0.000001
            && (self.state.passage.is_none()
                || self.events[self.event_cursor].timestamp.as_secs_f64() < end)
        {
            let event = self.events[self.event_cursor].clone();
            self.event_cursor += 1;
            if self
                .config
                .tracks
                .iter()
                .any(|t| t.track_id == event.track_id && t.player == PlayerConfig::Mute)
            {
                continue;
            }
            if let midi_file::midly::MidiMessage::Controller { controller, value } = event.message {
                if controller.as_int() == 64 && self.selected(event.track_id) {
                    self.performance_capture.pedal(self.session_clock.as_secs_f64(),event.timestamp.as_secs_f64(),value.as_int());
                    self.matcher.score_pedal(self.session_clock, value.as_int());
                }
            }
            use midi_file::midly::MidiMessage as M;
            let bytes = match event.message {
                M::NoteOn { key, vel } => {
                    if self.state.mode != "listen"
                        && self
                            .event_notes
                            .get(&(
                                event.track_id,
                                event.channel,
                                key.as_int(),
                                event.timestamp,
                                true,
                            ))
                            .is_some_and(|&i| {
                                self.note_player(event.track_id, i) == PlayerConfig::Human
                            })
                    {
                        continue;
                    }
                    vec![144 | event.channel, key.as_int(), vel.as_int()]
                }
                M::NoteOff { key, vel } => {
                    if self.state.mode != "listen"
                        && self
                            .event_notes
                            .get(&(
                                event.track_id,
                                event.channel,
                                key.as_int(),
                                event.timestamp,
                                false,
                            ))
                            .is_some_and(|&i| {
                                self.note_player(event.track_id, i) == PlayerConfig::Human
                            })
                    {
                        continue;
                    }
                    vec![128 | event.channel, key.as_int(), vel.as_int()]
                }
                M::Controller { controller, value } => {
                    vec![176 | event.channel, controller.as_int(), value.as_int()]
                }
                M::ProgramChange { program } => vec![192 | event.channel, program.as_int()],
                M::PitchBend { bend } => {
                    let n = bend.0.as_int();
                    vec![224 | event.channel, (n & 127) as u8, (n >> 7) as u8]
                }
                _ => continue,
            };
            self.send_track(event.track_id,&bytes);
        }
        if self.state.mode != "listen" {
            while self.note_cursor < self.notes.len()
                && self.notes[self.note_cursor].start.as_secs_f64() <= position + 0.000001
                && (self.state.passage.is_none()
                    || self.notes[self.note_cursor].start.as_secs_f64() < end)
            {
                let note = self.notes[self.note_cursor].clone();
                self.note_cursor += 1;
                if self.note_player(note.track_id, note.index) == PlayerConfig::Human {
                    let reference=self.performance_capture.target(performance_archive::Reference{at:self.session_clock.as_secs_f64()-((position-note.start.as_secs_f64())/self.state.speed).max(0.),end:self.session_clock.as_secs_f64()-((position-note.start.as_secs_f64())/self.state.speed).max(0.)+note.duration.as_secs_f64()/self.state.speed,pitch:note.note,velocity:note.velocity,measure:self.file.as_ref().map(|f|f.measures.partition_point(|m|*m<=note.start)).unwrap_or(0),song_time:note.start.as_secs_f64(),track:note.track_id});
                    self.matcher.score_target_with_reference(
                        self.session_clock.saturating_sub(Duration::from_secs_f64(
                            ((position - note.start.as_secs_f64()) / self.state.speed).max(0.),
                        )),
                        PracticeTarget {
                            note: note.note,
                            velocity: note.velocity,
                            score_time: note.start,
                            duration: note.duration.div_f64(self.state.speed),
                            track_id: note.track_id,
                            measure: self
                                .file
                                .as_ref()
                                .map(|f| f.measures.partition_point(|m| *m <= note.start))
                                .unwrap_or(0),
                            part: hands::part(
                                &self.config,
                                &self.note_hands,
                                note.track_id,
                                note.index,
                            ),
                        },
                        true,
                        reference,
                    );
                }
            }
        }
        if is_timed_mode(&self.state.mode) {
            self.matcher.finalize_missed(self.session_clock);
        }
        if position >= end && is_timed_mode(&self.state.mode) {
            self.end_hold += delta.as_secs_f64();
            if self.end_hold < 0.5 {
                return;
            }
        }
        if position >= end && (!self.state.wait || self.matcher.are_required_notes_pressed()) {
            self.state.status = "finished".into();
            self.panic();
            self.matcher.finish();
            if self.state.mode != "listen" {
                if let Err(e) = self.save() {
                    self.state.error = Some(e);
                }
            }
            if self.state.error.is_none() && self.state.passage.is_some(){self.state.repetitions+=1;}
            let ladder_round=self.ladder.as_ref().is_some_and(|r|r.active);
            if self.state.error.is_none() && ladder_round {
                        let target_notes=self.history_context().target_notes;
                        if let Some(run)=&mut self.ladder {
                            run.progress.evaluate(&run.preset.plan,&self.matcher.summary(),target_notes);
                            self.state.speed=f64::from(run.progress.speed(&run.preset.plan))/100.;
                            if run.progress.completed||run.progress.limited{run.active=false;}
                        }
                        if let Err(e)=self.persist(){self.state.error=Some(e);return;}
            }
            if self.state.error.is_none() && self.routine_round_finished(){return;}
            if ladder_round && self.ladder.as_ref().is_some_and(|r|!r.active&&(r.progress.completed||r.progress.limited)){return;}
            if self.routine.is_some() && self.state.passage.is_none() && !is_recital_mode(&self.state.mode) && self.state.error.is_none(){
                self.reset(0.);self.begin_play();return;
            }
            if let Some(passage) = self.state.passage.clone() {
                if self.state.error.is_none() {
                    if self.state.rounds > 0 && self.state.repetitions >= self.state.rounds {
                        self.persist()
                            .unwrap_or_else(|e| self.state.error = Some(e));
                        return;
                    }
                    if self.state.adaptive && self.state.mode == "flow" {
                        let decision = self.coach.evaluate(
                            &self.matcher.summary(),
                            self.state.speed as f32,
                            AdaptiveTempoRules::default(),
                        );
                        self.state.speed = f64::from(decision.speed);
                    }
                    self.reset(passage.start);
                    self.begin_play();
                }
            }
        }
    }
    fn snapshot(&self) -> Snapshot {
        let mut state = self.state.clone();
        state.practice_ms=self.session_clock.as_millis().min(u128::from(u64::MAX)) as u64;
        state.ladder = self.ladder.clone();
        state.routine=self.routine_status();
        state.input_connected = self.input_connection.is_some();
        state.required = self.matcher.required_note_pitches();
        state.score = self.matcher.snapshot();
        state.summary = self.previous_summary.clone();
        state.recording = self.recorder.is_recording();
        state.record_duration = self.recorder.duration().as_secs_f64();
        if let Some(f) = &self.file {
            state.tick = f.tempo_track.seconds_to_pulses(state.position);
            state.bpm = f.tempo_track.bpm_at_seconds(state.position);
            let display_tick = if state.position >= state.duration {
                (state.tick - 0.000001).max(0.)
            } else {
                state.tick
            };
            if let Some(m) = f.musical_time.measure_at(display_tick) {
                state.measure = m.number;
                state.beat = 1.
                    + (display_tick - m.start_tick as f64)
                        / (f64::from(f.musical_time.ppq) * 4. / f64::from(m.denominator)
                            * if m.denominator == 8 && m.numerator >= 6 && m.numerator % 3 == 0 {
                                3.
                            } else {
                                1.
                            });
            }
        }
        state
    }
}

/// Open the actual output stream without emitting notes, for background
/// validation of the installed audio device and packaged SoundFont.
pub fn probe_audio(soundfont: &std::path::Path) -> Result<(), String> {
    let audio = audio::Audio::open(soundfont)?;
    std::thread::sleep(Duration::from_millis(100));
    audio.stop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn player() -> Player {
        static TEST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "neothesia-engine-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        let midi = path.join("exercise.mid");
        std::fs::write(&midi, include_bytes!("../assets/five-finger.mid")).unwrap();
        let mut player = Player::new(path, PathBuf::new(), true);
        player
            .command(Command::Load {
                path: midi,
                title: "五指热身".into(),
            })
            .unwrap();
        player.state.count_in = 0;
        player
    }
    #[test]
    fn wait_freezes_transport_and_pause_keeps_required_target() {
        let mut player = player();
        player.command(Command::Play).unwrap();
        player.tick(Duration::from_millis(5));
        assert_eq!(player.snapshot().required, vec![60]);
        player.tick(Duration::from_secs(1));
        assert_eq!(player.state.position, 0.);
        player.command(Command::Pause).unwrap();
        player.tick(Duration::from_secs(1));
        assert_eq!(player.snapshot().required, vec![60]);
        player.command(Command::Play).unwrap();
        player.midi(&[144, 60, 90]);
        player.midi(&[128, 60, 0]);
        assert_eq!(player.matcher.snapshot().matched_notes, 1);
        player.tick(Duration::from_millis(100));
        assert!(player.state.position > 0.);
    }
    #[test]
    fn input_overflow_pauses_and_invalid_seek_does_not_reset_results() {
        let mut player = player();
        player.command(Command::Play).unwrap();
        player.tick(Duration::from_millis(5));
        player.midi(&[144, 60, 90]);
        assert!(
            player
                .command(Command::Seek { position: f64::NAN })
                .is_err()
        );
        assert_eq!(player.matcher.snapshot().matched_notes, 1);
        player.input_overflow.store(true, Ordering::Release);
        player.tick(Duration::from_millis(5));
        assert_eq!(player.state.status, "paused");
        assert!(player.state.pressed.is_empty());
    }
    #[test]
    fn result_is_persisted_and_duplicate_save_is_ignored() {
        let mut player = player();
        player.command(Command::Play).unwrap();
        player.tick(Duration::from_millis(5));
        player.midi(&[144, 60, 90]);
        player.command(Command::Save).unwrap();
        player.command(Command::Save).unwrap();
        let id = player.file.as_ref().unwrap().content_id.clone();
        assert_eq!(player.history.song(&id).unwrap().sessions.len(), 1);
        let recovered = PracticeHistoryStore::load(
            player
                .file
                .as_ref()
                .unwrap()
                .source_path
                .as_ref()
                .unwrap()
                .parent()
                .unwrap()
                .join("web-practice-history.ron"),
        );
        assert_eq!(
            recovered.song(&id).unwrap().sessions[0]
                .summary
                .overall
                .matched_notes,
            1
        );
    }
    #[test]
    fn loop_saves_each_round_and_excludes_the_end_boundary() {
        let mut p = player();
        p.command(Command::Loop {
            start: 0.,
            end: 0.6,
            enabled: true,
        })
        .unwrap();
        for round in 1..=2 {
            p.command(Command::Play).unwrap();
            p.tick(Duration::from_millis(1));
            assert_eq!(p.snapshot().required, vec![60]);
            p.midi(&[144, 60, 90]);
            p.midi(&[128, 60, 0]);
            p.tick(Duration::from_secs(1));
            assert_eq!(p.state.repetitions, round);
            assert_eq!(p.state.position, 0.);
            assert_eq!(p.state.status, "playing");
        }
        let id = &p.file.as_ref().unwrap().content_id;
        let sessions = &p.history.song(id).unwrap().sessions;
        assert_eq!(sessions.len(), 2);
        assert!(
            sessions
                .iter()
                .all(|s| s.summary.overall.matched_notes == 1)
        );
        assert!(matches!(sessions[0].kind, PracticeSessionKind::Loop { .. }));
        assert!(
            p.command(Command::Loop {
                start: 2.,
                end: 1.,
                enabled: true
            })
            .is_err()
        );
        assert_eq!(p.state.passage.as_ref().unwrap().end, 0.6);
    }
    #[test]
    fn restart_restores_preferences_without_starting_playback() {
        let mut p = player();
        p.command(Command::Speed { value: 0.65 }).unwrap();
        p.command(Command::Wait { enabled: false }).unwrap();
        let track = p.notes[0].track_id;
        p.command(Command::Track {
            id: track,
            mode: "human".into(),
            part: "right".into(),
            visible: true,
        })
        .unwrap();
        p.command(Command::Hands {
            value: "right".into(),
        })
        .unwrap();
        p.command(Command::Loop {
            start: 0.6,
            end: 1.8,
            enabled: true,
        })
        .unwrap();
        let data = p.preferences_path.parent().unwrap().to_owned();
        let before = std::fs::read(&p.preferences_path).unwrap();
        drop(p);
        let mut restored = Player::new(data, PathBuf::new(), true);
        restored.restore();
        assert_eq!(restored.state.status, "ready");
        assert_eq!(restored.state.speed, 0.65);
        assert!(!restored.state.wait);
        assert_eq!(restored.state.hands, "right");
        assert_eq!(restored.state.position, 0.6);
        assert_eq!(restored.notes.len(), 9);
        assert_eq!(std::fs::read(&restored.preferences_path).unwrap(), before);
    }
    #[test]
    fn recent_and_history_use_verified_indexed_renamed_files_and_reject_replacements(){
      let mut p=player();let original=p.preferences_path.parent().unwrap().join("renamed-source.mid");
      std::fs::write(&original,include_bytes!("../assets/five-finger.mid")).unwrap();
      p.command(Command::Load{path:original.clone(),title:"移动曲目".into()}).unwrap();
      p.command(Command::CountIn{bars:0}).unwrap();p.command(Command::Play).unwrap();p.tick(Duration::from_millis(1));p.command(Command::Note{pitch:60,velocity:90,active:true}).unwrap();p.command(Command::Save).unwrap();
      let id=p.file.as_ref().unwrap().content_id.clone();let record=p.history.song(&id).unwrap().sessions.last().unwrap().id.clone().unwrap();
      let moved=original.with_file_name("relocated-source.mid");std::fs::rename(&original,&moved).unwrap();
      library_workspace::execute(Command::InspectSong{path:moved.clone(),force:true},&p.data).unwrap();
      let inventory=p.command(Command::Collection).unwrap();let row=inventory["songs"].as_array().unwrap().iter().find(|r|r["contentId"]==id).unwrap();assert_eq!(row["available"],true);assert_eq!(row["path"],moved.to_string_lossy().to_string());
      let opened=p.command(Command::OpenRecent{content_id:id.clone()}).unwrap();assert_eq!(opened["contentId"],id);assert_eq!(opened["sourcePath"],moved.to_string_lossy().to_string());
      p.command(Command::HistoryOpen{content_id:id.clone(),id:record,restore:true}).unwrap();assert_eq!(p.file.as_ref().unwrap().source_path.as_ref(),Some(&moved));
      std::fs::write(&moved,include_bytes!("../assets/basic-chords.mid")).unwrap();assert!(p.command(Command::OpenRecent{content_id:id}).is_err());
    }
    #[test]
    fn favorites_queue_and_imported_paths_survive_restart() {
        let mut p = player();
        let first = p.file.as_ref().unwrap().content_id.clone();
        p.command(Command::Favorite { enabled: true }).unwrap();
        p.command(Command::Queue { enabled: true }).unwrap();
        let second_path = p.preferences_path.parent().unwrap().join("chords.mid");
        std::fs::write(&second_path, include_bytes!("../assets/basic-chords.mid")).unwrap();
        p.command(Command::Load {
            path: second_path,
            title: "三和弦".into(),
        })
        .unwrap();
        let second = p.file.as_ref().unwrap().content_id.clone();
        p.command(Command::Queue { enabled: true }).unwrap();
        p.command(Command::MoveQueue { direction: -1 }).unwrap();
        p.command(Command::Volume { value: 0.4 }).unwrap();
        assert!(p.command(Command::Volume { value: 1.1 }).is_err());
        let data = p.preferences_path.parent().unwrap().to_owned();
        drop(p);
        let mut restored = Player::new(data, PathBuf::new(), true);
        restored.restore();
        assert_eq!(restored.state.volume, 0.4);
        assert_eq!(
            restored
                .history
                .song(&first)
                .unwrap()
                .library
                .queue_position,
            Some(1)
        );
        assert_eq!(
            restored
                .history
                .song(&second)
                .unwrap()
                .library
                .queue_position,
            Some(0)
        );
        assert!(restored.history.song(&first).unwrap().library.favorite);
        restored
            .command(Command::OpenRecent { content_id: first })
            .unwrap();
        assert_eq!(restored.notes.len(), 9);
    }
    #[test]
    fn chord_lesson_requires_all_three_pitches() {
        let mut p = player();
        let path = p.preferences_path.parent().unwrap().join("chords.mid");
        std::fs::write(&path, include_bytes!("../assets/basic-chords.mid")).unwrap();
        p.command(Command::Load {
            path,
            title: "三和弦".into(),
        })
        .unwrap();
        p.command(Command::CountIn { bars: 0 }).unwrap();
        p.command(Command::Play).unwrap();
        p.tick(Duration::from_millis(1));
        assert_eq!(p.snapshot().required, vec![60, 64, 67]);
        p.midi(&[144, 60, 90]);
        p.midi(&[144, 64, 90]);
        p.tick(Duration::from_millis(200));
        assert_eq!(p.state.position, 0.);
        p.midi(&[144, 67, 90]);
        p.tick(Duration::from_millis(200));
        assert!(p.state.position > 0.);
        assert_eq!(p.snapshot().score.matched_notes, 3);
    }
    #[test]
    fn continuous_mode_records_misses_and_structured_measures() {
        let mut p = player();
        p.command(Command::Mode {
            value: "flow".into(),
        })
        .unwrap();
        p.command(Command::Play).unwrap();
        for _ in 0..1500 {
            p.tick(Duration::from_millis(5));
        }
        assert_eq!(p.state.status, "finished");
        assert_eq!(p.snapshot().score.missed_notes, 9);
        assert!(p.state.saved);
        assert!(!p.previous_summary.as_ref().unwrap().measures.is_empty());
    }
    #[test]
    fn generated_practice_restores_fingers_hand_roles_and_collection() {
        let mut p = player();
        p.command(Command::Generate {
            spec: ExerciseSpec::default(),
        })
        .unwrap();
        let song = p.command(Command::CurrentSong).unwrap();
        let id = p.file.as_ref().unwrap().content_id.clone();
        assert!(
            song["notes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|n| n["finger"].as_u64().is_some())
        );
        assert!(
            p.config
                .tracks
                .iter()
                .any(|t| t.practice_part == PracticePart::LeftHand)
        );
        p.command(Command::Favorite { enabled: true }).unwrap();
        let directory = p.data.clone();
        drop(p);
        let mut restored = Player::new(directory, PathBuf::new(), true);
        restored.restore();
        assert_eq!(restored.file.as_ref().unwrap().content_id, id);
        assert!(restored.history.song(&id).unwrap().library.favorite);
        assert!(
            restored
                .command(Command::OpenRecent { content_id: id })
                .is_ok()
        );
        let exported = restored.command(Command::ExportMidi).unwrap();
        let bytes: Vec<u8> = serde_json::from_value(exported["bytes"].clone()).unwrap();
        assert!(midi_file::midly::Smf::parse(&bytes).is_ok());
    }
    #[test]
    fn hand_assignment_uses_track_even_when_pitch_crosses_middle_c() {
        let mut p = player();
        assert!(
            p.command(Command::Hands {
                value: "left".into()
            })
            .is_err()
        );
        let track = p.notes[0].track_id;
        p.command(Command::Track {
            id: track,
            mode: "human".into(),
            part: "left".into(),
            visible: true,
        })
        .unwrap();
        p.command(Command::Hands {
            value: "left".into(),
        })
        .unwrap();
        p.command(Command::Play).unwrap();
        p.tick(Duration::from_millis(5));
        assert_eq!(p.snapshot().required, vec![60]);
        p.midi(&[144, 60, 100]);
        assert_eq!(p.matcher.summary().parts[0].part, PracticePart::LeftHand);
    }
    #[test]
    fn count_in_does_not_move_transport_or_score() {
        let mut p = player();
        p.command(Command::CountIn { bars: 1 }).unwrap();
        p.command(Command::Play).unwrap();
        assert_eq!(p.state.status, "countIn");
        p.tick(Duration::from_millis(100));
        assert_eq!(p.state.position, 0.);
        assert!(p.snapshot().required.is_empty());
        p.tick(Duration::from_secs(10));
        assert_eq!(p.state.status, "playing");
        p.tick(Duration::from_millis(5));
        assert_eq!(p.snapshot().required, vec![60]);
    }
    #[test]
    fn recording_is_saved_and_reopens_as_playable_midi() {
        let mut p = player();
        let original = p.file.as_ref().unwrap().content_id.clone();
        p.command(Command::Recording { active: true }).unwrap();
        assert_eq!(p.state.mode, "wait");
        assert!(p.command(Command::Play).is_err());
        p.midi(&[144, 60, 99]);
        p.midi(&[176, 64, 127]);
        p.midi(&[128, 60, 0]);
        p.command(Command::Recording { active: false }).unwrap();
        assert_eq!(p.notes.len(), 1);
        assert!(
            p.file
                .as_ref()
                .unwrap()
                .source_path
                .as_ref()
                .unwrap()
                .is_file()
        );
        assert!(!p.snapshot().recording);
        assert_eq!(p.state.mode, "listen");
        p.command(Command::OpenRecent {
            content_id: original,
        })
        .unwrap();
        assert_eq!(p.state.mode, "wait");
        assert!(p.state.wait);
    }
    #[test]
    fn fingering_preview_acceptance_undo_and_content_guard() {
        let mut p = player();
        assert!(
            p.command(Command::SuggestFingers {
                hand: None,
                track: 0,
                index: 0,
                profile: Default::default(),
                range: None
            })
            .is_err()
        );
        let track = p
            .file
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| !t.notes.is_empty())
            .unwrap()
            .track_id;
        p.config
            .tracks
            .iter_mut()
            .find(|t| t.track_id == track)
            .unwrap()
            .practice_part = PracticePart::RightHand;
        let preview = p
            .command(Command::SuggestFingers {
                hand: None,
                track,
                index: 0,
                profile: Default::default(),
                range: Some((1, 1)),
            })
            .unwrap();
        assert_eq!(preview["proposals"].as_array().unwrap().len(), 4);
        assert!(p.saved_hints().unwrap().is_empty());
        let id = p.file.as_ref().unwrap().content_id.clone();
        let hint = neothesia_core::library::FingerHint {
            track_id: track,
            note_index: 0,
            finger: 2,
        };
        assert!(
            p.command(Command::ApplyFingers {
                content_id: "different".into(),
                hints: vec![hint]
            })
            .is_err()
        );
        let invalid = neothesia_core::library::FingerHint {
            note_index: 999,
            ..hint
        };
        assert!(
            p.command(Command::ApplyFingers {
                content_id: id.clone(),
                hints: vec![hint, invalid]
            })
            .is_err()
        );
        assert!(p.saved_hints().unwrap().is_empty());
        p.command(Command::ApplyFingers {
            content_id: id,
            hints: vec![hint],
        })
        .unwrap();
        assert_eq!(p.saved_hints().unwrap(), vec![hint]);
        p.command(Command::UndoFingers).unwrap();
        assert!(p.saved_hints().unwrap().is_empty());
    }
    #[test]
    fn generated_annotations_and_score_pair_survive_reopening() {
        let mut p = player();
        p.command(Command::Generate {
            spec: ExerciseSpec::default(),
        })
        .unwrap();
        let id = p.file.as_ref().unwrap().content_id.clone();
        let track = p
            .file
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| !t.notes.is_empty())
            .unwrap()
            .track_id;
        let hint = neothesia_core::library::FingerHint {
            track_id: track,
            note_index: 0,
            finger: 5,
        };
        p.command(Command::ApplyFingers {
            content_id: id.clone(),
            hints: vec![hint],
        })
        .unwrap();
        let xml=br#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note></measure></part></score-partwise>"#;
        p.command(Command::PairScore {
            bytes: xml.to_vec(),
            name: "exercise.musicxml".into(),
            content_id: id.clone(),
        })
        .unwrap();
        p.command(Command::OpenRecent { content_id: id }).unwrap();
        assert_eq!(p.saved_hints().unwrap(), vec![hint]);
        assert!(p.notation.is_some());
        let n = &p
            .file
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.track_id == track)
            .unwrap()
            .notes[0];
        assert_eq!(p.fingers.get(&(track, n.start, n.note)), Some(&5));
    }
    #[test]
    fn suggestions_preserve_note_identity_when_release_order_differs() {
        let mut p = player();
        let file = p.file.as_mut().unwrap();
        let mut tracks = file.tracks.to_vec();
        let track = tracks.iter_mut().find(|t| !t.notes.is_empty()).unwrap();
        let track_id = track.track_id;
        let mut notes = track.notes.to_vec();
        notes.swap(0, 1);
        track.notes = notes.into();
        file.tracks = tracks.into();
        p.config
            .tracks
            .iter_mut()
            .find(|t| t.track_id == track_id)
            .unwrap()
            .practice_part = PracticePart::RightHand;
        let preview = p
            .command(Command::SuggestFingers {
                hand: None,
                track: track_id,
                index: 1,
                profile: Default::default(),
                range: None,
            })
            .unwrap();
        assert_eq!(preview["proposals"][0]["index"], 1);
        assert_eq!(preview["proposals"][0]["pitch"], 60);
    }
    #[test]
    fn per_note_hands_drive_targets_recommendations_undo_and_restore() {
        let mut p = player();
        let track = p.notes[0].track_id;
        let n = p.notes[0].clone();
        let second = p
            .notes
            .iter()
            .find(|v| v.track_id == track && v.index != n.index)
            .unwrap()
            .clone();
        let id = p.file.as_ref().unwrap().content_id.clone();
        p.command(Command::HandSpanDefault {
            hand: PracticePart::LeftHand,
            profile: neothesia_core::fingering::HandSpanProfile::Compact,
        })
        .unwrap();
        let hint = neothesia_core::library::NoteHandHint {
            track_id: track,
            note_index: n.index,
            part: PracticePart::LeftHand,
        };
        assert!(
            p.command(Command::ApplyHands {
                content_id: "stale".into(),
                hints: vec![hint]
            })
            .is_err()
        );
        p.command(Command::ApplyHands {
            content_id: id.clone(),
            hints: vec![hint],
        })
        .unwrap();
        p.command(Command::Hands {
            value: "left".into(),
        })
        .unwrap();
        assert_eq!(p.note_player(track, n.index), PlayerConfig::Human);
        assert_eq!(p.note_player(track, second.index), PlayerConfig::Auto);
        p.command(Command::Play).unwrap();
        p.tick(Duration::from_millis(1));
        assert_eq!(p.snapshot().required, vec![n.note]);
        p.midi(&[144, n.note, 90]);
        assert_eq!(
            p.matcher
                .summary()
                .parts
                .iter()
                .find(|s| s.part == PracticePart::LeftHand)
                .unwrap()
                .breakdown
                .matched_notes,
            1
        );
        let proposed = p
            .command(Command::SuggestFingers {
                hand: Some(PracticePart::LeftHand),
                track,
                index: n.index,
                profile: Default::default(),
                range: Some((1, 1)),
            })
            .unwrap();
        assert!(
            proposed["proposals"]
                .as_array()
                .unwrap()
                .iter()
                .all(|h| h["index"] == n.index)
        );
        let data = p.data.clone();
        drop(p);
        let mut p = Player::new(data, PathBuf::new(), true);
        p.restore();
        assert_eq!(
            p.preferences
                .hand_span_defaults
                .get(&PracticePart::LeftHand),
            Some(&neothesia_core::fingering::HandSpanProfile::Compact)
        );
        assert_eq!(
            p.note_hands.get(&(track, n.index)),
            Some(&PracticePart::LeftHand)
        );
        p.command(Command::NoteHand {
            track,
            index: n.index,
            part: Some(PracticePart::RightHand),
        })
        .unwrap();
        p.command(Command::UndoHands).unwrap();
        assert_eq!(
            p.note_hands.get(&(track, n.index)),
            Some(&PracticePart::LeftHand)
        );
        let before = hands::hints(&p.note_hands);
        assert!(
            p.command(Command::ApplyHands {
                content_id: id,
                hints: vec![neothesia_core::library::NoteHandHint {
                    track_id: track,
                    note_index: usize::MAX,
                    part: PracticePart::LeftHand
                }]
            })
            .is_err()
        );
        assert_eq!(hands::hints(&p.note_hands), before);
    }
    #[test]
    fn corrected_meter_preserves_notes_changes_scope_and_restores_original() {
        let mut p = player();
        let original = p.file.as_ref().unwrap().musical_time.clone();
        let id = p.file.as_ref().unwrap().content_id.clone();
        let times: Vec<_> = p.notes.iter().map(|n| (n.start, n.end)).collect();
        let scope = p.practice_scope();
        let meter = neothesia_core::library::MeterCorrection {
            numerator: 3,
            denominator: 4,
            pickup_ticks: u64::from(original.ppq),
        };
        p.command(Command::Meter { value: Some(meter) }).unwrap();
        assert_ne!(p.practice_scope(), scope);
        assert!(p.file.as_ref().unwrap().musical_time.measures[0].partial);
        assert_eq!(
            p.file.as_ref().unwrap().musical_time.measures[0].end_tick,
            u64::from(original.ppq)
        );
        assert_eq!(
            times,
            p.notes.iter().map(|n| (n.start, n.end)).collect::<Vec<_>>()
        );
        assert!(
            p.command(Command::Meter {
                value: Some(neothesia_core::library::MeterCorrection {
                    numerator: 3,
                    denominator: 4,
                    pickup_ticks: u64::from(original.ppq) * 3
                })
            })
            .is_err()
        );
        let path = p.file.as_ref().unwrap().source_path.clone().unwrap();
        p.command(Command::Load {
            path,
            title: "reopened".into(),
        })
        .unwrap();
        assert_eq!(p.meter_correction, Some(meter));
        assert_eq!(p.file.as_ref().unwrap().content_id, id);
        p.command(Command::Meter { value: None }).unwrap();
        assert_eq!(p.practice_scope(), scope);
        assert_eq!(
            p.file.as_ref().unwrap().musical_time.measures.len(),
            original.measures.len()
        );
    }
    #[test]
    fn score_versions_keep_previous_files_and_switch_without_overwriting() {
        let mut p = player();
        let id = p.file.as_ref().unwrap().content_id.clone();
        let xml=br#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure></part></score-partwise>"#;
        for name in ["原版谱面", "教学版谱面"] {
            p.command(Command::PairScore {
                bytes: xml.to_vec(),
                name: name.into(),
                content_id: id.clone(),
            })
            .unwrap();
        }
        let rows = p.command(Command::ScoreVersions).unwrap();
        assert_eq!(rows["versions"].as_array().unwrap().len(), 2);
        let first = rows["versions"][0]["id"].as_str().unwrap().to_owned();
        let file = PathBuf::from(rows["versions"][0]["path"].as_str().unwrap());
        p.command(Command::ActivateScore {
            id: first.clone(),
            content_id: id.clone(),
        })
        .unwrap();
        assert_eq!(p.notation.as_ref().unwrap().name, "原版谱面");
        assert!(
            p.command(Command::RemoveScoreVersion {
                id: first.clone(),
                content_id: id.clone()
            })
            .is_err()
        );
        p.command(Command::DetachScore {
            content_id: id.clone(),
        })
        .unwrap();
        assert!(p.notation.is_none());
        p.command(Command::RemoveScoreVersion {
            id: first,
            content_id: id,
        })
        .unwrap();
        assert!(file.is_file());
    }
    #[test]
    fn direct_musicxml_import_persists_performance_score_hands_and_original_fingers() {
        let mut p = player();
        let imported = p
            .command(Command::ImportScore {
                bytes: include_bytes!("../../neothesia-core/src/score_performance_test.musicxml")
                    .to_vec(),
                name: "lesson.musicxml".into(),
                default_bpm: 100,
            })
            .unwrap();
        assert_eq!(imported["title"], "弱起与连音练习");
        assert!(imported["hasScore"].as_bool().unwrap());
        assert_eq!(imported["notes"].as_array().unwrap().len(), 6);
        assert!(
            imported["notes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["part"] == "left")
        );
        assert!(
            imported["notes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["finger"] == 3)
        );
        assert!((p.state.duration - 5.).abs() < 0.00001);
        let id = p.file.as_ref().unwrap().content_id.clone();
        let data = p.data.clone();
        drop(p);
        let mut p = Player::new(data, PathBuf::new(), true);
        p.restore();
        assert_eq!(p.file.as_ref().unwrap().content_id, id);
        assert!(p.notation.is_some());
        assert_eq!(p.notes.len(), 6);
        assert!(p.fingers.values().any(|f| *f == 3));
    }
    #[test]
    fn recital_and_memory_complete_once_without_waiting_and_restore_mode() {
        let mut p = player();
        p.command(Command::CountIn { bars: 0 }).unwrap();
        p.command(Command::MeasureLoop {
            start: 1,
            end: 1,
            enabled: true,
        })
        .unwrap();
        let expected = p.notes.len() as u64;
        for mode in ["recital", "memory"] {
            p.command(Command::Mode { value: mode.into() }).unwrap();
            assert!(p.state.passage.is_none());
            assert!(!p.state.wait);
            assert_eq!(p.state.rounds, 1);
            p.command(Command::Play).unwrap();
            assert!(p.command(Command::SeekMeasure { measure: 1 }).is_err());
            assert!(p.command(Command::Speed { value: 0.6 }).is_err());
            for _ in 0..1000 {
                p.tick(Duration::from_millis(20));
                if p.state.status == "finished" {
                    break;
                }
            }
            assert_eq!(p.state.status, "finished");
            assert!(p.snapshot().measure <= p.file.as_ref().unwrap().musical_time.measures.len());
            assert!(p.state.saved);
            let song = p
                .history
                .song(&p.file.as_ref().unwrap().content_id)
                .unwrap();
            let session = song.sessions.last().unwrap();
            assert_eq!(session.mode.as_deref(), Some(mode));
            assert_eq!(session.summary.overall.missed_notes as u64, expected);
            p.tick(Duration::from_secs(2));
            assert_eq!(p.state.status, "finished");
        }
        let data = p.data.clone();
        drop(p);
        let mut restored = Player::new(data, PathBuf::new(), true);
        restored.restore();
        assert_eq!(restored.state.mode, "memory");
        assert_eq!(restored.state.status, "ready");
        assert!(restored.state.passage.is_none());
    }
    #[test]
    fn history_filters_comparison_and_conditions_survive_restart() {
        let mut p = player();
        p.command(Command::Mode { value: "recital".into() }).unwrap();
        p.command(Command::Speed { value: 0.8 }).unwrap();
        let content_id = p.file.as_ref().unwrap().content_id.clone();
        for speed in [0.8, 0.8, 1.0] {
            p.command(Command::Speed { value: speed }).unwrap();
            p.command(Command::Restart).unwrap();
            p.command(Command::Play).unwrap();
            for _ in 0..1200 {
                p.tick(Duration::from_millis(20));
                if p.state.status == "finished" { break; }
            }
            assert!(p.state.saved);
        }
        let result=p.history_query("五指", "recital", "Both", "whole", 1, 1, 1).unwrap();
        assert_eq!(result["total"],3);
        assert_eq!(result["entries"].as_array().unwrap().len(),1);
        assert_eq!(p.history_query("", "memory", "", "", 0, 0, 50).unwrap()["total"],0);
        let sessions=&p.history.song(&content_id).unwrap().sessions;
        let id=sessions[1].id.clone().unwrap();
        let later=sessions[2].id.clone().unwrap();
        let data=p.data.clone();drop(p);
        let mut p=Player::new(data,PathBuf::new(),true);
        p.restore();
        let detail=p.history_detail(&content_id,&id).unwrap();
        assert_eq!(detail["comparison"]["count"],1);
        assert_eq!(p.history_detail(&content_id,&later).unwrap()["comparison"]["count"],0);
        assert!(detail["context"].is_object());
        p.command(Command::HistoryOpen {content_id:content_id.clone(),id:id.clone(),restore:true}).unwrap();
        assert!((p.state.speed-0.8).abs()<0.0001);
        assert_eq!(p.state.mode,"recital");
        assert_eq!(p.state.status,"ready");
        assert_eq!(p.state.count_in,0);
        let original=p.file.as_ref().unwrap().source_path.clone().unwrap();
        let relocated=original.with_file_name("relocated.mid");
        std::fs::rename(&original,&relocated).unwrap();
        p.command(Command::Load {path:relocated.clone(),title:"已移动的五指练习".into()}).unwrap();
        p.command(Command::HistoryOpen {content_id:content_id.clone(),id:id.clone(),restore:true}).unwrap();
        assert_eq!(p.file.as_ref().unwrap().source_path.as_ref(),Some(&relocated));
        p.command(Command::Meter {value:Some(neothesia_core::library::MeterCorrection {
            numerator:3,denominator:4,pickup_ticks:0
        })}).unwrap();
        let result=p.command(Command::HistoryOpen {content_id,id,restore:true}).unwrap();
        assert!(result["restoreWarning"].as_str().unwrap().contains("未恢复"));
        assert_eq!(p.meter_correction.unwrap().numerator,3);
    }
    #[test]
    fn history_open_refuses_changed_original_file_before_replacing_song() {
        let mut p=player();
        p.command(Command::Play).unwrap();p.tick(Duration::from_millis(10));
        p.command(Command::Note {pitch:60,velocity:90,active:true}).unwrap();
        p.command(Command::Save).unwrap();
        let content_id=p.file.as_ref().unwrap().content_id.clone();
        let id=p.history.song(&content_id).unwrap().sessions.last().unwrap().id.clone().unwrap();
        let path=p.file.as_ref().unwrap().source_path.clone().unwrap();
        std::fs::write(path,b"changed").unwrap();
        assert!(p.command(Command::HistoryOpen {content_id:content_id.clone(),id,restore:true}).is_err());
        assert_eq!(p.file.as_ref().unwrap().content_id,content_id);
    }

    #[test]fn generated_annotations_keep_task_identity_and_restore_changed_cache(){let mut p=player();let spec=ExerciseSpec{tempo_bpm:75,hands:neothesia_core::exercise::ExerciseHands::Right,..Default::default()};p.command(Command::Generate{spec}).unwrap();let cid=p.file.as_ref().unwrap().content_id.clone();let path=p.annotation_path(p.file.as_ref().unwrap()).unwrap();assert_ne!(midi_file::MidiFile::new(&path).unwrap().content_id,cid);let mut metadata=neothesia_core::library::SongMetadata::default();metadata.notes=Some("保留技术练习备注".into());neothesia_core::library::save_song_metadata(&path,&cid,metadata.clone()).unwrap();let sidecar=neothesia_core::library::metadata_sidecar_path(&path);let bytes=std::fs::read(&sidecar).unwrap();p.command(Command::Generate{spec:ExerciseSpec{tempo_bpm:160,repetitions:3,hands:neothesia_core::exercise::ExerciseHands::Left,..spec}}).unwrap();assert_eq!(p.file.as_ref().unwrap().content_id,cid);assert_eq!(p.annotation_path(p.file.as_ref().unwrap()).unwrap(),path);assert_eq!(neothesia_core::library::load_song_metadata(&path,&cid).unwrap(),metadata);let foreign=include_bytes!("../assets/basic-chords.mid");std::fs::write(&path,foreign).unwrap();p.command(Command::Generate{spec}).unwrap();let recovered=p.annotation_path(p.file.as_ref().unwrap()).unwrap();assert_ne!(recovered,path);assert_eq!(std::fs::read(&path).unwrap(),foreign);assert_eq!(std::fs::read(&sidecar).unwrap(),bytes);assert_eq!(neothesia_core::library::load_song_metadata(&recovered,&cid).unwrap(),metadata);p.command(Command::Generate{spec:ExerciseSpec{tempo_bpm:90,..spec}}).unwrap();assert_eq!(p.annotation_path(p.file.as_ref().unwrap()).unwrap(),recovered);}

    #[test]
    fn history_restores_original_generated_variant_and_partial_is_not_mastery() {
        let mut p=player();
        let spec=ExerciseSpec {tempo_bpm:60,repetitions:1,hands:neothesia_core::exercise::ExerciseHands::Right,..Default::default()};
        p.command(Command::Generate {spec}).unwrap();
        p.command(Command::Mode {value:"wait".into()}).unwrap();
        p.command(Command::CountIn {bars:0}).unwrap();
        p.command(Command::Play).unwrap();p.tick(Duration::from_millis(10));
        let pitch=p.snapshot().required[0];
        p.command(Command::Note {pitch,velocity:90,active:true}).unwrap();
        p.command(Command::Save).unwrap();
        let content_id=p.file.as_ref().unwrap().content_id.clone();
        let history=p.history.song(&content_id).unwrap();
        let id=history.sessions.last().unwrap().id.clone().unwrap();
        assert_eq!(history.review_status(u64::MAX).unwrap().mastery_streak,0);
        let notes=p.notes.len();let duration=p.state.duration;
        p.command(Command::Generate {spec:ExerciseSpec {tempo_bpm:120,repetitions:2,..spec}}).unwrap();
        let data=p.data.clone();drop(p);
        let mut p=Player::new(data,PathBuf::new(),true);p.restore();
        p.command(Command::HistoryOpen {content_id,id,restore:true}).unwrap();
        assert_eq!(p.exercise,Some(spec));assert_eq!(p.notes.len(),notes);
        assert!((p.state.duration-duration).abs()<0.0001);
        assert_eq!(p.state.status,"ready");
    }
    #[test]
    fn ladder_advances_from_real_wait_rounds_records_stages_and_checkpoints_restart() {
        let mut p=player();
        let plan=neothesia_core::speed_ladder::SpeedLadderPlan {start_percent:50,target_percent:60,step_percent:10,passes_required:1,accuracy_percent:90,on_time_percent:None,failures_before_step_back:0,attempt_limit:10};
        let saved=p.command(Command::SaveLadder {id:None,name:"左手速度阶梯".into(),start:1,end:1,plan}).unwrap();
        let id=saved["id"].as_str().unwrap().to_string();
        p.command(Command::StartLadder {id:id.clone(),resume:false}).unwrap();
        assert_eq!(p.state.speed,0.5);assert_eq!(p.state.status,"ready");
        assert!(p.command(Command::Speed{value:1.}).is_err());
        assert!(p.command(Command::Save).is_err());
        assert!(!p.state.adaptive);assert_eq!(p.state.rounds,0);
        p.command(Command::Play).unwrap();
        for _ in 0..2000 {
            p.tick(Duration::from_millis(10));
            let required=p.snapshot().required;
            for pitch in required {
                p.command(Command::Note{pitch,velocity:90,active:true}).unwrap();
                p.command(Command::Note{pitch,velocity:0,active:false}).unwrap();
            }
            if p.ladder.as_ref().unwrap().progress.total_rounds>=1{break;}
        }
        assert_eq!(p.ladder.as_ref().unwrap().progress.stage,1);
        assert_eq!(p.state.speed,0.6);
        p.command(Command::StopLadder).unwrap();
        let content_id=p.file.as_ref().unwrap().content_id.clone();
        assert_eq!(p.history.song(&content_id).unwrap().review_status(u64::MAX).unwrap().mastery_streak,0);
        let plan=p.ladder.as_ref().unwrap().preset.plan.clone();
        p.command(Command::SaveLadder{id:Some(id.clone()),name:"仅改名保留进度".into(),start:1,end:1,plan}).unwrap();
        assert_eq!(p.ladder.as_ref().unwrap().progress.stage,1);
        let data=p.data.clone();let content_id=p.file.as_ref().unwrap().content_id.clone();drop(p);
        let mut p=Player::new(data,PathBuf::new(),true);p.restore();
        assert!(p.ladder.is_none());assert_eq!(p.state.status,"ready");
        p.command(Command::StartLadder{id:id.clone(),resume:true}).unwrap();
        assert_eq!(p.state.speed,0.6);assert_eq!(p.ladder.as_ref().unwrap().progress.total_rounds,1);
        p.command(Command::Play).unwrap();
        for _ in 0..2000 {
            p.tick(Duration::from_millis(10));
            for pitch in p.snapshot().required {
                p.command(Command::Note{pitch,velocity:90,active:true}).unwrap();
                p.command(Command::Note{pitch,velocity:0,active:false}).unwrap();
            }
            if p.ladder.as_ref().unwrap().progress.completed{break;}
        }
        assert!(p.ladder.as_ref().unwrap().progress.completed);
        assert!(!p.ladder.as_ref().unwrap().active);assert_eq!(p.state.status,"finished");
        let history=p.history.song(&content_id).unwrap();assert_eq!(history.sessions.len(),2);
        assert_eq!(history.sessions[0].context.as_ref().unwrap().ladder.as_ref().unwrap().stage,1);
        assert_eq!(history.sessions[1].context.as_ref().unwrap().ladder.as_ref().unwrap().stage,2);
        assert_eq!(history.review_status(u64::MAX).unwrap().mastery_streak,1);
        assert!(p.command(Command::StartLadder{id:id.clone(),resume:true}).is_err());
        p.command(Command::Meter{value:Some(neothesia_core::library::MeterCorrection{numerator:3,denominator:4,pickup_ticks:0})}).unwrap();
        assert!(p.command(Command::StartLadder{id,resume:false}).is_err());
    }
    fn finish_routine_rounds(p:&mut Player,attempts:u64) {
        p.command(Command::Play).unwrap();
        for _ in 0..2500 {
            p.tick(Duration::from_millis(10));
            for pitch in p.snapshot().required {
                p.command(Command::Note{pitch,velocity:90,active:true}).unwrap();
                p.command(Command::Note{pitch,velocity:0,active:false}).unwrap();
            }
            if p.routine_status().unwrap()["progress"]["attempts"].as_u64().unwrap()>=attempts{break;}
        }
    }
    #[test]
    fn daily_routines_grade_real_rounds_resume_keep_dates_separate_and_preserve_snapshots() {
        let mut p=player();
        p.command(Command::MeasureLoop{start:1,end:1,enabled:true}).unwrap();
        let rid=p.command(Command::SaveRoutine{id:None,day:None,name:"每日练习".into(),notes:"连奏".into(),copy_of:None}).unwrap()["id"].as_str().unwrap().to_string();
        let goal=routine_commands::RoutineGoal{expression:None,passes:2,accuracy:90,on_time:None,consecutive:true,attempt_limit:5};
        let item=p.command(Command::SaveRoutineItem{routine_id:rid.clone(),day:None,id:None,source:Some("current".into()),source_id:None,title:"选段练习".into(),notes:"放松手腕".into(),goal:goal.clone()}).unwrap()["id"].as_str().unwrap().to_string();
        p.command(Command::OpenRoutineItem{routine_id:rid.clone(),day:"2026-10-02".into(),item_id:item.clone(),resume:true}).unwrap();
        assert!(p.command(Command::Speed{value:0.6}).is_err());assert!(p.command(Command::Save).is_err());
        finish_routine_rounds(&mut p,1);
        assert_eq!(p.routine_status().unwrap()["progress"]["passed"],1);assert_eq!(p.routine_status().unwrap()["progress"]["completed"],false);
        p.command(Command::StopRoutine).unwrap();let data=p.data.clone();let content=p.file.as_ref().unwrap().content_id.clone();drop(p);
        let mut p=Player::new(data,PathBuf::new(),true);p.restore();assert!(p.routine.is_none());assert_eq!(p.state.status,"ready");
        p.command(Command::OpenRoutineItem{routine_id:rid.clone(),day:"2026-10-02".into(),item_id:item.clone(),resume:true}).unwrap();
        finish_routine_rounds(&mut p,2);assert_eq!(p.routine_status().unwrap()["progress"]["completed"],true);assert_eq!(p.state.status,"finished");
        let h=p.history.song(&content).unwrap();assert_eq!(h.sessions.len(),2);assert_eq!(h.sessions[0].context.as_ref().unwrap().routine.as_ref().unwrap().day,"2026-10-02");
        p.command(Command::StopRoutine).unwrap();
        p.command(Command::SaveRoutineItem{routine_id:rid.clone(),day:None,id:Some(item.clone()),source:None,source_id:None,title:"模板新目标".into(),notes:"新要求".into(),goal:routine_commands::RoutineGoal{expression:None,passes:3,..goal.clone()}}).unwrap();
        assert_eq!(p.routines("2026-10-02").unwrap()["runs"][0]["routine"]["items"][0]["goal"]["passes"],2);
        let copied=p.command(Command::SaveRoutine{id:None,day:Some("2026-10-04".into()),name:"预览复制".into(),notes:"".into(),copy_of:Some(rid.clone())}).unwrap()["id"].as_str().unwrap().to_string();
        assert_eq!(p.preferences.routines.iter().find(|r|r.id==copied).unwrap().items.len(),1);

        p.command(Command::OpenRoutineItem{routine_id:rid.clone(),day:"2026-10-03".into(),item_id:item.clone(),resume:true}).unwrap();
        assert_eq!(p.routine_status().unwrap()["progress"]["passed"],0);assert_eq!(p.routine_status().unwrap()["goal"]["passes"],3);
        p.command(Command::SkipRoutineItem{routine_id:rid.clone(),day:"2026-10-03".into(),item_id:item.clone(),reason:"今天休息".into()}).unwrap();
        let result=p.routines("2026-10-03").unwrap();assert_eq!(result["runs"][0]["progress"][&item]["skipped"],true);assert_eq!(result["runs"][0]["progress"][&item]["completed"],false);
        p.command(Command::RemoveRoutine{id:rid}).unwrap();assert_eq!(p.routines("2026-10-02").unwrap()["runs"][0]["progress"][&item]["completed"],true);
    }
    #[test]
    fn routine_ladder_snapshot_survives_removal_of_original_and_completes_from_actual_grading() {
        let mut p=player();
        let ladder=p.command(Command::SaveLadder{id:None,name:"单级阶梯".into(),start:1,end:1,plan:neothesia_core::speed_ladder::SpeedLadderPlan{start_percent:80,target_percent:80,step_percent:5,passes_required:1,accuracy_percent:90,on_time_percent:None,failures_before_step_back:0,attempt_limit:5}}).unwrap()["id"].as_str().unwrap().to_string();
        let rid=p.command(Command::SaveRoutine{id:None,day:None,name:"阶梯日课".into(),notes:"".into(),copy_of:None}).unwrap()["id"].as_str().unwrap().to_string();
        let item=p.command(Command::SaveRoutineItem{routine_id:rid.clone(),day:None,id:None,source:Some("ladder".into()),source_id:Some(ladder.clone()),title:"阶梯项目".into(),notes:"".into(),goal:routine_commands::RoutineGoal{expression:None,passes:1,accuracy:90,on_time:None,consecutive:false,attempt_limit:5}}).unwrap()["id"].as_str().unwrap().to_string();
        p.command(Command::RemoveLadder{id:ladder}).unwrap();
        p.command(Command::OpenRoutineItem{routine_id:rid,day:"2026-10-02".into(),item_id:item,resume:true}).unwrap();
        finish_routine_rounds(&mut p,1);assert_eq!(p.routine_status().unwrap()["progress"]["completed"],true);
        assert!(p.ladder.as_ref().unwrap().progress.completed);assert_eq!(p.state.status,"finished");
    }
    #[test]
    fn beat_navigation_uses_pickup_and_real_tempo_and_rejects_invalid_beats() {
        let mut p = player();
        p.command(Command::ImportScore {
            bytes: include_bytes!("../../neothesia-core/src/score_performance_test.musicxml")
                .to_vec(),
            name: "乐谱.musicxml".into(),
            default_bpm: 120,
        })
        .unwrap();
        assert!(
            p.command(Command::SeekBeat {
                measure: 1,
                beat: 2.
            })
            .is_err()
        );
        p.command(Command::SeekBeat {
            measure: 2,
            beat: 2.,
        })
        .unwrap();
        let before = p.state.position;
        p.command(Command::StepBeat { direction: 1 }).unwrap();
        assert!(p.state.position > before);
        p.command(Command::StepBeat { direction: -1 }).unwrap();
        assert!((p.state.position - before).abs() < 0.00001);
        assert!(
            p.command(Command::SeekBeat {
                measure: 2,
                beat: f64::NAN
            })
            .is_err()
        );
    }
    #[test] fn scoped_score_finger_edits_are_atomic_guarded_clearable_and_undoable(){
     let mut p=player();let id=p.file.as_ref().unwrap().content_id.clone();let track=p.file.as_ref().unwrap().tracks.iter().find(|t|t.notes.len()>1).unwrap().track_id;
     let edit=|index,finger|fingerings::Edit{track_id:track,note_index:index,finger};
     let before=p.saved_hints().unwrap();
     assert!(p.command(Command::EditFingersFor{content_id:id.clone(),edits:vec![edit(0,Some(2)),edit(9999,None)]}).is_err());assert_eq!(p.saved_hints().unwrap(),before);
     assert!(p.command(Command::EditFingersFor{content_id:"another-song".into(),edits:vec![edit(0,Some(2))]}).is_err());
     p.command(Command::EditFingersFor{content_id:id.clone(),edits:vec![edit(0,Some(2)),edit(1,Some(4))]}).unwrap();assert!(p.command(Command::CurrentSong).unwrap()["fingerUndoAvailable"].as_bool().unwrap());
     p.command(Command::EditFingersFor{content_id:id.clone(),edits:vec![edit(0,None)]}).unwrap();assert!(!p.saved_hints().unwrap().iter().any(|h|h.note_index==0));
     assert!(p.command(Command::UndoFingersFor{content_id:"another-song".into()}).is_err());
     p.command(Command::UndoFingersFor{content_id:id.clone()}).unwrap();assert_eq!(p.saved_hints().unwrap().iter().find(|h|h.note_index==0).unwrap().finger,2);
     p.command(Command::UndoFingersFor{content_id:id}).unwrap();assert_eq!(p.saved_hints().unwrap(),before);
    }

    #[test]
    fn saved_practice_time_excludes_pause_count_in_and_survives_reload() {
        let mut p=player();let cid=p.file.as_ref().unwrap().content_id.clone();
        p.state.count_in=1;p.command(Command::Play).unwrap();p.tick(Duration::from_secs(5));
        assert_eq!(p.session_clock,Duration::ZERO);
        p.tick(Duration::from_secs(2));assert_eq!(p.state.position,0.);
        p.command(Command::Pause).unwrap();p.tick(Duration::from_secs(10));
        p.command(Command::Play).unwrap();p.midi(&[144,60,90]);p.midi(&[128,60,0]);p.tick(Duration::from_millis(100));
        p.command(Command::Pause).unwrap();p.command(Command::Save).unwrap();p.command(Command::Save).unwrap();
        let stats=p.practice_time(&cid,0,9_000_000_000_000_000).unwrap();
        assert_eq!(stats["totalMs"],2100);assert_eq!(stats["periodMs"],2100);assert_eq!(stats["retainedSessions"],1);
        let path=p.data.clone();drop(p);let mut reloaded=Player::new(path,PathBuf::new(),true);
        assert_eq!(reloaded.practice_time(&cid,0,9_000_000_000_000_000).unwrap()["totalMs"],2100);
        let mut legacy=reloaded.history.song(&cid).unwrap().sessions[0].clone();legacy.id=Some("old-untimed".into());legacy.playing_ms=None;
        reloaded.history.record_session(&cid,"old",legacy).unwrap();
        let stats=reloaded.practice_time(&cid,0,9_000_000_000_000_000).unwrap();assert_eq!(stats["totalMs"],2100);assert_eq!(stats["unknownSessions"],1);
        assert_eq!(reloaded.practice_time(&cid,0,1).unwrap()["periodMs"],0);
        assert!(reloaded.practice_time(&cid,1,0).is_err());
    }

    #[test]
    fn library_collection_does_not_load_other_song_and_guards_content() {
        let mut p=player();let current=p.file.as_ref().unwrap().content_id.clone();let position=p.state.position;
        let other=p.data.join("other piece.mid");std::fs::write(&other,include_bytes!("../assets/c-major-scale.mid")).unwrap();
        let mut w=library_workspace::Workspace::load(&p.data).unwrap();let e=w.inspect(other.clone(),true).unwrap();w.save(&p.data).unwrap();let cid=e.content_id.unwrap();
        p.command(Command::SetLibraryCollection{path:other.clone(),content_id:cid.clone(),favorite:Some(true),queued:None}).unwrap();
        p.command(Command::SetLibraryCollection{path:other.clone(),content_id:cid.clone(),favorite:None,queued:Some(true)}).unwrap();
        assert!(p.history.song(&cid).unwrap().library.favorite);assert_eq!(p.history.song(&cid).unwrap().library.queue_position,Some(0));
        assert_eq!(p.file.as_ref().unwrap().content_id,current);assert_eq!(p.state.position,position);
        std::fs::write(&other,include_bytes!("../assets/basic-chords.mid")).unwrap();
        assert!(p.command(Command::SetLibraryCollection{path:other.clone(),content_id:cid.clone(),favorite:Some(false),queued:None}).is_err());assert!(p.history.song(&cid).unwrap().library.favorite);
        std::fs::remove_file(&other).unwrap();p.command(Command::SetLibraryCollection{path:other.clone(),content_id:cid.clone(),favorite:None,queued:Some(false)}).unwrap();assert_eq!(p.history.song(&cid).unwrap().library.queue_position,None);
        assert!(p.command(Command::SetLibraryCollection{path:other,content_id:cid,favorite:Some(true),queued:Some(true)}).is_err());
    }

    #[test]
    fn midi_import_keeps_searchable_title_and_personal_metadata_on_repeat() {
        let mut p = player();
        let bytes = include_bytes!("../assets/basic-chords.mid").to_vec();
        let first = p.command(Command::ImportMidi { bytes: bytes.clone(), name: "课堂和弦.mid".into() }).unwrap();
        let path = PathBuf::from(first["sourcePath"].as_str().unwrap());
        let cid = first["contentId"].as_str().unwrap();
        let rows = library::enriched(&[], &p.data);
        assert!(rows.iter().any(|r| r.path == path && r.title == "课堂和弦"));
        let mut sidecar = neothesia_core::library::load_song_sidecar(&path, cid).unwrap();
        sidecar.metadata.title = Some("老师命名的和弦课".into());
        sidecar.metadata.notes = Some("保持共同音".into());
        sidecar.metadata.tags = vec!["课堂".into()];
        sidecar.fingerings = vec![neothesia_core::library::FingerHint { track_id: 0, note_index: 0, finger: 1 }];
        neothesia_core::library::save_song_sidecar(&path, cid, sidecar.clone()).unwrap();
        let second = p.command(Command::ImportMidi { bytes, name: "其他文件名.mid".into() }).unwrap();
        assert_eq!(second["contentId"], first["contentId"]);
        let saved = neothesia_core::library::load_song_sidecar(&path, cid).unwrap();
        assert_eq!(saved.metadata, sidecar.metadata);
        assert_eq!(saved.fingerings, sidecar.fingerings);
        assert!(library::enriched(&[], &p.data).iter().any(|r| r.path == path && r.title == "老师命名的和弦课"));
    }

    #[test]
    fn changed_import_cache_recovers_score_and_midi_without_overwriting() {
        let mut p=player();let xml=br#"<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>5</octave></pitch><duration>4</duration><type>whole</type></note></measure></part></score-partwise>"#.to_vec();
        let first=p.command(Command::ImportScore{bytes:xml.clone(),name:"original.musicxml".into(),default_bpm:100}).unwrap();let path=PathBuf::from(first["sourcePath"].as_str().unwrap());let cid=first["contentId"].as_str().unwrap().to_owned();
        let mut annotations=neothesia_core::library::load_song_sidecar(&path,&cid).unwrap();annotations.metadata.notes=Some("teacher".into());annotations.fingerings=vec![neothesia_core::library::FingerHint{track_id:p.file.as_ref().unwrap().tracks.iter().find(|t|!t.notes.is_empty()).unwrap().track_id,note_index:0,finger:3}];neothesia_core::library::save_song_sidecar(&path,&cid,annotations).unwrap();let sidecar=neothesia_core::library::metadata_sidecar_path(&path);let personal=std::fs::read(&sidecar).unwrap();
        let foreign=include_bytes!("../assets/basic-chords.mid");std::fs::write(&path,foreign).unwrap();
        let restored=p.command(Command::ImportScore{bytes:xml.clone(),name:"original.musicxml".into(),default_bpm:100}).unwrap();let restored_path=PathBuf::from(restored["sourcePath"].as_str().unwrap());assert_ne!(path,restored_path);assert_eq!(restored["contentId"],cid);assert_eq!(std::fs::read(&path).unwrap(),foreign);assert_eq!(std::fs::read(&sidecar).unwrap(),personal);assert_eq!(restored["importRecovery"]["carriedAnnotations"],true);assert_eq!(neothesia_core::library::load_song_sidecar(&restored_path,&cid).unwrap().fingerings[0].finger,3);
        assert_eq!(p.command(Command::ImportScore{bytes:xml,name:"original.musicxml".into(),default_bpm:100}).unwrap()["sourcePath"],restored["sourcePath"]);
        let bytes=include_bytes!("../assets/five-finger.mid").to_vec();let midi=p.command(Command::ImportMidi{bytes:bytes.clone(),name:"test.mid".into()}).unwrap();let old=PathBuf::from(midi["sourcePath"].as_str().unwrap());std::fs::write(&old,foreign).unwrap();let recovered=p.command(Command::ImportMidi{bytes:bytes.clone(),name:"test.mid".into()}).unwrap();assert_ne!(recovered["sourcePath"],midi["sourcePath"]);assert_eq!(recovered["contentId"],midi["contentId"]);assert_eq!(std::fs::read(&old).unwrap(),foreign);
        let good=PathBuf::from(recovered["sourcePath"].as_str().unwrap());let broken=neothesia_core::library::metadata_sidecar_path(&good);std::fs::write(&broken,b"broken").unwrap();let next=p.command(Command::ImportMidi{bytes,name:"test.mid".into()}).unwrap();assert_ne!(next["sourcePath"],recovered["sourcePath"]);assert_eq!(std::fs::read(&broken).unwrap(),b"broken");
    }

}
