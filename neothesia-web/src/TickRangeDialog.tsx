import { useEffect, useRef, useState } from "react";
import { api, type LoadedSong, type Snapshot } from "./api";
export type TickRange = {
  contentId: string;
  grid: string;
  startTick: number;
  endTick: number;
  description: string;
};
type Point = { measure: number; beat: number };
type Preview = {
  range: TickRange;
  description: string;
  notes: number;
  targets: number;
  carry: number;
};
type Preset = {
  id: string;
  name: string;
  notes: string;
  preciseRange: TickRange;
};
export function TickRangeDialog({
  song,
  state,
  first,
  last,
  preset,
  close,
  changed,
}: {
  song: LoadedSong;
  state: Snapshot;
  first: number;
  last: number;
  preset?: Preset;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const length = (m: LoadedSong["measures"][number]) =>
    ((song.ppq * 4) / m.denominator) *
    (m.denominator === 8 && m.numerator >= 6 && m.numerator % 3 === 0 ? 3 : 1);
  const point = (tick: number, end = false): Point => {
    let index = song.measures.findIndex((m) =>
      end
        ? tick > m.startTick && tick <= m.endTick
        : tick >= m.startTick && tick < m.endTick,
    );
    if (index < 0) index = song.measures.length - 1;
    const m = song.measures[index];
    return { measure: index + 1, beat: 1 + (tick - m.startTick) / length(m) };
  };
  const endPoint = (bar: number) => {
    let tick = song.measures[bar - 1].endTick;
    const tempo = [...song.tempo]
      .reverse()
      .find((t) => t.seconds <= song.duration);
    if (tempo)
      tick = Math.min(
        tick,
        Math.round(
          tempo.tick +
            ((song.duration - tempo.seconds) * song.ppq * tempo.bpm) / 60,
        ),
      );
    return point(tick, true);
  };
  first = Math.min(song.measures.length, Math.max(1, first || 1));
  last = Math.min(song.measures.length, Math.max(first, last || first));
  const [savedId, setSavedId] = useState<string | null>(preset?.id ?? null);
  const [a, setA] = useState<Point>({ measure: first, beat: 1 }),
    [b, setB] = useState<Point>(() => endPoint(last)),
    [grid, setGrid] = useState(""),
    [name, setName] = useState(preset?.name ?? ""),
    [notes, setNotes] = useState(preset?.notes ?? ""),
    [preview, setPreview] = useState<Preview | null>(null),
    [busy, setBusy] = useState(false),
    [dirty, setDirty] = useState(false),
    [error, setError] = useState(""),
    [saved, setSaved] = useState(false);
  const alive = useRef(true),
    working = useRef(false),
    section = useRef<HTMLElement>(null),
    draft = useRef(false);
  draft.current = dirty;
  working.current = busy;
  const exit = () => {
    if (
      !working.current &&
      (!draft.current || window.confirm("拍内段落尚未保存，放弃这些修改？"))
    )
      close();
  };
  useEffect(() => {
    alive.current = true;
    void api
      .command<{ grid: string }>({ type: "passages" })
      .then((v) => {
        if (!alive.current) return;
        setGrid(v.grid);
        if (preset) {
          if (
            preset.preciseRange.contentId === song.contentId &&
            preset.preciseRange.grid === v.grid
          ) {
            setA(point(preset.preciseRange.startTick));
            setB(point(preset.preciseRange.endTick, true));
          } else setError("原小节网格已改变，请重新设置范围并预览后保存。");
        }
      })
      .catch((e) => {
        if (alive.current) setError(String(e));
      });
    return () => {
      alive.current = false;
    };
  }, []);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    section.current?.querySelector<HTMLElement>("button")?.focus();
    const listener = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        exit();
      }
      if (e.key === "Tab") {
        e.stopPropagation();
        const nodes = [
          ...section.current!.querySelectorAll<HTMLElement>(
            "button:not(:disabled),input:not(:disabled),textarea:not(:disabled)",
          ),
        ].filter((n) => n.offsetParent !== null);
        const first = nodes[0],
          last = nodes.at(-1);
        if (
          !section.current?.contains(document.activeElement) ||
          (e.shiftKey && document.activeElement === first)
        ) {
          e.preventDefault();
          (e.shiftKey ? last : first)?.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first?.focus();
        }
      }
    };
    document.addEventListener("keydown", listener, true);
    const unload = (e: BeforeUnloadEvent) => {
      if (draft.current) {
        e.preventDefault();
        e.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", unload);
    return () => {
      document.removeEventListener("keydown", listener, true);
      window.removeEventListener("beforeunload", unload);
      if (previous?.isConnected) previous.focus();
    };
  }, []);
  const edit = () => {
    setDirty(true);
    setSaved(false);
  };
  const boundary = (v: Point, set: (v: Point) => void, title: string) => {
    const m = song.measures[v.measure - 1];
    const max = m ? 1 + (m.endTick - m.startTick) / length(m) : 1;
    return (
      <fieldset className="tick-boundary">
        <legend>{title}</legend>
        <label>
          小节
          <input
            aria-label={`${title}小节`}
            type="number"
            min={1}
            max={song.measures.length}
            value={v.measure}
            disabled={busy}
            onChange={(e) => {
              set({ measure: Number(e.target.value), beat: 1 });
              setPreview(null);
              edit();
            }}
          />
        </label>
        <label>
          拍位
          <input
            aria-label={`${title}拍位`}
            type="number"
            min={1}
            max={max}
            step="any"
            value={v.beat}
            disabled={busy}
            onChange={(e) => {
              set({ ...v, beat: Number(e.target.value) });
              setPreview(null);
              edit();
            }}
          />
        </label>
        <small>
          {m
            ? `${m.numerator}/${m.denominator}${m.partial ? " · 不完整小节" : ""} · ${m.denominator === 8 && m.numerator >= 6 && m.numerator % 3 === 0 ? "每拍三个八分音符" : "按拍号计拍"}`
            : "小节超出曲目"}
        </small>
      </fieldset>
    );
  };
  const work = async (job: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await job();
    } catch (e) {
      if (alive.current) setError(e instanceof Error ? e.message : String(e));
    } finally {
      if (alive.current) setBusy(false);
    }
  };
  const blocked =
    !!state.routine ||
    !!state.recording ||
    ["playing", "countIn"].includes(state.status);
  return (
    <div
      className="modal-backdrop"
      onClick={(e) => {
        e.stopPropagation();
        exit();
      }}
    >
      <section
        ref={section}
        className="settings-dialog tick-range-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="tick-range-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="tick-range-title">拍内练习段落</h2>
            <p>{song.title}</p>
          </div>
          <button disabled={busy} onClick={exit}>
            关闭拍内段落
          </button>
        </div>
        <p>
          按实际演奏顺序的小节与拍位选择。1.5
          表示第一拍的后半拍；终点处的起音不计入。
        </p>
        <div className="tick-boundaries">
          {boundary(a, setA, "起点")}
          {boundary(b, setB, "终点")}
        </div>
        <div className="file-actions">
          <button
            disabled={busy || !grid}
            onClick={() =>
              void work(async () => {
                const v = await api.command<Preview>({
                  type: "previewTickRange",
                  content_id: song.contentId,
                  grid,
                  start: a,
                  end: b,
                });
                if (alive.current) {
                  setPreview(v);
                  setSaved(false);
                }
              })
            }
          >
            预览拍内范围
          </button>
          <button
            disabled={busy}
            onClick={() => {
              setA({ measure: first, beat: 1 });
              setB(endPoint(last));
              setPreview(null);
              edit();
            }}
          >
            采用当前小节范围
          </button>
        </div>
        {error && (
          <p className="dialog-error" role="alert">
            {error}
          </p>
        )}
        {preview && (
          <div className="tick-preview" role="status">
            <strong>{preview.description}</strong>
            <p>
              {preview.notes} 个起音 · {preview.targets} 个自己弹的目标音
            </p>
            {preview.carry > 0 && (
              <p>
                起点前有 {preview.carry}{" "}
                个延续音；从中途开始不会把它们作为新的按键目标。
              </p>
            )}
            <p>
              边界取曲目可表达的最接近拍位。按原曲变速计算，不随练习速度改变。
            </p>
          </div>
        )}
        <label>
          段落名称
          <input
            aria-label="拍内段落名称"
            maxLength={80}
            value={name}
            disabled={busy}
            onChange={(e) => {
              setName(e.target.value);
              edit();
            }}
          />
        </label>
        <label>
          练习备注
          <textarea
            aria-label="拍内段落备注"
            rows={3}
            value={notes}
            disabled={busy}
            onChange={(e) => {
              setNotes(e.target.value);
              edit();
            }}
          />
        </label>
        <p>
          保存当前声部、分手、模式、速度、预备拍与遍数。打开后保持暂停，可加入练习计划。
        </p>
        <div className="file-actions">
          <button
            disabled={busy || !preview || blocked}
            onClick={() =>
              void work(async () => {
                await api.command({
                  type: "applyTickRange",
                  range: preview!.range,
                });
                await changed();
                if (alive.current) {
                  setDirty(false);
                  close();
                }
              })
            }
          >
            练这个范围
          </button>
          <button
            className="primary-button"
            disabled={busy || !preview || !name.trim() || blocked}
            onClick={() =>
              void work(async () => {
                const result = await api.command<{ passage: { id: string } }>({
                  type: "saveTickPassage",
                  id: savedId,
                  name,
                  notes,
                  range: preview!.range,
                });
                await changed();
                if (alive.current) {
                  setDirty(false);
                  setSavedId(result.passage.id);
                  setSaved(true);
                }
              })
            }
          >
            {savedId ? "更新拍内段落" : "保存拍内段落"}
          </button>
        </div>
        {blocked && <p>先停止当前演奏、录音或活动计划，再采用或保存范围。</p>}
        {saved && <p role="status">拍内段落已保存。</p>}
      </section>
    </div>
  );
}
