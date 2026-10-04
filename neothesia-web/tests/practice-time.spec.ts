import { test, expect } from "@playwright/test";
test("实际练习用时：暂停排除、保存去重、历史与本周目标", async ({
  page,
  request,
}) => {
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
  const xml = `<score-partwise version="4.0"><work><work-title>本周用时课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>${["D", "E", "A", "F"].map((step) => `<note><pitch><step>${step}</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note>`).join("")}</measure></part></score-partwise>`;
  const song = await cmd({
    type: "importScore",
    bytes: Array.from(Buffer.from(xml)),
    name: "本周用时课堂.musicxml",
    default_bpm: 120,
  });
  await cmd({ type: "inspectSong", path: song.sourcePath, force: true });
  await cmd({
    type: "updateSongMetadata",
    path: song.sourcePath,
    value: {
      title: "本周用时课堂",
      composer: null,
      artist: null,
      collection: null,
      difficulty: null,
      tags: [],
      notes: null,
    },
  });
  await cmd({ type: "countIn", bars: 0 });
  await cmd({ type: "play" });
  await expect
    .poll(async () => (await state()).practiceMs)
    .toBeGreaterThan(1100);
  await cmd({ type: "note", pitch: 62, active: true, velocity: 90 });
  await cmd({ type: "note", pitch: 62, active: false, velocity: 0 });
  await cmd({ type: "pause" });
  const paused = (await state()).practiceMs;
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  expect((await state()).practiceMs).toBe(paused);
  await cmd({ type: "save" });
  await cmd({ type: "save" });
  const stats = await cmd({
    type: "practiceTime",
    content_id: song.contentId,
    from: 0,
    to: 9000000000000000,
  });
  expect(stats.totalMs).toBe(paused);
  expect(stats.timedSessions).toBe(1);
  await page.getByRole("button", { name: "练习历史", exact: true }).click();
  const history = page.getByRole("dialog", { name: "练习历史", exact: true });
  await history.getByLabel("查找历史曲目").fill("本周用时课堂");
  await history.locator(".history-list button").first().click();
  await expect(history).toContainText("练习运行用时");
  await history.getByRole("button", { name: "关闭练习历史" }).click();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByLabel("管理曲库搜索").fill("本周用时课堂");
  await page.locator(".library-manager-table tbody tr").first().click();
  let panel = page.getByRole("region", { name: "已保存练习用时" });
  await expect(panel).toContainText("1 次有用时记录");
  await page.getByRole("button", { name: "学习目标", exact: true }).click();
  const d = page.getByRole("dialog", { name: "曲目学习", exact: true });
  await d.getByLabel("曲目每周计划分钟").fill("30");
  panel = d.getByRole("region", { name: "已保存练习用时" });
  await expect(panel).toContainText("计划 30 分钟");
  await expect(panel.getByRole("progressbar")).toHaveAttribute(
    "max",
    "1800000",
  );
  await expect(panel.getByRole("progressbar")).toHaveAttribute(
    "value",
    String(paused),
  );
  await panel.getByRole("button", { name: "刷新用时" }).click();
  await expect(panel).toContainText("1 次有用时记录");
  await page.screenshot({
    path: "../outputs/Neothesia-学习目标与真实用时.png",
  });
  await d.getByRole("button", { name: "保存学习资料", exact: true }).click();
  const weekStart = new Date();
  weekStart.setHours(0, 0, 0, 0);
  weekStart.setDate(weekStart.getDate() - ((weekStart.getDay() + 6) % 7));
  const weekEnd = new Date(weekStart);
  weekEnd.setDate(weekEnd.getDate() + 7);
  expect(
    (
      await cmd({
        type: "practiceTime",
        content_id: song.contentId,
        from: weekStart.getTime(),
        to: weekEnd.getTime(),
      })
    ).periodMs,
  ).toBe(paused);
});
