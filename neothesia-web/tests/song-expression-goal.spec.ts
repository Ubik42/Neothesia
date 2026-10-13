import { expect, test } from "@playwright/test";

test("单曲力度目标从练习参数保存并在重开后恢复", async ({ page, request }) => {
  const cmd = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const events: number[] = [];
  for (const velocity of [40, 55, 70, 85, 100, 85, 70, 55]) {
    events.push(0, 144, 60, velocity, 129, 112, 128, 60, 0);
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
  await cmd({ type: "importMidi", name: "单曲目标课堂.mid", bytes: [...midi] });
  await cmd({ type: "mode", value: "flow" });
  await page.goto("/");
  const goal = page.locator(".song-expression-goal");
  if (!(await goal.isVisible())) await page.getByRole("button", { name: "练习参数" }).click();
  await goal.getByLabel("当前曲目要求力度起伏一致率").check();
  await goal.getByLabel("当前曲目力度起伏一致率").fill("85");
  await goal.getByRole("button", { name: "保存目标" }).click();
  await expect(goal).toContainText("已保存：起伏一致率 ≥ 85%");
  await page.screenshot({ path: "../outputs/Neothesia-单曲力度踏板目标.png" });
  const saved = await (await request.get("http://127.0.0.1:32124/api/state")).json();
  expect(saved.songExpressionGoal.contourPercent).toBe(85);
  await page.reload();
  await expect(goal.getByLabel("当前曲目力度起伏一致率")).toHaveValue("85");
  await cmd({ type: "mode", value: "wait" });
  await cmd({ type: "countIn", bars: 0 });
  await cmd({ type: "speed", value: 2 });
  await cmd({ type: "play" });
  let played = 0;
  await expect.poll(async () => {
    const current = await (await request.get("http://127.0.0.1:32124/api/state")).json();
    if (current.status !== "finished") {
      for (const note of current.required) {
        await cmd({ type: "note", pitch: note, active: true, velocity: [40, 55, 70, 85, 100, 85, 70, 55][played % 8] });
        await cmd({ type: "note", pitch: note, active: false, velocity: 0 });
        played++;
      }
    }
    return (await (await request.get("http://127.0.0.1:32124/api/state")).json()).status;
  }, { timeout: 12000, intervals: [20] }).toBe("finished");
  expect(played).toBe(8);
  const finished = await (await request.get("http://127.0.0.1:32124/api/state")).json();
  expect(finished.songExpressionGoalResult.passed).toBe(true);
  await cmd({ type: "save" });
  await page.getByRole("button", { name: "练习历史", exact: true }).click();
  const history = page.getByRole("dialog", { name: "练习历史", exact: true });
  await history.locator(".history-record").first().click();
  await expect(history).toContainText("本次单曲目标：起伏一致率 ≥ 85%");
  await expect(history).toContainText("本轮目标：本轮力度/踏板要求达标");
  await page.getByRole("button", { name: "关闭练习历史" }).click();
  await goal.getByLabel("当前曲目要求力度起伏一致率").uncheck();
  await goal.getByRole("button", { name: "保存目标" }).click();
  await expect(goal).not.toContainText("已保存：");
});
