import {test,expect} from "@playwright/test";
test("双手建议独立跨度、逐音修改、审阅保存与整笔撤销",async({page,request})=>{
 const cmd=async(data:unknown)=>{const r=await request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data});expect(r.ok(),await r.text()).toBeTruthy();return r.json()};
 await cmd({type:"generate",spec:{tonic:2,tonality:"Major",minor_form:"Natural",pattern:"Scale",direction:"UpAndDown",hands:"Both",octaves:1,repetitions:1,tempo_bpm:120}});await page.goto("/");await page.getByRole("button",{name:"指法建议",exact:true}).click();await page.getByRole("button",{name:"双手一起审阅",exact:true}).click();const d=page.getByRole("dialog",{name:"双手指法审阅",exact:true});await d.getByLabel("左手推荐跨度").selectOption("Compact");await d.getByLabel("右手推荐跨度").selectOption("Large");await d.getByRole("button",{name:"生成双手建议",exact:true}).click();await expect(d.locator(".finger-pair-hands>section")).toHaveCount(2);const input=d.getByRole("spinbutton",{name:/右手指法 /}).first();const finger=await input.inputValue();await input.fill("9");await d.getByRole("button",{name:"核对双手保存结果",exact:true}).click();await expect(d.getByRole("alert")).toContainText("1 至 5");await expect(d.getByRole("button",{name:"保存双手指法",exact:true})).toBeDisabled();await input.fill(finger);await d.getByRole("checkbox",{name:/右手采用 /}).first().uncheck();await d.getByRole("button",{name:"核对双手保存结果",exact:true}).click();await expect(d.locator(".finger-pair-review")).toContainText("检查通过");await d.getByLabel("确认双手保留与待复核位置").check();page.once("dialog",d=>d.dismiss());await page.keyboard.press("Escape");await expect(d).toBeVisible();await page.screenshot({path:"../outputs/Neothesia-双手指法审阅.png"});await page.setViewportSize({width:700,height:900});await page.screenshot({path:"../outputs/Neothesia-双手指法审阅-窄窗.png"});await page.setViewportSize({width:1440,height:960});await d.getByRole("button",{name:"保存双手指法",exact:true}).click();await expect(d).toHaveCount(0);const song=await cmd({type:"currentSong"});expect(song.fingerUndoAvailable).toBeTruthy();await cmd({type:"undoFingersFor",content_id:song.contentId});expect((await cmd({type:"currentSong"})).fingerUndoAvailable).toBeFalsy();
});

test("双手未标记音可逐个采用现有建议并重做审阅", async ({ page, request }) => {
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const xml = `<score-partwise version="4.0"><work><work-title>双手覆盖审阅</work-title></work><part-list><score-part id="P1"><part-name>Right</part-name></score-part><score-part id="P2"><part-name>Left</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>2</duration><type>half</type></note><note><pitch><step>D</step><octave>4</octave></pitch><duration>2</duration><type>half</type></note></measure></part><part id="P2"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>F</sign><line>4</line></clef></attributes><note><pitch><step>C</step><octave>3</octave></pitch><duration>2</duration><type>half</type></note><note><pitch><step>D</step><octave>3</octave></pitch><duration>2</duration><type>half</type></note></measure></part></score-partwise>`;
  await command({ type: "importScore", bytes: Array.from(Buffer.from(xml)), name: "双手覆盖审阅.musicxml", default_bpm: 120 });
  const song = await command({ type: "currentSong" });
  const left = song.notes.find((n: { pitch: number }) => n.pitch === 48)?.track;
  const right = song.notes.find((n: { pitch: number }) => n.pitch === 60)?.track;
  expect(left).not.toBe(right);
  await command({ type: "track", id: left, mode: "human", part: "left", visible: true });
  await command({ type: "track", id: right, mode: "human", part: "right", visible: true });
  await page.goto("/");
  await page.getByRole("button", { name: "指法建议", exact: true }).click();
  await page.getByRole("button", { name: "双手一起审阅", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "双手指法审阅" });
  await dialog.getByRole("button", { name: "生成双手建议" }).click();
  await dialog.getByRole("checkbox", { name: /右手采用 / }).first().uncheck();
  await dialog.getByRole("button", { name: "核对双手保存结果" }).click();
  const coverage = dialog.getByRole("region", { name: "未标记指法衔接清单" });
  await expect(coverage).toBeVisible();
  await expect(coverage.getByText(/第 \d+ 小节/).first()).toBeVisible();
  await coverage.getByRole("button", { name: "采用这条建议" }).first().click();
  await expect(coverage).toHaveCount(0);
  await expect(dialog.locator(".finger-pair-review")).toContainText("已标记部分的同时按键检查通过");
});
