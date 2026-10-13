import { expect, test } from "@playwright/test";

test("曲目资料修改历史进入备份，预览并跳过重复恢复", async ({ page, request }) => {
  test.setTimeout(60000);
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const xml = '<score-partwise><work><work-title>资料备份课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions></attributes><note><pitch><step>G</step><octave>4</octave></pitch><duration>1</duration></note></measure></part></score-partwise>';
  const song = await command({ type: "importScore", name: "资料备份课堂.musicxml", bytes: [...Buffer.from(xml)], default_bpm: 100 });
  await command({ type: "inspectSong", path: song.sourcePath, force: true });
  await page.goto("/");
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByLabel("管理曲库搜索").fill("资料备份课堂");
  await page.locator(".library-manager-table tbody tr").first().click();
  await page.getByRole("button", { name: "编辑信息", exact: true }).click();
  const editor = page.getByRole("region", { name: "曲目信息编辑" });
  await editor.getByLabel("管理作曲家", { exact: true }).fill("课堂教师");
  await editor.getByRole("button", { name: "保存曲目信息", exact: true }).click();
  await expect(editor).toHaveCount(0);

  await page.goto("/");
  await page.getByRole("button", { name: "练习计划", exact: true }).click();
  await page.getByRole("button", { name: "练习资料备份", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "练习资料备份", exact: true });
  const download = page.waitForEvent("download");
  await dialog.getByRole("button", { name: "导出练习备份", exact: true }).click();
  const file = "../work/cycle211-metadata-history.neopractice";
  await (await download).saveAs(file);
  await dialog.getByLabel("练习备份文件").setInputFiles(file);
  await expect(dialog.locator(".practice-backup-counts")).toContainText("曲目资料修改历史 / 曲目数");
  await expect(dialog.locator(".practice-backup-counts")).toContainText("资料修改历史预计新增 / 重复 / 未定位 / 多路径待核对");
  await expect(dialog.locator(".practice-backup-counts div").filter({ hasText: "曲目资料修改历史 / 曲目数" })).toContainText("1 / 1");
  await expect(dialog.locator(".practice-backup-counts div").filter({ hasText: "资料修改历史预计新增 / 重复 / 未定位 / 多路径待核对" })).toContainText("0 / 1 / 0 / 0");
  await dialog.getByRole("button", { name: "按所选范围导入", exact: true }).click();
  await expect(dialog.getByRole("status")).toContainText("曲目资料历史新增 0 条、跳过重复 1 条");
  await page.screenshot({ path: "../outputs/Neothesia-资料历史备份.png" });
});
