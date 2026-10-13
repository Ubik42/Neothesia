import { test, expect } from "@playwright/test";
import { mkdir, writeFile, readFile, rename } from "node:fs/promises";
import path from "node:path";
const xml = (last: string, extra = "") =>
  `<score-partwise version="4.0"><work><work-title>原谱更新课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes>${["C", "D", "E", last].map((step) => `<note><pitch><step>${step}</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note>`).join("")}${extra}</measure></part></score-partwise>`;
test("原谱更新：自动发现、预览防变、新版本和独立演奏身份", async ({
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
  const folder = path.resolve("../work/cycle141-source-fixture");
  await mkdir(folder, { recursive: true });
  const original = path.join(folder, `原谱-${Date.now()}.musicxml`),
    first = xml("F");
  await writeFile(original, first);
  await cmd({ type: "stopRoutine" });
  await cmd({ type: "pause" });
  const song = await cmd({
    type: "importScoreOriginal",
    path: original,
    default_bpm: 60,
  });
  const versions = await cmd({ type: "scoreVersions" });
  const copy = versions.versions.find((v: any) => v.active).path;
  expect(copy).not.toBe(original);
  await cmd({
    type: "editFingersFor",
    content_id: song.contentId,
    edits: [{ track_id: song.notes[0].track, note_index: 0, finger: 2 }],
  });
  const sources = await cmd({
    type: "scoreSources",
    content_id: song.contentId,
  });
  const oldId = sources.entries.find(
    (s: any) => s.versionId === versions.versions.find((v: any) => v.active).id,
  ).id;
  await page.goto("/");
  await page.getByRole("button", { name: "谱面管理", exact: true }).click();
  await page.getByRole("button", { name: "检查原谱更新", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "原谱文件", exact: true });
  await expect(dialog).toContainText("原文件与保存版本一致");
  await writeFile(original, xml("F", "<!-- layout version 2 -->"));
  const oldRow = dialog.locator(`[data-source-id="${oldId}"]`);
  await expect(oldRow).toContainText("原文件有更新", { timeout: 12000 });
  await expect
    .poll(
      async () => (await cmd({ type: "libraryMonitorStatus" })).scoreUpdates,
      { timeout: 12000 },
    )
    .toBeGreaterThan(0);
  await oldRow.getByRole("button", { name: "预览更新", exact: true }).click();
  await expect(
    dialog.getByRole("region", { name: "原谱更新预览" }),
  ).toContainText("演奏内容身份相同");
  await expect(
    dialog.getByRole("region", { name: "更新后谱面的实际演奏顺序" }),
  ).toContainText("第 1 小节");
  const changed = await cmd({
    type: "scoreSources",
    content_id: song.contentId,
  });
  const token = changed.entries.find((s: any) => s.id === oldId).fingerprint;
  await writeFile(original, xml("G"));
  const stale = await request.post("http://127.0.0.1:32124/api/command", {
    headers: { "X-Neothesia-Client": "web" },
    data: {
      type: "applyScoreSource",
      id: oldId,
      fingerprint: token,
      mode: "notation",
    },
  });
  expect(stale.ok()).toBeFalsy();
  expect(await stale.text()).toContain("重新检查");
  await expect(
    dialog.getByText("原文件或关联已变化，请重新预览后应用。"),
  ).toBeVisible({ timeout: 12000 });
  await expect(
    dialog.getByRole("button", { name: "添加为新谱面版本" }),
  ).toHaveCount(0);
  await writeFile(original, xml("F", "<!-- layout version 2 -->"));
  await expect(
    dialog.getByRole("button", { name: "添加为新谱面版本" }),
  ).toBeVisible({ timeout: 12000 });
  await dialog
    .getByRole("button", { name: "添加为新谱面版本", exact: true })
    .click();
  await expect(dialog).toContainText("已添加新的谱面版本");
  expect(await readFile(copy, "utf8")).toBe(first);
  expect((await cmd({ type: "currentSong" })).contentId).toBe(song.contentId);
  expect((await cmd({ type: "currentSong" })).notes[0].finger).toBe(2);
  const updated = await cmd({
    type: "scoreSources",
    content_id: song.contentId,
  });
  const currentSource = updated.entries.find(
    (s: any) =>
      s.status === "ready" && s.path.endsWith(path.basename(original)),
  );
  await writeFile(original, xml("G"));
  const currentRow = dialog.locator(`[data-source-id="${currentSource.id}"]`);
  await expect(currentRow).toContainText("原文件有更新", { timeout: 12000 });
  await currentRow
    .getByRole("button", { name: "预览更新", exact: true })
    .click();
  await expect(
    dialog.getByRole("region", { name: "原谱更新预览" }),
  ).toContainText("演奏内容身份不同");
  await page.screenshot({
    path: "../outputs/Neothesia-原谱更新预览.png",
    fullPage: true,
  });
  await dialog
    .getByRole("button", { name: "建立并打开新演奏曲目", exact: true })
    .click();
  await expect(dialog).toContainText("已生成并打开新谱演奏");
  const next = await cmd({ type: "currentSong" });
  expect(next.contentId).not.toBe(song.contentId);
  expect(next.notes[0].finger).toBeNull();
  expect(await readFile(copy, "utf8")).toBe(first);
  await rename(original, `${original}.gone`);
  await expect(dialog).toContainText("原文件缺失", { timeout: 12000 });
  expect((await cmd({ type: "currentSong" })).hasScore).toBeTruthy();
  await currentRow
    .getByRole("button", { name: "取消原文件关联", exact: true })
    .click();
  await expect(dialog).toContainText("已取消原文件关联");
  expect((await cmd({ type: "currentSong" })).hasScore).toBeTruthy();
});

test("原谱编辑后保存并使用新谱面，重开仍使用该版本", async ({ page, request }) => {
  test.setTimeout(45000);
  const cmd = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const folder = path.resolve("../work/cycle222-source-fixture");
  await mkdir(folder, { recursive: true });
  const original = path.join(folder, `原谱直接启用-${Date.now()}.musicxml`);
  const first = xml("F");
  await writeFile(original, first);
  await cmd({ type: "stopRoutine" });
  await cmd({ type: "pause" });
  const song = await cmd({ type: "importScoreOriginal", path: original, default_bpm: 60 });
  const before = await cmd({ type: "scoreVersions" });
  const firstVersion = before.versions.find((v: any) => v.active);
  await page.goto("/");
  await page.getByRole("button", { name: "谱面管理", exact: true }).click();
  await page.getByRole("button", { name: "检查原谱更新", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "原谱文件", exact: true });
  await writeFile(original, xml("F", "<!-- MuseScore saved layout -->"));
  const row = dialog.locator(`[data-source-id]`).filter({ hasText: path.basename(original) }).first();
  await expect(row).toContainText("原文件有更新", { timeout: 12000 });
  await row.getByRole("button", { name: "预览更新", exact: true }).click();
  await expect(dialog.getByRole("button", { name: "保存并使用新谱面" })).toBeEnabled();
  await dialog.getByRole("button", { name: "保存并使用新谱面" }).click();
  await expect(dialog).toContainText("已保存并启用新谱面");
  const updated = await cmd({ type: "scoreVersions" });
  const active = updated.versions.find((v: any) => v.active);
  expect(active.id).not.toBe(firstVersion.id);
  expect((await cmd({ type: "currentSong" })).contentId).toBe(song.contentId);
  await cmd({ type: "load", path: song.sourcePath, title: song.title });
  const reopened = await cmd({ type: "scoreVersions" });
  expect(reopened.versions.find((v: any) => v.active).id).toBe(active.id);
  expect((await cmd({ type: "currentSong" })).hasScore).toBeTruthy();
  expect(await readFile(firstVersion.path, "utf8")).toBe(first);
});
