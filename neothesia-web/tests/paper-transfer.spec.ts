import { test, expect } from "@playwright/test";
test("新版批注对照迁移、分页选择、小节冲突与整笔保护",async({page,request})=>{
  const raw=(data:unknown)=>request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data});
  const cmd=async(data:unknown)=>{const r=await raw(data);expect(r.ok(),await r.text()).toBeTruthy();return r.json();};
  const png=async(text:string)=>Buffer.from(await page.evaluate(text=>{const c=document.createElement("canvas");c.width=600;c.height=400;const x=c.getContext("2d")!;x.fillStyle="white";x.fillRect(0,0,600,400);x.fillStyle="black";x.font="24px sans-serif";x.fillText(text,50,80);return c.toDataURL("image/png").split(",")[1];},text),"base64");
  const stamp=Date.now(),oldBytes=await png(`Original score ${stamp}`),newBytes=await png(`Updated score ${stamp}`),second=await png(`Second page ${stamp}`);
  const xml=`<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note></measure></part></score-partwise>`;
  await cmd({type:"importScore",name:"新版谱面迁移.musicxml",bytes:[...Buffer.from(xml)],default_bpm:80});
  const cid=(await cmd({type:"currentSong"})).contentId;
  let papers=await cmd({type:"addScoreAttachment",content_id:cid,name:"原教师谱.png",bytes:[...oldBytes],append:null});const source=papers.active;
  papers=await cmd({type:"addScoreAttachment",content_id:cid,name:"原第二页.png",bytes:[...second],append:source});const src=papers.attachments.find((a:any)=>a.id===source);
  for(let i=0;i<22;i++)await cmd({type:"editPaperAnnotation",content_id:cid,book:source,id:null,draft:{assetId:src.pages[i===21?1:0].id,page:1,x:0.2,y:0.2,text:`旧谱批注 ${i+1}`,color:"yellow"},expected:null});
  papers=await cmd({type:"addScoreAttachment",content_id:cid,name:"新教师谱.png",bytes:[...newBytes],append:null});const target=papers.active;
  papers=await cmd({type:"addScoreAttachment",content_id:cid,name:"新第二页.png",bytes:[...second],append:target});const tgt=papers.attachments.find((a:any)=>a.id===target);
  await cmd({type:"editPaperAnnotation",content_id:cid,book:target,id:null,draft:{assetId:tgt.pages[0].id,page:1,x:0.1,y:0.1,text:"新版已有批注",color:"blue"},expected:null});
  const grid=(await cmd({type:"paperPracticeContext"})).grid;
  await cmd({type:"setPaperMapping",content_id:cid,id:source,mapping:{grid,enabled:true,anchors:[{measure:1,page:1}]}});
  await cmd({type:"setPaperMapping",content_id:cid,id:target,mapping:{grid,enabled:true,anchors:[{measure:1,page:2}]}});
  const pre=await cmd({type:"previewPaperTransfer",content_id:cid,source,target});
  const note={id:pre.source.annotations[0].id,assetId:tgt.pages[1].id,page:1,x:0.45,y:0.35};
  const transfer={source,target,baseline:pre.baseline,annotations:[note],anchors:[{measure:1,page:1}],replaceAnchors:false};
  expect((await raw({type:"applyPaperTransfer",content_id:cid,request:transfer})).ok()).toBeFalsy();
  expect((await cmd({type:"previewPaperTransfer",content_id:cid,source,target})).target.annotations).toHaveLength(1);
  await page.goto("/");await expect(page.getByText("本地引擎已连接",{exact:true})).toBeVisible();await page.getByRole("button",{name:"谱面管理",exact:true}).click();
  const d=page.getByRole("dialog",{name:"谱面管理"});await d.getByRole("button",{name:"PDF / 图片谱面",exact:true}).click();await d.locator(".paper-transfer-review summary").click();
  const panel=d.locator(".paper-transfer-review");await panel.getByLabel("迁移来源谱面").selectOption(source);await panel.getByRole("button",{name:"审阅迁移资料",exact:true}).click();
  await expect(panel.locator(".paper-transfer-note")).toHaveCount(20);await panel.getByLabel("迁移批注 1",{exact:true}).check();await panel.getByLabel("批注目标页 1",{exact:true}).fill("2");
  await panel.getByLabel("批注横向位置 1",{exact:true}).fill("45");await panel.getByLabel("批注纵向位置 1",{exact:true}).fill("35");
  await expect(panel.getByRole("region",{name:"迁移目标谱页"}).locator("canvas")).toBeVisible();
  await panel.getByRole("button",{name:"下一页批注",exact:true}).click();await expect(panel.locator(".paper-transfer-note")).toHaveCount(2);await panel.getByLabel("迁移批注 22",{exact:true}).check();
  await panel.getByLabel("迁移第 1 小节对应",{exact:true}).check();await panel.getByLabel("小节对应目标页 1",{exact:true}).fill("1");await panel.getByLabel("替换目标同小节对应").check();
  await panel.getByRole("button",{name:"上一页批注",exact:true}).click();await expect(panel.getByLabel("迁移批注 1",{exact:true})).toBeChecked();
  await panel.getByRole("region",{name:"迁移目标谱页"}).locator("canvas").scrollIntoViewIfNeeded();await page.screenshot({path:"../outputs/Neothesia-新版谱面资料迁移.png",fullPage:true});
  await panel.getByRole("button",{name:"确认迁移选中资料",exact:true}).click();await expect(panel.getByRole("status")).toContainText("已迁移 2 条批注、1 项小节对应");
  const result=await cmd({type:"previewPaperTransfer",content_id:cid,source,target});expect(result.source.annotations).toHaveLength(22);expect(result.target.annotations).toHaveLength(3);expect(result.target.mapping).toMatchObject({enabled:false,anchors:[{measure:1,page:1}]});
  const migrated=result.target.annotations.find((n:any)=>n.text==="旧谱批注 1");
  expect(migrated).toMatchObject({assetId:tgt.pages[1].id,page:1});expect(migrated.x).toBeCloseTo(0.45);expect(migrated.y).toBeCloseTo(0.35);
  await panel.getByRole("button",{name:"审阅迁移资料",exact:true}).click();await expect(panel.getByLabel("迁移批注 1",{exact:true})).toBeDisabled();
  expect((await cmd({type:"applyPaperTransfer",content_id:cid,request:{...transfer,baseline:result.baseline,anchors:[]}})).added).toBe(0);
  expect((await raw({type:"applyPaperTransfer",content_id:cid,request:{...transfer,baseline:result.baseline,anchors:[],annotations:[note,{...note,id:pre.source.annotations[1].id,x:2}]}})).ok()).toBeFalsy();
  expect((await raw({type:"applyPaperTransfer",content_id:cid,request:{...transfer,anchors:[],annotations:[note]}})).ok()).toBeFalsy();
  const after=await cmd({type:"previewPaperTransfer",content_id:cid,source,target});expect(after.target.annotations).toHaveLength(3);
});

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
test("PDF 不同页批注迁移与目标页数核对",async({page,request})=>{
 const cmd=async(data:unknown)=>{const r=await request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data});expect(r.ok(),await r.text()).toBeTruthy();return r.json();};
 const cid=(await cmd({type:"currentSong"})).contentId,stamp=Date.now(),sourceBytes=Buffer.concat([pdf(),Buffer.from(`\n% source ${stamp}`)]),targetBytes=Buffer.concat([pdf(),Buffer.from(`\n% target ${stamp}`)]);
 let papers=await cmd({type:"addScoreAttachment",content_id:cid,name:"旧 PDF.pdf",bytes:[...sourceBytes],append:null});const source=papers.active,asset=papers.attachments.find((b:any)=>b.id===source).pages[0].id;
 await cmd({type:"editPaperAnnotation",content_id:cid,book:source,id:null,draft:{assetId:asset,page:2,x:0.2,y:0.3,text:"第二页手位",color:"green"},expected:null});
 papers=await cmd({type:"addScoreAttachment",content_id:cid,name:"新 PDF.pdf",bytes:[...targetBytes],append:null});const target=papers.active;
 await page.goto("/");await expect(page.getByText("本地引擎已连接",{exact:true})).toBeVisible();await page.getByRole("button",{name:"谱面管理",exact:true}).click();
 const d=page.getByRole("dialog",{name:"谱面管理"});await d.getByRole("button",{name:"PDF / 图片谱面",exact:true}).click();await d.locator(".paper-transfer-review summary").click();const panel=d.locator(".paper-transfer-review");
 await panel.getByLabel("迁移来源谱面").selectOption(source);await panel.getByRole("button",{name:"审阅迁移资料",exact:true}).click();await expect(panel.getByRole("region",{name:"迁移来源谱页"}).locator("canvas")).toHaveAttribute("aria-label","谱面第 2 页");
 await expect(panel.getByLabel("批注目标页 1")).toHaveAttribute("max","2");await panel.getByLabel("迁移批注 1",{exact:true}).check();await panel.getByLabel("批注目标页 1").fill("3");await expect(panel.getByRole("button",{name:"确认迁移选中资料"})).toBeDisabled();await panel.getByLabel("批注目标页 1").fill("1");await panel.getByRole("button",{name:"确认迁移选中资料"}).click();await expect(panel.getByRole("status")).toContainText("已迁移 1 条批注");
 const result=await cmd({type:"previewPaperTransfer",content_id:cid,source,target});expect(result.target.annotations[0]).toMatchObject({page:1,text:"第二页手位",assetId:papers.attachments.find((b:any)=>b.id===target).pages[0].id});
});
