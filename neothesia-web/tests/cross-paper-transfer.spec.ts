import { test, expect } from "@playwright/test";

test("跨演奏纸谱迁移逐条对应小节、保存重开与过期拒绝", async ({
  page,
  request,
}) => {
  const raw = (data: unknown) =>
    request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
  const cmd = async (data: unknown) => {
    const r = await raw(data);
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const stamp = Date.now();
  const xml = (step: string) =>
    `<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">${[1, 2, 3].map((n) => `<measure number="${n}">${n === 1 ? "<attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>" : ""}<note><pitch><step>${step}</step><octave>4</octave></pitch><duration>4</duration></note></measure>`).join("")}</part></score-partwise>`;
  const png = async (label: string) =>
    Buffer.from(
      await page.evaluate((label) => {
        const c = document.createElement("canvas");
        c.width = 640;
        c.height = 360;
        const x = c.getContext("2d")!;
        x.fillStyle = "white";
        x.fillRect(0, 0, 640, 360);
        x.fillStyle = "black";
        x.font = "24px sans-serif";
        x.fillText(label, 30, 50);
        for (let i = 0; i < 5; i++) {
          x.beginPath();
          x.moveTo(30, 100 + i * 12);
          x.lineTo(610, 100 + i * 12);
          x.stroke();
        }
        return c.toDataURL("image/png").split(",")[1];
      }, label),
      "base64",
    );
  await cmd({
    type: "importScore",
    name: `教师演奏 ${stamp}.musicxml`,
    bytes: [...Buffer.from(xml("C"))],
    default_bpm: 80,
  });
  const sourceCid = (await cmd({ type: "currentSong" })).contentId;
  let papers = await cmd({
    type: "addScoreAttachment",
    content_id: sourceCid,
    name: `教师批注谱 ${stamp}.png`,
    bytes: [...(await png(`Teacher score ${stamp}`))],
    append: null,
  });
  const source = papers.active,
    sourceAsset = papers.attachments.find((b: any) => b.id === source).pages[0]
      .id;
  await cmd({
    type: "editPaperAnnotation",
    content_id: sourceCid,
    book: source,
    id: null,
    draft: {
      assetId: sourceAsset,
      page: 1,
      x: 0.2,
      y: 0.3,
      text: `换位练习 ${stamp}`,
      color: "yellow",
    },
    expected: null,
  });
  await cmd({
    type: "setPaperMapping",
    content_id: sourceCid,
    id: source,
    mapping: {
      grid: (await cmd({ type: "paperPracticeContext" })).grid,
      enabled: true,
      anchors: [{ measure: 1, page: 1 }],
    },
  });
  await cmd({
    type: "importScore",
    name: `练习演奏 ${stamp}.musicxml`,
    bytes: [...Buffer.from(xml("D"))],
    default_bpm: 90,
  });
  const targetCid = (await cmd({ type: "currentSong" })).contentId;
  expect(targetCid).not.toBe(sourceCid);
  papers = await cmd({
    type: "addScoreAttachment",
    content_id: targetCid,
    name: `练习谱 ${stamp}.png`,
    bytes: [...(await png(`Practice score ${stamp}`))],
    append: null,
  });
  const target = papers.active,
    targetAsset = papers.attachments.find((b: any) => b.id === target).pages[0]
      .id;
  const previewRequest = {
    type: "previewCrossPaperTransfer",
    content_id: targetCid,
    source_content_id: sourceCid,
    source,
    target,
  };
  const original = await cmd(previewRequest);
  await page.goto("/");
  await page.getByRole("button", { name: "谱面管理", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "谱面管理" });
  await dialog
    .getByRole("button", { name: "PDF / 图片谱面", exact: true })
    .click();
  const panel = dialog.locator(".paper-transfer-review");
  await panel.locator("summary").click();
  await expect(panel.getByLabel("迁移来源演奏")).toBeEnabled();
  await panel.getByLabel("迁移来源演奏").selectOption(sourceCid);
  await panel.getByLabel("迁移来源谱面").selectOption(source);
  await panel
    .getByRole("button", { name: "审阅迁移资料", exact: true })
    .click();
  await expect(
    panel.getByRole("region", { name: "迁移来源谱页" }).locator("canvas"),
  ).toBeVisible();
  await expect(
    panel.getByRole("region", { name: "迁移目标谱页" }).locator("canvas"),
  ).toBeVisible();
  await panel.getByLabel("迁移批注 1", { exact: true }).check();
  await panel.getByLabel("批注横向位置 1", { exact: true }).fill("40");
  await panel.getByLabel("迁移第 1 小节对应", { exact: true }).check();
  await expect(panel.getByLabel("来源小节 1 的目标导航小节")).toHaveValue("");
  await expect(
    panel.getByRole("button", { name: "确认迁移选中资料" }),
  ).toBeDisabled();
  await panel.getByLabel("来源小节 1 的目标导航小节").fill("2");
  await panel.getByRole("button", { name: "对照小节 1", exact: true }).click();
  await page.screenshot({
    path: "../outputs/Neothesia-跨演奏资料迁移.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 700, height: 900 });
  await page.screenshot({
    path: "../outputs/Neothesia-跨演奏资料迁移-窄窗.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 1440, height: 960 });
  await panel.getByRole("button", { name: "确认迁移选中资料" }).click();
  await expect(panel.getByText("已迁移 1 条批注、1 项小节对应。请核对后再启用自动翻页。", { exact: true })).toBeVisible();
  const saved = await cmd(previewRequest);
  expect(saved.source.annotations).toEqual(original.source.annotations);
  expect(saved.source.mapping).toEqual(original.source.mapping);
  expect(saved.target.annotations.at(-1)).toMatchObject({
    text: `换位练习 ${stamp}`,
    assetId: targetAsset,
    page: 1,
  });
  expect(saved.target.annotations.at(-1).x).toBeCloseTo(0.4);
  expect(saved.target.mapping).toMatchObject({
    enabled: false,
    anchors: [{ measure: 2, page: 1 }],
  });
  const stale = await raw({
    type: "applyCrossPaperTransfer",
    content_id: targetCid,
    request: {
      sourceContentId: sourceCid,
      source,
      target,
      baseline: original.baseline,
      annotations: [],
      anchors: [{ sourceMeasure: 1, measure: 3, page: 1 }],
    },
  });
  expect(stale.ok()).toBeFalsy();
  await page.reload();
  await page.getByRole("button", { name: "谱面管理", exact: true }).click();
  await dialog
    .getByRole("button", { name: "PDF / 图片谱面", exact: true })
    .click();
  await panel.locator("summary").click();
  await expect(panel.getByLabel("迁移来源演奏")).toBeEnabled();
  await panel.getByLabel("迁移来源演奏").selectOption(sourceCid);
  await panel.getByLabel("迁移来源谱面").selectOption(source);
  await panel
    .getByRole("button", { name: "审阅迁移资料", exact: true })
    .click();
  await expect(panel.getByLabel("迁移批注 1", { exact: true })).toBeDisabled();
  const mark = saved.target.annotations.at(-1);
  await cmd({
    type: "editPaperAnnotation",
    content_id: targetCid,
    book: target,
    id: mark.id,
    draft: { ...mark, text: `已编辑 ${stamp}` },
    expected: mark,
  });
  expect((await cmd(previewRequest)).target.annotations.at(-1).text).toBe(
    `已编辑 ${stamp}`,
  );
});
