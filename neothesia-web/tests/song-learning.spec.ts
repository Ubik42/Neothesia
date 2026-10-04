import { test, expect } from "@playwright/test";
test("曲目学习：目标、阶段历史、筛选排序、草稿冲突与批量保留", async ({
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
  const pieces: any[] = [];
  for (const [suffix, step] of [
    ["甲", "C"],
    ["乙", "E"],
  ]) {
    const xml = `<score-partwise version="4.0"><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>${step}</step><octave>5</octave></pitch><duration>4</duration><type>whole</type></note></measure></part></score-partwise>`;
    const song = await cmd({
      type: "importScore",
      name: `阶段学习课堂${suffix}.musicxml`,
      bytes: Array.from(Buffer.from(xml)),
      default_bpm: 95,
    });
    await cmd({ type: "inspectSong", path: song.sourcePath, force: true });
    await cmd({
      type: "updateSongMetadata",
      path: song.sourcePath,
      value: {
        title: `阶段学习课堂${suffix}`,
        composer: null,
        artist: null,
        collection: null,
        difficulty: null,
        tags: [],
        notes: null,
      },
    });
    const previous = (await cmd({ type: "libraryWorkspace" })).learning[
      song.contentId
    ];
    if (previous)
      await cmd({
        type: "saveSongLearning",
        path: song.sourcePath,
        content_id: song.contentId,
        value: null,
        expected: previous,
      });
    pieces.push(song);
  }
  await page.goto("/");
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByLabel("管理曲库搜索").fill("阶段学习课堂");
  const rows = page.locator(".library-manager-table tbody tr");
  await expect(rows).toHaveCount(2);
  await rows.filter({ hasText: "阶段学习课堂甲" }).click();
  await page.getByRole("button", { name: "学习目标", exact: true }).click();
  let dialog = page.getByRole("dialog", { name: "曲目学习", exact: true });
  await dialog.getByLabel("曲目学习阶段").selectOption("learning");
  await dialog.getByLabel("曲目目标日期").fill("2020-01-01");
  await dialog.getByLabel("曲目每周计划分钟").fill("75");
  await dialog.getByLabel("曲目目标速度").fill("100");
  await dialog.getByLabel("曲目学习目标").fill(`双手稳定演奏，保持连贯
重点处理第二乐句`);
  await dialog
    .getByRole("button", { name: "保存学习资料", exact: true })
    .click();
  await expect(dialog).toHaveCount(0);
  await expect(rows.filter({ hasText: "阶段学习课堂甲" })).toContainText(
    "已逾期",
  );
  await expect(
    page.getByRole("region", { name: "当前曲目学习资料" }),
  ).toContainText("每周计划 75 分钟");
  await rows.filter({ hasText: "阶段学习课堂乙" }).click();
  await page.getByRole("button", { name: "学习目标", exact: true }).click();
  dialog = page.getByRole("dialog", { name: "曲目学习", exact: true });
  await dialog.getByLabel("曲目目标日期").fill("2030-01-01");
  await dialog
    .getByRole("button", { name: "保存学习资料", exact: true })
    .click();
  await page.getByLabel("曲库管理排序").selectOption("learningDue");
  await expect(rows.first()).toContainText("阶段学习课堂甲");
  await page.getByLabel("曲库学习阶段筛选").selectOption("overdue");
  await expect(rows).toHaveCount(1);
  await rows.first().click();
  await page.getByRole("button", { name: "学习目标", exact: true }).click();
  dialog = page.getByRole("dialog", { name: "曲目学习", exact: true });
  await dialog.getByLabel("曲目学习目标").fill("未保存草稿");
  await dialog
    .getByRole("button", { name: "关闭曲目学习", exact: true })
    .click();
  await expect(dialog).toContainText("有未保存修改");
  await dialog.getByRole("button", { name: "继续编辑", exact: true }).click();
  const old = (await cmd({ type: "libraryWorkspace" })).learning[
    pieces[0].contentId
  ];
  await cmd({
    type: "saveSongLearning",
    path: pieces[0].sourcePath,
    content_id: pieces[0].contentId,
    expected: old,
    value: { ...old, stage: "polishing" },
  });
  await dialog
    .getByRole("button", { name: "保存学习资料", exact: true })
    .click();
  await expect(dialog).toContainText("学习资料已在其他位置修改");
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await dialog.getByRole("button", { name: "放弃修改", exact: true }).click();
  await page.getByRole("button", { name: "关闭曲库管理", exact: true }).click();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByLabel("曲库学习阶段筛选").selectOption("all");
  await page.getByLabel("管理曲库搜索").fill("阶段学习课堂");
  await expect(rows).toHaveCount(2);
  await page.getByLabel("选择本页曲目").check();
  await page.getByLabel("批量学习阶段").selectOption("mastered");
  await page
    .getByRole("button", { name: "设定学习阶段 · 2", exact: true })
    .click();
  await expect(page.locator(".library-learning-result")).toContainText("已设置 2 首为已掌握");
  const all = (await cmd({ type: "libraryWorkspace" })).learning;
  expect(all[pieces[0].contentId].weeklyMinutes).toBe(75);
  expect(all[pieces[0].contentId].goal).toContain("第二乐句");
  expect(all[pieces[0].contentId].transitions.length).toBe(3);
  await rows.filter({ hasText: "阶段学习课堂甲" }).click();
  await page.getByRole("button", { name: "学习目标", exact: true }).click();
  dialog = page.getByRole("dialog", { name: "曲目学习", exact: true });
  await dialog.locator("summary").click();
  await expect(dialog).toContainText("精练中");
  await page.screenshot({ path: "../outputs/Neothesia-曲目学习目标.png" });
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await page.getByLabel("曲库学习阶段筛选").selectOption("overdue");
  await expect(rows).toHaveCount(0);
});
