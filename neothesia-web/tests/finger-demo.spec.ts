import { test, expect } from "@playwright/test";

test("整段指法示范：真实时序、方案切换、琴键和关闭停止、不写入练习", async ({
  page,
  request,
}) => {
  test.setTimeout(60000);
  const cmd = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  await cmd({ type: "stopRoutine" });
  await cmd({ type: "pause" });
  const xml = `<score-partwise version="4.0"><work><work-title>整段指法示范</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>${["C", "E", "G", "F"].map((step) => `<note><pitch><step>${step}</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note>`).join("")}</measure></part></score-partwise>`;
  const song = await cmd({
    type: "importScore",
    bytes: Array.from(Buffer.from(xml)),
    name: "整段指法示范.musicxml",
    default_bpm: 120,
  });
  const track = song.notes[0].track;
  await cmd({
    type: "track",
    id: track,
    mode: "human",
    part: "right",
    visible: true,
  });
  const before = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  const fingers = (await cmd({ type: "currentSong" })).notes.map(
    (n: any) => n.finger,
  );
  await page.goto("/");
  await page.getByRole("button", { name: "指法建议", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "指法建议", exact: true });
  await dialog.getByLabel("指法起始小节").fill("1");
  await dialog.getByLabel("指法结束小节").fill("1");
  await dialog.getByRole("button", { name: "生成建议", exact: true }).click();
  await dialog.getByLabel("指法示范速度").selectOption("0.25");
  await dialog
    .getByRole("button", { name: "示范整个建议选段", exact: true })
    .click();
  await expect(dialog).toContainText("正在示范");
  await expect
    .poll(async () => (await cmd({ type: "fingerDemoState" })).position)
    .toBeGreaterThan(0.5);
  await expect(
    dialog.getByRole("button", { name: "下一个位置", exact: true }),
  ).toBeDisabled();
  const during = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  expect(during.position).toBe(before.position);
  expect(during.score).toEqual(before.score);
  expect(
    (await cmd({ type: "currentSong" })).notes.map((n: any) => n.finger),
  ).toEqual(fingers);
  await page.screenshot({ path: "../outputs/Neothesia-整段指法示范.png" });
  await dialog.getByRole("button", { name: /方案 2/ }).click();
  await expect
    .poll(async () => (await cmd({ type: "fingerDemoState" })).id)
    .toBeNull();
  await dialog
    .getByRole("button", { name: "示范整个建议选段", exact: true })
    .click();
  await expect(dialog).toContainText("正在示范");
  await cmd({ type: "note", pitch: 60, active: true, velocity: 80 });
  await expect(dialog).toContainText("示范已停止");
  await cmd({ type: "note", pitch: 60, active: false, velocity: 0 });
  await dialog.getByLabel("指法示范速度").selectOption("1");
  await dialog
    .getByRole("button", { name: "示范整个建议选段", exact: true })
    .click();
  await expect(dialog).toContainText("示范完成", { timeout: 6000 });
  await dialog
    .getByRole("button", { name: "示范整个建议选段", exact: true })
    .click();
  await expect(dialog).toContainText("正在示范");
  await dialog
    .getByRole("button", { name: "关闭指法建议", exact: true })
    .click();
  await expect
    .poll(async () => (await cmd({ type: "fingerDemoState" })).id)
    .toBeNull();
  const after = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  expect(after.position).toBe(before.position);
  expect(after.score).toEqual(before.score);
  expect(
    (await cmd({ type: "currentSong" })).notes.map((n: any) => n.finger),
  ).toEqual(fingers);
});
