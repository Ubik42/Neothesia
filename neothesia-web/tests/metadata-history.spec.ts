import { test, expect } from "@playwright/test";
import fs from "node:fs";
test("资料历史逐字段恢复、再次冲突、反向恢复与换内容拒绝", async ({
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
  const xml =
    '<score-partwise version="4.0"><work><work-title>历史课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>F</step><octave>5</octave></pitch><duration>4</duration><type>whole</type><notations><technical><fingering>3</fingering></technical></notations></note></measure></part></score-partwise>';
  const song = await cmd({
    type: "importScore",
    name: "历史课堂.musicxml",
    bytes: Array.from(Buffer.from(xml)),
    default_bpm: 105,
  });
  await cmd({
    type: "updateSongMetadata",
    path: song.sourcePath,
    value: {
      title: "历史课堂",
      composer: "原作",
      artist: null,
      collection: null,
      difficulty: null,
      tags: ["旧标签"],
      notes: "原备注",
    },
  });
  await cmd({ type: "inspectSong", path: song.sourcePath, force: true });
  const initial = (
    await cmd({
      type: "readLibraryMetadata",
      path: song.sourcePath,
      content_id: song.contentId,
    })
  ).value;
  await page.goto("/");
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByLabel("管理曲库搜索").fill("历史课堂");
  await page.locator(".library-manager-table tbody tr").first().click();
  await page.getByRole("button", { name: "编辑信息", exact: true }).click();
  const editor = page.getByRole("region", { name: "曲目信息编辑" });
  await editor.getByLabel("管理作曲家", { exact: true }).fill("新作曲家");
  await editor.getByLabel("管理标签", { exact: true }).fill("新标签");
  await editor.getByLabel("管理备注", { exact: true }).fill("第一次修改备注");
  await editor
    .getByRole("button", { name: "保存曲目信息", exact: true })
    .click();
  await expect(editor).toHaveCount(0);
  let current = (
    await cmd({
      type: "readLibraryMetadata",
      path: song.sourcePath,
      content_id: song.contentId,
    })
  ).value;
  await cmd({
    type: "updateSongMetadata",
    path: song.sourcePath,
    value: { ...current, artist: "后来演奏者", notes: "外部的新备注" },
  });
  await page.getByRole("button", { name: "资料变更历史", exact: true }).click();
  const dialog = page.getByRole("dialog", {
    name: "资料变更历史",
    exact: true,
  });
  await expect(dialog.locator(".metadata-history-list button")).toHaveCount(1);
  await dialog.locator(".metadata-history-list button").first().click();
  await expect(dialog.locator(".metadata-history-detail")).toContainText(
    "旧标签",
  );
  await expect(dialog.locator(".metadata-history-detail")).toContainText(
    "新标签",
  );
  await dialog
    .getByRole("button", { name: "预览恢复到修改前", exact: true })
    .click();
  const review = dialog.getByRole("region", { name: "资料恢复审阅" });
  await expect(review).toContainText("后来改过");
  await expect(
    review.getByRole("button", { name: /恢复所选字段/ }),
  ).toBeDisabled();
  await review.getByLabel("恢复作曲家操作").selectOption("keep");
  await review.getByLabel("恢复备注操作").selectOption("keep");
  await expect(
    review.getByRole("button", { name: "恢复所选字段 · 1", exact: true }),
  ).toBeEnabled();
  await page.screenshot({
    path: "../outputs/Neothesia-资料历史恢复审阅.png",
    fullPage: true,
  });
  current = (
    await cmd({
      type: "readLibraryMetadata",
      path: song.sourcePath,
      content_id: song.contentId,
    })
  ).value;
  await cmd({
    type: "updateSongMetadata",
    path: song.sourcePath,
    value: { ...current, artist: "再次更新演奏者" },
  });
  await review
    .getByRole("button", { name: "恢复所选字段 · 1", exact: true })
    .click();
  await expect(dialog).toContainText("预览后资料再次变化，尚未恢复");
  await expect(dialog.locator(".metadata-history-list button")).toHaveCount(1);
  await dialog
    .getByRole("button", { name: "预览恢复到修改前", exact: true })
    .click();
  await review.getByLabel("恢复作曲家操作").selectOption("keep");
  await review.getByLabel("恢复备注操作").selectOption("keep");
  await review
    .getByRole("button", { name: "恢复所选字段 · 1", exact: true })
    .click();
  await expect(dialog).toContainText("已恢复所选字段");
  await expect(dialog.locator(".metadata-history-list button")).toHaveCount(2);
  current = (
    await cmd({
      type: "readLibraryMetadata",
      path: song.sourcePath,
      content_id: song.contentId,
    })
  ).value;
  expect(current).toMatchObject({
    title: "历史课堂",
    composer: "新作曲家",
    artist: "再次更新演奏者",
    notes: "外部的新备注",
    tags: initial.tags,
  });
  const history = await cmd({
    type: "libraryMetadataHistory",
    path: song.sourcePath,
    content_id: song.contentId,
  });
  expect(history.records[0].restoredFrom).toBe(history.records[1].id);
  expect(history.records[0].fields).toEqual(["tags"]);
  await dialog.locator(".metadata-history-list button").first().click();
  await dialog
    .getByRole("button", { name: "预览恢复到修改前", exact: true })
    .click();
  await review
    .getByRole("button", { name: "恢复所选字段 · 1", exact: true })
    .click();
  await expect(dialog.locator(".metadata-history-list button")).toHaveCount(3);
  expect(
    (
      await cmd({
        type: "readLibraryMetadata",
        path: song.sourcePath,
        content_id: song.contentId,
      })
    ).value.tags,
  ).toEqual(["新标签"]);
  expect((await cmd({ type: "currentSong" })).contentId).toBe(song.contentId);
  await expect(dialog.getByRole("button",{name:"关闭资料变更历史",exact:true})).toBeEnabled();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "资料变更历史", exact: true }),
  ).toBeFocused();
  await page.getByRole("button", { name: "资料变更历史", exact: true }).click();
  await expect(dialog.locator(".metadata-history-list button")).toHaveCount(3);
  await dialog.locator(".metadata-history-list button").first().click();
  await dialog
    .getByRole("button", { name: "预览恢复到修改前", exact: true })
    .click();
  await expect(review.getByRole("button",{name:"恢复所选字段 · 1",exact:true})).toBeEnabled();
  expect(song.sourcePath.replaceAll("\\", "/")).toContain(
    "/work/cycle151-browser-final3/imports/",
  );
  const sidecar = song.sourcePath + ".neothesia.ron",
    bytes = fs.readFileSync(sidecar);
  fs.writeFileSync(
    song.sourcePath,
    fs.readFileSync("../neothesia-engine/assets/basic-chords.mid"),
  );
  await review
    .getByRole("button", { name: "恢复所选字段 · 1", exact: true })
    .click();
  await expect(dialog.getByRole("alert")).toContainText("文件内容已改变");
  expect(fs.readFileSync(sidecar)).toEqual(bytes);
  await dialog
    .getByRole("button", { name: "刷新资料历史", exact: true })
    .click();
  await expect(dialog).toContainText("历史仍可查看");
  await expect(dialog.locator(".metadata-history-list button")).toHaveCount(3);
  await expect(
    dialog.getByRole("button", { name: "预览恢复到修改前", exact: true }),
  ).toBeDisabled();
});
