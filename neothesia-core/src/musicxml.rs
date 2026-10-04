//! A notation-neutral score model and a deliberately small MusicXML importer.
//!
//! This module preserves learning semantics that MIDI cannot express reliably.
//! It is not an engraving engine: renderers should consume [`Score`] instead of
//! depending on MusicXML's document structure.

use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::{Cursor, Read},
    path::{Component, Path},
    str,
};

use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Score {
    pub version: Option<String>,
    pub title: Option<String>,
    pub composer: Option<String>,
    pub parts: Vec<Part>,
    pub warnings: Vec<ImportWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub id: String,
    pub name: String,
    pub measures: Vec<Measure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measure {
    pub number: String,
    pub implicit: bool,
    pub start: ScoreTime,
    pub duration: ScoreTime,
    pub attributes: Vec<MeasureAttributes>,
    pub barlines: Vec<Barline>,
    pub events: Vec<ScoreEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Barline {
    pub location: BarlineLocation,
    pub repeat: Option<RepeatMark>,
    pub endings: Vec<EndingMark>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BarlineLocation {
    Left,
    Middle,
    Right,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepeatMark {
    pub direction: RepeatDirection,
    pub times: Option<u16>,
    pub winged: Option<String>,
    pub after_jump: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepeatDirection {
    Forward,
    Backward,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndingMark {
    pub kind: EndingType,
    pub number: Option<String>,
    pub passes: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndingType {
    Start,
    Stop,
    Discontinue,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScoreEvent {
    Note(Note),
    Direction(Direction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub id: ScoreEventId,
    pub onset: ScoreTime,
    pub duration: ScoreTime,
    pub voice: Option<String>,
    pub staff: u8,
    pub pitch: Option<Pitch>,
    pub chord: bool,
    pub grace: bool,
    pub ties: Vec<SpanType>,
    pub slurs: Vec<NumberedSpan>,
    pub time_modification: Option<TupletRatio>,
    pub tuplets: Vec<TupletSpan>,
    pub fingering: Option<String>,
    pub articulations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Direction {
    pub id: ScoreEventId,
    pub onset: ScoreTime,
    pub staff: Option<u8>,
    pub dynamics: Vec<String>,
    pub words: Vec<String>,
    pub tempo_bpm: Option<String>,
    pub navigation: Vec<SoundNavigation>,
    pub pedals: Vec<PedalMark>,
}

/// Playback instructions from one MusicXML sound element. Its own offset
/// overrides the surrounding direction offset, independently of visual layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundNavigation {
    pub onset: ScoreTime,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScoreEventId {
    pub part_id: String,
    pub measure_ordinal: u32,
    pub kind: ScoreEventKind,
    pub ordinal: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScoreEventKind {
    Note,
    Direction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PedalMark {
    pub kind: PedalType,
    pub number: u16,
    pub line: Option<bool>,
    pub sign: Option<bool>,
    pub abbreviated: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PedalType {
    Start,
    Stop,
    Sostenuto,
    Change,
    Continue,
    Discontinue,
    Resume,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAttributes {
    pub onset: ScoreTime,
    pub divisions: u32,
    pub key_fifths: Option<i8>,
    pub time_signature: Option<TimeSignature>,
    pub staves: Option<u8>,
    pub clefs: Vec<Clef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSignature {
    pub beats: String,
    pub beat_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clef {
    pub staff: u8,
    pub sign: String,
    pub line: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pitch {
    pub step: Step,
    pub alter: i8,
    pub octave: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

/// Exact score position measured in quarter notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreTime {
    pub numerator: i64,
    pub denominator: u32,
}

impl Default for ScoreTime {
    fn default() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }
}

impl PartialOrd for ScoreTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoreTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (i128::from(self.numerator) * i128::from(other.denominator))
            .cmp(&(i128::from(other.numerator) * i128::from(self.denominator)))
    }
}

impl ScoreTime {
    pub fn new(numerator: i64, denominator: u32) -> Self {
        let denominator = denominator.max(1);
        let divisor = gcd(numerator.unsigned_abs(), u64::from(denominator)) as i64;
        Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor as u32,
        }
    }

    pub(crate) fn add(self, other: Self) -> Self {
        Self::new(
            self.numerator * i64::from(other.denominator)
                + other.numerator * i64::from(self.denominator),
            self.denominator * other.denominator,
        )
    }

    pub(crate) fn subtract(self, other: Self) -> Self {
        self.add(Self::new(-other.numerator, other.denominator))
    }
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left.max(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanType {
    Start,
    Stop,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberedSpan {
    pub kind: SpanType,
    pub number: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupletRatio {
    pub actual_notes: u16,
    pub normal_notes: u16,
    pub normal_type: Option<String>,
    pub normal_dots: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupletSpan {
    pub kind: SpanType,
    pub number: u8,
    pub bracket: Option<bool>,
    pub show_number: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportWarning {
    pub location: String,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("invalid XML: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("MusicXML text is not valid UTF-8")]
    Utf8(#[from] str::Utf8Error),
    #[error("expected a score-partwise MusicXML document, found {0}")]
    UnsupportedRoot(String),
    #[error("MusicXML contains no score root")]
    MissingRoot,
    #[error("could not read MusicXML file: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid MXL archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("MXL archive contains too many entries ({0})")]
    TooManyArchiveEntries(usize),
    #[error("MXL archive is missing META-INF/container.xml")]
    MissingContainer,
    #[error("MXL container does not identify a MusicXML root file")]
    MissingRootFile,
    #[error("unsafe MXL root file path {0:?}")]
    UnsafeRootFilePath(String),
    #[error("{kind} exceeds the {limit} byte safety limit")]
    SizeLimit { kind: &'static str, limit: usize },
    #[error("{field} contains invalid integer {value:?}")]
    InvalidInteger { field: &'static str, value: String },
    #[error("note pitch is incomplete")]
    IncompletePitch,
}

const MAX_ARCHIVE_ENTRIES: usize = 4096;
const MAX_ARCHIVE_BYTES: usize = 256 * 1024 * 1024;
const MAX_CONTAINER_BYTES: usize = 1024 * 1024;
const MAX_SCORE_BYTES: usize = 64 * 1024 * 1024;
const MUSICXML_MEDIA_TYPE: &str = "application/vnd.recordare.musicxml+xml";
const MXL_MEDIA_TYPE: &str = "application/vnd.recordare.musicxml";

/// Imports either an uncompressed MusicXML document or a compressed MXL
/// container based on its file signature.
pub fn import_musicxml_document(source: &[u8]) -> Result<Score, ImportError> {
    if source.starts_with(b"PK\x03\x04")
        || source.starts_with(b"PK\x05\x06")
        || source.starts_with(b"PK\x07\x08")
    {
        import_mxl(source)
    } else {
        if source.len() > MAX_SCORE_BYTES {
            return Err(ImportError::SizeLimit {
                kind: "MusicXML document",
                limit: MAX_SCORE_BYTES,
            });
        }
        import_musicxml(source)
    }
}

/// Reads and imports `.musicxml`, `.xml` or `.mxl` without trusting the file
/// extension to identify compressed content.
pub fn import_musicxml_file(path: impl AsRef<Path>) -> Result<Score, ImportError> {
    let metadata = fs::metadata(path.as_ref())?;
    if metadata.len() > MAX_ARCHIVE_BYTES as u64 {
        return Err(ImportError::SizeLimit {
            kind: "MusicXML file",
            limit: MAX_ARCHIVE_BYTES,
        });
    }
    import_musicxml_document(&fs::read(path)?)
}

fn import_mxl(source: &[u8]) -> Result<Score, ImportError> {
    if source.len() > MAX_ARCHIVE_BYTES {
        return Err(ImportError::SizeLimit {
            kind: "MXL archive",
            limit: MAX_ARCHIVE_BYTES,
        });
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(source))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(ImportError::TooManyArchiveEntries(archive.len()));
    }

    let score_xml = extract_mxl_root(&mut archive)?;

    let mut score = import_musicxml(&score_xml)?;
    let mimetype_valid = match archive.by_name("mimetype") {
        Ok(mut entry) => {
            let mut value = Vec::new();
            read_limited(&mut entry, &mut value, "MXL mimetype", 256)?;
            value == MXL_MEDIA_TYPE.as_bytes()
        }
        Err(zip::result::ZipError::FileNotFound) => false,
        Err(error) => return Err(ImportError::Zip(error)),
    };
    if !mimetype_valid {
        warn_once(
            &mut score.warnings,
            "MXL container".into(),
            format!("missing or invalid {MXL_MEDIA_TYPE} mimetype marker"),
        );
    }
    Ok(score)
}

/// Returns the original XML root, retaining notation and engraving fields that
/// the learning model does not represent. MXL extraction uses the importer limits.
pub fn musicxml_source(source: &[u8]) -> Result<Vec<u8>, ImportError> {
    if source.starts_with(b"PK") {
        if source.len() > MAX_ARCHIVE_BYTES {
            return Err(ImportError::SizeLimit {
                kind: "MXL archive",
                limit: MAX_ARCHIVE_BYTES,
            });
        }
        let mut archive = zip::ZipArchive::new(Cursor::new(source))?;
        if archive.len() > MAX_ARCHIVE_ENTRIES {
            return Err(ImportError::TooManyArchiveEntries(archive.len()));
        }
        extract_mxl_root(&mut archive)
    } else {
        if source.len() > MAX_SCORE_BYTES {
            return Err(ImportError::SizeLimit {
                kind: "MusicXML document",
                limit: MAX_SCORE_BYTES,
            });
        }
        Ok(source.to_vec())
    }
}

fn extract_mxl_root(archive: &mut zip::ZipArchive<Cursor<&[u8]>>) -> Result<Vec<u8>, ImportError> {
    let mut container = Vec::new();
    {
        let mut entry = archive
            .by_name("META-INF/container.xml")
            .map_err(|error| match error {
                zip::result::ZipError::FileNotFound => ImportError::MissingContainer,
                other => ImportError::Zip(other),
            })?;
        if entry.size() > MAX_CONTAINER_BYTES as u64 {
            return Err(ImportError::SizeLimit {
                kind: "MXL container",
                limit: MAX_CONTAINER_BYTES,
            });
        }
        read_limited(
            &mut entry,
            &mut container,
            "MXL container",
            MAX_CONTAINER_BYTES,
        )?;
    }

    let root_path = parse_container_rootfile(&container)?;
    validate_rootfile_path(&root_path)?;

    let mut score_xml = Vec::new();
    {
        let mut entry = archive.by_name(&root_path).map_err(|error| match error {
            zip::result::ZipError::FileNotFound => ImportError::MissingRootFile,
            other => ImportError::Zip(other),
        })?;
        if entry.size() > MAX_SCORE_BYTES as u64 {
            return Err(ImportError::SizeLimit {
                kind: "MXL MusicXML root file",
                limit: MAX_SCORE_BYTES,
            });
        }
        read_limited(
            &mut entry,
            &mut score_xml,
            "MXL MusicXML root file",
            MAX_SCORE_BYTES,
        )?;
    }

    Ok(score_xml)
}

fn read_limited(
    source: &mut impl Read,
    destination: &mut Vec<u8>,
    kind: &'static str,
    limit: usize,
) -> Result<(), ImportError> {
    source.take(limit as u64 + 1).read_to_end(destination)?;
    if destination.len() > limit {
        return Err(ImportError::SizeLimit { kind, limit });
    }
    Ok(())
}

fn parse_container_rootfile(container: &[u8]) -> Result<String, ImportError> {
    let mut reader = Reader::from_reader(container);
    let mut first_path = None;
    loop {
        match reader.read_event()? {
            Event::Start(start) | Event::Empty(start) if start.name().as_ref() == b"rootfile" => {
                let path = attribute(&start, b"full-path")?;
                let media_type = attribute(&start, b"media-type")?;
                if let Some(path) = path {
                    if media_type.as_deref() == Some(MUSICXML_MEDIA_TYPE) {
                        return Ok(path);
                    }
                    first_path.get_or_insert(path);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    first_path.ok_or(ImportError::MissingRootFile)
}

fn validate_rootfile_path(path: &str) -> Result<(), ImportError> {
    let parsed = Path::new(path);
    let unsafe_component = parsed.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    });
    if path.is_empty() || path.contains('\\') || unsafe_component {
        return Err(ImportError::UnsafeRootFilePath(path.to_owned()));
    }
    Ok(())
}

#[derive(Default)]
struct NoteBuilder {
    duration: i64,
    voice: Option<String>,
    staff: u8,
    step: Option<Step>,
    alter: i8,
    octave: Option<i8>,
    rest: bool,
    chord: bool,
    grace: bool,
    ties: Vec<SpanType>,
    slurs: Vec<NumberedSpan>,
    tuplet_actual: Option<u16>,
    tuplet_normal: Option<u16>,
    tuplet_normal_type: Option<String>,
    tuplet_normal_dots: u8,
    tuplets: Vec<TupletSpan>,
    fingering: Option<String>,
    articulations: Vec<String>,
}

#[derive(Default)]
struct DirectionBuilder {
    staff: Option<u8>,
    offset: i64,
    dynamics: Vec<String>,
    words: Vec<String>,
    tempo_bpm: Option<String>,
    navigation: Vec<SoundBuilder>,
    pedals: Vec<PedalMark>,
}

#[derive(Default)]
struct SoundBuilder {
    tempo_bpm: Option<String>,
    attributes: BTreeMap<String, String>,
    offset: Option<i64>,
}

fn finish_direction(
    built: DirectionBuilder,
    measure: &mut Measure,
    part_id: &str,
    measure_ordinal: u32,
    ordinal: &mut u32,
    origin: ScoreTime,
    divisions: u32,
) {
    let id = ScoreEventId {
        part_id: part_id.into(),
        measure_ordinal,
        kind: ScoreEventKind::Direction,
        ordinal: *ordinal,
    };
    *ordinal = ordinal.saturating_add(1);
    let tempos: Vec<_> = built
        .navigation
        .iter()
        .filter_map(|sound| {
            sound.tempo_bpm.as_ref().map(|tempo| {
                (
                    origin.add(ScoreTime::new(
                        sound.offset.unwrap_or(built.offset),
                        divisions,
                    )),
                    tempo.clone(),
                )
            })
        })
        .collect();
    let direction_onset = origin.add(ScoreTime::new(built.offset, divisions));
    let primary_index = tempos
        .iter()
        .position(|(onset, _)| *onset == direction_onset);
    let primary_tempo = primary_index.map(|i| tempos[i].1.clone()).or_else(|| {
        if tempos.is_empty() {
            built.tempo_bpm.clone()
        } else {
            None
        }
    });
    let navigation = built
        .navigation
        .into_iter()
        .filter(|s| !s.attributes.is_empty())
        .map(|s| SoundNavigation {
            onset: origin.add(ScoreTime::new(s.offset.unwrap_or(built.offset), divisions)),
            attributes: s.attributes,
        })
        .collect();
    measure.events.push(ScoreEvent::Direction(Direction {
        id,
        onset: origin.add(ScoreTime::new(built.offset, divisions)),
        staff: built.staff,
        dynamics: built.dynamics,
        words: built.words,
        tempo_bpm: primary_tempo,
        navigation,
        pedals: built.pedals,
    }));
    for (index, (onset, tempo)) in tempos.into_iter().enumerate() {
        if Some(index) == primary_index {
            continue;
        }
        let id = ScoreEventId {
            part_id: part_id.into(),
            measure_ordinal,
            kind: ScoreEventKind::Direction,
            ordinal: *ordinal,
        };
        *ordinal = ordinal.saturating_add(1);
        measure.events.push(ScoreEvent::Direction(Direction {
            id,
            onset,
            staff: built.staff,
            dynamics: vec![],
            words: vec![],
            tempo_bpm: Some(tempo),
            navigation: vec![],
            pedals: vec![],
        }));
    }
}

#[derive(Default)]
struct AttributesBuilder {
    divisions: Option<u32>,
    key_fifths: Option<i8>,
    beats: Option<String>,
    beat_type: Option<String>,
    staves: Option<u8>,
    clefs: Vec<ClefBuilder>,
}

#[derive(Default)]
struct ClefBuilder {
    staff: u8,
    sign: Option<String>,
    line: Option<u8>,
}

struct BarlineBuilder {
    location: BarlineLocation,
    repeat: Option<RepeatMark>,
    endings: Vec<EndingMark>,
}

/// Imports uncompressed, partwise MusicXML (`.musicxml` or `.xml`).
///
/// Compressed `.mxl` containers and timewise scores are intentionally deferred
/// so callers can distinguish unsupported input from a damaged score.
pub fn import_musicxml(source: &[u8]) -> Result<Score, ImportError> {
    let mut reader = Reader::from_reader(source);
    reader.config_mut().trim_text(false);

    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut text_value = String::new();
    let mut root_seen = false;
    let mut score = Score {
        version: None,
        title: None,
        composer: None,
        parts: Vec::new(),
        warnings: Vec::new(),
    };
    let mut part_names = HashMap::new();
    let mut current_part: Option<Part> = None;
    let mut current_measure: Option<Measure> = None;
    let mut note: Option<NoteBuilder> = None;
    let mut direction: Option<DirectionBuilder> = None;
    let mut standalone_sound = false;
    let mut attributes: Option<AttributesBuilder> = None;
    let mut barline: Option<BarlineBuilder> = None;
    let mut divisions = 1_u32;
    let mut cursor = ScoreTime::default();
    let mut measure_max = ScoreTime::default();
    let mut part_position = ScoreTime::default();
    let mut previous_note_onset = ScoreTime::default();
    let mut backup_duration: Option<i64> = None;
    let mut forward_duration: Option<i64> = None;
    let mut score_part_id: Option<String> = None;
    let mut creator_is_composer = false;
    let mut non_onset_fingering = false;
    let mut measure_ordinal = 0_u32;
    let mut note_ordinal = 0_u32;
    let mut direction_ordinal = 0_u32;

    loop {
        match reader.read_event()? {
            Event::Start(start) => {
                let name = start.name().as_ref().to_vec();
                if !root_seen {
                    root_seen = true;
                    let root = str::from_utf8(&name)?.to_owned();
                    if root != "score-partwise" {
                        return Err(ImportError::UnsupportedRoot(root));
                    }
                    score.version = attribute(&start, b"version")?;
                }
                path.push(name.clone());
                text_value.clear();
                match name.as_slice() {
                    b"score-part" => score_part_id = attribute(&start, b"id")?,
                    b"creator" => {
                        creator_is_composer =
                            attribute(&start, b"type")?.as_deref() == Some("composer");
                    }
                    b"part" if parent_is(&path, 2, b"score-partwise") => {
                        let id = attribute(&start, b"id")?.unwrap_or_default();
                        current_part = Some(Part {
                            name: part_names.get(&id).cloned().unwrap_or_else(|| id.clone()),
                            id,
                            measures: Vec::new(),
                        });
                        part_position = ScoreTime::default();
                        divisions = 1;
                    }
                    b"measure" => {
                        measure_ordinal = current_part
                            .as_ref()
                            .map(|part| part.measures.len() as u32)
                            .unwrap_or_default();
                        note_ordinal = 0;
                        direction_ordinal = 0;
                        current_measure = Some(Measure {
                            number: attribute(&start, b"number")?.unwrap_or_default(),
                            implicit: attribute(&start, b"implicit")?.as_deref() == Some("yes"),
                            start: part_position,
                            duration: ScoreTime::default(),
                            attributes: Vec::new(),
                            barlines: Vec::new(),
                            events: Vec::new(),
                        });
                        cursor = ScoreTime::default();
                        measure_max = ScoreTime::default();
                        previous_note_onset = ScoreTime::default();
                    }
                    b"backup" => backup_duration = Some(0),
                    b"forward" => forward_duration = Some(0),
                    b"note" => {
                        note = Some(NoteBuilder {
                            staff: 1,
                            ..Default::default()
                        })
                    }
                    b"fingering" if note.is_some() => {
                        non_onset_fingering=attribute(&start,b"substitution")?.as_deref()==Some("yes") || attribute(&start,b"alternate")?.as_deref()==Some("yes");
                    }
                    b"direction" => direction = Some(DirectionBuilder::default()),
                    b"attributes" => attributes = Some(AttributesBuilder::default()),
                    b"barline" => {
                        barline = Some(BarlineBuilder {
                            location: parse_barline_location(attribute(&start, b"location")?),
                            repeat: None,
                            endings: Vec::new(),
                        });
                    }
                    b"clef" if attributes.is_some() => {
                        attributes.as_mut().unwrap().clefs.push(ClefBuilder {
                            staff: attribute(&start, b"number")?
                                .map(|value| parse_u8("clef number", &value))
                                .transpose()?
                                .unwrap_or(1),
                            ..Default::default()
                        });
                    }
                    b"rest" if note.is_some() => note.as_mut().unwrap().rest = true,
                    b"chord" if note.is_some() => note.as_mut().unwrap().chord = true,
                    b"grace" if note.is_some() => note.as_mut().unwrap().grace = true,
                    b"tie" | b"tied" if note.is_some() => {
                        push_span(&start, &mut note.as_mut().unwrap().ties)?
                    }
                    b"slur" if note.is_some() => {
                        let kind = span_attribute(&start)?;
                        let number = attribute(&start, b"number")?
                            .map(|value| parse_u8("slur number", &value))
                            .transpose()?
                            .unwrap_or(1);
                        note.as_mut()
                            .unwrap()
                            .slurs
                            .push(NumberedSpan { kind, number });
                    }
                    b"tuplet" if note.is_some() => {
                        push_tuplet(&start, &mut note.as_mut().unwrap().tuplets)?;
                    }
                    b"normal-dot" if note.is_some() => {
                        let builder = note.as_mut().unwrap();
                        builder.tuplet_normal_dots = builder.tuplet_normal_dots.saturating_add(1);
                    }
                    b"sound" if current_measure.is_some() => {
                        if direction.is_none() {
                            direction = Some(DirectionBuilder::default());
                            standalone_sound = true;
                        }
                        let built = direction.as_mut().unwrap();
                        if let Some(tempo) = attribute(&start, b"tempo")? {
                            built.tempo_bpm = Some(tempo);
                        }
                        built.navigation.push(sound_navigation(&start)?);
                    }
                    b"pedal" if direction.is_some() => {
                        direction
                            .as_mut()
                            .unwrap()
                            .pedals
                            .push(parse_pedal(&start)?);
                    }
                    b"repeat" if barline.is_some() => {
                        barline.as_mut().unwrap().repeat = Some(parse_repeat(&start)?);
                    }
                    b"ending" if barline.is_some() => {
                        barline
                            .as_mut()
                            .unwrap()
                            .endings
                            .push(parse_ending(&start)?);
                    }
                    name if note.is_some()
                        && parent_is(&path, 2, b"articulations")
                        && name != b"articulations" =>
                    {
                        note.as_mut()
                            .unwrap()
                            .articulations
                            .push(str::from_utf8(name)?.to_owned());
                    }
                    name if direction.is_some()
                        && parent_is(&path, 2, b"dynamics")
                        && name != b"dynamics" =>
                    {
                        direction
                            .as_mut()
                            .unwrap()
                            .dynamics
                            .push(str::from_utf8(name)?.to_owned());
                    }
                    b"transpose" | b"ornaments" | b"wedge" => {
                        warn_deferred(
                            &mut score.warnings,
                            current_part.as_ref(),
                            current_measure.as_ref(),
                            str::from_utf8(&name)?,
                        );
                    }
                    _ => {}
                }
            }
            Event::Empty(start) => {
                if !root_seen {
                    root_seen = true;
                    let name = start.name();
                    let root = str::from_utf8(name.as_ref())?.to_owned();
                    if root != "score-partwise" {
                        return Err(ImportError::UnsupportedRoot(root));
                    }
                    score.version = attribute(&start, b"version")?;
                }
                let name = start.name();
                if matches!(name.as_ref(), b"transpose" | b"ornaments" | b"wedge") {
                    warn_deferred(
                        &mut score.warnings,
                        current_part.as_ref(),
                        current_measure.as_ref(),
                        str::from_utf8(name.as_ref())?,
                    );
                }
                if name.as_ref() == b"sound" && direction.is_none() && current_measure.is_some() {
                    let built = DirectionBuilder {
                        tempo_bpm: attribute(&start, b"tempo")?,
                        navigation: vec![sound_navigation(&start)?],
                        ..Default::default()
                    };
                    finish_direction(
                        built,
                        current_measure.as_mut().unwrap(),
                        &current_part.as_ref().unwrap().id,
                        measure_ordinal,
                        &mut direction_ordinal,
                        part_position.add(cursor),
                        divisions,
                    );
                } else {
                    handle_empty(&start, &path, &mut note, &mut direction, &mut barline)?;
                }
            }
            Event::Text(text) => {
                let decoded = str::from_utf8(text.as_ref())?;
                text_value.push_str(
                    &quick_xml::escape::unescape(decoded)
                        .map_err(|error| ImportError::Xml(quick_xml::Error::Escape(error)))?,
                );
            }
            Event::CData(text) => text_value.push_str(str::from_utf8(text.as_ref())?),
            Event::GeneralRef(reference) => {
                let name = str::from_utf8(reference.as_ref())?;
                match name {
                    "amp" => text_value.push('&'),
                    "lt" => text_value.push('<'),
                    "gt" => text_value.push('>'),
                    "quot" => text_value.push('"'),
                    "apos" => text_value.push('\''),
                    _ => {
                        text_value.push('&');
                        text_value.push_str(name);
                        text_value.push(';');
                    }
                }
            }
            Event::End(end) => {
                let name = end.name().as_ref().to_vec();
                let value = text_value.trim().to_owned();
                match name.as_slice() {
                    b"movement-title" if score.title.is_none() => score.title = nonempty(value),
                    b"work-title" if score.title.is_none() => score.title = nonempty(value),
                    b"creator" if score.composer.is_none() && creator_is_composer => {
                        score.composer = nonempty(value);
                    }
                    b"creator" => creator_is_composer = false,
                    b"part-name" if score_part_id.is_some() => {
                        part_names.insert(score_part_id.clone().unwrap(), value);
                    }
                    b"score-part" => score_part_id = None,
                    b"divisions" if attributes.is_some() => {
                        let parsed = parse_u32("divisions", &value)?;
                        if parsed == 0 {
                            return Err(ImportError::InvalidInteger {
                                field: "divisions",
                                value,
                            });
                        }
                        attributes.as_mut().unwrap().divisions = Some(parsed);
                    }
                    b"fifths" if attributes.is_some() => {
                        attributes.as_mut().unwrap().key_fifths = Some(parse_i8("fifths", &value)?);
                    }
                    b"beats" if attributes.is_some() => {
                        attributes.as_mut().unwrap().beats = nonempty(value)
                    }
                    b"beat-type" if attributes.is_some() => {
                        attributes.as_mut().unwrap().beat_type = nonempty(value);
                    }
                    b"staves" if attributes.is_some() => {
                        attributes.as_mut().unwrap().staves = Some(parse_u8("staves", &value)?);
                    }
                    b"sign"
                        if attributes.is_some()
                            && !attributes.as_ref().unwrap().clefs.is_empty() =>
                    {
                        attributes.as_mut().unwrap().clefs.last_mut().unwrap().sign =
                            nonempty(value);
                    }
                    b"line"
                        if attributes.is_some()
                            && !attributes.as_ref().unwrap().clefs.is_empty() =>
                    {
                        attributes.as_mut().unwrap().clefs.last_mut().unwrap().line =
                            Some(parse_u8("clef line", &value)?);
                    }
                    b"attributes" => {
                        let built = attributes.take().unwrap();
                        divisions = built.divisions.unwrap_or(divisions);
                        current_measure
                            .as_mut()
                            .unwrap()
                            .attributes
                            .push(MeasureAttributes {
                                onset: part_position.add(cursor),
                                divisions,
                                key_fifths: built.key_fifths,
                                time_signature: built
                                    .beats
                                    .zip(built.beat_type)
                                    .map(|(beats, beat_type)| TimeSignature { beats, beat_type }),
                                staves: built.staves,
                                clefs: built
                                    .clefs
                                    .into_iter()
                                    .filter_map(|clef| {
                                        clef.sign.map(|sign| Clef {
                                            staff: clef.staff,
                                            sign,
                                            line: clef.line,
                                        })
                                    })
                                    .collect(),
                            });
                    }
                    b"step" if note.is_some() => {
                        note.as_mut().unwrap().step = Some(parse_step(&value)?)
                    }
                    b"alter" if note.is_some() => {
                        note.as_mut().unwrap().alter = parse_i8("alter", &value)?
                    }
                    b"octave" if note.is_some() => {
                        note.as_mut().unwrap().octave = Some(parse_i8("octave", &value)?)
                    }
                    b"duration" if note.is_some() => {
                        note.as_mut().unwrap().duration = parse_i64("note duration", &value)?
                    }
                    b"duration" if backup_duration.is_some() => {
                        backup_duration = Some(parse_i64("backup duration", &value)?)
                    }
                    b"duration" if forward_duration.is_some() => {
                        forward_duration = Some(parse_i64("forward duration", &value)?)
                    }
                    b"voice" if note.is_some() => note.as_mut().unwrap().voice = nonempty(value),
                    b"staff" if note.is_some() => {
                        note.as_mut().unwrap().staff = parse_u8("note staff", &value)?
                    }
                    b"staff" if direction.is_some() => {
                        direction.as_mut().unwrap().staff =
                            Some(parse_u8("direction staff", &value)?)
                    }
                    b"fingering" if note.is_some() => {
                        if !non_onset_fingering {note.as_mut().unwrap().fingering = nonempty(value);}
                    }
                    b"actual-notes" if note.is_some() => {
                        note.as_mut().unwrap().tuplet_actual =
                            Some(parse_u16("tuplet actual-notes", &value)?);
                    }
                    b"normal-notes" if note.is_some() => {
                        note.as_mut().unwrap().tuplet_normal =
                            Some(parse_u16("tuplet normal-notes", &value)?);
                    }
                    b"normal-type" if note.is_some() => {
                        note.as_mut().unwrap().tuplet_normal_type = nonempty(value);
                    }
                    b"words" if direction.is_some() => {
                        if let Some(value) = nonempty(value) {
                            direction.as_mut().unwrap().words.push(value);
                        }
                    }
                    b"offset" if direction.is_some() => {
                        let offset = parse_i64("sound/direction offset", &value)?;
                        if parent_is(&path, 2, b"sound") {
                            if let Some(sound) = direction.as_mut().unwrap().navigation.last_mut() {
                                sound.offset = Some(offset);
                            }
                        } else {
                            direction.as_mut().unwrap().offset = offset;
                        }
                    }
                    b"sound" if standalone_sound => {
                        standalone_sound = false;
                        finish_direction(
                            direction.take().unwrap(),
                            current_measure.as_mut().unwrap(),
                            &current_part.as_ref().unwrap().id,
                            measure_ordinal,
                            &mut direction_ordinal,
                            part_position.add(cursor),
                            divisions,
                        );
                    }
                    b"note" => {
                        let built = note.take().unwrap();
                        let id = ScoreEventId {
                            part_id: current_part
                                .as_ref()
                                .map(|part| part.id.clone())
                                .unwrap_or_default(),
                            measure_ordinal,
                            kind: ScoreEventKind::Note,
                            ordinal: note_ordinal,
                        };
                        note_ordinal = note_ordinal.saturating_add(1);
                        let time_modification = match (built.tuplet_actual, built.tuplet_normal) {
                            (Some(actual_notes), Some(normal_notes))
                                if actual_notes > 0 && normal_notes > 0 =>
                            {
                                Some(TupletRatio {
                                    actual_notes,
                                    normal_notes,
                                    normal_type: built.tuplet_normal_type,
                                    normal_dots: built.tuplet_normal_dots,
                                })
                            }
                            (None, None) => None,
                            _ => {
                                warn_once(
                                    &mut score.warnings,
                                    current_part
                                        .as_ref()
                                        .zip(current_measure.as_ref())
                                        .map(|(part, measure)| {
                                            format!("{} measure {}", part.id, measure.number)
                                        })
                                        .unwrap_or_else(|| "score".into()),
                                    "incomplete or zero tuplet ratio was ignored".into(),
                                );
                                None
                            }
                        };
                        let onset = if built.chord {
                            previous_note_onset
                        } else {
                            cursor
                        };
                        let duration = ScoreTime::new(built.duration, divisions);
                        let pitch = if built.rest {
                            None
                        } else {
                            Some(Pitch {
                                step: built.step.ok_or(ImportError::IncompletePitch)?,
                                alter: built.alter,
                                octave: built.octave.ok_or(ImportError::IncompletePitch)?,
                            })
                        };
                        current_measure
                            .as_mut()
                            .unwrap()
                            .events
                            .push(ScoreEvent::Note(Note {
                                id,
                                onset: part_position.add(onset),
                                duration,
                                voice: built.voice,
                                staff: built.staff,
                                pitch,
                                chord: built.chord,
                                grace: built.grace,
                                ties: built.ties,
                                slurs: built.slurs,
                                time_modification,
                                tuplets: built.tuplets,
                                fingering: built.fingering,
                                articulations: built.articulations,
                            }));
                        if !built.chord {
                            previous_note_onset = onset;
                            if !built.grace {
                                cursor = cursor.add(duration);
                                measure_max = measure_max.max(cursor);
                            }
                        } else {
                            measure_max = measure_max.max(onset.add(duration));
                        }
                    }
                    b"direction" => {
                        finish_direction(
                            direction.take().unwrap(),
                            current_measure.as_mut().unwrap(),
                            &current_part.as_ref().unwrap().id,
                            measure_ordinal,
                            &mut direction_ordinal,
                            part_position.add(cursor),
                            divisions,
                        );
                    }
                    b"barline" => {
                        let built = barline.take().unwrap();
                        current_measure.as_mut().unwrap().barlines.push(Barline {
                            location: built.location,
                            repeat: built.repeat,
                            endings: built.endings,
                        });
                    }
                    b"backup" => {
                        let duration = backup_duration.take().unwrap_or_default();
                        cursor = cursor.subtract(ScoreTime::new(duration, divisions));
                    }
                    b"forward" => {
                        let duration = forward_duration.take().unwrap_or_default();
                        cursor = cursor.add(ScoreTime::new(duration, divisions));
                        measure_max = measure_max.max(cursor);
                    }
                    b"measure" => {
                        let mut finished = current_measure.take().unwrap();
                        finished.duration = measure_max;
                        part_position = part_position.add(measure_max);
                        current_part.as_mut().unwrap().measures.push(finished);
                    }
                    b"part" if current_part.is_some() => {
                        score.parts.push(current_part.take().unwrap())
                    }
                    _ => {}
                }
                path.pop();
                text_value.clear();
            }
            Event::Eof => break,
            _ => {}
        }
    }

    if !root_seen {
        return Err(ImportError::MissingRoot);
    }
    if score.parts.is_empty() {
        score.warnings.push(ImportWarning {
            location: "score".into(),
            message: "No playable parts were found".into(),
        });
    }
    Ok(score)
}

fn handle_empty(
    start: &BytesStart<'_>,
    path: &[Vec<u8>],
    note: &mut Option<NoteBuilder>,
    direction: &mut Option<DirectionBuilder>,
    barline: &mut Option<BarlineBuilder>,
) -> Result<(), ImportError> {
    match start.name().as_ref() {
        b"rest" if note.is_some() => note.as_mut().unwrap().rest = true,
        b"chord" if note.is_some() => note.as_mut().unwrap().chord = true,
        b"grace" if note.is_some() => note.as_mut().unwrap().grace = true,
        b"tie" | b"tied" if note.is_some() => push_span(start, &mut note.as_mut().unwrap().ties)?,
        b"slur" if note.is_some() => {
            let kind = span_attribute(start)?;
            let number = attribute(start, b"number")?
                .map(|value| parse_u8("slur number", &value))
                .transpose()?
                .unwrap_or(1);
            note.as_mut()
                .unwrap()
                .slurs
                .push(NumberedSpan { kind, number });
        }
        b"tuplet" if note.is_some() => {
            push_tuplet(start, &mut note.as_mut().unwrap().tuplets)?;
        }
        b"normal-dot" if note.is_some() => {
            let builder = note.as_mut().unwrap();
            builder.tuplet_normal_dots = builder.tuplet_normal_dots.saturating_add(1);
        }
        name if note.is_some() && path.iter().any(|part| part == b"articulations") => {
            note.as_mut()
                .unwrap()
                .articulations
                .push(str::from_utf8(name)?.to_owned());
        }
        name if direction.is_some() && path.iter().any(|part| part == b"dynamics") => {
            direction
                .as_mut()
                .unwrap()
                .dynamics
                .push(str::from_utf8(name)?.to_owned());
        }
        b"sound" if direction.is_some() => {
            let built = direction.as_mut().unwrap();
            if let Some(tempo) = attribute(start, b"tempo")? {
                built.tempo_bpm = Some(tempo);
            }
            built.navigation.push(sound_navigation(start)?);
        }
        b"pedal" if direction.is_some() => {
            direction.as_mut().unwrap().pedals.push(parse_pedal(start)?);
        }
        b"repeat" if barline.is_some() => {
            barline.as_mut().unwrap().repeat = Some(parse_repeat(start)?);
        }
        b"ending" if barline.is_some() => {
            barline.as_mut().unwrap().endings.push(parse_ending(start)?);
        }
        _ => {}
    }
    Ok(())
}

fn parent_is(path: &[Vec<u8>], levels: usize, expected: &[u8]) -> bool {
    path.len() >= levels && path[path.len() - levels].as_slice() == expected
}

fn attribute(start: &BytesStart<'_>, name: &[u8]) -> Result<Option<String>, ImportError> {
    for attribute in start.attributes().with_checks(false) {
        let attribute = attribute.map_err(quick_xml::Error::InvalidAttr)?;
        if attribute.key.as_ref() == name {
            return Ok(Some(str::from_utf8(attribute.value.as_ref())?.to_owned()));
        }
    }
    Ok(None)
}

fn push_span(start: &BytesStart<'_>, spans: &mut Vec<SpanType>) -> Result<(), ImportError> {
    spans.push(span_attribute(start)?);
    Ok(())
}

fn push_tuplet(start: &BytesStart<'_>, tuplets: &mut Vec<TupletSpan>) -> Result<(), ImportError> {
    let kind = span_attribute(start)?;
    let number = attribute(start, b"number")?
        .map(|value| parse_u8("tuplet number", &value))
        .transpose()?
        .unwrap_or(1);
    let bracket = match attribute(start, b"bracket")?.as_deref() {
        Some("yes") => Some(true),
        Some("no") => Some(false),
        _ => None,
    };
    tuplets.push(TupletSpan {
        kind,
        number,
        bracket,
        show_number: attribute(start, b"show-number")?,
    });
    Ok(())
}

fn parse_pedal(start: &BytesStart<'_>) -> Result<PedalMark, ImportError> {
    let value = attribute(start, b"type")?.unwrap_or_default();
    let kind = match value.as_str() {
        "start" => PedalType::Start,
        "stop" => PedalType::Stop,
        "sostenuto" => PedalType::Sostenuto,
        "change" => PedalType::Change,
        "continue" => PedalType::Continue,
        "discontinue" => PedalType::Discontinue,
        "resume" => PedalType::Resume,
        _ => PedalType::Other(value),
    };
    Ok(PedalMark {
        kind,
        number: attribute(start, b"number")?
            .map(|value| parse_u16("pedal number", &value))
            .transpose()?
            .unwrap_or(1),
        line: yes_no_attribute(start, b"line")?,
        sign: yes_no_attribute(start, b"sign")?,
        abbreviated: yes_no_attribute(start, b"abbreviated")?,
    })
}

fn parse_barline_location(value: Option<String>) -> BarlineLocation {
    match value.as_deref() {
        Some("left") => BarlineLocation::Left,
        Some("middle") => BarlineLocation::Middle,
        None | Some("right") => BarlineLocation::Right,
        Some(value) => BarlineLocation::Other(value.to_owned()),
    }
}

fn sound_navigation(start: &BytesStart<'_>) -> Result<SoundBuilder, ImportError> {
    let mut result = SoundBuilder {
        tempo_bpm: attribute(start, b"tempo")?,
        ..Default::default()
    };
    for name in [
        "dacapo",
        "dalsegno",
        "tocoda",
        "segno",
        "coda",
        "fine",
        "time-only",
        "forward-repeat",
    ] {
        if let Some(value) = attribute(start, name.as_bytes())? {
            let value = value.trim().to_owned();
            if value != "no" || !matches!(name, "dacapo" | "forward-repeat") {
                result.attributes.insert(name.into(), value);
            }
        }
    }
    Ok(result)
}
fn parse_repeat(start: &BytesStart<'_>) -> Result<RepeatMark, ImportError> {
    let value = attribute(start, b"direction")?.unwrap_or_default();
    let direction = match value.as_str() {
        "forward" => RepeatDirection::Forward,
        "backward" => RepeatDirection::Backward,
        _ => RepeatDirection::Other(value),
    };
    Ok(RepeatMark {
        direction,
        times: attribute(start, b"times")?
            .map(|value| parse_u16("repeat times", &value))
            .transpose()?,
        winged: attribute(start, b"winged")?,
        after_jump: attribute(start, b"after-jump")?.map(|v| v == "yes"),
    })
}

fn parse_ending(start: &BytesStart<'_>) -> Result<EndingMark, ImportError> {
    let value = attribute(start, b"type")?.unwrap_or_default();
    let kind = match value.as_str() {
        "start" => EndingType::Start,
        "stop" => EndingType::Stop,
        "discontinue" => EndingType::Discontinue,
        _ => EndingType::Other(value),
    };
    let number = attribute(start, b"number")?;
    Ok(EndingMark {
        passes: number
            .as_deref()
            .map(parse_ending_passes)
            .unwrap_or_default(),
        number,
        kind,
    })
}

fn parse_ending_passes(value: &str) -> Vec<u16> {
    const MAX_ENDING_PASS: u16 = 128;
    let mut result = Vec::new();
    for item in value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        if let Some((start, end)) = item.split_once('-') {
            if let (Ok(start), Ok(end)) = (start.trim().parse::<u16>(), end.trim().parse::<u16>())
                && start <= end
            {
                result.extend(start..=end.min(MAX_ENDING_PASS));
            }
        } else if let Ok(pass) = item.parse::<u16>()
            && pass <= MAX_ENDING_PASS
        {
            result.push(pass);
        }
    }
    result.sort_unstable();
    result.dedup();
    result
}

fn yes_no_attribute(start: &BytesStart<'_>, name: &[u8]) -> Result<Option<bool>, ImportError> {
    Ok(match attribute(start, name)?.as_deref() {
        Some("yes") => Some(true),
        Some("no") => Some(false),
        _ => None,
    })
}

fn span_attribute(start: &BytesStart<'_>) -> Result<SpanType, ImportError> {
    Ok(match attribute(start, b"type")?.as_deref() {
        Some("stop") => SpanType::Stop,
        Some("continue") => SpanType::Continue,
        _ => SpanType::Start,
    })
}

fn nonempty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn warn_once(warnings: &mut Vec<ImportWarning>, location: String, message: String) {
    if !warnings
        .iter()
        .any(|warning| warning.location == location && warning.message == message)
    {
        warnings.push(ImportWarning { location, message });
    }
}

fn warn_deferred(
    warnings: &mut Vec<ImportWarning>,
    part: Option<&Part>,
    measure: Option<&Measure>,
    element: &str,
) {
    let location = match (part, measure) {
        (Some(part), Some(measure)) => format!("{} measure {}", part.id, measure.number),
        (None, Some(measure)) => format!("measure {}", measure.number),
        _ => "score".into(),
    };
    let message = match element {
        "ornaments" => "ornament notation and playback are not represented yet",
        "transpose" => "written-to-sounding transposition is not represented yet",
        "wedge" => "crescendo and diminuendo wedge spans are not represented yet",
        _ => "notation element is not represented yet",
    };
    warn_once(warnings, location, message.into());
}

fn parse_step(value: &str) -> Result<Step, ImportError> {
    match value {
        "C" => Ok(Step::C),
        "D" => Ok(Step::D),
        "E" => Ok(Step::E),
        "F" => Ok(Step::F),
        "G" => Ok(Step::G),
        "A" => Ok(Step::A),
        "B" => Ok(Step::B),
        _ => Err(ImportError::InvalidInteger {
            field: "pitch step",
            value: value.to_owned(),
        }),
    }
}

macro_rules! parse_integer {
    ($name:ident, $kind:ty) => {
        fn $name(field: &'static str, value: &str) -> Result<$kind, ImportError> {
            value.parse().map_err(|_| ImportError::InvalidInteger {
                field,
                value: value.to_owned(),
            })
        }
    };
}

parse_integer!(parse_u8, u8);
parse_integer!(parse_u16, u16);
parse_integer!(parse_u32, u32);
parse_integer!(parse_i8, i8);
parse_integer!(parse_i64, i64);

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

    const PIANO_SCORE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<score-partwise version="4.0">
  <movement-title>Learning &amp; Study</movement-title>
  <identification>
    <creator type="lyricist">Not the composer</creator>
    <creator type="composer">Neothesia</creator>
  </identification>
  <part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
  <part id="P1">
    <measure number="1">
      <attributes>
        <divisions>4</divisions><key><fifths>0</fifths></key>
        <time><beats>4</beats><beat-type>4</beat-type></time><staves>2</staves>
        <clef number="1"><sign>G</sign><line>2</line></clef>
        <clef number="2"><sign>F</sign><line>4</line></clef>
      </attributes>
      <direction><direction-type><dynamics><p/></dynamics><words>dolce</words></direction-type><sound tempo="72"/></direction>
      <note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><voice>1</voice><staff>1</staff>
        <notations><slur type="start" number="1"/><technical><fingering>1</fingering></technical></notations></note>
      <note><chord/><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration><voice>1</voice><staff>1</staff>
        <notations><articulations><staccato/></articulations><technical><fingering>3</fingering></technical></notations></note>
      <note><pitch><step>G</step><octave>4</octave></pitch><duration>4</duration><voice>1</voice><staff>1</staff>
        <tie type="start"/><notations><slur type="stop" number="1"/><ornaments><trill-mark/></ornaments></notations></note>
      <backup><duration>8</duration></backup>
      <note><pitch><step>C</step><octave>3</octave></pitch><duration>8</duration><voice>2</voice><staff>2</staff></note>
    </measure>
  </part>
</score-partwise>"#;

    fn mxl_with_container(container: &str, include_mimetype: bool) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        if include_mimetype {
            writer.start_file("mimetype", stored).unwrap();
            writer.write_all(MXL_MEDIA_TYPE.as_bytes()).unwrap();
        }
        writer
            .start_file("META-INF/container.xml", deflated)
            .unwrap();
        writer.write_all(container.as_bytes()).unwrap();
        writer
            .start_file("scores/piano.musicxml", deflated)
            .unwrap();
        writer.write_all(PIANO_SCORE.as_bytes()).unwrap();
        writer.finish().unwrap().into_inner()
    }

    fn valid_container(path: &str) -> String {
        format!(
            r#"<?xml version="1.0"?>
<container version="1.0">
  <rootfiles>
    <rootfile full-path="{path}" media-type="{MUSICXML_MEDIA_TYPE}"/>
  </rootfiles>
</container>"#
        )
    }

    #[test]
    fn imports_piano_learning_semantics_and_exact_timing() {
        let score = import_musicxml(PIANO_SCORE.as_bytes()).unwrap();
        assert_eq!(score.version.as_deref(), Some("4.0"));
        assert_eq!(score.title.as_deref(), Some("Learning & Study"));
        assert_eq!(score.composer.as_deref(), Some("Neothesia"));
        assert_eq!(score.parts[0].name, "Piano");
        assert!(
            score
                .warnings
                .iter()
                .any(|warning| { warning.message.starts_with("ornament notation") })
        );

        let measure = &score.parts[0].measures[0];
        assert_eq!(measure.duration, ScoreTime::new(2, 1));
        assert_eq!(measure.attributes[0].staves, Some(2));
        assert_eq!(measure.attributes[0].clefs.len(), 2);

        let ScoreEvent::Direction(direction) = &measure.events[0] else {
            panic!("direction should be preserved");
        };
        assert_eq!(
            direction.id,
            ScoreEventId {
                part_id: "P1".into(),
                measure_ordinal: 0,
                kind: ScoreEventKind::Direction,
                ordinal: 0,
            }
        );
        assert_eq!(direction.dynamics, ["p"]);
        assert_eq!(direction.words, ["dolce"]);
        assert_eq!(direction.tempo_bpm.as_deref(), Some("72"));

        let notes: Vec<_> = measure
            .events
            .iter()
            .filter_map(|event| match event {
                ScoreEvent::Note(note) => Some(note),
                _ => None,
            })
            .collect();
        assert_eq!(notes.len(), 4);
        assert_eq!(
            notes.iter().map(|note| note.id.ordinal).collect::<Vec<_>>(),
            [0, 1, 2, 3]
        );
        assert!(notes.iter().all(|note| note.id.part_id == "P1"
            && note.id.measure_ordinal == 0
            && note.id.kind == ScoreEventKind::Note));
        assert_eq!(notes[0].onset, ScoreTime::new(0, 1));
        assert_eq!(notes[1].onset, notes[0].onset);
        assert!(notes[1].chord);
        assert_eq!(notes[0].fingering.as_deref(), Some("1"));
        assert_eq!(notes[1].articulations, ["staccato"]);
        assert_eq!(notes[2].ties, [SpanType::Start]);
        assert_eq!(notes[3].staff, 2);
        assert_eq!(notes[3].voice.as_deref(), Some("2"));
        assert_eq!(notes[3].onset, ScoreTime::new(0, 1));
    }

    #[test]
    fn rejects_timewise_scores_explicitly() {
        let error = import_musicxml(b"<score-timewise version=\"4.0\"/>").unwrap_err();
        assert!(matches!(error, ImportError::UnsupportedRoot(root) if root == "score-timewise"));
    }

    #[test]
    fn rational_time_reduces_without_losing_tuplets() {
        assert_eq!(ScoreTime::new(6, 12), ScoreTime::new(1, 2));
        assert_eq!(
            ScoreTime::new(1, 3).add(ScoreTime::new(1, 6)),
            ScoreTime::new(1, 2)
        );
    }

    #[test]
    fn imports_compressed_mxl_by_signature() {
        let source = mxl_with_container(&valid_container("scores/piano.musicxml"), true);
        assert_eq!(musicxml_source(&source).unwrap(), PIANO_SCORE.as_bytes());
        assert_eq!(
            musicxml_source(PIANO_SCORE.as_bytes()).unwrap(),
            PIANO_SCORE.as_bytes()
        );
        let score = import_musicxml_document(&source).unwrap();
        assert_eq!(score.title.as_deref(), Some("Learning & Study"));
        assert_eq!(score.parts[0].measures.len(), 1);
        assert!(
            score
                .warnings
                .iter()
                .all(|warning| warning.location != "MXL container")
        );
    }

    #[test]
    fn accepts_missing_mimetype_with_visible_warning() {
        let source = mxl_with_container(&valid_container("scores/piano.musicxml"), false);
        let score = import_musicxml_document(&source).unwrap();
        assert!(
            score
                .warnings
                .iter()
                .any(|warning| warning.location == "MXL container")
        );
    }

    #[test]
    fn rejects_container_path_traversal_before_reading_score() {
        let source = mxl_with_container(&valid_container("../piano.musicxml"), true);
        let error = import_musicxml_document(&source).unwrap_err();
        assert!(
            matches!(error, ImportError::UnsafeRootFilePath(path) if path == "../piano.musicxml")
        );
    }

    #[test]
    fn rejects_zip_without_musicxml_container() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("readme.txt", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"not an MXL file").unwrap();
        let source = writer.finish().unwrap().into_inner();
        assert!(matches!(
            import_musicxml_document(&source).unwrap_err(),
            ImportError::MissingContainer
        ));
    }

    #[test]
    fn accepts_explicit_rest_tags_from_older_exporters() {
        let source = br#"<score-partwise>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1"><measure number="1"><attributes><divisions>2</divisions></attributes>
<note><rest></rest><duration>2</duration><voice>1</voice></note>
</measure></part></score-partwise>"#;
        let score = import_musicxml(source).unwrap();
        let ScoreEvent::Note(rest) = &score.parts[0].measures[0].events[0] else {
            panic!("rest should remain a score event");
        };
        assert_eq!(rest.pitch, None);
        assert_eq!(rest.duration, ScoreTime::new(1, 1));
    }

    #[test]
    fn preserves_repeat_barlines_and_numbered_endings() {
        let source = br#"<score-partwise version="4.0">
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1">
  <measure number="1">
    <barline location="left"><repeat direction="forward" winged="curved"/></barline>
    <note><rest/><duration>1</duration></note>
  </measure>
  <measure number="2">
    <note><rest/><duration>1</duration></note>
    <barline>
      <ending number="1, 3-4" type="start"/>
      <repeat direction="backward" times="4" winged="double-curved"/>
    </barline>
  </measure>
  <measure number="3">
    <barline location="right"><ending number="1" type="discontinue"/></barline>
    <note><rest/><duration>1</duration></note>
  </measure>
</part></score-partwise>"#;
        let score = import_musicxml(source).unwrap();
        let measures = &score.parts[0].measures;

        assert_eq!(measures[0].barlines[0].location, BarlineLocation::Left);
        assert_eq!(
            measures[0].barlines[0].repeat,
            Some(RepeatMark {
                direction: RepeatDirection::Forward,
                times: None,
                winged: Some("curved".into()),
                after_jump: None,
            })
        );
        assert_eq!(measures[1].barlines[0].location, BarlineLocation::Right);
        assert_eq!(
            measures[1].barlines[0].repeat,
            Some(RepeatMark {
                direction: RepeatDirection::Backward,
                times: Some(4),
                winged: Some("double-curved".into()),
                after_jump: None,
            })
        );
        assert_eq!(
            measures[1].barlines[0].endings[0],
            EndingMark {
                kind: EndingType::Start,
                number: Some("1, 3-4".into()),
                passes: vec![1, 3, 4],
            }
        );
        assert_eq!(
            measures[2].barlines[0].endings[0].kind,
            EndingType::Discontinue
        );
    }

    #[test]
    fn preserves_empty_pedal_direction_elements() {
        let source = br#"<score-partwise>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1"><measure number="1">
<direction><direction-type>
<pedal type="start" number="2" line="yes" sign="yes" abbreviated="no"/>
<pedal type="change" number="2" line="yes"/>
</direction-type></direction>
<note><rest/><duration>1</duration></note>
</measure></part></score-partwise>"#;
        let score = import_musicxml(source).unwrap();
        assert!(score.warnings.is_empty());
        let ScoreEvent::Direction(direction) = &score.parts[0].measures[0].events[0] else {
            panic!("pedal should remain a direction");
        };
        assert_eq!(
            direction.pedals,
            [
                PedalMark {
                    kind: PedalType::Start,
                    number: 2,
                    line: Some(true),
                    sign: Some(true),
                    abbreviated: Some(false),
                },
                PedalMark {
                    kind: PedalType::Change,
                    number: 2,
                    line: Some(true),
                    sign: None,
                    abbreviated: None,
                }
            ]
        );
    }

    #[test]
    fn preserves_tuplet_ratio_and_display_span() {
        let source = br#"<score-partwise>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1"><measure number="1"><attributes><divisions>12</divisions></attributes>
<note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration>
<time-modification><actual-notes>3</actual-notes><normal-notes>2</normal-notes><normal-type>eighth</normal-type></time-modification>
<notations><tuplet type="start" number="1" bracket="yes" show-number="actual"/></notations></note>
<note><pitch><step>D</step><octave>4</octave></pitch><duration>4</duration>
<time-modification><actual-notes>3</actual-notes><normal-notes>2</normal-notes><normal-type>eighth</normal-type></time-modification>
<notations><tuplet type="stop" number="1"/></notations></note>
</measure></part></score-partwise>"#;
        let score = import_musicxml(source).unwrap();
        assert!(score.warnings.is_empty());
        let notes: Vec<_> = score.parts[0].measures[0]
            .events
            .iter()
            .filter_map(|event| match event {
                ScoreEvent::Note(note) => Some(note),
                _ => None,
            })
            .collect();
        assert_eq!(notes[0].duration, ScoreTime::new(1, 3));
        assert_eq!(
            notes[0].time_modification,
            Some(TupletRatio {
                actual_notes: 3,
                normal_notes: 2,
                normal_type: Some("eighth".into()),
                normal_dots: 0,
            })
        );
        assert_eq!(
            notes[0].tuplets,
            [TupletSpan {
                kind: SpanType::Start,
                number: 1,
                bracket: Some(true),
                show_number: Some("actual".into()),
            }]
        );
        assert_eq!(notes[1].tuplets[0].kind, SpanType::Stop);
    }

    #[test]
    fn score_event_ids_are_deterministic_across_measures() {
        let source = br#"<score-partwise>
<part-list><score-part id="Piano"><part-name>Piano</part-name></score-part></part-list>
<part id="Piano">
<measure number="1"><direction><direction-type><words>one</words></direction-type></direction>
<note><rest/><duration>1</duration></note></measure>
<measure number="1"><note><rest/><duration>1</duration></note></measure>
</part></score-partwise>"#;
        let first = import_musicxml(source).unwrap();
        let second = import_musicxml(source).unwrap();
        assert_eq!(first, second);

        let first_measure = &first.parts[0].measures[0];
        let second_measure = &first.parts[0].measures[1];
        let ScoreEvent::Note(first_note) = &first_measure.events[1] else {
            panic!("first measure note expected");
        };
        let ScoreEvent::Note(second_note) = &second_measure.events[0] else {
            panic!("second measure note expected");
        };
        assert_eq!(first_note.id.measure_ordinal, 0);
        assert_eq!(second_note.id.measure_ordinal, 1);
        assert_eq!(first_note.id.ordinal, 0);
        assert_eq!(second_note.id.ordinal, 0);
        assert_eq!(first_note.id.part_id, "Piano");
    }
}
