import {useEffect,useRef,useState} from "react";
import {api,pitchName,type LoadedSong} from "./api";
import {musicalActionPosition} from "./fingerActions";
type FingerNote={track:number;index:number;pitch:number;part:string;finger:number|null;held?:boolean;changed?:boolean};
type Context={baseline:string;start:number;end:number;notes:(FingerNote&{onset:number;end:number})[];actions:{track:number;index:number;at:number;from:number;to:number}[]};
type Demo={id:string|null;active:boolean;position?:number;fingerNotes?:FingerNote[];actionsCompleted?:number;actionsTotal?:number};
export function HeldFingerDemo({song,track,index,disabled,onActive}:{song:LoadedSong;track:number;index:number;disabled:boolean;onActive?:(value:boolean)=>void}){
 const [rate,setRate]=useState(.5),[context,setContext]=useState<Context|null>(null),[live,setLive]=useState<Demo|null>(null),[starting,setStarting]=useState(false),[playing,setPlaying]=useState(false),[error,setError]=useState(""),[status,setStatus]=useState(""),[position,setPosition]=useState(0),[step,setStep]=useState(0);
 const token=useRef<string|null>(null),epoch=useRef(0),querying=useRef(false),alive=useRef(false),launch=useRef(0),activeCallback=useRef(onActive);
 activeCallback.current=onActive;
 const apply=(data:Demo)=>{
  if(data.id!==token.current){token.current=null;setPlaying(false);setLive(null);setStatus("示范已停止");activeCallback.current?.(false);return;}
  setLive(data);setPlaying(data.active);activeCallback.current?.(data.active);setStatus(data.active?"正在示范持音换指":"示范完成");
  if(!data.active){const id=token.current;token.current=null;if(id)void api.command({type:"stopFingerDemo",id}).catch(()=>{});}
 };
 useEffect(()=>{
  const ticket=++epoch.current;alive.current=true;setStarting(false);setPlaying(false);setLive(null);setContext(null);setStatus("");setError("");activeCallback.current?.(false);
  const poll=async()=>{if(!token.current||querying.current)return;querying.current=true;const id=token.current;try{const data=await api.command<Demo>({type:"fingerDemoState"});if(alive.current&&epoch.current===ticket&&token.current===id)apply(data);}catch(e){if(alive.current&&epoch.current===ticket){const old=token.current;token.current=null;setPlaying(false);activeCallback.current?.(false);setError(String(e));if(old)void api.command({type:"stopFingerDemo",id:old}).catch(()=>{});}}finally{querying.current=false;}};
  const timer=window.setInterval(()=>void poll(),80);
  const hide=()=>{if(document.hidden){launch.current++;const id=token.current;token.current=null;if(id)void api.command({type:"stopFingerDemo",id}).catch(()=>{});setStarting(false);setPlaying(false);setLive(null);setStatus("示范已停止");activeCallback.current?.(false);}};
  document.addEventListener("visibilitychange",hide);
  return ()=>{alive.current=false;epoch.current++;launch.current++;clearInterval(timer);document.removeEventListener("visibilitychange",hide);const id=token.current;token.current=null;if(id)void api.command({type:"stopFingerDemo",id}).catch(()=>{});activeCallback.current?.(false);};
 },[song.contentId,song.fingerActionRevision,track,index,rate,disabled]);
 const start=async()=>{
  if(starting||playing||disabled)return;const ticket=epoch.current,attempt=++launch.current;setStarting(true);setError("");activeCallback.current?.(true);
  try{
   const prepared=await api.command<Context>({type:"heldFingerDemoContext",content_id:song.contentId,track,index,rate});
   if(!alive.current||ticket!==epoch.current||attempt!==launch.current||document.hidden)return;
   setContext(prepared);setStep(0);setPosition(prepared.start);
   const data=await api.command<Demo>({type:"startHeldFingerDemo",content_id:song.contentId,track,index,baseline:prepared.baseline,rate});
   if(!alive.current||ticket!==epoch.current||attempt!==launch.current||document.hidden){if(data.id)await api.command({type:"stopFingerDemo",id:data.id});return;}
   token.current=data.id;apply(data);
  }catch(e){if(alive.current&&ticket===epoch.current){setError(e instanceof Error?e.message:String(e));setPlaying(false);activeCallback.current?.(false);}}
  finally{if(alive.current&&ticket===epoch.current&&attempt===launch.current){setStarting(false);if(!token.current)activeCallback.current?.(false);}}
 };
 const stop=()=>{launch.current++;const id=token.current;token.current=null;setStarting(false);setPlaying(false);setLive(null);setStatus("示范已停止");activeCallback.current?.(false);if(id)void api.command({type:"stopFingerDemo",id}).catch(e=>setError(String(e)));};
 const time=live?.position??position;
 const notes:FingerNote[]=live?.fingerNotes??context?.notes.filter(n=>n.onset<=time&&n.end>time).map(n=>{
  const change=context.actions.filter(a=>a.track===n.track&&a.index===n.index&&a.at<=time).at(-1);return {...n,finger:change?.to??n.finger,held:n.onset<time,changed:!!change};
 })??[];
 const changes=context?.actions.filter(a=>a.at>=context.start)??[];
 const black=(pitch:number)=>[1,3,6,8,10].includes(pitch%12);
 const pitches=context?context.notes.map(n=>n.pitch):[60,64];
 const low=Math.max(21,Math.floor(Math.min(...pitches)/12)*12),high=Math.min(108,Math.max(low+12,Math.ceil(Math.max(...pitches)/12)*12));
 const white=Array.from({length:high-low+1},(_,i)=>low+i).filter(p=>!black(p));
 const keys=Array.from({length:high-low+1},(_,i)=>low+i);
 const keyboardKey=(pitch:number)=>{const n=notes.find(n=>n.pitch===pitch),sharp=black(pitch),x=sharp?white.filter(p=>p<pitch).length*32-10:white.indexOf(pitch)*32;
  return <g key={pitch} data-pitch={pitch} data-finger={n?.finger??""} data-changed={n?.changed?"true":"false"}><rect x={x+1} y={0} width={sharp?20:30} height={sharp?64:102} rx={2} fill={n?(n.changed?"#f0bd6d":"#b8df85"):(sharp?"#161c23":"#e0e7ed")} stroke="#263543"/>{n&&<text x={x+(sharp?11:16)} y={sharp?40:76} textAnchor="middle" fill="#243427" fontSize="16" fontWeight="700">{n.finger??"·"}</text>}{!sharp&&<text x={x+16} y={94} textAnchor="middle" fill="#536271" fontSize="9">{pitchName(pitch)}</text>}</g>;
 };
 return <section className="held-finger-demo" aria-label="持音换指手位示范">
  <div className="held-demo-controls"><label>示范速度<select aria-label="持音换指示范速度" value={rate} disabled={playing||starting} onChange={e=>setRate(Number(e.target.value))}>{[.25,.5,.75,1].map(r=><option key={r} value={r}>{r*100}%</option>)}</select></label>
   <button disabled={disabled||starting||playing} onClick={()=>void start()}>{starting?"正在准备示范":"示范此处持音换指"}</button><button disabled={!playing&&!starting} onClick={stop}>停止换指示范</button></div>
  <span role="status">{status}</span>{error&&<p role="alert">{error}</p>}
  {context&&<>
   <strong>{context.notes.find(n=>n.track===track&&n.index===index)?.part==="left"?"左手":"右手"} · {pitchName(context.notes.find(n=>n.track===track&&n.index===index)?.pitch??60)} 持音换指示范</strong>
   <p>{musicalActionPosition(song,time)}{live?.actionsTotal!=null?` · 已换指 ${live.actionsCompleted} / ${live.actionsTotal}`:""}</p>
   <div className="held-demo-keyboard"><svg aria-label="持音换指示范键盘" width={white.length*32} height={108} viewBox={`0 0 ${white.length*32} 108`}>{white.map(keyboardKey)}{keys.filter(black).map(keyboardKey)}</svg></div>
   <div className="held-demo-active">{notes.map(n=><span key={`${n.track}:${n.index}`}>{pitchName(n.pitch)} · {n.finger??"未标记"} 指{n.changed?" · 已接替":n.held?" · 保持按下":" · 起音"}</span>)}</div>
   {!!changes.length&&<div className="held-demo-steps"><button disabled={playing||starting||step===0} onClick={()=>{setLive(null);setStep(step-1);setPosition(Math.min(context.end,changes[step-1].at+.01));}}>上一次换指</button><span>{step+1} / {changes.length}</span><button disabled={playing||starting||step+1>=changes.length} onClick={()=>{setLive(null);setStep(step+1);setPosition(Math.min(context.end,changes[step+1].at+.01));}}>下一次换指</button>
    <button disabled={playing||starting} onClick={()=>{setLive(null);setPosition(Math.max(context.start,changes[step].at-.01));}}>换指前手位</button><button disabled={playing||starting} onClick={()=>{setLive(null);setPosition(Math.min(context.end,changes[step].at+.01));}}>换指后手位</button></div>}
  </>}
  <small>按实际起音、换指和离键示范；换指不会重新发音。按下琴键、开始练习或关闭此处会停止，不计成绩。</small>
 </section>;
}
