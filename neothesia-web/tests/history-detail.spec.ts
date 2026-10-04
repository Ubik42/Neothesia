import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
test("历史筛选、同条件详情、恢复设置和单小节重练", async ({
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
  await command({
    type: "importMidi",
    bytes: Array.from(
      readFileSync("../neothesia-engine/assets/five-finger.mid"),
    ),
    name: "历史详情练习.mid",
  });
  await command({ type: "mode", value: "wait" });
  await command({ type: "countIn", bars: 0 });
  for (const speed of [0.6, 0.6, 0.8]) {
    await command({ type: "speed", value: speed });
    await command({ type: "restart" });
    await command({ type: "play" });
    await expect
      .poll(
        async () =>
          (await (await request.get("http://127.0.0.1:32124/api/state")).json())
            .required.length,
      )
      .toBeGreaterThan(0);
    await command({ type: "note", pitch: 60, velocity: 90, active: true });
    await command({ type: "note", pitch: 60, velocity: 0, active: false });
    await command({ type: "save" });
    await command({ type: "pause" });
  }
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "练习历史", exact: true }).click();
  await page.getByLabel("查找历史曲目").fill("历史详情练习");
  await expect(page.locator(".history-record")).toHaveCount(3);
  await page.getByLabel("历史练习模式").selectOption("memory");
  await expect(page.getByText("没有符合条件的记录。")).toBeVisible();
  await page.getByLabel("历史练习模式").selectOption("wait");
  await expect(page.locator(".history-record")).toHaveCount(3);
  await page.locator(".history-record").nth(1).click();
  await expect(page.locator(".history-comparison")).toContainText("共 1 次");
  await expect(
    page.getByRole("img", { name: /同条件正确率走势/ }),
  ).toBeVisible();
  await expect(page.locator(".history-bar-table tbody tr")).not.toHaveCount(0);
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-历史详情.png",
      fullPage: true,
    });
  await page.getByRole("button", { name: "恢复这次练习", exact: true }).click();
  await expect(page.locator(".history-workspace")).toHaveCount(0);
  let state = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  expect(state.speed).toBeCloseTo(0.6);
  expect(state.mode).toBe("wait");
  expect(state.status).toBe("ready");
  await page.getByRole("button", { name: "练习历史", exact: true }).click();
  await page.getByLabel("查找历史曲目").fill("历史详情练习");
  await expect(page.locator(".history-record")).toHaveCount(3);
  await page.locator(".history-record").first().click();
  await expect(
    page.getByText("此前没有可比较记录。", { exact: false }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "重练第 1 小节", exact: true })
    .click();
  await expect(page.locator(".history-workspace")).toHaveCount(0);
  state = await (await request.get("http://127.0.0.1:32124/api/state")).json();
  expect(state.mode).toBe("flow");
  expect(state.speed).toBeCloseTo(0.8);
  expect(state.passage).not.toBeNull();
});
