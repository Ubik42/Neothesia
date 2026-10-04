import { test, expect, chromium } from "@playwright/test";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
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

test("批次重开保留真实文件和配对；中断项核对采用或恢复，删除批次保留谱面", async ({
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
  const prefix = "恢复课堂" + Date.now(),
    batchName = prefix + "配对清单",
    xmlName = prefix + "教师原谱.xml";
  const pngA = Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAD0lEQVR4nGNgSDmBQMRxANtDEsHsiNYJAAAAAElFTkSuQmCC",
      "base64",
    ),
    pngB = Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAEUlEQVR4nGP4n3ICjhiI4wAAibEisVa1kVcAAAAASUVORK5CYII=",
      "base64",
    );
  const originalMidi = readFileSync(
      "../neothesia-engine/assets/bass-five-finger.mid",
    ),
    text = Buffer.from(prefix),
    track = Buffer.concat([
      Buffer.from([0, 255, 1, text.length]),
      text,
      Buffer.from([0, 255, 47, 0]),
    ]),
    chunk = Buffer.alloc(8),
    sourceMidi = Buffer.from(originalMidi);
  sourceMidi.writeUInt16BE(sourceMidi.readUInt16BE(10) + 1, 10);
  chunk.write("MTrk");
  chunk.writeUInt32BE(track.length, 4);
  const uniqueMidi = Buffer.concat([sourceMidi, chunk, track]);
  const open = async () => {
    await page.getByRole("button", { name: "导入曲目", exact: true }).click();
    await page
      .getByRole("button", { name: "批量配对谱面", exact: true })
      .click();
  };
  const dialog = page.getByRole("dialog", { name: "批量配对曲目与谱面" });
  await page.goto("/");
  await open();
  await dialog.getByLabel("批量配对文件").setInputFiles([
    {
      name: prefix + ".mid",
      mimeType: "audio/midi",
      buffer: uniqueMidi,
    },
    { name: prefix + ".pdf", mimeType: "application/pdf", buffer: pdf() },
    { name: prefix + "-page01.png", mimeType: "image/png", buffer: pngA },
    { name: prefix + "-page02.png", mimeType: "image/png", buffer: pngB },
    {
      name: xmlName,
      mimeType: "application/xml",
      buffer: readFileSync(
        "../neothesia-core/src/score_performance_test.musicxml",
      ),
    },
  ]);
  await dialog.getByLabel("批次名称", { exact: true }).fill(batchName);
  await dialog.getByRole("button", { name: "保存此批次", exact: true }).click();
  await expect(dialog).toContainText("文件副本与进度已保存在本机");
  await dialog
    .getByRole("button", { name: "导入 MIDI 并读取配对候选" })
    .click();
  await expect(dialog).toContainText("已完成 1 / 5");
  const rows = dialog.locator("tbody tr");
  await rows.nth(4).getByRole("button", { name: "选择曲目" }).click();
  await dialog.getByLabel("搜索配对曲目").fill(prefix);
  await dialog
    .locator(".batch-target-list button")
    .filter({ has: page.getByText(prefix, { exact: true }) })
    .click();
  await dialog
    .getByLabel(`纸谱名称 ${prefix}-page01.png`, { exact: true })
    .fill("课堂纸谱");
  await dialog
    .getByLabel(`纸谱名称 ${prefix}-page02.png`, { exact: true })
    .fill("课堂纸谱");
  await dialog.getByLabel(`保存 ${prefix}.pdf`, { exact: true }).uncheck();
  await dialog
    .getByLabel(`保存 ${prefix}-page02.png`, { exact: true })
    .uncheck();
  await dialog.getByLabel(`保存 ${xmlName}`, { exact: true }).uncheck();
  await dialog
    .getByRole("button", { name: "保存选中配对（1）", exact: true })
    .click();
  await expect(dialog).toContainText("已完成 2 / 5");
  await dialog
    .getByRole("button", { name: "保存批次进度", exact: true })
    .click();
  const songBefore = await cmd({ type: "currentSong" });
  // Simulate termination at a persisted "working" checkpoint. One write
  // already reached the engine, the other never reached it.
  await page.evaluate(async (name) => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const r = indexedDB.open("neothesia-score-imports", 1);
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction("batches", "readwrite"),
        store = tx.objectStore("batches"),
        r = store.getAll();
      r.onsuccess = () => {
        const b = r.result.find((b: any) => b.name === name);
        b.items[2].state = "working";
        b.items[4].state = "working";
        b.revision++;
        store.put(b);
      };
      tx.oncomplete = () => resolve();
      tx.onabort = () => reject(tx.error);
    });
    db.close();
  }, batchName);
  await page.reload();
  await open();
  await dialog.getByRole("button", { name: /^已保存批次/ }).click();
  const savedList = dialog.getByRole("region", { name: "已保存的配对批次" });
  await expect(savedList).toContainText(batchName);
  await savedList
    .getByRole("button", { name: "继续此批次", exact: true })
    .click();
  await expect(dialog.getByLabel("批次名称")).toHaveValue(batchName);
  await expect(rows.nth(4)).toContainText("上次保存中断");
  await expect(
    dialog.getByLabel(`保存 ${xmlName}`, { exact: true }),
  ).toBeDisabled();
  await expect(
    dialog.getByLabel(`纸谱名称 ${prefix}-page02.png`, { exact: true }),
  ).toHaveValue("课堂纸谱");
  await rows.nth(1).getByRole("button", { name: "预览", exact: true }).click();
  const preview = dialog.getByRole("region", { name: "待导入谱面预览" });
  await expect(preview.locator("canvas")).toBeVisible();
  await expect(preview).toContainText("第 1 / 2 页");
  await preview.getByRole("button", { name: "返回配对列表" }).click();
  await rows
    .nth(2)
    .getByRole("button", { name: "核对中断项", exact: true })
    .click();
  const review = dialog.getByRole("region", { name: "核对中断保存结果" });
  await expect(review).toContainText("课堂纸谱");
  await review
    .getByRole("button", { name: "采用此保存结果", exact: true })
    .click();
  await expect(rows.nth(2)).toHaveAttribute("data-state", "done");
  await rows
    .nth(4)
    .getByRole("button", { name: "核对中断项", exact: true })
    .click();
  await expect(review).toContainText("未找到同内容的已保存文件");
  await review
    .getByRole("button", { name: "恢复为待保存项", exact: true })
    .click();
  await expect(rows.nth(4)).toHaveAttribute("data-state", "pending");
  await dialog.getByLabel(`保存 ${prefix}.pdf`, { exact: true }).check();
  await dialog.getByLabel(`保存 ${prefix}-page02.png`, { exact: true }).check();
  await page.screenshot({ path: "../outputs/Neothesia-配对批次恢复.png" });
  await dialog
    .getByRole("button", { name: "保存选中配对（3）", exact: true })
    .click();
  await expect(dialog).toContainText("已完成 5 / 5");
  const now = await cmd({ type: "currentSong" });
  expect(now.contentId).toBe(songBefore.contentId);
  const scores = await cmd({
    type: "libraryScores",
    path: now.sourcePath,
    content_id: now.contentId,
  });
  expect(scores.versions).toHaveLength(1);
  expect(scores.papers.attachments).toHaveLength(2);
  expect(
    scores.papers.attachments
      .find((b: any) => b.name === "课堂纸谱")
      .pages.map((p: any) => p.name),
  ).toEqual([prefix + "-page01.png", prefix + "-page02.png"]);
  const imageBook = scores.papers.attachments.find(
      (b: any) => b.name === "课堂纸谱",
    ),
    pdfBook = scores.papers.attachments.find((b: any) => b.format === "pdf");
  const verifyImage = async (target: string) => {
    const response = await request.post(
      "http://127.0.0.1:32124/api/library-score/check",
      {
        headers: {
          "X-Neothesia-Client": "web",
          "X-Neothesia-Path": encodeURIComponent(now.sourcePath),
          "X-Neothesia-Song": now.contentId,
          "X-Neothesia-Kind": "image",
          "X-Neothesia-Target": target,
        },
        data: pngB,
      },
    );
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  expect((await verifyImage(imageBook.id)).matches).toHaveLength(1);
  expect((await verifyImage(pdfBook.id)).matches).toHaveLength(0);
  // Stale local revisions are rejected atomically, without overwriting
  // either the metadata or the cached source files.
  const guard = await page.evaluate(async () => {
    const store = await import("/src/batchImportStore.ts");
    const rows = await store.listBatches(),
      a = await store.loadBatch(rows[0].id),
      b = await store.loadBatch(rows[0].id);
    await store.saveBatch({ ...a, name: a.name });
    let refused = false;
    try {
      await store.saveBatch({ ...b, name: "旧窗口覆盖" });
    } catch {
      refused = true;
    }
    return {
      refused,
      name: (await store.loadBatch(a.id)).name,
      files: (await store.loadBatch(a.id)).items.length,
    };
  });
  expect(guard).toEqual({ refused: true, name: batchName, files: 5 });
  await page.reload();
  await open();
  await dialog.getByRole("button", { name: /^已保存批次/ }).click();
  await expect(savedList).toContainText("已完成 5 / 5");
  await page.setViewportSize({ width: 700, height: 1100 });
  await page.screenshot({ path: "../outputs/Neothesia-配对批次列表-窄窗.png" });
  await savedList
    .getByRole("button", { name: "移除批次", exact: true })
    .click();
  await savedList
    .getByRole("button", { name: "确认移除批次副本", exact: true })
    .click();
  await expect(savedList).toContainText("还没有保存的批次");
  const retained = await cmd({
    type: "libraryScores",
    path: now.sourcePath,
    content_id: now.contentId,
  });
  expect(retained.versions).toHaveLength(1);
  expect(retained.papers.attachments).toHaveLength(2);
});

