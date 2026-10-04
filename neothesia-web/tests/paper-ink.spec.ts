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
test("手写笔迹、整笔擦除、撤销、草稿、旋转与导出", async ({
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
  await panel.getByRole("button", { name: "手写标记", exact: true }).click();
  const layer = panel.getByLabel("谱页手写笔迹"),
    paths = layer.locator("path[data-ink-id]");
  const draw = async (points: [number, number][]) => {
    const b = (await layer.boundingBox())!;
    await page.mouse.move(
      b.x + b.width * points[0][0],
      b.y + b.height * points[0][1],
    );
    await page.mouse.down();
    for (const p of points.slice(1))
      await page.mouse.move(b.x + b.width * p[0], b.y + b.height * p[1], {
        steps: 5,
      });
    await page.mouse.up();
  };
  await draw([
    [0.12, 0.06],
    [0.2, 0.09],
    [0.27, 0.06],
    [0.35, 0.09],
  ]);
  await expect(paths).toHaveCount(1);
  await panel.getByLabel("笔迹颜色").selectOption("blue");
  await panel.getByLabel("笔迹粗细").selectOption("0.004");
  await draw([
    [0.45, 0.06],
    [0.6, 0.12],
  ]);
  await expect(paths).toHaveCount(2);
  await panel.getByRole("button", { name: "撤销笔画", exact: true }).click();
  await expect(paths).toHaveCount(1);
  await panel.getByRole("button", { name: "重做笔画", exact: true }).click();
  await expect(paths).toHaveCount(2);
  await panel.getByRole("button", { name: "橡皮", exact: true }).click();
  const b = (await layer.boundingBox())!;
  await page.mouse.click(b.x + b.width * 0.12, b.y + b.height * 0.06);
  await expect(paths).toHaveCount(1);
  await panel.getByRole("button", { name: "撤销笔画", exact: true }).click();
  await expect(paths).toHaveCount(2);
  await panel
    .getByRole("button", { name: "暂存并退出手写", exact: true })
    .click();
  await expect(paths).toHaveCount(0);
  await page.reload();
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  await expect(paths).toHaveCount(2);
  await panel.getByRole("button", { name: "保存笔迹", exact: true }).click();
  await expect(
    panel.getByRole("button", { name: "手写标记", exact: true }),
  ).toBeVisible();
  await expect(paths).toHaveCount(2);
  const stored = await cmd({
    type: "scoreAttachments",
    content_id: song.contentId,
  });
  const notes = stored.attachments.find(
    (a: any) => a.id === papers.active,
  ).annotations;
  expect(notes).toHaveLength(2);
  expect(notes[1].ink.thickness).toBeCloseTo(0.004, 5);
  expect(notes[0].ink.points.length).toBeGreaterThan(10);
  await panel
    .getByRole("button", { name: "撤销上次笔迹保存", exact: true })
    .click();
  await expect(paths).toHaveCount(0);
  await panel
    .getByRole("button", { name: "重做笔迹保存", exact: true })
    .click();
  await expect(paths).toHaveCount(2);
  const before = await paths.first().getAttribute("d");
  await panel.getByRole("button", { name: "旋转 90°", exact: true }).click();
  await expect.poll(() => paths.first().getAttribute("d")).not.toBe(before);
  await page.screenshot({ path: "../outputs/Neothesia-纸谱手写标记.png" });
  await page.setViewportSize({ width: 700, height: 1000 });
  await page.screenshot({ path: "../outputs/Neothesia-纸谱手写标记-窄窗.png" });
  const download = page.waitForEvent("download");
  await panel
    .getByRole("button", { name: "导出本页批注", exact: true })
    .click();
  const file = await download;
  expect(
    (await readFile((await file.path())!)).readUInt32BE(16),
  ).toBeGreaterThan(1000);
  await file.saveAs("../outputs/Neothesia-纸谱手写导出.png");
  await page.reload();
  await page.getByRole("button", { name: "谱页", exact: true }).click();
  await expect(paths).toHaveCount(2);
  // A stale page-level ink baseline cannot replace a later edit, and keeps the local draft.
  await panel.getByRole("button", { name: "手写标记", exact: true }).click();
  await draw([
    [0.12, 0.06],
    [0.2, 0.09],
  ]);
  await expect(paths).toHaveCount(3);
  const current = (
    await cmd({ type: "scoreAttachments", content_id: song.contentId })
  ).attachments.find((a: any) => a.id === papers.active).annotations;
  const changed = structuredClone(current);
  changed[0].color = "green";
  await cmd({
    type: "editPaperInk",
    content_id: song.contentId,
    book: papers.active,
    asset_id: notes[0].assetId,
    page: 1,
    expected: current,
    strokes: changed,
  });
  await panel.getByRole("button", { name: "保存笔迹", exact: true }).click();
  await expect(
    panel.getByText("笔迹未保存，草稿保留。", { exact: false }),
  ).toBeVisible();
  await panel
    .getByRole("button", { name: "核对已保存笔迹", exact: true })
    .click();
  const review = page.getByRole("dialog", { name: "核对笔迹更改" });
  await expect(review).toContainText("没有同笔冲突");
  await review.getByRole("button", { name: "合并到草稿", exact: true }).click();
  await expect(paths).toHaveCount(3);
  await expect(paths.first()).toHaveAttribute("stroke", "#35845d");
  await panel.getByRole("button", { name: "保存笔迹", exact: true }).click();
  await expect(
    panel.getByRole("button", { name: "手写标记", exact: true }),
  ).toBeVisible();
  await panel.getByRole("button", { name: "手写标记", exact: true }).click();
  await panel.getByRole("button", { name: "橡皮", exact: true }).click();
  const eraseBox = (await layer.boundingBox())!,
    a = changed[0],
    p0 = a.ink.points[0],
    origX = a.x + p0[0] * a.width,
    origY = a.y + p0[1] * a.height;
  await page.mouse.click(
    eraseBox.x + (1 - origY) * eraseBox.width,
    eraseBox.y + origX * eraseBox.height,
  );
  await expect(paths).toHaveCount(2);
  const beforeConflict = (
    await cmd({ type: "scoreAttachments", content_id: song.contentId })
  ).attachments.find((a: any) => a.id === papers.active).annotations;
  const later = structuredClone(beforeConflict);
  later[0].color = "blue";
  await cmd({
    type: "editPaperInk",
    content_id: song.contentId,
    book: papers.active,
    asset_id: notes[0].assetId,
    page: 1,
    expected: beforeConflict,
    strokes: later,
  });
  await panel.getByRole("button", { name: "保存笔迹", exact: true }).click();
  await panel
    .getByRole("button", { name: "核对已保存笔迹", exact: true })
    .click();
  await expect(review.getByLabel("保留已保存笔迹 1")).toBeChecked();
  await page.screenshot({ path: "../outputs/Neothesia-笔迹冲突审阅.png" });
  await review.getByLabel("采用本机草稿笔迹 1").check();
  await review.getByRole("button", { name: "合并到草稿", exact: true }).click();
  await expect(paths).toHaveCount(2);
  await panel.getByRole("button", { name: "保存笔迹", exact: true }).click();
  await expect(
    panel.getByRole("button", { name: "手写标记", exact: true }),
  ).toBeVisible();
  const final = (
    await cmd({ type: "scoreAttachments", content_id: song.contentId })
  ).attachments.find((a: any) => a.id === papers.active).annotations;
  expect(final).toHaveLength(2);
  expect(final.some((n: any) => n.id === later[0].id)).toBeFalsy();
});
