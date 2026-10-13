import { test, expect } from "@playwright/test";
import { readFile } from "node:fs/promises";

test("完整资料导出、批次预览、部分失败重试与仅批次恢复", async ({
  page,
  request,
}) => {
  test.setTimeout(90000);
  const command = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  await command({ type: "stopRoutine" });
  await command({ type: "pause" });
  const plan = await command({
    type: "saveRoutine",
    id: null,
    day: null,
    name: "完整备份课堂",
    notes: "恢复测试",
    copy_of: null,
  });
  await page.goto("/");
  await page.evaluate(async () => {
    const store = await import("/src/batchImportStore.ts");
    for (let index = 1; index <= 2; index++)
      await store.saveBatch({
        id: `workspace-dialog-source-${index}`,
        name: `课堂整理 ${index}`,
        revision: 0,
        updated: Date.now(),
        prepared: false,
        books: [],
        midiTargets: [],
        items: [
          {
            id: `source-file-${index}`,
            file: new File(
              [
                `<score-partwise version="4.0"><work><work-title>课堂${index}</work-title></work></score-partwise>`,
              ],
              `课堂${index}.musicxml`,
              { type: "application/xml", lastModified: Date.now() },
            ),
            kind: "notation",
            selected: true,
            manual: false,
            group: "",
            state: "pending",
            message: "",
          },
        ],
      });
  });
  const count = () =>
    page.evaluate(
      async () =>
        (await (await import("/src/batchImportStore.ts")).listBatches()).length,
    );
  await page.getByRole("button", { name: "练习计划", exact: true }).click();
  await page.getByRole("button", { name: "练习资料备份", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "练习资料备份" });
  await expect(
    dialog.getByRole("region", { name: "完整资料备份" }),
  ).toContainText("本机已保存 2 个整理批次");
  const download = page.waitForEvent("download");
  await dialog
    .getByRole("button", { name: "导出完整资料备份", exact: true })
    .click();
  const output = await download;
  expect(output.suggestedFilename()).toMatch(/\.neoworkspace$/);
  const file = "../work/cycle206-dialog.neoworkspace";
  await output.saveAs(file);
  await expect(
    dialog.getByText("完整资料备份已导出", { exact: false }),
  ).toBeVisible();
  const pick = async () => {
    await dialog.getByLabel("完整资料备份文件").setInputFiles(file);
    await expect(
      dialog.getByRole("region", { name: "整理批次恢复预览" }),
    ).toBeVisible();
  };
  await pick();
  expect(await count()).toBe(2);
  await expect(dialog.getByText("完整备份课堂", { exact: true })).toBeVisible();
  await dialog.getByLabel("恢复整理批次 课堂整理 2").uncheck();
  await page.screenshot({ path: "../outputs/Neothesia-完整资料备份.png" });
  let rejectImport = true;
  await page.route("**/api/command", async (route) => {
    const body = route.request().postDataJSON();
    if (body?.type === "importStagedPracticeBackup" && rejectImport) {
      rejectImport = false;
      await route.fulfill({
        status: 409,
        contentType: "application/json",
        body: JSON.stringify({ error: "练习资料暂时不可写" }),
      });
    } else await route.continue();
  });
  await dialog
    .getByRole("button", { name: "导入所选完整资料", exact: true })
    .click();
  await expect(dialog.getByRole("alert")).toContainText("整理批次已保存 1 个");
  expect(await count()).toBe(3);
  await dialog
    .getByRole("button", { name: "重试练习资料导入", exact: true })
    .click();
  await expect(dialog.getByRole("status")).toContainText("导入完成");
  expect(await count()).toBe(3);
  await pick();
  await dialog
    .getByRole("button", { name: "导入所选完整资料", exact: true })
    .click();
  await expect(dialog.getByRole("alert")).toContainText("已恢复到本机");
  expect(await count()).toBe(3);
  await dialog.getByLabel("恢复整理批次 课堂整理 1").uncheck();
  for (const name of [
    "计划、日课与曲目学习",
    "个人成绩与演奏回放",
    "片段、阶梯、技术练习、音轨与资料模板",
  ])
    await dialog.getByLabel(name, { exact: true }).uncheck();
  await expect(
    dialog.getByRole("button", { name: "导出完整资料备份", exact: true }),
  ).toBeDisabled();
  await dialog
    .getByRole("button", { name: "导入所选完整资料", exact: true })
    .click();
  await expect(dialog.getByRole("status")).toContainText(
    "已保存 1 个整理批次及其原始文件",
  );
  expect(await count()).toBe(4);
  const restored = await page.evaluate(async () => {
    const store = await import("/src/batchImportStore.ts");
    const summaries = await store.listBatches();
    return Promise.all(
      summaries
        .filter(
          (b) =>
            b.id.startsWith("workspace-") &&
            !b.id.startsWith("workspace-dialog-source"),
        )
        .map(async (b) => {
          const batch = await store.loadBatch(b.id);
          return { name: batch.name, text: await batch.items[0].file.text() };
        }),
    );
  });
  expect(restored).toHaveLength(2);
  expect(restored.every((b) => b.text.includes("score-partwise"))).toBeTruthy();
  const broken = await readFile(file);
  broken[broken.length - 1] ^= 1;
  await dialog
    .getByLabel("完整资料备份文件")
    .setInputFiles({
      name: "损坏.neoworkspace",
      mimeType: "application/octet-stream",
      buffer: broken,
    });
  await expect(dialog.getByRole("alert")).toBeVisible();
  expect(await count()).toBe(4);
  await expect(
    dialog.getByRole("region", { name: "整理批次恢复预览" }),
  ).toHaveCount(0);
  expect(plan.id).toBeTruthy();
});
