use super::*;
use serde_json::{Value, json};

pub(super) struct Demo {
    pub id: String,
    pub start: f64,
    pub end: f64,
    pub position: f64,
    pub rate: f64,
    pub active: bool,
    pub(super) paused: bool,
    pub(super) history: bool,
    replay_loop:Option<(f64,f64)>,
    loop_rounds:u64,
    events: Vec<(f64, usize, [u8; 3])>,
    cursor: usize,
    lease: Duration,
    held:Option<crate::held_demo::HeldContext>,
}
// Reconstruct MIDI state, including released notes sustained by CC64.
fn replay_state(events:&[(f64,usize,[u8;3])],position:f64)->(usize,Vec<[u8;3]>) {
    let mut voices=std::collections::BTreeMap::<(u8,u8),(u8,bool)>::new();
    let mut pedals=[0u8;16];
    let cursor=events.partition_point(|e|e.0<=position+0.000001);
    for (_,_,[status,key,value]) in &events[..cursor] {
        let channel=status&15;
        match status&240 {
            144 if *value>0=>{voices.insert((channel,*key),(*value,true));},
            128|144=>{if pedals[channel as usize]>=64 {if let Some(v)=voices.get_mut(&(channel,*key)){v.1=false;}}else{voices.remove(&(channel,*key));}},
            176 if *key==64=>{pedals[channel as usize]=*value;if *value<64{voices.retain(|(ch,_),(_,held)|*ch!=channel || *held);}},
            176 if *key==120=>{voices.retain(|(ch,_),_|*ch!=channel);},
            176 if *key==123=>{if pedals[channel as usize]>=64{for ((ch,_),v) in &mut voices{if *ch==channel{v.1=false;}}}else{voices.retain(|(ch,_),_|*ch!=channel);}},
            _=>{}
        }
    }
    let mut restore=Vec::new();
    for (channel,value) in pedals.into_iter().enumerate(){if value>0{restore.push([176|channel as u8,64,value]);}}
    for ((channel,pitch),(velocity,held)) in voices{restore.push([144|channel,pitch,velocity]);if !held{restore.push([128|channel,pitch,0]);}}
    (cursor,restore)
}
impl Player {
    pub(super) fn history_replay_loop(&mut self,id:&str,start:f64,end:f64,enabled:bool)->Result<Value,String>{
        let current=self.finger_demo.as_ref().filter(|d|d.history && d.id==id).ok_or("这次回放已结束或被接管，请重新回放")?;
        if enabled && (!start.is_finite() || !end.is_finite() || start<current.start || end>current.end || end-start<0.1){return Err("循环范围应在保留的演奏内，至少 0.1 秒且终点晚于起点".into());}
        let mut demo=self.finger_demo.take().unwrap();
        demo.replay_loop=enabled.then_some((start,end));demo.loop_rounds=0;demo.lease=Duration::ZERO;
        if enabled && (demo.position<start || demo.position>=end){
            self.panic();demo.position=start;let (cursor,events)=replay_state(&demo.events,start);demo.cursor=cursor;
            if demo.active{for e in events{self.send(&e);}}else{demo.paused=true;}
        }
        self.finger_demo=Some(demo);Ok(self.finger_demo_state())
    }

