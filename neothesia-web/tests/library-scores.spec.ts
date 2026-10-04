import { test, expect } from "@playwright/test";
import fs from "node:fs/promises";
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
test("曲库跨曲目版本改名、真实 PDF 预览、关联切换及本页选择", async ({
  page,
  request,
}) => {
  test.setTimeout(90000);
  page.setDefaultTimeout(15000);
  test.skip(!process.env.NEOTHESIA_LIBRARY_SCORE_DATA, "独立曲库谱面服务");
  const command = async (v: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data: v,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const xml = `<score-partwise version="4.0"><work><work-title>曲库版本课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note></measure></part></score-partwise>`;
  await command({
    type: "importScore",
    bytes: Array.from(Buffer.from(xml)),
    name: "课前原谱.musicxml",
    default_bpm: 100,
  });
  const song = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  await command({
    type: "pairScore",
    content_id: song.contentId,
    name: "老师批注版.musicxml",
    bytes: Array.from(Buffer.from(xml)),
  });
  await command({ type: "inspectSong", path: song.sourcePath, force: true });
  const upload = await request.post(
    "http://127.0.0.1:32124/api/score-attachment/add",
    {
      headers: {
        "X-Neothesia-Client": "web",
        "X-Neothesia-Song": song.contentId,
        "X-Neothesia-Name": encodeURIComponent("课堂打印谱.pdf"),
      },
      data: pdf(),
    },
  );
  expect(upload.ok()).toBeTruthy();
  const rows = await (
    await request.get("http://127.0.0.1:32124/api/library")
  ).json();
  const row = rows.songs.find((r: any) => r.path.endsWith("five-finger.mid"));
  await command({ type: "load", path: row.path, title: row.title });
  const current = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  expect(current.contentId).not.toBe(song.contentId);
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "曲库管理" });
  await dialog
    .getByRole("textbox", { name: "管理曲库搜索" })
    .fill("曲库版本课堂");
  await dialog.locator(".library-manager-table tbody tr").first().click();
  const panel = dialog.getByRole("region", { name: "曲目谱面版本" });
  await expect(panel.getByLabel("版本名称 老师批注版.musicxml")).toBeVisible();
  await expect(panel.getByText("2 份演奏乐谱", { exact: false })).toBeVisible();
  const teacher = panel
    .getByLabel("版本名称 老师批注版.musicxml")
    .locator("..")
    .locator("..");
  await teacher.getByRole("textbox").fill("本周老师版");
  await teacher.getByRole("button", { name: "保存名称" }).click();
  await expect(panel.getByLabel("版本名称 本周老师版")).toHaveValue(
    "本周老师版",
  );
  expect(
    (await (await request.get("http://127.0.0.1:32124/api/song")).json())
      .contentId,
  ).toBe(current.contentId);
  const paper = panel
    .getByLabel("版本名称 课堂打印谱.pdf")
    .locator("..")
    .locator("..");
  await paper.getByRole("button", { name: "预览谱页" }).click();
  const preview = page.getByRole("dialog", { name: "曲库谱页预览" });
  await expect
    .poll(() => preview.locator("canvas").evaluate((c: any) => c.width))
    .toBeGreaterThan(300);
  await preview.getByRole("button", { name: "下一谱页" }).click();
  await expect(preview.getByLabel("谱页页码")).toHaveValue("2");
  await page.keyboard.press("Escape");
  await expect(preview).not.toBeVisible();
  await expect(dialog).toBeVisible();
  await expect(panel.getByText(/阅读位置：第 2 页/)).toBeVisible();
  expect(
    (await (await request.get("http://127.0.0.1:32124/api/song")).json())
      .contentId,
  ).toBe(current.contentId);
  const monitor = await command({ type: "libraryMonitorStatus" });
  await dialog.getByLabel("选择本页曲目").check();
  await expect(
    dialog.locator(".library-manager-table tbody input[type=checkbox]").first(),
  ).toBeChecked();
  expect((await command({ type: "libraryMonitorStatus" })).enabled).toBe(
    monitor.enabled,
  );
  await dialog.getByLabel("选择本页曲目").uncheck();
  await expect(
    dialog.locator(".library-manager-table tbody input[type=checkbox]").first(),
  ).not.toBeChecked();
  await page.screenshot({ path: "../outputs/Neothesia-曲库谱面版本.png" });
  const v = await command({
    type: "libraryScores",
    path: song.sourcePath,
    content_id: song.contentId,
  });
  const original = v.versions.find((x: any) => !x.active);
  await fs.writeFile(original.path, "changed");
  await panel.getByRole("button", { name: "重新检查谱面资料" }).click();
  await expect(panel.getByText("内容已改变", { exact: true })).toBeVisible();
  expect(
    (await (await request.get("http://127.0.0.1:32124/api/song")).json())
      .contentId,
  ).toBe(current.contentId);
  await panel
    .getByLabel("版本名称 本周老师版")
    .locator("..")
    .locator("..")
    .getByRole("button", { name: "打开乐谱练习" })
    .click();
  await expect(dialog).not.toBeVisible();
  await expect(page.locator(".score-paper svg").first()).toBeVisible();
  expect(
    (await (await request.get("http://127.0.0.1:32124/api/song")).json())
      .contentId,
  ).toBe(song.contentId);
});
