import { test, expect } from "@playwright/test";
test("曲库字段、排除短语、成绩排序、组合筛选、键盘选择及保存视图", async ({
  page,
  request,
}) => {
  const command = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok()).toBeTruthy();
    return r.json();
  };
  const state = async () =>
    await (await request.get("http://127.0.0.1:32124/api/state")).json();
  const items = [];
  for (const [suffix, pitch, duration] of [
    ["A", "C", 1],
    ["B", "D", 2],
  ] as const) {
    const xml = `<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>2</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>${pitch}</step><octave>4</octave></pitch><duration>${duration}</duration></note></measure></part></score-partwise>`;
    await command({
      type: "importScore",
      name: `集训 ${suffix}.musicxml`,
      bytes: Array.from(Buffer.from(xml)),
      default_bpm: 120,
    });
    const song = await (
      await request.get("http://127.0.0.1:32124/api/song")
    ).json();
    items.push(song);
    await command({ type: "inspectSong", path: song.sourcePath, force: true });
    await command({
      type: "updateSongMetadata",
      path: song.sourcePath,
      value: {
        title: `集训 ${suffix}`,
        composer: suffix === "A" ? "作曲家甲" : "作曲家乙",
        artist: "课堂示范",
        collection: "第一册",
        difficulty: "入门",
        tags: ["复习", "晨练"],
        notes: suffix === "A" ? "保持手腕放松" : "稳定节奏",
      },
    });
    await command({
      type: "assignLibrary",
      paths: [song.sourcePath],
      group: null,
      rating: suffix === "A" ? 4 : 2,
    });
    if (suffix === "A") {
      await command({ type: "favorite", enabled: true });
      await command({ type: "countIn", bars: 0 });
      await command({ type: "mode", value: "wait" });
      await command({ type: "play" });
      await expect
        .poll(
          async () => {
            const s = await state();
            for (const pitch of s.required) {
              await command({
                type: "note",
                pitch,
                velocity: 90,
                active: true,
              });
              await command({
                type: "note",
                pitch,
                velocity: 0,
                active: false,
              });
            }
            return (
              (await command({ type: "collection" })).songs.find(
                (h: { contentId: string }) => h.contentId === song.contentId,
              )?.sessions ?? 0
            );
          },
          { timeout: 12000 },
        )
        .toBeGreaterThan(0);
    }
  }
  const history = (await command({ type: "collection" })).songs;
  expect(
    history.find(
      (h: { contentId: string }) => h.contentId === items[0].contentId,
    ).lastPracticed,
  ).toBeGreaterThan(0);
  expect(
    history.find(
      (h: { contentId: string }) => h.contentId === items[1].contentId,
    ).lastPracticed,
  ).toBeNull();
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  const table = page.locator(".library-manager-table");
  const rows = table.locator("tbody tr");
  await page.getByLabel("管理曲库搜索").fill("集训");
  await expect(rows).toHaveCount(2);
  await page.getByLabel("曲库搜索字段").selectOption("composer");
  await page.getByLabel("管理曲库搜索").fill("作曲家甲");
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("集训 A");
  await page.getByLabel("曲库搜索字段").selectOption("notes");
  await page.getByLabel("管理曲库搜索").fill("手腕");
  await expect(rows).toHaveCount(1);
  await page.getByLabel("曲库搜索字段").selectOption("all");
  await page.getByLabel("管理曲库搜索").fill('"集训" -"集训 B"');
  await expect(rows).toHaveCount(1);
  await page.getByLabel("管理曲库搜索").fill("集训");
  await page.getByLabel("曲库管理排序").selectOption("rating");
  await expect(rows.first()).toContainText("集训 A");
  await page.getByLabel("切换曲库排序方向").click();
  await expect(rows.first()).toContainText("集训 B");
  await page.getByLabel("曲库管理排序").selectOption("lastUsed");
  await expect(rows.first()).toContainText("集训 A");
  await expect(rows.first()).toContainText("1 次");
  await page.getByLabel("练习记录筛选").selectOption("played");
  await expect(rows).toHaveCount(1);
  await page.getByLabel("练习记录筛选").selectOption("unplayed");
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("集训 B");
  await page.getByLabel("练习记录筛选").selectOption("favorite");
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("集训 A");
  await page.getByLabel("练习记录筛选").selectOption("all");
  await page.getByLabel("最低评级").selectOption("3");
  await expect(rows).toHaveCount(1);
  await page.getByLabel("最低评级").selectOption("0");
  await page.getByLabel("最短时长分钟").fill("0.01");
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("集训 B");
  await page.getByLabel("最短时长分钟").fill("");
  await rows.first().focus();
  await page.keyboard.press("Shift+ArrowDown");
  await expect(rows.nth(1)).toBeFocused();
  await expect(page.locator(".library-manager-pagination")).toContainText(
    "已选择 2",
  );
  await page.getByRole("button", { name: "取消选择", exact: true }).click();
  await rows.first().focus();
  await page.keyboard.press("Control+a");
  await expect(page.locator(".library-manager-pagination")).toContainText(
    "已选择 2",
  );
  await page.getByLabel("曲库搜索字段").selectOption("collection");
  await page.getByLabel("管理曲库搜索").fill("第一册");
  await page.getByLabel("最低评级").selectOption("3");
  await page.getByLabel("关闭曲库管理").click();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await expect(page.getByLabel("曲库搜索字段")).toHaveValue("collection");
  await expect(rows).toHaveCount(1);
  await page.getByRole("button", { name: "清除筛选", exact: true }).click();
  await expect(page.getByLabel("管理曲库搜索")).toHaveValue("");
  await page.getByLabel("管理曲库搜索").fill("集训");
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path: process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-曲库筛选.png",
      fullPage: true,
    });
  await page.setViewportSize({ width: 1000, height: 800 });
  await expect(page.getByLabel("管理曲库搜索")).toBeVisible();
  expect(
    await page
      .locator(".library-manager")
      .evaluate((e) => e.scrollWidth <= e.clientWidth),
  ).toBeTruthy();
});
test("曲库跨页键盘定位与反向收缩连续选择", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.removeItem("neothesia-library-view-v1"),
  );
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  const table = page.locator(".library-manager-table");
  const first = table.locator("tbody tr").first();
  await first.focus();
  await page.keyboard.press("Control+End");
  await expect(page.getByLabel("下一页曲目")).toBeDisabled();
  const focused = table.locator("tbody tr:focus");
  await expect(focused).toHaveCount(1);
  const last = Number(await focused.getAttribute("data-index"));
  expect(last).toBeGreaterThan(50);
  await page.keyboard.press("Shift+PageUp");
  await expect(table.locator("tbody tr:focus")).toHaveAttribute(
    "data-index",
    String(last - 50),
  );
  await expect(page.locator(".library-manager-pagination")).toContainText(
    "已选择 51",
  );
  await page.keyboard.press("Shift+PageDown");
  await expect(table.locator("tbody tr:focus")).toHaveAttribute(
    "data-index",
    String(last),
  );
  await expect(page.locator(".library-manager-pagination")).toContainText(
    "已选择 1 首",
  );
  await page.keyboard.press("Control+Home");
  await expect(table.locator("tbody tr:focus")).toHaveAttribute(
    "data-index",
    "0",
  );
  await expect(page.getByLabel("上一页曲目")).toBeDisabled();
});
