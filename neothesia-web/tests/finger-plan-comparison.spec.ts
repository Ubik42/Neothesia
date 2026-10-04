import { test, expect } from "@playwright/test";

test("双手方案逐音比较保持编辑、陈旧保护、局部采用与真实保存", async ({
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
  const xml = `<score-partwise version="4.0"><work><work-title>逐音对照 ${Date.now()}</work-title></work><part-list><score-part id="P1"><part-name>Right</part-name></score-part><score-part id="P2"><part-name>Left</part-name></score-part></part-list>${[4, 3].map((oct, i) => `<part id="P${i + 1}"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>${i ? "F" : "G"}</sign><line>${i ? 4 : 2}</line></clef></attributes>${["C", "E", "G", "F"].map((p) => `<note><pitch><step>${p}</step><octave>${oct}</octave></pitch><duration>1</duration><type>quarter</type></note>`).join("")}</measure></part>`).join("")}</score-partwise>`;
  await cmd({
    type: "importScore",
    bytes: Array.from(Buffer.from(xml)),
    name: `逐音对照 ${Date.now()}.musicxml`,
    default_bpm: 100,
  });
  const source = await cmd({ type: "currentSong" });
  const tracks = [...new Set(source.notes.map((n: any) => n.track))];
  for (let i = 0; i < tracks.length; i++)
    await cmd({
      type: "track",
      id: tracks[i],
      mode: "human",
      part: i === 0 ? "right" : "left",
      visible: true,
    });
  await cmd({
    type: "editFingersFor",
    content_id: source.contentId,
    edits: source.notes.map((n: any) => ({
      track_id: n.track,
      note_index: n.index,
      finger: null,
    })),
  });
  await page.goto("/");
  await page.getByRole("button", { name: "指法建议", exact: true }).click();
  await page.getByRole("button", { name: "双手一起审阅", exact: true }).click();
  const d = page.getByRole("dialog", { name: "双手指法审阅", exact: true });
  await d.getByRole("button", { name: "生成双手建议", exact: true }).click();
  const hand = d.locator(".finger-pair-hands>section").nth(1),
    input = hand.getByRole("spinbutton").first();
  const original = await input.inputValue();
  const compare = hand.getByRole("region", {
    name: "右手方案比较",
    exact: true,
  });
  await compare
    .getByRole("button", { name: "逐音比较方案", exact: true })
    .click();
  await compare.getByLabel("右手方案比较对照方案").selectOption("1");
  await expect(input).toHaveValue(original);
  expect(await compare.getByRole("checkbox").count()).toBeGreaterThan(0);
  await compare.getByRole("checkbox").first().check();
  await input.fill(original === "5" ? "4" : "5");
  await expect(compare.getByRole("status")).toContainText("已变化");
  await expect(
    compare.getByRole("button", { name: /采用选中差异/ }),
  ).toBeDisabled();
  await input.fill(original);
  await compare
    .getByRole("button", { name: "重新比较当前编辑结果", exact: true })
    .click();
  const changed = compare.getByRole("checkbox").first();
  const label = (await changed.getAttribute("aria-label"))!;
  const identity = label.split(" ").at(-1)!;
  const row = changed.locator("..", {}).locator("..");
  const target = await row.locator("td").nth(3).innerText();
  await row.getByRole("button", { name: "定位当前指法", exact: true }).click();
  await expect(
    hand.getByLabel(`右手指法 ${identity}`, { exact: true }),
  ).toBeFocused();
  await compare.getByRole("checkbox").first().check();
  await compare
    .getByRole("button", { name: "采用选中差异（1）", exact: true })
    .click();
  await expect(
    hand.getByLabel(`右手指法 ${identity}`, { exact: true }),
  ).toHaveValue(target);
  await expect(compare.getByRole("status")).toContainText("已变化");
  await d
    .getByRole("button", { name: "核对双手保存结果", exact: true })
    .click();
  // A mixed solution may conflict: select complete B through the comparator, keeping real review required.
  await compare
    .getByRole("button", { name: "重新比较当前编辑结果", exact: true })
    .click();
  await compare
    .getByRole("button", { name: "选择当前小节差异", exact: true })
    .click();
  if (await compare.getByRole("button", { name: /采用选中差异/ }).isEnabled())
    await compare.getByRole("button", { name: /采用选中差异/ }).click();
  await d
    .getByRole("button", { name: "核对双手保存结果", exact: true })
    .click();
  await expect(d.locator(".finger-pair-review")).toContainText("检查通过");
  const ack = d.getByLabel("确认双手保留与待复核位置");
  if (await ack.count()) await ack.check();
  await compare.scrollIntoViewIfNeeded();
  await page.screenshot({ path: "../outputs/Neothesia-逐音方案比较.png" });
  await page.setViewportSize({ width: 700, height: 900 });
  await compare.scrollIntoViewIfNeeded();
  await page.screenshot({ path: "../outputs/Neothesia-逐音方案比较-窄窗.png" });
  await d.getByRole("button", { name: "保存双手指法", exact: true }).click();
  await expect(d).toHaveCount(0);
  const saved = await cmd({ type: "currentSong" });
  const [track, index] = identity.split(":").map(Number);
  expect(
    saved.notes.find((n: any) => n.track === track && n.index === index).finger,
  ).toBe(Number(target));
  await cmd({ type: "undoFingersFor", content_id: saved.contentId });
});

test("单手比较局部采用固定重算并保留未采用位置", async ({ page, request }) => {
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const xml = `<score-partwise version="4.0"><work><work-title>逐音对照 ${Date.now()}</work-title></work><part-list><score-part id="P1"><part-name>Right</part-name></score-part><score-part id="P2"><part-name>Left</part-name></score-part></part-list>${[4, 3].map((oct, i) => `<part id="P${i + 1}"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>${i ? "F" : "G"}</sign><line>${i ? 4 : 2}</line></clef></attributes>${["C", "E", "G", "F"].map((p) => `<note><pitch><step>${p}</step><octave>${oct}</octave></pitch><duration>1</duration><type>quarter</type></note>`).join("")}</measure></part>`).join("")}</score-partwise>`;
  await cmd({
    type: "importScore",
    bytes: Array.from(Buffer.from(xml)),
    name: `逐音对照 ${Date.now()}.musicxml`,
    default_bpm: 100,
  });
  const source = await cmd({ type: "currentSong" });
  const tracks = [...new Set(source.notes.map((n: any) => n.track))];
  for (let i = 0; i < tracks.length; i++)
    await cmd({
      type: "track",
      id: tracks[i],
      mode: "human",
      part: i === 0 ? "right" : "left",
      visible: true,
    });
  await cmd({
    type: "editFingersFor",
    content_id: source.contentId,
    edits: source.notes.map((n: any) => ({
      track_id: n.track,
      note_index: n.index,
      finger: null,
    })),
  });
  await page.goto("/");
  await page.getByRole("button", { name: "指法建议", exact: true }).click();
  const d = page.getByRole("dialog", { name: "指法建议", exact: true });
  await d.getByRole("button", { name: "生成建议", exact: true }).click();
  const compare = d.getByRole("region", { name: "指法方案比较", exact: true });
  await compare
    .getByRole("button", { name: "逐音比较方案", exact: true })
    .click();
  await compare.getByLabel("指法方案比较对照方案").selectOption("1");
  expect(await compare.getByRole("checkbox").count()).toBeGreaterThan(0);
  const first = compare.getByRole("checkbox").first();
  const identity = (await first.getAttribute("aria-label"))!.split(" ").at(-1)!;
  const row = first.locator("..").locator("..");
  const target = await row.locator("td").nth(3).innerText();
  const index = Number(identity.split(":")[1]);
  const unchecked = d.locator(".fingering-table").getByRole("checkbox").last();
  const uncheckedLabel = (await unchecked.getAttribute("aria-label"))!;
  await unchecked.uncheck();
  await expect(compare.getByRole("status")).toContainText("已变化");
  await compare
    .getByRole("button", { name: "重新比较当前编辑结果", exact: true })
    .click();
  await compare.getByRole("checkbox").first().check();
  await compare
    .getByRole("button", { name: "采用选中差异（1）", exact: true })
    .click();
  await expect(
    d.getByLabel(`音符 ${index + 1} 建议手指`, { exact: true }),
  ).toHaveValue(target);
  await d.getByRole("button", { name: "按修改重新推荐", exact: true }).click();
  await expect(d.getByLabel(uncheckedLabel, { exact: true })).not.toBeChecked();
  await expect(
    d.getByLabel(`音符 ${index + 1} 建议手指`, { exact: true }),
  ).toHaveValue(target);
});
