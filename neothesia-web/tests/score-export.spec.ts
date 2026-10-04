import { test, expect } from "@playwright/test";
import { readFile } from "node:fs/promises";

test("个人标记谱：反复分歧、原谱保留、下载与过期预览", async ({
  page,
  request,
}) => {
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
  const notes = ["C", "D", "E", "F"]
    .map(
      (step, i) =>
        `<note default-x="${42 + i * 40}"><pitch><step>${step}</step><octave>4</octave></pitch><duration>1</duration>${i === 0 ? "<notations><articulations><staccato/></articulations><technical><fingering>4</fingering></technical></notations><lyric><text>保留歌词</text></lyric>" : ""}</note>`,
    )
    .join("");
  const xml = `<score-partwise version="4.0"><work><work-title>反复指法课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><barline location="left"><repeat direction="forward"/></barline>${notes}<barline location="right"><repeat direction="backward"/></barline></measure></part></score-partwise>`;
  await cmd({
    type: "importScore",
    name: "反复指法课堂.musicxml",
    bytes: [...Buffer.from(xml)],
    default_bpm: 120,
  });
  const song = await cmd({ type: "currentSong" });
  const track = song.tracks.find((t: any) => t.notes === 8);
  expect(track).toBeTruthy();
  const edits = [1, 3, null, null, 2, 3, null, null].map(
    (finger, note_index) => ({ track_id: track.id, note_index, finger }),
  );
  await cmd({ type: "editFingersFor", content_id: song.contentId, edits });
  await cmd({
    type: "applyHands",
    content_id: song.contentId,
    hints: [0, 1, 4, 5].map((note_index) => ({
      track_id: track.id,
      note_index,
      part: note_index === 4 ? "RightHand" : "LeftHand",
    })),
  });
  await page.goto("/");
  await page.getByRole("button", { name: "乐谱", exact: true }).click();
  await page.getByRole("button", { name: "导出标记谱", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "导出个人标记谱" });
  await expect(dialog.locator(".score-export-summary")).toContainText(
    "指法谱音1",
  );
  await expect(dialog).toContainText(
    "第 1 次：指法 1，左手；第 2 次：指法 2，右手",
  );
  const download = page.waitForEvent("download");
  await dialog.getByRole("button", { name: "保存 MusicXML" }).click();
  const file = await download;
  const output = await readFile((await file.path())!, "utf8");
  expect(output).toContain("<fingering>4</fingering>");
  expect(output).toContain('<fingering placement="below">3</fingering>');
  expect(output).toContain("保留歌词");
  expect(output).toContain('default-x="42"');
  expect(output).toContain("staccato");
  expect(output.match(/<other-technical/g)?.length).toBe(1);
  await dialog.getByLabel("反复段落导出方式").selectOption("first");
  await expect(dialog.locator(".score-export-summary")).toContainText(
    "指法谱音2",
  );
  const firstDownload = page.waitForEvent("download");
  await dialog.getByRole("button", { name: "保存 MusicXML" }).click();
  const first = await firstDownload;
  const firstXml = await readFile((await first.path())!, "utf8");
  expect(firstXml).toContain('<fingering placement="below">1</fingering>');
  expect(firstXml.match(/<other-technical/g)?.length).toBe(2);
  await page.screenshot({
    path: "../outputs/Neothesia-个人指法谱导出.png",
    fullPage: true,
  });
  const exportCommand = {
    type: "exportAnnotatedScore",
    content_id: song.contentId,
    score_revision: song.scoreRevision,
    options: { fingers: true, hands: true, repeatPolicy: "first" },
  };
  const preview = await cmd(exportCommand);
  await cmd({
    type: "editFingersFor",
    content_id: song.contentId,
    edits: [{ track_id: track.id, note_index: 0, finger: 5 }],
  });
  const rejected = await request.post("http://127.0.0.1:32124/api/command", {
    headers: { "X-Neothesia-Client": "web" },
    data: {
      ...exportCommand,
      download: true,
      fingerprint: preview.fingerprint,
    },
  });
  expect(rejected.ok()).toBeFalsy();
  expect(await rejected.text()).toContain("刷新预览");
  await cmd({
    type: "editFingersFor",
    content_id: song.contentId,
    edits: [0, 4].map((note_index) => ({
      track_id: track.id,
      note_index,
      finger: 4,
    })),
  });
  const identicalPreview = await cmd(exportCommand);
  await cmd({
    type: "editFingersFor",
    content_id: song.contentId,
    edits: [0, 4].map((note_index) => ({
      track_id: track.id,
      note_index,
      finger: null,
    })),
  });
  const sameDigitRemoved = await request.post(
    "http://127.0.0.1:32124/api/command",
    {
      headers: { "X-Neothesia-Client": "web" },
      data: {
        ...exportCommand,
        download: true,
        fingerprint: identicalPreview.fingerprint,
      },
    },
  );
  expect(sameDigitRemoved.ok()).toBeFalsy();
  expect(await sameDigitRemoved.text()).toContain("刷新预览");
  // The exported XML remains playable, including its original repeat structure.
  await cmd({
    type: "importScore",
    name: "导出谱重新载入.musicxml",
    bytes: [...Buffer.from(firstXml)],
    default_bpm: 120,
  });
  const reloaded = await cmd({ type: "currentSong" });
  expect(reloaded.tracks.find((t: any) => t.notes === 8)).toBeTruthy();
});
