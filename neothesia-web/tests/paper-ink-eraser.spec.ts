import { test, expect } from "@playwright/test";
import { paperPoint } from "../src/PaperGeometry";
import { eraseInk } from "../src/paperInkEditing";
import type { PaperAnnotation } from "../src/PaperAnnotations";
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

const line = (id = "line"): PaperAnnotation => ({
  id,
  assetId: "asset",
  page: 1,
  x: 0.15,
  y: 0.3,
  width: 0.7,
  height: 0.002,
  text: "保持连奏",
  color: "blue",
  ink: {
    points: [
      [0, 0.5],
      [1, 0.5],
    ],
    thickness: 0.0022,
  },
});
test("局部橡皮切开长线段，保留未命中笔迹并拒绝容量溢出", () => {
  const n = line(),
    untouched = { ...line("untouched"), y: 0.7 };
  const output = eraseInk(
    [n, untouched],
    [0.5, 0.25],
    [0.5, 0.36],
    600,
    800,
    8,
    "partial",
  );
  expect(output).toHaveLength(3);
  expect(output[2]).toBe(untouched);
  expect(output[0].x + output[0].width!).toBeLessThan(0.5);
  expect(output[1].x).toBeGreaterThan(0.5);
  expect(output[0].text).toBe(n.text);
  expect(output[0].color).toBe(n.color);
  expect(output[0].ink!.thickness).toBe(n.ink!.thickness);
  expect(new Set(output.map((n) => n.id)).size).toBe(3);
  expect(output.some((n) => n.id === "line")).toBeFalsy();
  for (const row of output)
    expect(row.ink!.points.length).toBeGreaterThanOrEqual(2);
  expect(
    eraseInk([n], [0.5, 0.25], [0.5, 0.36], 600, 800, 8, "whole"),
  ).toHaveLength(0);
  expect(eraseInk([n], [0, 0], [0.1, 0.1], 600, 800, 8, "partial")[0]).toBe(n);
  expect(() =>
    eraseInk([n], [0.5, 0.25], [0.5, 0.36], 600, 800, 8, "partial", 3),
  ).toThrow("20 万点");
  const crowded = Array.from({ length: 2000 }, (_, i) =>
    i === 0 ? n : { ...untouched, id: `other-${i}` },
  );
  expect(() =>
    eraseInk(crowded, [0.5, 0.25], [0.5, 0.36], 600, 800, 8, "partial"),
  ).toThrow("2000");
  expect(() =>
    eraseInk(
      [n, untouched],
      [0.5, 0.25],
      [0.5, 0.36],
      600,
      800,
      8,
      "partial",
      200000,
      2,
    ),
  ).toThrow("整份谱面 2000 条");
  expect(n.ink!.points).toEqual([
    [0, 0.5],
    [1, 0.5],
  ]);
  const dot = {
    ...line(),
    width: 0.002,
    ink: {
      points: [
        [0.5, 0.5],
        [0.5, 0.5],
      ] as [number, number][],
      thickness: 0.0022,
    },
  };
  expect(
    eraseInk([dot], [0.151, 0.301], [0.151, 0.301], 600, 800, 8, "partial"),
  ).toHaveLength(0);
});

