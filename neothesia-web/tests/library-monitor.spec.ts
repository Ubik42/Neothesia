import { test, expect } from "@playwright/test";
import { mkdir, writeFile, copyFile, rename } from "node:fs/promises";
import { join } from "node:path";
test("真实文件变化自动发现、改名恢复、暂停手动检查与断开目录恢复", async ({
  page,
  request,
}) => {
  test.setTimeout(120000);
  const data = process.env.NEOTHESIA_MONITOR_TEST_DATA;
  test.skip(!data, "需要隔离的曲库更新测试数据目录");
  const root = join(data!, "自动更新曲库");
  await mkdir(root, { recursive: true });
  await writeFile(join(data!, "library-folders.json"), JSON.stringify([root]));
  const first = join(root, "自动更新一.mid"),
    second = join(root, "自动更新改名.mid"),
    added = join(root, "自动更新二.mid");
  const command = async (value: unknown) => {
    const r = await request.post("http://127.0.0.1:32124/api/command", {
      headers: { "X-Neothesia-Client": "web" },
      data: value,
    });
    expect(r.ok(), await r.text()).toBeTruthy();
    return r.json();
  };
  const index = async () => await command({ type: "libraryWorkspace" });
  const row = async (path: string) => {
    const w = await index();
    return Object.values(w.entries).find((e: any) => e.path === path) as any;
  };
  const songs = async () =>
    await (await request.get("http://127.0.0.1:32124/api/library")).json();
  const state = async () =>
    await (await request.get("http://127.0.0.1:32124/api/state")).json();
  await page.goto("/");
  await expect(page.getByText("本地引擎已连接")).toBeVisible();
  await page.getByRole("button", { name: "曲库管理", exact: true }).click();
  await page.getByLabel("管理曲库搜索").fill("自动更新");
  await copyFile("../neothesia-engine/assets/five-finger.mid", first);
  await expect
    .poll(async () => Boolean((await row(first))?.contentId), {
      timeout: 20000,
    })
    .toBe(true);
  const identity = (await row(first)).contentId;
  await expect(page.getByTitle(first, { exact: true })).toBeVisible();
  const loaded = await command({
    type: "load",
    path: first,
    title: "自动更新一",
  });
  await command({ type: "favorite", enabled: true });
  await command({
    type: "finger",
    track: loaded.notes[0].track,
    index: loaded.notes[0].index,
    finger: 5,
  });
  await command({
    type: "assignLibrary",
    paths: [first],
    group: null,
    rating: 4,
  });
  await command({ type: "countIn", bars: 0 });
  await command({ type: "play" });
  await rename(first, second);
  await expect
    .poll(async () => Boolean((await row(second))?.contentId === identity), {
      timeout: 20000,
    })
    .toBe(true);
  await expect(page.getByTitle(second, { exact: true })).toBeVisible();
  expect((await row(first)).available).toBe(false);
  expect((await row(second)).rating).toBe(4);
  expect((await state()).status).toBe("playing");
  const recent = (await command({ type: "collection" })).songs.find(
    (h: any) => h.contentId === identity,
  );
  expect(recent.available).toBe(true);
  expect(recent.path).toBe(second);
  const reopened = await command({ type: "openRecent", content_id: identity });
  expect(reopened.sourcePath).toBe(second);
  expect(reopened.notes[0].finger).toBe(5);
  await page.getByLabel("自动更新曲库", { exact: true }).uncheck();
  await expect
    .poll(async () => (await command({ type: "libraryMonitorStatus" })).enabled)
    .toBe(false);
  const paused = (await command({ type: "libraryMonitorStatus" })).lastScan;
  const began = Date.now();
  await copyFile("../neothesia-engine/assets/basic-chords.mid", added);
  await expect
    .poll(
      async () => {
        const s = await command({ type: "libraryMonitorStatus" });
        expect(s.lastScan).toBe(paused);
        return Date.now() - began;
      },
      { timeout: 10000, intervals: [1000] },
    )
    .toBeGreaterThan(5500);
  expect((await songs()).songs.some((r: any) => r.path === added)).toBe(false);
  await page.getByRole("button", { name: "立即检查", exact: true }).click();
  await expect
    .poll(async () => Boolean((await row(added))?.contentId), {
      timeout: 20000,
    })
    .toBe(true);
  await expect(page.getByTitle(added, { exact: true })).toBeVisible();
  expect((await command({ type: "libraryMonitorStatus" })).enabled).toBe(false);
  await page.getByLabel("自动更新曲库", { exact: true }).check();
  const offline = join(data!, "离线目录");
  await rename(root, offline);
  await expect
    .poll(async () => Boolean((await row(second))?.available), {
      timeout: 15000,
    })
    .toBe(false);
  await expect.poll(async()=>(await command({type:"libraryMonitorStatus"})).errors.some((e:string)=>e.includes(root)),{timeout:15000}).toBe(true);
  await expect(page.locator(".library-monitor-status summary")).toBeVisible();
  await page.locator(".library-monitor-status summary").click();
  await expect(page.locator(".library-monitor-status")).toContainText(root);
  await page.getByLabel("曲库管理筛选").selectOption("missing");
  await expect(page.getByTitle(second, { exact: true })).toBeVisible();
  if (process.env.NEOTHESIA_SCREENSHOT_DIR)
    await page.screenshot({
      path:
        process.env.NEOTHESIA_SCREENSHOT_DIR + "/Neothesia-自动更新曲库.png",
      fullPage: true,
    });
  await rename(offline, root);
  await expect
    .poll(async () => Boolean((await row(second))?.available), {
      timeout: 20000,
    })
    .toBe(true);
  expect((await row(second)).contentId).toBe(identity);
  expect((await row(second)).rating).toBe(4);
  await copyFile("../neothesia-engine/assets/basic-chords.mid", second);
  await expect
    .poll(async () => (await row(second))?.contentId, { timeout: 20000 })
    .not.toBe(identity);
  expect((await row(second)).rating).toBe(0);
  const refused = await request.post("http://127.0.0.1:32124/api/command", {
    headers: { "X-Neothesia-Client": "web" },
    data: { type: "openRecent", content_id: identity },
  });
  expect(refused.ok()).toBe(false);
  await command({ type: "setLibraryMonitor", enabled: false });
  await expect
    .poll(async () => (await command({ type: "libraryMonitorStatus" })).enabled)
    .toBe(false);
});
test('目录断开与恢复需要完成真实后台检查',async({page,request})=>{
 test.setTimeout(45000);const data=process.env.NEOTHESIA_MONITOR_TEST_DATA;test.skip(!data,'需要隔离测试目录');
 const root=join(data!,'自动更新曲库'),offline=join(data!,'离线目录'),added=join(root,'自动更新二.mid');
 const command=async(value:unknown)=>{const r=await request.post('http://127.0.0.1:32124/api/command',{headers:{'X-Neothesia-Client':'web'},data:value});expect(r.ok()).toBeTruthy();return r.json();};
 const entry=async()=>Object.values((await command({type:'libraryWorkspace'})).entries).find((e:any)=>e.path===added) as any;
 await command({type:'setLibraryMonitor',enabled:true});await expect.poll(async()=>(await command({type:'libraryMonitorStatus'})).lastScan,{timeout:10000}).toBeGreaterThan(0);
 await page.goto('/');await page.getByRole('button',{name:'曲库管理',exact:true}).click();await page.getByLabel('管理曲库搜索').fill('自动更新');
 await rename(root,offline);const disconnectedAt=Date.now();
 await expect.poll(async()=>{const s=await command({type:'libraryMonitorStatus'});return s.lastScan>=disconnectedAt&&s.errors.some((e:string)=>e.includes(root+'：'));},{timeout:15000}).toBe(true);
 expect((await entry()).available).toBe(false);expect((await entry()).error).toBeTruthy();
 await expect(page.locator('.library-monitor-status summary')).toBeVisible();await page.locator('.library-monitor-status summary').click();await expect(page.locator('.library-monitor-status')).toContainText(root+'：');
 if(process.env.NEOTHESIA_SCREENSHOT_DIR)await page.screenshot({path:process.env.NEOTHESIA_SCREENSHOT_DIR+'/Neothesia-自动更新曲库.png',fullPage:true});
 await rename(offline,root);const restoredAt=Date.now();
 await expect.poll(async()=>{const s=await command({type:'libraryMonitorStatus'});const e=await entry();return s.lastScan>=restoredAt&&!s.errors.some((m:string)=>m.includes(root+'：'))&&e.available&&!e.error;},{timeout:20000}).toBe(true);
 await command({type:'setLibraryMonitor',enabled:false});
});