    pub(super) fn history_replay_control(&mut self,id:&str,action:&str,position:Option<f64>,rate:Option<f64>)->Result<Value,String>{
        let current=self.finger_demo.as_ref().filter(|d|d.history && d.id==id).ok_or("这次回放已结束或被接管，请重新回放")?;
        if !["pause","resume","seek","rate"].contains(&action){return Err("回放操作无效".into());}
        if action=="seek" && position.is_none_or(|p|!p.is_finite() || p<current.start || p>current.end){return Err("回放位置超出已保留范围".into());}
        if action=="seek" && current.replay_loop.is_some_and(|(start,end)|position.is_some_and(|p|p<start || p>=end)){return Err("请在当前回放循环内定位，或先关闭循环".into());}
        if action=="rate" && rate.is_none_or(|r|!r.is_finite() || !(0.25..=1.).contains(&r)){return Err("回放速度应为 25% 至 100%".into());}
        if action=="resume" && current.position>=current.end{return Err("回放已结束，请先定位到较早位置".into());}
        let mut demo=self.finger_demo.take().unwrap();
        demo.lease=Duration::ZERO;
        match action{
            "pause"=>{self.panic();demo.active=false;demo.paused=true;},
            "resume"=>{self.panic();let (cursor,events)=replay_state(&demo.events,demo.position);demo.cursor=cursor;for e in events{self.send(&e);}demo.active=true;demo.paused=false;},
            "seek"=>{self.panic();demo.position=position.unwrap();let (cursor,events)=replay_state(&demo.events,demo.position);demo.cursor=cursor;if demo.active && demo.position<demo.end{for e in events{self.send(&e);}}else{demo.active=false;demo.paused=demo.position<demo.end;}},
            "rate"=>demo.rate=rate.unwrap(),
            _=>unreachable!()
        }
        self.finger_demo=Some(demo);
        Ok(self.finger_demo_state())
    }

    pub(super) fn finger_demo_control(&mut self,id:&str,action:&str,position:Option<f64>)->Result<Value,String>{
        let current=self.finger_demo.as_ref().filter(|d|!d.history && d.id==id).ok_or("这次示范已停止或被接管，请重新开始")?;
        if !["pause","resume","seek"].contains(&action){return Err("示范操作无效".into());}
        if action=="seek"&&position.is_none_or(|p|!p.is_finite()||p<current.start||p>current.end){return Err("定位超出本次示范的小节范围".into());}
        if action=="resume"&&current.position>=current.end{return Err("示范已结束，请先定位到较早小节".into());}
        let mut demo=self.finger_demo.take().unwrap();demo.lease=Duration::ZERO;
        if action=="pause" {self.panic();demo.active=false;demo.paused=true;}
        else {
            self.panic();if action=="seek" {demo.position=position.unwrap();} else {demo.active=true;demo.paused=false;}
            // Auditions use per-track routes; restoring raw channels would mix voices across tracks.
            let cursor=demo.events.partition_point(|e|e.0<=demo.position+0.000001);
            let mut voices=std::collections::BTreeMap::<(usize,u8,u8),u8>::new();
            for (_,track,[status,pitch,velocity]) in &demo.events[..cursor] {
                match status&240 {144 if *velocity>0=>{voices.insert((*track,status&15,*pitch),*velocity);},128|144=>{voices.remove(&(*track,status&15,*pitch));},_=>{}}
            }
            demo.cursor=cursor;
            if demo.position>=demo.end {demo.active=false;demo.paused=false;}
            else if demo.active {for ((track,channel,pitch),velocity) in voices {self.send_track(track,&[144|channel,pitch,velocity]);}}
            else {demo.paused=true;}
        }
        self.finger_demo=Some(demo);Ok(self.finger_demo_state())
    }

