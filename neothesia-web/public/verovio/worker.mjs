import createModule from "./module.mjs";
import { VerovioToolkit } from "./toolkit.mjs";
let toolkit;
self.onmessage=async({data})=>{
 try{
  if(!toolkit)toolkit=new VerovioToolkit(await createModule());
  toolkit.setOptions({breaks:"auto",pageWidth:2100,pageHeight:2970,scale:40,svgHtml5:true,svgViewBox:true,xmlIdChecksum:true,expandNever:true});
  const bytes=new Uint8Array(data.bytes);
  const ok=data.compressed?toolkit.loadZipDataBuffer(bytes.buffer):toolkit.loadData(new TextDecoder().decode(bytes));
  if(!ok)throw new Error("无法读取这份乐谱");
  const count=toolkit.getPageCount();if(count<1||count>300)throw new Error("乐谱页数超出支持范围");
  const pages=[],notes=[];
  const midi=toolkit.renderToMIDI();
  for(let page=1;page<=count;page++){
   const svg=toolkit.renderToSVG(page);pages.push(svg);
   for(const tag of svg.matchAll(/<g\b[^>]*\bclass="[^"]*\bnote\b[^"]*"[^>]*>/g)){
    const rendererId=tag[0].match(/\bid="([^"]+)"/)?.[1];if(!rendererId)continue;
    const value=toolkit.getMIDIValuesForElement(rendererId);
    if([value.time,value.pitch,value.duration].every(Number.isFinite))notes.push({renderer_id:rendererId,page_index:page-1,onset_millis:value.time,pitch:value.pitch,duration_millis:value.duration});
   }
  }
  self.postMessage({pages,notes,midi});
 }catch(e){self.postMessage({error:String(e.message||e)})}
};
