import { test, expect } from "@playwright/test";
import fs from "node:fs";
test("曲目信息审阅：合并、再次冲突与换内容草稿保留", async ({
  page,
  request,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const cmd = async (data: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const xml = `<score-partwise version="4.0"><work><work-title>审阅课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>B</step><octave>4</octave></pitch><duration>4</duration><type>whole</type><notations><technical><fingering>2</fingering></technical></notations></note></measure></part></score-partwise>`;
  const song = await cmd({
    type: "importScore",
    name: "审阅课堂.musicxml",
    bytes: Array.from(Buffer.from(xml)),
    default_bpm: 97,
  });
  await cmd({ type: "inspectSong", path: song.sourcePath, force: true });
  const initial = {
    title: "审阅课堂",
    composer: "原作曲家",
    artist: null,
    collection: null,
    difficulty: null,
    tags: ["初始"],
    notes: null,
  };
  await cmd({
    type: "updateSongMetadata",
    path: song.sourcePath,
    value: initial,
  });
  await page.goto("/");
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByLabel("管理曲库搜索").fill("审阅课堂");
  const row = page.locator(".library-manager-table tbody tr").first();
  await row.click();
  await page.getByRole("button", { name: "编辑信息", exact: true }).click();
  let editor = page.getByRole("region", { name: "曲目信息编辑" });
  await editor.getByLabel("管理曲名", { exact: true }).fill("审阅课堂·我的");
  await editor.getByLabel("管理备注", { exact: true }).fill("我的练习备注");
  const theirs = {
    ...initial,
    title: "审阅课堂·外部",
    composer: "外部作曲家",
    tags: ["远程"],
  };
  await cmd({
    type: "updateSongMetadata",
    path: song.sourcePath,
    value: theirs,
  });
  await editor.getByRole("button", { name: "保存曲目信息" }).click();
  const review = editor.getByRole("region", { name: "曲目信息冲突审阅" });
  await expect(review).toBeVisible();
  await expect(
    review.getByRole("button", { name: "合并到草稿" }),
  ).toBeDisabled();
  await expect(editor.getByLabel("管理备注")).toHaveValue("我的练习备注");
  expect(
    (
      await cmd({
        type: "readLibraryMetadata",
        path: song.sourcePath,
        content_id: song.contentId,
      })
    ).value,
  ).toEqual(theirs);
  await review.getByLabel("冲突曲名采用").selectOption("mine");
  await page.screenshot({ path: "../outputs/Neothesia-曲目信息冲突审阅.png" });
  await review.getByRole("button", { name: "合并到草稿" }).click();
  await expect(editor.getByLabel("管理作曲家")).toHaveValue("外部作曲家");
  await expect(editor.getByLabel("管理标签")).toHaveValue("远程");
  await cmd({
    type: "updateSongMetadata",
    path: song.sourcePath,
    value: { ...theirs, notes: "另一处再次修改备注" },
  });
  await editor.getByRole("button", { name: "保存曲目信息" }).click();
  await expect(review).toBeVisible();
  await review.getByLabel("冲突备注采用").selectOption("latest");
  await review.getByRole("button", { name: "合并到草稿" }).click();
  await editor.getByRole("button", { name: "保存曲目信息" }).click();
  await expect(editor).toHaveCount(0);
  expect(
    (
      await cmd({
        type: "readLibraryMetadata",
        path: song.sourcePath,
        content_id: song.contentId,
      })
    ).value,
  ).toEqual({ ...theirs, title: "审阅课堂·我的", notes: "另一处再次修改备注" });
  await row.click();
  await page.getByRole("button", { name: "编辑信息", exact: true }).click();
  editor = page.getByRole("region", { name: "曲目信息编辑" });
  await editor.getByLabel("管理备注").fill("换曲后仍保留的草稿");
  expect(song.sourcePath.replaceAll("\\", "/")).toMatch(
    /\/work\/cycle148-browser-[^/]+\/imports\//,
  );
  fs.writeFileSync(
    song.sourcePath,
    fs.readFileSync("../neothesia-engine/assets/basic-chords.mid"),
  );
  await editor.getByRole("button", { name: "保存曲目信息" }).click();
  await expect(editor.getByRole("alert")).toContainText("文件内容已改变");
  await expect(editor.getByLabel("管理备注")).toHaveValue("换曲后仍保留的草稿");
  await editor.getByRole("button", { name: "复制信息草稿" }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toContain(
    "换曲后仍保留的草稿",
  );
  await editor.getByRole("button", { name: "取消信息编辑" }).click();
});
