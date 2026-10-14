import { expect, test } from "@playwright/test";

for (const { name, ornament, pitches, notationLabel } of [
  { name: "普通回音", ornament: "turn", pitches: [62, 60, 58, 60, 62, 60, 58, 60], notationLabel: "回音 1 处" },
  { name: "明确下邻音起始的倒回音", ornament: 'inverted-turn start-note="below"', pitches: [58, 60, 62, 60, 58, 60, 62, 60], notationLabel: "倒回音 1 处" },
]) test(`${name}的反复击键保存为同一谱音并可在重开后定位`, async ({ page, request }) => {
  test.setTimeout(90_000);
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const xml = `<score-partwise version="4.0"><work><work-title>${name}定位</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>4</divisions><time><beats>1</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><barline location="left"><repeat direction="forward"/></barline><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><type>quarter</type><notations><ornaments><${ornament}/></ornaments></notations></note><barline location="right"><repeat direction="backward"/></barline></measure></part></score-partwise>`;
  await command({ type: "importScore", bytes: [...Buffer.from(xml)], name: `${name}定位.musicxml`, default_bpm: 120 });
  const song = await command({ type: "currentSong" });
  expect(song.notes.map((note: { pitch: number }) => note.pitch)).toEqual(pitches);
  await command({ type: "pairScore", content_id: song.contentId, name: `${name}定位.musicxml`, bytes: [...Buffer.from(xml)] });
  await page.goto("/");
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await expect(page.getByText(notationLabel, { exact: false })).toBeVisible();
  const generate = page.getByRole("button", { name: "生成可定位纸谱", exact: true });
  await expect(generate).toBeEnabled({ timeout: 30_000 });
  await generate.click();
  const dialog = page.getByRole("dialog", { name: "生成可定位纸谱" });
  await expect(dialog).toContainText("可定位 8 个演奏音符");
  await dialog.getByRole("button", { name: "采用并保存" }).click();
  await expect(dialog).toContainText("已保存。纸谱管理中可以查看这份谱", { timeout: 30_000 });
  const saved = await command({ type: "scoreAttachments", content_id: song.contentId });
  const book = saved.attachments.find((item: { name: string }) => item.name.includes("可定位谱"));
  expect(book.noteAnchors).toHaveLength(8);
  const anchors = book.noteAnchors as Array<{ region: unknown; timestamp: number }>;
  for (let index = 1; index < anchors.length; index++) {
    expect(anchors[index].region).toEqual(anchors[0].region);
  }
  await page.reload();
  const reopened = await command({ type: "scoreAttachments", content_id: song.contentId });
  expect(reopened.attachments.find((item: { id: string }) => item.id === book.id).noteAnchors).toHaveLength(8);
  const seek = await command({ type: "seekPaperNote", content_id: song.contentId, book: book.id, expected: anchors[5] });
  expect(seek).toBeTruthy();
  const state = await (await request.get("http://127.0.0.1:32124/api/state")).json();
  expect(state.position).toBeCloseTo(song.notes[5].start, 3);
});
