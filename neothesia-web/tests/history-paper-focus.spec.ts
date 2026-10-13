import { expect, test } from "@playwright/test";

function twoPagePdf() {
  const body = (page: number) => `BT /F1 24 Tf 50 750 Td (Piano ${page}) Tj ET\n`;
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << /Font << /F1 7 0 R >> >> /Contents 4 0 R >>",
    `<< /Length ${body(1).length} >>\nstream\n${body(1)}endstream`,
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << /Font << /F1 7 0 R >> >> /Contents 6 0 R >>",
    `<< /Length ${body(2).length} >>\nstream\n${body(2)}endstream`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ];
  let source = "%PDF-1.4\n";
  const offsets = [0];
  objects.forEach((object, index) => {
    offsets.push(source.length);
    source += `${index + 1} 0 obj\n${object}\nendobj\n`;
  });
  const crossReference = source.length;
  source += "xref\n0 8\n0000000000 65535 f \n";
  for (const offset of offsets.slice(1))
    source += `${String(offset).padStart(10, "0")} 00000 n \n`;
  source += `trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n${crossReference}\n%%EOF\n`;
  return [...Buffer.from(source)];
}

test("历史问题音先恢复练习，再在自动翻页关闭时定位纸谱", async ({ page, request }) => {
  test.setTimeout(90_000);
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const state = async () => (await request.get("http://127.0.0.1:32124/api/state")).json();
  await command({ type: "stopRoutine" });
  await command({ type: "pause" });
  const xml = `<score-partwise><work><work-title>历史纸谱重练</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration></note></measure><measure number="2"><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration></note></measure><measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration></note></measure></part></score-partwise>`;
  await command({ type: "importScore", bytes: [...Buffer.from(xml)], name: "历史纸谱重练.musicxml", default_bpm: 120 });
  const song = await command({ type: "currentSong" });
  const paper = await command({ type: "addScoreAttachment", content_id: song.contentId, name: "两页练习谱.pdf", bytes: twoPagePdf() });
  const practice = await command({ type: "paperPracticeContext" });
  await command({
    type: "setPaperMapping", content_id: song.contentId, id: paper.active,
    mapping: { grid: practice.grid, enabled: false, anchors: [{ measure: 1, page: 1 }, { measure: 2, page: 2, region: { x: 0.2, y: 0.3, width: 0.25, height: 0.12 } }] },
  });
  await command({ type: "restart" });
  await command({ type: "track", id: song.notes[0].track, mode: "human", part: "right", visible: true });
  await command({ type: "countIn", bars: 0 });
  await command({ type: "mode", value: "recital" });
  await command({ type: "play" });
  await expect.poll(async () => (await state()).status).toBe("finished");

  await page.goto("/");
  await page.getByRole("button", { name: "关闭练习反馈" }).click();
  await page.getByRole("button", { name: "练习历史", exact: true }).click();
  await page.getByLabel("查找历史曲目").fill(song.title);
  await page.locator(".history-record").first().click();
  const performance = page.locator(".history-performance");
  await performance.locator("summary").click();
  await performance.getByRole("button", { name: "查看这次演奏", exact: true }).click();
  await expect(performance.locator(".history-performance-table tbody tr")).toHaveCount(3);
  await performance.getByLabel("重练前置小节").selectOption("1");
  await performance.getByLabel("重练后置小节").selectOption("0");
  await performance.getByRole("button", { name: "在纸谱中重练演奏第 2 小节参考音 2", exact: true }).click();

  const reader = page.getByRole("region", { name: "谱页浏览" });
  await expect(reader).toBeVisible();
  await expect(reader.getByLabel("谱页页码")).toHaveValue("2");
  await expect(reader.getByLabel("纸谱自动翻页")).not.toBeChecked();
  await expect(reader.locator(".paper-history-result")).toContainText("已定位第 2 小节的参考音 D4：第 2 页");
  await expect(reader.locator(".paper-history-result")).toContainText("不是单个音符的精确位置");
  await expect(reader.getByRole("button", { name: "历史参考音 D4 所在第 2 小节区域" })).toHaveClass(/history-target/);
  await expect(reader.locator(".paper-history-location")).toContainText("目标音在第 2 小节；本次重练第 1–2 小节");
  const resumed = await state();
  expect(resumed.mode).toBe("flow");
  expect(resumed.passage).toBeTruthy();
  await page.screenshot({ path: "../outputs/Neothesia-历史纸谱重练.png" });

  const relaunch = async () => {
    await page.getByRole("button", { name: "练习历史", exact: true }).click();
    await page.getByLabel("查找历史曲目").fill(song.title);
    await page.locator(".history-record").first().click();
    const details = page.locator(".history-performance");
    await details.locator("summary").click();
    await details.getByRole("button", { name: "查看这次演奏", exact: true }).click();
    await details.getByRole("button", { name: "在纸谱中重练演奏第 2 小节参考音 2", exact: true }).click();
  };
  await reader.getByLabel("谱页页码").fill("1");
  await expect(reader.getByLabel("谱页页码")).toHaveValue("1");
  await page.getByRole("button", { name: "键盘", exact: true }).click();
  await command({
    type: "setPaperMapping", content_id: song.contentId, id: paper.active,
    mapping: { grid: practice.grid, enabled: false, anchors: [{ measure: 2, page: 3 }] },
  });
  await relaunch();
  await expect(reader.getByLabel("谱页页码")).toHaveValue("1");
  await expect(reader.locator(".paper-history-result")).toContainText("第 3 页不可用，纸谱没有自动定位");
  await expect(reader.locator(".paper-measure-region.history-target")).toHaveCount(0);

  await page.getByRole("button", { name: "键盘", exact: true }).click();
  await command({
    type: "setPaperMapping", content_id: song.contentId, id: paper.active,
    mapping: { grid: practice.grid, enabled: false, anchors: [{ measure: 1, page: 1, region: { x: 0.1, y: 0.1, width: 0.3, height: 0.1 } }] },
  });
  await relaunch();
  await expect(reader.getByLabel("谱页页码")).toHaveValue("1");
  await expect(reader.locator(".paper-history-result")).toContainText("目标小节没有独立的页内区域");
  await expect(reader.locator(".paper-measure-region.history-target")).toHaveCount(0);

  await page.getByRole("button", { name: "键盘", exact: true }).click();
  await command({
    type: "setPaperMapping", content_id: song.contentId, id: paper.active,
    mapping: { grid: practice.grid, enabled: false, anchors: [{ measure: 3, page: 2 }] },
  });
  await relaunch();
  await expect(reader.getByLabel("谱页页码")).toHaveValue("1");
  await expect(reader.locator(".paper-history-location")).toContainText("第 2 小节之前没有纸谱对应点");
  await expect(reader.locator(".paper-history-result")).toHaveCount(0);
  await expect(reader.locator(".paper-measure-region.history-target")).toHaveCount(0);
});
