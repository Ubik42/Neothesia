import { test, expect } from "@playwright/test";

test("分段拍号、弱起与显示编号预览后保存，并恢复原文件网格", async ({
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
      tonic: 7,
      tonality: "Major",
      minor_form: "Natural",
      pattern: "Scale",
      direction: "UpAndDown",
      hands: "Both",
      octaves: 2,
      repetitions: 4,
      tempo_bpm: 120,
    },
  });
  const original = await cmd({ type: "currentSong" });
  await page.goto("/");
  await page.getByRole("button", { name: "修正小节网格", exact: true }).click();
  const d = page.getByRole("dialog", { name: "修正小节网格", exact: true });
  await d.getByLabel("网格弱起拍数").fill("0.5");
  await d.getByLabel("第 1 段显示起始编号").fill("0");
  await d.getByRole("button", { name: "添加拍号变化", exact: true }).click();
  await d.getByLabel("第 2 段起始小节").fill("1");
  await d.getByRole("button", { name: "预览小节网格", exact: true }).click();
  await expect(d.getByRole("alert")).toBeVisible();
  await expect(
    d.getByRole("button", { name: "保存小节网格", exact: true }),
  ).toBeDisabled();
  await d.getByLabel("第 2 段起始小节").fill("3");
  await d.getByLabel("第 2 段每小节拍数").fill("3");
  await d.getByLabel("第 2 段显示起始编号").fill("20");
  await d.getByRole("button", { name: "添加不完整小节" }).click();
  await d.getByLabel("第 1 个短小节位置").fill("2");
  await d.getByLabel("第 1 个短小节实际拍数").fill("2");
  page.once("dialog", (dialog) => dialog.dismiss());
  await page.keyboard.press("Escape");
  await expect(d).toBeVisible();
  await d.getByRole("button", { name: "预览小节网格", exact: true }).click();
  const table = d.getByRole("region", { name: "小节网格预览" });
  const rows = table.locator("tbody tr");
  await expect(rows.first()).toContainText("0:00.250");
  await expect(rows.nth(2).locator("td").nth(1)).toHaveText("20");
  await expect(rows.nth(2).locator("td").nth(2)).toHaveText("3/4");
  await expect(rows.nth(2).locator("td").nth(3)).toContainText("0:01.250");
  await d.getByRole("button", { name: "定位导航小节 3", exact: true }).click();
  await expect(d.getByRole("status")).toContainText("已定位到导航小节 3");
  expect((await cmd({ type: "currentSong" })).meterMap).toEqual(
    original.meterMap,
  );
  await page.screenshot({ path: "../outputs/Neothesia-分段小节网格.png" });
  await page.setViewportSize({ width: 700, height: 900 });
  await page.screenshot({ path: "../outputs/Neothesia-分段小节网格-窄窗.png" });
  await d.getByRole("button", { name: "保存小节网格", exact: true }).click();
  await expect(d).toHaveCount(0);
  const saved = await cmd({ type: "currentSong" });
  expect(saved.meterMap.segments).toEqual([
    { measure: 1, numerator: 4, denominator: 4, display_start: 0 },
    { measure: 3, numerator: 3, denominator: 4, display_start: 20 },
  ]);
  expect(saved.meterMap.measure_lengths).toEqual([{ measure: 2, ticks: 960 }]);
  expect(saved.measures[2].startTick).toBe(1200);
  expect(saved.measures[0].displayNumber).toBe(0);
  expect(saved.measures[2].displayNumber).toBe(20);
  expect(saved.notes).toEqual(original.notes);
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.reload();
  await page.getByRole("button", { name: "修正小节网格", exact: true }).click();
  await expect(d.getByLabel("第 2 段显示起始编号")).toHaveValue("20");
  await expect(d.getByLabel("第 1 个短小节实际拍数")).toHaveValue("2");
  await d
    .getByRole("button", { name: "恢复文件原始网格", exact: true })
    .click();
  await expect(d.getByRole("status")).toContainText("文件原始网格");
  expect((await cmd({ type: "currentSong" })).meterMap.segments).toHaveLength(
    2,
  );
  await d.getByRole("button", { name: "保存小节网格", exact: true }).click();
  await expect(d).toHaveCount(0);
  const restored = await cmd({ type: "currentSong" });
  expect(restored.meterMap).toBeNull();
  expect(restored.measures).toEqual(original.measures);
});
