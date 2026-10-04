import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import { importKind, imageBook, pairingCandidates } from "../src/scorePairing";
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

test("跨曲目批量配对：真实预览、人工改配、图片组页、独立失败与继续", async ({
  page,
  request,
}) => {
  test.setTimeout(90000);
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const prefix = `批量课堂${Date.now()}`,
    a = prefix + "甲",
    b = prefix + "乙";
  const originalFirst = readFileSync(
      "../neothesia-engine/assets/five-finger.mid",
    ),
    originalSecond = readFileSync(
      "../neothesia-engine/assets/basic-chords.mid",
    );
  const uniqueMidi = (source: Buffer) => {
    const text = Buffer.from(prefix),
      track = Buffer.concat([
        Buffer.from([0, 255, 1, text.length]),
        text,
        Buffer.from([0, 255, 47, 0]),
      ]);
    const original = Buffer.from(source);
    original.writeUInt16BE(original.readUInt16BE(10) + 1, 10);
    const chunk = Buffer.alloc(8);
    chunk.write("MTrk");
    chunk.writeUInt32BE(track.length, 4);
    return Buffer.concat([original, chunk, track]);
  };
  const first = uniqueMidi(originalFirst),
    second = uniqueMidi(originalSecond);
  const xml = readFileSync(
    "../neothesia-core/src/score_performance_test.musicxml",
  );
  const png = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAD0lEQVR4nGNgSDmBQMRxANtDEsHsiNYJAAAAAElFTkSuQmCC",
    "base64",
  );
  await page.goto("/");
  await page.getByRole("button", { name: "导入曲目", exact: true }).click();
  await page.getByRole("button", { name: "批量配对谱面", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "批量配对曲目与谱面" });
  await dialog.getByLabel("批量配对文件").setInputFiles([
    { name: a + ".mid", mimeType: "audio/midi", buffer: first },
    { name: b + ".mid", mimeType: "audio/midi", buffer: second },
    { name: a + ".musicxml", mimeType: "application/xml", buffer: xml },
    { name: b + ".pdf", mimeType: "application/pdf", buffer: pdf() },
    {
      name: a + "-page02.png",
      mimeType: "image/png",
      buffer: Buffer.from(
        "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAEUlEQVR4nGP4n3ICjhiI4wAAibEisVa1kVcAAAAASUVORK5CYII=",
        "base64",
      ),
    },
    { name: a + "-page01.png", mimeType: "image/png", buffer: png },
    {
      name: "损坏乐谱.xml",
      mimeType: "application/xml",
      buffer: Buffer.from("broken xml"),
    },
  ]);
  await dialog
    .getByRole("button", { name: "导入 MIDI 并读取配对候选" })
    .click();
  await expect(dialog).toContainText("已完成 2 / 7");
  const rows = dialog.locator("tbody tr");
  await expect(rows.nth(2)).toContainText(a);
  await expect(rows.nth(3)).toContainText(b);
  await rows.nth(3).getByRole("button", { name: "预览", exact: true }).click();
  const preview = dialog.getByRole("region", { name: "待导入谱面预览" });
  await expect(preview.locator("canvas")).toBeVisible();
  await expect(preview).toContainText("第 1 / 2 页");
  await preview.getByRole("button", { name: "下一页", exact: true }).click();
  await expect(preview).toContainText("第 2 / 2 页");
  await preview.getByRole("button", { name: "返回配对列表" }).click();
  await rows.nth(2).getByRole("button", { name: "预览", exact: true }).click();
  await expect(preview.locator("svg").first()).toBeVisible({ timeout: 20000 });
  await preview.getByRole("button", { name: "返回配对列表" }).click();
  // A manual target is re-inspected. Selecting a different performance and
  // then restoring it must not alter the current playing song.
  await rows.nth(2).getByRole("button", { name: "选择曲目" }).click();
  await dialog.getByLabel("搜索配对曲目").fill(b);
  await dialog
    .locator(".batch-target-list button")
    .filter({ has: page.getByText(b, { exact: true }) })
    .click();
  await expect(rows.nth(2)).toContainText("已人工指定");
  await rows.nth(2).getByRole("button", { name: "选择曲目" }).click();
  await dialog.getByLabel("搜索配对曲目").fill(a);
  await dialog
    .locator(".batch-target-list button")
    .filter({ has: page.getByText(a, { exact: true }) })
    .click();
  await rows.nth(6).getByRole("button", { name: "选择曲目" }).click();
  await dialog.getByLabel("搜索配对曲目").fill(a);
  await dialog
    .locator(".batch-target-list button")
    .filter({ has: page.getByText(a, { exact: true }) })
    .click();
  await dialog
    .getByRole("button", { name: `上移 ${a}-page01.png`, exact: true })
    .click();
  await dialog.getByLabel(`保存 ${b}.pdf`, { exact: true }).uncheck();
  await page.screenshot({ path: "../outputs/Neothesia-批量谱面配对.png" });
  await dialog
    .getByRole("button", { name: "保存选中配对（4）", exact: true })
    .click();
  await expect(dialog).toContainText("已完成 5 / 7");
  await expect(rows.nth(6)).toContainText(/XML|xml|乐谱|score|谱/);
  await expect(rows.nth(6)).toHaveAttribute("data-state", "error");
  const current = await cmd({ type: "currentSong" });
  expect(current.notes).toHaveLength(12);
  const library = await (
    await request.get("http://127.0.0.1:32124/api/library")
  ).json();
  const rowA = library.songs.find((r: any) => r.title === a),
    rowB = library.songs.find((r: any) => r.title === b);
  expect(rowA).toBeTruthy();
  expect(rowB).toBeTruthy();
  const identityA = await cmd({
      type: "inspectSong",
      path: rowA.path,
      force: true,
    }),
    identityB = await cmd({
      type: "inspectSong",
      path: rowB.path,
      force: true,
    });
  expect(current.contentId).toBe(identityB.contentId);
  const savedA = await cmd({
    type: "libraryScores",
    path: rowA.path,
    content_id: identityA.contentId,
  });
  expect(savedA.versions).toHaveLength(1);
  expect(savedA.papers.attachments).toHaveLength(1);
  expect(savedA.papers.attachments[0].name).toBe(a);
  expect(savedA.papers.attachments[0].pages.map((p: any) => p.name)).toEqual([
    a + "-page01.png",
    a + "-page02.png",
  ]);
  expect(
    (
      await cmd({
        type: "libraryScores",
        path: rowB.path,
        content_id: identityB.contentId,
      })
    ).papers.attachments,
  ).toHaveLength(0);
  await dialog.getByLabel(`保存 ${b}.pdf`, { exact: true }).check();
  await dialog
    .getByRole("button", { name: "保存选中配对（2）", exact: true })
    .click();
  await expect(dialog).toContainText("已完成 6 / 7");
  const savedB = await cmd({
    type: "libraryScores",
    path: rowB.path,
    content_id: identityB.contentId,
  });
  expect(savedB.papers.attachments).toHaveLength(1);
  expect(savedB.papers.attachments[0].pages).toHaveLength(1);
  expect(
    (
      await cmd({
        type: "libraryScores",
        path: rowA.path,
        content_id: identityA.contentId,
      })
    ).papers.attachments,
  ).toHaveLength(1);
  await page.setViewportSize({ width: 700, height: 1100 });
  await page.screenshot({ path: "../outputs/Neothesia-批量谱面配对-窄窗.png" });
  await dialog.getByRole("button", { name: "完成", exact: true }).click();
  expect(
    readFileSync("../neothesia-engine/assets/five-finger.mid").equals(
      originalFirst,
    ),
  ).toBeTruthy();
  expect(
    readFileSync("../neothesia-engine/assets/basic-chords.mid").equals(
      originalSecond,
    ),
  ).toBeTruthy();
});

