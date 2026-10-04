import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
test("批次真实文件导出、独立浏览器迁移、内容迁址和继续组页", async ({
  page,
  browser,
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
  const prefix = "迁移课堂" + Date.now(),
    source = Buffer.from(
      readFileSync("../neothesia-engine/assets/bass-five-finger.mid"),
    ),
    text = Buffer.from(prefix),
    track = Buffer.concat([
      Buffer.from([0, 255, 1, text.length]),
      text,
      Buffer.from([0, 255, 47, 0]),
    ]),
    chunk = Buffer.alloc(8);
  source.writeUInt16BE(source.readUInt16BE(10) + 1, 10);
  chunk.write("MTrk");
  chunk.writeUInt32BE(track.length, 4);
  const midi = Buffer.concat([source, chunk, track]);
  const a = Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAD0lEQVR4nGNgSDmBQMRxANtDEsHsiNYJAAAAAElFTkSuQmCC",
      "base64",
    ),
    b = Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAEUlEQVR4nGP4n3ICjhiI4wAAibEisVa1kVcAAAAASUVORK5CYII=",
      "base64",
    );
  const open = async (p: any) => {
    await p.goto("/");
    await p.getByRole("button", { name: "导入曲目", exact: true }).click();
    await p.getByRole("button", { name: "批量配对谱面", exact: true }).click();
    return p.getByRole("dialog", { name: "批量配对曲目与谱面" });
  };
  const d = await open(page);
  await d.getByLabel("批量配对文件").setInputFiles([
    { name: prefix + ".mid", mimeType: "audio/midi", buffer: midi },
    { name: prefix + "-page01.png", mimeType: "image/png", buffer: a },
    { name: prefix + "-page02.png", mimeType: "image/png", buffer: b },
  ]);
  await d.getByLabel("批次名称", { exact: true }).fill(prefix);
  await d.getByRole("button", { name: "保存此批次", exact: true }).click();
  await d
    .getByRole("button", { name: "导入 MIDI 并读取配对候选", exact: true })
    .click();
  for (const name of ["-page01.png", "-page02.png"])
    await d
      .getByLabel(`纸谱名称 ${prefix}${name}`, { exact: true })
      .fill("教师示范纸谱");
  await d.getByLabel(`保存 ${prefix}-page02.png`, { exact: true }).uncheck();
  await d
    .getByRole("button", { name: "保存选中配对（1）", exact: true })
    .click();
  await expect(d).toContainText("已完成 2 / 3");
  await page.evaluate(() => {
    const original = crypto.subtle.digest.bind(crypto.subtle);
    (window as any).batchOriginalDigest = original;
    crypto.subtle.digest = async (
      ...args: Parameters<SubtleCrypto["digest"]>
    ) => {
      await new Promise((r) => setTimeout(r, 300));
      return original(...args);
    };
  });
  let exportEvents = 0;
  page.on("download", () => exportEvents++);
  await d.getByRole("button", { name: "导出整理批次", exact: true }).click();
  await d.getByRole("button", { name: "取消批次导出", exact: true }).click();
  await expect(d.getByRole("alert")).toContainText("已取消批次导出");
  expect(exportEvents).toBe(0);
  await page.evaluate(() => {
    crypto.subtle.digest = (window as any).batchOriginalDigest;
  });
  const downloaded = page.waitForEvent("download");
  await d.getByRole("button", { name: "导出整理批次", exact: true }).click();
  const download = await downloaded;
  expect(download.suggestedFilename()).toBe(prefix + ".neoscorebatch");
  const bytes = readFileSync((await download.path())!);
  expect(bytes.subarray(0, 8).toString()).toBe("NEOBAT01");
  const len = bytes.readUInt32LE(8),
    manifest = JSON.parse(bytes.subarray(12, 12 + len).toString());
  expect(manifest.items.map((i: any) => i.state)).toEqual([
    "done",
    "done",
    "pending",
  ]);
  for (const item of manifest.items)
    if (item.target) item.target.path = "Z:/old-computer/performance.mid";
  const metadata = Buffer.from(JSON.stringify(manifest)),
    header = Buffer.from(bytes.subarray(0, 12));
  header.writeUInt32LE(metadata.length, 8);
  const portable = Buffer.concat([header, metadata, bytes.subarray(12 + len)]);
  const context = await browser.newContext(),
    other = await context.newPage(),
    restored = await open(other);
  await restored.getByLabel("选择曲谱整理批次文件").setInputFiles({
    name: "课堂.neoscorebatch",
    mimeType: "application/octet-stream",
    buffer: portable,
  });
  const preview = restored.getByRole("region", { name: "整理批次导入预览" });
  await expect(preview).toContainText("3 个文件");
  await expect(preview).toContainText("1 项需要核对");
  await expect(preview).toContainText("演奏已核对");
  await preview
    .locator("tbody tr")
    .nth(1)
    .getByRole("button", { name: "预览文件", exact: true })
    .click();
  await expect(restored.locator(".batch-file-preview img")).toBeVisible();
  await restored
    .getByRole("button", { name: "返回配对列表", exact: true })
    .click();
  await preview.scrollIntoViewIfNeeded();
  await other.screenshot({ path: "../outputs/Neothesia-整理批次迁移.png" });
  await other.setViewportSize({ width: 700, height: 900 });
  await preview.scrollIntoViewIfNeeded();
  await other.screenshot({
    path: "../outputs/Neothesia-整理批次迁移-窄窗.png",
  });
  await preview
    .getByRole("button", { name: "另存为本机新批次", exact: true })
    .click();
  await restored
    .getByRole("button", { name: "导入 MIDI 并读取配对候选", exact: true })
    .click();
  await expect(restored).toContainText("已完成 1 / 3");
  await restored
    .locator("tbody tr")
    .nth(1)
    .getByRole("button", { name: "核对中断项", exact: true })
    .click();
  await restored
    .getByRole("region", { name: "核对中断保存结果" })
    .getByRole("button", { name: "采用此保存结果", exact: true })
    .click();
  await restored
    .getByLabel(`保存 ${prefix}-page02.png`, { exact: true })
    .check();
  await restored
    .getByRole("button", { name: "保存选中配对（1）", exact: true })
    .click();
  await expect(restored).toContainText("已完成 3 / 3");
  const song = await cmd({ type: "currentSong" }),
    papers = await cmd({
      type: "libraryScores",
      path: song.sourcePath,
      content_id: song.contentId,
    });
  expect(
    papers.papers.attachments.filter((s: any) => s.name === "教师示范纸谱"),
  ).toHaveLength(1);
  expect(
    papers.papers.attachments.find((s: any) => s.name === "教师示范纸谱").pages,
  ).toHaveLength(2);
  const corrupt = Buffer.from(portable);
  corrupt[corrupt.length - 8] ^= 1;
  await restored.getByLabel("选择曲谱整理批次文件").setInputFiles({
    name: "损坏.neoscorebatch",
    mimeType: "application/octet-stream",
    buffer: corrupt,
  });
  await expect(restored.getByRole("alert")).toContainText("文件副本损坏");
  await expect(
    restored.getByRole("region", { name: "整理批次导入预览" }),
  ).toHaveCount(0);
  await context.close();
});
