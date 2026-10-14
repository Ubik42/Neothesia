import { test, expect } from "@playwright/test";

function largeValidPng(png: Buffer): Buffer {
  let offset = 8;
  while (offset + 12 <= png.length) {
    const size = png.readUInt32BE(offset);
    if (png.toString("ascii", offset + 4, offset + 8) === "IEND") break;
    offset += size + 12;
  }
  if (offset + 12 !== png.length) throw new Error("谱页 PNG 结束块无效");
  const chunkData = Buffer.concat([Buffer.from("Comment\0"), Buffer.alloc(3_100_000, 88)]);
  const nameAndData = Buffer.concat([Buffer.from("tEXt"), chunkData]);
  let crc = 0xffffffff;
  for (const byte of nameAndData) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
  }
  const chunk = Buffer.allocUnsafe(nameAndData.length + 8);
  chunk.writeUInt32BE(chunkData.length, 0);
  nameAndData.copy(chunk, 4);
  chunk.writeUInt32BE((crc ^ 0xffffffff) >>> 0, chunk.length - 4);
  return Buffer.concat([png.subarray(0, offset), chunk, png.subarray(offset)]);
}

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
  await page.addInitScript(() => {
    const originalFetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      if (String(input).includes("/api/located-paper/create") && init?.body instanceof Blob)
        (window as any).__locatedPaperUpload = init.body.arrayBuffer().then(bytes => Array.from(new Uint8Array(bytes)));
      return originalFetch(input, init);
    };
  });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  const generate = page.getByRole("button", { name: "生成可定位纸谱", exact: true });
  await expect(generate).toBeEnabled({ timeout: 30_000 });
  await generate.click();
  const dialog = page.getByRole("dialog", { name: "生成可定位纸谱" });
  await expect(dialog).toContainText("可定位 8 个演奏音符");
  const firstLocation = dialog.getByRole("checkbox", { name: /保留音轨 .* 的位置/ }).first();
  await firstLocation.uncheck();
  await expect(dialog).toContainText("本次保留 7 个");
  await firstLocation.check();
  await expect(dialog).toContainText("本次保留 8 个");
  await dialog.getByLabel("定位演奏小节").selectOption("2");
  await expect(dialog).toContainText("本节保留 4 / 4 个位置");
  await dialog.getByRole("button", { name: "跳过本节" }).click();
  await expect(dialog).toContainText("本次保留 4 个");
  await dialog.getByRole("button", { name: "保留本节" }).click();
  await expect(dialog).toContainText("本次保留 8 个");
  await dialog.getByLabel("定位演奏小节").selectOption("0");
  await dialog.getByRole("button", { name: /查看第 1 小节 .* 的位置/ }).first().click();
  await expect(dialog.locator(".located-paper-highlight")).toBeVisible();
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
  const target = song.notes[1];
  const expectedAnchor = book.noteAnchors.find((item: any) => item.track === target.track && item.index === target.index);
  const stale = await request.post("http://127.0.0.1:32124/api/command", {
    headers: { "X-Neothesia-Client": "web" },
    data: { type: "seekPaperNote", content_id: song.contentId, book: book.id,
      expected: { ...expectedAnchor, pitch: 99 } },
  });
  expect(stale.ok()).toBe(false);
  await dialog.getByRole("button", { name: "完成" }).click();
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  await page.locator(".paper-note-editor summary").click();
  await page.getByLabel("纸谱位置目标音").selectOption(`${target.track}:${target.index}`);
  await page.getByRole("button", { name: /从此音练习/ }).click();
  await expect(page.getByRole("status").filter({ hasText: "练习已定位到" })).toBeVisible();
  const state = await (await request.get("http://127.0.0.1:32124/api/state")).json();
  expect(state.position).toBeCloseTo(target.start, 3);
  const original = Buffer.from(await page.evaluate(async () => await (window as any).__locatedPaperUpload));
  const metaSize = original.readUInt32LE(4);
  const metadata = JSON.parse(original.toString("utf8", 8, 8 + metaSize));
  metadata.name = "大谱二进制传输验收";
  const pageSizeAt = 8 + metaSize;
  const pageSize = original.readUInt32LE(pageSizeAt);
  const png = largeValidPng(original.subarray(pageSizeAt + 4, pageSizeAt + 4 + pageSize));
  expect(png.length).toBeGreaterThan(3_000_000);
  const meta = Buffer.from(JSON.stringify(metadata));
  const prefix = Buffer.alloc(8);
  prefix.write("NLP1", 0);
  prefix.writeUInt32LE(meta.length, 4);
  const pageLength = Buffer.alloc(4);
  pageLength.writeUInt32LE(png.length);
  const response = await request.post("http://127.0.0.1:32124/api/located-paper/create", {
    headers: { "X-Neothesia-Client": "web", "Content-Type": "application/x-neothesia-located-paper" },
    data: Buffer.concat([prefix, meta, pageLength, png]),
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  const large = await command({ type: "scoreAttachments", content_id: song.contentId });
  expect(large.attachments.find((item: any) => item.name === metadata.name)?.pages[0].size).toBe(png.length);
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

test("长谱逐页核对并跳过整页后仍保存全部谱页", async ({ page, request }) => {
  test.setTimeout(120_000);
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const steps = ["C", "D", "E", "F", "G", "A", "B"];
  const measures = Array.from({ length: 72 }, (_, bar) => `<measure number="${bar + 1}">${bar === 0 ? '<attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes>' : ""}${Array.from({ length: 4 }, (_, beat) => `<note><pitch><step>${steps[(bar * 4 + beat) % steps.length]}</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note>`).join("")}</measure>`).join("");
  const xml = `<score-partwise><work><work-title>长谱逐页校对验收</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">${measures}</part></score-partwise>`;
  await command({ type: "importScore", bytes: [...Buffer.from(xml)], name: "长谱逐页校对验收.musicxml", default_bpm: 120 });
  const song = await command({ type: "currentSong" });
  await command({ type: "pairScore", content_id: song.contentId, name: "长谱逐页校对验收.musicxml", bytes: [...Buffer.from(xml)] });
  await page.goto("/");
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  const generate = page.getByRole("button", { name: "生成可定位纸谱", exact: true });
  await expect(generate).toBeEnabled({ timeout: 30_000 });
  await generate.click();
  const dialog = page.getByRole("dialog", { name: "生成可定位纸谱" });
  const summary = await dialog.getByRole("status").first().innerText();
  const totalPages = Number(summary.match(/共 (\d+) 页/)?.[1]);
  expect(totalPages).toBeGreaterThan(1);
  await dialog.getByRole("button", { name: "下一页" }).click();
  await expect(dialog.getByLabel("生成谱页预览 · 第 2 页")).toBeVisible();
  const secondPageNotes = await dialog.getByRole("checkbox", { name: /保留音轨 .* 的位置/ }).count();
  expect(secondPageNotes).toBeGreaterThan(0);
  const secondPageMeasure = Number((await dialog.getByRole("button", { name: /查看第 .* 小节 .* 的位置/ }).last().innerText()).match(/第 (\d+) 小节/)?.[1]);
  expect(secondPageMeasure).toBeGreaterThan(0);
  await dialog.getByLabel("预览页码").fill("1");
  await dialog.getByLabel("定位演奏小节").selectOption(String(secondPageMeasure));
  await expect(dialog).toContainText(`第 ${secondPageMeasure} 小节音符位置`);
  await dialog.getByRole("button", { name: "跳过本节" }).click();
  await expect(dialog).toContainText("本节保留 0 /");
  await dialog.getByRole("button", { name: "保留本节" }).click();
  await dialog.getByLabel("预览页码").fill("2");
  await dialog.getByLabel("定位演奏小节").selectOption("0");
  await dialog.getByRole("button", { name: /查看第 .* 小节 .* 的位置/ }).first().click();
  await expect(dialog.locator(".located-paper-highlight")).toBeVisible();
  await dialog.getByLabel("谱页预览缩放").selectOption("1100");
  await page.screenshot({ path: "../outputs/Neothesia-可定位纸谱逐页校对.png" });
  await dialog.getByRole("button", { name: "跳过本页" }).click();
  await expect(dialog.getByRole("checkbox", { name: /保留音轨 .* 的位置/ }).first()).not.toBeChecked();
  await dialog.getByRole("button", { name: "采用并保存" }).click();
  await expect(dialog).toContainText("已保存。纸谱管理中可以查看这份谱", { timeout: 30_000 });
  const saved = await command({ type: "scoreAttachments", content_id: song.contentId });
  const book = saved.attachments.find((item: any) => item.name.includes("可定位谱"));
  expect(book.pages).toHaveLength(totalPages);
  expect(book.noteAnchors.length).toBeGreaterThan(0);
  expect(book.noteAnchors.length).toBeLessThan(song.notes.length);
  expect(book.noteAnchors.every((item: any) => item.page !== 2)).toBe(true);
});
