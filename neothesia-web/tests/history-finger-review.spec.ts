import { expect, test } from "@playwright/test";

test("从历史漏音直接定位原曲音符并检查指法", async ({ page, request }) => {
  test.setTimeout(45000);
  const cmd = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const events: number[] = [];
  for (const pitch of [60, 62, 64, 65])
    events.push(0, 144, pitch, 80, 129, 112, 128, pitch, 0);
  events.push(0, 255, 47, 0);
  const size = Buffer.alloc(4);
  size.writeUInt32BE(events.length);
  const midi = Buffer.concat([
    Buffer.from([77, 84, 104, 100, 0, 0, 0, 6, 0, 0, 0, 1, 1, 224]),
    Buffer.from("MTrk"), size, Buffer.from(events),
  ]);
  const song = await cmd({ type: "importMidi", name: "历史指法复核课堂.mid", bytes: [...midi] });
  await cmd({ type: "track", id: song.notes[0].track, mode: "human", part: "right", visible: true });
  await cmd({ type: "mode", value: "recital" });
  await cmd({ type: "countIn", bars: 0 });
  await cmd({ type: "speed", value: 2 });
  await page.goto("/");
  await cmd({ type: "play" });
  await expect.poll(async () => (await (await request.get("http://127.0.0.1:32124/api/state")).json()).status, { timeout: 12000 }).toBe("finished");
  const feedback = page.getByRole("dialog", { name: "演奏完成" });
  await expect(feedback).toBeVisible();
  await feedback.getByRole("button", { name: "关闭练习反馈" }).click();
  await page.getByRole("button", { name: "练习历史", exact: true }).click();
  const history = page.getByRole("dialog", { name: "练习历史", exact: true });
  await history.locator(".history-record").first().click();
  const pane = history.locator(".history-performance");
  await pane.locator(":scope > summary").click();
  await pane.getByRole("button", { name: "查看这次演奏" }).click();
  await expect(pane).toContainText("漏音");
  await pane.getByRole("button", { name: "检查演奏第 1 小节参考音 2 的指法" }).click();
  const finger = page.getByRole("dialog", { name: "指法建议", exact: true });
  await expect(finger).toBeVisible();
  await expect(finger.getByLabel("指法建议范围")).toHaveValue("note");
  await expect(finger.getByLabel("当前指法音符")).toContainText("D4 · 音符 2");
  const loaded = await cmd({ type: "currentSong" });
  expect(loaded.contentId).toBe(song.contentId);
  expect(loaded.notes[1].pitch).toBe(62);
  await finger.getByRole("button", { name: "生成建议" }).click();
  await expect(finger).toContainText("完整推荐方案");
  await expect(finger.getByRole("button", { name: /接受并保存/ })).toBeEnabled();
  await finger.getByRole("button", { name: /接受并保存/ }).click();
  await expect(finger).toHaveCount(0);
  await cmd({ type: "load", path: song.sourcePath, title: song.title });
  const reopened = await cmd({ type: "currentSong" });
  expect(reopened.notes[1].finger).toBeGreaterThanOrEqual(1);
});
