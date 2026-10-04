import { test, expect } from "@playwright/test";
import fs from "node:fs/promises";
import path from "node:path";
test("曲库直接多文件谱面导入、图片页序、原件修复和错误拒绝不切换演奏", async ({
  page,
  request,
}) => {
  test.setTimeout(90000);
  const root = process.env.NEOTHESIA_LIBRARY_SCORE_DATA;
  test.skip(!root, "独立谱面导入服务");
  const command = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const xml = `<score-partwise version="4.0"><work><work-title>附件整理课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note></measure></part></score-partwise>`;
  await command({
    type: "importScore",
    bytes: Array.from(Buffer.from(xml)),
    name: "课前.musicxml",
    default_bpm: 60,
  });
  const target = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  await command({ type: "inspectSong", path: target.sourcePath, force: true });
  const library = await (
    await request.get("http://127.0.0.1:32124/api/library")
  ).json();
  const warm = library.songs.find((r: any) =>
    r.path.endsWith("five-finger.mid"),
  );
  await command({ type: "load", path: warm.path, title: warm.title });
  const current = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "曲库管理" });
  await dialog.getByLabel("管理曲库搜索").fill("附件整理课堂");
  await dialog.locator(".library-manager-table tbody tr").first().click();
  const panel = dialog.getByRole("region", { name: "曲目谱面版本" });
  await expect(
    panel.getByRole("button", { name: "添加演奏乐谱" }),
  ).toBeVisible();
  await panel.getByRole("button", { name: "添加演奏乐谱" }).click();
  await panel.getByLabel("曲库添加谱面文件").setInputFiles([
    {
      name: "课堂批注.musicxml",
      mimeType: "application/xml",
      buffer: Buffer.from(xml),
    },
    {
      name: "无效.musicxml",
      mimeType: "application/xml",
      buffer: Buffer.from("bad xml"),
    },
  ]);
  await expect(panel.getByRole("status",{name:"谱面文件处理进度"})).toContainText("成功 1，失败 1");
  await expect(panel.getByLabel("版本名称 课堂批注.musicxml")).toBeVisible();
  expect(
    (await (await request.get("http://127.0.0.1:32124/api/song")).json())
      .contentId,
  ).toBe(current.contentId);
  const imgs = await page.evaluate(() => {
    return ["#ffffff", "#dddddd", "#aaaaaa"].map((color) => {
      const c = document.createElement("canvas");
      c.width = 160;
      c.height = 200;
      const ctx = c.getContext("2d")!;
      ctx.fillStyle = color;
      ctx.fillRect(0, 0, 160, 200);
      ctx.fillStyle = "#111";
      ctx.fillText("Piano", 10, 20);
      return c.toDataURL("image/png").split(",")[1];
    });
  });
  const files = imgs.map((s, i) => ({
    name: `谱页${i + 1}.png`,
    mimeType: "image/png",
    buffer: Buffer.from(s, "base64"),
  }));
  await panel.getByRole("button", { name: "添加 PDF / 图片" }).click();
  await panel.getByLabel("曲库添加谱面文件").setInputFiles(files.slice(0, 2));
  await expect(panel.getByRole("status",{name:"谱面文件处理进度"})).toContainText("成功 2，失败 0");
  const book = panel
    .getByLabel("版本名称 谱页1.png")
    .locator("..")
    .locator("..");
  await expect(book.getByText("2 页图片", { exact: true })).toBeVisible();
  await book.getByRole("button", { name: "追加图片页" }).click();
  await panel.getByLabel("曲库添加谱面文件").setInputFiles(files[2]);
  await expect(book.getByText("3 页图片", { exact: true })).toBeVisible();
  await book.locator("summary").filter({ hasText: "页序与修复" }).click();
  await book.getByLabel("上移谱页 3").click();
  await expect(book.locator(".library-paper-page").nth(1)).toContainText(
    "谱页3.png",
  );
  let info = await command({
    type: "libraryScores",
    path: target.sourcePath,
    content_id: target.contentId,
  });
  const paper = info.papers.attachments[0];
  await command({
    type: "updateScoreAttachment",
    content_id: target.contentId,
    id: paper.id,
    action: "view",
    view: { page: 2, zoom: 125, fit: false, rotation: 90 },
  });
  const folders = await fs.readdir(path.join(root!, "score-attachments"));
  let folder = "";
  for (const name of folders) {
    const dir = path.join(root!, "score-attachments", name);
    const manifest = JSON.parse(
      await fs.readFile(path.join(dir, "manifest.json"), "utf8"),
    );
    if (manifest.contentId === target.contentId) folder = dir;
  }
  const first = paper.pages[0];
  const physical = path.join(folder, `${first.id}.${first.kind}`);
  await fs.writeFile(physical, "corrupt");
  await panel.getByRole("button", { name: "重新检查谱面资料" }).click();
  const updatedBook = panel
    .getByLabel("版本名称 谱页1.png")
    .locator("..")
    .locator("..");
  if (!await updatedBook.locator(".library-paper-pages").evaluate((el:any)=>el.open)) await updatedBook.locator("summary").filter({hasText:"页序与修复"}).click();
  const damaged = updatedBook
    .locator(".library-paper-page")
    .filter({ hasText: "谱页1.png" });
  await damaged.getByRole("button", { name: "修复此页原文件" }).click();
  await panel.getByLabel("曲库添加谱面文件").setInputFiles(files[1]);
  await expect(panel.getByRole("status",{name:"谱面文件处理进度"})).toContainText("成功 0，失败 1");
  await expect(
    panel.getByText("文件内容与原谱页不符", { exact: false }),
  ).toBeVisible();
  await damaged.getByRole("button", { name: "修复此页原文件" }).click();
  await panel.getByLabel("曲库添加谱面文件").setInputFiles(files[0]);
  await expect(panel.getByRole("status",{name:"谱面文件处理进度"})).toContainText("成功 1，失败 0");
  info = await command({
    type: "libraryScores",
    path: target.sourcePath,
    content_id: target.contentId,
  });
  expect(info.papers.attachments[0].id).toBe(paper.id);
  expect(info.papers.attachments[0].view).toEqual({
    page: 2,
    zoom: 125,
    fit: false,
    rotation: 90,
  });
  expect(info.papers.attachments[0].pages[0].status).toBe("ready");
  const notation = info.versions.find(
    (v: any) => v.name === "课堂批注.musicxml",
  );
  await fs.writeFile(notation.path, "damaged");
  await panel.getByRole("button", { name: "重新检查谱面资料" }).click();
  const score = panel
    .getByLabel("版本名称 课堂批注.musicxml")
    .locator("..")
    .locator("..");
  await score.getByRole("button", { name: "用原文件修复" }).click();
  await panel
    .getByLabel("曲库添加谱面文件")
    .setInputFiles({
      name: "原始.musicxml",
      mimeType: "application/xml",
      buffer: Buffer.from(xml),
    });
  await expect(panel.getByRole("status",{name:"谱面文件处理进度"})).toContainText("成功 1，失败 0");
  info = await command({
    type: "libraryScores",
    path: target.sourcePath,
    content_id: target.contentId,
  });
  expect(info.versions.find((v: any) => v.id === notation.id).status).toBe(
    "ready",
  );
  expect(
    info.versions.filter((v: any) => v.name === "课堂批注.musicxml"),
  ).toHaveLength(1);
  if (!await updatedBook.locator(".library-paper-pages").evaluate((el:any)=>el.open)) await updatedBook.locator("summary").filter({hasText:"页序与修复"}).click();
  await updatedBook
    .locator(".library-paper-page")
    .last()
    .getByRole("button", { name: "移除页登记" })
    .click();
  await panel
    .getByRole("button", { name: "确认移除图片页", exact: true })
    .click();
  await expect(
    updatedBook.getByText("2 页图片", { exact: true }),
  ).toBeVisible();
  await expect(panel.getByRole("button",{name:"添加演奏乐谱"})).toBeEnabled();
  await panel.evaluate(el=>{const side=el.closest<HTMLElement>(".library-file-details")!;side.scrollTop+=el.getBoundingClientRect().top-side.getBoundingClientRect().top-8;});
  await page.screenshot({
    path: "../outputs/Neothesia-曲库批量谱面整理.png",
    fullPage: true,
  });
  expect(
    (await (await request.get("http://127.0.0.1:32124/api/song")).json())
      .contentId,
  ).toBe(current.contentId);
});
