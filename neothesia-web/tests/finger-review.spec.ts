import { test, expect } from "@playwright/test";

test("按速度推荐、固定穿指复核、定位、重算与保存", async ({ page, request }) => {
  const raw = (data: unknown) => request.post("http://127.0.0.1:32124/api/command", {
    headers: { "X-Neothesia-Client": "web" }, data,
  });
  const command = async (data: unknown) => { const r = await raw(data); expect(r.ok(), await r.text()).toBeTruthy(); return r.json(); };
  const xml = `<score-partwise version="4.0"><work><work-title>快速穿指审阅</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes>${["E","F","G","A"].map(step => `<note><pitch><step>${step}</step><octave>4</octave></pitch><duration>1</duration><type>16th</type></note>`).join("")}<note><rest/><duration>12</duration><type>half</type><dot/></note></measure></part></score-partwise>`;
  await command({ type:"importScore", bytes:Array.from(Buffer.from(xml)), name:"快速穿指.musicxml", default_bpm:120 });
  const song = await (await request.get("http://127.0.0.1:32124/api/song")).json();
  const track = song.notes[0].track;
  await command({ type:"track", id:track, mode:"human", part:"right", visible:true });
  const notes = [...song.notes].sort((a:any,b:any) => a.start-b.start || a.pitch-b.pitch);
  await command({ type:"editFingersFor", content_id:song.contentId, edits:notes.map((n:any,i:number) => ({track_id:track,note_index:n.index,finger:[3,1,2,3][i]})) });
  const req = { content_id:song.contentId,track,index:notes[0].index,hand:"RightHand",profile:"Standard",range:[1,1],keep_saved:true,pins:[],practice_rate:1 };
  const fast = await command({type:"suggestFingerPlans",request:req});
  expect(fast.plans[0].review.some((r:any) => r.kind === "rapidTurn" && r.intervalMs === 125)).toBeTruthy();
  const slow = await command({type:"suggestFingerPlans",request:{...req,practice_rate:0.5}});
  expect(slow.plans[0].review).toHaveLength(0);
  expect(fast.plans[0].cost).toBeGreaterThan(slow.plans[0].cost);
  const stale = await raw({type:"acceptFingerPlan",request:{...req,practice_rate:0.5},fingerprint:fast.fingerprint,edits:[]});
  expect(stale.ok()).toBeFalsy();
  const invalid = await raw({type:"suggestFingerPlans",request:{...req,practice_rate:0}});
  expect(invalid.ok()).toBeFalsy();
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接",{exact:true})).toBeVisible();
  await page.getByRole("button",{name:"指法建议",exact:true}).click();
  const dialog=page.getByRole("dialog",{name:"指法建议"});
  await dialog.getByLabel("指法起始小节").fill("1");
  await dialog.getByLabel("指法结束小节").fill("1");
  await dialog.getByRole("button",{name:"生成建议",exact:true}).click();
  const review=dialog.getByRole("region",{name:"指法难点复核"});
  await expect(review.getByRole("button",{name:/快速穿 \/ 跨指 · 125 毫秒/})).toBeVisible();
  await review.getByRole("button",{name:/快速穿 \/ 跨指 · 125 毫秒/}).click();
  await expect(dialog.locator("tbody tr.finger-preview-row")).toHaveCount(1);
  await page.screenshot({path:"../outputs/Neothesia-指法速度与难点审阅.png",fullPage:true});
  await page.setViewportSize({width:900,height:800});
  expect(await review.evaluate(el=>el.scrollWidth <= el.clientWidth+1)).toBeTruthy();
  await dialog.getByLabel("指法推荐速度").selectOption("0.5");
  await expect(review).toHaveCount(0);
  await dialog.getByRole("button",{name:"生成建议",exact:true}).click();
  await expect(review.getByText("当前规则没有发现上述难点；仍可逐位置审阅。")).toBeVisible();
  await dialog.getByRole("button",{name:/接受并保存/}).click();
  await expect(dialog).toHaveCount(0);
  const saved=await (await request.get("http://127.0.0.1:32124/api/song")).json();
  expect(saved.notes.map((n:any)=>n.finger)).toEqual(song.notes.map((n:any)=>[3,1,2,3][notes.findIndex((o:any)=>o.index===n.index)]));
});

test("较小手型的固定八度仍复核，按手型重算并定位", async ({page,request})=>{
  const command=async(data:unknown)=>{const r=await request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data});expect(r.ok(),await r.text()).toBeTruthy();return r.json();};
  const xml=`<score-partwise version="4.0"><work><work-title>八度手型复核</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note><note><chord/><pitch><step>C</step><octave>5</octave></pitch><duration>4</duration><type>whole</type></note></measure></part></score-partwise>`;
  await command({type:"importScore",bytes:Array.from(Buffer.from(xml)),name:"八度复核.musicxml",default_bpm:120});
  const song=await(await request.get("http://127.0.0.1:32124/api/song")).json();
  const track=song.notes[0].track;
  await command({type:"track",id:track,mode:"human",part:"right",visible:true});
  await command({type:"editFingersFor",content_id:song.contentId,edits:song.notes.map((n:any)=>({track_id:track,note_index:n.index,finger:n.pitch===60?1:5}))});
  await page.goto("/");await expect(page.getByText("本地引擎已连接",{exact:true})).toBeVisible();
  await page.getByRole("button",{name:"指法建议",exact:true}).click();
  const d=page.getByRole("dialog",{name:"指法建议"});
  await d.getByLabel("指法起始小节").fill("1");await d.getByLabel("指法结束小节").fill("1");
  await d.getByLabel("指法手跨度").selectOption("Compact");await d.getByRole("button",{name:"生成建议",exact:true}).click();
  const review=d.getByRole("region",{name:"指法难点复核"});
  await expect(review.getByRole("button",{name:/跨度 12 半音，当前手型 10 半音/})).toBeVisible();
  await review.getByRole("button",{name:/跨度 12 半音/}).click();
  await expect(d.locator("tbody tr.finger-preview-row")).toHaveCount(2);
  await d.getByLabel("指法手跨度").selectOption("Standard");await d.getByRole("button",{name:"生成建议",exact:true}).click();
  await expect(review.getByText("当前规则没有发现上述难点；仍可逐位置审阅。")).toBeVisible();
});
