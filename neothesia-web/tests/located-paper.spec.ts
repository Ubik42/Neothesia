import { test, expect } from "@playwright/test";

test("关联乐谱生成可定位纸谱并持久保存逐音位置", async ({ page, request }) => {
  test.setTimeout(120_000);
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const xml = `<score-partwise version="4.0"><work><work-title>纸谱逐音定位验收</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes>${["C", "D", "E", "F"].map(step => `<note><pitch><step>${step}</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note>`).join("")}</measure><measure number="2">${["G", "A", "B", "C"].map((step, index) => `<note><pitch><step>${step}</step><octave>${index === 3 ? 5 : 4}</octave></pitch><duration>1</duration><type>quarter</type></note>`).join("")}</measure></part></score-partwise>`;
  await command({ type: "importScore", bytes: [...Buffer.from(xml)], name: "纸谱逐音定位验收.musicxml", default_bpm: 120 });
  const song = await command({ type: "currentSong" });
  await command({ type: "pairScore", content_id: song.contentId, name: "纸谱逐音定位验收.musicxml", bytes: [...Buffer.from(xml)] });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  const generate = page.getByRole("button", { name: "生成可定位纸谱", exact: true });
  await expect(generate).toBeEnabled({ timeout: 30_000 });
  await generate.click();
  const dialog = page.getByRole("dialog", { name: "生成可定位纸谱" });
  await expect(dialog).toContainText("可定位 8 个演奏音符");
  await dialog.getByRole("button", { name: "采用并保存" }).click();
  await expect(dialog).toContainText("已保存。纸谱管理中可以查看这份谱", { timeout: 30_000 });
  const saved = await command({ type: "scoreAttachments", content_id: song.contentId });
  const book = saved.attachments.find((item: any) => item.name.includes("可定位谱"));
  expect(book?.format).toBe("images");
  expect(book.pages.length).toBeGreaterThan(0);
  expect(book.noteAnchors).toHaveLength(8);
  expect(book.mapping?.enabled).toBe(true);
  expect(book.mapping?.anchors.length).toBe(2);
  expect(book.noteAnchors.every((item: any) => item.page === 1 && item.region.width > 0)).toBe(true);
  await page.reload();
  const persisted = await command({ type: "scoreAttachments", content_id: song.contentId });
  expect(persisted.attachments.find((item: any) => item.id === book.id)?.noteAnchors).toHaveLength(8);
});

test("反复演奏音符共用对应的书面音符位置", async ({ page, request }) => {
  test.setTimeout(120_000);
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const xml = `<score-partwise><work><work-title>反复纸谱定位验收</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><barline location="left"><repeat direction="forward"/></barline><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note></measure><measure number="2"><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note><barline location="right"><repeat direction="backward"/></barline></measure><measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note></measure></part></score-partwise>`;
  await command({ type: "importScore", bytes: [...Buffer.from(xml)], name: "反复纸谱定位验收.musicxml", default_bpm: 120 });
  const song = await command({ type: "currentSong" });
  expect(song.notes).toHaveLength(5);
  await command({ type: "pairScore", content_id: song.contentId, name: "反复纸谱定位验收.musicxml", bytes: [...Buffer.from(xml)] });
  await page.goto("/");
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  const generate = page.getByRole("button", { name: "生成可定位纸谱", exact: true });
  await expect(generate).toBeEnabled({ timeout: 30_000 });
  await generate.click();
  const dialog = page.getByRole("dialog", { name: "生成可定位纸谱" });
  await expect(dialog).toContainText("可定位 5 个演奏音符");
  await dialog.getByRole("button", { name: "采用并保存" }).click();
  await expect(dialog).toContainText("已保存。纸谱管理中可以查看这份谱", { timeout: 30_000 });
  const saved = await command({ type: "scoreAttachments", content_id: song.contentId });
  const anchors = saved.attachments.find((item: any) => item.name.includes("可定位谱"))?.noteAnchors;
  expect(anchors).toHaveLength(5);
  expect(anchors[0].region).toEqual(anchors[2].region);
  expect(anchors[1].region).toEqual(anchors[3].region);
});
