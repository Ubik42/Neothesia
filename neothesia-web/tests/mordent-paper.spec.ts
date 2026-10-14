import { expect, test } from "@playwright/test";

test("上下波音按反复展开并共用原谱位置", async ({ page, request }) => {
  test.setTimeout(90_000);
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const xml = `<score-partwise version="4.0"><work><work-title>短颤音原谱定位</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions><time><beats>2</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><barline location="left"><repeat direction="forward"/></barline><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><type>quarter</type><notations><ornaments><mordent trill-step="half"/></ornaments></notations></note><note><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration><type>quarter</type><notations><ornaments><inverted-mordent trill-step="half"/></ornaments></notations></note><barline location="right"><repeat direction="backward"/></barline></measure></part></score-partwise>`;
  await command({ type: "importScore", bytes: [...Buffer.from(xml)], name: "短颤音原谱定位.musicxml", default_bpm: 120 });
  const song = await command({ type: "currentSong" });
  expect(song.notes.map((note: { pitch: number }) => note.pitch)).toEqual([60, 59, 60, 64, 65, 64, 60, 59, 60, 64, 65, 64]);
  await command({ type: "pairScore", content_id: song.contentId, name: "短颤音原谱定位.musicxml", bytes: [...Buffer.from(xml)] });
  await page.goto("/");
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await expect(page.getByText(/波音 2 处/)).toBeVisible();
  const generate = page.getByRole("button", { name: "生成可定位纸谱", exact: true });
  await expect(generate).toBeEnabled({ timeout: 30_000 });
  await generate.click();
  const dialog = page.getByRole("dialog", { name: "生成可定位纸谱" });
  await expect(dialog).toContainText("可定位 12 个演奏音符");
  await dialog.getByRole("button", { name: "采用并保存" }).click();
  await expect(dialog).toContainText("已保存。纸谱管理中可以查看这份谱", { timeout: 30_000 });
  const saved = await command({ type: "scoreAttachments", content_id: song.contentId });
  const book = saved.attachments.find((item: { name: string }) => item.name.includes("可定位谱"));
  expect(book.noteAnchors).toHaveLength(12);
  const anchors = book.noteAnchors as Array<{ pitch: number; region: unknown; timestamp: number }>;
  for (const offset of [0, 6]) {
    expect(anchors[offset].region).toEqual(anchors[offset + 1].region);
    expect(anchors[offset + 1].region).toEqual(anchors[offset + 2].region);
    expect(anchors[offset + 3].region).toEqual(anchors[offset + 4].region);
    expect(anchors[offset + 4].region).toEqual(anchors[offset + 5].region);
  }
  expect(anchors[0].region).toEqual(anchors[6].region);
  await page.reload();
  const reopened = await command({ type: "scoreAttachments", content_id: song.contentId });
  expect(reopened.attachments.find((item: { id: string }) => item.id === book.id).noteAnchors).toHaveLength(12);
  const seek = await command({ type: "seekPaperNote", content_id: song.contentId, book: book.id, expected: anchors[7] });
  expect(seek).toBeTruthy();
  const state = await (await request.get("http://127.0.0.1:32124/api/state")).json();
  expect(state.position).toBeCloseTo(song.notes[7].start, 3);
});
