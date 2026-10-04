import { test, expect } from "@playwright/test";
import { mkdir, writeFile, readFile } from "node:fs/promises";
import path from "node:path";

test("跨曲目原谱批量预览：独立选择、陈旧失败继续、新演奏与旧资料保留", async ({
  page,
  request,
}) => {
  test.setTimeout(60000);
  const cmd = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  await cmd({ type: "stopRoutine" });
  await cmd({ type: "stopLadder" });
  await cmd({ type: "pause" });
  const folder = path.resolve(`../work/cycle142-source-batch-${Date.now()}`);
  await mkdir(folder, { recursive: true });
  const xml = (title: string, step: string, extra = "") =>
    `<score-partwise version="4.0"><work><work-title>${title}</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>${step}</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note>${extra}</measure></part></score-partwise>`;
  const pieces: any[] = [];
  for (const [index, step] of ["C", "E", "G", "B"].entries()) {
    const title = `批量原谱课堂${index + 1}`,
      file = path.join(folder, `${title}.musicxml`),
      original = xml(title, step);
    await writeFile(file, original);
    const song = await cmd({
      type: "importScoreOriginal",
      path: file,
      default_bpm: 60,
    });
    const versions = await cmd({ type: "scoreVersions" });
    const saved = versions.versions.find((v: any) => v.active);
    const source = (
      await cmd({ type: "scoreSources", content_id: song.contentId })
    ).entries.find((s: any) => s.versionId === saved.id);
    pieces.push({ title, step, file, original, song, saved, source });
    await writeFile(
      file,
      xml(title, index === 3 ? "A" : step, "<!-- changed layout -->"),
    );
  }
  const current = await cmd({ type: "currentSong" });
  await page.goto("/");
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByRole("button", { name: /^原谱文件/ }).click();
  const dialog = page.getByRole("dialog", { name: "原谱文件", exact: true });
  await dialog
    .getByRole("textbox", { name: "查找原谱文件" })
    .fill("批量原谱课堂");
  await dialog.getByRole("button", { name: "立即检查", exact: true }).click();
  for (const piece of pieces)
    await dialog
      .locator(`[data-source-id="${piece.source.id}"]`)
      .getByRole("checkbox")
      .check();
  await dialog
    .getByRole("button", { name: "批量预览 · 4", exact: true })
    .click();
  const batch = dialog.getByRole("region", {
    name: "批量原谱更新",
    exact: true,
  });
  await batch
    .getByRole("button", { name: "生成逐项预览", exact: true })
    .click();
  await expect(batch).toContainText("预览完成");
  for (const piece of pieces)
    await expect(
      batch
        .locator(`[data-batch-source-id="${piece.source.id}"]`)
        .getByRole("combobox"),
    ).toHaveValue("skip");
  await batch
    .getByRole("button", { name: "已预览项添加新谱面", exact: true })
    .click();
  await batch
    .locator(`[data-batch-source-id="${pieces[2].source.id}"]`)
    .getByRole("combobox")
    .selectOption("keep");
  await batch
    .locator(`[data-batch-source-id="${pieces[3].source.id}"]`)
    .getByRole("combobox")
    .selectOption("performance");
  await writeFile(pieces[0].file, xml(pieces[0].title, "D"));
  await page.screenshot({ path: "../outputs/Neothesia-批量原谱更新.png" });
  await batch
    .getByRole("button", { name: "执行所选处理 · 4", exact: true })
    .click();
  await expect(batch).toContainText("处理完成 · 3 个已完成");
  await expect(
    batch.locator(`[data-batch-source-id="${pieces[0].source.id}"]`),
  ).toContainText("重新检查");
  await expect(
    batch.locator(`[data-batch-source-id="${pieces[1].source.id}"]`),
  ).toContainText("已保存新谱面版本");
  await expect(
    batch.locator(`[data-batch-source-id="${pieces[2].source.id}"]`),
  ).toContainText("已保留当前版本");
  expect((await cmd({ type: "currentSong" })).contentId).not.toBe(
    current.contentId,
  );
  await expect(
    batch.locator(`[data-batch-source-id="${pieces[3].source.id}"]`),
  ).toContainText("已生成并打开演奏曲目");
  for (const piece of pieces)
    expect(await readFile(piece.saved.path, "utf8")).toBe(piece.original);
  const all = (await cmd({ type: "scoreSources" })).entries;
  expect(all.find((s: any) => s.id === pieces[0].source.id).pending).toBe(true);
  expect(
    all.filter(
      (s: any) =>
        s.contentId === pieces[1].song.contentId &&
        s.path === pieces[1].source.path,
    ),
  ).toHaveLength(2);
  expect(all.find((s: any) => s.id === pieces[2].source.id).status).toBe(
    "acknowledged",
  );
});
