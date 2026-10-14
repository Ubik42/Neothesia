import { expect, test } from "@playwright/test";

test("阶梯方案保存力度目标并在日课和重开后保留", async ({ page, request }) => {
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(response.ok(), await response.text()).toBe(true);
    return response.json();
  };
  const events: number[] = [];
  for (const velocity of [40, 55, 70, 85, 100, 85, 70, 55]) {
    events.push(0, 144, 60, velocity, 120, 128, 60, 0);
  }
  events.push(0, 255, 47, 0);
  const size = Buffer.alloc(4);
  size.writeUInt32BE(events.length);
  const midi = Buffer.concat([
    Buffer.from([77, 84, 104, 100, 0, 0, 0, 6, 0, 0, 0, 1, 1, 224]),
    Buffer.from("MTrk"),
    size,
    Buffer.from(events),
  ]);
  await command({ type: "importMidi", name: "阶梯力度目标.mid", bytes: [...midi] });
  await command({ type: "mode", value: "wait" });
  await command({ type: "countIn", bars: 0 });
  await page.goto("/");
  await page.getByRole("button", { name: "速度阶梯方案", exact: true }).click();
  await page.getByLabel("阶梯方案名称").fill("力度逐级练习");
  await page.getByLabel("阶梯目标速度").fill("100");
  await page.getByLabel("阶梯要求力度平均差", { exact: true }).check();
  await page.getByLabel("阶梯力度平均差", { exact: true }).fill("50");
  await expect(page.getByLabel("阶梯要求踏板时机偏移", { exact: true })).toBeDisabled();
  await page.getByRole("button", { name: "保存方案", exact: true }).click();
  const preset = (await command({ type: "ladderPresets" })).presets.find(
    (row: { name: string }) => row.name === "力度逐级练习",
  );
  expect(preset.expression.velocityDifference).toBe(50);
  await page.getByRole("button", { name: "关闭速度阶梯" }).click();
  const routine = await command({ type: "saveRoutine", id: null, day: null, name: "阶梯力度日课", notes: "", copy_of: null });
  await command({ type: "saveRoutineItem", routine_id: routine.id, day: null, id: null, source: "ladder", source_id: preset.id, title: "同条件练习", notes: "", goal: { expression: null, passes: 1, accuracy: 90, onTime: null, consecutive: true, attemptLimit: 5 } });
  const routines = await command({ type: "routines", day: "2026-10-02" });
  expect(routines.plans.find((row: { id: string }) => row.id === routine.id).items[0].goal.expression.velocityDifference).toBe(50);
  await page.reload();
  await page.getByRole("button", { name: "速度阶梯方案", exact: true }).click();
  await page.locator(".ladder-preset").filter({ hasText: "力度逐级练习" }).click();
  await expect(page.getByLabel("阶梯要求力度平均差", { exact: true })).toBeChecked();
  await expect(page.getByLabel("阶梯力度平均差", { exact: true })).toHaveValue("50");
  await page.getByRole("button", { name: "从起始速度应用", exact: true }).click();
  await expect(page.locator(".ladder-status")).toContainText("力度平均差 ≤ 50");
  await command({ type: "play" });
  await expect.poll(async () => {
    const state = await (await request.get("http://127.0.0.1:32124/api/state")).json();
    for (const pitch of state.required) {
      await command({ type: "note", pitch, velocity: 1, active: true });
      await command({ type: "note", pitch, velocity: 0, active: false });
    }
    return state.ladder.progress.totalRounds;
  }, { timeout: 12000, intervals: [20] }).toBe(1);
  await expect(page.locator(".ladder-outcome")).toContainText("力度平均差未达标");
});
