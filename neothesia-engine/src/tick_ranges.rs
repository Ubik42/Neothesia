use super::*;
use crate::preferences::{PassagePreset, SessionSettings};
use serde_json::{Value, json};
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TickRange {
    #[serde(default)]
    pub description: String,
    pub content_id: String,
    pub grid: String,
    pub start_tick: u64,
    pub end_tick: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BeatPoint {
    pub measure: usize,
    pub beat: f64,
}
impl TickRange {
    pub fn validate(&self) -> Result<(), String> {
        if self.content_id.len() != 64
            || !self.content_id.bytes().all(|c| c.is_ascii_hexdigit())
            || !self.grid.starts_with("grid-v1-")
            || self.grid.len() > 80
            || self.start_tick >= self.end_tick
            || self.description.len() > 1024
            || self.end_tick > 1_000_000_000_000
        {
            return Err("拍内范围资料无效".into());
        }
        Ok(())
    }
}
fn beat_length(file: &MidiFile, m: &midi_file::musical_time::Measure) -> f64 {
    let group = if m.denominator == 8 && m.numerator >= 6 && m.numerator % 3 == 0 {
        3.
    } else {
        1.
    };
    f64::from(file.musical_time.ppq) * 4. / f64::from(m.denominator) * group
}
impl Player {
    fn tick_point(&self, point: &BeatPoint) -> Result<u64, String> {
        let f = self.file.as_ref().ok_or("请先选择曲目")?;
        let m = f
            .musical_time
            .measures
            .get(point.measure.checked_sub(1).ok_or("小节从 1 开始")?)
            .ok_or("小节超出曲目")?;
        let limit = m.end_tick.min(
            f.tempo_track
                .seconds_to_pulses(f.duration.as_secs_f64())
                .round() as u64,
        );
        let tick = m.start_tick as f64 + (point.beat - 1.) * beat_length(f, m);
        if !point.beat.is_finite() || point.beat < 1. || tick > limit as f64 + 0.000001 {
            return Err("拍位超出该小节的实际范围".into());
        }
        Ok(tick.round() as u64)
    }
    pub(super) fn preview_tick_range(
        &self,
        cid: &str,
        grid: &str,
        start: &BeatPoint,
        end: &BeatPoint,
    ) -> Result<Value, String> {
        let range = TickRange {
            description: String::new(),
            content_id: cid.into(),
            grid: grid.into(),
            start_tick: self.tick_point(start)?,
            end_tick: self.tick_point(end)?,
        };
        self.validate_tick_range(&range)
    }
    pub(super) fn validate_tick_range(&self, range: &TickRange) -> Result<Value, String> {
        range.validate()?;
        let f = self.file.as_ref().ok_or("请先选择曲目")?;
        if f.content_id != range.content_id || self.grid_signature() != range.grid {
            return Err("曲目或小节网格已改变，请重新核对拍内范围".into());
        }
        let start = f
            .tempo_track
            .pulses_to_duration(range.start_tick)
            .as_secs_f64();
        let end = f
            .tempo_track
            .pulses_to_duration(range.end_tick)
            .as_secs_f64();
        if end - start < 0.1 || end > f.duration.as_secs_f64() + 0.000001 {
            return Err("范围至少需要 0.1 秒，并位于实际曲目内".into());
        }
        let point = |tick: u64, is_end: bool| -> Result<Value, String> {
            let index = f
                .musical_time
                .measures
                .partition_point(|m| {
                    if is_end {
                        m.start_tick < tick
                    } else {
                        m.start_tick <= tick
                    }
                })
                .checked_sub(1)
                .ok_or("范围位置无效")?;
            let m = &f.musical_time.measures[index];
            if tick > m.end_tick {
                return Err("范围位置超出网格".into());
            }
            Ok(json!({"measure":index+1,"beat":1.+(tick-m.start_tick)as f64/beat_length(f,m)}))
        };
        let a = point(range.start_tick, false)?;
        let b = point(range.end_tick, true)?;
        let label = |p: &Value| {
            format!(
                "第 {} 小节第 {:.3} 拍",
                p["measure"],
                p["beat"].as_f64().unwrap()
            )
        };
        let notes = self
            .notes
            .iter()
            .filter(|n| n.start.as_secs_f64() >= start && n.start.as_secs_f64() < end)
            .count();
        let targets = self
            .notes
            .iter()
            .filter(|n| {
                n.start.as_secs_f64() >= start
                    && n.start.as_secs_f64() < end
                    && self.note_player(n.track_id, n.index) == PlayerConfig::Human
            })
            .count();
        let carry = self
            .notes
            .iter()
            .filter(|n| n.start.as_secs_f64() < start && n.end.as_secs_f64() > start)
            .count();
        let description = format!("{} → {}", label(&a), label(&b));
        let mut canonical = range.clone();
        canonical.description = description.clone();
        Ok(
            json!({"range":canonical,"start":start,"end":end,"first":a,"last":b,"description":description,"notes":notes,"targets":targets,"carry":carry}),
        )
    }
    pub(super) fn tick_range_for_current_loop(&self) -> Option<TickRange> {
        let p = self.state.passage.as_ref()?;
        let f = self.file.as_ref()?;
        let r = TickRange {
            description: String::new(),
            content_id: f.content_id.clone(),
            grid: self.grid_signature(),
            start_tick: f.tempo_track.seconds_to_pulses(p.start).round() as u64,
            end_tick: f.tempo_track.seconds_to_pulses(p.end).round() as u64,
        };
        let f = self.file.as_ref()?;
        if f.measures
            .iter()
            .any(|m| (m.as_secs_f64() - p.start).abs() < 0.000001)
            && f.measures
                .iter()
                .any(|m| (m.as_secs_f64() - p.end).abs() < 0.000001)
        {
            return None;
        }
        serde_json::from_value(self.validate_tick_range(&r).ok()?["range"].clone()).ok()
    }
    pub(super) fn apply_tick_range(&mut self, range: &TickRange) -> Result<Value, String> {
        let v = self.validate_tick_range(range)?;
        self.command(Command::Loop {
            start: v["start"].as_f64().unwrap(),
            end: v["end"].as_f64().unwrap(),
            enabled: true,
        })
    }
    pub(super) fn save_tick_passage(
        &mut self,
        id: Option<String>,
        name: String,
        notes: String,
        range: TickRange,
    ) -> Result<Value, String> {
        if self.state.recording
            || self.routine.is_some()
            || self.ladder.as_ref().is_some_and(|l| l.active)
            || matches!(self.state.status.as_str(), "playing" | "countIn")
            || !self.state.pressed.is_empty()
        {
            return Err("请先停止演奏、录音和活动计划，再保存段落".into());
        }
        if is_recital_mode(&self.state.mode) {
            return Err("拍内段落请切换到连续、等音或聆听模式".into());
        }
        let v = self.validate_tick_range(&range)?;
        if name.trim().is_empty() || name.chars().count() > 80 || notes.len() > 8192 {
            return Err("名称需要 1～80 个字，备注最多 8192 字节".into());
        }
        let cid = range.content_id.clone();
        if id.as_ref().is_some_and(|id| {
            !self
                .preferences
                .passages
                .get(&cid)
                .is_some_and(|ps| ps.iter().any(|p| &p.id == id))
        }) {
            return Err("原练习段已不存在，请重新读取".into());
        }
        let preset = PassagePreset {
            id: id.unwrap_or_else(|| {
                format!(
                    "passage-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos()
                )
            }),
            name: name.trim().into(),
            notes,
            start: v["first"]["measure"].as_u64().unwrap() as usize,
            end: v["last"]["measure"].as_u64().unwrap() as usize,
            grid: Some(range.grid.clone()),
            score_range: None,
            precise_range: Some(
                serde_json::from_value(v["range"].clone()).map_err(|e| e.to_string())?,
            ),
            speed: self.state.speed,
            settings: SessionSettings {
                tracks: self.config.practice_track_setup(),
                parts: self
                    .config
                    .tracks
                    .iter()
                    .map(|t| (t.track_id, t.practice_part))
                    .collect(),
                mode: Some(self.state.mode.clone()),
                hands: self.state.hands.clone(),
                count_in: self.state.count_in,
                metronome: self.state.metronome,
                latency: self.state.latency,
                rounds: self.state.rounds,
                adaptive: self.state.adaptive,
            },
        };
        let mut prefs = self.preferences.clone();
        let ps = prefs.passages.entry(cid).or_default();
        if let Some(old) = ps.iter_mut().find(|p| p.id == preset.id) {
            *old = preset.clone()
        } else {
            if ps.len() >= 200 {
                return Err("每曲最多 200 个段落".into());
            }
            ps.push(preset.clone())
        }
        prefs.save(&self.preferences_path)?;
        self.preferences = prefs;
        Ok(json!({"saved":true,"passage":preset,"preview":v}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::practice_backup::{Backup, Selection};
    use crate::routine_commands::{RoutineGoal, RoutineTarget};
    fn player() -> Player {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work")
            .join(format!(
                "cycle181-unit-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut p = Player::new(data, PathBuf::new(), true);
        let notes=["C","D","E","F"].iter().map(|s|format!("<note><pitch><step>{s}</step><octave>4</octave></pitch><duration>1</duration></note>")).collect::<String>();
        let xml = format!(
            "<score-partwise><part-list><score-part id='P1'><part-name>Piano</part-name></score-part></part-list><part id='P1'><measure number='1'><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><direction><sound tempo='120'/></direction>{notes}</measure><measure number='2'><direction><sound tempo='60'/></direction>{notes}</measure></part></score-partwise>"
        );
        p.command(Command::ImportScore {
            bytes: xml.into_bytes(),
            name: "beats.musicxml".into(),
            default_bpm: 120,
        })
        .unwrap();
        p.command(Command::Mode {
            value: "wait".into(),
        })
        .unwrap();
        p.command(Command::Track {
            id: p.notes[0].track_id,
            mode: "human".into(),
            part: "right".into(),
            visible: true,
        })
        .unwrap();
        p.command(Command::CountIn { bars: 0 }).unwrap();
        p
    }
    fn range(p: &Player) -> TickRange {
        serde_json::from_value(
            p.preview_tick_range(
                &p.file.as_ref().unwrap().content_id,
                &p.grid_signature(),
                &BeatPoint {
                    measure: 1,
                    beat: 3.5,
                },
                &BeatPoint {
                    measure: 2,
                    beat: 2.5,
                },
            )
            .unwrap()["range"]
                .clone(),
        )
        .unwrap()
    }
    #[test]
    fn precise_range_tempo_restart_daily_and_portable_roundtrip() {
        let mut p = player();
        let r = range(&p);
        let v = p.validate_tick_range(&r).unwrap();
        assert_eq!(v["start"], 1.25);
        assert_eq!(v["end"], 3.5);
        assert_eq!(v["targets"], 3);
        p.state.speed = 0.7;
        let saved = p
            .save_tick_passage(None, "跨速度短段".into(), "保持节奏".into(), r.clone())
            .unwrap();
        let id = saved["passage"]["id"].as_str().unwrap().to_string();
        p.command(Command::OpenPassage { id: id.clone() }).unwrap();
        assert_eq!(p.state.passage.as_ref().unwrap().start, 1.25);
        assert_eq!(p.state.passage.as_ref().unwrap().end, 3.5);
        assert_eq!(p.state.speed, 0.7);
        let goal = RoutineGoal {
            expression: None,
            passes: 1,
            accuracy: 90,
            on_time: None,
            consecutive: false,
            attempt_limit: 5,
        };
        let plan = p
            .save_routine(None, None, "日课".into(), "".into(), None)
            .unwrap();
        let rid = plan["id"].as_str().unwrap();
        let item = p
            .save_routine_item(
                rid,
                None,
                None,
                Some("passage"),
                Some(&id),
                "拍内".into(),
                "".into(),
                goal.clone(),
            )
            .unwrap();
        assert!(matches!(
            p.preferences.routines[0].items[0].target,
            RoutineTarget::PrecisePassage { .. }
        ));
        p.save_routine_item(
            rid,
            None,
            None,
            Some("current"),
            None,
            "当前拍内".into(),
            "".into(),
            goal,
        )
        .unwrap();
        assert!(matches!(
            p.preferences.routines[0].items[1].target,
            RoutineTarget::PrecisePassage { .. }
        ));
        let b:Backup = serde_json::from_value(p.practice_backup_snapshot(&Selection::default()).unwrap()).unwrap();
        assert_eq!(b.version(), 10);
        let b = Backup::decode(&b.encode().unwrap()).unwrap();
        assert_eq!(
            b.passages.values().next().unwrap()[0].precise_range,
            Some(r.clone())
        );
        let bytes: Vec<u8> =
            serde_json::from_value(p.command(Command::ExportPackage).unwrap()["bytes"].clone())
                .unwrap();
        let pack = crate::piece_package::Package::decode(&bytes).unwrap();
        assert_eq!(pack.manifest.version, 7);
        assert_eq!(pack.manifest.passages[0].precise_range, Some(r.clone()));
        let mut target = player();
        target
            .command(Command::ImportPackage {
                bytes,
                policy: "replace".into(),
            })
            .unwrap();
        target
            .command(Command::OpenPassage { id: id.clone() })
            .unwrap();
        assert_eq!(target.state.passage.as_ref().unwrap().start, 1.25);
        let data = p.data.clone();
        let path = p.file.as_ref().unwrap().source_path.clone().unwrap();
        drop(p);
        let mut p = Player::new(data, PathBuf::new(), true);
        p.command(Command::Load {
            path,
            title: "reopen".into(),
        })
        .unwrap();
        p.command(Command::OpenPassage { id }).unwrap();
        assert_eq!(p.state.passage.as_ref().unwrap().end, 3.5);
        p.open_routine_item(rid, "2026-10-02", item["id"].as_str().unwrap(), true)
            .unwrap();
        p.command(Command::Play).unwrap();
        for _ in 0..500 {
            p.tick(Duration::from_millis(10));
            for pitch in p.snapshot().required {
                p.command(Command::Note {
                    pitch,
                    velocity: 90,
                    active: true,
                })
                .unwrap();
                p.command(Command::Note {
                    pitch,
                    velocity: 0,
                    active: false,
                })
                .unwrap();
            }
            if p.routine_status().unwrap()["progress"]["completed"] == true {
                break;
            }
        }
        assert_eq!(p.routine_status().unwrap()["progress"]["completed"], true);
        assert_eq!(p.matcher.snapshot().matched_notes, 3);
    }
    #[test]
    fn precise_guards_end_exclusion_and_meter() {
        let mut p = player();
        let r = range(&p);
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let grid = p.grid_signature();
        let endpoint = p
            .preview_tick_range(
                &cid,
                &grid,
                &BeatPoint {
                    measure: 1,
                    beat: 2.,
                },
                &BeatPoint {
                    measure: 1,
                    beat: 3.,
                },
            )
            .unwrap();
        assert_eq!(endpoint["targets"], 1);
        let original = p.state.position;
        for beat in [f64::NAN, 0., 6.] {
            assert!(
                p.preview_tick_range(
                    &cid,
                    &grid,
                    &BeatPoint { measure: 1, beat },
                    &BeatPoint {
                        measure: 2,
                        beat: 2.
                    }
                )
                .is_err()
            )
        }
        assert!(
            p.preview_tick_range(
                &cid,
                &grid,
                &BeatPoint {
                    measure: 2,
                    beat: 2.
                },
                &BeatPoint {
                    measure: 1,
                    beat: 2.
                }
            )
            .is_err()
        );
        assert_eq!(p.state.position, original);
        p.command(Command::Meter {
            value: Some(neothesia_core::library::MeterCorrection {
                numerator: 6,
                denominator: 8,
                pickup_ticks: 0,
            }),
        })
        .unwrap();
        assert!(
            p.save_tick_passage(None, "旧范围".into(), "".into(), r)
                .is_err()
        );
        assert!(p.preferences.passages.is_empty());
        let c = p
            .preview_tick_range(
                &cid,
                &p.grid_signature(),
                &BeatPoint {
                    measure: 1,
                    beat: 1.5,
                },
                &BeatPoint {
                    measure: 1,
                    beat: 2.,
                },
            )
            .unwrap();
        assert_eq!(
            c["range"]["startTick"].as_u64().unwrap(),
            u64::from(p.file.as_ref().unwrap().musical_time.ppq) * 3 / 4
        );
        let range: TickRange = serde_json::from_value(c["range"].clone()).unwrap();
        p.preferences_path = p.data.clone();
        assert!(
            p.save_tick_passage(None, "写入失败".into(), "".into(), range)
                .is_err()
        );
        assert!(p.preferences.passages.is_empty());
    }
}