test("关闭浏览器进程后仍可恢复批次文件副本", async () => {
  test.setTimeout(45000);
  const name = "进程恢复" + Date.now(),
    profile = resolve("../work/cycle200-persistent-" + Date.now());
  let context = await chromium.launchPersistentContext(profile, {
    headless: true,
    viewport: { width: 1440, height: 960 },
  });
  const enter = async (p: any) => {
    await p.goto("http://127.0.0.1:5173/");
    await p.getByRole("button", { name: "导入曲目", exact: true }).click();
    await p.getByRole("button", { name: "批量配对谱面", exact: true }).click();
    return p.getByRole("dialog", { name: "批量配对曲目与谱面" });
  };
  try {
    const p = context.pages()[0],
      dialog = await enter(p);
    await dialog.getByLabel("批量配对文件").setInputFiles({
      name: name + ".pdf",
      mimeType: "application/pdf",
      buffer: pdf(),
    });
    await dialog.getByLabel("批次名称").fill(name);
    await dialog
      .getByRole("button", { name: "保存此批次", exact: true })
      .click();
    await expect(dialog).toContainText("文件副本与进度已保存在本机");
    await context.close();
    context = await chromium.launchPersistentContext(profile, {
      headless: true,
      viewport: { width: 1440, height: 960 },
    });
    const reopened = context.pages()[0],
      restored = await enter(reopened);
    await restored.getByRole("button", { name: /^已保存批次/ }).click();
    await restored
      .getByRole("region", { name: "已保存的配对批次" })
      .getByRole("button", { name: "继续此批次", exact: true })
      .click();
    await expect(restored).toContainText(name + ".pdf");
    await restored
      .locator("tbody tr")
      .getByRole("button", { name: "预览", exact: true })
      .click();
    const preview = restored.getByRole("region", { name: "待导入谱面预览" });
    await expect(preview.locator("canvas")).toBeVisible();
    await expect(preview).toContainText("第 1 / 2 页");
    await preview.getByRole("button", { name: "下一页", exact: true }).click();
    await expect(preview).toContainText("第 2 / 2 页");
  } finally {
    await context.close();
  }
});
