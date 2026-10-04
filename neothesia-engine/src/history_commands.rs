use super::*;
use neothesia_core::practice_history::PracticeSessionContext;
use serde_json::{Value, json};

fn session_id(session: &PracticeSession) -> String {session.stable_id()}
fn entry(
    content_id: &str,
    title: &str,
    path: Option<&PathBuf>,
    session: &PracticeSession,
) -> Value {
    let range = match session.kind {
        PracticeSessionKind::WholeSong => None,
        PracticeSessionKind::Loop {
            start_measure,
            end_measure,
        } => Some([start_measure, end_measure]),
    };
    json!({"id":session_id(session),"contentId":content_id,"title":title,
        "path":session.context.as_ref().and_then(|c|c.source_path.as_ref()).or(path),
        "recordedAt":session.recorded_at_unix_ms,"playingMs":session.playing_ms,"speed":session.speed,"hands":session.hands,
        "annotation":session.annotation,"mode":session.mode,"scope":session.scope,"range":range,"loop":range.is_some(),
        "score":session.summary.overall,"tempo":session.effective_tempo_bpm,
        "restorable":session.context.is_some(),"coverage":session.context.as_ref().map(|c|json!({"target":c.target_notes,"judged":c.judged_notes}))})
}
fn comparable(a: &PracticeSession, b: &PracticeSession) -> bool {
    a.scope.is_some()
        && a.mode.is_some()
        && a.scope == b.scope
        && a.mode == b.mode
        && a.hands == b.hands
        && a.kind == b.kind
        && (a.speed - b.speed).abs() < 0.0001
        && a.effective_tempo_bpm == b.effective_tempo_bpm
        && match (&a.context, &b.context) {
            (Some(a), Some(b)) => {
                a.tracks == b.tracks
                    && a.parts == b.parts
                    && a.exercise_spec == b.exercise_spec
                    && a.target_notes == b.target_notes
                    && a.judged_notes == b.judged_notes
                    && a.metronome == b.metronome
                    && a.latency == b.latency
                    && a.pedal_latency_ms.unwrap_or(0)==b.pedal_latency_ms.unwrap_or(0)
                    && a.adaptive == b.adaptive
            }
            (None, None) => true,
            _ => false,
        }
}
impl Player {
    pub(super) fn set_library_collection(&mut self,path:&PathBuf,content_id:&str,favorite:Option<bool>,queued:Option<bool>) -> Result<Value,String> {
        if content_id.len()!=64 || !content_id.bytes().all(|b|b.is_ascii_hexdigit()) || favorite.is_some()==queued.is_some() {return Err("请选择一项有效的曲目收藏或队列操作".into());}
        let mut workspace=crate::library_workspace::Workspace::load(&self.data)?;
        let entry=if path.is_file(){workspace.inspect(path.clone(),true)?}else{workspace.entries.get(&crate::library_workspace::key(path)).cloned().ok_or("未知文件位置，请先检查文件")?};
        if entry.content_id.as_deref()!=Some(content_id) {return Err("文件内容已改变，请重新检查并选择曲目".into());}
        let title=entry.metadata.title.clone().unwrap_or_else(||path.file_stem().unwrap_or_default().to_string_lossy().into());
        if let Some(enabled)=favorite {self.history.set_favorite(content_id,&title,Some(path.clone()),enabled).map_err(|e|e.to_string())?;}
        if let Some(enabled)=queued {self.history.set_queued(content_id,&title,Some(path.clone()),enabled).map_err(|e|e.to_string())?;}
        let library=self.history.song(content_id).map(|s|&s.library);
        Ok(json!({"favorite":library.is_some_and(|l|l.favorite),"queuePosition":library.and_then(|l|l.queue_position)}))
    }

    pub(super) fn practice_time(&self, content_id: &str, from: u64, to: u64) -> Result<Value,String> {
        if content_id.len()!=64 || !content_id.bytes().all(|b|b.is_ascii_hexdigit()) || from>=to || to>9_000_000_000_000_000 {return Err("曲目或统计时间范围无效".into());}
        let sessions=self.history.song(content_id).map(|h|h.sessions.as_slice()).unwrap_or(&[]);
        let mut total=0u64; let mut period=0u64; let mut timed=0usize; let mut unknown=0usize; let mut period_unknown=0usize;
        for s in sessions {
            let within=s.recorded_at_unix_ms>=from && s.recorded_at_unix_ms<to;
            if let Some(ms)=s.playing_ms {total=total.saturating_add(ms);timed+=1;if within{period=period.saturating_add(ms);}}
            else {unknown+=1;if within{period_unknown+=1;}}
        }
        Ok(json!({"totalMs":total,"periodMs":period,"timedSessions":timed,"unknownSessions":unknown,"periodUnknownSessions":period_unknown,"retainedSessions":sessions.len(),"from":from,"to":to}))
    }

