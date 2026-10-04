import { test, expect } from "@playwright/test";
test("谱面准确音符指法编辑、推荐原因、清除撤销与重开保存", async ({
  page,
  request,
}) => {
  test.setTimeout(90000);
  test.skip(
    !process.env.NEOTHESIA_SCORE_FINGER_DATA,
    "需要独立乐谱指法测试服务",
  );
  const command = async (v: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data: v,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const xml = `<?xml version="1.0"?><score-partwise version="4.0"><work><work-title>指法课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><key><fifths>0</fifths></key><time><beats>4</beats><beat-type>4</beat-type></time><staves>2</staves><clef number="1"><sign>G</sign><line>2</line></clef><clef number="2"><sign>F</sign><line>4</line></clef></attributes><direction><sound tempo="60"/></direction>${["C", "D", "E", "F"].map((p, i) => `<note><pitch><step>${p}</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice><type>quarter</type><staff>1</staff>${i === 0 ? "<notations><technical><fingering>1</fingering></technical></notations>" : ""}</note>`).join("")}</measure><measure number="2"><note><pitch><step>G</step><octave>4</octave></pitch><duration>4</duration><voice>1</voice><type>whole</type><staff>1</staff></note></measure></part></score-partwise>`;
  await command({
    type: "importScore",
    bytes: Array.from(Buffer.from(xml)),
    name: "指法课堂.musicxml",
    default_bpm: 60,
  });
  const song = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  const firstNote = song.notes.find((n: any) => n.pitch === 60);
  await command({
    type: "editFingersFor",
    content_id: song.contentId,
    edits: [
      { track_id: firstNote.track, note_index: firstNote.index, finger: 1 },
    ],
  });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await expect(page.locator(".score-paper svg").first()).toBeVisible();
  await page.getByLabel("乐谱缩放").selectOption("75");
  const small = await page
    .locator(".score-paper")
    .evaluate((e) => e.getBoundingClientRect().width);
  await page.getByLabel("乐谱缩放").selectOption("150");
  expect(
    await page
      .locator(".score-paper")
      .evaluate((e) => e.getBoundingClientRect().width),
  ).toBeCloseTo(small * 2, 0);
  await page.getByLabel("指法显示密度").selectOption("important");
  await page.getByLabel("指法显示密度").selectOption("all");
  await page.getByLabel("乐谱缩放").selectOption("0");
  await expect(
    page.getByRole("button", { name: "谱面指法", exact: true }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "谱面指法", exact: true }).click();
  const notes = page.locator('.score-paper g.note[role="button"]');
  await expect(notes).toHaveCount(5);
  await notes.first().locator(".notehead").click();
  const editor = page.getByRole("complementary", { name: "谱面指法编辑" });
  await expect(editor).toBeVisible();
  await expect(editor.locator(".score-finger-values")).toContainText(
    "原谱指法1",
  );
  const note = song.notes.find((n: any) => n.pitch === 60);
  const current = async () => {
    const s = await (
      await request.get("http://127.0.0.1:32124/api/song")
    ).json();
    return s.notes.find(
      (n: any) => n.track === note.track && n.index === note.index,
    ).finger;
  };
  await editor.getByRole("button", { name: "5", exact: true }).click();
  await expect.poll(current).toBe(5);
  await expect(page.locator(".score-practice-finger").first()).toHaveText("5");
  await editor
    .getByRole("button", { name: "撤销上一笔指法", exact: true })
    .click();
  await expect.poll(current).toBe(1);
  await editor
    .getByRole("button", { name: "清除个人覆盖", exact: true })
    .click();
  await expect.poll(current).toBe(null);
  await expect(editor.locator(".score-finger-values")).toContainText(
    "原谱指法1",
  );
  await editor
    .getByRole("button", { name: "撤销上一笔指法", exact: true })
    .click();
  await expect.poll(current).toBe(1);
  await editor.getByLabel("谱音左右手").selectOption("RightHand");
  await expect(
    editor.getByRole("button", { name: "推荐此音指法", exact: true }),
  ).toBeEnabled();
  await editor.getByLabel("谱面指法手型").selectOption("Large");
  await editor
    .getByRole("button", { name: "推荐此音指法", exact: true })
    .click();
  await expect(editor.locator(".score-note-proposal")).toBeVisible();
  await expect(editor.locator(".score-note-proposal p")).not.toBeEmpty();
  const recommended = Number(
    (await editor.locator(".score-note-proposal strong").innerText()).match(
      /建议 (\d)/,
    )![1],
  );
  await page.screenshot({
    path: "../outputs/Neothesia-谱面指法.png",
    fullPage: true,
  });
  await editor
    .getByRole("button", { name: "采用推荐指法", exact: true })
    .click();
  await expect.poll(current).toBe(recommended);
  await page.getByLabel("练习指法", { exact: true }).uncheck();
  await expect(page.locator(".score-practice-finger")).toHaveCount(0);
  await page.getByLabel("练习指法", { exact: true }).check();
  await expect(page.locator(".score-practice-finger")).toHaveCount(1);
  await command({ type: "load", path: song.sourcePath, title: song.title });
  await expect.poll(current).toBe(recommended);
  await page.reload();
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await expect(page.locator(".score-practice-finger").first()).toHaveText(
    String(recommended),
  );
  const mismatch = await request.post("http://127.0.0.1:32124/api/command", {
    headers: { "X-Neothesia-Client": "web" },
    data: {
      type: "editFingersFor",
      content_id: "another-piece",
      edits: [{ track_id: note.track, note_index: note.index, finger: 2 }],
    },
  });
  expect(mismatch.status()).toBe(400);
  expect(await current()).toBe(recommended);
});

test("反复谱音的演奏位置独立指法、全部应用与一次撤销", async ({
  page,
  request,
}) => {
  test.setTimeout(45000);
  test.skip(!process.env.NEOTHESIA_SCORE_FINGER_DATA, "需要独立乐谱指法服务");
  const command = async (v: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data: v,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const xml =
    '<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><barline location="left"><repeat direction="forward"/></barline><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note><barline location="right"><repeat direction="backward"/></barline></measure></part></score-partwise>';
  await command({
    type: "importScore",
    bytes: Array.from(Buffer.from(xml)),
    name: "反复指法.musicxml",
    default_bpm: 60,
  });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "谱面指法", exact: true }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "谱面指法", exact: true }).click();
  await page.locator(".score-paper g.note .notehead").first().click();
  const editor = page.getByRole("complementary", { name: "谱面指法编辑" });
  const select = editor.getByLabel("谱音演奏位置");
  await expect(select.locator("option")).toHaveCount(2);
  await expect(select).toContainText("小节");
  const values = async () => {
    const v = await (
      await request.get("http://127.0.0.1:32124/api/song")
    ).json();
    return v.notes
      .filter((n: any) => n.pitch === 60)
      .sort((a: any, b: any) => a.start - b.start)
      .map((n: any) => n.finger);
  };
  await editor.getByRole("button", { name: "2", exact: true }).click();
  await select.selectOption({ index: 1 });
  await editor.getByRole("button", { name: "4", exact: true }).click();
  await expect.poll(values).toEqual([2, 4]);
  await expect(page.locator(".score-practice-finger").first()).toHaveText(
    "2/4",
  );
  await editor.getByRole("checkbox", { name: /应用到该谱音/ }).check();
  await editor.getByRole("button", { name: "5", exact: true }).click();
  await expect.poll(values).toEqual([5, 5]);
  await editor
    .getByRole("button", { name: "撤销上一笔指法", exact: true })
    .click();
  await expect.poll(values).toEqual([2, 4]);
});
