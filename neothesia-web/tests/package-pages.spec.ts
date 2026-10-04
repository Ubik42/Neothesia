import { test, expect } from "@playwright/test";
import { randomBytes } from "node:crypto";
test("大曲目包直接文件传输、谱页预览、合并及替换阅读设置", async ({
  page,
  request,
}) => {
  test.setTimeout(90000);
  test.skip(!process.env.NEOTHESIA_PACKAGE_TEST_DATA, "独立曲目包测试服务");
  const command = async (v: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data: v,
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
    title: "曲目资料携带课堂",
  });
  // Valid PDF comment with incompressible content makes the archive exceed the former 3 MB limit.
  let pdf = "%PDF-1.4\n%" + randomBytes(4_000_000).toString("base64") + "\n";
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] >>",
  ];
  const offsets = [0];
  objects.forEach((o, i) => {
    offsets.push(pdf.length);
    pdf += `${i + 1} 0 obj\n${o}\nendobj\n`;
  });
  const xref = pdf.length;
  pdf +=
    "xref\n0 4\n0000000000 65535 f \n" +
    offsets
      .slice(1)
      .map((n) => `${String(n).padStart(10, "0")} 00000 n \n`)
      .join("") +
    `trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  const added = await request.post(
    "http://127.0.0.1:32124/api/score-attachment/add",
    {
      headers: {
        "X-Neothesia-Client": "web",
        "X-Neothesia-Song": song.contentId,
        "X-Neothesia-Name": encodeURIComponent("教师归档.pdf"),
      },
      data: Buffer.from(pdf),
    },
  );
  expect(added.ok()).toBeTruthy();
  const sheets = await added.json();
  await command({
    type: "updateScoreAttachment",
    content_id: song.contentId,
    id: sheets.active,
    action: "view",
    view: { page: 1, zoom: 150, fit: false, rotation: 90 },
  });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "曲目包", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "曲目包" });
  const pending = page.waitForEvent("download");
  await dialog.getByRole("button", { name: "导出当前曲目包" }).click();
  const download = await pending;
  const path = await download.path();
  expect(path).toBeTruthy();
  const fs = await import("node:fs/promises");
  const bytes = await fs.readFile(path!);
  expect(bytes.length).toBeGreaterThan(3_000_000);
  await dialog
    .getByLabel("导入曲目包文件")
    .setInputFiles({
      name: "教师资料.neopiece",
      mimeType: "application/zip",
      buffer: bytes,
    });
  await expect(
    dialog.getByText("教师归档.pdf（PDF）", { exact: true }),
  ).toBeVisible();
  await expect(dialog.getByText(/本地已有同一内容/)).toBeVisible();
  await command({
    type: "updateScoreAttachment",
    content_id: song.contentId,
    id: sheets.active,
    action: "view",
    view: { page: 1, zoom: 100, fit: true, rotation: 0 },
  });
  await dialog.getByRole("button", { name: "确认导入曲目包" }).click();
  await expect(
    dialog.getByRole("status").filter({ hasText: "已导入" }),
  ).toBeVisible();
  const merged = await command({
    type: "scoreAttachments",
    content_id: song.contentId,
  });
  expect(merged.attachments).toHaveLength(1);
  expect(merged.attachments[0].view.zoom).toBe(100);
  await dialog
    .getByLabel("导入曲目包文件")
    .setInputFiles({
      name: "教师资料.neopiece",
      mimeType: "application/zip",
      buffer: bytes,
    });
  await expect(
    dialog.getByText("教师归档.pdf（PDF）", { exact: true }),
  ).toBeVisible();
  await dialog.getByLabel("曲目包导入策略").selectOption("replace");
  await dialog.getByRole("button", { name: "确认导入曲目包" }).click();
  await expect(
    dialog.getByRole("status").filter({ hasText: "已导入" }),
  ).toBeVisible();
  const replaced = await command({
    type: "scoreAttachments",
    content_id: song.contentId,
  });
  expect(replaced.attachments).toHaveLength(1);
  expect(replaced.attachments[0].view.zoom).toBe(150);
  expect(replaced.attachments[0].view.rotation).toBe(90);
  await page.screenshot({ path: "../outputs/Neothesia-曲目资料携带.png" });
});
