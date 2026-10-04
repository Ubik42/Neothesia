import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
test("曲目包导出、预览与冲突处理保留谱面、指法和练习段落", async ({
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
  await command({
    type: "importScore",
    bytes: Array.from(
      readFileSync("../neothesia-core/src/score_performance_test.musicxml"),
    ),
    name: "连音教学.musicxml",
    default_bpm: 120,
  });
  await command({ type: "mode", value: "wait" });
  await command({ type: "noteHand", track: 1, index: 0, part: "LeftHand" });
  await command({
    type: "savePassage",
    id: null,
    name: "反复连音",
    start: 1,
    end: 2,
    notes: "保持第一指法",
  });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "曲目包", exact: true }).click();
  const downloading = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出当前曲目包" }).click();
  const download = await downloading;
  expect(download.suggestedFilename()).toMatch(/\.neopiece$/);
  const data = readFileSync((await download.path())!);
  await command({ type: "noteHand", track: 1, index: 0, part: "RightHand" });
  await command({ type: "finger", track: 1, index: 0, finger: 5 });
  const input = page.getByLabel("导入曲目包文件");
  await input.setInputFiles({
    name: "教学.neopiece",
    mimeType: "application/zip",
    buffer: data,
  });
  await expect(page.getByRole("dialog")).toContainText("本地已有同一内容");
  await expect(page.getByRole("dialog")).toContainText("反复连音");
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-曲目包.png",
      fullPage: true,
    });
  await page.getByLabel("曲目包导入策略").selectOption("merge");
  await page.getByRole("button", { name: "确认导入曲目包" }).click();
  await expect(page.getByRole("dialog")).toContainText("曲目包已导入");
  let song = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  let note = song.notes.find(
    (n: { track: number; index: number }) => n.track === 1 && n.index === 0,
  );
  expect(note.part).toBe("right");
  expect(note.finger).toBe(5);
  expect(song.hasScore).toBe(true);
  await input.setInputFiles({
    name: "教学.neopiece",
    mimeType: "application/zip",
    buffer: data,
  });
  await expect(
    page.getByRole("button", { name: "确认导入曲目包" }),
  ).toBeEnabled();
  await page.getByLabel("曲目包导入策略").selectOption("replace");
  await page.getByRole("button", { name: "确认导入曲目包" }).click();
  await expect(page.getByRole("dialog")).toContainText("曲目包已导入");
  song = await (await request.get("http://127.0.0.1:32124/api/song")).json();
  note = song.notes.find(
    (n: { track: number; index: number }) => n.track === 1 && n.index === 0,
  );
  expect(note.part).toBe("left");
  expect(note.finger).toBe(3);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "练习段落", exact: true }).click();
  await expect(page.getByRole("dialog")).toContainText("反复连音");
});
