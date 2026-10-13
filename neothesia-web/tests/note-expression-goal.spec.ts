import { expect, test } from "@playwright/test";

test("逐音力度目标保存重开，失败和达标分别留在历史记录", async ({ page, request }) => {
  test.setTimeout(60000);
  const cmd = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const reference = [40, 55, 70, 85, 100, 85, 70, 55];
  const events: number[] = [];
  for (const velocity of reference)
    events.push(0, 144, 60, velocity, 129, 112, 128, 60, 0);
  events.push(0, 255, 47, 0);
  const size = Buffer.alloc(4);
  size.writeUInt32BE(events.length);
  const midi = Buffer.concat([
    Buffer.from([77, 84, 104, 100, 0, 0, 0, 6, 0, 0, 0, 1, 1, 224]),
    Buffer.from("MTrk"),
    size,
    Buffer.from(events),
  ]);
  await cmd({ type: "importMidi", name: "逐音力度目标课堂.mid", bytes: [...midi] });
  await cmd({ type: "mode", value: "wait" });
  await cmd({ type: "countIn", bars: 0 });
  await cmd({ type: "speed", value: 2 });
  await page.goto("/");
  const goal = page.locator(".song-expression-goal");
  if (!(await goal.isVisible()))
    await page.getByRole("button", { name: "练习参数" }).click();
  await goal.getByLabel("当前曲目要求逐音力度差").check();
  await goal.getByLabel("当前曲目逐音力度差").fill("8");
  await goal.getByRole("button", { name: "保存目标" }).click();
  await expect(goal).toContainText("已保存：逐音力度差 ≤ 8");
  await page.reload();
  await expect(goal.getByLabel("当前曲目逐音力度差")).toHaveValue("8");

  const play = async (wrongIndex: number | null) => {
    await cmd({ type: "play" });
    let played = 0;
    await expect.poll(async () => {
      const state = await (await request.get("http://127.0.0.1:32124/api/state")).json();
      if (state.status !== "finished") {
        for (const note of state.required) {
          const velocity = reference[played % reference.length] + (played === wrongIndex ? 20 : 0);
          await cmd({ type: "note", pitch: note, active: true, velocity });
          await cmd({ type: "note", pitch: note, active: false, velocity: 0 });
          played++;
        }
      }
      return (await (await request.get("http://127.0.0.1:32124/api/state")).json()).status;
    }, { timeout: 12000, intervals: [20] }).toBe("finished");
    expect(played).toBe(reference.length);
    const result = await (await request.get("http://127.0.0.1:32124/api/state")).json();
    await cmd({ type: "save" });
    return result.songExpressionGoalResult;
  };

  const failed = await play(3);
  expect(failed.passed).toBe(false);
  expect(failed.message).toContain("逐音力度");
  expect(failed.noteVelocity.matchedNotes).toBe(reference.length);
  expect(failed.noteVelocity.maxAbsDifference).toBeGreaterThan(8);
  const passed = await play(null);
  expect(passed.passed).toBe(true);
  expect(passed.noteVelocity.maxAbsDifference).toBeLessThanOrEqual(8);

  await page.getByRole("button", { name: "练习历史", exact: true }).click();
  const history = page.getByRole("dialog", { name: "练习历史", exact: true });
  await history.locator(".history-record").first().click();
  await expect(history).toContainText("本次单曲目标：逐音力度差 ≤ 8");
  await expect(history).toContainText("本轮目标：本轮力度/踏板要求达标");
  await history.locator(".history-performance summary").click();
  await history.getByRole("button", { name: "查看这次演奏" }).click();
  await expect(history).toContainText("本轮保存的逐音目标是力度差最多 8 级");
  await history.locator(".history-record").nth(1).click();
  await expect(history).toContainText("本次单曲目标：逐音力度差 ≤ 8");
  await expect(history).toContainText("逐音力度");
});
