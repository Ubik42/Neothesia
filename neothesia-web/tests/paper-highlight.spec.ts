import { test, expect } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { PDFDocument } from "pdf-lib";
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
test("纸谱高亮拖选、旋转、重开与导出",async({page,request})=>{
 const cmd=async(data:unknown)=>{const r=await request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data});expect(r.ok(),await r.text()).toBeTruthy();return r.json()};
 await cmd({type:"generate",spec:{tonic:11,tonality:"Major",minor_form:"Natural",pattern:"Scale",direction:"UpAndDown",hands:"Both",octaves:1,repetitions:1,tempo_bpm:120}});const song=await cmd({type:"currentSong"});const papers=await cmd({type:"addScoreAttachment",content_id:song.contentId,name:"高亮课堂.pdf",bytes:[...Buffer.concat([pdf(),Buffer.from("\n%"+Date.now())])]});await cmd({type:"updateScoreAttachment",content_id:song.contentId,id:papers.active,action:"view",view:{page:1,zoom:100,fit:true,rotation:0}});
 await page.goto("/");await page.getByRole("button",{name:"谱页",exact:true}).click();await expect(page.getByRole("button",{name:"谱页",exact:true})).toHaveClass(/selected/);const panel=page.getByRole("region",{name:"谱页浏览"});await expect(panel.locator("canvas")).toBeVisible();await panel.getByRole("button",{name:"添加高亮区域",exact:true}).click();const sheet=panel.locator(".paper-annotation-sheet"),b=await sheet.boundingBox();await page.mouse.move(b!.x+b!.width*.2,b!.y+b!.height*.06);await page.mouse.down();await page.mouse.move(b!.x+b!.width*.5,b!.y+b!.height*.14);await page.mouse.up();await expect(panel.getByLabel("谱页批注内容")).toBeVisible();await panel.getByRole("button",{name:"保存批注",exact:true}).click();const highlight=panel.locator("button.paper-highlight");await expect(highlight).toHaveCount(1);const stored=await cmd({type:"scoreAttachments",content_id:song.contentId});const note=stored.attachments.find((a:any)=>a.id===papers.active).annotations[0];expect(note.width).toBeCloseTo(.3,2);expect(note.height).toBeCloseTo(.08,2);expect(note.text).toBe("");
 await highlight.click();await panel.getByLabel("谱页批注内容").fill("这一行慢练，保持连奏");await panel.getByRole("button",{name:"保存批注",exact:true}).click();await panel.getByRole("button",{name:"旋转 90°",exact:true}).click();await expect.poll(async()=>parseFloat(await highlight.evaluate(el=>(el as HTMLElement).style.left))).toBeCloseTo((1-note.y-note.height)*100,2);await expect.poll(async()=>parseFloat(await highlight.evaluate(el=>(el as HTMLElement).style.width))).toBeCloseTo(note.height*100,2);await highlight.scrollIntoViewIfNeeded();await page.screenshot({path:"../outputs/Neothesia-纸谱高亮区域.png"});await page.getByRole("button",{name:"隐藏曲库",exact:true}).click();await page.getByRole("button",{name:"隐藏练习参数",exact:true}).click();await page.setViewportSize({width:700,height:1100});await highlight.scrollIntoViewIfNeeded();await page.screenshot({path:"../outputs/Neothesia-纸谱高亮区域-窄窗.png"});
 const download=page.waitForEvent("download");await panel.getByRole("button",{name:"导出本页批注",exact:true}).click();const file=await download;const bytes=await readFile((await file.path())!);expect(bytes.readUInt32BE(16)).toBeGreaterThan(1000);await file.saveAs("../outputs/Neothesia-纸谱高亮导出.png");
 const bookDownload=page.waitForEvent("download");await panel.getByRole("button",{name:"导出整本批注 PDF",exact:true}).click();const book=await bookDownload;const bookBytes=await readFile((await book.path())!);expect(bookBytes.subarray(0,5).toString()).toBe("%PDF-");const compiled=await PDFDocument.load(bookBytes);expect(compiled.getPageCount()).toBeGreaterThanOrEqual(2);expect(compiled.getPageCount()).toBeLessThanOrEqual(5);await book.saveAs("../outputs/Neothesia-整本纸谱批注.pdf");
 await page.reload();await page.getByRole("button",{name:"谱页",exact:true}).click();await expect(highlight).toHaveCount(1);await highlight.click();await expect(panel.getByLabel("谱页批注内容")).toHaveValue("这一行慢练，保持连奏");await panel.getByRole("button",{name:"删除此批注",exact:true}).click();await expect(highlight).toHaveCount(0);
});

test("多张图片谱能导出完整 PDF",async({page,request})=>{
 const cmd=async(data:unknown)=>{const r=await request.post("http://127.0.0.1:32124/api/command",{headers:{"X-Neothesia-Client":"web"},data});expect(r.ok(),await r.text()).toBeTruthy();return r.json()};
 await cmd({type:"generate",spec:{tonic:7,tonality:"Major",minor_form:"Natural",pattern:"Scale",direction:"UpAndDown",hands:"Both",octaves:1,repetitions:1,tempo_bpm:120}});
 const song=await cmd({type:"currentSong"});
 const png=async(label:string)=>Buffer.from(await page.evaluate(label=>{const canvas=document.createElement("canvas");canvas.width=600;canvas.height=800;const context=canvas.getContext("2d")!;context.fillStyle="white";context.fillRect(0,0,600,800);context.fillStyle="black";context.font="48px sans-serif";context.fillText(label,40,100);return canvas.toDataURL("image/png").split(",")[1]},label),"base64");
 const first=await cmd({type:"addScoreAttachment",content_id:song.contentId,name:"课堂第一页.png",bytes:[...await png("Page 1")],append:null});
 await cmd({type:"addScoreAttachment",content_id:song.contentId,name:"课堂第二页.png",bytes:[...await png("Page 2")],append:first.active});
 await page.goto("/");await page.getByRole("button",{name:"谱页",exact:true}).click();await expect(page.getByRole("button",{name:"谱页",exact:true})).toHaveClass(/selected/);
 const panel=page.getByRole("region",{name:"谱页浏览"});await expect(panel.locator("canvas")).toBeVisible();
 const download=page.waitForEvent("download");await panel.getByRole("button",{name:"导出整本批注 PDF",exact:true}).click();const file=await download;
 const document=await PDFDocument.load(await readFile((await file.path())!));expect(document.getPageCount()).toBe(2);
});
