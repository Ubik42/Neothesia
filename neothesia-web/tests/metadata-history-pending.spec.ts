import { expect, test } from "@playwright/test";
import { readFile, unlink, writeFile } from "node:fs/promises";

test("未匹配的资料修改历史在导入后保留，曲目回库后由用户认领", async ({ page, request }) => {
  test.setTimeout(90000);
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", { headers: { "X-Neothesia-Client": "web" }, data });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const xml = '<score-partwise><work><work-title>待认领课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions></attributes><note><pitch><step>A</step><octave>4</octave></pitch><duration>1</duration></note></measure></part></score-partwise>';
  const song = await command({ type: "importScore", name: "待认领课堂.musicxml", bytes: [...Buffer.from(xml)], default_bpm: 100 });
  await command({ type: "inspectSong", path: song.sourcePath, force: true });
  await page.goto("/");
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByLabel("管理曲库搜索").fill("待认领课堂");
  await page.locator(".library-manager-table tbody tr").first().click();
  await page.getByRole("button", { name: "编辑信息", exact: true }).click();
  const editor = page.getByRole("region", { name: "曲目信息编辑" });
  await editor.getByLabel("管理备注", { exact: true }).fill("课堂记号");
  await editor.getByRole("button", { name: "保存曲目信息", exact: true }).click();
  await expect(editor).toHaveCount(0);
  const backup = await command({ type: "practiceBackupSnapshot", selection: { plans: true, records: true, presets: true, days: 0 } });
  expect(backup.metadataHistory.length).toBeGreaterThan(0);
  const original = await readFile(song.sourcePath);
  await unlink(song.sourcePath);
  try {
    await command({ type: "inspectSong", path: song.sourcePath, force: true });
    const imported = await command({ type: "applyPracticeBackup", backup, selection: { plans: false, records: false, presets: true, days: 0 }, policy: "keep" });
    expect(imported.metadataHistoryPendingAdded).toBeGreaterThan(0);
    await writeFile(song.sourcePath, original);
    await command({ type: "inspectSong", path: song.sourcePath, force: true });
    await page.goto("/");
    await page.getByRole("button", { name: "练习计划", exact: true }).click();
    await page.getByRole("button", { name: "练习资料备份", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "练习资料备份", exact: true });
    const section = dialog.getByRole("region", { name: "待认领资料修改历史" });
    await expect(section).toContainText("1 条记录");
    await expect(section).toContainText("待认领课堂");
    await expect(section).toContainText(song.sourcePath);
    await expect(section).toContainText(song.contentId);
    await page.screenshot({ path: "../outputs/Neothesia-资料历史待认领.png" });
    await section.getByRole("button", { name: "认领到所选曲目" }).click();
    await expect(section).toContainText("目前没有待认领记录");
    await page.reload();
    await page.getByRole("button", { name: "练习计划", exact: true }).click();
    await page.getByRole("button", { name: "练习资料备份", exact: true }).click();
    await expect(page.getByRole("region", { name: "待认领资料修改历史" })).toContainText("目前没有待认领记录");
  } finally { await writeFile(song.sourcePath, original); }
});
