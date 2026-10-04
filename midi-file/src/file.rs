use crate::{MidiTrack, program_track::ProgramTrack, tempo_track::TempoTrack};
use midly::{Format, Smf, Timing};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub struct MidiFile {
    pub name: String,
    /// Stable content identity; moving or renaming the source file does not
    /// change this value.
    pub content_id: String,
    pub source_path: Option<PathBuf>,
    pub format: Format,
    pub tracks: Arc<[MidiTrack]>,
    pub program_track: ProgramTrack,
    pub tempo_track: TempoTrack,
    pub measures: Arc<[std::time::Duration]>,
    pub beats: Arc<[std::time::Duration]>,
    pub duration: std::time::Duration,
    pub musical_time: crate::musical_time::MusicalTime,
}

impl MidiFile {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let name = path
            .as_ref()
            .file_name()
            .ok_or(String::from("File not found"))?
            .to_string_lossy()
            .to_string();

        let data = match fs::read(path.as_ref()) {
            Ok(buff) => buff,
            Err(_) => return Err(String::from("Could Not Open File")),
        };

        let content_id = blake3::hash(&data).to_hex().to_string();
        let smf = match Smf::parse(&data) {
            Ok(smf) => smf,
            Err(_) => return Err(String::from("Midi Parsing Error (midly lib)")),
        };

        let mut file = Self::from_parsed_smf(name, content_id, &smf)?;
        file.source_path = Some(path.as_ref().to_path_buf());
        Ok(file)
    }

    pub fn from_smf(name: impl Into<String>, smf: &Smf<'_>) -> Result<Self, String> {
        let mut data = Vec::new();
        smf.write_std(&mut data)
            .map_err(|_| String::from("MIDI Serialization Error"))?;
        let content_id = blake3::hash(&data).to_hex().to_string();
        Self::from_parsed_smf(name.into(), content_id, smf)
    }

    fn from_parsed_smf(name: String, content_id: String, smf: &Smf<'_>) -> Result<Self, String> {
        let u_per_quarter_note: u16 = match smf.header.timing {
            Timing::Metrical(t) => t.as_int(),
            Timing::Timecode(_fps, _u) => {
                return Err(String::from("Midi With Timecode Timing, Not Supported!"));
            }
        };

        if smf.tracks.is_empty() {
            return Err(String::from("Midi File Has No Tracks"));
        }

        if u_per_quarter_note == 0 {
            return Err("MIDI 每拍分辨率不能为零".into());
        }
        if smf.tracks.iter().flatten().any(|e|matches!(e.kind,midly::TrackEventKind::Meta(midly::MetaMessage::Tempo(t)) if t.as_int()==0)){return Err("MIDI 速度不能为零".into())}
        let tempo_track = TempoTrack::build(&smf.tracks, u_per_quarter_note);

        let mut track_color_id = 0;
        let tracks: Vec<MidiTrack> = smf
            .tracks
            .iter()
            .enumerate()
            .map(|(id, events)| {
                let track = MidiTrack::new(id, track_color_id, &tempo_track, events);

                if !track.notes.is_empty() {
                    track_color_id += 1;
                }

                track
            })
            .collect();

        let last_tick = smf
            .tracks
            .iter()
            .map(|track| {
                track
                    .iter()
                    .map(|event| u64::from(event.delta.as_int()))
                    .sum()
            })
            .max()
            .unwrap_or(0);
        let duration = tempo_track.pulses_to_duration(last_tick);
        let musical_time = crate::musical_time::MusicalTime::build(
            &smf.tracks,
            u_per_quarter_note,
            smf.tracks
                .iter()
                .map(|track| {
                    track
                        .iter()
                        .map(|event| u64::from(event.delta.as_int()))
                        .sum()
                })
                .max()
                .unwrap_or(0),
        )?;
        let measures = musical_time.bar_times(&tempo_track);
        let beats = musical_time.beat_times(&tempo_track);
        let program_track = ProgramTrack::new(&tracks);

        Ok(Self {
            name,
            content_id,
            source_path: None,
            format: smf.header.format,
            tracks: tracks.into(),
            program_track,
            tempo_track,
            measures: measures.into(),
            beats: beats.into(),
            musical_time,
            duration,
        })
    }
}
