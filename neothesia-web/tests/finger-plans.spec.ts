import { test, expect } from "@playwright/test";
test("指法备选比较、固定重算、上下文手位、陈旧方案拒绝与撤销", async ({
  page,
  request,
}) => {
  test.setTimeout(60000);
  const raw = async (data: unknown) =>
    request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
  const command = async (data: unknown) => {
    const r = await raw(data);
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const xml = `<score-partwise version="4.0"><work><work-title>指法方案比较</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes>${["C", "E", "G", "F"].map((p) => `<note><pitch><step>${p}</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note>`).join("")}</measure></part></score-partwise>`;
  await command({
    type: "importScore",
    bytes: Array.from(Buffer.from(xml)),
    name: "指法方案.musicxml",
    default_bpm: 100,
  });
  let song = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  const track = song.notes[0].track,
    index = song.notes[0].index;
  await command({
    type: "track",
    id: track,
    mode: "human",
    part: "right",
    visible: true,
  });
  const req = {
    content_id: song.contentId,
    track,
    index,
    hand: "RightHand",
    profile: "Standard",
    range: [1, 1],
    keep_saved: false,
    pins: [],
  };
  const generated = await command({ type: "suggestFingerPlans", request: req });
  expect(generated.plans).toHaveLength(3);
  expect(
    new Set(
      generated.plans.map((p: any) =>
        p.proposals.map((n: any) => n.finger).join(","),
      ),
    ).size,
  ).toBe(3);
  await command({
    type: "editFingersFor",
    content_id: song.contentId,
    edits: [{ track_id: track, note_index: index, finger: 2 }],
  });
  const stale = await raw({
    type: "acceptFingerPlan",
    request: req,
    fingerprint: generated.fingerprint,
    edits: generated.plans[0].proposals.map((n: any) => ({
      track_id: n.track,
      note_index: n.index,
      finger: n.finger,
    })),
  });
  expect(stale.ok()).toBeFalsy();
  expect(await stale.text()).toContain("已变化");
  await command({ type: "undoFingersFor", content_id: song.contentId });
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "指法建议", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "指法建议" });
  await dialog.getByLabel("指法起始小节").fill("1");
  await dialog.getByLabel("指法结束小节").fill("1");
  await dialog.getByText("固定选段内已有指法", { exact: true }).click();
  await dialog.getByRole("button", { name: "生成建议", exact: true }).click();
  await expect(
    dialog.getByRole("group", { name: "备选指法方案" }).getByRole("button"),
  ).toHaveCount(3);
  await dialog.getByRole("button", { name: /方案 2/ }).click();
  await expect(dialog.getByRole("button", { name: /方案 2/ })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(
    dialog.getByRole("img", { name: "当前和弦及仍按住的音符指法" }),
  ).toBeVisible();
  await dialog.getByRole("button", { name: "下一个位置" }).click();
  await dialog.locator("tbody select").first().selectOption("2");
  await expect(
    dialog.getByRole("button", { name: /接受并保存/ }),
  ).toBeDisabled();
  await dialog.getByRole("button", { name: "按修改重新推荐" }).click();
  await expect(
    dialog.getByRole("button", { name: /接受并保存/ }),
  ).toBeEnabled();
  await expect(dialog.locator("tbody select").first()).toHaveValue("2");
  await page.screenshot({
    path: "../outputs/Neothesia-指法备选与手位.png",
    fullPage: true,
  });
  await dialog.getByRole("button", { name: /接受并保存/ }).click();
  await expect(dialog).toHaveCount(0);
  song = await (await request.get("http://127.0.0.1:32124/api/song")).json();
  expect(
    song.notes.find((n: any) => n.index === index && n.track === track).finger,
  ).toBe(2);
  await page.getByRole("button", { name: "指法建议", exact: true }).click();
  await page.getByRole("button", { name: "撤销上次指法修改" }).click();
  await page.keyboard.press("Escape");
  song = await (await request.get("http://127.0.0.1:32124/api/song")).json();
  expect(
    song.notes.find((n: any) => n.index === index && n.track === track).finger,
  ).toBeNull();
});
