import type {LoadedSong, Note} from "./api";
export type HeldAction=NonNullable<LoadedSong["fingerActions"]>[number];
export function musicalActionPosition(song:LoadedSong,at:number){
 const bar=song.measures.find(m=>m.start<=at&&m.end>at)??song.measures.at(-1);
 if(!bar)return "换指位置";
 const tempo=song.tempo.filter(t=>t.seconds<=at).at(-1);
 const tick=tempo?tempo.tick+(at-tempo.seconds)*tempo.bpm*song.ppq/60:at*song.ppq*2;
 const beat=1+(tick-bar.startTick)/(song.ppq*4/bar.denominator);
 return `第 ${bar.number} 小节 · 第 ${Number(beat.toFixed(2))} 拍`;
}
export function actionsForNote(song:LoadedSong,n:Pick<Note,"track"|"index">){
 return (song.fingerActions??[]).filter(a=>a.track===n.track&&a.index===n.index).sort((a,b)=>a.at-b.at);
}
export function actionUsable(a:HeldAction,rate:number){return a.valid&&Math.abs(a.validationRate-rate)<0.0001;}
export function fingerSequence(note:Note|undefined,actions:HeldAction[],rate:number){
 if(!note?.finger)return actions.length?"· ⚠":"";
 if(!actions.length)return String(note.finger);
 if(actions.some(a=>!actionUsable(a,rate)))return `${note.finger} ⚠`;
 return [note.finger,...actions.map(a=>a.to)].join("→");
}
