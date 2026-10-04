import { test, expect } from "@playwright/test";
test("逐音分手建议可以审阅、保存、撤销并用于分手指法", async ({
  page,
  request,
}) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "技术练习", exact: true }).click();
  await page.getByRole("button", { name: "生成并打开" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator("h1")).toHaveText("C大调 · 音阶");
  const initial = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  await page.getByRole("button", { name: "逐音分手", exact: true }).click();
  await page.getByRole("button", { name: "生成分手建议" }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog.locator("tbody tr").first()).toBeVisible();
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-逐音分手.png",
      fullPage: true,
    });
  const preview = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  expect(preview.notes).toEqual(initial.notes);
  await page.getByRole("button", { name: "取消选择", exact: true }).click();
  await dialog.locator("tbody select").first().selectOption("LeftHand");
  await page.getByRole("button", { name: "保存所选分手" }).click();
  const changed = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  expect(
    changed.notes.some(
      (n: { part: string; manualHand: boolean }) =>
        n.part === "left" && n.manualHand,
    ),
  ).toBe(true);
  await page.getByRole("button", { name: "撤销上次分手修改" }).click();
  await expect
    .poll(async () => {
      const s = await (
        await request.get("http://127.0.0.1:32124/api/song")
      ).json();
      return s.notes.filter((n: { manualHand: boolean }) => n.manualHand)
        .length;
    })
    .toBe(0);
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
});
test("整曲拍号和弱起修正可以保存并恢复原始网格", async ({ page, request }) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "技术练习", exact: true }).click();
  await page.getByRole("button", { name: "生成并打开" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator("h1")).toHaveText("C大调 · 音阶");
  const initial = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  await page.getByRole("button", { name: "修正小节网格", exact: true }).click();
  await page.getByLabel("网格每小节拍数").fill("3");
  await page.getByLabel("网格弱起拍数").fill("1");
  await page.getByRole("button", { name: "保存小节网格" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  const changed = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  expect(changed.measures[0].partial).toBe(true);
  expect(changed.measures[0].endTick).toBe(initial.ppq);
  expect(changed.notes).toEqual(initial.notes);
  await page.getByRole("button", { name: "修正小节网格", exact: true }).click();
  await page.getByRole("button", { name: "恢复文件原始网格" }).click();
  const restored = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  expect(restored.measures).toEqual(initial.measures);
});
test("谱面版本保留、切换、重命名、解除关联和移除", async ({
  page,
  request,
}) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "技术练习", exact: true }).click();
  await page.getByRole("button", { name: "生成并打开" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator("h1")).toHaveText("C大调 · 音阶");
  await page.getByRole("button", { name: "谱面管理", exact: true }).click();
  const xml = Buffer.from(
    '<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note></measure></part></score-partwise>',
  );
  for (const name of ["原版.musicxml", "教学版.musicxml"]) {
    await page
      .getByLabel("添加谱面版本文件")
      .setInputFiles({ name, mimeType: "application/xml", buffer: xml });
    await expect(page.getByLabel(`谱面名称 ${name}`)).toBeVisible();
  }
  const dialog = page.getByRole("dialog");
  expect(await dialog.locator("tbody tr").count()).toBe(2);
  await dialog
    .locator("tbody tr")
    .first()
    .getByRole("button", { name: "使用此谱面" })
    .click();
  await expect(dialog.locator("tbody tr").first()).toContainText("当前使用");
  await page.getByLabel("谱面名称 原版.musicxml").fill("老师标注版");
  await dialog
    .locator("tbody tr")
    .first()
    .getByRole("button", { name: "保存名称" })
    .click();
  await expect(page.getByLabel("谱面名称 老师标注版")).toBeVisible();
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-谱面管理.png",
      fullPage: true,
    });
  await page.getByRole("button", { name: "解除当前谱面关联" }).click();
  await expect
    .poll(async () => {
      const s = await (
        await request.get("http://127.0.0.1:32124/api/song")
      ).json();
      return s.hasScore;
    })
    .toBe(false);
  await dialog
    .locator("tbody tr")
    .first()
    .getByRole("button", { name: "移除版本" })
    .click();
  await expect(dialog.locator("tbody tr")).toHaveCount(1);
  await page.keyboard.press("Escape");
});
