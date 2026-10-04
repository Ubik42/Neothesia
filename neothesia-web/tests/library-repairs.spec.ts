import { test, expect } from "@playwright/test";
import { unlink, copyFile, stat } from "node:fs/promises";
import { join } from "node:path";
test("缺失位置批量关联、错误内容拒绝、恢复指法历史与重复路径常用位置", async ({
  page,
  request,
}) => {
  test.setTimeout(60000);
  const data = process.env.NEOTHESIA_REPAIR_TEST_DATA;
  test.skip(!data, "需要准备隔离文件位置测试目录");
  const root = join(data!, "修复曲库"),
    old1 = join(root, "修复原始一.mid"),
    new1 = join(root, "修复新版一.mid"),
    copy1 = join(root, "修复副本一.mid"),
    old2 = join(root, "修复原始二.mid"),
    new2 = join(root, "修复新版二.mid"),
    wrong = join(root, "错误文件.mid");
  const command = async (value: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data: value,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const songs = async () =>
    await (await request.get("http://127.0.0.1:32124/api/library")).json();
  const song = async () =>
    await (await request.get("http://127.0.0.1:32124/api/song")).json();
  const ids = [];
  for (const p of [old1, new1, old2, new2, wrong])
    ids.push(
      (await command({ type: "inspectSong", path: p, force: true })).contentId,
    );
  const first = await command({
    type: "load",
    path: old1,
    title: "修复练习一",
  });
  await command({
    type: "finger",
    track: first.notes[0].track,
    index: first.notes[0].index,
    finger: 4,
  });
  await command({ type: "favorite", enabled: true });
  await command({
    type: "assignLibrary",
    paths: [old1],
    group: null,
    rating: 4,
  });
  await command({
    type: "updateSongMetadata",
    path: old1,
    value: {
      title: "修复练习一",
      composer: "课堂练习",
      artist: null,
      collection: null,
      difficulty: "入门",
      tags: ["作业"],
      notes: "手腕保持放松",
    },
  });
  await command({ type: "countIn", bars: 0 });
  await command({ type: "play" });
  await expect
    .poll(async () => {
      const s = await (
        await request.get("http://127.0.0.1:32124/api/state")
      ).json();
      return s.required.includes(60);
    })
    .toBe(true);
  await command({ type: "note", pitch: 60, velocity: 90, active: true });
  await command({ type: "note", pitch: 60, velocity: 0, active: false });
  await command({ type: "save" });
  await command({ type: "pause" });
  const history = await command({
    type: "historyQuery",
    query: "修复练习一",
    offset: 0,
    limit: 50,
  });
  const historyRow = history.rows?.[0] ?? history.entries?.[0];
  await unlink(old1);
  await unlink(old2);
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByRole("button", { name: "文件位置", exact: true }).click();
  await page.getByLabel("文件位置搜索").fill("修复");
  await expect(page.locator(".library-repair-group")).toHaveCount(2);
  await page
    .getByRole("button", { name: "选择本页可匹配项", exact: true })
    .click();
  await expect(page.locator(".library-repair-footer")).toContainText(
    "已选择 2",
  );
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path:
        process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-文件位置修复.png",
      fullPage: true,
    });
  await copyFile(wrong, new2);
  await page.getByRole("button", { name: "关联所选位置", exact: true }).click();
  await expect(page.locator(".library-repair-results")).toContainText(
    "1 个成功",
  );
  await expect(page.locator(".library-repair-results")).toContainText(
    "1 个未处理",
  );
  expect((await songs()).songs.some((r: any) => r.path === old1)).toBe(false);
  expect((await song()).sourcePath).toBe(old1);
  const reopened = await command({ type: "openRecent", content_id: ids[0] });
  expect(reopened.sourcePath).toBe(new1);
  expect(reopened.notes[0].finger).toBe(4);
  const historyList = await command({
    type: "historyQuery",
    query: "修复练习一",
    offset: 0,
    limit: 50,
  });
  const record = (historyList.rows ??
    historyList.entries ??
    historyList.sessions)[0];
  expect(record).toBeTruthy();
  await command({
    type: "historyOpen",
    content_id: ids[0],
    id: record.id,
    restore: true,
  });
  expect((await song()).sourcePath).toBe(new1);
  await copyFile("../neothesia-engine/assets/basic-chords.mid", new2);
  await command({ type: "inspectSong", path: new2, force: true });
  const secondGroup = page.locator(
    `.library-repair-group[data-content-id="${ids[2]}"]`,
  );
  await expect(secondGroup).toBeVisible();
  const originalRow = secondGroup.locator(".library-repair-path").filter({
    has: page.getByRole("checkbox", {
      name: `处理位置 ${old2}`,
      exact: true,
    }),
  });
  await originalRow
    .getByRole("button", { name: "重新关联", exact: true })
    .click();
  await expect(page.locator(".library-repair-results")).toContainText(
    "1 个成功",
  );
  await command({ type: "inspectSong", path: copy1, force: true });
  await page.getByRole("tab", { name: /重复路径/ }).click();
  const duplicateGroup = page.locator(
    `.library-repair-group[data-content-id="${ids[0]}"]`,
  );
  await expect(duplicateGroup).toBeVisible();
  const copiedRow = duplicateGroup.locator(".library-repair-path").filter({
    has: page.getByRole("checkbox", {
      name: `处理位置 ${copy1}`,
      exact: true,
    }),
  });
  await copiedRow
    .getByRole("button", { name: "设为常用位置", exact: true })
    .click();
  expect((await song()).sourcePath).toBe(new1);
  await expect(copiedRow).toContainText("常用位置");
  const former = duplicateGroup.locator(".library-repair-path").filter({
    has: page.getByRole("checkbox", {
      name: `处理位置 ${new1}`,
      exact: true,
    }),
  });
  await former.getByRole("button", { name: "合并显示", exact: true }).click();
  await expect(page.locator(".library-repair-results")).toContainText(
    "1 个成功",
  );
  expect((await stat(new1)).isFile()).toBe(true);
  expect((await stat(copy1)).isFile()).toBe(true);
  expect((await songs()).songs.some((r: any) => r.path === new1)).toBe(false);
  expect((await songs()).songs.some((r: any) => r.path === copy1)).toBe(true);
  await former
    .getByRole("button", { name: "恢复路径显示", exact: true })
    .click();
  expect((await songs()).songs.some((r: any) => r.path === new1)).toBe(true);
  await command({ type: "openRecent", content_id: ids[0] });
  expect((await song()).sourcePath).toBe(copy1);
  expect((await song()).notes[0].finger).toBe(4);
  await duplicateGroup
    .getByRole("button", { name: "使用默认位置", exact: true })
    .click();
  await expect(
    duplicateGroup.getByRole("button", { name: "使用默认位置", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "返回曲库管理", exact: true }),
  ).toBeEnabled();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toContainText("曲库管理");
  await expect(page.locator(".library-repairs")).toHaveCount(0);
});
