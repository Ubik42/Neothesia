//! A notation-neutral score model and a deliberately small MusicXML importer.
//!
//! This module preserves learning semantics that MIDI cannot express reliably.
//! It is not an engraving engine: renderers should consume [`Score`] instead of
//! depending on MusicXML's document structure.

use std::{
    collections::HashMap,
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
    pub events: Vec<ScoreEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScoreEvent {
    Note(Note),
    Direction(Direction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub onset: ScoreTime,
    pub duration: ScoreTime,
    pub voice: Option<String>,
    pub staff: u8,
    pub pitch: Option<Pitch>,
    pub chord: bool,
    pub grace: bool,
    pub ties: Vec<SpanType>,
    pub slurs: Vec<NumberedSpan>,
    pub fingering: Option<String>,
    pub articulations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Direction {
    pub onset: ScoreTime,
    pub staff: Option<u8>,
    pub dynamics: Vec<String>,
    pub words: Vec<String>,
    pub tempo_bpm: Option<String>,
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

    fn add(self, other: Self) -> Self {
        Self::new(
            self.numerator * i64::from(other.denominator)
                + other.numerator * i64::from(self.denominator),
            self.denominator * other.denominator,
        )
    }

    fn subtract(self, other: Self) -> Self {
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
    let mut attributes: Option<AttributesBuilder> = None;
    let mut divisions = 1_u32;
    let mut cursor = ScoreTime::default();
    let mut measure_max = ScoreTime::default();
    let mut part_position = ScoreTime::default();
    let mut previous_note_onset = ScoreTime::default();
    let mut backup_duration: Option<i64> = None;
    let mut forward_duration: Option<i64> = None;
    let mut score_part_id: Option<String> = None;
    let mut creator_is_composer = false;

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
                        current_measure = Some(Measure {
                            number: attribute(&start, b"number")?.unwrap_or_default(),
                            implicit: attribute(&start, b"implicit")?.as_deref() == Some("yes"),
                            start: part_position,
                            duration: ScoreTime::default(),
                            attributes: Vec::new(),
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
                    b"direction" => direction = Some(DirectionBuilder::default()),
                    b"attributes" => attributes = Some(AttributesBuilder::default()),
                    b"clef" if attributes.is_some() => {
                        attributes.as_mut().unwrap().clefs.push(ClefBuilder {
                            staff: attribute(&start, b"number")?
                                .map(|value| parse_u8("clef number", &value))
                                .transpose()?
                                .unwrap_or(1),
                            ..Default::default()
                        });
                    }
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
                    b"sound" if direction.is_some() => {
                        direction.as_mut().unwrap().tempo_bpm = attribute(&start, b"tempo")?;
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
                    b"transpose" | b"time-modification" | b"tuplet" | b"ornaments" | b"pedal"
                    | b"wedge" => {
                        let location = current_measure
                            .as_ref()
                            .map(|measure| format!("measure {}", measure.number))
                            .unwrap_or_else(|| "score".into());
                        warn_once(
                            &mut score.warnings,
                            location,
                            format!(
                                "{} is preserved neither in the feasibility model nor playback yet",
                                str::from_utf8(&name)?
                            ),
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
                handle_empty(&start, &path, &mut note, &mut direction)?;
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
                                onset: cursor,
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
                        note.as_mut().unwrap().fingering = nonempty(value)
                    }
                    b"words" if direction.is_some() => {
                        if let Some(value) = nonempty(value) {
                            direction.as_mut().unwrap().words.push(value);
                        }
                    }
                    b"offset" if direction.is_some() => {
                        direction.as_mut().unwrap().offset = parse_i64("direction offset", &value)?
                    }
                    b"note" => {
                        let built = note.take().unwrap();
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
                                onset: part_position.add(onset),
                                duration,
                                voice: built.voice,
                                staff: built.staff,
                                pitch,
                                chord: built.chord,
                                grace: built.grace,
                                ties: built.ties,
                                slurs: built.slurs,
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
                        let built = direction.take().unwrap();
                        let onset = cursor.add(ScoreTime::new(built.offset, divisions));
                        current_measure
                            .as_mut()
                            .unwrap()
                            .events
                            .push(ScoreEvent::Direction(Direction {
                                onset: part_position.add(onset),
                                staff: built.staff,
                                dynamics: built.dynamics,
                                words: built.words,
                                tempo_bpm: built.tempo_bpm,
                            }));
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
            direction.as_mut().unwrap().tempo_bpm = attribute(start, b"tempo")?;
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
        assert!(score.warnings.iter().any(|warning| {
            warning
                .message
                .starts_with("ornaments is preserved neither")
        }));

        let measure = &score.parts[0].measures[0];
        assert_eq!(measure.duration, ScoreTime::new(2, 1));
        assert_eq!(measure.attributes[0].staves, Some(2));
        assert_eq!(measure.attributes[0].clefs.len(), 2);

        let ScoreEvent::Direction(direction) = &measure.events[0] else {
            panic!("direction should be preserved");
        };
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
}
