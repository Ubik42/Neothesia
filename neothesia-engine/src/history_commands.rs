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
fn condition_summary(session: &PracticeSession) -> Value {
    let range = match session.kind {
        PracticeSessionKind::WholeSong => None,
        PracticeSessionKind::Loop { start_measure, end_measure } => Some([start_measure, end_measure]),
    };
    let Some(context) = session.context.as_ref() else {
        return json!({"complete":false,"range":range,"mode":session.mode,"hands":session.hands,
            "speed":session.speed,"effectiveTempoBpm":session.effective_tempo_bpm,
            "scope":session.scope});
    };
    json!({"complete":session.scope.as_deref().is_some_and(|s|!s.is_empty())
            && session.mode.as_deref().is_some_and(|s|!s.is_empty())
            && context.target_notes>0 && context.judged_notes>0
            && !context.grid.is_empty() && !context.hands.is_empty(),
        "range":range,"mode":session.mode,"hands":session.hands,"speed":session.speed,
        "effectiveTempoBpm":session.effective_tempo_bpm,"scope":session.scope,"grid":context.grid,
        "targetNotes":context.target_notes,"judgedNotes":context.judged_notes,
        "tracks":context.tracks,"parts":context.parts,"assignedHands":context.hands,
        "exerciseSpec":context.exercise_spec,"countIn":context.count_in,
        "metronome":context.metronome,"inputLatencyMs":context.latency,
        "pedalLatencyMs":context.pedal_latency_ms.unwrap_or(context.latency),
        "pedalLatencySource":if context.pedal_latency_ms.is_some(){"saved"}else{"legacyInputLatency"},
        "rounds":context.rounds,"adaptive":context.adaptive,
        "expressionGoal":context.expression.as_ref().or_else(||context.routine.as_ref().and_then(|r|r.expression.as_ref()))})
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

    pub(super) fn practice_time(&self,content_id:&str,from:u64,to:u64)->Result<Value,String>{
        if content_id.len()!=64||!content_id.bytes().all(|b|b.is_ascii_hexdigit())||from>=to||to>9_000_000_000_000_000{return Err("曲目或统计时间范围无效".into())}
        let sessions=self.history.song(content_id).map(|s|s.sessions.as_slice()).unwrap_or(&[]);let ledger=self.history.time_ledger().get(content_id);
        let mut total=0u64;let mut period=0u64;let mut untracked=0u64;let mut earliest=None;
        for entry in ledger.into_iter().flat_map(|entries|entries.values()){untracked=untracked.saturating_add(entry.untracked_ms);for slice in &entry.slices{total=total.saturating_add(slice.running_ms);period=period.saturating_add(slice.intersection_ms(from,to));earliest=Some(earliest.map_or(slice.from,|old:u64|old.min(slice.from)));}}
        let mut legacy=0;let mut legacy_timed=0;let mut legacy_ms=0u64;let mut period_legacy=0;let mut unknown=0;let mut period_unknown=0;
        for session in sessions{if ledger.is_some_and(|entries|entries.contains_key(&session.stable_id())){continue}legacy+=1;let within=session.recorded_at_unix_ms>=from&&session.recorded_at_unix_ms<to;if within{period_legacy+=1}if let Some(ms)=session.playing_ms{legacy_timed+=1;legacy_ms=legacy_ms.saturating_add(ms)}else{unknown+=1;if within{period_unknown+=1}}}
        Ok(json!({"totalMs":total,"periodMs":period,"timedSessions":ledger.map_or(0,|entries|entries.len()),"unknownSessions":unknown,"periodUnknownSessions":period_unknown,"retainedSessions":sessions.len(),"legacySessions":legacy,"legacyTimedSessions":legacy_timed,"legacyTimedMs":legacy_ms,"periodLegacySessions":period_legacy,"untrackedMs":untracked,"earliestIntervalAt":earliest,"coverage":"saved-running-ledger","from":from,"to":to}))
    }
    pub(super) fn practice_calendar(&self,from:u64,to:u64,content_id:Option<&str>)->Result<Value,String>{
        if from>=to||to>9_000_000_000_000_000||to-from>366*86_400_000||content_id.is_some_and(|id|id.is_empty()||id.len()>160){return Err("练习日历需选择不超过 366 天的有效日期范围".into())}
        let mut intervals=Vec::new();let mut saves=Vec::new();let mut total=0u64;let mut untracked=0u64;let mut earliest=None;
        for(cid,entries)in self.history.time_ledger(){if content_id.is_some_and(|wanted|wanted!=cid){continue}let title=self.history.song(cid).map(|s|s.display_name.as_str()).unwrap_or("练习记录");for(id,entry)in entries{if entry.saved_at_unix_ms>=from&&entry.saved_at_unix_ms<to{saves.push(json!({"contentId":cid,"saveId":id,"savedAt":entry.saved_at_unix_ms}));untracked=untracked.saturating_add(entry.untracked_ms);}if saves.len()>100_000{return Err("该日期范围的保存记录过多，请缩小日期范围".into())}for slice in &entry.slices{earliest=Some(earliest.map_or(slice.from,|old:u64|old.min(slice.from)));let ms=slice.intersection_ms(from,to);if ms>0{total=total.saturating_add(ms);intervals.push(json!({"contentId":cid,"title":title,"saveId":id,"from":slice.from.max(from),"to":slice.to.min(to),"runningMs":ms}));}if intervals.len()+saves.len()>100_000{return Err("该日期范围的计时记录过多，请缩小日期范围".into())}}}}
        let mut legacy=0;let mut legacy_ms=0u64;for(cid,song)in self.history.snapshot(){if content_id.is_some_and(|wanted|wanted!=cid){continue}let ledger=self.history.time_ledger().get(&cid);for session in &song.sessions{if session.recorded_at_unix_ms>=from&&session.recorded_at_unix_ms<to&&!ledger.is_some_and(|entries|entries.contains_key(&session.stable_id())){legacy+=1;legacy_ms=legacy_ms.saturating_add(session.playing_ms.unwrap_or(0));}}}
        Ok(json!({"from":from,"to":to,"intervals":intervals,"saves":saves,"totalMs":total,"legacySessions":legacy,"legacyTimedMs":legacy_ms,"untrackedMs":untracked,"earliestIntervalAt":earliest,"coverage":"saved-running-ledger"}))
    }

    pub(super) fn history_context(&self) -> PracticeSessionContext {
        PracticeSessionContext {
            expression:self.active_expression_goal.as_ref().map(|e|neothesia_core::practice_history::PracticeExpressionCondition{velocity_difference:e.velocity_difference,contour_percent:e.contour_percent,pedal_offset_ms:e.pedal_offset_ms}),
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
    pub(super) fn history_query(&self,query:&str,mode:&str,hands:&str,range:&str,days:u32,offset:usize,limit:usize)->Result<Value,String>{self.history_query_window(query,mode,hands,range,days,offset,limit,None,None)}
    pub(super) fn history_query_window(
        &self,
        query: &str,
        mode: &str,
        hands: &str,
        range: &str,
        days: u32,
        offset: usize,
        limit: usize,
        from:Option<u64>,
        to:Option<u64>,
    ) -> Result<Value, String> {
        let window=match(from,to){(None,None)=>None,(Some(a),Some(b))if a<b&&b<=9_000_000_000_000_000&&b-a<=366*86_400_000=>Some((a,b)),_=>return Err("历史日期范围需完整填写，且不超过 366 天".into())};
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
                    if window.is_some_and(|(a,b)|session.recorded_at_unix_ms<a||session.recorded_at_unix_ms>=b){continue}
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
        let legacy_approvals=crate::legacy_exercise_history::active_approvals(&self.data,content_id);
        let peers = history.sessions[..index]
            .iter()
            .filter(|s| crate::legacy_exercise_history::same_reviewed(&legacy_approvals,session,s))
            .collect::<Vec<_>>();
        let best = peers
            .iter()
            .filter_map(|s| s.summary.overall.accuracy())
            .max_by(f32::total_cmp);
        let excluded_recent = history.sessions[..index]
            .iter()
            .rev()
            .filter(|older| !crate::legacy_exercise_history::same_reviewed(&legacy_approvals,session,older))
            .take(12)
            .map(|older| json!({
                "entry": entry(content_id, &history.display_name, None, older),
                "reasons": practice_conditions::attempt_difference_reasons(session, older),
            }))
            .collect::<Vec<_>>();
        let mut row = entry(
            content_id,
            &history.display_name,
            history.library.source_path.as_ref(),
            session,
        );
        row["performanceAvailable"]=json!(crate::performance_archive::available(&self.data,content_id,id));
        row["summary"] = json!(session.summary);
        if let Some(e)=session.context.as_ref().and_then(|c|c.expression.as_ref().or_else(||c.routine.as_ref().and_then(|r|r.expression.as_ref()))){let g=crate::expression_goal::ExpressionGoal{velocity_difference:e.velocity_difference,contour_percent:e.contour_percent,pedal_offset_ms:e.pedal_offset_ms};row["expressionGoalResult"]=json!(g.result(session.summary.expression));}
        row["context"] = json!(session.context);
        let conditions=condition_summary(session);
        let (reason_code,reason)=if !peers.is_empty(){(None,None)}
            else if !conditions["complete"].as_bool().unwrap_or(false){
                (Some("incomplete-conditions"),Some("这条记录缺少完整的演奏条件或有效音符判断，无法证明与旧记录同条件。"))
            }else if index==0{
                (Some("first-attempt"),Some("这是该曲保留的第一条记录，暂无更早的练习可比较。"))
            }else{
                (Some("different-conditions"),Some("更早的记录在范围、模式、速度、音符覆盖、分手、设备校准或其他练习条件上不同。"))
            };
        row["comparison"] = json!({"count":peers.len(),"bestAccuracy":best,
            "reviewedLegacyCount":peers.iter().filter(|s|!practice_conditions::same_attempt_conditions(session,s)).count(),
            "conditions":conditions,"reasonCode":reason_code,"reason":reason,
            "excludedRecent":excluded_recent,
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

#[cfg(test)]
mod tests {
    use super::*;
    use neothesia_core::practice_history::PracticeSessionContext;
    use std::collections::BTreeMap;

    #[test]
    fn comparison_conditions_expose_the_exact_saved_task_and_legacy_pedal_basis() {
        let mut parts = BTreeMap::new();
        parts.insert(2, PracticePart::RightHand);
        let session = PracticeSession {
            replay_clips: vec![], playing_ms: None, annotation: None, id: None,
            context: Some(PracticeSessionContext {
                expression: None,
                routine: None, ladder: None, target_notes: 18, judged_notes: 17,
                tracks: vec![], parts, hands: "right".into(), count_in: 2,
                metronome: true, latency: 24, pedal_latency_ms: None,
                rounds: 3, adaptive: true, grid: "grid-1".into(),
                source_path: None, exercise_spec: None,
            }),
            recorded_at_unix_ms: 1,
            kind: PracticeSessionKind::Loop { start_measure: 5, end_measure: 7 },
            hands: PracticeHands::Right, speed: 0.75,
            mode: Some("flow".into()), scope: Some("scope-1".into()),
            effective_tempo_bpm: Some(90), summary: AttemptSummary::default(),
        };
        let summary=condition_summary(&session);
        assert_eq!(summary["range"],json!([5,7]));
        assert_eq!(summary["targetNotes"],18);
        assert_eq!(summary["judgedNotes"],17);
        assert_eq!(summary["pedalLatencyMs"],24);
        assert_eq!(summary["pedalLatencySource"],"legacyInputLatency");
        assert_eq!(summary["parts"]["2"],"RightHand");
        assert_eq!(summary["adaptive"],true);
        assert_eq!(summary["effectiveTempoBpm"],90);
        assert_eq!(summary["complete"],true);
    }
}
