import { expect, test } from "@playwright/test";

test("逐音踏板目标保留条件，并把缺少实弹 CC64 的结果写入历史", async ({ page, request }) => {
  test.setTimeout(60000);
  const cmd = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const state = async () => (await request.get("http://127.0.0.1:32124/api/state")).json();
  const events: number[] = [0, 176, 64, 127];
  for (let i = 0; i < 5; i++) {
    events.push(0x81, 0x70, 144, 60, 80, 0x81, 0x70, 128, 60, 0);
  }
  events.push(0, 255, 47, 0);
  const size = Buffer.alloc(4);
  size.writeUInt32BE(events.length);
  const midi = Buffer.concat([
    Buffer.from([77, 84, 104, 100, 0, 0, 0, 6, 0, 0, 0, 1, 1, 224]),
    Buffer.from("MTrk"), size, Buffer.from(events),
  ]);
  const song = await cmd({ type: "importMidi", name: "逐音踏板课堂.mid", bytes: [...midi] });
  await cmd({ type: "setSongExpressionGoal", goal: null });
  await cmd({ type: "track", id: song.notes[0].track, mode: "human", part: "right", visible: true });
  await cmd({ type: "mode", value: "wait" });
  await cmd({ type: "countIn", bars: 0 });
  await cmd({ type: "speed", value: 2 });
  await page.goto("/");
  const goal = page.locator(".song-expression-goal");
  if (!(await goal.isVisible())) await page.getByRole("button", { name: "练习参数" }).click();
  await goal.getByLabel("当前曲目要求逐音踏板不一致音数").check();
  await goal.getByLabel("当前曲目逐音踏板不一致音数").fill("0");
  await goal.getByRole("button", { name: "保存目标" }).click();
  await expect(goal).toContainText("已保存：逐音踏板不一致 ≤ 0 音");
  await page.reload();
  await expect(goal.getByLabel("当前曲目逐音踏板不一致音数")).toHaveValue("0");
  await cmd({ type: "play" });
  let played = 0;
  await expect.poll(async () => {
    const current = await state();
    if (current.status !== "finished") {
      for (const pitch of current.required) {
        await cmd({ type: "note", pitch, active: true, velocity: 80 });
        await cmd({ type: "note", pitch, active: false, velocity: 0 });
        played++;
      }
    }
    return (await state()).status;
  }, { timeout: 12000, intervals: [20] }).toBe("finished");
  expect(played).toBe(5);
  const finished = await state();
  expect(finished.songExpressionGoalResult.passed).toBe(false);
  expect(finished.songExpressionGoalResult.message).toContain("CC64");
  expect(finished.songExpressionGoalResult.notePedal.unknownStateNotes).toBe(5);
  await cmd({ type: "save" });
  await page.getByRole("button", { name: "练习历史", exact: true }).click();
  const history = page.getByRole("dialog", { name: "练习历史", exact: true });
  await history.locator(".history-record").first().click();
  await expect(history).toContainText("本次单曲目标：逐音踏板不一致 ≤ 0 音");
  await expect(history).toContainText("状态未知 5 音");
});
