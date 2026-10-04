import { test, expect } from "@playwright/test";
import { writeFile, mkdir, rename } from "node:fs/promises";
import { resolve } from "node:path";
test("图片原件更新预览、新版本保留旧批注、搬家关联与过期保护",async({page,request})=>{
  const raw=(data:unknown)=>request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data});
  const cmd=async(data:unknown)=>{const r=await raw(data);expect(r.ok(),await r.text()).toBeTruthy();return r.json();};
  const png=async(text:string)=>Buffer.from(await page.evaluate(text=>{const c=document.createElement("canvas");c.width=600;c.height=400;const x=c.getContext("2d")!;x.fillStyle="white";x.fillRect(0,0,600,400);x.fillStyle="black";x.font="30px sans-serif";x.fillText(text,50,100);return c.toDataURL("image/png").split(",")[1];},text),"base64");
  const stamp=Date.now();const first=await png(`Teacher score - Original ${stamp}`),second=await png("Page 2"),updated=await png("Teacher score - Updated");
  const folder=resolve(`../work/cycle160-source-files-${stamp}`);await mkdir(folder,{recursive:true});const source=resolve(folder,"teacher.png"),moved=resolve(folder,"teacher-moved.png");await writeFile(source,first);
  const xml=`<score-partwise><part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list><part id="P1"><measure number="1"><attributes><divisions>1</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note></measure></part></score-partwise>`;
  await cmd({type:"importScore",name:"原件谱页课堂.musicxml",bytes:[...Buffer.from(xml)],default_bpm:80});
  const song=await cmd({type:"currentSong"}),cid=song.contentId;
  let papers=await cmd({type:"addScoreAttachment",content_id:cid,name:"原件课堂.png",bytes:[...first],append:null});const book=papers.active;
  papers=await cmd({type:"addScoreAttachment",content_id:cid,name:"第二页.png",bytes:[...second],append:book});
  const old=papers.attachments.find((a:any)=>a.id===book),asset=old.pages[0].id;
  const context=await cmd({type:"paperPracticeContext"});
  await cmd({type:"setPaperMapping",content_id:cid,id:book,mapping:{grid:context.grid,enabled:true,anchors:[{measure:1,page:1}]}});
  await cmd({type:"editPaperAnnotation",content_id:cid,book,id:null,draft:{assetId:asset,page:1,x:0.2,y:0.2,text:"原谱手位",color:"yellow"},expected:null});
  await cmd({type:"linkPaperSource",content_id:cid,book,asset,path:source});
  await writeFile(source,updated);
  const inspection=await cmd({type:"inspectPaperSources",content_id:cid,book});expect(inspection.rows[0].status).toBe("changed");
  const preview=await cmd({type:"previewPaperSource",content_id:cid,book,asset,fingerprint:inspection.rows[0].fingerprint});
  await cmd({type:"updateScoreAttachment",content_id:cid,id:book,action:"rename",name:"教师原谱",view:null,page:0,direction:0});
  expect((await raw({type:"applyPaperSource",content_id:cid,book,asset,fingerprint:preview.fingerprint,baseline:preview.baseline,action:"version"})).ok()).toBeFalsy();
  await page.goto("/");await expect(page.getByText("本地引擎已连接",{exact:true})).toBeVisible();
  await page.getByRole("button",{name:"谱面管理",exact:true}).click();const d=page.getByRole("dialog",{name:"谱面管理"});await d.getByRole("button",{name:"PDF / 图片谱面",exact:true}).click();
  await d.locator(".paper-sources summary").click();await expect(d.getByText("图片第 1 页 · 原件已更新",{exact:true})).toBeVisible();
  await d.getByRole("button",{name:"核对更新谱页 1",exact:true}).click();const pane=d.getByRole("region",{name:"原件更新预览"});await expect(pane.locator("canvas")).toBeVisible();
  await pane.locator("canvas").scrollIntoViewIfNeeded();
  await page.screenshot({path:"../outputs/Neothesia-谱页原件更新预览.png",fullPage:true});
  await pane.getByRole("button",{name:"保存为新版本",exact:true}).click();
  await expect(d.getByLabel("PDF 与图片谱面版本")).not.toHaveValue(book);
  const result=await cmd({type:"scoreAttachments",content_id:cid}),original=result.attachments.find((a:any)=>a.id===book),newBook=result.attachments.find((a:any)=>a.id===result.active);
  expect(original.annotations).toHaveLength(1);expect(original.mapping.anchors).toHaveLength(1);expect(newBook.pages).toHaveLength(2);expect(newBook.pages[1].id).toBe(old.pages[1].id);expect(newBook.mapping??null).toBeNull();expect(newBook.annotations??[]).toHaveLength(0);
  await rename(source,moved);expect((await cmd({type:"inspectPaperSources",content_id:cid,book:newBook.id})).rows[0].status).toBe("missing");
  await cmd({type:"linkPaperSource",content_id:cid,book:newBook.id,asset:newBook.pages[0].id,path:moved});expect((await cmd({type:"inspectPaperSources",content_id:cid,book:newBook.id})).rows[0].status).toBe("same");
  expect((await raw({type:"linkPaperSource",content_id:cid,book,asset,path:moved})).ok()).toBeFalsy();
  await writeFile(moved,first);
  const changed=await cmd({type:"inspectPaperSources",content_id:cid,book:newBook.id});
  const again=await cmd({type:"previewPaperSource",content_id:cid,book:newBook.id,asset:newBook.pages[0].id,fingerprint:changed.rows[0].fingerprint});
  await cmd({type:"applyPaperSource",content_id:cid,book:newBook.id,asset:newBook.pages[0].id,fingerprint:again.fingerprint,baseline:again.baseline,action:"ignore"});
  expect((await cmd({type:"inspectPaperSources",content_id:cid,book:newBook.id})).rows[0].status).toBe("ignored");
  await writeFile(moved,second);expect((await cmd({type:"inspectPaperSources",content_id:cid,book:newBook.id})).rows[0].status).toBe("changed");
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
test("PDF 原件新版预览翻页与原件再次变化拒绝",async({page,request})=>{
  const raw=(data:unknown)=>request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data});
  const cmd=async(data:unknown)=>{const r=await raw(data);expect(r.ok(),await r.text()).toBeTruthy();return r.json();};
  const cid=(await cmd({type:"currentSong"})).contentId, bytes=pdf(),folder=resolve(`../work/cycle160-pdf-${Date.now()}`);
  await mkdir(folder,{recursive:true});const path=resolve(folder,"teacher.pdf");await writeFile(path,bytes);
  const papers=await cmd({type:"addScoreAttachment",content_id:cid,name:"更新预览.pdf",bytes:[...bytes],append:null});
  const book=papers.active,asset=papers.attachments.find((a:any)=>a.id===book).pages[0].id;
  await cmd({type:"linkPaperSource",content_id:cid,book,asset,path});await writeFile(path,Buffer.concat([bytes,Buffer.from("\n% first update")]));
  await page.goto("/");await expect(page.getByText("本地引擎已连接",{exact:true})).toBeVisible();await page.getByRole("button",{name:"谱面管理",exact:true}).click();
  const d=page.getByRole("dialog",{name:"谱面管理"});await d.getByRole("button",{name:"PDF / 图片谱面",exact:true}).click();await d.locator(".paper-sources summary").click();
  await d.getByRole("button",{name:"核对更新谱页 1",exact:true}).click();const preview=d.getByRole("region",{name:"原件更新预览"});await expect(preview.locator("canvas")).toBeVisible();
  await expect(preview.getByLabel("谱页页码")).toHaveAttribute("max","2");await preview.getByLabel("谱页页码").fill("2");await expect(preview.locator("canvas")).toHaveAttribute("aria-label","谱面第 2 页");
  await writeFile(path,Buffer.concat([bytes,Buffer.from("\n% second update")]));await preview.getByRole("button",{name:"保存为新版本",exact:true}).click();
  await expect(d.getByRole("alert")).toContainText("原件已再次变化");expect((await cmd({type:"scoreAttachments",content_id:cid})).active).toBe(book);
});
