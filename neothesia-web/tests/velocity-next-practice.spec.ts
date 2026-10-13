import { expect, test } from "@playwright/test";

test("逐音力度超限可从反馈安排次日复习并保留目标", async ({ page, request }) => {
  test.setTimeout(60000);
  const cmd = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const state = async () => (await (await request.get("http://127.0.0.1:32124/api/state")).json());
  const velocities = [40, 55, 70, 85, 100, 85, 70, 55];
  const events: number[] = [];
  for (const velocity of velocities)
    events.push(0, 144, 60, velocity, 129, 112, 128, 60, 0);
  events.push(0, 255, 47, 0);
  const size = Buffer.alloc(4);
  size.writeUInt32BE(events.length);
  const midi = Buffer.concat([
    Buffer.from([77, 84, 104, 100, 0, 0, 0, 6, 0, 0, 0, 1, 1, 224]),
    Buffer.from("MTrk"), size, Buffer.from(events),
  ]);
  const song = await cmd({ type: "importMidi", name: "力度复习课堂.mid", bytes: [...midi] });
  await cmd({ type: "mode", value: "wait" });
  await cmd({ type: "countIn", bars: 0 });
  await cmd({ type: "speed", value: 2 });
  await page.goto("/");
  const editor = page.locator(".song-expression-goal");
  if (!(await editor.isVisible())) await page.getByRole("button", { name: "练习参数" }).click();
  await expect(editor.getByLabel("当前曲目要求逐音力度差")).toBeEnabled();
  await editor.getByLabel("当前曲目要求逐音力度差").check();
  await editor.getByLabel("当前曲目逐音力度差").fill("8");
  await editor.getByRole("button", { name: "保存目标" }).click();
  await cmd({ type: "play" });
  let played = 0;
  await expect.poll(async () => {
    const current = await state();
    if (current.status !== "finished") {
      for (const pitch of current.required) {
        const velocity = velocities[played] + (played === 3 ? 20 : 0);
        await cmd({ type: "note", pitch, active: true, velocity });
        await cmd({ type: "note", pitch, active: false, velocity: 0 });
        played++;
      }
    }
    return (await state()).status;
  }, { timeout: 12000, intervals: [20] }).toBe("finished");
  expect(played).toBe(8);
  const failed = await state();
  expect(failed.songExpressionGoalResult.passed).toBe(false);
  await cmd({ type: "save" });
  const saved = await state();
  expect(saved.currentHistoryId).toMatch(/^attempt-/);
  const problems = await cmd({ type: "historyProblemNotes", content_id: song.contentId, id: saved.currentHistoryId, offset: 0 });
  expect(problems.total).toBe(1);
  expect(problems.notes[0].kind).toBe("velocityHigh");
  expect(problems.notes[0].velocityDifference).toBe(20);
  await page.getByRole("button", { name: "逐小节反馈" }).click();
  const feedback = page.getByRole("dialog", { name: "逐小节反馈" });
  await expect(feedback.getByRole("button", { name: "预览并安排下次练习" })).toBeVisible();
  await feedback.getByRole("button", { name: "预览并安排下次练习" }).click();
  const plan = page.getByRole("dialog", { name: "历史问题段落编排" });
  await expect(plan).toContainText("逐音力度超限");
  await expect(plan).toContainText("本项目保留原轮逐音力度目标：每音差 ≤ 8");
  await plan.getByLabel("编排达标次数").fill("1");
  await plan.getByRole("button", { name: "保存练习编排" }).click();
  await expect(plan.getByRole("status")).toContainText("新增 1 项");
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  const day = `${tomorrow.getFullYear()}-${String(tomorrow.getMonth() + 1).padStart(2, "0")}-${String(tomorrow.getDate()).padStart(2, "0")}`;
  const routines = await cmd({ type: "routines", day });
  const run = routines.runs.at(-1);
  const item = run.routine.items[0];
  expect(item.goal.expression.noteVelocityDifference).toBe(8);
  await cmd({ type: "openRoutineItem", routine_id: run.routine.id, day, item_id: item.id, resume: false });
  const practiceRound = async (attempt: number, wrong: boolean) => {
    await cmd({ type: "play" });
    let index = 0;
    await expect.poll(async () => {
      const current = await state();
      if (current.routine?.progress.attempts >= attempt) return current.routine.progress.attempts;
      for (const pitch of current.required) {
        await cmd({ type: "note", pitch, active: true, velocity: velocities[index] + (wrong && index === 3 ? 20 : 0) });
        await cmd({ type: "note", pitch, active: false, velocity: 0 });
        index++;
      }
      return (await state()).routine.progress.attempts;
    }, { timeout: 12000, intervals: [20] }).toBe(attempt);
    expect(index).toBe(8);
  };
  await practiceRound(1, true);
  expect((await state()).routine.progress.passed).toBe(0);
  await practiceRound(2, false);
  expect((await state()).routine.progress.completed).toBe(true);
});
