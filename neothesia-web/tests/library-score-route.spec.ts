import { expect, test } from "@playwright/test";

test("曲库谱面预览反复位置，打开第二次经过并循环练习", async ({ page, request }) => {
  test.setTimeout(60000);
  const cmd = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const xml = `<score-partwise version="4.0"><work><work-title>曲库反复路线课堂</work-title></work><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>1</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes><barline location="left"><repeat direction="forward"/></barline><note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note></measure><measure number="2"><note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note><barline location="right"><repeat direction="backward"/></barline></measure><measure number="3"><note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration><type>quarter</type></note></measure></part></score-partwise>`;
  await cmd({ type: "importScore", bytes: [...Buffer.from(xml)], name: "曲库反复路线课堂.musicxml", default_bpm: 120 });
  const song = await cmd({ type: "currentSong" });
  await cmd({ type: "pairScore", content_id: song.contentId, name: "课堂可编辑原谱.musicxml", bytes: [...Buffer.from(xml)] });
  await cmd({ type: "inspectSong", path: song.sourcePath, force: true });
  const libraryScores = await cmd({ type: "libraryScores", path: song.sourcePath, content_id: song.contentId });
  const version = libraryScores.versions.find((v: any) => v.name === "课堂可编辑原谱.musicxml");
  expect(version?.status).toBe("ready");
  const otherRows = (await (await request.get("http://127.0.0.1:32124/api/library")).json()).songs;
  const other = otherRows.find((row: any) => row.path.endsWith("five-finger.mid"));
  expect(other).toBeTruthy();
  await cmd({ type: "load", path: other.path, title: other.title });
  const prior = await cmd({ type: "currentSong" });
  expect(prior.contentId).not.toBe(song.contentId);

  await page.goto("/");
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "曲库管理" });
  await dialog.getByRole("textbox", { name: "管理曲库搜索" }).fill("曲库反复路线课堂");
  await dialog.locator(".library-manager-table tbody tr").first().click();
  const panel = dialog.getByRole("region", { name: "曲目谱面版本" });
  const row = panel.getByLabel("版本名称 课堂可编辑原谱.musicxml").locator("..").locator("..");
  await row.getByRole("button", { name: "按原谱小节练习" }).click();
  await expect(row.getByText("5 次小节经过")).toBeVisible();
  expect((await cmd({ type: "currentSong" })).contentId).toBe(prior.contentId);
  const preview = await cmd({ type: "previewLibraryScoreRoute", path: song.sourcePath, content_id: song.contentId, id: version.id });
  const second = preview.route.visits.find((visit: any) => visit.measure === 0 && visit.ordinal === 2);
  expect(second).toBeTruthy();
  await row.getByLabel("曲库原谱小节演奏位置").selectOption(String(second.ordinal));
  await row.getByRole("button", { name: "打开并循环这次小节" }).click();
  await expect(dialog).not.toBeVisible();
  const opened = await cmd({ type: "currentSong" });
  expect(opened.contentId).toBe(song.contentId);
  const state = (await (await request.get("http://127.0.0.1:32124/api/state")).json());
  expect(state.passage.start).toBeCloseTo(second.start, 3);
  expect(state.passage.end).toBeCloseTo(second.end, 3);
  expect(state.position).toBeCloseTo(second.start, 3);
  await expect(page.locator(".score-route")).toContainText("演奏顺序");
  await page.reload();
  const reopened = (await (await request.get("http://127.0.0.1:32124/api/state")).json());
  expect(reopened.passage).toEqual(state.passage);
  await cmd({ type: "load", path: other.path, title: other.title });
  await cmd({ type: "load", path: song.sourcePath, title: song.title });
  const restored = (await (await request.get("http://127.0.0.1:32124/api/state")).json());
  expect(restored.passage).toEqual(state.passage);
  expect((await cmd({ type: "currentSong" })).hasScore).toBe(true);
});
