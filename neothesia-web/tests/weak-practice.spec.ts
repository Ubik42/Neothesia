import { test, expect } from "@playwright/test";
test("实际错音证据生成复习、预览接受、去重及两轮达标", async ({
  page,
  request,
}) => {
  test.setTimeout(90000);
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
  const notes = Array(8)
    .fill(
      "<note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note>",
    )
    .join("");
  const xml = `<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>8</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes>${notes}</measure></part></score-partwise>`;
  await cmd({
    type: "importScore",
    name: "薄弱证据课堂.musicxml",
    bytes: Array.from(Buffer.from(xml)),
    default_bpm: 120,
  });
  await cmd({ type: "mode", value: "wait" });
  await cmd({ type: "countIn", bars: 0 });
  for (let i = 0; i < 2; i++) {
    await cmd({ type: "restart" });
    await cmd({ type: "play" });
    await expect
      .poll(
        async () => {
          const s = await (
            await request.get("http://127.0.0.1:32124/api/state")
          ).json();
          for (const pitch of s.required) {
            await cmd({ type: "note", pitch: 61, active: true, velocity: 90 });
            await cmd({ type: "note", pitch: 61, active: false, velocity: 0 });
            await cmd({ type: "note", pitch, active: true, velocity: 90 });
            await cmd({ type: "note", pitch, active: false, velocity: 0 });
          }
          return (
            await cmd({
              type: "historyQuery",
              query: "薄弱证据课堂",
              mode: "",
              hands: "",
              range: "",
              days: 0,
              offset: 0,
              limit: 50,
            })
          ).total;
        },
        { timeout: 20000, intervals: [50] },
      )
      .toBe(i + 1);
    if (i === 0)
      expect((await cmd({ type: "weakPractice" })).suggestions).toHaveLength(0);
  }
  const suggestion = (await cmd({ type: "weakPractice" })).suggestions[0];
  expect(suggestion.attempts).toBe(2);
  expect(suggestion.accuracy).toBeLessThan(0.9);
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "练习计划", exact: true }).click();
  await page.getByLabel("练习计划日期").fill("2026-10-04");
  await page.locator(".weak-practice-panel summary").click();
  await page.getByRole("button", { name: "生成复习建议", exact: true }).click();
  await expect(page.locator(".weak-practice-rows")).toContainText(
    "薄弱证据课堂",
  );
  await expect(page.locator(".weak-practice-rows")).toContainText(
    "2 次完整小节",
  );
  await page.getByLabel("复习计划名称").fill("本周薄弱段复习");
  await page.screenshot({
    path: "../outputs/Neothesia-薄弱小节复习.png",
    fullPage: true,
  });
  await page
    .getByRole("button", { name: "加入勾选的复习项目", exact: true })
    .click();
  await expect(
    page.locator(".weak-practice-panel").getByRole("status"),
  ).toContainText("已加入 1 项");
  await expect(page.locator(".routine-item")).toHaveCount(1);
  await page
    .getByRole("button", { name: "加入勾选的复习项目", exact: true })
    .click();
  await expect(
    page.locator(".weak-practice-panel").getByRole("status"),
  ).toContainText("跳过相同条件项目 1 项");
  const data = await cmd({ type: "routines", day: "2026-10-04" });
  const plan = data.plans.find((p: any) => p.name === "本周薄弱段复习");
  expect(plan.items).toHaveLength(1);
  expect(
    data.runs.find((r: any) => r.routine.id === plan.id).routine.items,
  ).toHaveLength(1);
  await page
    .getByRole("button", {
      name: `练习项目：${plan.items[0].title}`,
      exact: true,
    })
    .click();
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
      { timeout: 25000, intervals: [50] },
    )
    .toBe(true);
  await cmd({ type: "stopRoutine" });
  const done = (await cmd({ type: "routines", day: "2026-10-04" })).runs.find(
    (r: any) => r.routine.id === plan.id,
  );
  expect(done.progress[plan.items[0].id].passed).toBe(2);
});
