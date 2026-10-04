import { test, expect } from "@playwright/test";

test("原谱多小节路径审阅、循环、命名恢复、备注改名与重新审阅", async ({
  page,
  request,
}) => {
  test.setTimeout(90000);
  test.skip(!process.env.NEOTHESIA_RANGE_TEST_DATA, "需要独立原谱选段服务");
  const command = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const pitches = ["C", "D", "E", "F", "G", "A", "B"];
  const xml = `<score-partwise version="4.0"><work><work-title>原谱选段课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1">${pitches.map((p, i) => `<measure number="${i + 1}">${i === 0 ? '<attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><direction><sound tempo="60"/></direction>' : ""}${i === 1 ? '<direction><direction-type><segno/></direction-type><sound segno="S"/></direction>' : i === 5 ? '<direction><direction-type><coda/></direction-type><sound coda="C"/></direction>' : ""}<note><pitch><step>${p}</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note>${i === 2 ? '<direction><direction-type><words>To Coda</words></direction-type><sound tocoda="C"/></direction>' : i === 4 ? '<direction><direction-type><words>D.S. al Coda</words></direction-type><sound dalsegno="S"/></direction>' : ""}</measure>`).join("")}</part></score-partwise>`;
  const song = await command({
    type: "importScore",
    bytes: Array.from(Buffer.from(xml)),
    name: "原谱选段课堂.musicxml",
    default_bpm: 120,
  });
  await command({ type: "mode", value: "wait" });
  await command({ type: "speed", value: 0.75 });
  await command({ type: "countIn", bars: 0 });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await expect(page.locator('.score-paper g.note[role="button"]')).toHaveCount(
    7,
  );
  await page.locator(".score-route summary").click();
  await page.getByRole("button", { name: "选段练习", exact: true }).click();
  const dialog = page.getByRole("dialog", {
    name: "原谱选段练习",
    exact: true,
  });
  await expect(
    dialog.getByRole("button", { name: "关闭原谱选段", exact: true }),
  ).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(
    dialog.getByRole("button", { name: "取消", exact: true }),
  ).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(
    dialog.getByRole("button", { name: "关闭原谱选段", exact: true }),
  ).toBeFocused();
  await dialog.getByLabel("起点原谱小节").selectOption("1");
  await dialog.getByLabel("起点演奏次数").selectOption("1");
  await dialog.getByLabel("终点原谱小节").selectOption("5");
  await expect(dialog.locator("tbody tr")).toHaveCount(7);
  await expect(dialog.locator("tbody tr td:nth-child(2)")).toHaveText([
    "第 2 小节",
    "第 3 小节",
    "第 4 小节",
    "第 5 小节",
    "第 2 小节",
    "第 3 小节",
    "第 6 小节",
  ]);
  await expect(
    dialog.getByRole("button", { name: "循环所选范围", exact: true }),
  ).toBeDisabled();
  await dialog.getByLabel("按上述实际路径练习，包含返回或跳过的小节").check();
  await dialog.getByLabel("原谱练习段名称").fill("再现到尾声");
  await dialog.getByLabel("原谱练习段备注").fill("保持连接，注意返回后换指");
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path:
        process.env.NEOTHESIA_SCREENSHOT_DIR +
        "/Neothesia-原谱选段路径审阅.png",
      fullPage: true,
    });
  await dialog
    .getByRole("button", { name: "保存原谱练习段", exact: true })
    .click();
  await expect(dialog).toHaveCount(0);
  const saved = (await command({ type: "passages" })).passages[0];
  expect(saved.scoreRange.firstVisit).toBe(1);
  expect(saved.scoreRange.lastVisit).toBe(7);
  expect(saved.speed).toBe(0.75);
  await page.getByRole("button", { name: "练习段落", exact: true }).click();
  const manager = page.getByRole("dialog", { name: "练习段落", exact: true });
  await manager
    .getByRole("button", { name: /再现到尾声.*原谱第 2 小节/ })
    .click();
  await expect(manager.getByLabel("保存段落起始小节")).toBeDisabled();
  await manager.getByLabel("练习段名称").fill("返回到尾声连接");
  await manager.getByLabel("练习段备注").fill("教师确认连接顺序");
  await manager
    .getByRole("button", { name: "更新练习段", exact: true })
    .click();
  await expect(manager.locator(".passage-list")).toContainText(
    "返回到尾声连接",
  );
  const edited = (await command({ type: "passages" })).passages[0];
  expect(edited.scoreRange).toEqual(saved.scoreRange);
  expect(edited.speed).toBe(0.75);
  await manager
    .getByRole("button", { name: /返回到尾声连接.*原谱第 2 小节/ })
    .click();
  await manager
    .getByRole("button", { name: "重新审阅原谱范围", exact: true })
    .click();
  await expect(dialog.locator("tbody tr")).toHaveCount(7);
  await dialog.getByLabel("起点演奏次数").selectOption("5");
  await expect(dialog.locator("tbody tr")).toHaveCount(3);
  await dialog.getByLabel("按上述实际路径练习，包含返回或跳过的小节").check();
  await dialog
    .getByRole("button", { name: "更新原谱练习段", exact: true })
    .click();
  await expect(dialog).toHaveCount(0);
  await expect(manager).toBeVisible();
  await expect(
    manager.getByRole("button", { name: "重新审阅原谱范围", exact: true }),
  ).toBeFocused();
  await manager.getByRole("button", { name: "练这一段", exact: true }).click();
  await expect(manager).toHaveCount(0);
  await expect
    .poll(
      async () =>
        (await (await request.get("http://127.0.0.1:32124/api/state")).json())
          .passage,
    )
    .toEqual({ start: 20, end: 32 });
  await command({ type: "load", path: song.sourcePath, title: song.title });
  await command({ type: "openPassage", id: saved.id });
  expect(
    (await command({ type: "passages" })).passages[0].scoreRange.firstVisit,
  ).toBe(5);
  const asset = await command({ type: "score" });
  const stale = await request.post("http://127.0.0.1:32124/api/command", {
    headers: { "X-Neothesia-Client": "web" },
    data: {
      type: "applyScoreRange",
      content_id: song.contentId,
      score_revision: asset.scoreRevision - 1,
      range: edited.scoreRange,
    },
  });
  expect(stale.status()).toBe(400);
  await page.setViewportSize({ width: 900, height: 800 });
  await page.reload();
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await page.locator(".score-route summary").click();
  await page.getByRole("button", { name: "选段练习", exact: true }).click();
  await expect(dialog).toBeVisible();
  expect(await dialog.evaluate((e) => e.scrollWidth <= e.clientWidth + 2)).toBe(
    true,
  );
  await expect(
    dialog.getByRole("button", { name: "循环所选范围", exact: true }),
  ).toBeEnabled();
  await dialog
    .getByRole("button", { name: "循环所选范围", exact: true })
    .click();
  await expect(dialog).toHaveCount(0);
  await expect
    .poll(
      async () =>
        (await (await request.get("http://127.0.0.1:32124/api/state")).json())
          .passage,
    )
    .toEqual({ start: 20, end: 24 });
});
