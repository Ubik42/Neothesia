import { test, expect } from "@playwright/test";
test("显示与电脑键盘：实际发音、释放、重映射、画布提示与持久保存", async ({
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
  const state = async () =>
    (await request.get("http://127.0.0.1:32124/api/state")).json();
  await cmd({ type: "stopRoutine" });
  await cmd({ type: "pause" });
  await cmd({ type: "panic" });
  const xml = `<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>${["C", "E", "G", "F"].map((step) => `<note><pitch><step>${step}</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note>`).join("")}</measure></part></score-partwise>`;
  await cmd({
    type: "importScore",
    bytes: Array.from(Buffer.from(xml)),
    name: "显示课堂.musicxml",
    default_bpm: 120,
  });
  const before = await state();
  await page.addInitScript(() => {
    const original = CanvasRenderingContext2D.prototype.fillText;
    (window as any).paintTexts = [];
    CanvasRenderingContext2D.prototype.fillText = function (
      text: string,
      ...args: any[]
    ) {
      const list = (window as any).paintTexts;
      list.push(text);
      if (list.length > 500) list.splice(0, 250);
      return (original as any).call(this, text, ...args);
    };
  });
  await page.goto("/");
  const blur = async () =>
    page.evaluate(() => {
      (document.activeElement as HTMLElement)?.blur();
    });
  const open = async () => {
    await page
      .getByRole("button", { name: "显示与电脑键盘设置", exact: true })
      .click();
    return page.getByRole("dialog", { name: "显示与电脑键盘", exact: true });
  };
  await expect(
    page.getByRole("button", { name: "显示与电脑键盘设置" }),
  ).toBeVisible();
  await expect(page.getByText("本地引擎已连接", {exact:true})).toBeVisible();
  await blur();
  await page.keyboard.down("a");
  await expect.poll(async () => (await state()).pressed).toContain(60);
  let d = await open();
  await expect.poll(async () => (await state()).pressed).toEqual([]);
  await page.keyboard.up("a");
  await d.getByLabel("音符名称格式").selectOption("solfege");
  await d.getByLabel("音符提示内容").selectOption("name");
  await d.getByLabel("键上名称范围").selectOption("all");
  await expect
    .poll(() => page.evaluate(() => (window as any).paintTexts.includes("Do4")))
    .toBeTruthy();
  await d.getByLabel("音符名称格式").selectOption("degree");
  await expect
    .poll(() => page.evaluate(() => (window as any).paintTexts.includes("1₄")))
    .toBeTruthy();
  await d.getByLabel("键盘声部配色").selectOption("accessible");
  await d.getByLabel("电脑键盘起始音区").selectOption("3");
  await d
    .getByRole("button", { name: "修改 C3 的电脑按键", exact: true })
    .click();
  await page.keyboard.press("s");
  await expect(d.getByRole("alert")).toContainText("已经分配");
  await page.keyboard.press("q");
  await expect(
    d.getByRole("button", { name: "修改 C3 的电脑按键", exact: true }),
  ).toContainText("Q");
  await d.getByLabel("显示左侧曲库").uncheck();
  await d.getByLabel("显示练习参数").uncheck();
  await page.screenshot({ path: "../outputs/Neothesia-显示与电脑键盘.png" });
  await d.getByRole("button", { name: "完成", exact: true }).click();
  await blur();
  await page.keyboard.press("a");
  expect((await state()).pressed).toEqual([]);
  await page.keyboard.down("q");
  await expect.poll(async () => (await state()).pressed).toContain(48);
  await page.keyboard.up("q");
  await expect.poll(async () => (await state()).pressed).toEqual([]);
  await page.getByLabel("键盘显示范围").selectOption("88");
  await page.getByLabel("预览小节数").selectOption("8");
  await page.reload();
  await expect(page.getByLabel("键盘显示范围")).toHaveValue("88");
  await expect(page.getByLabel("预览小节数")).toHaveValue("8");
  d = await open();
  await expect(d.getByLabel("音符名称格式")).toHaveValue("degree");
  await expect(d.getByLabel("显示左侧曲库")).not.toBeChecked();
  await expect(
    d.getByRole("button", { name: "修改 C3 的电脑按键", exact: true }),
  ).toContainText("Q");
  await d.getByRole("button", { name: "Z X C 行", exact: true }).click();
  await d.getByLabel("启用电脑键盘演奏").uncheck();
  await page.keyboard.press("Escape");
  await expect(d).toHaveCount(0);
  await blur();
  await page.keyboard.down("z");
  expect((await state()).pressed).toEqual([]);
  await page.keyboard.up("z");
  d = await open();
  await d.getByLabel("启用电脑键盘演奏").check();
  await d.getByRole("button", { name: "完成", exact: true }).click();
  await blur();
  await page.keyboard.down("z");
  await expect.poll(async () => (await state()).pressed).toContain(48);
  await page.keyboard.up("z");
  await expect.poll(async () => (await state()).pressed).toEqual([]);
  expect((await state()).score).toEqual(before.score);
  await page.evaluate(() =>
    localStorage.setItem(
      "neothesia-display-v1",
      JSON.stringify({ range: 123, keyCodes: Array(13).fill("KeyQ") }),
    ),
  );
  await page.reload();
  await expect(page.getByLabel("键盘显示范围")).toHaveValue("49");
  d = await open();
  await expect(
    d.getByRole("button", { name: "修改 C4 的电脑按键", exact: true }),
  ).toContainText("A");
});
