import { test, expect } from "@playwright/test";
test("常用计划排序、当天实际达标、下一项、跳过与日期隔离", async ({
  page,
  request,
}) => {
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(response.ok()).toBe(true);
    return response.json();
  };
  const state = async () =>
    await (await request.get("http://127.0.0.1:32124/api/state")).json();
  const xml =
    '<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure></part></score-partwise>';
  await command({
    type: "importScore",
    name: "日课界面练习.musicxml",
    bytes: Array.from(Buffer.from(xml)),
    default_bpm: 120,
  });
  await command({ type: "mode", value: "wait" });
  await command({ type: "countIn", bars: 0 });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "练习计划", exact: true }).click();
  await page.getByRole("button", { name: "常用计划", exact: true }).click();
  await page.getByLabel("练习计划名称").fill("每日日课");
  await page.getByRole("button", { name: "创建计划", exact: true }).click();
  await expect(page.locator(".routine-plan")).toHaveCount(1);
  for (const title of ["热身", "复习"]) {
    await page
      .getByRole("button", { name: "添加当前练习", exact: true })
      .click();
    await page.getByLabel("计划项目名称").fill(title);
    await page.getByLabel("计划项目练习要求").fill("轻松连奏，保持手腕放松");
    await page.getByRole("button", { name: "保存项目", exact: true }).click();
  }
  await expect(page.locator(".routine-item")).toHaveCount(2);
  await page.getByRole("button", { name: "上移", exact: true }).click();
  await expect(page.locator(".routine-item").first()).toContainText("复习");
  await page.getByRole("button", { name: "当天安排", exact: true }).click();
  await page.getByLabel("练习计划日期").fill("2026-10-02");
  await page.locator(".routine-item-select").first().click();
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-练习计划.png",
      fullPage: true,
    });
  await page.getByRole("button", { name: "打开本项练习", exact: true }).click();
  await expect(page.locator(".routine-dialog")).toHaveCount(0);
  await expect(page.locator(".routine-status")).toContainText("复习");
  await page.getByRole("button", { name: "开始练习", exact: true }).click();
  await expect
    .poll(
      async () => {
        const s = await state();
        for (const pitch of s.required) {
          await command({ type: "note", pitch, velocity: 90, active: true });
          await command({ type: "note", pitch, velocity: 0, active: false });
        }
        return (await state()).routine.progress.completed;
      },
      { timeout: 12000 },
    )
    .toBe(true);
  await expect(page.locator(".routine-status")).toContainText("本项已达标");
  await page.getByRole("button", { name: "打开下一项", exact: true }).click();
  await expect(page.locator(".routine-status")).toContainText("热身");
  await page.getByRole("button", { name: "查看安排", exact: true }).click();
  await page.locator(".routine-plan.selected").click();
  await page.getByLabel("练习计划日期").fill("2026-10-02");
  await page.locator(".routine-item-select").nth(1).click();
  await page.getByRole("button", { name: "本次跳过", exact: true }).click();
  await expect(page.locator(".routine-item").nth(1)).toContainText("已跳过");
  await expect(page.locator(".routine-items-heading")).toContainText(
    "1/2 项达标",
  );
  await page.getByLabel("练习计划日期").fill("2026-10-03");
  await expect(page.locator(".routine-items-heading")).toContainText(
    "0/2 项达标",
  );
});
