import { test, expect } from "@playwright/test";
import { readFile } from "node:fs/promises";
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
test("纸谱批注位置、旋转缩放、草稿、页隔离与持久化", async ({
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
  const library = await (
    await request.get("http://127.0.0.1:32124/api/library")
  ).json();
  const builtin = library.songs.find((s: any) =>
    s.path.endsWith("five-finger.mid"),
  );
  await cmd({ type: "load", path: builtin.path, title: "教师批注课堂" });
  const song = await cmd({ type: "currentSong" });
  const papers = await cmd({
    type: "addScoreAttachment",
    content_id: song.contentId,
    name: "教师批注.pdf",
    bytes: [...pdf()],
  });
  await cmd({
    type: "updateScoreAttachment",
    content_id: song.contentId,
    id: papers.active,
    action: "view",
    view: { page: 1, zoom: 100, fit: true, rotation: 0 },
  });
  const annotations =
    papers.attachments.find((a: any) => a.id === papers.active).annotations ??
    [];
  for (const n of annotations)
    await cmd({
      type: "editPaperAnnotation",
      content_id: song.contentId,
      book: papers.active,
      id: n.id,
      draft: null,
      expected: n,
    });
  await page.goto("/");
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  const panel = page.getByRole("region", { name: "谱页浏览" });
  await expect(panel.locator("canvas")).toBeVisible();
  await expect(
    panel.getByRole("button", { name: "添加批注", exact: true }),
  ).toBeEnabled();
  await panel.getByRole("button", { name: "添加批注", exact: true }).click();
  const sheet = panel.locator(".paper-annotation-sheet");
  await sheet.click({ position: { x: 200, y: 110 } });
  await panel.getByLabel("谱页批注内容").fill("这里换指 3→1，保持连奏");
  await panel.getByLabel("谱页批注颜色").selectOption("blue");
  await panel.getByRole("button", { name: "保存批注", exact: true }).click();
  const pin = panel.locator("button.paper-note-pin");
  await expect(pin).toHaveCount(1);
  const saved = await cmd({
    type: "scoreAttachments",
    content_id: song.contentId,
  });
  const note = saved.attachments.find((a: any) => a.id === papers.active)
    .annotations[0];
  expect(note.text).toContain("保持连奏");
  await panel.getByRole("button", { name: "旋转 90°", exact: true }).click();
  await expect
    .poll(async () =>
      Number.parseFloat(
        await pin.evaluate((el) => (el as HTMLElement).style.left),
      ),
    )
    .toBeCloseTo((1 - note.y) * 100, 3);
  await panel.getByLabel("谱页缩放").selectOption("50");
  await expect(pin).toHaveCount(1);
  await pin.click();
  await panel.getByLabel("谱页批注内容").fill("教师：先慢练，连接和弦后再加速");
  await expect(
    panel.getByRole("button", { name: "导出本页批注", exact: true }),
  ).toBeDisabled();
  await panel.getByRole("button", { name: "下一谱页" }).click();
  await expect(panel).toContainText("请先保存或取消当前批注");
  await page.reload();
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  await expect(panel.getByLabel("谱页批注内容")).toHaveValue(
    "教师：先慢练，连接和弦后再加速",
  );
  await panel.getByRole("button", { name: "保存批注", exact: true }).click();
  await expect(panel.getByLabel("谱页批注内容")).toHaveCount(0);
  await panel.getByRole("button", { name: "旋转 90°", exact: true }).click();
  await panel.getByLabel("谱页缩放").selectOption("50");
  await expect
    .poll(async () =>
      Number.parseFloat(
        await pin.evaluate((el) => (el as HTMLElement).style.left),
      ),
    )
    .toBeCloseTo((1 - note.y) * 100, 3);
  await panel.getByRole("button", { name: /^隐藏批注/ }).click();
  await expect(pin).toHaveCount(0);
  const downloaded = page.waitForEvent("download");
  await panel
    .getByRole("button", { name: "导出本页批注", exact: true })
    .click();
  const file = await downloaded;
  const image = await readFile((await file.path())!);
  expect(image.readUInt32BE(16)).toBe(1900);
  await file.saveAs("../outputs/Neothesia-纸谱批注导出示例.png");
  const pixel = await page.evaluate(
    async ({ base64, x, y }) => {
      const img = new Image();
      img.src = `data:image/png;base64,${base64}`;
      await img.decode();
      const canvas = document.createElement("canvas");
      canvas.width = img.width;
      canvas.height = img.height;
      const ctx = canvas.getContext("2d")!;
      ctx.drawImage(img, 0, 0);
      return Array.from(
        ctx.getImageData(Math.round(x), Math.round(y), 1, 1).data,
      );
    },
    {
      base64: image.toString("base64"),
      x: 50 + (1 - note.y) * 1800 + 11,
      y: 120 + note.x * 1350 + 11,
    },
  );
  expect(pixel.slice(0, 3)).toEqual([54, 121, 185]);
  await expect(panel).toContainText("已导出本页图片");
  await panel.getByRole("button", { name: /^显示批注/ }).click();
  await panel.getByRole("button", { name: "下一谱页" }).click();
  await expect(panel.locator("button.paper-note-pin")).toHaveCount(0);
  await panel.getByRole("button", { name: "上一谱页" }).click();
  await expect(pin).toHaveCount(1);
  await pin.click();
  await expect(panel.getByLabel("谱页批注内容")).toHaveValue(
    "教师：先慢练，连接和弦后再加速",
  );
  await page.screenshot({
    path: "../outputs/Neothesia-纸谱教师批注.png",
    fullPage: true,
  });
  await panel.getByRole("button", { name: "删除此批注", exact: true }).click();
  await expect(pin).toHaveCount(0);
});

test("图片谱页导出保留边缘标记与完整长批注", async ({ page, request }) => {
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  await page.goto("/");
  const bytes = await page.evaluate(async () => {
    const canvas = document.createElement("canvas");
    canvas.width = 320;
    canvas.height = 480;
    const ctx = canvas.getContext("2d")!;
    ctx.fillStyle = "#fff";
    ctx.fillRect(0, 0, 320, 480);
    ctx.fillStyle = "#111";
    ctx.fillRect(30, 90, 260, 3);
    const blob = await new Promise<Blob>((resolve) =>
      canvas.toBlob((b) => resolve(b!), "image/png"),
    );
    return Array.from(new Uint8Array(await blob.arrayBuffer()));
  });
  const song = await cmd({ type: "currentSong" });
  const papers = await cmd({
    type: "addScoreAttachment",
    content_id: song.contentId,
    name: "图片指法课堂.png",
    bytes,
  });
  const book = papers.attachments.find((a: any) => a.id === papers.active);
  for (const note of book.annotations ?? [])
    await cmd({
      type: "editPaperAnnotation",
      content_id: song.contentId,
      book: book.id,
      id: note.id,
      draft: null,
      expected: note,
    });
  const text =
    "教师提示：" +
    "连奏换指后保持手腕自然。".repeat(25) +
    "\n最后一行必须完整保留。";
  await cmd({
    type: "editPaperAnnotation",
    content_id: song.contentId,
    book: book.id,
    id: null,
    draft: {
      assetId: book.pages[0].id,
      page: 1,
      x: 1,
      y: 1,
      text,
      color: "green",
    },
    expected: null,
  });
  await page.reload();
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  const panel = page.getByRole("region", { name: "谱页浏览" });
  await expect(panel.locator("button.paper-note-pin")).toHaveCount(1);
  await page.evaluate(() => {
    const data = window as any;
    data.exportText = [];
    data.originalFillText = CanvasRenderingContext2D.prototype.fillText;
    CanvasRenderingContext2D.prototype.fillText = function (
      text,
      x,
      y,
      ...args
    ) {
      data.exportText.push(text);
      return data.originalFillText.call(this, text, x, y, ...args);
    };
  });
  const downloaded = page.waitForEvent("download");
  await panel
    .getByRole("button", { name: "导出本页批注", exact: true })
    .click();
  const file = await downloaded,
    output = await readFile((await file.path())!);
  expect(output.readUInt32BE(16)).toBe(1200);
  const drawn = await page.evaluate(() => {
    const data = window as any;
    CanvasRenderingContext2D.prototype.fillText = data.originalFillText;
    return data.exportText as string[];
  });
  expect(drawn.join("")).toContain(text.replace(/\n/g, ""));
  await expect(panel.locator("button.paper-note-pin")).toHaveCount(1);
});
