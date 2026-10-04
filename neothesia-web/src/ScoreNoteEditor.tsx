import { useEffect, useRef, useState } from "react";
import { api, pitchName, type Note, type LoadedSong } from "./api";
import {actionsForNote,actionUsable,musicalActionPosition} from "./fingerActions";
import {HeldActionEditor} from "./HeldActionEditor";
import {HeldFingerDemo} from "./HeldFingerDemo";
export interface ScoreMapping {
  id: string;
  page: number;
  start: number;
  end: number;
  pitch: number;
  track: number;
  index: number;
  scoreFinger: string | null;
}
interface Proposal {
  track: number;
  index: number;
  finger: number;
  confidence: number;
  reason: string;
  measure: number;
}
export function ScoreNoteEditor({
  song,
  positions,
  selected,
  choose,
  close,
  changed,
  detail,
  assignHands,
  onBusy,
  seek,
  locate,
  rate,
}: {
  song: LoadedSong;
  positions: ScoreMapping[];
  selected: ScoreMapping;
  choose: (p: ScoreMapping) => void;
  close: () => void;
  changed: () => Promise<void>;
  detail: (note: Note) => void;
  assignHands: () => void;
  onBusy: (busy: boolean) => void;
  seek: (position:number)=>void;
  locate: (track:number,index:number)=>boolean;
  rate:number;
}) {
  const note = song.notes.find(
      (n) => n.track === selected.track && n.index === selected.index,
    ),
    [workingBusy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [result, setResult] = useState<Proposal | null>(null),
    [choices, setChoices] = useState<Proposal[]>([]),
    [profile, setProfile] = useState("Standard"),
    [all, setAll] = useState(false),
    [status, setStatus] = useState("");
  const [demonstrating,setDemonstrating]=useState(false);
  const busy=workingBusy||demonstrating;
  useEffect(() => {
    onBusy(busy);
    return () => onBusy(false);
  }, [busy, onBusy]);
  const aside = useRef<HTMLElement>(null);
  useEffect(() => {
    const node = aside.current,
      pane = node?.closest<HTMLElement>(".notation-pane");
    if (!node || !pane) return;
    const resize = new ResizeObserver(() => {
      const toolbar =
        pane.querySelector<HTMLElement>(".notation-toolbar")?.offsetHeight ??
        70;
      node.style.setProperty(
        "--score-editor-height",
        `${Math.max(180, pane.clientHeight - toolbar - 24)}px`,
      );
      node.style.setProperty("--score-editor-top", `${toolbar + 8}px`);
    });
    resize.observe(pane);
    return () => resize.disconnect();
  }, []);
  const alive = useRef(true),
    generation = useRef(0),
    working = useRef(false),
    profileTouched = useRef(false);
  useEffect(() => {
    alive.current = true;
    generation.current++;
    setResult(null);
    setChoices([]);
    setError("");
    setStatus("");
    setAll(false);
    profileTouched.current = false;
    const ticket = generation.current;
    if (note)
      void api
        .command<{
          profilesByHand: Record<string, string> | null;
          defaults: Record<string, string>;
        }>({ type: "fingerProfiles" })
        .then((v) => {
          if (
            alive.current &&
            generation.current === ticket &&
            !profileTouched.current
          )
            setProfile(
              v.profilesByHand?.[`${note.track}:${note.part}`] ??
                v.defaults[note.part === "left" ? "LeftHand" : "RightHand"] ??
                "Standard",
            );
        })
        .catch(
          (e) =>
            alive.current &&
            generation.current === ticket &&
            setError(String(e.message)),
        );
    return () => {
      alive.current = false;
    };
  }, [song.contentId, selected.track, selected.index, note?.part]);
  const perform = async (job: () => Promise<void>) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    setStatus("");
    try {
      await job();
    } catch (e) {
      if (alive.current) setError(e instanceof Error ? e.message : String(e));
    } finally {
      working.current = false;
      if (alive.current) setBusy(false);
    }
  };
  const save = (finger: number | null) =>
    void perform(async () => {
      const ticket = generation.current;
      const targets = all ? positions : [selected];
      const unique = new Map(targets.map((p) => [`${p.track}:${p.index}`, p]));
      await api.command({
        type: "editFingersFor",
        content_id: song.contentId,
        edits: Array.from(unique.values(), (p) => ({
          track_id: p.track,
          note_index: p.index,
          finger,
        })),
      });
      await changed();
      if (alive.current && generation.current === ticket) {
        setResult(null);
        setStatus(
          finger ? `已保存 ${unique.size} 个音符的指法` : "已清除个人指法覆盖",
        );
      }
    });
  const suggest = () =>
    void perform(async () => {
      const ticket = generation.current;
      if (!note || note.part === "other")
        throw new Error("请先给此音符指定左右手");
      const v = await api.command<{
        contentId: string;
        plans: { proposals: Proposal[] }[];
      }>({
        type: "suggestFingerPlans",
        request: {
          content_id: song.contentId,
          track: note.track,
          index: note.index,
          hand: note.part === "left" ? "LeftHand" : "RightHand",
          profile,
          range: null,
          keep_saved: false,
          pins: [],
        },
      });
      if (v.contentId !== song.contentId)
        throw new Error("曲目已切换，请重新生成建议");
      const unique = new Map<number, Proposal>();
      for (const plan of v.plans) {
        const p = plan.proposals.find(
          (p) => p.track === note.track && p.index === note.index,
        );
        if (p && !unique.has(p.finger)) unique.set(p.finger, p);
      }
      const options = Array.from(unique.values());
      const proposal = options[0];
      if (!proposal) throw new Error("这枚音符没有可用建议，请检查逐音分手");
      if (alive.current && generation.current === ticket) {
        setChoices(options);
        setResult(proposal);
      }
    });
  if (!note)
    return (
      <aside className="score-note-editor">
        <p role="alert">音符身份已改变，请重新点选。</p>
        <button onClick={close}>关闭</button>
      </aside>
    );
  const measure =
    song.measures.find((m) => note.start >= m.start && note.start < m.end)
      ?.number ?? 1;
  return (
    <aside ref={aside} className="score-note-editor" aria-label="谱面指法编辑">
      <div className="score-editor-heading">
        <strong>
          {pitchName(note.pitch)} · 第 {measure} 小节
        </strong>
        <button aria-label="关闭谱面指法编辑" disabled={workingBusy} onClick={close}>
          ×
        </button>
      </div>
      <p>
        {song.tracks.find((t) => t.id === note.track)?.name ??
          `音轨 ${note.track + 1}`}{" "}
        ·{" "}
        {note.part === "left"
          ? "左手"
          : note.part === "right"
            ? "右手"
            : "未分手"}
      </p>
      {positions.length > 1 && (
        <label>
          演奏位置
          <select
            aria-label="谱音演奏位置"
            value={`${selected.track}:${selected.index}`}
            disabled={busy}
            onChange={(e) => {
              const p = positions.find(
                (p) => `${p.track}:${p.index}` === e.target.value,
              );
              if (p) choose(p);
            }}
          >
            {positions.map((p, i) => (
              <option
                key={`${p.track}:${p.index}`}
                value={`${p.track}:${p.index}`}
              >
                第 {i + 1} 次 · 第{" "}
                {song.measures.find(
                  (m) => p.start >= m.start && p.start < m.end,
                )?.number ?? 1}{" "}
                小节
              </option>
            ))}
          </select>
        </label>
      )}
      <label>
        此音分手
        <select
          aria-label="谱音左右手"
          disabled={busy}
          value={
            note.part === "left"
              ? "LeftHand"
              : note.part === "right"
                ? "RightHand"
                : "Other"
          }
          onChange={(e) => {
            const part = e.target.value;
            void perform(async () => {
              await api.command({
                type: "applyHands",
                content_id: song.contentId,
                hints: [{ track_id: note.track, note_index: note.index, part }],
              });
              await changed();
            });
          }}
        >
          <option value="Other">未分手</option>
          <option value="LeftHand">左手</option>
          <option value="RightHand">右手</option>
        </select>
      </label>
      <dl className="score-finger-values">
        <dt>原谱指法</dt>
        <dd>{selected.scoreFinger ?? "未标注"}</dd>
        <dt>当前练习指法</dt>
        <dd>{note.finger ?? "未设置"}</dd>
      </dl>
      {!!actionsForNote(song,note).length&&<section className="score-held-actions" aria-label="此谱音持音换指">
        <strong>此演奏位置的持音换指</strong>
        <HeldFingerDemo song={song} track={note.track} index={note.index} disabled={workingBusy} onActive={setDemonstrating}/>
        {actionsForNote(song,note).map((a,i)=>{
          const next=song.notes.find(n=>n.track===a.beforeTrack&&n.index===a.beforeIndex);
          return <div key={i}>
            <span>{musicalActionPosition(song,a.at)} · {a.from} → {a.to} 指</span>
            <small>{actionUsable(a,rate)?"保持按键，用接替手指接住":"当前指法、分手或速度需要重新审阅，练习提示已停用"}</small>
            <div><button disabled={busy} onClick={()=>seek(a.at)}>定位换指</button>
              <button disabled={busy} onClick={()=>{if(!locate(a.beforeTrack,a.beforeIndex))setError("衔接音没有可靠谱面对应；可用定位换指查看演奏位置。");}}>查看衔接音{next?` ${pitchName(next.pitch)}`:""}</button></div>
            <HeldActionEditor key={`${song.contentId}:${a.track}:${a.index}:${a.atTick}`} song={song} action={a} rate={rate} busy={busy} perform={perform} changed={changed}/>
          </div>;
        })}
        <button disabled={busy} onClick={()=>void perform(async()=>{
          await api.command({type:"clearNoteHeldFingerActions",content_id:song.contentId,track:note.track,index:note.index});await changed();setStatus("已清除此演奏位置的换指，起音指法保留；可撤销。");
        })}>清除此演奏位置的换指</button>
      </section>}
      <div className="score-finger-buttons" aria-label="选择手指">
        {[1, 2, 3, 4, 5].map((f) => (
          <button
            key={f}
            disabled={busy}
            className={note.finger === f ? "selected" : ""}
            onClick={() => save(f)}
          >
            {f}
          </button>
        ))}
      </div>
      <button disabled={busy} onClick={() => save(null)}>
        清除个人覆盖
      </button>
      {positions.length > 1 && (
        <label className="score-editor-check">
          <input
            type="checkbox"
            checked={all}
            disabled={busy}
            onChange={(e) => setAll(e.target.checked)}
          />
          应用到该谱音的全部 {positions.length} 个演奏位置
        </label>
      )}
      <div className="score-recommend-options">
        <label>
          手型跨度
          <select
            aria-label="谱面指法手型"
            value={profile}
            disabled={busy}
            onChange={(e) => {
              profileTouched.current = true;
              setProfile(e.target.value);
              setResult(null);
            }}
          >
            <option value="Compact">较小</option>
            <option value="Standard">标准</option>
            <option value="Large">较大</option>
          </select>
        </label>
        <button disabled={busy || note.part === "other"} onClick={suggest}>
          推荐此音指法
        </button>
        {note.part === "other" && (
          <button disabled={busy} onClick={assignHands}>
            指定左右手
          </button>
        )}
      </div>
      {result && (
        <div className="score-note-proposal">
          {choices.length > 1 && (
            <label>
              此音备选手指
              <select
                aria-label="谱音备选手指"
                disabled={busy}
                value={result.finger}
                onChange={(e) =>
                  setResult(
                    choices.find((p) => p.finger === Number(e.target.value)) ??
                      result,
                  )
                }
              >
                {choices.map((p) => (
                  <option key={p.finger} value={p.finger}>
                    {p.finger} 指
                  </option>
                ))}
              </select>
            </label>
          )}
          <strong>
            建议 {result.finger} 指 · {result.confidence}%
          </strong>
          <p>{result.reason}</p>
          <button disabled={busy} onClick={() => save(result.finger)}>
            采用推荐指法
          </button>
          <small>
            仅保存此音的选择；整段衔接请打开和弦 /
            选段建议。规则评分不是正确率。
          </small>
        </div>
      )}
      <div className="score-editor-footer">
        <button
          disabled={busy || note.part === "other"}
          onClick={() => detail(note)}
        >
          和弦 / 选段建议
        </button>
        <button
          disabled={busy || !song.fingerUndoAvailable}
          onClick={() =>
            void perform(async () => {
              await api.command({
                type: "undoFingersFor",
                content_id: song.contentId,
              });
              await changed();
              if (alive.current) {
                setResult(null);
                setStatus("已撤销上一笔指法修改");
              }
            })
          }
        >
          撤销上一笔指法
        </button>
      </div>
      {busy && <p role="status">正在处理…</p>}
      {status && (
        <p role="status" className="score-edit-success">
          {status}
        </p>
      )}
      {error && (
        <p role="alert" className="dialog-error">
          {error}
        </p>
      )}
      <p className="score-editor-hint">
        彩色数字为当前练习指法。原谱的印刷标记保留；生成练习清除个人覆盖后使用默认指法。
      </p>
    </aside>
  );
}
