import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
test("最近文件位置管理与换内容后的重新导入", async ({ page, request }) => {
  const root = path.resolve("../work/cycle149-browser-final");
  const a = path.join(root, "fixtures", "钢琴谱"),
    b = path.join(root, "fixtures", "课堂"),
    missing = path.join(root, "fixtures", "断开的目录");
  fs.mkdirSync(a, { recursive: true });
  fs.mkdirSync(b, { recursive: true });
  fs.writeFileSync(path.join(a, "保留文件.txt"), "保留");
  fs.writeFileSync(
    path.join(root, "file-dialog-locations.json"),
    JSON.stringify({
      version: 1,
      kinds: { music: [missing, b, a], notation: [a] },
    }),
  );
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const xml =
    '<score-partwise version="4.0"><work><work-title>重新导入课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>D</step><octave>5</octave></pitch><duration>4</duration><type>whole</type></note></measure></part></score-partwise>';
  const command = {
    type: "importScore",
    name: "重新导入课堂.musicxml",
    bytes: Array.from(Buffer.from(xml)),
    default_bpm: 102,
  };
  const original = await cmd(command);
  expect(original.sourcePath.replaceAll("\\", "/")).toContain(
    "/work/cycle149-browser-final/imports/",
  );
  const foreign = fs.readFileSync(
    "../neothesia-engine/assets/basic-chords.mid",
  );
  fs.writeFileSync(original.sourcePath, foreign);
  const restored = await cmd(command);
  expect(restored.contentId).toBe(original.contentId);
  expect(restored.sourcePath).not.toBe(original.sourcePath);
  expect(restored.importRecovery.preservedPath).toBe(original.sourcePath);
  expect(fs.readFileSync(original.sourcePath)).toEqual(foreign);
  expect((await cmd(command)).sourcePath).toBe(restored.sourcePath);
  await page.goto("/");
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByRole("button", { name: "最近文件位置", exact: true }).click();
  const dialog = page.getByRole("dialog", {
    name: "最近文件位置",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  const unavailable = dialog
    .locator(".file-location-row")
    .filter({ hasText: missing });
  await expect(unavailable).toContainText("当前不可用");
  await expect(
    unavailable.getByRole("button", { name: "用于下次选择" }),
  ).toBeDisabled();
  const target = dialog.locator(".file-location-row").filter({ hasText: a });
  await target.getByRole("button", { name: "用于下次选择" }).click();
  await expect(dialog.locator(".file-location-row").first()).toContainText(a);
  await dialog.getByLabel(`移除位置记录 ${missing}`).click();
  await expect(dialog.locator(".file-location-row")).toHaveCount(2);
  expect(fs.readFileSync(path.join(a, "保留文件.txt"), "utf8")).toBe("保留");
  await page.screenshot({
    path: "../outputs/Neothesia-最近文件位置.png",
    fullPage: true,
  });
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(page.getByLabel("管理曲库搜索")).toBeVisible();
  await page.getByRole("button", { name: "最近文件位置", exact: true }).click();
  await expect(dialog.locator(".file-location-row").first()).toContainText(a);
  await dialog.getByRole("button", { name: "清除此类记录" }).click();
  await expect(dialog).toContainText("此类文件还没有位置记录");
  await dialog.getByLabel("文件位置用途").selectOption("notation");
  await expect(dialog.locator(".file-location-row")).toHaveCount(1);
  await expect(dialog.locator(".file-location-row")).toContainText(a);
  const saved = JSON.parse(
    fs.readFileSync(path.join(root, "file-dialog-locations.json"), "utf8"),
  );
  expect(saved.kinds.music).toEqual([]);
  expect(saved.kinds.notation).toEqual([a]);
  expect(fs.existsSync(a)).toBeTruthy();
});
