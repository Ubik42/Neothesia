import { test, expect } from "@playwright/test";
import { readFile } from "node:fs/promises";
function pdf() {
  const body = (n: number) =>
    `BT /F1 24 Tf 50 750 Td (Piano Study - Page ${n}) Tj ET\n0.5 w\n50 650 m 550 650 l S\n50 660 m 550 660 l S\n50 670 m 550 670 l S\n50 680 m 550 680 l S\n50 690 m 550 690 l S\n`;
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << /Font << /F1 7 0 R >> >> /Contents 4 0 R >>",
    `<< /Length ${body(1).length} >>\nstream\n${body(1)}endstream`,
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << /Font << /F1 7 0 R >> >> /Contents 6 0 R >>",
    `<< /Length ${body(2).length} >>\nstream\n${body(2)}endstream`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ];
  let s = "%PDF-1.4\n";
  const offsets = [0];
  objects.forEach((o, i) => {
    offsets.push(s.length);
    s += `${i + 1} 0 obj\n${o}\nendobj\n`;
  });
  const at = s.length;
  s += "xref\n0 8\n0000000000 65535 f \n";
  for (const n of offsets.slice(1))
    s += `${n.toString().padStart(10, "0")} 00000 n \n`;
  s += `trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n${at}\n%%EOF\n`;
  return Buffer.from(s);
}
test("页内小节框选、真实定位、跟随与网格失效",async({page,request})=>{
 const cmd=async(data:unknown)=>{const r=await request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data});expect(r.ok(),await r.text()).toBeTruthy();return r.json()};
 await cmd({type:"generate",spec:{tonic:5,tonality:"Major",minor_form:"Natural",pattern:"Scale",direction:"UpAndDown",hands:"Both",octaves:1,repetitions:1,tempo_bpm:120}});await cmd({type:"meter",value:null});const song=await cmd({type:"currentSong"});const papers=await cmd({type:"addScoreAttachment",content_id:song.contentId,name:"小节定位课堂.pdf",bytes:[...Buffer.concat([pdf(),Buffer.from("\n%"+Date.now())])]});await cmd({type:"updateScoreAttachment",content_id:song.contentId,id:papers.active,action:"view",view:{page:1,zoom:75,fit:false,rotation:0}});
 await page.goto("/");await page.getByRole("button",{name:"谱页",exact:true}).click();await page.getByRole("button",{name:"隐藏曲库",exact:true}).click();await page.getByRole("button",{name:"隐藏练习参数",exact:true}).click();const panel=page.getByRole("region",{name:"谱页浏览"});await expect(panel.locator("canvas")).toBeVisible();await panel.locator(".paper-mapping-editor summary").click();await panel.getByLabel("谱页对应小节").fill("2");await panel.getByRole("button",{name:"框选小节位置",exact:true}).click();const sheet=panel.locator(".paper-annotation-sheet"),b=await sheet.boundingBox();await page.mouse.move(b!.x+b!.width*.15,b!.y+b!.height*.06);await page.mouse.down();await page.mouse.move(b!.x+b!.width*.65,b!.y+b!.height*.14);await page.mouse.up();await expect(panel.getByRole("button",{name:"清除页内位置",exact:true})).toBeVisible();await panel.getByRole("button",{name:"保存此页对应",exact:true}).click();const region=panel.getByRole("button",{name:"谱页第 2 小节位置",exact:true});await expect(region).toBeVisible();const stored=await cmd({type:"scoreAttachments",content_id:song.contentId});const map=stored.attachments.find((a:any)=>a.id===papers.active).mapping;expect(map.anchors[0].region.width).toBeCloseTo(.5,2);await region.click();await expect.poll(async()=> (await cmd({type:"paperPracticeContext"})).measure).toBe(2);
 await panel.getByLabel("纸谱自动翻页").check();await cmd({type:"seekMeasure",measure:1});await cmd({type:"seekMeasure",measure:2});await expect(region).toHaveClass(/active/);await panel.getByRole("button",{name:"旋转 90°",exact:true}).click();await expect.poll(async()=>parseFloat(await region.evaluate(el=>(el as HTMLElement).style.width))).toBeCloseTo(map.anchors[0].region.height*100,2);await page.screenshot({path:"../outputs/Neothesia-纸谱小节位置.png"});
 await page.reload();await page.getByRole("button",{name:"谱页",exact:true}).click();await expect(region).toBeVisible();await region.click();await expect.poll(async()=>(await cmd({type:"paperPracticeContext"})).measure).toBe(2);
 await panel.locator(".paper-mapping-editor summary").click();await panel.getByRole("button",{name:"建立练习段",exact:true}).click();const builder=page.getByRole("dialog",{name:"从纸谱建立练习段"});await expect(builder.getByLabel("纸谱练习段名称")).toBeVisible();await builder.getByLabel("纸谱练习段名称").fill("纸谱连奏慢练");await builder.getByLabel("纸谱练习起始小节").fill("1");await builder.getByLabel("纸谱练习结束小节").fill("3");await builder.getByLabel("纸谱练习速度").fill("60");await builder.getByLabel("纸谱练习遍数").fill("3");await builder.getByLabel("纸谱练习要求").fill("慢练这一行，保持连奏");await builder.getByRole("button",{name:"保留草稿并关闭",exact:true}).click();await panel.getByRole("button",{name:"建立练习段",exact:true}).click();await expect(builder.getByLabel("纸谱练习要求")).toHaveValue("慢练这一行，保持连奏");
 const modified=structuredClone(map);modified.anchors[0].region.x+=.01;await cmd({type:"setPaperMapping",content_id:song.contentId,id:papers.active,mapping:modified});await builder.getByRole("button",{name:"保存练习段",exact:true}).click();await expect(builder.getByRole("alert")).toContainText("已改变");await builder.getByRole("button",{name:"重新核对条件",exact:true}).click();await expect(builder.getByRole("alert")).toHaveCount(0);await page.setViewportSize({width:700,height:900});await page.screenshot({path:"../outputs/Neothesia-纸谱建立练习段.png"});await builder.getByRole("button",{name:"保存练习段",exact:true}).click();await expect(builder.getByRole("status")).toContainText("已保存");const catalog=await cmd({type:"passages"});const preset=catalog.passages.find((p:any)=>p.name==="纸谱连奏慢练");expect(preset.start).toBe(1);expect(preset.end).toBe(3);expect(preset.speed).toBeCloseTo(.6,6);expect(preset.settings.rounds).toBe(3);await builder.getByRole("button",{name:"打开练习段",exact:true}).click();await expect(builder).toHaveCount(0);const state=await (await request.get("http://127.0.0.1:32124/api/state")).json();expect(state.speed).toBeCloseTo(.6,6);expect(state.rounds).toBe(3);expect(state.passage).toBeTruthy();
 await cmd({type:"meter",value:{numerator:3,denominator:4,pickup_ticks:0}});await page.reload();await page.getByRole("button",{name:"谱页",exact:true}).click();await expect(region).toBeDisabled();const rejected=await request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data:{type:"seekPaperMeasure",content_id:song.contentId,id:papers.active,measure:2}});expect(rejected.ok()).toBeFalsy();
});
