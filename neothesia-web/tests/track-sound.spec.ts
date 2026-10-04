import { test, expect } from "@playwright/test";
test("共用源通道的双轨声音独立保存、跳转、曲目包与备份", async ({
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
  const chunk = (events: number[]) => {
    const size = Buffer.alloc(4);
    size.writeUInt32BE(events.length);
    return Buffer.concat([Buffer.from("MTrk"), size, Buffer.from(events)]);
  };
  const midi = Buffer.concat([
    Buffer.from([77, 84, 104, 100, 0, 0, 0, 6, 0, 1, 0, 2, 1, 224]),
    chunk([0, 192, 0, 0, 144, 60, 80, 131, 96, 128, 60, 0, 0, 255, 47, 0]),
    chunk([0, 192, 0, 0, 144, 48, 75, 131, 96, 128, 48, 0, 0, 255, 47, 0]),
  ]);
  await cmd({ type: "importMidi", name: "双轨混音.mid", bytes: [...midi] });
  await cmd({ type: "trackSound", id: 0, sound: null });
  await page.goto("/");
  const controls = page.locator(".track-control");
  await expect(controls).toHaveCount(2);
  await controls.nth(0).locator(".track-sound summary").click();
  const volume = controls.nth(0).getByRole("slider");
  await volume.fill("45");
  await controls
    .nth(0)
    .getByRole("combobox", { name: "音轨 1声像" })
    .selectOption("32");
  const song = await (
    await request.get("http://127.0.0.1:32124/api/song")
  ).json();
  expect(song.soundPresets.length).toBeGreaterThan(0);
  const program = String(song.soundPresets[1].program);
  await controls
    .nth(0)
    .getByRole("combobox", { name: "音轨 1音色" })
    .selectOption(program);
  await controls.nth(0).getByRole("button", { name: "应用并保存" }).click();
  await expect(controls.nth(0).locator(".track-sound summary")).toContainText(
    "45%",
  );
  let current = await cmd({ type: "currentSong" });
  expect(current.trackSounds["0"]).toEqual({
    volume: 45,
    pan: 32,
    program: Number(program),
  });
  expect(current.trackSounds["1"]).toBeUndefined();
  await cmd({ type: "seek", position: 0.25 });
  await cmd({ type: "mode", value: "listen" });
  await cmd({ type: "countIn", bars: 0 });
  await cmd({ type: "play" });
  await cmd({ type: "pause" });
  current = await cmd({ type: "currentSong" });
  expect(current.trackSounds["0"].volume).toBe(45);
  const backup = await cmd({
    type: "practiceBackupSnapshot",
    selection: { plans: false, records: false, presets: true, days: 0 },
  });
  expect(backup.trackSounds[current.contentId]["0"].volume).toBe(45);
  const pkg = await cmd({ type: "exportPackage" });
  await cmd({ type: "trackSound", id: 0, sound: null });
  await cmd({ type: "importPackage", bytes: pkg.bytes, policy: "replace" });
  current = await cmd({ type: "currentSong" });
  expect(current.trackSounds["0"].volume).toBe(45);
  await page.reload();
  await controls.nth(0).locator(".track-sound summary").click();
  await expect(controls.nth(0).getByRole("slider")).toHaveValue("45");
  await expect(
    controls.nth(0).getByRole("combobox", { name: "音轨 1声像" }),
  ).toHaveValue("32");
  await page.screenshot({ path: "../outputs/Neothesia-音轨声音设置.png" });
  await controls.nth(0).getByRole("button", { name: "恢复原曲" }).click();
  await expect(controls.nth(0).locator(".track-sound summary")).toHaveText(
    "声音设置",
  );
});
