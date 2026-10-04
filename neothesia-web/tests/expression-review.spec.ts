import { test, expect } from "@playwright/test";

test("完整演奏力度证据生成专项日课，去重并实际达标", async ({
  page,
  request,
}) => {
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
  await cmd({ type: "stopRoutine" });
  await cmd({ type: "pause" });
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
  await cmd({ type: "importMidi", name: "专项推荐课堂.mid", bytes: [...midi] });
  await cmd({ type: "mode", value: "wait" });
  await cmd({ type: "countIn", bars: 0 });
  await cmd({ type: "speed", value: 2 });
  const song = await cmd({ type: "currentSong" });
  const play = async (flat: boolean, expected: number, routine: boolean) => {
    if (!routine) await cmd({ type: "restart" });
    await cmd({ type: "play" });
    let index = 0;
    await expect
      .poll(
        async () => {
          const s = await state();
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
          if (routine) return (await state()).routine.progress.attempts;
          return (
            await cmd({
              type: "historyQuery",
              query: "专项推荐课堂",
              mode: "",
              hands: "",
              range: "",
              days: 0,
              offset: 0,
              limit: 50,
            })
          ).total;
        },
        { timeout: 15000, intervals: [20] },
      )
      .toBe(expected);
    expect(index).toBe(8);
  };
  await play(true, 1, false);
  expect(
    (await cmd({ type: "weakPractice" })).suggestions.filter(
      (r: any) => r.item.contentId === song.contentId && r.kind === "dynamics",
    ),
  ).toHaveLength(0);
  await play(true, 2, false);
  const suggestion = (await cmd({ type: "weakPractice" })).suggestions.find(
    (r: any) => r.kind === "dynamics" && r.item.contentId === song.contentId,
  );
  expect(suggestion.range).toBe("全曲");
  expect(suggestion.expression.contourPercent).toBe(0);
  expect(suggestion.item.goal.expression.contourPercent).toBe(85);
  await page.goto("/");
  await page.getByRole("button", { name: "练习计划", exact: true }).click();
  await page.getByLabel("练习计划日期").fill("2026-10-07");
  await page.locator(".weak-practice-panel summary").click();
  await page.getByRole("button", { name: "生成复习建议", exact: true }).click();
  await expect(page.locator(".weak-practice-rows")).toContainText(
    "全曲 · 力度起伏",
  );
  await expect(page.locator(".weak-practice-rows")).toContainText(
    "起伏一致率 ≥ 85%",
  );
  const name = `力度专项复习-${Date.now()}`;
  await page.getByLabel("复习计划名称").fill(name);
  await page.screenshot({
    path: "../outputs/Neothesia-力度踏板专项推荐.png",
    fullPage: true,
  });
  await page
    .getByRole("button", { name: "加入勾选的复习项目", exact: true })
    .click();
  await expect(
    page.locator(".weak-practice-panel").getByRole("status"),
  ).toContainText("已加入 1 项");
  await page
    .getByRole("button", { name: "加入勾选的复习项目", exact: true })
    .click();
  await expect(
    page.locator(".weak-practice-panel").getByRole("status"),
  ).toContainText("跳过相同条件项目 1 项");
  const plans = await cmd({ type: "routines", day: "2026-10-07" });
  const plan = plans.plans.find((p: any) => p.name === name);
  expect(plan.items[0].goal.expression.contourPercent).toBe(85);
  await page.getByRole("button", { name: "关闭练习计划" }).click();
  await cmd({
    type: "openRoutineItem",
    routine_id: plan.id,
    day: "2026-10-07",
    item_id: plan.items[0].id,
    resume: false,
  });
  await play(false, 1, true);
  await play(false, 2, true);
  expect((await state()).routine.progress.completed).toBe(true);
  await cmd({ type: "stopRoutine" });
});
