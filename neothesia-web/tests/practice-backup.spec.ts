import { test, expect } from "@playwright/test";
import fs from "node:fs/promises";
test("练习资料导出预览、计划冲突、日期进度、成绩去重与导入后实际达标", async ({
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
  const xml='<score-partwise version="4.0"><work><work-title>迁移课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure></part></score-partwise>';
  await command({type:"importScore",name:"迁移课堂.musicxml",bytes:Array.from(Buffer.from(xml)),default_bpm:120});
  await command({ type: "mode", value: "wait" });
  await command({ type: "countIn", bars: 0 });
  const plan = await command({
    type: "saveRoutine",
    id: null,
    day: null,
    name: "迁移日课",
    notes: "本周教师要求",
    copy_of: null,
  });
  const item = await command({
    type: "saveRoutineItem",
    routine_id: plan.id,
    day: null,
    id: null,
    source: "current",
    source_id: null,
    title: "课后连奏",
    notes: "保持离键轻柔",
    goal: {
      passes: 1,
      accuracy: 90,
      onTime: null,
      consecutive: false,
      attemptLimit: 20,
    },
  });
  await command({type:"openRoutineItem",routine_id:plan.id,day:"2026-10-04",item_id:item.id,resume:true});await command({type:"play"});
  await expect.poll(async()=>{const state=await(await request.get("http://127.0.0.1:32124/api/state")).json();for(const pitch of state.required){await command({type:"note",pitch,active:true,velocity:90});await command({type:"note",pitch,active:false,velocity:0});}return(await(await request.get("http://127.0.0.1:32124/api/state")).json()).routine.progress.completed;},{timeout:15000}).toBe(true);
  await command({type:"stopRoutine"});
  await command({
    type: "skipRoutineItem",
    routine_id: plan.id,
    day: "2026-10-05",
    item_id: item.id,
    reason: "已在课堂完成，暂不重复",
  });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "练习计划", exact: true }).click();
  await page.getByRole("button", { name: "练习资料备份", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "练习资料备份" });
  const downloadPromise = page.waitForEvent("download");
  await dialog.getByRole("button", { name: "导出练习备份" }).click();
  const download = await downloadPromise;
  const file = "../work/cycle128-export.neopractice";
  await download.saveAs(file);
  const bytes = await fs.readFile(file);
  expect(bytes.length).toBeGreaterThan(200);
  await expect(
    dialog.getByText("练习资料备份已导出", { exact: true }),
  ).toBeVisible();
  await command({
    type: "saveRoutine",
    id: plan.id,
    day: null,
    name: "本地改过的计划",
    notes: "本机的要求",
    copy_of: null,
  });
  await dialog.getByRole("button", { name: "选择练习备份" }).click();
  await dialog.getByLabel("练习备份文件").setInputFiles(file);
  await expect(dialog.getByText("迁移日课", { exact: true })).toBeVisible();
  await expect(dialog.locator(".practice-backup-counts")).toContainText("1");
  await page.screenshot({
    path: "../outputs/Neothesia-练习资料备份.png",
    fullPage: true,
  });
  await dialog.getByRole("button", { name: "按所选范围导入" }).click();
  await expect(dialog.getByRole("status")).toContainText("导入完成");
  let data = await command({ type: "routines", day: "2026-10-05" });
  expect(data.plans.find((p: any) => p.id === plan.id).name).toBe(
    "本地改过的计划",
  );
  expect(
    data.runs.find((r: any) => r.routine.id === plan.id).progress[item.id]
      .skipped,
  ).toBe(true);
  await dialog.getByRole("button", { name: "选择练习备份" }).click();
  await dialog.getByLabel("练习备份文件").setInputFiles(file);
  await expect(dialog.getByText("迁移日课", { exact: true })).toBeVisible();
  await dialog.getByLabel("练习备份冲突策略").selectOption("backup");
  await dialog.getByRole("button", { name: "按所选范围导入" }).click();
  await expect(dialog.getByRole("status")).toContainText("跳过重复 1");
  data = await command({ type: "routines", day: "2026-10-05" });
  expect(data.plans.find((p: any) => p.id === plan.id).name).toBe("迁移日课");
  expect(data.plans.find((p: any) => p.id === plan.id).items[0].notes).toBe(
    "保持离键轻柔",
  );
  expect(
    data.runs.find((r: any) => r.routine.id === plan.id).progress[item.id]
      .skipped,
  ).toBe(true);
  await command({ type: "removeRoutine", id: plan.id });
  await dialog.getByRole("button", { name: "选择练习备份" }).click();
  await dialog.getByLabel("练习备份文件").setInputFiles(file);
  await expect(dialog.getByText("迁移日课", { exact: true })).toBeVisible();
  await dialog.getByRole("button", { name: "按所选范围导入" }).click();
  await expect(dialog.getByRole("status")).toContainText("导入完成");
  await page.getByLabel("关闭练习资料备份").click();
  await command({
    type: "openRoutineItem",
    routine_id: plan.id,
    day: "2026-10-06",
    item_id: item.id,
    resume: true,
  });
  await command({ type: "play" });
  await expect
    .poll(
      async () => {
        const state = await (
          await request.get("http://127.0.0.1:32124/api/state")
        ).json();
        for (const pitch of state.required) {
          await command({ type: "note", pitch, active: true, velocity: 90 });
          await command({ type: "note", pitch, active: false, velocity: 0 });
        }
        return (
          await (await request.get("http://127.0.0.1:32124/api/state")).json()
        ).routine.progress.completed;
      },
      { timeout: 15000 },
    )
    .toBe(true);
  await command({ type: "stopRoutine" });
  const corrupt = Buffer.from(bytes);
  corrupt[0] = 0;
  const invalid = await request.post(
    "http://127.0.0.1:32124/api/practice-backup/stage",
    { headers: { "X-Neothesia-Client": "web" }, data: corrupt },
  );
  expect(invalid.ok()).toBeFalsy();
});
