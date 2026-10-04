import { test, expect } from "@playwright/test";
test("星期规则、独立日期、暂停恢复和实际练习", async ({ page, request }) => {
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  await cmd({ type: "stopRoutine" });
  await cmd({ type: "pause" });
  const xml =
    '<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure></part></score-partwise>';
  await cmd({
    type: "importScore",
    name: "周期课堂.musicxml",
    bytes: Array.from(Buffer.from(xml)),
    default_bpm: 120,
  });
  await cmd({ type: "mode", value: "wait" });
  await cmd({ type: "countIn", bars: 0 });
  const plan = await cmd({
    type: "saveRoutine",
    id: null,
    day: null,
    name: "周五日课",
    notes: "每周教师检查",
    copy_of: null,
  });
  const item = await cmd({
    type: "saveRoutineItem",
    routine_id: plan.id,
    day: null,
    id: null,
    source: "current",
    source_id: null,
    title: "复习本周曲目",
    notes: "先慢后快",
    goal: {
      passes: 1,
      accuracy: 90,
      onTime: null,
      consecutive: false,
      attemptLimit: 20,
    },
  });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "练习计划", exact: true }).click();
  await page.getByRole("button", { name: "常用计划", exact: true }).click();
  await page.locator(".routine-plan").filter({ hasText: "周五日课" }).click();
  await page.locator(".routine-schedule summary").click();
  for (const d of ["一", "二", "三", "四"])
    await page.getByLabel(`星期${d}`, { exact: true }).uncheck();
  await page.getByLabel("周期开始日期").fill("2026-10-02");
  await page.getByLabel("周期结束日期").fill("2026-10-16");
  await page.getByRole("button", { name: "保存周期安排", exact: true }).click();
  await expect(page.locator(".routine-schedule summary")).toContainText(
    "每周五",
  );
  await page.screenshot({
    path: "../outputs/Neothesia-周期日课.png",
    fullPage: true,
  });
  const date = async (day: string) =>
    (await cmd({ type: "routines", day })).runs.filter(
      (r: any) => r.routine.id === plan.id,
    );
  expect(await date("2026-10-03")).toHaveLength(0);
  expect(await date("2026-10-23")).toHaveLength(0);
  expect(await date("2026-10-02")).toHaveLength(1);
  await cmd({
    type: "skipRoutineItem",
    routine_id: plan.id,
    day: "2026-10-02",
    item_id: item.id,
    reason: "课堂已完成",
  });
  await page.getByRole("button", { name: "暂停周期安排", exact: true }).click();
  await expect(page.locator(".routine-schedule summary")).toContainText(
    "已暂停",
  );
  expect(await date("2026-10-09")).toHaveLength(0);
  await page.getByRole("button", { name: "恢复周期安排", exact: true }).click();
  expect(await date("2026-10-09")).toHaveLength(1);
  expect((await date("2026-10-02"))[0].progress[item.id].skipped).toBe(true);
  await page.getByRole("button", { name: "当天安排", exact: true }).click();
  await page.getByLabel("练习计划日期").fill("2026-10-09");
  await expect(
    page.locator(".routine-plan").filter({ hasText: "周五日课" }),
  ).toContainText("已建立当天安排");
  await page
    .locator(".routine-item-select")
    .filter({ hasText: "复习本周曲目" })
    .click();
  await page.getByRole("button", { name: "练习项目：复习本周曲目", exact: true }).click();
  await cmd({ type: "play" });
  await expect
    .poll(
      async () => {
        const s = await (
          await request.get("http://127.0.0.1:32124/api/state")
        ).json();
        for (const pitch of s.required) {
          await cmd({ type: "note", pitch, active: true, velocity: 90 });
          await cmd({ type: "note", pitch, active: false, velocity: 0 });
        }
        return (
          await (await request.get("http://127.0.0.1:32124/api/state")).json()
        ).routine.progress.completed;
      },
      { timeout: 15000 },
    )
    .toBe(true);
  await cmd({ type: "stopRoutine" });
  const backup = await cmd({
    type: "practiceBackupSnapshot",
    selection: { plans: true, records: true, presets: true, days: 0 },
  });
  expect(
    backup.routines.find((r: any) => r.id === plan.id).schedule.weekdays,
  ).toEqual([4]);
  await cmd({ type: "saveRoutineSchedule", id: plan.id, schedule: null });
  expect(await date("2026-10-16")).toHaveLength(0);
  expect((await date("2026-10-09"))[0].progress[item.id].completed).toBe(true);
});
