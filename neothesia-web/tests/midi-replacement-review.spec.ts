import { test, expect } from "@playwright/test";
import { copyFile, mkdir } from "node:fs/promises";
import { join } from "node:path";

test("原路径换 MIDI 后审阅差异，旧指法只随原内容迁移", async ({ page, request }) => {
  test.setTimeout(60000);
  const data = process.env.NEOTHESIA_REPAIR_TEST_DATA;
  test.skip(!data, "需要隔离的文件位置测试目录");
  const root = join(data!, "原路径更新审阅"), path = join(root, "课堂曲.mid"), copy = join(root, "课堂曲原版.mid");
  await mkdir(root, { recursive: true });
  await copyFile("../neothesia-engine/assets/five-finger.mid", path);
  await copyFile(path, copy);
  const cmd = async (value: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data: value,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  await cmd({ type: "refreshLibrary" });
  await expect.poll(async () => {
    const workspace = await cmd({ type: "libraryWorkspace" });
    return Object.values(workspace.entries).some((entry: any) => entry.path === path);
  }, { timeout: 20000 }).toBe(true);
  await cmd({ type: "inspectSong", path, force: true });
  const original = await cmd({ type: "load", path, title: "课堂曲" });
  await cmd({ type: "finger", track: original.notes[0].track, index: original.notes[0].index, finger: 4 });
  const oldId = original.contentId;
  await cmd({ type: "inspectSong", path, force: true });
  await cmd({ type: "inspectSong", path: copy, force: true });
  await copyFile("../neothesia-engine/assets/basic-chords.mid", path);
  const changed = await cmd({ type: "inspectSong", path, force: true });
  expect(changed.contentId).not.toBe(oldId);
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByRole("button", { name: "文件位置", exact: true }).click();
  await page.getByRole("tab", { name: /原路径内容变化/ }).click();
  await page.getByLabel("文件位置搜索").fill("课堂曲");
  const row = page.locator(".library-repair-group").filter({ hasText: path });
  await expect(row).toBeVisible();
  await row.getByRole("button", { name: "核对音符与处理方式" }).click();
  await expect(row.getByLabel("MIDI 内容变化预览")).toContainText("逐音核对");
  await expect(row.getByLabel("MIDI 内容变化预览")).toContainText("练习历史继续归属旧内容");
  await row.getByRole("button", { name: "保留旧曲目，隐藏新文件" }).click();
  await expect(row).toHaveCount(0);
  await page.getByLabel("显示已处理位置").check();
  await expect(row).toContainText("保留旧曲目");
  await row.getByRole("button", { name: "核对音符与处理方式" }).click();
  await row.getByRole("button", { name: "新文件建立独立曲目" }).click();
  await expect(row).toContainText("新文件作为新曲目显示");
  const newer = await cmd({ type: "load", path, title: "课堂曲新版" });
  expect(newer.contentId).toBe(changed.contentId);
  expect(newer.notes[0].finger).not.toBe(4);
  await row.getByRole("button", { name: "核对音符与处理方式" }).click();
  await row.getByRole("button", { name: "旧曲目重新关联到所选位置" }).click();
  await expect(row).toContainText("旧曲目已关联至");
  const reopened = await cmd({ type: "openRecent", content_id: oldId });
  expect(reopened.sourcePath).toBe(copy);
  expect(reopened.notes[0].finger).toBe(4);
});
