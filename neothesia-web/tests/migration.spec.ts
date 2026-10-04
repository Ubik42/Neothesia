import { test, expect } from "@playwright/test";
const base = "http://127.0.0.1:32124";
const xml = `<?xml version="1.0" encoding="utf-8"?><score-partwise version="4.0"><work><work-title>五指练习</work-title></work><part-list><score-part id="P1"><part-name>钢琴</part-name></score-part></part-list><part id="P1">${[["C", "D", "E", "F"], ["G", "F", "E", "D"], ["C"]].map((notes, i) => `<measure number="${i + 1}">${i === 0 ? '<attributes><divisions>480</divisions><key><fifths>0</fifths></key><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><direction><sound tempo="100"/></direction>' : ""}${notes.map((step) => `<note><pitch><step>${step}</step><octave>4</octave></pitch><duration>480</duration><type>quarter</type></note>`).join("")}</measure>`).join("")}</part></score-partwise>`;
test("生成器、真实声部、小节循环、连续漏音与反馈", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "技术练习", exact: true }).click();
  await page.getByLabel("主音", { exact: true }).selectOption("2");
  await page.getByLabel("练习类型").selectOption("Arpeggio");
  await page.getByRole("button", { name: "生成并打开" }).click();
  await expect(page.locator("h1")).toHaveText("D大调 · 琶音");
  await expect(page.getByLabel("音轨 2声部")).toHaveValue("right");
  await expect(page.getByLabel("音轨 3声部")).toHaveValue("left");
  await page.getByLabel("预备拍", { exact: true }).selectOption("0");
  await page.getByRole("button", { name: "连续", exact: true }).click();
  await page.getByLabel("循环结束小节").fill("1");
  await page.getByRole("button", { name: "开始循环", exact: true }).click();
  await page.getByRole("button", { name: "开始练习", exact: true }).click();
  await expect(page.getByText("已完成 1 轮", { exact: true })).toBeVisible({
    timeout: 12000,
  });
  await page.getByRole("button", { name: "暂停", exact: true }).click();
  await page.getByRole("button", { name: "逐小节反馈", exact: true }).click();
  await expect(page.getByRole("dialog")).toContainText("漏音");
  await expect(
    page.getByRole("button", { name: "重练第 1 小节", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  const data = await (await request.get(base + "/api/song")).json();
  expect(data.generated).toBe(true);
  expect(
    data.notes.some((n: { finger: number | null }) => n.finger !== null),
  ).toBe(true);
  expect(errors).toEqual([]);
});
test("MusicXML 配对、WASM 排版与原生音符映射", async ({ page, request }) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByLabel("曲库来源").selectOption("内置练习");
  await page
    .getByRole("listitem")
    .filter({ hasText: "五指热身 · 从中央 C 开始" })
    .click();
  await expect(
    page.getByRole("button", { name: "配对乐谱", exact: true }),
  ).toBeEnabled();
  await page
    .getByLabel("配对乐谱文件")
    .setInputFiles({
      name: "five-finger.musicxml",
      mimeType: "application/xml",
      buffer: Buffer.from(xml),
    });
  await expect(page.locator(".score-paper > svg")).toBeVisible({
    timeout: 20000,
  });
  await expect(page.locator(".score-paper g.note")).toHaveCount(9);
  await expect(page.getByRole("checkbox", { name: "跟随演奏" })).toBeEnabled();
  await expect(page.locator(".notation-toolbar")).toContainText("配对 100%");
  const result = await (
    await request.post(base + "/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data: { type: "score" },
    })
  ).json();
  expect(result.readiness).toBe("Ready");
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-乐谱.png",
      fullPage: true,
    });
  expect(errors).toEqual([]);
});
test("自由演奏录音、MIDI 导出与最近记录", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "录制演奏", exact: true }).click();
  await page.locator("h1").click();
  await page.keyboard.press("a");
  await page.keyboard.press("s");
  await page.keyboard.press("d");
  await page.getByRole("button", { name: /停止录音/ }).click();
  await expect(page.locator("h1")).toHaveText("自由演奏录音");
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 MIDI", exact: true }).click();
  expect((await download).suggestedFilename()).toBe("自由演奏录音.mid");
  await page.getByRole("button", { name: "最近", exact: true }).click();
  await expect(
    page.getByRole("listitem").filter({ hasText: "自由演奏录音" }),
  ).toBeVisible();
});
