import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { api } from "./api";
import { PaperReader, type PaperAttachment } from "./PaperScore";
type Row={assetId:string;page:number;name:string;status:string;path?:string;fingerprint?:string;error?:string};
type Inspection={rows:Row[];baseline:string};
type Preview={baseline:string;fingerprint:string;asset:{id:string;name:string;kind:string;size:number};name:string;format:"pdf"|"images";row:Row};
const labels:Record<string,string>={unlinked:"未关联原件",same:"与副本一致",changed:"原件已更新",ignored:"已忽略这次更新",missing:"原件位置缺失",unavailable:"原件暂时无法读取"};
export function PaperSources({contentId,attachment,disabled,changed,onBusy}:{contentId:string;attachment:PaperAttachment;disabled:boolean;changed:()=>Promise<void>;onBusy:(value:boolean)=>void}){
  const [inspection,setInspection]=useState<Inspection|null>(null),[paths,setPaths]=useState<Record<string,string>>({}),[busy,setBusy]=useState(false),[error,setError]=useState(""),[preview,setPreview]=useState<Preview|null>(null);
  const base={content_id:contentId,book:attachment.id};
  useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
  const check=async()=>{const v=await api.command<Inspection>({type:"inspectPaperSources",...base});setInspection(v);setPreview(null);};
  useEffect(()=>{let live=true;void api.command<Inspection>({type:"inspectPaperSources",...base}).then(v=>live&&setInspection(v)).catch(e=>live&&setError(String(e)));return()=>{live=false;};},[contentId,attachment.id]);
  async function work(fn:()=>Promise<void>){setBusy(true);setError("");try{await fn();}catch(e){setError(String(e));}finally{setBusy(false);}}
  async function link(row:Row,path:string|null){await api.command({type:"linkPaperSource",...base,asset:row.assetId,path});await changed();await check();}
  const locked=disabled||busy;
  return <details className="paper-sources"><summary>原件位置与更新</summary>
    <p>导入副本可独立使用。原件更新先核对再保存为新版本，旧版批注与小节对应保留。内容相同的文件可直接重新关联位置。</p>
    <button disabled={locked} onClick={()=>void work(check)}>检查原件更新</button>
    {error&&<p role="alert">{error}</p>}
    {inspection?.rows.map(row=><div key={row.assetId} className="paper-source-row">
      <strong>{attachment.format==="pdf"?"PDF 原件":`图片第 ${row.page} 页`} · {labels[row.status]}</strong>
      {row.error&&<small>{row.error}</small>}
      <label>原文件位置<input aria-label={`谱页原件位置 ${row.page}`} disabled={locked} value={paths[row.assetId]??row.path??""} placeholder="选择或填写原文件完整位置" onChange={e=>setPaths({...paths,[row.assetId]:e.target.value})}/></label>
      <div className="paper-source-actions">
        {isTauri()&&<button disabled={locked} onClick={()=>void work(async()=>{const path=await invoke<string|null>("pick_paper_source");if(path)await link(row,path);})}>选择原件</button>}
        <button disabled={locked||!(paths[row.assetId]??row.path)} onClick={()=>void work(()=>link(row,paths[row.assetId]??row.path??""))}>关联这个位置</button>
        {row.path&&<button disabled={locked} onClick={()=>void work(()=>link(row,null))}>解除原件关联</button>}
        {row.fingerprint&&["changed","ignored"].includes(row.status)&&<button disabled={locked} onClick={()=>void work(async()=>{const v=await api.command<Omit<Preview,"row">>({type:"previewPaperSource",...base,asset:row.assetId,fingerprint:row.fingerprint});setPreview({...v,row});})}>核对更新谱页 {row.page}</button>}
      </div>
    </div>)}
    {preview&&<section className="paper-source-preview" aria-label="原件更新预览">
      <strong>{preview.asset.name} · 新原件预览</strong><p>保存为新版本会保留其他图片页，重新确认小节对应和批注。原版本及其资料继续保留。</p>
      <PaperReader key={preview.fingerprint} contentId={contentId} attachment={{id:preview.asset.id,name:preview.asset.name,format:preview.asset.kind==="pdf"?"pdf":"images",pages:[{...preview.asset,available:true}],view:{page:1,zoom:100,fit:true,rotation:0}}}
        previewKind={preview.asset.kind} disabled={locked} manage={false} practice={null} update={async()=>{}} saveMapping={async()=>{}} saveAnnotation={async()=>false}/>
      <div className="paper-source-actions">{["version","ignore"].map(action=><button key={action} disabled={locked} onClick={()=>void work(async()=>{await api.command({type:"applyPaperSource",...base,asset:preview.row.assetId,fingerprint:preview.fingerprint,baseline:preview.baseline,action});await changed();await check();})}>{action==="version"?"保存为新版本":"忽略这次更新"}</button>)}<button disabled={locked} onClick={()=>setPreview(null)}>关闭预览</button></div>
    </section>}
  </details>;
}
