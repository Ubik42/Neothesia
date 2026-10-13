import { test, expect } from "@playwright/test";
import { paperPoint } from "../src/PaperGeometry";
import { moveInk } from "../src/paperInkEditing";
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

test("笔迹选择、四向旋转移动、样式、草稿与保存撤销", async ({
  page,
  request,
}) => {
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  await cmd({
    type: "generate",
    spec: {
      tonic: 8,
      tonality: "Major",
      minor_form: "Natural",
      pattern: "Scale",
      direction: "UpAndDown",
      hands: "Both",
      octaves: 1,
      repetitions: 1,
      tempo_bpm: 120,
    },
  });
  await cmd({ type: "meter", value: null });
  const song = await cmd({ type: "currentSong" });
  const papers = await cmd({
    type: "addScoreAttachment",
    content_id: song.contentId,
    name: "手写课堂.pdf",
    bytes: [...Buffer.concat([pdf(), Buffer.from("\n%" + Date.now())])],
  });
  await cmd({
    type: "updateScoreAttachment",
    content_id: song.contentId,
    id: papers.active,
    action: "view",
    view: { page: 1, zoom: 75, fit: false, rotation: 0 },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  await page.getByRole("button", { name: "隐藏曲库", exact: true }).click();
  await page.getByRole("button", { name: "隐藏练习参数", exact: true }).click();
  const panel = page.getByRole("region", { name: "谱页浏览" });
  await expect(panel.locator("canvas")).toBeVisible();

  const book = papers.attachments.find((a: any) => a.id === papers.active);
  const assetId = book.pages[0].id;
  const original = [0, 1].map((i) => ({
    id: `ink-edit-${Date.now()}-${i}`,
    assetId,
    page: 1,
    x: 0.2 + i * 0.22,
    y: 0.2,
    width: 0.12,
    height: 0.03,
    text: "",
    color: "red",
    ink: {
      points: [
        [0, 0],
        [1, 1],
      ],
      thickness: 0.0022,
    },
  }));
  await cmd({
    type: "editPaperInk",
    content_id: song.contentId,
    book: papers.active,
    asset_id: assetId,
    page: 1,
    expected: [],
    strokes: original,
  });
  await page.reload();
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  const layer = panel.getByLabel("谱页手写笔迹");
  const paths = layer.locator("path[data-ink-id]");
  const saved = async () =>
    (
      await cmd({ type: "scoreAttachments", content_id: song.contentId })
    ).attachments.find((a: any) => a.id === papers.active).annotations;
  for (const rotation of [0, 90, 180, 270]) {
    const before = await saved();
    await panel.getByRole("button", { name: "手写标记", exact: true }).click();
    await panel
      .getByRole("button", { name: "选择笔迹", exact: true })
      .first()
      .click();
    await paths.first().scrollIntoViewIfNeeded();
    const b = (await layer.boundingBox())!;
    const n = before[0];
    const p = paperPoint(n.x + n.width / 2, n.y + n.height / 2, rotation);
    await page.mouse.move(b.x + b.width * p[0], b.y + b.height * p[1]);
    await page.mouse.down();
    await page.mouse.move(
      b.x + b.width * (p[0] + 0.04),
      b.y + b.height * (p[1] + 0.02),
      { steps: 4 },
    );
    await page.mouse.up();
    await expect(layer.locator("[data-selected-ink-id]")).toHaveCount(1);
    await panel.getByLabel("所选笔迹颜色").selectOption("blue");
    await panel.getByLabel("所选笔迹粗细").selectOption("0.004");
    await panel.getByRole("button", { name: "保存笔迹", exact: true }).click();
    await expect(
      panel.getByRole("button", { name: "手写标记", exact: true }),
    ).toBeVisible();
    const after = await saved();
    const a = paperPoint(0.5, 0.5, (360 - rotation) % 360),
      z = paperPoint(0.54, 0.52, (360 - rotation) % 360);
    expect(after[0].x).toBeCloseTo(n.x + z[0] - a[0], 2);
    expect(after[0].y).toBeCloseTo(n.y + z[1] - a[1], 2);
    expect(after[0].color).toBe("blue");
    expect(after[0].ink.thickness).toBeCloseTo(0.004, 5);
    expect(after[0].ink.points).toEqual(original[0].ink.points);
    expect(after[1]).toEqual(before[1]);
    if (rotation !== 270)
      await panel
        .getByRole("button", { name: "旋转 90°", exact: true })
        .click();
  }
  await panel.getByRole("button", { name: "手写标记", exact: true }).click();
  await panel
    .getByRole("button", { name: "选择笔迹", exact: true })
    .first()
    .click();
  await panel.getByRole("button", { name: "全选本页笔迹" }).click();
  await expect(layer.locator("[data-selected-ink-id]")).toHaveCount(2);
  const beforeCancel = await paths.first().getAttribute("d");
  await paths.first().scrollIntoViewIfNeeded();
  const start = await paths.first().evaluate((node) => {
    const p = (node as SVGPathElement).getPointAtLength(0);
    const bounds = (
      node as SVGPathElement
    ).ownerSVGElement!.getBoundingClientRect();
    return { x: bounds.x + p.x, y: bounds.y + p.y };
  });
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(start.x + 20, start.y + 10, { steps: 3 });
  await page.keyboard.press("Escape");
  await page.mouse.up();
  await expect(paths.first()).toHaveAttribute("d", beforeCancel!);
  await layer.focus();
  const path = await paths.first().getAttribute("d");
  await page.keyboard.press("Shift+ArrowRight");
  await expect(paths.first()).not.toHaveAttribute("d", path!);
  await page.keyboard.press("Control+z");
  await expect(paths.first()).toHaveAttribute("d", path!);
  await page.keyboard.press("Control+y");
  await expect(paths.first()).not.toHaveAttribute("d", path!);
  await panel
    .getByRole("button", { name: "暂存并退出手写", exact: true })
    .click();
  await page.reload();
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  await expect(paths.first()).not.toHaveAttribute("d", path!);
  await panel.getByRole("button", { name: "保存笔迹", exact: true }).click();
  await expect(
    panel.getByRole("button", { name: "手写标记", exact: true }),
  ).toBeVisible();
  await panel
    .getByRole("button", { name: "撤销上次笔迹保存", exact: true })
    .click();
  await expect(paths.first()).toHaveAttribute("d", path!);
  await panel
    .getByRole("button", { name: "重做笔迹保存", exact: true })
    .click();
  await expect(paths.first()).not.toHaveAttribute("d", path!);
  await panel.getByRole("button", { name: "手写标记", exact: true }).click();
  await panel
    .getByRole("button", { name: "选择笔迹", exact: true })
    .first()
    .click();
  await panel.getByRole("button", { name: "全选本页笔迹" }).click();
  await panel.getByRole("button", { name: "删除所选笔迹" }).click();
  await expect(paths).toHaveCount(0);
  await panel.getByRole("button", { name: "撤销笔画", exact: true }).click();
  await expect(paths).toHaveCount(2);
  await panel.getByRole("button", { name: "全选本页笔迹" }).click();
  const positionSelection = async () => {
    await paths.first().scrollIntoViewIfNeeded();
    await layer.evaluate((svg) => {
      const frame = svg.closest(".paper-sheet-scroll")!;
      const tools = frame
        .querySelector(".ink-editing")!
        .getBoundingClientRect();
      const boxes = [...svg.querySelectorAll("[data-selected-ink-id]")].map(
        (n) => n.getBoundingClientRect(),
      );
      const top = Math.min(...boxes.map((r) => r.top));
      if (top < tools.bottom + 8) frame.scrollTop -= tools.bottom + 8 - top;
    });
    const bar = panel.getByRole("toolbar", { name: "笔迹常用工具" });
    await expect(bar).toBeVisible();
    const barBox = (await bar.boundingBox())!,
      frameBox = (await panel.getByLabel("谱页画布").boundingBox())!;
    expect(barBox.y).toBeGreaterThanOrEqual(frameBox.y);
    expect(barBox.y + barBox.height).toBeLessThan(frameBox.y + frameBox.height);
    expect(barBox.x + barBox.width).toBeLessThanOrEqual(
      page.viewportSize()!.width,
    );
  };
  await page.setViewportSize({ width: 1440, height: 1000 });
  await expect(
    panel.getByRole("button", { name: "全选本页笔迹" }),
  ).toBeEnabled();
  await positionSelection();
  await page.screenshot({ path: "../outputs/Neothesia-纸谱笔迹编辑.png" });
  await page.setViewportSize({ width: 760, height: 1000 });
  await expect(
    panel.getByRole("button", { name: "全选本页笔迹" }),
  ).toBeEnabled();
  await positionSelection();
  await page.screenshot({ path: "../outputs/Neothesia-纸谱笔迹编辑-窄窗.png" });
  await panel
    .getByRole("button", { name: "放弃笔迹修改", exact: true })
    .click();
});

test("整组笔迹移动到页边时保留相对位置与原始笔画", () => {
  const rows = [0.2, 0.6].map((x, i) => ({
    id: String(i),
    assetId: "a",
    page: 1,
    x,
    y: 0.3,
    width: 0.2,
    height: 0.1,
    text: "",
    color: "red",
    ink: {
      points: [
        [0, 0],
        [1, 1],
      ] as [number, number][],
      thickness: 0.0022,
    },
  }));
  const moved = moveInk(rows, ["0", "1"], 5, -5);
  expect(moved[1].x + moved[1].width!).toBeCloseTo(1);
  expect(moved[0].y).toBe(0);
  expect(moved[1].x - moved[0].x).toBeCloseTo(0.4);
  expect(moved[0].ink).toEqual(rows[0].ink);
  expect(rows[0].x).toBe(0.2);
});