test("局部擦除四向旋转、一次撤销、取消、草稿重开及真实保存", async ({
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

  const original = [
    { ...line(`ink-cut-${Date.now()}`), assetId },
    { ...line(`ink-keep-${Date.now()}`), assetId, y: 0.6 },
  ];
  const saved = async () =>
    (
      await cmd({ type: "scoreAttachments", content_id: song.contentId })
    ).attachments.find((a: any) => a.id === papers.active).annotations ?? [];
  const layer = panel.getByLabel("谱页手写笔迹"),
    paths = layer.locator("path[data-ink-id]");
  for (const rotation of [0, 90, 180, 270]) {
    await cmd({
      type: "editPaperInk",
      content_id: song.contentId,
      book: papers.active,
      asset_id: assetId,
      page: 1,
      expected: await saved(),
      strokes: original,
    });
    const baseline = await saved();
    await page.reload();
    await page.getByRole("button", { name: "谱页", exact: true }).click();
    if (rotation !== 0)
      await panel
        .getByRole("button", { name: "旋转 90°", exact: true })
        .click();
    await panel
      .getByLabel("谱页缩放", { exact: true })
      .selectOption(rotation === 180 ? "100" : "75");
    await panel.getByRole("button", { name: "手写标记", exact: true }).click();
    await panel.getByRole("button", { name: "橡皮", exact: true }).click();
    await panel.getByLabel("笔迹擦除方式").selectOption("partial");
    // Center the hit point in the remaining paper viewport, below the sticky tools.
    const sweep = async (cancel = false) => {
      const p = paperPoint(0.5, 0.301, rotation);
      await layer.evaluate((svg, p) => {
        const frame = svg.closest(".paper-sheet-scroll")!,
          box = svg.getBoundingClientRect(),
          f = frame.getBoundingClientRect();
        frame.scrollTop += box.y + box.height * p[1] - (f.y + f.height * 0.65);
        frame.scrollLeft += box.x + box.width * p[0] - (f.x + f.width * 0.5);
      }, p);
      const b = (await layer.boundingBox())!;
      const a = paperPoint(0.5, 0.275, rotation),
        z = paperPoint(0.5, 0.327, rotation);
      await page.mouse.move(b.x + a[0] * b.width, b.y + a[1] * b.height);
      await page.mouse.down();
      await page.mouse.move(b.x + z[0] * b.width, b.y + z[1] * b.height, {
        steps: 3,
      });
      if (cancel) await page.keyboard.press("Escape");
      await page.mouse.up();
    };
    await sweep(true);
    await expect(paths).toHaveCount(2);
    await sweep();
    await expect(paths).toHaveCount(3);
    await panel.getByRole("button", { name: "撤销笔画", exact: true }).click();
    await expect(paths).toHaveCount(2);
    await expect(paths.first()).toHaveAttribute("data-ink-id", original[0].id);
    await panel.getByRole("button", { name: "重做笔画", exact: true }).click();
    await expect(paths).toHaveCount(3);
    await panel
      .getByRole("button", { name: "暂存并退出手写", exact: true })
      .click();
    await page.reload();
    await page.getByRole("button", { name: "谱页", exact: true }).click();
    await expect(paths).toHaveCount(3);
    await panel.getByRole("button", { name: "保存笔迹", exact: true }).click();
    await expect(
      panel.getByRole("button", { name: "手写标记", exact: true }),
    ).toBeVisible();
    const after = await saved();
    expect(after).toHaveLength(3);
    expect(after.find((n: any) => n.id === original[1].id)).toEqual(
      baseline.find((n: any) => n.id === original[1].id),
    );
    expect(after.some((n: any) => n.id === original[0].id)).toBeFalsy();
    const parts = after
      .filter((n: any) => n.id !== original[1].id)
      .sort((a: any, b: any) => a.x - b.x);
    expect(parts[0].x + parts[0].width).toBeLessThan(0.5);
    expect(parts[1].x).toBeGreaterThan(0.5);
    expect(
      parts.every((n: any) => n.text === "保持连奏" && n.color === "blue"),
    ).toBeTruthy();
  }
  await panel.getByRole("button", { name: "手写标记", exact: true }).click();
  await panel.getByRole("button", { name: "橡皮", exact: true }).click();
  await panel.getByLabel("笔迹擦除方式").selectOption("partial");
  await paths.first().scrollIntoViewIfNeeded();
  await layer.evaluate((svg) => {
    const frame = svg.closest(".paper-sheet-scroll")!,
      box = svg.getBoundingClientRect();
    const tools = frame.querySelector(".ink-editing")!.getBoundingClientRect(),
      bounds = frame.getBoundingClientRect();
    frame.scrollTop +=
      box.y +
      box.height * 0.5 -
      (tools.bottom + (bounds.bottom - tools.bottom) / 2);
  });
  await page.screenshot({ path: "../outputs/Neothesia-局部橡皮.png" });
  await panel
    .getByRole("button", { name: "暂存并退出手写", exact: true })
    .click();
});
