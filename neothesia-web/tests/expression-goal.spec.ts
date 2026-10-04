import { test, expect } from "@playwright/test";
test("力度目标编辑、实际评分与历史备份", async ({ page, request }) => {
  test.setTimeout(60000);
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const state = async () =>
    (await request.get("http://127.0.0.1:32124/api/state")).json();
  const velocities = [40, 55, 70, 85, 100, 85, 70, 55];
  const events: number[] = [];
  for (const v of velocities) events.push(0, 144, 60, v, 129, 112, 128, 60, 0);
  events.push(0, 255, 47, 0);
  const size = Buffer.alloc(4);
  size.writeUInt32BE(events.length);
  const midi = Buffer.concat([
    Buffer.from([77, 84, 104, 100, 0, 0, 0, 6, 0, 0, 0, 1, 1, 224]),
    Buffer.from("MTrk"),
    size,
    Buffer.from(events),
  ]);
  await cmd({ type: "importMidi", name: "力度课堂.mid", bytes: [...midi] });
  await cmd({ type: "mode", value: "wait" });
  await cmd({ type: "countIn", bars: 0 });
  await cmd({ type: "speed", value: 2 });
  const song = await cmd({ type: "currentSong" });
  const planName=`力度起伏日课-${Date.now()}`;
  const plan = await cmd({
    type: "saveRoutine",
    id: null,
    day: null,
    name: planName,
    notes: "",
    copy_of: null,
  });
  const item = await cmd({
    type: "saveRoutineItem",
    routine_id: plan.id,
    day: null,
    id: null,
    source: "current",
    source_id: null,
    title: "力度起伏练习",
    notes: "参考 MIDI 对照",
    goal: {
      passes: 2,
      accuracy: 90,
      onTime: null,
      consecutive: true,
      attemptLimit: 10,
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "练习计划", exact: true }).click();
  await page.getByRole("button", { name: "常用计划", exact: true }).click();
  await page
    .locator(".routine-plan")
    .filter({ hasText: planName })
    .click();
  await page
    .locator(".routine-item-select")
    .filter({ hasText: "力度起伏练习" })
    .click();
  await page.getByLabel("计划项目要求力度平均差", { exact: true }).check();
  await page.getByLabel("计划项目力度平均差", { exact: true }).fill("64");
  await page.getByLabel("计划项目要求力度起伏一致率", { exact: true }).check();
  await expect(
    page.getByLabel("计划项目要求踏板时机偏移", { exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "保存项目", exact: true }).click();
  await expect
    .poll(
      async () =>
        (await cmd({ type: "routines", day: "2026-10-06" })).plans.find(
          (p: any) => p.id === plan.id,
        ).items[0].goal.expression?.contourPercent,
    )
    .toBe(80);
  await page.screenshot({ path: "../outputs/Neothesia-力度踏板练习目标.png" });
  await page.getByRole("button", { name: "关闭练习计划" }).click();
  await cmd({
    type: "openRoutineItem",
    routine_id: plan.id,
    day: "2026-10-06",
    item_id: item.id,
    resume: false,
  });
  await page.reload();
  await expect(page.locator(".routine-expression-target")).toContainText(
    "起伏一致率 ≥ 80%",
  );
  const playRound = async (expected: number, flat: boolean) => {
    await cmd({ type: "play" });
    let index = 0;
    await expect
      .poll(
        async () => {
          const s = await state();
          if (s.routine.progress.attempts >= expected)
            return s.routine.progress.attempts;
          for (const pitch of s.required) {
            await cmd({
              type: "note",
              pitch,
              active: true,
              velocity: flat ? 80 : velocities[index % 8],
            });
            await cmd({ type: "note", pitch, active: false, velocity: 0 });
            index++;
          }
          return (await state()).routine.progress.attempts;
        },
        { timeout: 12000, intervals: [20] },
      )
      .toBe(expected);
    expect(index).toBe(8);
  };
  await playRound(1, true);
  expect((await state()).routine.progress.passed).toBe(0);
  await expect(page.locator(".routine-last-result")).toContainText(
    "力度起伏未达标",
  );
  await playRound(2, false);
  await playRound(3, false);
  expect((await state()).routine.progress.completed).toBe(true);
  await cmd({ type: "stopRoutine" });
  const history = await cmd({
    type: "historyQuery",
    query: "力度课堂",
    mode: "",
    hands: "",
    range: "",
    days: 0,
    offset: 0,
    limit: 10,
  });
  const rows = history.entries ?? history.rows;
  expect(rows.length).toBeGreaterThanOrEqual(3);
  const latest = await cmd({
    type: "historyDetail",
    content_id: song.contentId,
    id: rows[0].id,
  });
  expect(latest.context.routine.expression.contour_percent).toBe(80);
  expect(latest.expressionGoalResult.passed).toBe(true);
  const backup = await cmd({
    type: "practiceBackupSnapshot",
    selection: { plans: true, records: true, presets: false, days: 0 },
  });
  expect(
    backup.routines.find((p: any) => p.id === plan.id).items[0].goal.expression
      .contourPercent,
  ).toBe(80);
});
