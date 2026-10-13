import { expect, test } from "@playwright/test";

test("本轮小节与手别反馈能准备精确的音轨交集重练", async ({ page, request }) => {
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const readState = async () => (await (await request.get("http://127.0.0.1:32124/api/state")).json());
  const title = `分手反馈重练${Date.now()}`;
  const xml = `<score-partwise><work><work-title>${title}</work-title></work><part-list><score-part id="P1"><part-name>Right</part-name></score-part><score-part id="P2"><part-name>Left</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>5</octave></pitch><duration>1</duration></note></measure></part><part id="P2"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>3</octave></pitch><duration>1</duration></note></measure></part></score-partwise>`;
  await command({ type: "importScore", name: `${title}.musicxml`, bytes: [...Buffer.from(xml)], default_bpm: 120 });
  const song = await command({ type: "currentSong" });
  const right = song.notes.find((note: { pitch: number }) => note.pitch === 72)?.track;
  const left = song.notes.find((note: { pitch: number }) => note.pitch === 48)?.track;
  expect(right).toBeDefined();
  expect(left).toBeDefined();
  expect(right).not.toBe(left);
  await command({ type: "track", id: right, mode: "human", part: "right", visible: true });
  await command({ type: "track", id: left, mode: "human", part: "left", visible: true });
  await command({ type: "countIn", bars: 0 });
  await command({ type: "mode", value: "recital" });
  await page.goto("/");
  await command({ type: "play" });
  await expect.poll(async () => (await readState()).status).toBe("finished");
  const response = await command({ type: "feedback" });
  expect(response.measure_parts).toEqual(expect.arrayContaining([
    expect.objectContaining({ measure: 1, part: "LeftHand", track_id: left, breakdown: expect.objectContaining({ target_notes: 1, missed_notes: 1 }) }),
    expect.objectContaining({ measure: 1, part: "RightHand", track_id: right, breakdown: expect.objectContaining({ target_notes: 1, missed_notes: 1 }) }),
  ]));
  const dialog = page.getByRole("dialog", { name: "演奏完成" });
  await expect(dialog.getByRole("region", { name: "本轮分手问题" })).toBeVisible();
  const leftRow = dialog.locator(".feedback-part-table tbody tr").filter({ hasText: "左手" });
  await expect(leftRow).toContainText("0/1");
  await leftRow.getByRole("button", { name: "准备重练" }).click();
  await expect(dialog.getByRole("group", { name: "分手重练设置预览" })).toContainText("第 1 小节 · 左手");
  await command({ type: "mode", value: "listen" });
  await expect(dialog.getByRole("group", { name: "分手重练设置预览" })).toContainText("当前模式会先切换到连续练习");
  await dialog.getByRole("button", { name: "应用设置并重练" }).click();
  await expect(dialog).toHaveCount(0);
  const next = await readState();
  expect(next.mode).toBe("flow");
  expect(next.hands).toBe("left");
  expect(next.passage).not.toBeNull();
  const selected = await command({ type: "currentSong" });
  expect(selected.tracks.find((track: { id: number }) => track.id === left).mode).toBe("human");
  expect(selected.tracks.find((track: { id: number }) => track.id === right).mode).toBe("auto");
});
