import { useEffect, useState } from "react";
import { TickRangeDialog, type TickRange } from "./TickRangeDialog";
import { X } from "lucide-react";
import { api, type LoadedSong, type Snapshot } from "./api";
import { ScoreRangeDialog, type ScoreRange } from "./ScoreRangeDialog";
type Passage = {
  preciseRange?: TickRange;
  scoreRange?: ScoreRange;
  id: string;
  name: string;
  start: number;
  end: number;
  notes: string;
  speed: number;
  settings: { mode: string; hands: string; countIn: number; rounds: number };
};
export function PassageDialog({
  song,
  state,
  start,
  end,
  close,
  changed,
}: {
  song: LoadedSong;
  state: Snapshot;
  start: number;
  end: number;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [precise, setPrecise] = useState<Passage | "new" | null>(null);
  const [review, setReview] = useState<Passage | null>(null);
  const [passages, setPassages] = useState<Passage[]>([]),
    [id, setId] = useState<string | null>(null),
    [name, setName] = useState(""),
    [first, setFirst] = useState(start),
    [last, setLast] = useState(end),
    [notes, setNotes] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const reload = async () => {
    const rows = (
      await api.command<{ passages: Passage[] }>({ type: "passages" })
    ).passages;
    setPassages(rows);
    return rows;
  };
  useEffect(() => {
    void reload().catch((e) => setError(String(e.message)));
  }, [song.contentId]);
  const work = async (job: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await job();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  const reset = () => {
    setId(null);
    setName("");
    setFirst(start);
    setLast(end);
    setNotes("");
  };
  const active = passages.find((p) => p.id === id);
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!busy) close();
      }}
    >
      <section
        className="settings-dialog passage-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="passage-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="passage-title">练习段落</h2>
            <p>{song.title}</p>
          </div>
          <button
            autoFocus
            disabled={busy}
            aria-label="关闭练习段落"
            onClick={close}
          >
            <X size={18} />
          </button>
        </div>
        {error && (
          <p role="alert" className="dialog-error">
            {error}
          </p>
        )}
        <div className="passage-layout">
          <div className="passage-list">
            {passages.map((p) => (
              <article key={p.id} className={id === p.id ? "active" : ""}>
                <button
                  className="passage-select"
                  onClick={() => {
                    setId(p.id);
                    setName(p.name);
                    setFirst(p.start);
                    setLast(p.end);
                    setNotes(p.notes);
                  }}
                >
                  <strong>{p.name}</strong>
                  <span>
                    {p.preciseRange?.description ??
                      p.scoreRange?.description ??
                      `第 ${p.start}–${p.end} 小节`}{" "}
                    · {Math.round(p.speed * 100)}% ·{" "}
                    {p.settings.mode === "flow"
                      ? "连续"
                      : p.settings.mode === "listen"
                        ? "聆听"
                        : "等音"}
                  </span>
                  <small>{p.notes || "未添加备注"}</small>
                </button>
                <div>
                  <button
                    disabled={busy}
                    onClick={() =>
                      void work(async () => {
                        await api.command({ type: "openPassage", id: p.id });
                        await changed();
                        close();
                      })
                    }
                  >
                    练这一段
                  </button>
                  <button
                    disabled={busy}
                    onClick={() =>
                      void work(async () => {
                        await api.command({ type: "removePassage", id: p.id });
                        await reload();
                        if (id === p.id) reset();
                      })
                    }
                  >
                    移除
                  </button>
                </div>
              </article>
            ))}
            {!passages.length && (
              <p className="library-empty">
                保存经常重练的小节段。每一段都可以有自己的配置和练习备注。
              </p>
            )}
          </div>
          <div className="passage-form">
            <h3>{id ? "编辑练习段" : "新建练习段"}</h3>
            <label>
              名称
              <input
                aria-label="练习段名称"
                placeholder="例如：左手伴奏、转指、结尾"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <div className="passage-range">
              <label>
                起始小节
                <input
                  aria-label="保存段落起始小节"
                  type="number"
                  disabled={
                    busy || !!active?.scoreRange || !!active?.preciseRange
                  }
                  min={1}
                  max={song.measures.length}
                  value={first}
                  onChange={(e) => setFirst(Number(e.target.value))}
                />
              </label>
              <label>
                结束小节
                <input
                  aria-label="保存段落结束小节"
                  type="number"
                  disabled={
                    busy || !!active?.scoreRange || !!active?.preciseRange
                  }
                  min={first}
                  max={song.measures.length}
                  value={last}
                  onChange={(e) => setLast(Number(e.target.value))}
                />
              </label>
            </div>
            <button
              disabled={busy}
              onClick={() => setPrecise(active?.preciseRange ? active : "new")}
            >
              {active?.preciseRange ? "编辑拍内范围" : "新建拍内段落"}
            </button>
            {active?.preciseRange && <p>{active.preciseRange.description}</p>}
            {active?.scoreRange && (
              <div className="score-range-original">
                <p>{active.scoreRange.description}</p>
                <button disabled={busy} onClick={() => setReview(active)}>
                  重新审阅原谱范围
                </button>
              </div>
            )}
            <label>
              练习备注
              <textarea
                aria-label="练习段备注"
                rows={4}
                placeholder="要解决的问题、教师要求或下次练习目标"
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
              />
            </label>
            <p>
              保存时记住当前音轨、手别、练习模式、
              {Math.round(state.speed * 100)}%
              速度、预备拍和遍数。再次打开不会自动播放。
            </p>
            <div className="file-actions">
              <button
                className="primary-button"
                disabled={busy || !name.trim()}
                onClick={() =>
                  void work(async () => {
                    await api.command(
                      active?.scoreRange || active?.preciseRange
                        ? {
                            type: "updatePassageDetails",
                            content_id: song.contentId,
                            id: active.id,
                            name,
                            notes,
                          }
                        : {
                            type: "savePassage",
                            id,
                            name,
                            start: first,
                            end: last,
                            notes,
                          },
                    );
                    await reload();
                    reset();
                  })
                }
              >
                {id ? "更新练习段" : "保存练习段"}
              </button>
              <button onClick={reset}>新建</button>
            </div>
          </div>
        </div>
      </section>
      {precise && (
        <TickRangeDialog
          song={song}
          state={state}
          first={first}
          last={last}
          preset={
            precise !== "new" && precise.preciseRange
              ? { ...precise, preciseRange: precise.preciseRange }
              : undefined
          }
          close={() => setPrecise(null)}
          changed={async () => {
            await changed();
            const rows = await reload();
            if (precise !== "new") {
              const updated = rows.find((p) => p.id === precise.id);
              if (updated) {
                setName(updated.name);
                setNotes(updated.notes);
                setFirst(updated.start);
                setLast(updated.end);
              }
            }
          }}
        />
      )}
      {review?.scoreRange && (
        <ScoreRangeDialog
          song={song}
          first={review.scoreRange.firstVisit}
          last={review.scoreRange.lastVisit}
          preset={{ ...review, scoreRange: review.scoreRange }}
          close={() => setReview(null)}
          changed={async () => {
            await changed();
            const rows = await reload();
            const updated = rows.find((p) => p.id === review.id);
            if (updated) {
              setId(updated.id);
              setName(updated.name);
              setNotes(updated.notes);
              setFirst(updated.start);
              setLast(updated.end);
            }
          }}
        />
      )}
    </div>
  );
}
