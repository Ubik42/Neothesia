import { test, expect } from "@playwright/test";
test("原曲名称、每曲重命名与颜色不重置练习，可移植并恢复", async ({
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
  const name = Buffer.from("钢琴旋律");
  const events = [
    0,
    255,
    3,
    name.length,
    ...name,
    0,
    144,
    60,
    80,
    131,
    96,
    128,
    60,
    0,
    0,
    255,
    47,
    0,
  ];
  const length = Buffer.alloc(4);
  length.writeUInt32BE(events.length);
  const midi = Buffer.concat([
    Buffer.from([77, 84, 104, 100, 0, 0, 0, 6, 0, 0, 0, 1, 1, 224]),
    Buffer.from("MTrk"),
    length,
    Buffer.from(events),
  ]);
  await cmd({ type: "importMidi", name: "音轨管理.mid", bytes: [...midi] });
  await cmd({ type: "trackAppearance", id: 0, appearance: null });
  await cmd({ type: "mode", value: "wait" });
  await cmd({ type: "countIn", bars: 0 });
  await cmd({ type: "restart" });
  await page.goto("/");
  const control = page.locator(".track-control").first();
  await expect(control.locator(".track-name")).toHaveText("钢琴旋律");
  await control.locator(".track-appearance summary").click();
  await control.getByRole("textbox", { name: "音轨 1名称" }).fill("旋律慢练");
  await control
    .getByRole("button", { name: "音轨 1颜色 3", exact: true })
    .click();
  await control.getByRole("button", { name: "保存名称与颜色" }).click();
  await expect(control.locator(".track-name")).toHaveText("旋律慢练");
  await expect(page.locator(".track-legend")).toHaveText("旋律慢练");
  let song = await cmd({ type: "currentSong" });
  expect(song.tracks[0].sourceName).toBe("钢琴旋律");
  expect(song.tracks[0].color).toBe("#f1ad67");
  const backup = await cmd({
    type: "practiceBackupSnapshot",
    selection: { plans: false, records: false, presets: true, days: 0 },
  });
  expect(backup.trackAppearances[song.contentId]["0"].name).toBe("旋律慢练");
  await cmd({ type: "play" });
  const before = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  await cmd({
    type: "trackAppearance",
    id: 0,
    appearance: { name: "旋律慢练", color: "#f1ad67" },
  });
  const after = await (
    await request.get("http://127.0.0.1:32124/api/state")
  ).json();
  expect(after.status).toBe("playing");
  expect(after.position).toBeGreaterThanOrEqual(before.position);
  await cmd({ type: "pause" });
  await cmd({ type: "restart" });
  const pkg = await cmd({ type: "exportPackage" });
  expect(pkg.summary.trackAppearances[0].name).toBe("旋律慢练");
  await page.getByRole("button",{name:"曲目包",exact:true}).click();
  await page.getByLabel("导入曲目包文件").setInputFiles({name:"音轨管理.neopiece",mimeType:"application/zip",buffer:Buffer.from(pkg.bytes)});
  await expect(page.locator(".package-track-settings")).toContainText("旋律慢练");
  await expect(page.locator(".package-track-settings .track-color-dot")).toHaveCSS("background-color","rgb(241, 173, 103)");
  await page.getByRole("button",{name:"关闭曲目包"}).click();
  await cmd({ type: "trackAppearance", id: 0, appearance: null });
  await cmd({ type: "importPackage", bytes: pkg.bytes, policy: "replace" });
  song = await cmd({ type: "currentSong" });
  expect(song.tracks[0].name).toBe("旋律慢练");
  expect(song.tracks[0].color).toBe("#f1ad67");
  await page.reload();
  await control.locator(".track-appearance summary").click();
  await expect(
    control.getByRole("textbox", { name: "音轨 1名称" }),
  ).toHaveValue("旋律慢练");
  await expect(
    control.getByRole("button", { name: "音轨 1颜色 3", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await page.screenshot({ path: "../outputs/Neothesia-音轨名称与颜色.png" });
  await control.getByRole("button", { name: "恢复名称与配色" }).click();
  await expect(control.locator(".track-name")).toHaveText("钢琴旋律");
  await cmd({
    type: "applyPracticeBackup",
    backup,
    selection: { plans: false, records: false, presets: true, days: 0 },
    policy: "backup",
  });
  song = await cmd({ type: "currentSong" });
  expect(song.tracks[0].name).toBe("旋律慢练");
  await page.getByText("五指热身 · 从中央 C 开始",{exact:true}).click();
  await expect(control.locator(".track-name")).toHaveText("音轨 1");
  await control.locator(".track-appearance summary").click();
  await control.getByRole("textbox",{name:"音轨 1名称"}).fill("尚未保存的草稿");
  await control.locator(".track-sound summary").click();await control.getByRole("slider").fill("35");
  await page.getByText("C 大调音阶 · 一个八度",{exact:true}).click();
  await expect(control.locator(".track-name")).toHaveText("音轨 1");
  await control.locator(".track-appearance summary").click();await expect(control.getByRole("textbox",{name:"音轨 1名称"})).toHaveValue("音轨 1");
  await control.locator(".track-sound summary").click();await expect(control.getByRole("slider")).toHaveValue("100");
});
