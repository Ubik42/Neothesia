import { test, expect } from "@playwright/test";
import fs from "node:fs/promises";
test("仅资料模板的备份导出、字段预览、保留与覆盖及范围隔离", async ({
  page,
  request,
}) => {
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const spec={tonic:0,tonality:"Major",minor_form:"Natural",pattern:"Scale",direction:"UpAndDown",hands:"Right",octaves:1,repetitions:1,tempo_bpm:75};
  const first=await cmd({type:"generate",spec});
  const second=await cmd({type:"generate",spec:{...spec,hands:"Left",repetitions:3,tempo_bpm:150}});
  expect(second.contentId).toBe(first.contentId);
  expect(second.tracks.filter((t:{notes:number})=>t.notes>0).every((t:{part:string})=>t.part==="left")).toBeTruthy();
  expect(second.notes.every((n:{part:string})=>n.part==="left")).toBeTruthy();
  expect(second.notes.length).toBeGreaterThan(first.notes.length);
  expect(second.tracks.filter((t:{notes:number})=>t.notes>0).every((t:{mode:string})=>t.mode==="human")).toBeTruthy();
  await cmd({type:"countIn",bars:0});await cmd({type:"play"});
  await expect.poll(async()=>{const value=await (await request.get("http://127.0.0.1:32124/api/state")).json();return value.required.length;}).toBeGreaterThan(0);
  await cmd({type:"pause"});
  let registry = await cmd({ type: "metadataTemplates" });
  await cmd({
    type: "updateMetadataTemplate",
    name: "可迁移课堂",
    rules: {
      notes: { mode: "append", value: "备份中的教学提示" },
      tags: { mode: "append", value: "课堂,连奏" },
    },
    expected: registry.revision,
  });
  await page.goto("/");
  await page.getByRole("button", { name: "练习计划", exact: true }).click();
  await page.getByRole("button", { name: "练习资料备份", exact: true }).click();
  const dialog = page.getByRole("dialog", {
    name: "练习资料备份",
    exact: true,
  });
  await dialog.getByLabel("计划、日课与曲目学习").uncheck();
  await dialog.getByLabel("个人成绩记录").uncheck();
  const downloadPromise = page.waitForEvent("download");
  await dialog
    .getByRole("button", { name: "导出练习备份", exact: true })
    .click();
  const download = await downloadPromise;
  const file = "../work/cycle152-template-export.neopractice";
  await download.saveAs(file);
  expect((await fs.readFile(file)).length).toBeGreaterThan(200);
  await expect(
    dialog.getByText("练习资料备份已导出", { exact: true }),
  ).toBeVisible();
  registry = await cmd({ type: "metadataTemplates" });
  await cmd({
    type: "updateMetadataTemplate",
    name: "可迁移课堂",
    rules: { notes: { mode: "append", value: "本机最新提示" } },
    expected: registry.revision,
  });
  registry = await cmd({ type: "metadataTemplates" });
  await cmd({
    type: "updateMetadataTemplate",
    name: "本机独有模板",
    rules: { difficulty: { mode: "set", value: "基础" } },
    expected: registry.revision,
  });
  await dialog.getByLabel("练习备份文件").setInputFiles(file);
  await expect(
    dialog.getByRole("button", { name: "按所选范围导入", exact: true }),
  ).toBeEnabled();
  await dialog.getByText("查看备份中的资料模板", { exact: true }).click();
  await expect(dialog).toContainText("备份中的教学提示");
  await expect(dialog).toContainText("追加备注");
  await expect(dialog).toContainText("添加标签");
  await dialog
    .getByText("查看计划、学习资料与模板冲突", { exact: true })
    .click();
  await page.screenshot({
    path: "../outputs/Neothesia-资料模板备份预览.png",
    fullPage: true,
  });
  await dialog.getByLabel("练习备份冲突策略").selectOption("keep");
  await dialog
    .getByRole("button", { name: "按所选范围导入", exact: true })
    .click();
  await expect(dialog.getByRole("status")).toContainText("导入完成");
  registry = await cmd({ type: "metadataTemplates" });
  expect(
    registry.templates.find((t: { name: string }) => t.name === "可迁移课堂")
      .rules.notes.value,
  ).toBe("本机最新提示");
  expect(
    registry.templates.some((t: { name: string }) => t.name === "本机独有模板"),
  ).toBeTruthy();
  await dialog.getByLabel("练习备份文件").setInputFiles(file);
  await expect(
    dialog.getByRole("button", { name: "按所选范围导入", exact: true }),
  ).toBeEnabled();
  await dialog.getByLabel("练习备份冲突策略").selectOption("backup");
  await dialog
    .getByRole("button", { name: "按所选范围导入", exact: true })
    .click();
  await expect(dialog.getByRole("status")).toContainText("导入完成");
  registry = await cmd({ type: "metadataTemplates" });
  expect(
    registry.templates.find((t: { name: string }) => t.name === "可迁移课堂")
      .rules.notes.value,
  ).toBe("备份中的教学提示");
  expect(
    registry.templates.some((t: { name: string }) => t.name === "本机独有模板"),
  ).toBeTruthy();
  // A backup with only templates cannot import them through an unrelated selected scope.
  await dialog.getByLabel("练习备份文件").setInputFiles(file);
  await expect(
    dialog.getByRole("button", { name: "按所选范围导入", exact: true }),
  ).toBeEnabled();
  await dialog.getByLabel("片段、阶梯、技术练习、音轨与资料模板").uncheck();
  await dialog.getByLabel("计划、日课与曲目学习").check();
  await expect(
    dialog.getByRole("button", { name: "按所选范围导入", exact: true }),
  ).toBeDisabled();
  await expect(
    dialog.getByRole("button", { name: "关闭练习资料备份", exact: true }),
  ).toBeEnabled();
  await dialog
    .getByRole("button", { name: "关闭练习资料备份", exact: true })
    .click();
});
