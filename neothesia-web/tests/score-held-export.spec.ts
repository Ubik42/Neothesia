import {test,expect} from "@playwright/test";
import {readFile} from "node:fs/promises";
test("持音换指标记谱：反复分歧、独立选项、实际下载和陈旧拒绝",async({page,request})=>{
 const raw=(data:unknown)=>request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data});
 const cmd=async(data:unknown)=>{const r=await raw(data);expect(r.ok(),await r.text()).toBeTruthy();return r.json();};
 const xml=`<score-partwise version="4.0"><work><work-title>反复持音换指课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>2</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><barline location="left"><repeat direction="forward"/></barline><note><pitch><step>C</step><octave>4</octave></pitch><duration>8</duration><voice>1</voice><type>whole</type></note><backup><duration>8</duration></backup><forward><duration>4</duration></forward><note><pitch><step>E</step><octave>4</octave></pitch><duration>2</duration><voice>2</voice><type>quarter</type></note><barline location="right"><repeat direction="backward" times="2"/></barline></measure></part></score-partwise>`;
 await cmd({type:"importScore",bytes:[...Buffer.from(xml)],name:"反复持音换指.musicxml",default_bpm:120});
 const song=await cmd({type:"currentSong"}),track=song.notes[0].track;
 expect(song.notes).toHaveLength(4);const sustained=song.notes.filter((n:any)=>n.pitch===60).sort((a:any,b:any)=>a.start-b.start);
 await cmd({type:"clearHeldFingerActions",content_id:song.contentId});await cmd({type:"speed",value:1});
 await cmd({type:"track",id:track,part:"right",mode:"human",visible:true});
 await cmd({type:"editFingersFor",content_id:song.contentId,edits:song.notes.map((n:any)=>({track_id:n.track,note_index:n.index,finger:3}))});
 const req={content_id:song.contentId,track,index:sustained[0].index,hand:"RightHand",profile:"Standard",range:[1,2],keep_saved:true,pins:[],practice_rate:1};
 const result=await cmd({type:"suggestHeldFingerPlans",request:req}),plan=result.plans[0];expect(plan.substitutions).toHaveLength(2);
 const actions=plan.substitutions.map((a:any,i:number)=>({...a,to:i?2:1}));
 await cmd({type:"acceptHeldFingerPlan",request:req,fingerprint:result.fingerprint,edits:plan.proposals.map((p:any)=>({track_id:p.track,note_index:p.index,finger:p.finger})),actions});

 const exportRequest={type:"exportAnnotatedScore",content_id:song.contentId,score_revision:song.scoreRevision,options:{fingers:false,hands:false,substitutions:true,repeatPolicy:"consistent"}};
 const skipped=await cmd(exportRequest);expect(skipped.substitutionActions).toBe(0);expect(skipped.conflicts[0].substitutions).toBeTruthy();
 const firstRequest={...exportRequest,options:{...exportRequest.options,repeatPolicy:"first"}};
 const first=await cmd(firstRequest);expect(first.substitutionActions).toBe(1);expect(first.fingerNotes).toBe(1);expect(first.handNotes).toBe(0);
 const download=await cmd({...firstRequest,download:true,fingerprint:first.fingerprint});
 const text=Buffer.from(download.bytes).toString();expect(text).toContain('substitution="yes"');expect(text).toContain('>3</fingering>');expect(text).toContain('>1</fingering>');expect(text).toContain('起音后 1.');expect(text).toContain('repeat direction="backward"');
 // Timing differences matter even when the two finger chains agree.
 const actual=await cmd({type:"currentSong"}),second=actual.fingerActions[1];
 const edit={contentId:song.contentId,track,index:sustained[1].index,originalTick:second.atTick,atTick:song.measures[1].startTick+Math.round(1.5*song.ppq),to:1,baseline:actual.fingerActionRevision,rate:1};
 const checked=await cmd({type:"reviewHeldActionEdit",request:edit});await cmd({type:"saveHeldActionEdit",request:edit,review:checked.review});
 const timing=await cmd(exportRequest);expect(timing.conflicts[0].substitutions).toBeTruthy();expect(timing.conflicts[0].positions[0].changes[0].to).toBe(1);expect(timing.conflicts[0].positions[1].changes[0].to).toBe(1);expect(timing.conflicts[0].positions[0].changes[0].offsetTick).not.toBe(timing.conflicts[0].positions[1].changes[0].offsetTick);
 expect((await raw({...firstRequest,download:true,fingerprint:first.fingerprint})).ok()).toBeFalsy();
 await page.goto("/");await page.getByRole("button",{name:"乐谱",exact:true}).click();await page.getByRole("button",{name:"导出标记谱",exact:true}).click();const dialog=page.getByRole("dialog",{name:"导出个人标记谱"});
 await expect(dialog.getByRole("checkbox",{name:"持音换指标记",exact:true})).toBeChecked();await expect(dialog).toContainText("3→1（起音后");
 await dialog.getByRole("checkbox",{name:"个人指法",exact:true}).uncheck();await dialog.getByRole("checkbox",{name:"逐音分手标记",exact:true}).uncheck();await dialog.getByLabel("反复段落导出方式").selectOption("first");
 await expect(dialog.locator(".score-export-summary")).toContainText("1 个动作 / 1 个谱音");
 const fileReady=page.waitForEvent("download");await dialog.getByRole("button",{name:"保存 MusicXML",exact:true}).click();const file=await fileReady;const bytes=await readFile((await file.path())!);expect(bytes.toString()).toContain('substitution="yes"');
 await page.screenshot({path:"../outputs/Neothesia-持音换指标记谱导出.png"});
 // Exported substitution digits must not replace the attack digit when read back.
 await dialog.getByRole("button",{name:"关闭导出谱",exact:true}).click();await cmd({type:"importScore",bytes:[...bytes],name:"换指标记导入校验.musicxml",default_bpm:120});const imported=await cmd({type:"currentSong"});expect(imported.notes.filter((n:any)=>n.pitch===60).map((n:any)=>n.finger)).toEqual([3,3]);
 // Return to the source and invalidate one saved source; only that occurrence is skipped.
 await cmd({type:"importScore",bytes:[...Buffer.from(xml)],name:"反复持音换指.musicxml",default_bpm:120});
 await cmd({type:"editFingersFor",content_id:song.contentId,edits:[{track_id:track,note_index:sustained[0].index,finger:5}]});
 const returned=await cmd({type:"currentSong"});firstRequest.score_revision=returned.scoreRevision;
 const invalid=await cmd(firstRequest);expect(invalid.invalidActions).toBeGreaterThan(0);expect(invalid.substitutionActions).toBe(0);
 const old165=await cmd({...firstRequest,options:{fingers:true,hands:true,repeatPolicy:"consistent"}});expect(old165.substitutionActions).toBe(0);
});
