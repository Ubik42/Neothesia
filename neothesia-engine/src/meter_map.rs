use super::*;
use midi_file::musical_time::MusicalTime;
use neothesia_core::library::{MeterMap, MeterSegment, SongSidecar};
impl Player {
    pub(crate) fn mapped_grid(file: &MidiFile, map: &MeterMap) -> Result<MusicalTime, String> {
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
        let segments: Vec<_> = map
            .segments
            .iter()
            .map(|s| (s.measure, s.numerator, s.denominator, s.display_start))
            .collect();
        let lengths: Vec<_> = map.measure_lengths.iter().map(|s| (s.measure, s.ticks)).collect();
        MusicalTime::segmented_with_lengths(file.musical_time.ppq, end, map.pickup_ticks, &segments, &lengths)
    }
    pub(crate) fn sidecar_grid(file: &MidiFile, s: &SongSidecar) -> Result<MusicalTime, String> {
        if let Some(m) = &s.meter_map {
            Self::mapped_grid(file, m)
        } else if let Some(m) = s.meter {
            Self::corrected_grid(file, m)
        } else {
            Ok(file.musical_time.clone())
        }
    }
    pub(super) fn current_meter_map(&self) -> Option<MeterMap> {
        self.meter_map.clone().or_else(|| {
            self.meter_correction.map(|m| MeterMap {
                pickup_ticks: m.pickup_ticks,
                measure_lengths: vec![],
                segments: vec![MeterSegment {
                    measure: 1,
                    numerator: m.numerator,
                    denominator: m.denominator,
                    display_start: Some(1),
                }],
            })
        })
    }
    fn meter_map_source(&self, content_id: &str) -> Result<(PathBuf, SongSidecar), String> {
        self.score_content_guard(content_id)?;
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        let path = self.annotation_path(file)?;
        if self.exercise.is_none() {
            let actual = MidiFile::new(&path).map_err(|e| e.to_string())?;
            if actual.content_id != file.content_id {
                return Err("曲目文件已改变，请重新打开曲目后编辑小节".into());
            }
        }
        let sidecar = if neothesia_core::library::metadata_sidecar_path(&path).exists() {
            neothesia_core::library::load_song_sidecar(&path, content_id)
                .map_err(|e| e.to_string())?
        } else {
            SongSidecar::default()
        };
        if sidecar.meter_map != self.meter_map || sidecar.meter != self.meter_correction {
            return Err("小节资料已被其他窗口修改，请重新打开曲目后编辑".into());
        }
        Ok((path, sidecar))
    }
    pub(super) fn preview_meter_map(
        &self,
        content_id: &str,
        value: Option<&MeterMap>,
    ) -> Result<serde_json::Value, String> {
        let (_, sidecar) = self.meter_map_source(content_id)?;
        let file = self.file.as_ref().unwrap();
        let grid = match value {
            Some(m) => Self::mapped_grid(file, m)?,
            None => self.original_grid.clone().ok_or("原始小节网格不存在")?,
        };
        let proof = blake3::hash(
            &serde_json::to_vec(&serde_json::json!([
                content_id,
                self.grid_signature(),
                self.current_meter_map(),
                sidecar.meter,
                sidecar.meter_map,
                value
            ]))
            .map_err(|e| e.to_string())?,
        )
        .to_hex()
        .to_string();
        let measures:Vec<_>=grid.measures.iter().map(|m|serde_json::json!({"number":m.number,"displayNumber":m.display_number,"start":file.tempo_track.pulses_to_duration(m.start_tick).as_secs_f64(),"end":file.tempo_track.pulses_to_duration(m.end_tick).as_secs_f64(),"startTick":m.start_tick,"endTick":m.end_tick,"numerator":m.numerator,"denominator":m.denominator,"explicit":m.explicit_meter,"partial":m.partial})).collect();
        Ok(serde_json::json!({"proof":proof,"value":value,"measures":measures}))
    }
    pub(super) fn save_meter_map(
        &mut self,
        content_id: &str,
        value: Option<MeterMap>,
        proof: &str,
    ) -> Result<serde_json::Value, String> {
        let lock = library_workspace::lock_for(&self.data)?;
        let _guard = lock.lock().map_err(|_| "曲目资料正在使用")?;
        let preview = self.preview_meter_map(content_id, value.as_ref())?;
        if preview["proof"].as_str() != Some(proof) {
            return Err("小节设置或曲目资料已经改变，请重新预览后保存".into());
        }
        let (path, mut sidecar) = self.meter_map_source(content_id)?;
        let file = self.file.as_ref().unwrap();
        let grid = match &value {
            Some(m) => Self::mapped_grid(file, m)?,
            None => self.original_grid.clone().ok_or("原始小节网格不存在")?,
        };
        let mut updated = file.clone();
        updated.measures = grid.bar_times(&updated.tempo_track).into();
        updated.beats = grid.beat_times(&updated.tempo_track).into();
        updated.musical_time = grid;
        let notation = match &self.notation {
            Some(asset) => Some(notation::ScoreAsset::new(
                asset.bytes.clone(),
                asset.name.clone(),
                &updated,
            )?),
            None => None,
        };
        sidecar.meter = None;
        sidecar.meter_map = value.clone();
        neothesia_core::library::save_song_sidecar(&path, content_id, sidecar)
            .map_err(|e| e.to_string())?;
        self.panic();
        self.state.status = "paused".into();
        self.state.passage = None;
        self.previous_summary = None;
        self.previous_mode = None;
        self.file = Some(updated);
        self.meter_correction = None;
        self.meter_map = value;
        self.notation = notation;
        self.score_revision = self.score_revision.wrapping_add(1);
        self.coach = AdaptiveTempoCoach::default();
        self.reset(0.);
        self.command(Command::CurrentSong)
    }
}
