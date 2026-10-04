import { test, expect } from "@playwright/test";
test("双手错开起音的同键持音、音域交叠定位与真实慢练段", async ({
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
  const title = "两手交接课堂" + Date.now();
  const attrs =
    "<attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>";
  const note = (step: string, octave: number, duration: number) =>
    `<note><pitch><step>${step}</step><octave>${octave}</octave></pitch><duration>${duration}</duration><type>${duration === 4 ? "whole" : "quarter"}</type></note>`;
  const rest = (duration: number) =>
    `<note><rest/><duration>${duration}</duration></note>`;
  const xml = `<score-partwise version="4.0"><work><work-title>${title}</work-title></work><part-list><score-part id="L"><part-name>左手课堂</part-name></score-part><score-part id="R"><part-name>右手课堂</part-name></score-part></part-list><part id="L"><measure number="1">${attrs}${note("C", 4, 4)}</measure><measure number="2">${note("E", 5, 4)}</measure><measure number="3">${note("A", 3, 4)}</measure></part><part id="R"><measure number="1">${attrs}${rest(1)}${note("C", 4, 1)}${rest(2)}</measure><measure number="2">${note("C", 5, 4)}</measure><measure number="3">${note("A", 4, 4)}</measure></part></score-partwise>`;
  const loaded = await cmd({
    type: "importScore",
    bytes: [...Buffer.from(xml)],
    name: title + ".xml",
    default_bpm: 120,
  });
  const l = loaded.notes.find((n: any) => n.pitch === 76).track,
    r = loaded.notes.find((n: any) => n.pitch === 72).track;
  await cmd({
    type: "track",
    id: l,
    part: "left",
    mode: "human",
    visible: true,
  });
  await cmd({
    type: "track",
    id: r,
    part: "right",
    mode: "human",
    visible: true,
  });
  const before = await cmd({ type: "currentSong" });
  await page.goto("/");
  await page.getByRole("button", { name: "指法建议", exact: true }).click();
  await page.getByRole("button", { name: "双手一起审阅", exact: true }).click();
  const dialog = page.getByRole("dialog", {
    name: "双手指法审阅",
    exact: true,
  });
  await dialog.getByLabel("双手终点小节").fill("2");
  await dialog
    .getByRole("button", { name: "生成双手建议", exact: true })
    .click();
  await dialog
    .getByRole("button", { name: "核对双手保存结果", exact: true })
    .click();
  const review = dialog.getByRole("region", { name: "双手交接与音域审阅" });
  await expect(review).toContainText("同键持音重叠 1 处");
  await expect(review).toContainText("同时演奏 1 处");
  await expect(review).toContainText("500 毫秒");
  await expect(
    dialog.getByRole("button", { name: "保存双手指法", exact: true }),
  ).toBeDisabled();
  await review
    .locator(".finger-interaction-row")
    .first()
    .getByRole("button", { name: "查看两手指法位置", exact: true })
    .click();
  await expect(
    dialog.getByLabel(`右手指法 ${r}:0`, { exact: true }),
  ).toBeFocused();
  await review.getByLabel("双手交接筛选").selectOption("crossedRange");
  await expect(review.locator(".finger-interaction-row")).toHaveCount(1);
  await expect(review.locator(".finger-interaction-row")).toContainText(
    "第 2 小节",
  );
  await review.locator(".finger-interaction-row").getByRole("button",{name:"查看两手指法位置",exact:true}).click();
  await expect(dialog.getByLabel(`右手指法 ${r}:1`,{exact:true})).toBeFocused();
  await review.scrollIntoViewIfNeeded();
  await page.screenshot({ path: "../outputs/Neothesia-双手交接审阅.png" });
  await page.setViewportSize({ width: 700, height: 1100 });
  await review.scrollIntoViewIfNeeded();
  await page.screenshot({ path: "../outputs/Neothesia-双手交接审阅-窄窗.png" });
  await page.setViewportSize({ width: 1440, height: 960 });
  await dialog.getByLabel("确认双手保留与待复核位置").check();
  await dialog
    .getByRole("button", { name: "整理双手难点练习段", exact: true })
    .click();
  const builder = dialog.getByRole("region", { name: "指法难点练习段" });
  await expect(builder.getByLabel("难点段名称 1", { exact: true })).toHaveValue(
    /同键交接/,
  );
  await expect(builder.getByLabel("难点段名称 2", { exact: true })).toHaveValue(
    /音域交叠/,
  );
  await expect(builder.getByLabel("难点段速度 1", { exact: true })).toHaveValue(
    "50",
  );
  await builder
    .getByLabel("难点段名称 1", { exact: true })
    .fill(title + "同键");
  await builder
    .getByLabel("难点段名称 2", { exact: true })
    .fill(title + "交叠");
  await builder
    .getByRole("button", { name: "保存 2 个双手练习段", exact: true })
    .click();
  await expect(builder.getByRole("status")).toContainText("已保存 2 个练习段");
  const passages = (await cmd({ type: "passages" })).passages,
    first = passages.find((p: any) => p.name === title + "同键"),
    second = passages.find((p: any) => p.name === title + "交叠");
  expect(first.start).toBe(1);
  expect(first.end).toBe(1);
  expect(first.speed).toBe(0.5);
  expect(first.notes).toContain("同键持音重叠 500");
  expect(second.start).toBe(2);
  expect(second.settings.hands).toBe("both");
  expect(
    second.settings.tracks.filter((t: any) => t.mode === "Human"),
  ).toHaveLength(2);
  expect(
    (await cmd({ type: "currentSong" })).notes.map((n: any) => n.finger),
  ).toEqual(before.notes.map((n: any) => n.finger));
  page.once("dialog", (d) => d.accept());
  await dialog.getByRole("button", { name: "关闭", exact: true }).click();
  await page.getByLabel("关闭指法建议").click();
  await page.getByRole("button", { name: "练习段落", exact: true }).click();
  const parent = page.getByRole("dialog", { name: "练习段落", exact: true });
  await parent
    .locator(".passage-select")
    .filter({ hasText: title + "同键" })
    .click();
  await parent
    .locator("article")
    .filter({ hasText: title + "同键" })
    .getByRole("button", { name: "练这一段", exact: true })
    .click();
  const state = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  expect(state.hands).toBe("both");
  expect(state.speed).toBe(0.5);
  expect(state.rounds).toBe(3);
});
