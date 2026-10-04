import { test, expect } from "@playwright/test";
import fs from "node:fs";
test("资料模板、逐首预览、批量保存与冲突保留", async ({ page, request }) => {
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const songs = [];
  for (let i = 0; i < 3; i++) {
    const xml = `<score-partwise version="4.0"><work><work-title>批量课堂 ${i + 1}</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>${["C", "D", "E"][i]}</step><octave>5</octave></pitch><duration>4</duration><type>whole</type></note></measure></part></score-partwise>`;
    const song = await cmd({
      type: "importScore",
      name: `批量课堂 ${i + 1}.musicxml`,
      bytes: Array.from(Buffer.from(xml)),
      default_bpm: 99,
    });
    songs.push(song);
    await cmd({
      type: "updateSongMetadata",
      path: song.sourcePath,
      value: {
        title: `批量课堂 ${i + 1}`,
        composer: `原作 ${i + 1}`,
        artist: null,
        collection: null,
        difficulty: "原难度",
        tags: ["旧", "保留"],
        notes: `原备注 ${i + 1}`,
      },
    });
    await cmd({ type: "inspectSong", path: song.sourcePath, force: true });
  }
  await page.goto("/");
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByLabel("管理曲库搜索").fill("批量课堂");
  await expect(page.locator(".library-manager-table tbody tr")).toHaveCount(3);
  await page.getByLabel("选择本页曲目").check();
  await page
    .getByRole("button", { name: "批量修改资料 · 3", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "批量修改资料",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  await page.keyboard.press("Shift+Tab");
  expect(await dialog.evaluate(element => element.contains(document.activeElement))).toBeTruthy();
  await dialog.getByLabel("批量作曲家操作").selectOption("set");
  await dialog.getByLabel("批量作曲家内容").fill("课堂作曲家");
  await dialog.getByLabel("批量标签操作").selectOption("append");
  await dialog.getByLabel("批量标签内容").fill("学生, NEW, new");
  await dialog.getByLabel("批量备注操作").selectOption("append");
  await dialog.getByLabel("批量备注内容").fill("慢速练习，先唱后弹");
  await dialog.getByLabel("资料模板名称").fill("课堂练习资料");
  await dialog.getByRole("button", { name: "保存模板", exact: true }).click();
  await expect(dialog).toContainText("资料模板已保存");
  const registry = await cmd({ type: "metadataTemplates" });
  expect(
    registry.templates.some((t: { name: string }) => t.name === "课堂练习资料"),
  ).toBeTruthy();
  await cmd({
    type: "updateMetadataTemplate",
    name: "其他模板",
    rules: { difficulty: { mode: "set", value: "基础" } },
    expected: registry.revision,
  });
  await dialog.getByRole("button", { name: "更新模板", exact: true }).click();
  await expect(dialog.getByRole("alert")).toContainText("资料模板已变化");
  await expect(dialog.getByLabel("批量备注内容")).toHaveValue(
    "慢速练习，先唱后弹",
  );
  await dialog.getByRole("button", { name: "刷新模板", exact: true }).click();
  await dialog.getByRole("button", { name: "更新模板", exact: true }).click();
  await expect(dialog).toContainText("资料模板已保存");
  let releaseSecond:()=>void=()=>{};
  let announceSecond:()=>void=()=>{};
  const secondRequest=new Promise<void>(resolve=>{announceSecond=resolve;});
  const gate=new Promise<void>(resolve=>{releaseSecond=resolve;});
  let previewCount=0;
  await page.route("**/api/command",async route=>{const data=route.request().postDataJSON();if(data.type==="previewLibraryMetadata"&&++previewCount===2){announceSecond();await gate;}await route.continue();});
  await dialog.getByRole("button", { name: "读取修改预览", exact: true }).click();
  await secondRequest;
  await dialog.getByRole("button",{name:"停止当前批次",exact:true}).click();
  await page.keyboard.press("Escape");await expect(dialog).toBeVisible();releaseSecond();
  await expect(dialog).toContainText("已停止读取，保留 2 个文件的预览");
  await expect(dialog.locator(".metadata-batch-row")).toHaveCount(2);
  for(let i=0;i<3;i++)expect((await cmd({type:"readLibraryMetadata",path:songs[i].sourcePath,content_id:songs[i].contentId})).value.composer).toBe(`原作 ${i+1}`);
  await page.unroute("**/api/command");
  await dialog
    .getByRole("button", { name: "读取修改预览", exact: true })
    .click();
  await expect(dialog.locator(".metadata-batch-row")).toHaveCount(3);
  await expect(
    dialog.getByRole("button", { name: "保存所选修改 · 3", exact: true }),
  ).toBeEnabled();
  await dialog
    .locator(".metadata-batch-row")
    .first()
    .getByText("查看 3 个字段变化", { exact: true })
    .click();
  await page.screenshot({
    path: "../outputs/Neothesia-批量资料预览.png",
    fullPage: true,
  });
  const second = await cmd({
    type: "readLibraryMetadata",
    path: songs[1].sourcePath,
    content_id: songs[1].contentId,
  });
  await cmd({
    type: "updateSongMetadata",
    path: songs[1].sourcePath,
    value: { ...second.value, notes: "外部的新备注" },
  });
  const thirdPath = songs[2].sourcePath;
  expect(thirdPath.replaceAll("\\", "/")).toContain(
    "/work/cycle150-browser-final5/imports/",
  );
  const sidecar = thirdPath + ".neothesia.ron";
  const originalBytes=fs.readFileSync(sidecar);
  fs.writeFileSync(
    thirdPath,
    fs.readFileSync("../neothesia-engine/assets/basic-chords.mid"),
  );
  await dialog
    .getByRole("button", { name: "保存所选修改 · 3", exact: true })
    .click();
  await expect(dialog).toContainText("已保存 1 个文件，2 个未完成");
  await expect(dialog).toContainText("资料冲突");
  await expect(dialog).toContainText("文件内容已改变");
  const first = await cmd({
    type: "readLibraryMetadata",
    path: songs[0].sourcePath,
    content_id: songs[0].contentId,
  });
  expect(first.value).toMatchObject({
    title: "批量课堂 1",
    composer: "课堂作曲家",
    difficulty: "原难度",
    notes: "原备注 1\n慢速练习，先唱后弹",
  });
  expect(first.value.tags).toEqual(
    expect.arrayContaining(["旧", "保留", "学生"]),
  );
  expect(
    first.value.tags.filter((t: string) => t.toLowerCase() === "new"),
  ).toHaveLength(1);
  expect(
    (
      await cmd({
        type: "readLibraryMetadata",
        path: songs[1].sourcePath,
        content_id: songs[1].contentId,
      })
    ).value,
  ).toMatchObject({ composer: "原作 2", notes: "外部的新备注" });
  expect(fs.readFileSync(sidecar)).toEqual(originalBytes);
  await dialog
    .getByRole("button", { name: "读取修改预览", exact: true })
    .click();
  await expect(
    dialog.getByRole("button", { name: "保存所选修改 · 1", exact: true }),
  ).toBeEnabled();
  await dialog
    .getByRole("button", { name: "保存所选修改 · 1", exact: true })
    .click();
  await expect(dialog).toContainText("已保存 1 个文件，0 个未完成");
  expect(
    (
      await cmd({
        type: "readLibraryMetadata",
        path: songs[1].sourcePath,
        content_id: songs[1].contentId,
      })
    ).value.notes,
  ).toBe("外部的新备注\n慢速练习，先唱后弹");
  expect((await cmd({ type: "currentSong" })).contentId).toBe(
    songs[2].contentId,
  );
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole("button",{name:"批量修改资料 · 3",exact:true})).toBeFocused();
  await page
    .getByRole("button", { name: "批量修改资料 · 3", exact: true })
    .click();
  await dialog.getByLabel("选择资料模板").selectOption("课堂练习资料");
  await expect(dialog.getByLabel("批量备注内容")).toHaveValue(
    "慢速练习，先唱后弹",
  );
  await dialog.getByRole("button", { name: "移除模板", exact: true }).click();
  await expect(dialog).toContainText("模板已移除");
  expect(
    (await cmd({ type: "metadataTemplates" })).templates.some(
      (t: { name: string }) => t.name === "课堂练习资料",
    ),
  ).toBeFalsy();
  await expect(dialog.getByLabel("批量备注内容")).toHaveValue(
    "慢速练习，先唱后弹",
  );
});
