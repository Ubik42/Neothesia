import { test, expect } from "@playwright/test";
test("曲库菜单：跨选择收藏队列、复制、快捷键与信息编辑保护", async ({
  page,
  request,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const songs: any[] = [];
  for (const [suffix, step] of [
    ["甲", "C"],
    ["乙", "G"],
  ]) {
    const xml = `<score-partwise version="4.0"><work><work-title>菜单课堂${suffix}</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>${step}</step><octave>5</octave></pitch><duration>4</duration><type>whole</type></note></measure></part></score-partwise>`;
    const song = await cmd({
      type: "importScore",
      bytes: Array.from(Buffer.from(xml)),
      name: `菜单课堂${suffix}.musicxml`,
      default_bpm: 100,
    });
    await cmd({ type: "inspectSong", path: song.sourcePath, force: true });
    await cmd({
      type: "updateSongMetadata",
      path: song.sourcePath,
      value: {
        title: `菜单课堂${suffix}`,
        composer: null,
        artist: null,
        collection: null,
        difficulty: null,
        tags: [],
        notes: null,
      },
    });
    songs.push(song);
  }
  const current = (await cmd({ type: "currentSong" })).contentId;
  await page.goto("/");
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "曲库管理", exact: true });
  await page.keyboard.press("Control+f");
  await expect(page.getByLabel("管理曲库搜索")).toBeFocused();
  await page.getByLabel("管理曲库搜索").fill("菜单课堂");
  const rows = page.locator(".library-manager-table tbody tr");
  await expect(rows).toHaveCount(2);
  await rows.first().click();
  await page.keyboard.press("Control+a");
  await expect(rows.locator('input[type="checkbox"]:checked')).toHaveCount(2);
  await rows.first().click({ button: "right" });
  let menu = page.getByRole("menu", { name: "曲目操作" });
  await expect(menu).toContainText("已选择 2 首");
  await expect(
    menu.getByRole("menuitem", { name: "打开文件位置" }),
  ).toBeDisabled();
  await page.screenshot({ path: "../outputs/Neothesia-曲库文件菜单.png" });
  await menu.getByRole("menuitem", { name: "收藏所选 2 首" }).click();
  await expect(dialog.getByRole("status")).toContainText("已收藏 2 首");
  let collection = (await cmd({ type: "collection" })).songs;
  expect(
    songs.every(
      (s) => collection.find((h: any) => h.contentId === s.contentId)?.favorite,
    ),
  ).toBeTruthy();
  expect((await cmd({ type: "currentSong" })).contentId).toBe(current);
  await rows.first().focus();
  await page.keyboard.press("Shift+F10");
  menu = page.getByRole("menu", { name: "曲目操作" });
  await menu.getByRole("menuitem", { name: /加入队列/ }).click();
  await expect(dialog.getByRole("status")).toContainText("已加入队列 2 首");
  collection = (await cmd({ type: "collection" })).songs;
  expect(
    songs.every(
      (s) =>
        collection.find((h: any) => h.contentId === s.contentId)
          ?.queuePosition != null,
    ),
  ).toBeTruthy();
  await rows.first().click({ button: "right" });
  await menu.getByRole("menuitem", { name: "复制所选文件位置" }).click();
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  expect(songs.every((s) => copied.includes(s.sourcePath))).toBeTruthy();
  await rows.first().focus();
  await page.keyboard.press("Shift+F10");
  await page.keyboard.press("ArrowDown");
  await expect(menu.getByRole("menuitem", { name: "编辑信息" })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(menu).toHaveCount(0);
  await expect(rows.first()).toBeFocused();
  await expect(dialog).toBeVisible();
  await page.keyboard.press("F2");
  const editor = page.locator(".library-metadata-editor");
  await expect(editor).toBeVisible();
  await expect(
    dialog.getByRole("button", { name: "关闭曲库管理" }),
  ).toBeDisabled();
  await editor.getByLabel("管理曲名", { exact: true }).fill("未保存的菜单草稿");
  await rows.last().click();
  await expect(editor.getByLabel("管理曲名", { exact: true })).toHaveValue(
    "未保存的菜单草稿",
  );
  await page.keyboard.press("Escape");
  await expect(dialog).toBeVisible();
  await editor.getByRole("button", { name: "取消信息编辑" }).click();
  await expect(editor).toHaveCount(0);
  await rows.first().click();
  await rows.last().click({ button: "right" });
  await expect(menu).not.toContainText("已选择 2 首");
  await menu.getByRole("menuitem", { name: "取消收藏", exact: true }).click();
  await expect(dialog.getByRole("status")).toContainText("已取消收藏 1 首");
  await rows.first().focus();
  await page.keyboard.press("Control+a");
  await rows.first().click({ button: "right" });
  let release!: () => void, arrived!: () => void;
  const barrier = new Promise<void>((resolve) => (release = resolve)),
    arrival = new Promise<void>((resolve) => (arrived = resolve));
  await page.route("http://127.0.0.1:32124/api/command", async (route) => {
    const data = route.request().postDataJSON();
    if (data.type === "setLibraryCollection" && data.queued === false) {
      const response = await route.fetch();
      arrived();
      await barrier;
      await route.fulfill({ response });
    } else await route.continue();
  });
  await menu.getByRole("menuitem", { name: "移出队列", exact: true }).click();
  await arrival;
  await page
    .getByRole("button", { name: "停止收藏 / 队列处理", exact: true })
    .click();
  release();
  await expect(dialog.getByRole("status")).toContainText(
    "已停止；已移出队列 1 首",
  );
  collection = (await cmd({ type: "collection" })).songs;
  expect(
    songs.filter(
      (s) =>
        collection.find((h: any) => h.contentId === s.contentId)
          ?.queuePosition != null,
    ),
  ).toHaveLength(1);
});