    pub(super) fn history_context(&self) -> PracticeSessionContext {
        PracticeSessionContext {
            routine: self.routine.as_ref().map(|a| {
                neothesia_core::practice_history::PracticeRoutineCondition {
                    expression:a.item.goal.expression.as_ref().map(|e|neothesia_core::practice_history::PracticeExpressionCondition{velocity_difference:e.velocity_difference,contour_percent:e.contour_percent,pedal_offset_ms:e.pedal_offset_ms}),
                    item_notes: a.item.notes.clone(),
                    consecutive: a.item.goal.consecutive,
                    routine_id: a.routine_id.clone(),
                    routine_name: self
                        .preferences
                        .routine_runs
                        .get(&format!("{}:{}", a.routine_id, a.day))
                        .map_or_else(|| a.routine_id.clone(), |r| r.routine.name.clone()),
                    day: a.day.clone(),
                    item_id: a.item.id.clone(),
                    item_title: a.item.title.clone(),
                    passes_required: a.item.goal.passes,
                    accuracy_percent: a.item.goal.accuracy,
                    on_time_percent: a.item.goal.on_time,
                }
            }),
            ladder: self.ladder.as_ref().filter(|r| r.active).map(|r| {
                neothesia_core::practice_history::PracticeLadderCondition {
                    plan_id: r.preset.id.clone(),
                    plan_name: r.preset.name.clone(),
                    stage: r.progress.stage + 1,
                    speed_percent: r.progress.speed(&r.preset.plan),
                    target_percent: r.preset.plan.target_percent,
                }
            }),
            target_notes: self
                .notes
                .iter()
                .filter(|n| {
                    self.note_player(n.track_id, n.index) == PlayerConfig::Human
                        && self.state.passage.as_ref().is_none_or(|p| {
                            n.start.as_secs_f64() >= p.start && n.start.as_secs_f64() < p.end
                        })
                })
                .count(),
            judged_notes: self.matcher.snapshot().matched_notes
                + self.matcher.snapshot().missed_notes,
            tracks: self.config.practice_track_setup(),
            parts: self
                .config
                .tracks
                .iter()
                .map(|t| (t.track_id, t.practice_part))
                .collect(),
            hands: self.state.hands.clone(),
            count_in: self.state.count_in,
            metronome: self.state.metronome,
            latency: self.state.latency,
            pedal_latency_ms:Some(self.state.latency),
            rounds: self.state.rounds,
            adaptive: self.state.adaptive,
            grid: self.grid_signature(),
            source_path: self.file.as_ref().and_then(|f| f.source_path.clone()),
            exercise_spec: self.exercise,
        }
    }
    pub(super) fn history_query(
        &self,
        query: &str,
        mode: &str,
        hands: &str,
        range: &str,
        days: u32,
        offset: usize,
        limit: usize,
    ) -> Result<Value, String> {
        let query = query.trim().to_lowercase();
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let cutoff = cutoff.saturating_sub(u64::from(days) * 86_400_000);
        let mut rows = Vec::new();
        for song in self.history.recent_songs(usize::MAX) {
            let song_matches=query.is_empty() || song.display_name.to_lowercase().contains(&query) || song.source_path.as_ref().is_some_and(|p|p.to_string_lossy().to_lowercase().contains(&query));
            if let Some(history) = self.history.song(&song.content_id) {
                for session in &history.sessions {
                    if !song_matches && !session.annotation.as_ref().is_some_and(|a|[&a.note,&a.teacher,&a.next].iter().any(|v|v.to_lowercase().contains(&query))){continue;}
                    if !mode.is_empty() && session.mode.as_deref() != Some(mode) {
                        continue;
                    }
                    if !hands.is_empty()
                        && serde_json::to_value(session.hands)
                            .ok()
                            .and_then(|v| v.as_str().map(str::to_owned))
                            .as_deref()
                            != Some(hands)
                    {
                        continue;
                    }
                    if days > 0 && session.recorded_at_unix_ms < cutoff {
                        continue;
                    }
                    let is_loop = matches!(session.kind, PracticeSessionKind::Loop { .. });
                    if (range == "loop" && !is_loop) || (range == "whole" && is_loop) {
                        continue;
                    }
                    rows.push(entry(
                        &song.content_id,
                        &song.display_name,
                        song.source_path.as_ref(),
                        session,
                    ));
                }
            }
        }
        rows.sort_by_key(|v| std::cmp::Reverse(v["recordedAt"].as_u64().unwrap_or(0)));
        let total = rows.len();
        let rows = rows
            .into_iter()
            .skip(offset)
            .take(if limit == 0 { 50 } else { limit.min(200) })
            .collect::<Vec<_>>();
        Ok(json!({"entries":rows,"total":total,"offset":offset}))
    }
    pub(super) fn annotate_history(&mut self, content_id:&str,id:&str,note:String,teacher:String,next:String,expected:Option<neothesia_core::practice_history::PracticeAnnotation>)->Result<Value,String>{
        let annotation=neothesia_core::practice_history::PracticeAnnotation{note,teacher,next,updated_at_unix_ms:std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64};
        self.history.annotate_session(content_id,id,expected,annotation)?;
        self.history_detail(content_id,id)
    }
    pub(super) fn history_detail(&self, content_id: &str, id: &str) -> Result<Value, String> {
        let history = self.history.song(content_id).ok_or("曲目记录不存在")?;
        let index = history
            .sessions
            .iter()
            .position(|s| session_id(s) == id)
            .ok_or("这条记录已不在历史中")?;
        let session = &history.sessions[index];
        let peers = history.sessions[..index]
            .iter()
            .filter(|s| comparable(session, s))
            .collect::<Vec<_>>();
        let best = peers
            .iter()
            .filter_map(|s| s.summary.overall.accuracy())
            .max_by(f32::total_cmp);
        let mut row = entry(
            content_id,
            &history.display_name,
            history.library.source_path.as_ref(),
            session,
        );
        row["performanceAvailable"]=json!(crate::performance_archive::available(&self.data,content_id,id));
        row["summary"] = json!(session.summary);
        if let Some(e)=session.context.as_ref().and_then(|c|c.routine.as_ref()).and_then(|r|r.expression.as_ref()){let g=crate::expression_goal::ExpressionGoal{velocity_difference:e.velocity_difference,contour_percent:e.contour_percent,pedal_offset_ms:e.pedal_offset_ms};let result=g.assess(session.summary.expression);row["expressionGoalResult"]=json!({"passed":result.is_ok(),"message":result.err().unwrap_or_else(||"本轮力度/踏板要求达标".into())});}
        row["context"] = json!(session.context);
        row["comparison"] = json!({"count":peers.len(),"bestAccuracy":best,
            "previous":peers.last().map(|s|entry(content_id,&history.display_name,None,s)),
            "recent":peers.iter().rev().take(29).rev().map(|s|entry(content_id,&history.display_name,None,s)).collect::<Vec<_>>()});
        Ok(row)
    }
    pub(super) fn history_open(
        &mut self,
        content_id: &str,
        id: &str,
        restore: bool,
    ) -> Result<Value, String> {
        let history = self.history.song(content_id).ok_or("曲目记录不存在")?;
        let session = history
            .sessions
            .iter()
            .find(|s| session_id(s) == id)
            .cloned()
            .ok_or("这条记录已不在历史中")?;
        if restore && session.context.is_none() {
            return Err("旧记录没有保存完整条件，请使用打开曲目".into());
        }
        let title = history.display_name.clone();
        let spec = session
            .context
            .as_ref()
            .and_then(|c| c.exercise_spec)
            .or_else(|| history.setup.as_ref().and_then(|s| s.exercise_spec));
        let paths = [
            session.context.as_ref().and_then(|c| c.source_path.clone()),
            history.setup.as_ref().and_then(|s| s.source_path.clone()),
            history.library.source_path.clone(),
        ];
        if let Some(spec) = spec {
            self.command(Command::Generate { spec })?;
        } else {
            let file=crate::library::verified_file(&self.data,content_id,paths.into_iter().flatten())?;
            self.install(file, title, None)?;
        }
        let mut warning = None;
        if restore {
            let context = session.context.as_ref().unwrap();
            let previous_config = self.config.clone();
            let previous_hands = self.state.hands.clone();
            self.config.apply_practice_setup(&SongPracticeSetup {
                tracks: context.tracks.clone(),
                ..Default::default()
            });
            for track in &mut self.config.tracks {
                if let Some(part) = context.parts.get(&track.track_id) {
                    track.practice_part = *part;
                }
            }
            self.state.hands = context.hands.clone();
            if self.grid_signature() != context.grid
                || session.scope.as_deref() != Some(self.practice_scope().as_str())
            {
                self.config = previous_config;
                self.state.hands = previous_hands;
                warning=Some("曲目已打开，但小节或逐音左右手标注已改变，未恢复旧条件。请重新确认声部和选段。".to_string());
            } else {
                self.state.mode = session.mode.clone().unwrap_or_else(|| "wait".into());
                self.state.wait = self.state.mode == "wait";
                self.state.speed = f64::from(session.speed);
                self.state.count_in = context.count_in;
                self.state.metronome = context.metronome;
                self.state.latency = context.latency;
                self.state.rounds = context.rounds;
                self.state.adaptive = context.adaptive;
                self.state.status = "ready".into();
                self.state.passage = None;
                if let PracticeSessionKind::Loop {
                    start_measure,
                    end_measure,
                } = session.kind
                {
                    self.command(Command::MeasureLoop {
                        start: start_measure,
                        end: end_measure,
                        enabled: true,
                    })?;
                } else {
                    self.reset(0.);
                }
            }
        }
        self.panic();
        self.state.status = "ready".into();
        self.persist()?;
        let mut result = self.command(Command::CurrentSong)?;
        result["restoreWarning"] = json!(warning);
        Ok(result)
    }
}
