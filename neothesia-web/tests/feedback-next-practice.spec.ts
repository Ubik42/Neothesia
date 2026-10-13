import { expect, test } from "@playwright/test";

test("保存的本轮问题可从反馈直接安排到明日计划", async ({ page, request }) => {
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const state = async () => (await (await request.get("http://127.0.0.1:32124/api/state")).json());
  const title = `反馈安排下次练习${Date.now()}`;
  const xml = `<score-partwise><work><work-title>${title}</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure><measure number="2"><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration></note></measure></part></score-partwise>`;
  await command({ type: "importScore", name: `${title}.musicxml`, bytes: [...Buffer.from(xml)], default_bpm: 120 });
  const song = await command({ type: "currentSong" });
  await command({ type: "restart" });
  await command({ type: "track", id: song.notes[0].track, mode: "human", part: "right", visible: true });
  await command({ type: "countIn", bars: 0 });
  await command({ type: "mode", value: "recital" });
  await page.goto("/");
  await command({ type: "play" });
  await expect.poll(async () => (await state()).status).toBe("finished");
  const saved = await state();
  expect(saved.saved).toBe(true);
  expect(saved.currentHistoryId).toMatch(/^attempt-\d+$/);
  const archive = await command({ type: "historyProblemNotes", content_id: song.contentId, id: saved.currentHistoryId, offset: 0 });
  expect(archive.total).toBe(2);
  const feedback = page.getByRole("dialog", { name: "演奏完成" });
  await expect(feedback).toBeVisible();
  await expect(feedback.getByRole("button", { name: "预览并安排下次练习" })).toBeVisible();
  await feedback.getByRole("button", { name: "预览并安排下次练习" }).click();
  const plan = page.getByRole("dialog", { name: "历史问题段落编排" });
  await expect(plan).toContainText("2 个问题音 → 2 个练习项目");
  await expect(plan.getByLabel("编排到指定日期")).toBeChecked();
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  const day = `${tomorrow.getFullYear()}-${String(tomorrow.getMonth() + 1).padStart(2, "0")}-${String(tomorrow.getDate()).padStart(2, "0")}`;
  await expect(plan.getByLabel("编排日期")).toHaveValue(day);
  await plan.getByRole("button", { name: "保存练习编排" }).click();
  await expect(plan.getByRole("status")).toContainText("新增 2 项");
  await plan.getByRole("button", { name: "打开已保存计划" }).click();
  await expect(page.getByRole("dialog", { name: "练习计划" })).toContainText("第 2 小节重练");
});
