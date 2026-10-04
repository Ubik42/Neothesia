import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
test("小节拖选、循环和逐拍定位使用真实音乐位置", async ({ page, request }) => {
  const command = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok()).toBe(true);
    return r.json();
  };
  await command({
    type: "importScore",
    bytes: Array.from(
      readFileSync("../neothesia-core/src/score_performance_test.musicxml"),
    ),
    name: "导航练习.musicxml",
    default_bpm: 120,
  });
  await command({ type: "mode", value: "wait" });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  const cells = page.locator(".bar-cell");
  const a = await cells.nth(0).boundingBox(),
    b = await cells.nth(1).boundingBox();
  expect(a).not.toBeNull();
  expect(b).not.toBeNull();
  await page.mouse.move(a!.x + a!.width / 2, a!.y + a!.height / 2);
  await page.mouse.down();
  await page.mouse.move(b!.x + b!.width / 2, b!.y + b!.height / 2, {
    steps: 5,
  });
  await page.mouse.up();
  await expect(page.locator(".bar-selection")).toHaveText("选中 1–2 小节");
  await page.getByRole("button", { name: "循环选段", exact: true }).click();
  await expect(page.getByRole("switch", { name: "分段循环" })).toHaveAttribute(
    "aria-checked",
    "true",
  );
  await page.getByLabel("跳转小节").fill("2");
  await page.getByLabel("跳转拍位").fill("2");
  await page.getByRole("button", { name: "定位", exact: true }).click();
  const state = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  expect(state.measure).toBe(2);
  expect(state.beat).toBeCloseTo(2, 4);
  await page.getByRole("button", { name: "上一拍", exact: true }).click();
  await expect
    .poll(
      async () =>
        (await (await request.get("http://127.0.0.1:32124/api/state")).json())
          .beat,
    )
    .toBeCloseTo(1, 4);
  await page.getByRole("button", { name: "放大小节" }).click();
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-小节导航.png",
      fullPage: true,
    });
});
test("背谱隐藏提示，不中断完成演奏，自动展示成绩并可再演奏", async ({
  page,
  request,
}) => {
  const command = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok()).toBe(true);
    return r.json();
  };
  await command({ type: "countIn", bars: 0 });
  await command({ type: "speed", value: 2 });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "背谱", exact: true }).click();
  await expect(page.locator(".recital-caption")).toContainText("已隐藏");
  await expect(page.locator(".results")).toHaveCount(0);
  await expect(page.locator(".score-paper")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "编辑指法", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "开始练习", exact: true }).click();
  await expect(page.getByRole("dialog")).toContainText("演奏完成", {
    timeout: 12000,
  });
  const state = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  expect(state.status).toBe("finished");
  expect(state.mode).toBe("memory");
  expect(state.score.missed_notes).toBe(6);
  expect(state.passage).toBeNull();
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-背谱完成.png",
      fullPage: true,
    });
  await page.getByRole("button", { name: "再演奏一遍" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect
    .poll(
      async () =>
        (await (await request.get("http://127.0.0.1:32124/api/state")).json())
          .status,
    )
    .toBe("playing");
  await command({ type: "pause" });
});
