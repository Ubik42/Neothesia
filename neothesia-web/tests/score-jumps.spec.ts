import { test, expect } from "@playwright/test";

test("D.S. al Coda 原谱多次定位、准确跟随和各次指法独立保存", async ({
  page,
  request,
}) => {
  test.setTimeout(90000);
  test.skip(!process.env.NEOTHESIA_JUMP_TEST_DATA, "需要独立跳转乐谱测试服务");
  const command = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const notes = ["C", "D", "E", "F", "G", "A", "B"];
  const before = [
    "",
    '<direction><direction-type><segno/></direction-type><sound segno="S"/></direction>',
    "",
    "",
    "",
    '<direction><direction-type><coda/></direction-type><sound coda="C"/></direction>',
    "",
  ];
  const after = [
    "",
    "",
    '<direction><direction-type><words>To Coda</words></direction-type><sound tocoda="C"/></direction>',
    "",
    '<direction><direction-type><words>D.S. al Coda</words></direction-type><sound dalsegno="S"/></direction>',
    "",
    "",
  ];
  const xml = `<?xml version="1.0"?><score-partwise version="4.0"><work><work-title>返回与尾声练习</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">${notes.map((pitch, i) => `<measure number="${i + 1}">${i === 0 ? '<attributes><divisions>1</divisions><key><fifths>0</fifths></key><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><direction><sound tempo="60"/></direction>' : ""}${before[i]}<note><pitch><step>${pitch}</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note>${after[i]}</measure>`).join("")}</part></score-partwise>`;
  const song = await command({
    type: "importScore",
    name: "返回与尾声练习.musicxml",
    bytes: Array.from(Buffer.from(xml)),
    default_bpm: 120,
  });
  expect(song.notes.map((n: any) => n.pitch)).toEqual([
    60, 62, 64, 65, 67, 62, 64, 69, 71,
  ]);
  const asset = await command({ type: "score" });
  expect(asset.coverage).toBe(100);
  expect(asset.route.complete).toBe(true);
  expect(asset.route.visits.map((v: any) => v.measure)).toEqual([
    0, 1, 2, 3, 4, 1, 2, 5, 6,
  ]);
  expect(asset.route.visits[5].start).toBe(20);
  await command({ type: "mode", value: "wait" });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  const scoreNotes = page.locator('.score-paper g.note[role="button"]');
  await expect(scoreNotes).toHaveCount(7);
  await expect(
    page.getByText("配对 100% · 可跟随", { exact: true }),
  ).toBeVisible();
  await page.locator(".score-route summary").click();
  await page.getByLabel("原谱小节", { exact: true }).selectOption("1");
  await expect(
    page.getByLabel("原谱小节演奏位置").locator("option"),
  ).toHaveCount(2);
  await page.getByLabel("原谱小节演奏位置").selectOption("5");
  await page.getByRole("button", { name: "定位原谱小节", exact: true }).click();
  await expect
    .poll(
      async () =>
        (await (await request.get("http://127.0.0.1:32124/api/state")).json())
          .position,
    )
    .toBeCloseTo(20, 4);
  await expect(page.locator(".score-route summary")).toContainText(
    "原谱第 2 小节 · 第 6 段",
  );
  await expect(
    page.locator(".score-route-sequence [aria-current=step]"),
  ).toHaveAttribute("aria-label", "演奏第 6 段，原谱第 2 小节");
  await page.getByRole("button", { name: "循环这一段", exact: true }).click();
  await expect
    .poll(
      async () =>
        (await (await request.get("http://127.0.0.1:32124/api/state")).json())
          .passage,
    )
    .toEqual({ start: 20, end: 24 });
  await page.getByRole("button", { name: "谱面指法", exact: true }).click();
  await scoreNotes.nth(1).locator(".notehead").click();
  const editor = page.getByRole("complementary", { name: "谱面指法编辑" });
  await expect(editor.getByLabel("谱音演奏位置").locator("option")).toHaveCount(
    2,
  );
  await editor.getByRole("button", { name: "5", exact: true }).click();
  await expect
    .poll(async () =>
      (
        await (await request.get("http://127.0.0.1:32124/api/song")).json()
      ).notes
        .filter((n: any) => n.pitch === 62)
        .map((n: any) => n.finger),
    )
    .toEqual([null, 5]);
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path:
        process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-原谱返回与尾声.png",
      fullPage: true,
    });
  await command({ type: "load", path: song.sourcePath, title: song.title });
  const reopened = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  expect(
    reopened.notes.filter((n: any) => n.pitch === 62).map((n: any) => n.finger),
  ).toEqual([null, 5]);
  expect((await command({ type: "score" })).route.visits).toHaveLength(9);
});

test("小节中途返回显示实际拍位并按准确范围循环，坏跳转保留当前曲目", async ({
  page,
  request,
}) => {
  test.skip(!process.env.NEOTHESIA_JUMP_TEST_DATA, "需要独立跳转乐谱测试服务");
  const post = (data: unknown) =>
    request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
  const xml =
    '<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>2</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><direction><sound tempo="60"/></direction><note><pitch><step>C</step><octave>4</octave></pitch><duration>2</duration><type>half</type></note><sound segno="S"><offset>-1</offset></sound><sound fine="yes"/></measure><measure number="2"><direction><sound tempo="120"/></direction><note><pitch><step>E</step><octave>4</octave></pitch><duration>2</duration><type>half</type></note><direction><direction-type><words>D.S. al Fine</words></direction-type><sound dalsegno="S"/></direction></measure></part></score-partwise>';
  const response = await post({
    type: "importScore",
    name: "小节中途返回.musicxml",
    bytes: Array.from(Buffer.from(xml)),
    default_bpm: 120,
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  const song = await response.json();
  expect(song.notes.map((n: any) => [n.start, n.duration])).toEqual([
    [0, 2],
    [2, 1],
    [3, 1],
  ]);
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await expect(page.locator('.score-paper g.note[role="button"]')).toHaveCount(
    2,
  );
  await page.locator(".score-route summary").click();
  await page.getByLabel("原谱小节", { exact: true }).selectOption("0");
  await page.getByLabel("原谱小节演奏位置").selectOption("2");
  await expect(page.locator(".score-route p")).toContainText("第 2 拍开始");
  await page.getByRole("button", { name: "循环这一段", exact: true }).click();
  await expect
    .poll(
      async () =>
        (await (await request.get("http://127.0.0.1:32124/api/state")).json())
          .passage,
    )
    .toEqual({ start: 3, end: 4 });
  await expect(page.locator(".score-route summary")).toContainText(
    "第 2 拍开始",
  );
  const failed = await post({
    type: "importScore",
    name: "缺失目标.musicxml",
    bytes: Array.from(
      Buffer.from(xml.replace('dalsegno="S"', 'dalsegno="missing"')),
    ),
    default_bpm: 120,
  });
  expect(failed.status()).toBe(400);
  expect(await failed.text()).toContain("找不到对应的 segno 目标 missing");
  expect(
    (await (await request.get("http://127.0.0.1:32124/api/song")).json())
      .contentId,
  ).toBe(song.contentId);
});
