import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";

test("收藏、队列排序、下一首与设备窗口", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByLabel("曲库来源").selectOption("内置练习");
  await page
    .getByRole("listitem")
    .filter({ hasText: "五指热身 · 从中央 C 开始" })
    .click();
  await expect(
    page.getByRole("button", { name: /^(收藏曲目|取消收藏)$/ }),
  ).toBeEnabled();
  if (await page.getByRole("button", { name: "收藏曲目", exact: true }).count())
    await page.getByRole("button", { name: "收藏曲目", exact: true }).click();
  await page.getByRole("button", { name: "取消收藏", exact: true }).waitFor();
  if (await page.getByRole("button", { name: "加入队列", exact: true }).count())
    await page.getByRole("button", { name: "加入队列", exact: true }).click();
  await page.getByRole("button", { name: "收藏", exact: true }).click();
  await expect(
    page.getByRole("listitem").filter({ hasText: "五指热身 · 从中央 C 开始" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "全部", exact: true }).click();
  await page
    .getByRole("listitem")
    .filter({ hasText: "C 大调音阶 · 一个八度" })
    .click();
  await expect(
    page.getByRole("button", { name: /^(加入队列|移出队列)$/ }),
  ).toBeEnabled();
  if (await page.getByRole("button", { name: "加入队列", exact: true }).count())
    await page.getByRole("button", { name: "加入队列", exact: true }).click();
  await page
    .locator(".library-tabs button")
    .filter({ hasText: "队列" })
    .click();
  await expect(page.getByRole("listitem")).toHaveCount(2);
  await page.getByRole("button", { name: "上移", exact: true }).click();
  await expect(page.getByRole("listitem").first()).toContainText("C 大调音阶");
  await page.getByRole("button", { name: "队列下一首", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      name: "五指热身 · 从中央 C 开始",
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("button", { name: "设备设置", exact: true }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "开始练习", exact: true }).click();
  await expect(page.locator(".required-notes strong")).toHaveText("C4");
  const output = process.env.NEOTHESIA_SCREENSHOT_DIR;
  if (output)
    await page.screenshot({
      path: output + "/Neothesia-新版.png",
      fullPage: true,
    });
  await page.getByRole("button", { name: "暂停", exact: true }).click();
  await page.reload();
  await expect(
    page.getByRole("button", { name: "取消收藏", exact: true }),
  ).toBeVisible();
});

test("分段重复、每轮成绩与历史窗口", async ({ page, request }) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page
    .getByRole("listitem")
    .filter({ hasText: "五指热身 · 从中央 C 开始" })
    .click();
  await page.getByLabel("循环结束小节").fill("1");
  await page.getByRole("button", { name: "开始循环", exact: true }).click();
  await page.getByRole("button", { name: "开始练习", exact: true }).click();
  for (let round = 1; round <= 2; round++) {
    for (const [pitch, key] of [
      ["C4", "a"],
      ["D4", "s"],
      ["E4", "d"],
      ["F4", "f"],
    ]) {
      await expect(page.locator(".required-notes strong")).toHaveText(pitch);
      await page.locator("h1").click();
      await page.keyboard.press(key);
    }
    await expect(
      page.getByText(new RegExp(`已完成 ${round} 轮`)),
    ).toBeVisible();
  }
  await page.getByRole("button", { name: "暂停", exact: true }).click();
  await page.getByRole("button", { name: "练习历史", exact: true }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  if (process.env.NEOTHESIA_SCREENSHOT_DIR) {
    await page.screenshot({
      path: `${process.env.NEOTHESIA_SCREENSHOT_DIR}/Neothesia-练习历史.png`,
      fullPage: true,
    });
  }
  const history = await (
    await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data: { type: "history" },
    })
  ).json();
  expect(
    history.entries.filter(
      (e: { loop: boolean; score: { matched_notes: number } }) =>
        e.loop && e.score.matched_notes === 4,
    ).length,
  ).toBeGreaterThanOrEqual(2);
  await expect(page.locator(".history-entry").first()).toContainText(
    "分段练习",
  );
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("switch", { name: "分段循环" }).click();
});

test("真实 Rust 引擎：等待、暂停、演奏九个音符并保存结果", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page
    .getByRole("listitem")
    .filter({ hasText: "五指热身 · 从中央 C 开始" })
    .click();
  await page.getByRole("button", { name: "开始练习", exact: true }).click();
  await expect(page.locator(".required-notes strong")).toHaveText("C4");
  const before = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  await page.waitForTimeout(200);
  const waiting = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  expect(waiting.position).toBe(before.position);
  await page.reload();
  await expect(
    page.getByRole("heading", {
      name: "五指热身 · 从中央 C 开始",
      exact: true,
    }),
  ).toBeVisible();
  await expect(page.locator(".required-notes strong")).toHaveText("C4");
  await page.getByRole("button", { name: "暂停", exact: true }).click();
  await expect(page.locator(".stage-status")).toContainText("已暂停");
  await page.getByRole("button", { name: "开始练习", exact: true }).click();
  for (const [pitch, key] of [
    ["C4", "a"],
    ["D4", "s"],
    ["E4", "d"],
    ["F4", "f"],
    ["G4", "g"],
    ["F4", "f"],
    ["E4", "d"],
    ["D4", "s"],
    ["C4", "a"],
  ]) {
    await expect(page.locator(".required-notes strong")).toHaveText(pitch);
    await page.locator("h1").click();
    await page.keyboard.press(key);
    await expect
      .poll(
        async () =>
          (await (await request.get("http://127.0.0.1:32124/api/state")).json())
            .required,
      )
      .toEqual([]);
  }
  await expect(page.locator(".stage-status")).toContainText("本次练习完成");
  await expect(page.getByText("结果已保存在本机")).toBeVisible();
  const finished = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  expect(finished.score.matched_notes).toBe(9);
  expect(finished.score.wrong_notes).toBe(0);
  expect(finished.saved).toBe(true);
  const file = process.env.NEOTHESIA_TEST_HISTORY;
  if (file) {
    const history = readFileSync(file, "utf8");
    expect(history).toContain("matched_notes: 9");
  }
  await page.getByRole("button", { name: "从头开始" }).click();
  await expect
    .poll(
      async () =>
        (await (await request.get("http://127.0.0.1:32124/api/state")).json())
          .score.matched_notes,
    )
    .toBe(0);
  expect(errors).toEqual([]);
});

test("真实曲库搜索、来源显示、桌面与窄屏布局", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page
    .getByRole("combobox", { name: "曲库来源" })
    .selectOption("GiantMIDI-Piano");
  await page.getByRole("textbox", { name: "搜索曲目" }).fill("Chopin");
  await expect(page.locator(".song-row").first()).toBeVisible();
  await page.locator(".song-row").first().click();
  await expect(page.getByText("GiantMIDI · 自动转录，尚未校对")).toBeVisible();
  await page.getByText("来源与许可", { exact: true }).click();
  await expect(
    page.getByText("CC BY 4.0; see upstream disclaimer", { exact: true }),
  ).toBeVisible();
  await page.getByText("来源与许可", { exact: true }).click();
  const output = process.env.NEOTHESIA_SCREENSHOT_DIR;
  if (output)
    await page.screenshot({
      path: output + "/Neothesia-工作台.png",
      fullPage: true,
    });
  await page.setViewportSize({ width: 760, height: 1040 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  if (output)
    await page.screenshot({
      path: output + "/Neothesia-工作台-窄屏.png",
      fullPage: true,
    });
});
