import { expect, test } from "@playwright/test";

const api = "http://127.0.0.1:32124";

const score = (jump: string, secondPitch: string, extraFine = "") =>
  `<score-partwise version="4.0"><work><work-title>文字返回练习</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">
  <measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><direction><sound tempo="60"/></direction><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note>${extraFine}</measure>
  <measure number="2"><note><pitch><step>${secondPitch}</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note><direction><direction-type><words>Fine</words></direction-type></direction></measure>
  <measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note>${jump}</measure>
  </part></score-partwise>`;

test("文字 D.C. al Fine 和文字 Fine 生成准确演奏，歧义预览不能替换当前谱面", async ({
  page,
  request,
}) => {
  test.setTimeout(90000);
  test.skip(!process.env.NEOTHESIA_TEXT_FINE_TEST_DATA, "需要独立的文字跳转测试服务");
  const post = (data: unknown) =>
    request.post(`${api}/api/command`, {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
  const command = async (data: unknown) => {
    const response = await post(data);
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const current = async () =>
    (await (await request.get(`${api}/api/song`)).json());

  const printedJump =
    '<direction><direction-type><words>D.C. al Fine</words></direction-type></direction>';
  const written = score(printedJump, "D");
  const imported = await command({
    type: "importScore",
    bytes: [...Buffer.from(written)],
    name: "文字返回练习.musicxml",
    default_bpm: 60,
  });
  expect(imported.notes.map((note: { pitch: number }) => note.pitch)).toEqual([
    60, 62, 64, 60, 62,
  ]);
  expect(imported.notes.map((note: { start: number }) => note.start)).toEqual([
    0, 1, 2, 3, 4,
  ]);
  const route = await command({ type: "score" });
  expect(route.route.complete).toBe(true);
  expect(route.route.visits.map((visit: { measure: number }) => visit.measure)).toEqual([
    0, 1, 2, 0, 1,
  ]);
  expect(route.route.visits.at(-1).end).toBe(5);
  await command({ type: "mode", value: "wait" });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await expect(page.locator('.score-paper g.note[role="button"]')).toHaveCount(3);
  await page.locator(".score-route summary").click();
  await page.getByLabel("原谱小节", { exact: true }).selectOption("1");
  await expect(page.getByLabel("原谱小节演奏位置").locator("option")).toHaveCount(2);
  await page.getByLabel("原谱小节演奏位置").selectOption("4");
  await page.getByRole("button", { name: "定位原谱小节", exact: true }).click();
  await expect.poll(async () =>
    (await (await request.get(`${api}/api/state`)).json()).position,
  ).toBeCloseTo(4, 3);
  await expect(page.locator(".score-route-sequence [aria-current=step]")).toHaveAttribute(
    "aria-label", "演奏第 5 段，原谱第 2 小节",
  );

  const duplicateFine = score(
    printedJump,
    "D",
    '<direction><direction-type><words>Fine</words></direction-type></direction>',
  );
  const preview = await command({
    type: "previewScoreBytesRoute",
    bytes: [...Buffer.from(duplicateFine)],
    name: "重复文字 Fine.musicxml",
    content_id: imported.contentId,
  });
  expect(preview.route.complete).toBe(false);
  expect(preview.diagnostics.join(" ")).toContain("唯一");
  await page.getByRole("button", { name: "谱面管理", exact: true }).click();
  await page.getByLabel("添加谱面版本文件").setInputFiles({
    name: "重复文字 Fine.musicxml",
    mimeType: "application/vnd.recordare.musicxml+xml",
    buffer: Buffer.from(duplicateFine),
  });
  const dialog = page.getByRole("dialog", { name: "谱面管理", exact: true });
  const candidate = dialog.getByRole("region", { name: "待添加谱面预览" });
  await expect(candidate).toContainText("顺序尚未确认");
  await expect(candidate).toContainText("唯一");
  await expect(candidate.getByRole("button", { name: "添加并使用" })).toBeDisabled();
  expect((await current()).contentId).toBe(imported.contentId);
  expect((await command({ type: "score" })).route.visits).toHaveLength(5);

  const soundJump = score('<sound dacapo="yes"/>', "F");
  const soundResult = await command({
    type: "importScore",
    bytes: [...Buffer.from(soundJump)],
    name: "声音返回与文字 Fine.musicxml",
    default_bpm: 60,
  });
  expect(soundResult.notes.map((note: { pitch: number }) => note.pitch)).toEqual([
    60, 65, 64, 60, 65,
  ]);
  expect(soundResult.notes.at(-1).start).toBe(4);
  expect((await command({ type: "score" })).route.visits.map((visit: { measure: number }) => visit.measure)).toEqual([
    0, 1, 2, 0, 1,
  ]);
});

test("数值 Fine 把返回后的末音延为指定拍数，谱面与曲长一致", async ({ page, request }) => {
  test.setTimeout(60000);
  test.skip(!process.env.NEOTHESIA_TEXT_FINE_TEST_DATA, "需要独立的文字跳转测试服务");
  const xml = `<score-partwise version="4.0"><work><work-title>数值 Fine 练习</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">
    <measure number="1"><attributes><divisions>480</divisions><time><beats>3</beats><beat-type>4</beat-type></time></attributes><direction><sound tempo="60"/></direction><note><pitch><step>C</step><octave>4</octave></pitch><duration>480</duration><type>quarter</type></note><note><pitch><step>D</step><octave>4</octave></pitch><duration>960</duration><type>half</type></note><direction><direction-type><words>Fine</words></direction-type><sound fine="3"/></direction></measure>
    <measure number="2"><note><pitch><step>E</step><octave>4</octave></pitch><duration>480</duration><type>quarter</type></note><direction><direction-type><words>D.C. al Fine</words></direction-type><sound dacapo="yes"/></direction></measure>
    </part></score-partwise>`;
  const response = await request.post(`${api}/api/command`, {
    headers: { "X-Neothesia-Client": "web" },
    data: { type: "importScore", bytes: [...Buffer.from(xml)], name: "数值 Fine 练习.musicxml", default_bpm: 60 },
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  const imported = await response.json();
  expect(imported.notes.map((note: { pitch: number }) => note.pitch)).toEqual([60, 62, 64, 60, 62]);
  expect(imported.notes.map((note: { start: number }) => note.start)).toEqual([0, 1, 3, 4, 5]);
  expect(imported.notes.at(-1).duration).toBeCloseTo(3, 3);
  expect(imported.duration).toBeCloseTo(8, 3);
  const routeResponse = await request.post(`${api}/api/command`, {
    headers: { "X-Neothesia-Client": "web" }, data: { type: "score" },
  });
  expect(routeResponse.ok(), await routeResponse.text()).toBeTruthy();
  const route = await routeResponse.json();
  expect(route.route.complete).toBe(true);
  expect(route.route.visits.map((visit: { measure: number }) => visit.measure)).toEqual([0, 1, 0]);
  expect(route.route.visits.at(-1).end).toBeCloseTo(8, 3);
  await page.goto("/");
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await expect(page.locator('.score-paper g.note[role="button"]')).toHaveCount(3);
});

test("印刷 D.S. al Fine 按 Segno 返回并在 Fine 结束", async ({ page, request }) => {
  test.setTimeout(60000);
  test.skip(!process.env.NEOTHESIA_TEXT_FINE_TEST_DATA, "需要独立的文字跳转测试服务");
  const xml = `<score-partwise version="4.0"><work><work-title>印刷反复练习</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">
    <measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><direction><sound tempo="60"/></direction><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note></measure>
    <measure number="2"><direction><direction-type><segno/></direction-type></direction><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note></measure>
    <measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note><direction><direction-type><words>Fine</words></direction-type></direction></measure>
    <measure number="4"><note><pitch><step>F</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note><direction><direction-type><words>D.S. al Fine</words></direction-type></direction></measure>
    </part></score-partwise>`;
  const response = await request.post(`${api}/api/command`, {
    headers: { "X-Neothesia-Client": "web" },
    data: { type: "importScore", bytes: [...Buffer.from(xml)], name: "印刷反复练习.musicxml", default_bpm: 60 },
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  const song = await response.json();
  expect(song.notes.map((note: { pitch: number }) => note.pitch)).toEqual([60, 62, 64, 65, 62, 64]);
  expect(song.notes.map((note: { start: number }) => note.start)).toEqual([0, 1, 2, 3, 4, 5]);
  const routeResponse = await request.post(`${api}/api/command`, {
    headers: { "X-Neothesia-Client": "web" }, data: { type: "score" },
  });
  expect(routeResponse.ok(), await routeResponse.text()).toBeTruthy();
  const scoreRoute = await routeResponse.json();
  expect(scoreRoute.route.complete).toBe(true);
  expect(scoreRoute.route.visits.map((visit: { measure: number }) => visit.measure)).toEqual([0, 1, 2, 3, 1, 2]);
  await page.goto("/");
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await expect(page.locator('.score-paper g.note[role="button"]')).toHaveCount(4);
});