    pub(super) fn finger_demo_state(&mut self) -> Value {
        if let Some(d) = &mut self.finger_demo {
            d.lease = Duration::ZERO;
        }
        match &self.finger_demo {
            Some(d) => {
                let mut state=json!({"id":d.id,"active":d.active,"position":d.position,"start":d.start,"end":d.end,"rate":d.rate,"paused":d.paused,"history":d.history,"loop":d.replay_loop.map(|(start,end)|json!({"start":start,"end":end})),"loopRounds":d.loop_rounds});
                if let Some(h)=&d.held{
                  state["kind"]=json!("held");
                  state["fingerNotes"]=json!(h.notes.iter().filter(|n|n.onset<=d.position&&n.end>d.position).map(|n|{
                    let change=h.actions.iter().filter(|a|a.track==n.track&&a.index==n.index&&a.at<=d.position).max_by(|a,b|a.at.total_cmp(&b.at));
                    json!({"track":n.track,"index":n.index,"pitch":n.pitch,"part":n.part,"finger":change.map(|a|a.to).or(n.finger),"held":n.onset<d.position,"changed":change.is_some()})
                  }).collect::<Vec<_>>());
                  state["actionsCompleted"]=json!(h.actions.iter().filter(|a|a.at>=h.start&&a.at<=d.position).count());
                  state["actionsTotal"]=json!(h.actions.iter().filter(|a|a.at>=h.start).count());
                }
                state
            }
            None => json!({"id":null,"active":false}),
        }
    }
    pub(super) fn start_finger_demo(
        &mut self,
        request: fingerings::PlanRequest,
        fingerprint: String,
        rate: f64,
    ) -> Result<Value, String> {
        if !rate.is_finite() || !(0.25..=1.0).contains(&rate) {
            return Err("示范速度应为原速的 25% 至 100%".into());
        }
        if matches!(self.state.status.as_str(), "playing" | "countIn")
            || self.recorder.is_recording()
            || self.routine.is_some()
            || self.ladder.as_ref().is_some_and(|r| r.active)
            || !self.state.pressed.is_empty()
        {
            return Err("请先停止练习和录音，并松开琴键，再开始指法示范".into());
        }
        let file = self.file.as_ref().ok_or("请先选择曲目")?;
        let prepared = fingerings::prepare(
            file,
            &self.config,
            &self.fingers,
            &self.note_hands,
            request.clone(),
        )?;
        if prepared.fingerprint != fingerprint {
            return Err("音符、分手或已有指法已变化，请重新生成方案".into());
        }
        let start = prepared
            .positions
            .iter()
            .filter(|n| n.target)
            .map(|n| n.onset)
            .fold(f64::INFINITY, f64::min);
        let end = prepared
            .positions
            .iter()
            .filter(|n| n.target)
            .map(|n| n.end)
            .fold(0.0, f64::max);
        if !start.is_finite() || !end.is_finite() || end <= start {
            return Err("该选段没有可示范的时值".into());
        }
        let mut events = Vec::new();
        // Include notes already held at the first selected onset; do not audition following context.
        for p in &prepared.positions {
            if p.onset >= end || p.end <= start || (!p.target && p.track == request.track && p.onset >= start) {
                continue;
            }
            let track=file.tracks.iter().find(|t|t.track_id==p.track).ok_or("联动音轨已变化")?;
            let n = &track.notes[p.index];
            events.push((
                p.onset.max(start),
                p.track,
                [144 | n.channel, n.note, n.velocity.max(1)],
            ));
            events.push((p.end.min(end), p.track, [128 | n.channel, n.note, 0]));
        }
        self.begin_finger_demo(start,end,rate,events,None)
    }
    pub(super) fn begin_finger_demo(&mut self,start:f64,end:f64,rate:f64,mut events:Vec<(f64,usize,[u8;3])>,held:Option<crate::held_demo::HeldContext>)->Result<Value,String>{
        events.sort_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then_with(|| if a.1==usize::MAX && b.1==usize::MAX {std::cmp::Ordering::Equal} else {(a.2[0] & 240).cmp(&(b.2[0] & 240))})
        });
        self.ensure_audio()?;
        self.panic();
        self.finger_demo = Some(Demo {
            id: format!(
                "demo-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_nanos()
            ),
            start,
            end,
            position: start,
            rate,
            active: true,
            paused: false,
            history: false,
            replay_loop:None,
            loop_rounds:0,
            events,
            cursor: 0,
            lease: Duration::ZERO,
            held,
        });
        self.tick_finger_demo(Duration::ZERO);
        Ok(self.finger_demo_state())
    }
    pub(super) fn tick_finger_demo(&mut self, delta: Duration) {
        let Some(mut demo) = self.finger_demo.take() else {
            return;
        };
        demo.lease += delta;
        if (demo.active || demo.paused || demo.history) && demo.lease > Duration::from_secs(3) {
            self.panic();
            return;
        }
        if !demo.active {
            self.finger_demo = Some(demo);
            return;
        }
        let next=demo.position+delta.as_secs_f64()*demo.rate;
        let boundary=demo.replay_loop.map_or(demo.end,|(_,end)|end);
        demo.position = next.min(boundary);
        while demo.cursor < demo.events.len()
            && demo.events[demo.cursor].0 <= demo.position + 0.000001
            && (demo.replay_loop.is_none() || demo.events[demo.cursor].0<boundary)
        {
            if demo.events[demo.cursor].1==usize::MAX {self.send(&demo.events[demo.cursor].2);}else{self.send_track(demo.events[demo.cursor].1, &demo.events[demo.cursor].2);}
            demo.cursor += 1;
        }
        if let Some((start,end))=demo.replay_loop.filter(|(_,end)|next>=*end){
            let length=end-start;demo.loop_rounds=demo.loop_rounds.saturating_add(((next-start)/length).floor() as u64);
            self.panic();demo.position=start+(next-start).rem_euclid(length);
            let (cursor,events)=replay_state(&demo.events,start);demo.cursor=cursor;
            for e in events{self.send(&e);}
            while demo.cursor<demo.events.len() && demo.events[demo.cursor].0<=demo.position+0.000001 && demo.events[demo.cursor].0<end{
                self.send(&demo.events[demo.cursor].2);demo.cursor+=1;
            }
        } else if demo.position >= demo.end {
            demo.active = false;
            if demo.history { self.panic(); }
        }
        self.finger_demo = Some(demo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neothesia_core::fingering::HandSpanProfile;
    #[test]
    fn history_replay_loop_wraps_keeps_paused_and_excludes_endpoint(){
        let data=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../work").join(format!("cycle176-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut p=Player::new(data,PathBuf::from("unused.sf2"),true);let before=p.snapshot();
        p.begin_finger_demo(0.,4.,1.,vec![(0.,usize::MAX,[144,60,80]),(2.,usize::MAX,[144,62,90]),(2.2,usize::MAX,[128,62,0]),(3.,usize::MAX,[128,60,0])],None).unwrap();
        p.finger_demo.as_mut().unwrap().history=true;let id=p.finger_demo.as_ref().unwrap().id.clone();
        assert!(p.history_replay_loop("stale",1.,2.,true).is_err());
        assert!(p.history_replay_loop(&id,2.,1.,true).is_err());
        p.history_replay_loop(&id,1.,2.,true).unwrap();assert_eq!(p.finger_demo.as_ref().unwrap().position,1.);
        p.tick_finger_demo(Duration::from_millis(1500));let demo=p.finger_demo.as_ref().unwrap();
        assert_eq!(demo.position,1.5);assert_eq!(demo.loop_rounds,1);assert_eq!(demo.cursor,1);
        assert!(p.history_replay_control(&id,"seek",Some(2.),None).is_err());
        p.history_replay_control(&id,"pause",None,None).unwrap();p.tick_finger_demo(Duration::from_millis(500));assert_eq!(p.finger_demo.as_ref().unwrap().position,1.5);
        p.history_replay_control(&id,"resume",None,None).unwrap();p.tick_finger_demo(Duration::from_millis(2800));
        assert_eq!(p.finger_demo.as_ref().unwrap().loop_rounds,4);
        assert!((p.finger_demo.as_ref().unwrap().position-1.3).abs()<0.000001);
        p.history_replay_loop(&id,0.,4.,false).unwrap();p.tick_finger_demo(Duration::from_millis(2800));assert!(!p.finger_demo.as_ref().unwrap().active);
        assert_eq!(p.snapshot().position,before.position);assert_eq!(p.snapshot().score,before.score);
    }
    #[test]
    fn history_replay_state_preserves_pedal_latched_notes_and_channels(){
        let events=vec![(0.,usize::MAX,[145,60,80]),(0.2,usize::MAX,[177,64,100]),(0.3,usize::MAX,[129,60,0]),(0.4,usize::MAX,[146,62,90]),(1.,usize::MAX,[177,64,0]),(2.,usize::MAX,[130,62,0])];
        let (cursor,state)=replay_state(&events,0.5);
        assert_eq!(cursor,4);assert_eq!(state,vec![[177,64,100],[145,60,80],[129,60,0],[146,62,90]]);
        assert_eq!(replay_state(&events,1.5).1,vec![[146,62,90]]);
        assert!(replay_state(&events,2.).1.is_empty());
    }
    #[test]
    fn history_replay_pause_seek_resume_scoped_state_and_lease(){
        let data=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../work").join(format!("cycle175-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut p=Player::new(data,PathBuf::from("unused.sf2"),true);
        let before=p.snapshot();
        p.begin_finger_demo(0.,4.,0.5,vec![(0.,usize::MAX,[144,60,88]),(3.,usize::MAX,[128,60,0])],None).unwrap();
        p.finger_demo.as_mut().unwrap().history=true;
        let id=p.finger_demo.as_ref().unwrap().id.clone();
        assert!(p.history_replay_control("stale","pause",None,None).is_err());
        assert!(p.finger_demo.as_ref().unwrap().active);
        p.tick_finger_demo(Duration::from_millis(500));
        p.history_replay_control(&id,"pause",None,None).unwrap();
        p.tick_finger_demo(Duration::from_secs(1));
        assert_eq!(p.finger_demo.as_ref().unwrap().position,0.25);
        assert!(p.finger_demo.as_ref().unwrap().paused);
        p.history_replay_control(&id,"seek",Some(2.),None).unwrap();
        assert!(!p.finger_demo.as_ref().unwrap().active);
        assert!(p.history_replay_control(&id,"seek",Some(5.),None).is_err());
        assert_eq!(p.finger_demo.as_ref().unwrap().position,2.);
        p.history_replay_control(&id,"rate",None,Some(1.)).unwrap();
        p.history_replay_control(&id,"resume",None,None).unwrap();
        p.tick_finger_demo(Duration::from_millis(500));
        assert_eq!(p.finger_demo.as_ref().unwrap().position,2.5);
        assert_eq!(p.snapshot().position,before.position);assert_eq!(p.snapshot().score,before.score);
        p.history_replay_control(&id,"pause",None,None).unwrap();
        p.tick_finger_demo(Duration::from_secs(4));assert!(p.finger_demo.is_none());
    }
    #[test]
    fn held_substitution_changes_fingers_without_retriggering_the_key(){
        let data=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../work").join(format!("held-demo-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut p=Player::new(data,PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../default.sf2"),true);
        let xml=br#"<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>2</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>8</duration><voice>1</voice></note><backup><duration>8</duration></backup><forward><duration>4</duration></forward><note><pitch><step>E</step><octave>4</octave></pitch><duration>2</duration><voice>2</voice></note><note><pitch><step>F</step><octave>4</octave></pitch><duration>2</duration><voice>2</voice></note></measure></part></score-partwise>"#;
        p.command(Command::ImportScore{bytes:xml.to_vec(),name:"持音示范单元.musicxml".into(),default_bpm:120}).unwrap();
        for t in &mut p.config.tracks{t.practice_part=PracticePart::RightHand;}
        p.note_hands.clear();
        let file=p.file.as_ref().unwrap();let content_id=file.content_id.clone();let track=file.tracks.iter().find(|t|!t.notes.is_empty()).unwrap();let track_id=track.track_id;
        let source=track.notes.iter().position(|n|n.note==60).unwrap();
        let edits:Vec<_>=track.notes.iter().enumerate().map(|(i,n)|fingerings::Edit{track_id,note_index:i,finger:Some(if n.note==65{4}else{3})}).collect();
        p.command(Command::EditFingersFor{content_id:content_id.clone(),edits}).unwrap();
        let request=fingerings::PlanRequest{content_id:content_id.clone(),track:track_id,index:source,hand:PracticePart::RightHand,profile:HandSpanProfile::Standard,range:Some((1,1)),keep_saved:true,practice_rate:1.,pins:vec![],target_tracks:vec![]};
        let prepared=fingerings::prepare(p.file.as_ref().unwrap(),&p.config,&p.fingers,&p.note_hands,request.clone()).unwrap();let plans=fingerings::held_plans(prepared).unwrap();let plan=&plans["plans"][0];
        let edits:Vec<_>=plan["proposals"].as_array().unwrap().iter().map(|n|json!({"track_id":n["track"],"note_index":n["index"],"finger":n["finger"]})).collect();
        p.command(serde_json::from_value(json!({"type":"acceptHeldFingerPlan","request":request,"fingerprint":plans["fingerprint"],"edits":edits,"actions":plan["substitutions"]})).unwrap()).unwrap();
        let fingers=p.saved_hints().unwrap();let actions=p.saved_actions().unwrap();let before=p.snapshot();let expected=actions[0].to;
        let context=p.prepare_held_demo(content_id.clone(),track_id,source,1.).unwrap();p.start_held_demo(content_id,track_id,source,context.baseline,1.).unwrap();
        let d=p.finger_demo.as_ref().unwrap();assert_eq!(d.events.len(),6);assert_eq!(d.events.iter().filter(|(_,_,b)|b[0]&240==144&&b[1]==60).count(),1);assert_eq!(d.events.iter().filter(|(_,_,b)|b[0]&240==128&&b[1]==60).count(),1);
        p.tick_finger_demo(Duration::from_millis(900));assert_eq!(p.finger_demo.as_ref().unwrap().cursor,1);
        let state=p.finger_demo_state();let note=state["fingerNotes"].as_array().unwrap().iter().find(|n|n["pitch"]==60).unwrap();assert_eq!(note["finger"],expected);assert_eq!(note["changed"],true);
        p.tick_finger_demo(Duration::from_millis(200));assert_eq!(p.finger_demo.as_ref().unwrap().cursor,2);
        assert_eq!(p.snapshot().position,before.position);assert_eq!(p.saved_hints().unwrap(),fingers);assert_eq!(p.saved_actions().unwrap(),actions);
        let id=p.finger_demo_state()["id"].as_str().unwrap().to_owned();p.command(Command::StopFingerDemo{id:Some("other".into())}).unwrap();assert!(p.finger_demo.is_some());p.command(Command::StopFingerDemo{id:Some(id)}).unwrap();assert!(p.finger_demo.is_none());
    }
    #[test]
    fn joint_demo_routes_each_event_to_its_original_track() {
        let data=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../work").join(format!("finger-joint-demo-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut player=Player::new(data,PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../default.sf2"),true);
        let attrs="<attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>";
        let xml=format!("<score-partwise version=\"4.0\"><part-list><score-part id=\"P1\"><part-name>旋律</part-name></score-part><score-part id=\"P2\"><part-name>持音</part-name></score-part></part-list><part id=\"P1\"><measure number=\"1\">{attrs}<note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note></measure></part><part id=\"P2\"><measure number=\"1\">{attrs}<note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note></measure></part></score-partwise>");
        player.command(Command::ImportScore{bytes:xml.into_bytes(),name:"联合示范.musicxml".into(),default_bpm:120}).unwrap();
        for track in &mut player.config.tracks {track.practice_part=PracticePart::RightHand;}
        player.note_hands.clear();
        let file=player.file.as_ref().unwrap();
        let melody=file.tracks.iter().find(|t|t.notes.iter().any(|n|n.note==64)).unwrap().track_id;
        let bass=file.tracks.iter().find(|t|t.notes.iter().any(|n|n.note==60)).unwrap().track_id;
        let request=fingerings::PlanRequest{content_id:file.content_id.clone(),track:melody,index:0,
            hand:PracticePart::RightHand,profile:HandSpanProfile::Standard,range:Some((1,1)),keep_saved:false,practice_rate:1.0,pins:vec![],target_tracks:vec![]};
        let prepared=fingerings::prepare(file,&player.config,&player.fingers,&player.note_hands,request.clone()).unwrap();
        player.start_finger_demo(request,prepared.fingerprint,0.5).unwrap();
        let events=&player.finger_demo.as_ref().unwrap().events;
        assert!(events.iter().any(|(_,track,bytes)|*track==bass && bytes[0]&240==144 && bytes[1]==60));
        assert!(events.iter().any(|(_,track,bytes)|*track==melody && bytes[0]&240==144 && bytes[1]==64));
        assert!(events.iter().any(|(_,track,bytes)|*track==bass && bytes[0]&240==128 && bytes[1]==60));
    }
    #[test]
    fn demo_preserves_practice_and_fingers_with_scoped_stop_and_lease() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../work")
            .join(format!(
                "finger-demo-test-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let mut player = Player::new(
            data,
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../default.sf2"),
            true,
        );
        player
            .command(Command::Generate {
                spec: ExerciseSpec::default(),
            })
            .unwrap();
        let file = player.file.as_ref().unwrap();
        let track = file
            .tracks
            .iter()
            .find(|t| {
                !t.notes.is_empty()
                    && crate::hands::part(&player.config, &player.note_hands, t.track_id, 0)
                        == PracticePart::RightHand
            })
            .unwrap()
            .track_id;
        let request = fingerings::PlanRequest {
            content_id: file.content_id.clone(),
            track,
            index: 0,
            profile: HandSpanProfile::Standard,
            hand: PracticePart::RightHand,
            range: Some((1, 1)),
            keep_saved: true,
            practice_rate: 1.0,
            pins: vec![],
            target_tracks: vec![],
        };
        let prepared = fingerings::prepare(
            file,
            &player.config,
            &player.fingers,
            &player.note_hands,
            request.clone(),
        )
        .unwrap();
        let before = serde_json::to_value(player.snapshot()).unwrap();
        let saved = player.fingers.clone();
        assert!(
            player
                .start_finger_demo(request.clone(), "stale".into(), 0.5)
                .is_err()
        );
        let demo = player
            .start_finger_demo(request.clone(), prepared.fingerprint.clone(), 0.5)
            .unwrap();
        assert_eq!(demo["active"], true);
        let initial = demo["position"].as_f64().unwrap();
        player.tick(Duration::from_millis(500));
        assert!(
            (player.finger_demo_state()["position"].as_f64().unwrap() - initial - 0.25).abs()
                < 0.00001
        );
        assert_eq!(
            serde_json::to_value(player.snapshot()).unwrap()["score"],
            before["score"]
        );
        assert_eq!(player.state.position, before["position"].as_f64().unwrap());
        assert_eq!(player.fingers, saved);
        player.midi(&[0xfe]);
        player.midi(&[0xf8]);
        assert_eq!(player.finger_demo_state()["active"],true);
        player
            .command(Command::StopFingerDemo {
                id: Some("older-demo".into()),
            })
            .unwrap();
        assert_eq!(player.finger_demo_state()["active"], true);
        player
            .command(Command::Note {
                pitch: 60,
                active: true,
                velocity: 80,
            })
            .unwrap();
        assert!(player.finger_demo.is_none());
        assert!(
            player
                .start_finger_demo(request.clone(), prepared.fingerprint.clone(), 0.5)
                .is_err()
        );
        player
            .command(Command::Note {
                pitch: 60,
                active: false,
                velocity: 0,
            })
            .unwrap();
        player
            .start_finger_demo(request.clone(), prepared.fingerprint.clone(), 0.25)
            .unwrap();
        player.tick(Duration::from_secs(4));
        assert!(player.finger_demo.is_none());
        player
            .start_finger_demo(request, prepared.fingerprint, 1.0)
            .unwrap();
        for _ in 0..40 {
            player.finger_demo_state();
            player.tick(Duration::from_millis(250));
        }
        let finished = player.finger_demo_state();
        assert_eq!(finished["active"], false);
        assert_eq!(finished["position"], finished["end"]);
        assert_eq!(player.fingers, saved);
    }
}
