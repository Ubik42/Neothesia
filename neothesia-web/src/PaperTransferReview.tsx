import { useState, useEffect } from "react";
import { api } from "./api";
import { PaperReader, type PaperAttachment } from "./PaperScore";
type Preview={source:PaperAttachment;target:PaperAttachment;baseline:string;imported:string[]};
type NoteDraft={id:string;selected:boolean;assetId:string;page:number;x:number;y:number;targetPage:number};
type AnchorDraft={measure:number;page:number;region?:import("./PaperAnnotations").PaperRegion;selected:boolean};
export function PaperTransferReview({contentId,target,books,disabled,changed,onBusy}:{contentId:string;target:PaperAttachment;books:PaperAttachment[];disabled:boolean;changed:()=>Promise<void>;onBusy:(v:boolean)=>void}){
  const [source,setSource]=useState(books.find(b=>b.id!==target.id)?.id??""),[preview,setPreview]=useState<Preview|null>(null),[notes,setNotes]=useState<NoteDraft[]>([]),[anchors,setAnchors]=useState<AnchorDraft[]>([]);
  const [busy,setBusy]=useState(false),[error,setError]=useState(""),[notice,setNotice]=useState(""),[replace,setReplace]=useState(false),[focus,setFocus]=useState(0),[count,setCount]=useState(target.format==="images"?target.pages.length:0);
  useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
  const [notePage,setNotePage]=useState(0),[anchorPage,setAnchorPage]=useState(0);
  const locked=disabled||busy;
  const selected=notes.filter(n=>n.selected),selectedAnchors=anchors.filter(a=>a.selected);
  const valid=!!count&&(selected.length+selectedAnchors.length)>0&&selected.every(n=>Number.isInteger(n.targetPage)&&n.targetPage>=1&&n.targetPage<=count&&Number.isFinite(n.x)&&Number.isFinite(n.y)&&n.x>=0&&n.x<=1&&n.y>=0&&n.y<=1&&n.x+(preview?.source.annotations?.find(a=>a.id===n.id)?.width??0)<=1.000001&&n.y+(preview?.source.annotations?.find(a=>a.id===n.id)?.height??0)<=1.000001)&&selectedAnchors.every(a=>Number.isInteger(a.page)&&a.page>=1&&a.page<=count);
  async function work(fn:()=>Promise<void>){setBusy(true);setError("");try{await fn();}catch(e){setError(String(e));}finally{setBusy(false);}}
  async function prepare(){const v=await api.command<Preview>({type:"previewPaperTransfer",content_id:contentId,source,target:target.id});setPreview(v);setReplace(false);setFocus(0);setNotePage(0);setAnchorPage(0);setNotice("");
    setNotes((v.source.annotations??[]).map(n=>{const page=v.source.format==="images"?v.source.pages.findIndex(p=>p.id===n.assetId)+1:n.page;
      const same=v.target.pages.findIndex(p=>p.id===n.assetId);const targetPage=v.target.format==="images"?(same>=0?same+1:Math.min(page,v.target.pages.length)):page;
      return{id:n.id,selected:false,assetId:v.target.pages[v.target.format==="images"?targetPage-1:0].id,page:v.target.format==="images"?1:targetPage,x:n.x,y:n.y,targetPage};}));
    setAnchors((v.source.mapping?.anchors??[]).map(a=>({...a,selected:false})));}
  const update=(i:number,patch:Partial<NoteDraft>)=>setNotes(notes.map((n,j)=>i===j?{...n,...patch}:n));
  const sourceNote=preview?.source.annotations?.[focus],draft=notes[focus];
  const sourcePage=sourceNote?(preview!.source.format==="images"?preview!.source.pages.findIndex(p=>p.id===sourceNote.assetId)+1:sourceNote.page):1;
  const targetPage=draft?.targetPage??1;
  const overlay=preview?{...preview.target,annotations:[...(preview.target.annotations??[]),...selected.map(n=>({...preview.source.annotations!.find(s=>s.id===n.id)!,id:`preview-${n.id}`,assetId:n.assetId,page:n.page,x:n.x,y:n.y}))],view:{page:Math.min(Math.max(1,targetPage),count||5000),zoom:100,fit:true,rotation:0}}:null;
  const readerProps={contentId,disabled:locked,manage:false,readOnly:true,practice:null,update:async()=>{},saveMapping:async()=>{},saveAnnotation:async()=>false};
  return <details className="paper-transfer-review"><summary>从其他版本迁移批注与小节对应</summary>
    <p>选择来源，逐条核对目标页与位置。保留目标已有批注，迁移后可在谱页继续编辑。小节对应保存后先关闭自动翻页。</p>
    <div className="paper-source-actions"><select aria-label="迁移来源谱面" value={source} disabled={locked} onChange={e=>{setSource(e.target.value);setPreview(null);}}>{books.filter(b=>b.id!==target.id).map(b=><option key={b.id} value={b.id}>{b.name}</option>)}</select><button disabled={locked||!source} onClick={()=>void work(prepare)}>审阅迁移资料</button></div>
    {error&&<p role="alert">{error}</p>}{notice&&<p role="status">{notice}</p>}
    {preview&&<>
      <div className="paper-transfer-compare">
        <section aria-label="迁移来源谱页"><strong>来源 · {preview.source.name}</strong><PaperReader key={`source:${sourcePage}`} {...readerProps} attachment={{...preview.source,view:{page:sourcePage,zoom:100,fit:true,rotation:0}}}/></section>
        <section aria-label="迁移目标谱页"><strong>目标 · {preview.target.name} · 已选批注预览</strong><PaperReader key={`target:${targetPage}`} {...readerProps} attachment={overlay!} onPageCount={setCount}/></section>
      </div>
      <strong>批注 · 选择 {selected.length} / {notes.length}</strong>
      {notes.slice(notePage*20,(notePage+1)*20).map((n,offset)=>{const i=notePage*20+offset;const original=preview.source.annotations![i],imported=preview.imported.includes(n.id);return <fieldset key={n.id} disabled={locked||imported} className="paper-transfer-note">
        <legend><label><input aria-label={`迁移批注 ${i+1}`} type="checkbox" disabled={locked||imported} checked={n.selected} onChange={e=>update(i,{selected:e.target.checked})}/>{original.text||(original.ink?"手写笔迹":original.width?"高亮区域":"未填写说明")}{imported?" · 已迁移":""}</label></legend>
        <div className="paper-transfer-fields"><button onClick={()=>setFocus(i)}>对照批注 {i+1}</button>
          <label>目标页<input aria-label={`批注目标页 ${i+1}`} type="number" min={1} max={count||5000} value={n.targetPage} onChange={e=>{const p=Number(e.target.value);update(i,{targetPage:p,page:preview.target.format==="images"?1:p,assetId:preview.target.pages[preview.target.format==="images"?p-1:0]?.id??""});setFocus(i);}}/></label>
          <label>横向位置 %<input aria-label={`批注横向位置 ${i+1}`} type="number" min={0} max={100} step={1} value={Math.round(n.x*100)} onChange={e=>{update(i,{x:Number(e.target.value)/100});setFocus(i);}}/></label>
          <label>纵向位置 %<input aria-label={`批注纵向位置 ${i+1}`} type="number" min={0} max={100} step={1} value={Math.round(n.y*100)} onChange={e=>{update(i,{y:Number(e.target.value)/100});setFocus(i);}}/></label>
        </div>
      </fieldset>;})}
      {notes.length>20&&<div className="paper-source-actions"><button disabled={locked||!notePage} onClick={()=>setNotePage(notePage-1)}>上一页批注</button><span>{notePage+1} / {Math.ceil(notes.length/20)}</span><button disabled={locked||(notePage+1)*20>=notes.length} onClick={()=>setNotePage(notePage+1)}>下一页批注</button></div>}
      {!notes.length&&<p>来源没有批注。</p>}
      <strong>小节对应 · 选择 {selectedAnchors.length} / {anchors.length}</strong>
      {anchors.slice(anchorPage*20,(anchorPage+1)*20).map((a,offset)=>{const i=anchorPage*20+offset;return <div key={a.measure} className="paper-transfer-anchor"><label><input aria-label={`迁移第 ${a.measure} 小节对应`} type="checkbox" checked={a.selected} disabled={locked} onChange={e=>setAnchors(anchors.map((r,j)=>j===i?{...r,selected:e.target.checked}:r))}/>第 {a.measure} 小节 · 原第 {preview.source.mapping!.anchors[i].page} 页{a.region?" · 含页内位置":""}</label><label>目标页<input aria-label={`小节对应目标页 ${a.measure}`} type="number" min={1} max={count||5000} value={a.page} disabled={locked} onChange={e=>setAnchors(anchors.map((r,j)=>j===i?{...r,page:Number(e.target.value)}:r))}/></label>{preview.target.mapping?.anchors.find(r=>r.measure===a.measure)&&<small>目标已有对应，保留或明确替换。</small>}{a.region&&<button disabled={locked} onClick={()=>setAnchors(anchors.map((r,j)=>j===i?{...r,region:undefined}:r))}>仅迁移页码</button>}</div>;})}
      {anchors.length>20&&<div className="paper-source-actions"><button disabled={locked||!anchorPage} onClick={()=>setAnchorPage(anchorPage-1)}>上一页小节对应</button><span>{anchorPage+1} / {Math.ceil(anchors.length/20)}</span><button disabled={locked||(anchorPage+1)*20>=anchors.length} onClick={()=>setAnchorPage(anchorPage+1)}>下一页小节对应</button></div>}
      <label><input type="checkbox" aria-label="替换目标同小节对应" checked={replace} disabled={locked} onChange={e=>setReplace(e.target.checked)}/>允许替换选中小节在目标谱面已有的页码对应</label>
      <div className="paper-source-actions"><button disabled={locked||!valid} onClick={()=>void work(async()=>{const result=await api.command<{added:number;anchors:number}>({type:"applyPaperTransfer",content_id:contentId,request:{source,target:target.id,baseline:preview.baseline,annotations:selected.map(n=>({id:n.id,assetId:n.assetId,page:n.page,x:n.x,y:n.y})),anchors:selectedAnchors.map(({selected:_,...a})=>a),replaceAnchors:replace}});await changed();setPreview(null);setNotice(`已迁移 ${result.added} 条批注、${result.anchors} 项小节对应。请核对后再启用自动翻页。`);})}>确认迁移选中资料</button><button disabled={locked} onClick={()=>setPreview(null)}>取消迁移</button></div>
    </>}
  </details>;
}
