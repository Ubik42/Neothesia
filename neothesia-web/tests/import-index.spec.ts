import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
test("直接导入乐谱与 MIDI，失败文件不阻止后续文件，原谱可显示", async ({
  page,
  request,
}) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "导入曲目", exact: true }).click();
  const xml = readFileSync(
      "../neothesia-core/src/score_performance_test.musicxml",
    ),
    midi = readFileSync("../neothesia-engine/assets/basic-chords.mid");
  await page.getByLabel("导入曲目文件").setInputFiles([
    {
      name: "错误乐谱.xml",
      mimeType: "application/xml",
      buffer: Buffer.from("not a score"),
    },
    { name: "弱起连音.musicxml", mimeType: "application/xml", buffer: xml },
    {
      name: "弱起连音.mxl",
      mimeType: "application/vnd.recordare.musicxml",
      buffer: readFileSync("tests/fixtures/pickup-ties.mxl"),
    },
    { name: "和弦练习.mid", mimeType: "audio/midi", buffer: midi },
  ]);
  await page.getByRole("button", { name: "开始导入" }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("已完成 3 / 4");
  await expect(dialog.locator("tbody tr").first()).toContainText(
    /XML|score|谱|root/,
  );
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-多文件导入.png",
      fullPage: true,
    });
  await dialog
    .locator("tbody tr")
    .nth(1)
    .getByRole("button", { name: "打开练习" })
    .click();
  await expect(page.locator("h1")).toHaveText("弱起与连音练习");
  const song = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  expect(song.notes).toHaveLength(6);
  expect(song.hasScore).toBe(true);
  expect(song.duration).toBeCloseTo(5, 4);
  expect(song.measures[0].partial).toBe(true);
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await expect(page.locator(".score-paper svg").first()).toBeVisible({
    timeout: 15000,
  });
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path:
        process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-直接乐谱练习.png",
      fullPage: true,
    });
});
test("全库后台索引可暂停、恢复和取消，关闭窗口后继续运行", async ({
  page,
  request,
}) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByRole("button", { name: "后台索引全库" }).click();
  await expect(
    page.getByRole("button", { name: "暂停后台索引" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "暂停后台索引" }).click();
  await expect(
    page.getByRole("button", { name: "继续后台索引" }),
  ).toBeVisible();
  const paused = await (
    await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data: { type: "indexStatus" },
    })
  ).json();
  expect(paused.status).toBe("paused");
  expect(paused.total).toBeGreaterThan(12000);
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path:
        process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-后台曲库索引.png",
      fullPage: true,
    });
  await page.getByRole("button", { name: "继续后台索引" }).click();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect
    .poll(
      async () => {
        const p = await (
          await request.post("http://127.0.0.1:32124/api/command", {
            headers: { "X-Neothesia-Client": "web" },
            data: { type: "indexStatus" },
          })
        ).json();
        return p.done;
      },
      { timeout: 15000 },
    )
    .toBeGreaterThan(paused.done);
  await request.post("http://127.0.0.1:32124/api/command", {
    headers: { "X-Neothesia-Client": "web" },
    data: { type: "cancelIndex" },
  });
});
