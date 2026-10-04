import { test, expect } from "@playwright/test";
test("真实成绩评语、未保存保护、搜索和并发冲突", async ({ page, request }) => {
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
  const xml =
    '<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure></part></score-partwise>';
  await cmd({
    type: "importScore",
    name: "评语课堂.musicxml",
    bytes: Array.from(Buffer.from(xml)),
    default_bpm: 120,
  });
  await cmd({ type: "mode", value: "wait" });
  await cmd({ type: "countIn", bars: 0 });
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
          await cmd({
            type: "historyQuery",
            query: "评语课堂",
            mode: "",
            hands: "",
            range: "",
            days: 0,
            offset: 0,
            limit: 50,
          })
        ).total;
      },
      { timeout: 15000 },
    )
    .toBeGreaterThan(0);
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "练习历史", exact: true }).click();
  await page.locator(".history-record").first().click();
  await page.getByLabel("练习备注", { exact: true }).fill("左手重复音容易紧张");
  await page.getByLabel("教师评语", { exact: true }).fill("放松手腕，保持连奏");
  await page
    .getByLabel("下次重点", { exact: true })
    .fill("先分手慢练第 1 小节");
  await expect(
    page.getByRole("button", { name: "关闭练习历史" }),
  ).toBeDisabled();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "练习历史" })).toBeVisible();
  await page.getByRole("button", { name: "保存评语", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "关闭练习历史" }),
  ).toBeEnabled();
  await expect(page.locator(".history-record").first()).toContainText("有评语");
  await page.screenshot({
    path: "../outputs/Neothesia-练习评语.png",
    fullPage: true,
  });
  await page.getByLabel("查找历史曲目").fill("手腕");
  await expect(page.locator(".history-record")).toHaveCount(1);
  await page.locator(".history-record").first().click();
  await expect(page.getByLabel("教师评语", { exact: true })).toHaveValue(
    "放松手腕，保持连奏",
  );
  const row = (
    await cmd({
      type: "historyQuery",
      query: "手腕",
      mode: "",
      hands: "",
      range: "",
      days: 0,
      offset: 0,
      limit: 50,
    })
  ).entries[0];
  const original = await cmd({
    type: "historyDetail",
    content_id: row.contentId,
    id: row.id,
  });
  await page.getByLabel("练习备注", { exact: true }).fill("尚未保存的本地修改");
  await cmd({
    type: "annotateHistory",
    content_id: row.contentId,
    id: row.id,
    note: "教师在另一窗口补充",
    teacher: "外部评语",
    next: "继续慢练",
    expected: original.annotation,
  });
  await page.getByRole("button", { name: "保存评语", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("评语已被其他操作更新");
  await expect(page.getByLabel("练习备注", { exact: true })).toHaveValue(
    "尚未保存的本地修改",
  );
  await page.getByRole("button", { name: "放弃修改", exact: true }).click();
  await page.getByRole("button", { name: "关闭练习历史" }).click();
  await page.getByRole("button", { name: "练习历史", exact: true }).click();
  await page.locator(".history-record").first().click();
  await expect(page.getByLabel("教师评语", { exact: true })).toHaveValue(
    "外部评语",
  );
  expect(
    (
      await cmd({
        type: "historyDetail",
        content_id: row.contentId,
        id: row.id,
      })
    ).score,
  ).toEqual(original.score);
});
