import { test, expect } from "@playwright/test";

test("配对原谱连线和断奏参与指法备选，重开后仍可审阅", async ({ page, request }) => {
  const command = async (data: unknown) => {
    const response = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" }, data,
    });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  };
  const xml = `<score-partwise version="4.0"><work><work-title>连线断奏指法</work-title></work>
    <part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
    <part id="P1"><measure number="1"><attributes><divisions>1</divisions>
    <time><beats>4</beats><beat-type>4</beat-type></time></attributes>
    <note><pitch><step>C</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
    <notations><slur type="start" number="1"/></notations></note>
    <note><pitch><step>D</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
    <notations><slur type="stop" number="1"/></notations></note>
    <note><pitch><step>E</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice>
    <notations><articulations><staccato/></articulations></notations></note>
    <note><pitch><step>F</step><octave>4</octave></pitch><duration>1</duration><voice>1</voice></note>
    </measure></part></score-partwise>`;
  await command({ type: "importScore", bytes: Array.from(Buffer.from(xml)),
    name: "连线断奏指法.musicxml", default_bpm: 120 });
  const song = await (await request.get("http://127.0.0.1:32124/api/song")).json();
  const track = song.notes[0].track;
  await command({ type: "track", id: track, mode: "human", part: "right", visible: true });
  const req = { content_id: song.contentId, track, index: song.notes[0].index,
    hand: "RightHand", profile: "Standard", range: [1, 1], keep_saved: false, pins: [] };
  const first = await command({ type: "suggestFingerPlans", request: req });
  expect(first.scoreArticulations).toMatchObject({ paired: true, slurLinks: 1, staccatoNotes: 1 });
  expect(first.plans[0].proposals.some((note: { reason: string }) => note.reason.includes("谱面连线"))).toBe(true);
  await page.goto("/");
  await page.getByRole("button", { name: "指法建议", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "指法建议" });
  await dialog.getByLabel("指法起始小节").fill("1");
  await dialog.getByLabel("指法结束小节").fill("1");
  await dialog.getByRole("button", { name: "生成建议", exact: true }).click();
  await expect(dialog.locator("p.fingering-help[role=status]")).toContainText("1 处连线衔接、1 个断奏音符");
  await expect(dialog.locator(".fingering-table")).toContainText("谱面连线");

  const proposal = first.plans[0].proposals[0];
  await command({ type: "editFingersFor", content_id: song.contentId,
    edits: [{ track_id: proposal.track, note_index: proposal.index, finger: proposal.finger }] });
  await command({ type: "load", path: song.sourcePath, title: song.title });
  const reopened = await command({ type: "suggestFingerPlans", request: req });
  expect(reopened.scoreArticulations).toMatchObject({ paired: true, slurLinks: 1, staccatoNotes: 1 });
  expect(reopened.plans[0].proposals.find((note: { index: number }) => note.index === proposal.index).saved)
    .toBe(proposal.finger);
});
