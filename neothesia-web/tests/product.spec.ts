import { test, expect } from "@playwright/test";
test("练习段保存并恢复完整配置，技术练习方案可以重复使用", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "技术练习", exact: true }).click();
  await page.getByLabel("主音", { exact: true }).selectOption("7");
  await page.getByLabel("技术练习方案名称").fill("G大调日常练习");
  await page.getByRole("button", { name: "保存方案", exact: true }).click();
  await expect(
    page.getByLabel("常用技术练习方案").locator("option"),
  ).toHaveCount(2);
  await page.getByRole("button", { name: "生成并打开" }).click();
  await expect(page.locator("h1")).toHaveText("G大调 · 音阶");
  await page.getByRole("button", { name: "连续", exact: true }).click();
  await page.getByLabel("预备拍", { exact: true }).selectOption("0");
  await page.getByRole("button", { name: "练习段落", exact: true }).click();
  await page.getByLabel("练习段名称").fill("开头穿指");
  await page.getByLabel("保存段落结束小节").fill("1");
  await page.getByLabel("练习段备注").fill("拇指提前准备，不要抬高手腕");
  await page.getByRole("button", { name: "保存练习段", exact: true }).click();
  await expect(page.locator(".passage-list")).toContainText("开头穿指");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "等音", exact: true }).click();
  await page.getByLabel("预备拍", { exact: true }).selectOption("1");
  await page.getByRole("button", { name: "练习段落", exact: true }).click();
  await page.getByRole("button", { name: "练这一段", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "连续", exact: true }),
  ).toHaveClass(/selected/);
  await expect(page.getByLabel("预备拍", { exact: true })).toHaveValue("0");
  await expect(page.getByLabel("循环结束小节")).toHaveValue("1");
  await page.getByRole("button", { name: "技术练习", exact: true }).click();
  await page
    .getByLabel("常用技术练习方案")
    .selectOption({ label: "G大调日常练习" });
  await expect(page.getByLabel("主音", { exact: true })).toHaveValue("7");
  await page.keyboard.press("Escape");
});
test("指法建议可预览、修改、保存、撤销并恢复生成练习", async ({
  page,
  request,
}) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "技术练习", exact: true }).click();
  await page.getByRole("button", { name: "生成并打开" }).click();
  await expect(page.locator("h1")).toHaveText("C大调 · 音阶");
  const before = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  await page.getByRole("button", { name: "指法建议", exact: true }).click();
  await page.getByLabel("指法手跨度").selectOption("Compact");
  await page.getByRole("button", { name: "生成建议", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog.locator("tbody tr").first()).toBeVisible();
  await dialog.locator("tbody select").first().selectOption("5");
  await page.getByRole("button", { name: "按修改重新推荐", exact: true }).click();
  await expect(page.getByRole("button", {name:/接受并保存/})).toBeEnabled();
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-指法推荐.png",
      fullPage: true,
    });
  const preview = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  expect(preview.notes).toEqual(before.notes);
  await page.getByRole("button", { name: /接受并保存/ }).click();
  await expect(dialog).toHaveCount(0);
  const after = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  const firstTrack = after.tracks.find(
    (t: { part: string }) => t.part === "right",
  ).id;
  expect(
    after.notes.find(
      (n: { track: number; index: number }) =>
        n.track === firstTrack && n.index === 0,
    ).finger,
  ).toBe(5);
  await page.getByRole("button", { name: "指法建议", exact: true }).click();
  await page
    .getByRole("button", { name: "撤销上次指法修改", exact: true })
    .click();
  await expect
    .poll(async () => {
      const current = await (
        await request.get("http://127.0.0.1:32124/api/song")
      ).json();
      return current.notes.find(
        (n: { track: number; index: number }) =>
          n.track === firstTrack && n.index === 0,
      ).finger;
    })
    .toBe(1);
  await page.keyboard.press("Escape");
});
test("曲库分组、多选评级、文件索引、元数据与标签搜索", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("已索引");
  await page.getByLabel("管理曲库搜索").fill("五指热身");
  await expect(dialog.locator("tbody tr")).toHaveCount(1);
  await page
    .getByRole("checkbox", {
      name: "选择 五指热身 · 从中央 C 开始",
      exact: true,
    })
    .check();
  await page.getByRole("button", { name: /索引所选/ }).click();
  await expect(dialog.locator("tbody tr")).toContainText("已索引");
  await page.getByLabel("新分组名称").fill("每日练习");
  await page.getByRole("button", { name: "新建分组", exact: true }).click();
  await page.getByLabel("批量目标分组").selectOption({ label: "每日练习" });
  await page.getByRole("button", { name: "加入分组", exact: true }).click();
  await expect(dialog.locator("tbody tr")).toHaveCount(1);
  await page.getByLabel("批量曲目评级").selectOption("4");
  await page.getByRole("button", { name: "设定评级", exact: true }).click();
  await expect(dialog.locator("tbody tr")).toContainText("★★★★");
  await dialog.locator("tbody tr").click();
  await page.getByRole("button", { name: "编辑信息", exact: true }).click();
  await page.getByLabel("管理难度", { exact: true }).fill("入门一级");
  await page.getByLabel("管理标签", { exact: true }).fill("晨练，五指");
  await page.getByRole("button", { name: "保存曲目信息", exact: true }).click();
  await expect(dialog.locator("tbody tr")).toContainText("入门一级");
  await page.getByLabel("管理曲库搜索").fill("晨练");
  await expect(dialog.locator("tbody tr")).toHaveCount(1);
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-曲库管理.png",
      fullPage: true,
    });
  await page.keyboard.press("Escape");
  await page.getByPlaceholder("搜索曲名、作曲家").fill("晨练");
  await expect(page.getByRole("listitem")).toHaveCount(1);
});
