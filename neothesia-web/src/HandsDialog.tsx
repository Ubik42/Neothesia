import { useMemo, useState } from "react";
import { X } from "lucide-react";
import { api, pitchName, type LoadedSong } from "./api";
type Part = "LeftHand" | "RightHand" | "Other";
type Proposal = {
  track: number;
  index: number;
  pitch: number;
  measure: number;
  part: Part;
  saved: Part;
  confidence: number;
  reason: string;
};
const nativePart = (part: string): Part =>
  part === "left" ? "LeftHand" : part === "right" ? "RightHand" : "Other";
export function HandsDialog({
  song,
  start,
  end,
  close,
  changed,
}: {
  song: LoadedSong;
  start: number;
  end: number;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [track, setTrack] = useState(song.tracks[0]?.id ?? 0);
  const [first, setFirst] = useState(start),
    [last, setLast] = useState(end),
    [reference, setReference] = useState(60);
  const [proposals, setProposals] = useState<Proposal[] | null>(null),
    [values, setValues] = useState<Record<number, Part>>({});
  const [accepted, setAccepted] = useState<Set<number>>(new Set()),
    [page, setPage] = useState(0),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const rows = useMemo(
    () =>
      proposals ??
      song.notes
        .filter(
          (n) =>
            n.track === track &&
            song.measures[first - 1] &&
            n.start >= song.measures[first - 1].start &&
            n.start < (song.measures[last - 1]?.end ?? 0),
        )
        .map((n) => ({
          track,
          index: n.index,
          pitch: n.pitch,
          measure:
            [...song.measures].reverse().find((m) => m.start <= n.start)
              ?.number ?? 1,
          part: nativePart(n.part),
          saved: nativePart(n.part),
          confidence: 0,
          reason: n.manualHand ? "已保存的逐音分手" : "沿用音轨声部",
        })),
    [song, track, first, last, proposals],
  );
  const perform = async (work: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await work();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  const reset = () => {
    setProposals(null);
    setValues({});
    setAccepted(new Set());
    setPage(0);
  };
  const visible = rows.slice(page * 40, (page + 1) * 40);
  return (
    <div className="modal-backdrop" onClick={close}>
      <section
        className="settings-dialog fingering-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="hands-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="hands-title">逐音分手</h2>
            <p>{song.title}</p>
          </div>
          <button autoFocus aria-label="关闭逐音分手" onClick={close}>
            <X size={18} />
          </button>
        </div>
        <div className="fingering-options">
          <label>
            音轨
            <select
              aria-label="分手音轨"
              value={track}
              disabled={busy}
              onChange={(e) => {
                setTrack(Number(e.target.value));
                reset();
              }}
            >
              {song.tracks.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name} · {t.notes} 音
                </option>
              ))}
            </select>
          </label>
          <label>
            起始小节
            <input
              aria-label="分手起始小节"
              type="number"
              min={1}
              max={song.measures.length}
              value={first}
              onChange={(e) => {
                setFirst(Number(e.target.value));
                reset();
              }}
            />
          </label>
          <label>
            结束小节
            <input
              aria-label="分手结束小节"
              type="number"
              min={first}
              max={song.measures.length}
              value={last}
              onChange={(e) => {
                setLast(Number(e.target.value));
                reset();
              }}
            />
          </label>
          <label>
            参考手位
            <select
              aria-label="分手参考手位"
              value={reference}
              onChange={(e) => {
                setReference(Number(e.target.value));
                reset();
              }}
            >
              {[48, 53, 55, 60, 65, 67, 72].map((p) => (
                <option key={p} value={p}>
                  {pitchName(p)}
                </option>
              ))}
            </select>
          </label>
          <button
            className="primary-button"
            disabled={busy}
            onClick={() =>
              void perform(async () => {
                const data = await api.command<{
                  contentId: string;
                  proposals: Proposal[];
                }>({
                  type: "suggestHands",
                  track,
                  range: [first, last],
                  reference,
                });
                if (data.contentId !== song.contentId)
                  throw new Error("曲目已切换");
                setProposals(data.proposals);
                setValues({});
                setAccepted(new Set(data.proposals.map((p) => p.index)));
                setPage(0);
              })
            }
          >
            生成分手建议
          </button>
        </div>
        <p className="parameter-help">
          建议结合和弦跨度、持续音和前后声部走向。参考手位可以调整；交叉声部请逐音确认。保存后用于分手练习、伴奏和指法推荐。
        </p>
        <div className="fingering-options">
          <button
            disabled={busy || rows.length > 4096}
            onClick={() => setAccepted(new Set(rows.map((p) => p.index)))}
          >
            选择全部音符
          </button>
          <button disabled={busy} onClick={() => setAccepted(new Set())}>
            取消选择
          </button>
          {(["LeftHand", "RightHand", "Other"] as Part[]).map((part) => (
            <button
              disabled={busy || !accepted.size}
              key={part}
              onClick={() =>
                setValues((v) =>
                  Object.assign(
                    {},
                    v,
                    Object.fromEntries([...accepted].map((i) => [i, part])),
                  ),
                )
              }
            >
              {part === "LeftHand"
                ? "所选设为左手"
                : part === "RightHand"
                  ? "所选设为右手"
                  : "所选设为未分手"}
            </button>
          ))}
        </div>
        {error && (
          <p role="alert" className="dialog-error">
            {error}
          </p>
        )}
        <div className="fingering-table-wrap">
          <table className="fingering-table">
            <thead>
              <tr>
                <th>采用</th>
                <th>小节</th>
                <th>音符</th>
                <th>当前手别</th>
                <th>演奏手</th>
                <th>建议依据</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((p) => (
                <tr key={p.index}>
                  <td>
                    <input
                      aria-label={`采用分手 ${p.index}`}
                      type="checkbox"
                      checked={accepted.has(p.index)}
                      disabled={busy}
                      onChange={(e) =>
                        setAccepted((old) => {
                          const next = new Set(old);
                          if (e.target.checked) next.add(p.index);
                          else next.delete(p.index);
                          return next;
                        })
                      }
                    />
                  </td>
                  <td>{p.measure}</td>
                  <td>{pitchName(p.pitch)}</td>
                  <td>
                    {p.saved === "LeftHand"
                      ? "左手"
                      : p.saved === "RightHand"
                        ? "右手"
                        : "未分手"}
                  </td>
                  <td>
                    <select
                      aria-label={`音符 ${p.index}演奏手`}
                      value={values[p.index] ?? p.part}
                      disabled={busy}
                      onChange={(e) => {
                        setValues((v) => ({
                          ...v,
                          [p.index]: e.target.value as Part,
                        }));
                        setAccepted((v) => new Set(v).add(p.index));
                      }}
                    >
                      <option value="LeftHand">左手</option>
                      <option value="RightHand">右手</option>
                      <option value="Other">未分手</option>
                    </select>
                  </td>
                  <td>
                    {p.reason}
                    {p.confidence > 0 && p.confidence < 60 && (
                      <span className="parameter-help"> · 需重点确认</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {rows.length === 0 && (
            <p className="parameter-help">这个选段没有音符。</p>
          )}
        </div>
        <div className="dialog-actions">
          <button
            disabled={busy}
            onClick={() =>
              void perform(async () => {
                await api.command({ type: "undoHands" });
                await changed();
                reset();
              })
            }
          >
            撤销上次分手修改
          </button>
          <span>
            {rows.length} 音 · 已选择 {accepted.size}
          </span>
          <button disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
            上一页
          </button>
          <span>
            {page + 1} / {Math.max(1, Math.ceil(rows.length / 40))}
          </span>
          <button
            disabled={(page + 1) * 40 >= rows.length}
            onClick={() => setPage((p) => p + 1)}
          >
            下一页
          </button>
          <button
            className="primary-button"
            disabled={busy || !accepted.size || accepted.size > 4096}
            onClick={() =>
              void perform(async () => {
                await api.command({
                  type: "applyHands",
                  content_id: song.contentId,
                  hints: rows
                    .filter((p) => accepted.has(p.index))
                    .map((p) => ({
                      track_id: p.track,
                      note_index: p.index,
                      part: values[p.index] ?? p.part,
                    })),
                });
                await changed();
                reset();
              })
            }
          >
            保存所选分手
          </button>
          <button onClick={close}>完成</button>
        </div>
      </section>
    </div>
  );
}