test("配对候选保留作品编号，同名歧义不自动归属，图片页码单独处理", () => {
  const targets = [
    { path: "a", title: "Prelude 1", composer: "", aliases: ["Prelude 1"] },
    { path: "b", title: "Prelude 2", composer: "", aliases: ["Prelude 2"] },
  ];
  expect(
    pairingCandidates("Prelude-1.xml", targets).map((t) => t.path),
  ).toEqual(["a"]);
  expect(
    pairingCandidates("Prelude 1-page02.png", targets).map((t) => t.path),
  ).toEqual(["a"]);
  expect(
    pairingCandidates("Prelude 1.xml", [
      ...targets,
      { ...targets[0], path: "c" },
    ]),
  ).toHaveLength(2);
  expect(pairingCandidates("Prelude.xml", targets)).toHaveLength(0);
  expect(imageBook("曲目-page12.png")).toBe("曲目");
  expect(importKind("score.exe")).toBeNull();
});

test("图片部分保存后补页必须核对实际顺序", async ({ page, request }) => {
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const name = "顺序课堂" + Date.now(),
    midi = readFileSync("../neothesia-engine/assets/parallel-five-finger.mid");
  const imageA = Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAD0lEQVR4nGNgSDmBQMRxANtDEsHsiNYJAAAAAElFTkSuQmCC",
      "base64",
    ),
    imageB = Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAEUlEQVR4nGP4n3ICjhiI4wAAibEisVa1kVcAAAAASUVORK5CYII=",
      "base64",
    );
  await page.goto("/");
  await page.getByRole("button", { name: "导入曲目", exact: true }).click();
  await page.getByRole("button", { name: "批量配对谱面", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "批量配对曲目与谱面" });
  await dialog.getByLabel("批量配对文件").setInputFiles([
    { name: name + ".mid", mimeType: "audio/midi", buffer: midi },
    { name: name + "-page01.png", mimeType: "image/png", buffer: imageA },
    { name: name + "-page02.png", mimeType: "image/png", buffer: imageB },
  ]);
  await dialog
    .getByRole("button", { name: "导入 MIDI 并读取配对候选" })
    .click();
  await expect(dialog).toContainText("已完成 1 / 3");
  await dialog
    .locator("tbody tr")
    .nth(1)
    .getByRole("button", { name: "预览", exact: true })
    .click();
  await expect(
    dialog.getByRole("region", { name: "待导入谱面预览" }).locator("img"),
  ).toBeVisible();
  await dialog.getByRole("button", { name: "返回配对列表" }).click();
  await dialog.getByLabel(`保存 ${name}-page01.png`, { exact: true }).uncheck();
  await dialog
    .getByRole("button", { name: "保存选中配对（1）", exact: true })
    .click();
  await expect(dialog).toContainText("已完成 2 / 3");
  await dialog.getByLabel(`保存 ${name}-page01.png`, { exact: true }).check();
  await dialog
    .getByRole("button", { name: "保存选中配对（1）", exact: true })
    .click();
  await expect(dialog).toContainText("后面的谱页已保存");
  await expect(dialog).toContainText("已完成 2 / 3");
  await dialog
    .getByRole("button", { name: `下移 ${name}-page01.png`, exact: true })
    .click();
  await dialog
    .getByRole("button", { name: "保存选中配对（1）", exact: true })
    .click();
  await expect(dialog).toContainText("已完成 3 / 3");
  const song = await cmd({ type: "currentSong" }),
    papers = await cmd({
      type: "scoreAttachments",
      content_id: song.contentId,
    });
  const book = papers.attachments.find((r: any) => r.name === name);
  expect(book.pages.map((r: any) => r.name)).toEqual([
    name + "-page02.png",
    name + "-page01.png",
  ]);
});
