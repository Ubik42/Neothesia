import { test, expect } from "@playwright/test";
test("实际晚弹生成节奏复习，正确但不准时不能达标", async ({
  page,
  request,
}) => {
  test.setTimeout(120000);
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
  const xml = `<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>8</beats><beat-type>4</beat-type></time></attributes>${Array(8).fill("<note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note>").join("")}</measure></part></score-partwise>`;
  await cmd({ type: "stopRoutine" });
  await cmd({ type: "pause" });
  await cmd({
    type: "importScore",
    name: "节奏证据课堂.musicxml",
    bytes: Array.from(Buffer.from(xml)),
    default_bpm: 120,
  });
  await cmd({ type: "mode", value: "flow" });
  await cmd({ type: "countIn", bars: 0 });
  const sourceRound = async () => {
    await cmd({ type: "restart" });
    await cmd({ type: "play" });
    let next = 0;
    const until = Date.now() + 12000;
    while (Date.now() < until) {
      const s = await state();
      if (s.status === "finished") break;
      if (
        s.status === "playing" &&
        next < 8 &&
        s.position >= next * 0.5 + 0.12
      ) {
        await cmd({ type: "note", pitch: 60, active: true, velocity: 90 });
        await cmd({ type: "note", pitch: 60, active: false, velocity: 0 });
        next++;
      }
      await new Promise((r) => setTimeout(r, 10));
    }
    expect(next).toBe(8);
  };
  await sourceRound();
  expect((await cmd({ type: "weakPractice" })).suggestions).toHaveLength(0);
  await sourceRound();
  const rows = (await cmd({ type: "weakPractice" })).suggestions;
  const rhythm = rows.find((r: any) => r.kind === "rhythm");
  expect(rhythm).toBeTruthy();
  expect(rhythm.accuracy).toBe(1);
  expect(rhythm.timing.onTime).toBeLessThan(0.8);
  expect(rhythm.timing.samples).toBeGreaterThanOrEqual(12);
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "练习计划", exact: true }).click();
  await page.getByLabel("练习计划日期").fill("2026-10-05");
  await page.locator(".weak-practice-panel summary").click();
  await page.getByRole("button", { name: "生成复习建议", exact: true }).click();
  await expect(page.locator(".weak-practice-rows")).toContainText("节奏");
  await expect(page.locator(".weak-practice-rows")).toContainText("准时率");
  await page.getByLabel("复习计划名称").fill("节奏专项复习");
  await page.screenshot({
    path: "../outputs/Neothesia-节奏专项复习.png",
    fullPage: true,
  });
  await page
    .getByRole("button", { name: "加入勾选的复习项目", exact: true })
    .click();
  const plan = (await cmd({ type: "routines", day: "2026-10-05" })).plans.find(
    (p: any) => p.name === "节奏专项复习",
  );
  const item = plan.items.find((i: any) => i.goal.onTime === 85);
  expect(item.settings.metronome).toBe(true);
  await page
    .getByRole("button", { name: `练习项目：${item.title}`, exact: true })
    .click();
  await cmd({ type: "play" });
  const drive = async (late: boolean, target: number) => {
    let next = 0,
      lastAttempt = 0;
    const until = Date.now() + 50000;
    while (Date.now() < until) {
      const s = await state();
      const progress = s.routine.progress;
      if (progress.attempts >= target) return s;
      if (progress.attempts !== lastAttempt) {
        lastAttempt = progress.attempts;
        next = 0;
      }
      if (
        s.status === "playing" &&
        s.countRemaining === 0 &&
        next < 8 &&
        s.position >= next * 0.5 + (late ? 0.12 : 0.01)
      ) {
        await cmd({ type: "note", pitch: 60, active: true, velocity: 90 });
        await cmd({ type: "note", pitch: 60, active: false, velocity: 0 });
        next++;
      }
      await new Promise((r) => setTimeout(r, 8));
    }
    throw Error("实际节奏轮次未完成");
  };
  const failed = await drive(true, 1);
  expect(failed.routine.progress.completed).toBe(false);
  expect(failed.routine.progress.passed).toBe(0);
  expect(failed.routine.progress.lastResult).toBe("准时率未达标");
  await expect(page.locator(".routine-last-result")).toContainText(
    "准时率未达标",
  );
  await cmd({ type: "pause" });
  await cmd({ type: "play" });
  const done = await drive(false, 3);
  expect(done.routine.progress.completed).toBe(true);
  expect(done.routine.progress.streak).toBe(2);
  await cmd({ type: "stopRoutine" });
});
