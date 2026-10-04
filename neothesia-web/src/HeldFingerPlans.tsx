import {useEffect, useState, useMemo} from "react";
import {api, pitchName, type LoadedSong} from "./api";
import {musicalActionPosition} from "./fingerActions";
import {HeldFingerDemo} from "./HeldFingerDemo";
type Action={track:number;index:number;pitch:number;measure:number;beforeTrack:number;beforeIndex:number;beforeMeasure:number;at:number;from:number;to:number};
type Proposal={track:number;index:number;pitch:number;measure:number;finger:number};
type Result={request:Record<string,unknown>;fingerprint:string;plans:{cost:number;proposals:Proposal[];substitutions:Action[]}[]};
export function HeldFingerPlans({song,request,disabled,changed,onBusy}:{song:LoadedSong;request:Record<string,unknown>;disabled:boolean;changed:()=>Promise<void>;onBusy:(busy:boolean)=>void}){
 const [result,setResult]=useState<Result|null>(null),[selected,setSelected]=useState(0),[busy,setBusy]=useState(false),[error,setError]=useState(""),[message,setMessage]=useState("");
 const [demoSource,setDemoSource]=useState<{track:number;index:number}|null>(null);
 const [showSaved,setShowSaved]=useState(false),[savedPage,setSavedPage]=useState(0);
 const saved=song.fingerActions??[];
 const notesById=useMemo(()=>new Map(song.notes.map(n=>[`${n.track}:${n.index}`,n])),[song.notes]);
 useEffect(()=>{setDemoSource(null);setShowSaved(false);setSavedPage(0);},[song.contentId]);
 useEffect(()=>setSavedPage(p=>Math.min(p,Math.max(0,Math.ceil(saved.length/20)-1))),[saved.length]);
 const signature=JSON.stringify(request);
 useEffect(()=>{setResult(null);setError("");setMessage("");},[signature,song.scoreRevision]);
 const run=async(work:()=>Promise<void>)=>{setBusy(true);onBusy(true);setError("");setMessage("");try{await work();}catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false);onBusy(false);}};
 const plan=result?.plans[selected];
 const beatAt=(at:number)=>musicalActionPosition(song,at);
 return <section className="finger-review held-finger-plans" aria-label="持音换指">
  <div className="finger-review-heading"><strong>持音换指</strong><button disabled={busy||disabled} onClick={()=>void run(async()=>{
   const data=await api.command<Result>({type:"suggestHeldFingerPlans",request});setResult(data);setSelected(0);
  })}>寻找持音换指方案</button></div>
  <p>保持琴键按下，在下一次发音前换用另一根手指。起音指法与换指动作一起保存，请先确认整个选段。</p>
  {error&&<p role="alert">{error}</p>}{message&&<p role="status">{message}</p>}
  {result&&<>
   <div className="fingering-selection">{result.plans.map((p,i)=><button key={i} disabled={busy||disabled} aria-pressed={selected===i} onClick={()=>setSelected(i)}>方案 {i+1} · {p.substitutions.length} 次换指</button>)}</div>
   {plan&&<>
    <div className="held-onsets"><strong>起音指法 · {plan.proposals.length} 个音符</strong><div>{plan.proposals.map(p=><span key={`${p.track}:${p.index}`}>第 {p.measure} 小节 {pitchName(p.pitch)}：{p.finger} 指</span>)}</div></div>
    {plan.substitutions.map((a,i)=><div className="held-action-row" key={i}><strong>{beatAt(a.at)}</strong><span>{pitchName(a.pitch)} 保持按下 · {a.from} 指 → {a.to} 指</span><small>在第 {a.beforeMeasure} 小节下一音发音前完成</small></div>)}
    {!plan.substitutions.length&&<p>这个方案不需要持音换指，可通过普通指法建议保存。</p>}
    <button className="primary-button" disabled={busy||disabled||!plan.substitutions.length} onClick={()=>void run(async()=>{
     await api.command({type:"acceptHeldFingerPlan",request:result.request,fingerprint:result.fingerprint,edits:plan.proposals.map(p=>({track_id:p.track,note_index:p.index,finger:p.finger})),actions:plan.substitutions});
     setResult(null);setShowSaved(true);await changed();setMessage("起音指法与持音换指已一起保存，可用撤销恢复。");
    })}>接受完整换指方案</button>
   </>}
  </>}
  {!!saved.length&&<details className="held-saved" open={showSaved} onToggle={e=>{setShowSaved(e.currentTarget.open);if(!e.currentTarget.open)setDemoSource(null);}}><summary><strong>已保存 · {saved.length} 次换指</strong>{saved.some(a=>!a.valid)&&<span> · 有动作需要复核</span>}</summary>
   {showSaved&&<>
   {saved.slice(savedPage*20,(savedPage+1)*20).map((a,i)=>{const n=notesById.get(`${a.track}:${a.index}`);return <div className="held-action-row" key={i}><span>{beatAt(a.at)} · {n?pitchName(n.pitch):"音符已变化"} · {a.from} → {a.to} 指</span><small>{a.valid?"当前速度下可提示":"当前指法、分手或速度需要重新审阅"}</small><button disabled={busy||disabled||!n} onClick={()=>setDemoSource({track:a.track,index:a.index})}>查看手位示范</button></div>;})}
   {saved.length>20&&<div className="fingering-pages"><button disabled={savedPage===0||busy||disabled} onClick={()=>setSavedPage(p=>p-1)}>上一页换指</button><span>{savedPage+1} / {Math.ceil(saved.length/20)}</span><button disabled={(savedPage+1)*20>=saved.length||busy||disabled} onClick={()=>setSavedPage(p=>p+1)}>下一页换指</button></div>}
   {demoSource&&saved.some(a=>a.track===demoSource.track&&a.index===demoSource.index)&&<HeldFingerDemo song={song} {...demoSource} disabled={busy||disabled}/>}
   <button disabled={busy||disabled} onClick={()=>void run(async()=>{await api.command({type:"clearHeldFingerActions",content_id:song.contentId});await changed();setMessage("已清除换指动作，起音指法保留；可撤销。");})}>清除全部换指动作</button>
   </>}
  </details>}
 </section>;
}
