import { test, expect } from "@playwright/test";
test("保存阶梯、实际达标晋级、结束后继续和目标级完成", async ({
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
    name: "速度阶梯界面练习.musicxml",
    bytes: Array.from(Buffer.from(xml)),
    default_bpm: 120,
  });
  await command({ type: "mode", value: "wait" });
  await command({ type: "countIn", bars: 0 });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "速度阶梯方案", exact: true }).click();
  await page.getByLabel("阶梯方案名称").fill("右手提速");
  await page.getByLabel("阶梯起始速度").fill("50");
  await page.getByLabel("阶梯目标速度").fill("100");
  await page.getByLabel("阶梯速度步长").fill("50");
  await page.getByLabel("阶梯连续达标轮数").fill("1");
  await page.getByRole("button", { name: "保存方案", exact: true }).click();
  await expect(page.locator(".ladder-preset")).toHaveCount(1);
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path:
        process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-速度阶梯方案.png",
      fullPage: true,
    });
  await page
    .getByRole("button", { name: "从起始速度应用", exact: true })
    .click();
  await expect(page.locator(".ladder-dialog")).toHaveCount(0);
  await expect(page.locator(".ladder-status")).toContainText("第 1/2 级");
  const denied = await request.post("http://127.0.0.1:32124/api/command", {
    headers: { "X-Neothesia-Client": "web" },
    data: { type: "speed", value: 1 },
  });
  expect(denied.ok()).toBe(false);
  await page.getByRole("button", { name: "开始练习", exact: true }).click();
  await expect
    .poll(
      async () => {
        const s = await state();
        for (const pitch of s.required) {
          await command({ type: "note", pitch, velocity: 90, active: true });
          await command({ type: "note", pitch, velocity: 0, active: false });
        }
        return (await state()).ladder.progress.stage;
      },
      { timeout: 12000 },
    )
    .toBe(1);
  await page.getByRole("button", { name: "结束阶梯", exact: true }).click();
  await expect.poll(async () => (await state()).ladder.active).toBe(false);
  await page.getByRole("button", { name: "查看方案", exact: true }).click();
  await page.locator(".ladder-preset").click();
  await expect(page.locator(".ladder-checkpoint")).toContainText("第 2 级");
  await page.getByRole("button", { name: "继续保存进度", exact: true }).click();
  await expect(page.locator(".ladder-dialog")).toHaveCount(0);
  expect((await state()).speed).toBe(1);
  await page.getByRole("button", { name: "开始练习", exact: true }).click();
  await expect
    .poll(
      async () => {
        const s = await state();
        for (const pitch of s.required) {
          await command({ type: "note", pitch, velocity: 90, active: true });
          await command({ type: "note", pitch, velocity: 0, active: false });
        }
        return (await state()).ladder.progress.completed;
      },
      { timeout: 12000 },
    )
    .toBe(true);
  await expect(page.locator(".ladder-status")).toContainText("目标速度已达标");
  expect((await state()).status).toBe("finished");
});
