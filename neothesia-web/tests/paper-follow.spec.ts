import { test, expect } from "@playwright/test";
function pdf() {
  const body = (n: number) =>
    `BT /F1 24 Tf 50 750 Td (Piano Study - Page ${n}) Tj ET\n0.5 w\n50 650 m 550 650 l S\n50 660 m 550 660 l S\n50 670 m 550 670 l S\n50 680 m 550 680 l S\n50 690 m 550 690 l S\n`;
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << /Font << /F1 7 0 R >> >> /Contents 4 0 R >>",
    `<< /Length ${body(1).length} >>\nstream\n${body(1)}endstream`,
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << /Font << /F1 7 0 R >> >> /Contents 6 0 R >>",
    `<< /Length ${body(2).length} >>\nstream\n${body(2)}endstream`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ];
  let s = "%PDF-1.4\n";
  const offsets = [0];
  objects.forEach((o, i) => {
    offsets.push(s.length);
    s += `${i + 1} 0 obj\n${o}\nendobj\n`;
  });
  const at = s.length;
  s += "xref\n0 8\n0000000000 65535 f \n";
  for (const n of offsets.slice(1))
    s += `${n.toString().padStart(10, "0")} 00000 n \n`;
  s += `trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n${at}\n%%EOF\n`;
  return Buffer.from(s);
}
test("纸谱小节对应、实际自动翻页、手动暂停与拍号校正", async ({
  page,
  request,
}) => {
  test.setTimeout(60000);
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
  const measures = Array.from(
    { length: 4 },
    (_, i) =>
      `<measure number="${i + 1}">${i === 0 ? "<attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>" : ""}<note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note></measure>`,
  ).join("");
  const xml = `<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">${measures}</part></score-partwise>`;
  await cmd({
    type: "importScore",
    name: "纸谱翻页课堂.musicxml",
    bytes: [...Buffer.from(xml)],
    default_bpm: 120,
  });
  const song = await cmd({ type: "currentSong" });
  await cmd({ type: "meter", value: null });
  const attachment = await cmd({
    type: "addScoreAttachment",
    content_id: song.contentId,
    name: "双页练习谱.pdf",
    bytes: [...pdf()],
  });
  await cmd({
    type: "setPaperMapping",
    content_id: song.contentId,
    id: attachment.active,
    mapping: null,
  });
  await page.goto("/");
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  const panel = page.getByRole("region", { name: "谱页浏览" });
  await expect(panel.getByLabel("谱页页码")).toHaveAttribute("max", "2");
  await expect(panel.locator("canvas")).toBeVisible();
  await panel.locator(".paper-mapping-editor summary").click();
  for (const [measure, p] of [
    [1, 1],
    [2, 2],
    [3, 1],
    [4, 2],
  ]) {
    await panel.getByLabel("谱页页码").fill(String(p));
    await panel.getByLabel("谱页对应小节").fill(String(measure));
    await panel
      .getByRole("button", { name: "保存此页对应", exact: true })
      .click();
    await expect(panel.locator(".paper-anchor-row")).toHaveCount(measure);
  }
  await panel.getByLabel("纸谱自动翻页").check();
  await cmd({ type: "seekMeasure", measure: 1 });
  await expect(panel.getByLabel("谱页页码")).toHaveValue("1");
  await cmd({ type: "mode", value: "listen" });
  await cmd({ type: "countIn", bars: 0 });
  await cmd({ type: "speed", value: 2 });
  await cmd({ type: "play" });
  await expect(panel.getByLabel("谱页页码")).toHaveValue("2", {
    timeout: 6000,
  });
  await cmd({ type: "pause" });
  await panel.getByRole("button", { name: "上一谱页" }).click();
  await expect(panel).toContainText("跟随暂停");
  await cmd({ type: "seekMeasure", measure: 4 });
  await expect(panel.getByLabel("谱页页码")).toHaveValue("1");
  await panel.getByRole("button", { name: "继续跟随", exact: true }).click();
  await expect(panel.getByLabel("谱页页码")).toHaveValue("2");
  await cmd({ type: "seekMeasure", measure: 3 });
  await expect(panel.getByLabel("谱页页码")).toHaveValue("1");
  await page.screenshot({
    path: "../outputs/Neothesia-纸谱小节对应与跟随.png",
    fullPage: true,
  });
  const before = await cmd({
    type: "scoreAttachments",
    content_id: song.contentId,
  });
  expect(before.attachments[0].mapping.anchors).toHaveLength(4);
  await page.getByRole("button", { name: "修正小节网格", exact: true }).click();
  await page.getByLabel("网格每小节拍数").fill("3");
  await page.getByRole("button", { name: "保存小节网格", exact: true }).click();
  await expect(panel).toContainText("小节位置已改变");
  await expect(panel.getByLabel("纸谱自动翻页")).toBeDisabled();
  const oldMapping = await request.post("http://127.0.0.1:32124/api/command", {
    headers: { "X-Neothesia-Client": "web" },
    data: {
      type: "setPaperMapping",
      content_id: song.contentId,
      id: before.active,
      mapping: before.attachments[0].mapping,
    },
  });
  expect(oldMapping.ok()).toBeFalsy();
  await panel.locator(".paper-mapping-editor summary").click();
  await panel
    .getByRole("button", { name: "按当前小节重新确认", exact: true })
    .click();
  await expect(panel.getByLabel("纸谱自动翻页")).not.toBeChecked();
  await panel.getByLabel("纸谱自动翻页").check();
  await page.reload();
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  await expect(panel.getByLabel("纸谱自动翻页")).toBeChecked();
  await cmd({ type: "seekMeasure", measure: 2 });
  await expect(panel.getByLabel("谱页页码")).toHaveValue("2");
});
