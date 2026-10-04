import {test,expect} from "@playwright/test";
test("持音换指动作计划保留起音、原件身份与只读参考",async({request,page})=>{
 const raw=(data:unknown)=>request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data});
 const cmd=async(data:unknown)=>{const r=await raw(data);expect(r.ok(),await r.text()).toBeTruthy();return r.json();};
 const xml=`<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>2</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>8</duration><voice>1</voice></note><backup><duration>8</duration></backup><forward><duration>4</duration></forward><note><pitch><step>E</step><octave>4</octave></pitch><duration>2</duration><voice>2</voice></note></measure></part></score-partwise>`;
 await cmd({type:"importScore",bytes:[...Buffer.from(xml)],name:"持音换指课堂.musicxml",default_bpm:120});
 const song=await cmd({type:"currentSong"}),notes=[...song.notes].sort((a:any,b:any)=>a.start-b.start),track=notes[0].track;
 await cmd({type:"clearHeldFingerActions",content_id:song.contentId});
 await cmd({type:"track",id:track,part:"right",mode:"human",visible:true});
 await cmd({type:"editFingersFor",content_id:song.contentId,edits:notes.map((n:any)=>({track_id:track,note_index:n.index,finger:3}))});
 await cmd({type:"speed",value:1});
 const req={content_id:song.contentId,track,index:notes[0].index,hand:"RightHand",profile:"Standard",range:[1,1],keep_saved:true,pins:[],practice_rate:1};
 expect((await raw({type:"suggestFingerPlans",request:req})).ok()).toBeFalsy();
 const before=await cmd({type:"currentSong"}),state=await (await request.get("http://127.0.0.1:32124/api/state")).json();
 const plans=await cmd({type:"suggestHeldFingerPlans",request:req});expect(plans.readOnly).toBeTruthy();expect(plans.fingerprint).toBeTruthy();expect(plans.plans.length).toBeGreaterThan(0);
 for(const plan of plans.plans){expect(plan.proposals.map((p:any)=>p.finger)).toEqual([3,3]);expect(plan.substitutions).toHaveLength(1);expect(plan.substitutions[0]).toMatchObject({track,index:notes[0].index,beforeTrack:track,beforeIndex:notes[1].index,from:3,pitch:60});expect(plan.substitutions[0].at).toBeCloseTo(0.88);expect([1,2]).toContain(plan.substitutions[0].to);}
 expect((await cmd({type:"currentSong"})).notes).toEqual(before.notes);
 const afterState=await (await request.get("http://127.0.0.1:32124/api/state")).json();expect(afterState.score).toEqual(state.score);expect(afterState.status).toBe(state.status);
 expect((await raw({type:"suggestHeldFingerPlans",request:{...req,index:notes[1].index,range:null}})).ok()).toBeFalsy();
 const plan=plans.plans[0],edits=plan.proposals.map((p:any)=>({track_id:p.track,note_index:p.index,finger:p.finger}));
 expect((await raw({type:"acceptHeldFingerPlan",request:req,fingerprint:plans.fingerprint,edits:edits.slice(1),actions:plan.substitutions})).ok()).toBeFalsy();
 expect((await cmd({type:"currentSong"})).fingerActions).toEqual([]);
 await cmd({type:"acceptHeldFingerPlan",request:req,fingerprint:plans.fingerprint,edits,actions:plan.substitutions});
 let saved=await cmd({type:"currentSong"});expect(saved.fingerActions).toHaveLength(1);expect(saved.fingerActions[0]).toMatchObject({valid:true,from:3,to:plan.substitutions[0].to,validationRate:1});expect(saved.notes.map((n:any)=>n.finger)).toEqual([3,3]);
 await cmd({type:"undoFingersFor",content_id:song.contentId});expect((await cmd({type:"currentSong"})).fingerActions).toEqual([]);
 await page.goto("/");await expect(page.getByText("本地引擎已连接")).toBeVisible();
 await page.getByRole("button",{name:"指法建议",exact:true}).click();
 await page.getByRole("button",{name:"寻找持音换指方案"}).click();
 await expect(page.getByRole("button",{name:"接受完整换指方案"})).toBeEnabled();
 await expect(page.getByLabel("持音换指").getByText(/保持按下/)).toHaveCount(1);
 await page.screenshot({path:"../outputs/Neothesia-持音换指方案.png"});
 await page.getByRole("button",{name:"接受完整换指方案"}).click();
 await expect(page.getByLabel("持音换指").getByText("已保存 · 1 次换指")).toBeVisible();
 await page.getByRole("button",{name:"关闭指法建议"}).click();
 await cmd({type:"seek",position:0.7});await page.waitForTimeout(250);await page.screenshot({path:"../outputs/Neothesia-换指前键盘.png"});
 await cmd({type:"seek",position:0.95});await page.waitForTimeout(250);await page.screenshot({path:"../outputs/Neothesia-换指后键盘.png"});

 saved=await cmd({type:"currentSong"});await cmd({type:"load",path:saved.sourcePath,title:saved.title});expect((await cmd({type:"currentSong"})).fingerActions[0].valid).toBeTruthy();
 await cmd({type:"clearHeldFingerActions",content_id:song.contentId});expect((await cmd({type:"currentSong"})).fingerActions).toEqual([]);
 await cmd({type:"undoFingersFor",content_id:song.contentId});expect((await cmd({type:"currentSong"})).fingerActions).toHaveLength(1);
 await cmd({type:"editFingersFor",content_id:song.contentId,edits:[{track_id:track,note_index:notes[0].index,finger:5}]});expect((await cmd({type:"currentSong"})).fingerActions[0].valid).toBeFalsy();
 await cmd({type:"undoFingersFor",content_id:song.contentId});expect((await cmd({type:"currentSong"})).fingerActions[0].valid).toBeTruthy();
 const packed=await cmd({type:"exportPackage"});expect(packed.summary.fingerActions).toBe(1);
 expect((await cmd({type:"inspectPackage",bytes:packed.bytes})).fingerActions).toBe(1);
 await cmd({type:"clearHeldFingerActions",content_id:song.contentId});
 await cmd({type:"importPackage",bytes:packed.bytes,policy:"replace"});
 expect((await cmd({type:"currentSong"})).fingerActions[0]).toMatchObject({from:3,to:plan.substitutions[0].to,valid:true});


});
