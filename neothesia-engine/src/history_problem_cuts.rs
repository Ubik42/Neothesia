use super::*;
use crate::history_problem_plan::{Built, ProblemPlanEdit};
use crate::tick_ranges::{BeatPoint, TickRange};
use midi_file::{musical_time::Measure, tempo_track::TempoTrack};
use serde_json::{Value, json};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProblemCut {
    pub start: BeatPoint,
    pub end: BeatPoint,
}
pub(super) struct CutContext {
    cid: String,
    grid: String,
    ppq: u16,
    bars: Vec<Measure>,
    tempo: TempoTrack,
    first: u64,
    last: u64,
    anchors: Vec<u64>,
}
impl CutContext {
    pub(super) fn new(
        file: &MidiFile,
        grid: &str,
        start: usize,
        end: usize,
        anchors: Vec<f64>,
    ) -> Result<Self, String> {
        let bars = file
            .musical_time
            .measures
            .get(start - 1..end)
            .ok_or("编排小节范围无效")?
            .to_vec();
        let first = bars[0].start_tick;
        let last = bars.last().unwrap().end_tick.min(
            file.tempo_track
                .seconds_to_pulses(file.duration.as_secs_f64())
                .round() as u64,
        );
        if first >= last {
            return Err("问题范围超出实际曲目".into());
        }
        Ok(Self {
            cid: file.content_id.clone(),
            grid: grid.into(),
            ppq: file.musical_time.ppq,
            bars,
            tempo: file.tempo_track.clone(),
            first,
            last,
            anchors: anchors
                .into_iter()
                .map(|s| file.tempo_track.seconds_to_pulses(s).round() as u64)
                .collect(),
        })
    }
    fn length(&self, m: &Measure) -> f64 {
        let group = if m.denominator == 8 && m.numerator >= 6 && m.numerator % 3 == 0 {
            3.
        } else {
            1.
        };
        f64::from(self.ppq) * 4. / f64::from(m.denominator) * group
    }
    fn point(&self, tick: u64, end: bool) -> Result<BeatPoint, String> {
        let m = self
            .bars
            .iter()
            .rev()
            .find(|m| {
                if end {
                    m.start_tick < tick
                } else {
                    m.start_tick <= tick
                }
            })
            .ok_or("拍内位置无效")?;
        Ok(BeatPoint {
            measure: m.number,
            beat: 1. + (tick - m.start_tick) as f64 / self.length(m),
        })
    }
    fn tick(&self, p: &BeatPoint) -> Result<u64, String> {
        let m = self
            .bars
            .iter()
            .find(|m| m.number == p.measure)
            .ok_or("拍内范围不能超出已预览的上下文小节")?;
        let tick = m.start_tick as f64 + (p.beat - 1.) * self.length(m);
        if !p.beat.is_finite() || p.beat < 1. || tick > m.end_tick.min(self.last) as f64 + 0.000001
        {
            return Err("拍位超出小节实际范围".into());
        }
        Ok(tick.round() as u64)
    }
    pub(super) fn bounds(&self) -> Result<Value, String> {
        Ok(
            json!({"first":self.point(self.first,false)?,"last":self.point(self.last,true)?,"measures":self.bars.iter().map(|m|json!({"number":m.number,"maxBeat":1.+(m.end_tick.min(self.last)-m.start_tick)as f64/self.length(m),"numerator":m.numerator,"denominator":m.denominator})).collect::<Vec<_>>(),"anchors":self.anchors.iter().map(|t|self.point(*t,false)).collect::<Result<Vec<_>,_>>()?}),
        )
    }
    fn preview(&self, cut: &ProblemCut) -> Result<(TickRange, ProblemCut), String> {
        let start = self.tick(&cut.start)?;
        let end = self.tick(&cut.end)?;
        if start < self.first
            || end > self.last
            || start >= end
            || self.tempo.pulses_to_duration(end).as_secs_f64()
                - self.tempo.pulses_to_duration(start).as_secs_f64()
                < 0.1
        {
            return Err("拍内范围须位于预览的小节内，且至少 0.1 秒".into());
        }
        if self.anchors.iter().any(|t| *t < start || *t >= end) {
            return Err(
                "裁剪范围必须包含全部所选问题音的起音；请扩大范围或移除范围外的问题音".into(),
            );
        }
        let a = self.point(start, false)?;
        let b = self.point(end, true)?;
        let label = |p: &BeatPoint| format!("第 {} 小节第 {:.3} 拍", p.measure, p.beat);
        let r = TickRange {
            content_id: self.cid.clone(),
            grid: self.grid.clone(),
            start_tick: start,
            end_tick: end,
            description: format!("{} → {}", label(&a), label(&b)),
        };
        r.validate()?;
        Ok((r, ProblemCut { start: a, end: b }))
    }
}
impl Player {
    pub(super) fn apply_problem_cuts(
        &self,
        mut built: Built,
        expected: &str,
        edits: &[ProblemPlanEdit],
    ) -> Result<Built, String> {
        if built.revision != expected {
            return Err("演奏证据、条件或目标计划已改变，请重新预览".into());
        }
        if edits.len() != built.items.len()
            || edits
                .iter()
                .map(|e| &e.key)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != edits.len()
        {
            return Err("编排项目已改变，请重新预览".into());
        }
        for (row, item) in built.rows.iter_mut().zip(&mut built.items) {
            let edit = edits
                .iter()
                .find(|e| e.key == item.id)
                .ok_or("项目范围已改变")?;
            if let Some(cut) = &edit.cut {
                let ctx = built.cuts.get(&item.id).ok_or("项目拍位资料缺失")?;
                let (r, normalized) = ctx
                    .preview(cut)
                    .map_err(|e| format!("{} · {}：{}", item.song_title, item.title, e))?;
                row["cut"] = json!(normalized);
                row["cutDescription"] = json!(r.description);
                item.target = crate::routine_commands::RoutineTarget::PrecisePassage { range: r };
            }
        }
        Ok(built)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_multi_plan::{HistoryProblemSelection, MultiProblemSpec};
    use crate::history_problem_plan::{ProblemPlanEdit, ProblemPlanSpec};
    use crate::practice_backup::{Backup, Selection};
    use crate::routine_commands::{RoutineGoal, RoutineTarget};
    fn player() -> Player {
        Player::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../work")
                .join(format!(
                    "cycle182-unit-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                )),
            PathBuf::new(),
            true,
        )
    }
    fn record(p: &mut Player, step: &str, bpm: u16, hand: &str) -> HistoryProblemSelection {
        if p.file.is_some() {
            p.command(Command::Restart).unwrap();
        }
        let notes=(0..4).map(|_|format!("<note><pitch><step>{step}</step><octave>4</octave></pitch><duration>1</duration></note>")).collect::<String>();
        let xml = format!(
            "<score-partwise><part-list><score-part id='P1'><part-name>Piano</part-name></score-part></part-list><part id='P1'><measure number='1'><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><direction><sound tempo='{bpm}'/></direction>{notes}</measure><measure number='2'><direction><sound tempo='60'/></direction>{notes}</measure></part></score-partwise>"
        );
        p.command(Command::ImportScore {
            bytes: xml.into_bytes(),
            name: format!("{step}.musicxml"),
            default_bpm: 120,
        })
        .unwrap();
        p.command(Command::Restart).unwrap();
        p.command(Command::Track {
            id: p.notes[0].track_id,
            mode: "human".into(),
            part: hand.into(),
            visible: true,
        })
        .unwrap();
        p.command(Command::CountIn { bars: 0 }).unwrap();
        p.command(Command::Mode {
            value: "recital".into(),
        })
        .unwrap();
        p.command(Command::Play).unwrap();
        for _ in 0..220 {
            p.tick(Duration::from_millis(50));
        }
        assert_eq!(p.state.status, "finished");
        let cid = p.file.as_ref().unwrap().content_id.clone();
        let id = p
            .history
            .song(&cid)
            .unwrap()
            .sessions
            .last()
            .unwrap()
            .stable_id();
        HistoryProblemSelection {
            content_id: cid,
            id,
            references: vec![2, 3],
        }
    }
    fn goal() -> RoutineGoal {
        RoutineGoal {
            expression: None,
            passes: 1,
            accuracy: 90,
            on_time: None,
            consecutive: false,
            attempt_limit: 4,
        }
    }
    fn edits(b: &Built) -> Vec<ProblemPlanEdit> {
        b.rows
            .iter()
            .map(|r| ProblemPlanEdit {
                key: r["key"].as_str().unwrap().into(),
                title: "精确问题".into(),
                notes: "节奏稳定".into(),
                cut: Some(ProblemCut {
                    start: BeatPoint {
                        measure: 1,
                        beat: 2.5,
                    },
                    end: BeatPoint {
                        measure: 2,
                        beat: 1.5,
                    },
                }),
            })
            .collect()
    }
    #[test]
    fn merged_problem_cuts_readonly_save_backup_and_real_daily() {
        let mut p = player();
        let source = record(&mut p, "C", 120, "right");
        p.history_score_review(&source.content_id, &source.id)
            .unwrap();
        let spec = ProblemPlanSpec {
            content_id: source.content_id.clone(),
            id: source.id.clone(),
            score_revision: p.score_revision,
            references: source.references,
            before: 0,
            after: 1,
            mode: "wait".into(),
            speed: 0.7,
            routine_id: None,
            day: Some("2026-10-02".into()),
        };
        let b = p.build_history_problem_plan(&spec).unwrap();
        assert_eq!(b.rows.len(), 1);
        let edits = edits(&b);
        let position = p.state.position;
        let score = serde_json::to_value(p.matcher.snapshot()).unwrap();
        let prefs = std::fs::read(&p.preferences_path).unwrap();
        let checked = p
            .apply_problem_cuts(
                p.build_history_problem_plan(&spec).unwrap(),
                &b.revision,
                &edits,
            )
            .unwrap();
        assert_eq!(checked.rows[0]["cut"]["start"]["beat"], 2.5);
        assert_eq!(std::fs::read(&p.preferences_path).unwrap(), prefs);
        assert_eq!(p.state.position, position);
        assert_eq!(serde_json::to_value(p.matcher.snapshot()).unwrap(), score);
        let saved = p
            .save_history_problem_plan(&spec, &b.revision, "短段".into(), "", &goal(), &edits)
            .unwrap();
        assert_eq!(saved["added"], 1);
        let retry = p
            .save_history_problem_plan(&spec, &b.revision, "短段".into(), "", &goal(), &edits)
            .unwrap();
        assert_eq!(retry["added"], 0);
        let backup: Backup =
            serde_json::from_value(p.practice_backup_snapshot(&Selection::default()).unwrap())
                .unwrap();
        assert_eq!(backup.version(), 10);
        let backup = Backup::decode(&backup.encode().unwrap()).unwrap();
        assert!(matches!(
            backup.routines[0].items[0].target,
            RoutineTarget::PrecisePassage { .. }
        ));
        let data = p.data.clone();
        drop(p);
        let mut p = Player::new(data, PathBuf::new(), true);
        p.restore();
        let item = p.preferences.routines[0].items[0].id.clone();
        p.open_routine_item(saved["id"].as_str().unwrap(), "2026-10-02", &item, true)
            .unwrap();
        assert_eq!(p.state.passage.as_ref().unwrap().start, 0.75);
        assert_eq!(p.state.passage.as_ref().unwrap().end, 2.5);
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
    fn cross_song_own_tempo_all_anchor_and_stale_protection() {
        let mut p = player();
        let a = record(&mut p, "C", 120, "right");
        let b = record(&mut p, "D", 60, "left");
        let spec = MultiProblemSpec {
            sources: vec![a, b],
            order: vec![],
            before: 0,
            after: 1,
            mode: "wait".into(),
            speed: 0.7,
            routine_id: None,
            day: None,
        };
        let built = p.build_multi_problem_plan(&spec).unwrap();
        let mut edits = edits(&built);
        let checked = p
            .apply_problem_cuts(
                p.build_multi_problem_plan(&spec).unwrap(),
                &built.revision,
                &edits,
            )
            .unwrap();
        let ranges = checked
            .items
            .iter()
            .map(|i| {
                if let RoutineTarget::PrecisePassage { range } = &i.target {
                    range.clone()
                } else {
                    panic!()
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(ranges[0].start_tick, ranges[1].start_tick);
        assert_ne!(ranges[0].content_id, ranges[1].content_id);
        edits[1].cut.as_mut().unwrap().start.beat = 3.5;
        assert!(
            p.save_multi_problem_plan(&spec, &built.revision, "bad", "", &goal(), &edits)
                .unwrap_err()
                .contains("全部所选问题")
        );
        assert!(p.preferences.routines.is_empty());
        edits = super::tests::edits(&built);
        edits[0].cut.as_mut().unwrap().end.measure = 3;
        assert!(
            p.save_multi_problem_plan(&spec, &built.revision, "bad", "", &goal(), &edits)
                .is_err()
        );
        assert!(p.preferences.routines.is_empty());
        edits = super::tests::edits(&built);
        assert!(
            p.apply_problem_cuts(p.build_multi_problem_plan(&spec).unwrap(), "old", &edits)
                .is_err()
        );
        let saved = p
            .save_multi_problem_plan(&spec, &built.revision, "跨曲短段", "", &goal(), &edits)
            .unwrap();
        let id = saved["id"].as_str().unwrap();
        let first = p.preferences.routines[0].items[0].id.clone();
        let second = p.preferences.routines[0].items[1].id.clone();
        p.open_routine_item(id, "2026-10-02", &first, true).unwrap();
        assert_eq!(p.state.passage.as_ref().unwrap().start, 0.75);
        p.command(Command::StopRoutine).unwrap();
        p.open_routine_item(id, "2026-10-02", &second, true)
            .unwrap();
        assert_eq!(p.state.passage.as_ref().unwrap().start, 1.5);
        assert_eq!(p.state.passage.as_ref().unwrap().end, 4.5);
        assert!(
            p.config
                .tracks
                .iter()
                .any(|t| t.practice_part == PracticePart::LeftHand
                    && t.player == PlayerConfig::Human)
        );
    }
}
