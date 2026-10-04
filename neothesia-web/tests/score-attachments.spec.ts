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
test("真实 PDF 多页显示、图片追加重排与曲目阅读位置持久保存", async ({
  page,
  request,
}) => {
  test.setTimeout(90000);
  test.skip(!process.env.NEOTHESIA_PAPER_TEST_DATA, "需要独立谱面测试服务");
  const command = async (value: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data: value,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const rows = await (
    await request.get("http://127.0.0.1:32124/api/library")
  ).json();
  const row = rows.songs.find((r: any) => r.path.endsWith("five-finger.mid"));
  const song = await command({
    type: "load",
    path: row.path,
    title: "谱面浏览练习",
  });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "谱面管理", exact: true }).click();
  await page
    .getByRole("button", { name: "PDF / 图片谱面", exact: true })
    .click();
  const panel = page.getByRole("region", { name: "PDF 与图片谱面管理" });
  const input = panel.getByLabel("添加 PDF 或图片谱面");
  await input.setInputFiles({
    name: "老师的双页谱.pdf",
    mimeType: "application/pdf",
    buffer: pdf(),
  });
  await expect(
    panel.getByRole("status").filter({ hasText: "已添加" }),
  ).toBeVisible();
  await expect(panel.getByLabel("谱页页码")).toHaveAttribute("max", "2");
  await expect(panel.locator("canvas")).toBeVisible();
  await expect
    .poll(() => panel.locator("canvas").evaluate((c: any) => c.width))
    .toBeGreaterThan(300);
  await panel.getByRole("button", { name: "下一谱页" }).click();
  await expect(panel.getByLabel("谱页页码")).toHaveValue("2");
  await panel.getByLabel("谱页缩放").selectOption("150");
  await panel.getByRole("button", { name: "旋转 90°", exact: true }).click();
  await expect
    .poll(async () => {
      const a = await command({
        type: "scoreAttachments",
        content_id: song.contentId,
      });
      return a.attachments[0].view;
    })
    .toMatchObject({ page: 2, zoom: 150, rotation: 90, fit: false });
  await panel.getByLabel("附件谱面名称").fill("课堂谱 · 双页");
  await panel.getByRole("button", { name: "保存名称", exact: true }).click();
  await expect(panel.getByLabel("PDF 与图片谱面版本")).toContainText(
    "课堂谱 · 双页",
  );
  const image = async (color: string) =>
    Buffer.from(
      await page.evaluate((color) => {
        const c = document.createElement("canvas");
        c.width = 600;
        c.height = 800;
        const x = c.getContext("2d")!;
        x.fillStyle = "white";
        x.fillRect(0, 0, 600, 800);
        x.strokeStyle = "black";
        for (let y = 230; y <= 270; y += 10) {
          x.beginPath();
          x.moveTo(40, y);
          x.lineTo(550, y);
          x.stroke();
        }
        x.fillStyle = color;
        x.beginPath();
        x.ellipse(200, 250, 8, 5, -0.3, 0, Math.PI * 2);
        x.fill();
        return c.toDataURL("image/png").split(",")[1];
      }, color),
      "base64",
    );
  await input.setInputFiles([
    {
      name: "扫描一.png",
      mimeType: "image/png",
      buffer: await image("#b30000"),
    },
    {
      name: "扫描二.png",
      mimeType: "image/png",
      buffer: await image("#0033b3"),
    },
  ]);
  await expect(panel.getByLabel("PDF 与图片谱面版本")).toContainText(
    "2 页图片",
  );
  await expect(panel.getByLabel("谱页页码")).toHaveAttribute("max", "2");
  await panel.getByRole("button", { name: "下一谱页" }).click();
  await expect(
    panel.getByRole("button", { name: "此页前移", exact: true }),
  ).toBeEnabled();
  await panel.getByRole("button", { name: "此页前移", exact: true }).click();
  await expect(panel.getByLabel("谱页页码")).toHaveValue("1");
  await expect
    .poll(async () => {
      const a = await command({
        type: "scoreAttachments",
        content_id: song.contentId,
      });
      return a.attachments.find((x: any) => x.format === "images").pages[0]
        .name;
    })
    .toBe("扫描二.png");
  await page.screenshot({
    path: "../outputs/Neothesia-谱面附件.png",
    fullPage: true,
  });
  await page.getByRole("button", { name: "完成", exact: true }).click();
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  const viewer = page.getByRole("region", { name: "谱页浏览" });
  await expect(viewer.getByLabel("PDF 与图片谱面版本")).toContainText(
    "2 页图片",
  );
  const data = await command({
    type: "scoreAttachments",
    content_id: song.contentId,
  });
  const pdfId = data.attachments.find((a: any) => a.format === "pdf").id;
  await viewer.getByLabel("PDF 与图片谱面版本").selectOption(pdfId);
  await expect(viewer.getByLabel("谱页页码")).toHaveValue("2");
  await expect(viewer.getByLabel("谱页缩放")).toHaveValue("150");
  await expect(viewer.locator("canvas")).toBeVisible();
  await page.reload();
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  await expect(page.getByLabel("谱页页码")).toHaveValue("2");
  await expect(page.getByLabel("谱页缩放")).toHaveValue("150");
  const canvas = page.locator(".paper-reader canvas");
  await expect
    .poll(() => canvas.evaluate((c: any) => c.width))
    .toBeGreaterThan(300);
  await page.getByRole("button", { name: "适合宽度", exact: true }).click();
  for (let i = 0; i < 3; i++)
    await page.getByRole("button", { name: "旋转 90°", exact: true }).click();
  await expect
    .poll(() =>
      canvas.evaluate((c: any) => {
        const a = c.getContext("2d").getImageData(0, 0, c.width, c.height).data;
        let dark = 0;
        for (let i = 0; i < a.length; i += 4)
          if (a[i] < 180 && a[i + 3] > 200) dark++;
        return dark;
      }),
    )
    .toBeGreaterThan(50);
  await page.screenshot({
    path: "../outputs/Neothesia-PDF练习谱页.png",
    fullPage: true,
  });
});
