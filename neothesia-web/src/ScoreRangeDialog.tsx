import { useEffect, useMemo, useRef, useState } from "react";
import { X } from "lucide-react";
import { api, type LoadedSong } from "./api";
import type { WrittenRoute, WrittenVisit } from "./ScoreRoute";

export interface ScoreRange {
  startTrim?: number;
  endTrim?: number;
  scoreIdentity: string;
  routeIdentity: string;
  firstVisit: number;
  lastVisit: number;
  startTick: number;
  endTick: number;
  description: string;
}
interface Limits {
  minBeat: number;
  maxBeat: number;
  ticksPerBeat: number;
}
interface Preview {
  firstLimits: Limits;
  lastLimits: Limits;
  range: ScoreRange;
  crossings: number;
  notes: number;
  rows: WrittenVisit[];
}
export function ScoreRangeDialog({
  song,
  first: initialFirst,
  last: initialLast,
  preset,
  close,
  changed,
}: {
  song: LoadedSong;
  first: number;
  last: number;
  preset?: { id: string; name: string; notes: string; scoreRange: ScoreRange };
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [startTrim, setStartTrim] = useState(preset?.scoreRange.startTrim ?? 0),
    [endTrim, setEndTrim] = useState(preset?.scoreRange.endTrim ?? 0);
  const [startText, setStartText] = useState(""),
    [endText, setEndText] = useState(""),
    [beatDirty, setBeatDirty] = useState(false);
  const limitsCache = useRef<{
    first: number;
    last: number;
    a: Limits;
    b: Limits;
  } | null>(null);
  const [dirty, setDirty] = useState(false);
  const dirtyRef = useRef(false);
  dirtyRef.current = dirty;
  const [route, setRoute] = useState<WrittenRoute | null>(null);
  const [first, setFirst] = useState(initialFirst),
    [last, setLast] = useState(initialLast);
  const [name, setName] = useState(preset?.name ?? ""),
    [notes, setNotes] = useState(preset?.notes ?? "");
  const [preview, setPreview] = useState<Preview | null>(null),
    [confirmed, setConfirmed] = useState(false);
  const [page, setPage] = useState(0),
    [reading, setReading] = useState(true),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const busyRef = useRef(false);
  const exit = () => {
    if (
      !busyRef.current &&
      (!dirtyRef.current || window.confirm("原谱选段尚未保存，放弃这些修改？"))
    )
      close();
  };
  useEffect(() => {
    const keyboard = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        exit();
      }
    };
    const unload = (e: BeforeUnloadEvent) => {
      if (dirtyRef.current) {
        e.preventDefault();
        e.returnValue = "";
      }
    };
    document.addEventListener("keydown", keyboard, true);
    window.addEventListener("beforeunload", unload);
    return () => {
      document.removeEventListener("keydown", keyboard, true);
      window.removeEventListener("beforeunload", unload);
    };
  }, []);
  const entryFocus = useRef(
    document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null,
  );
  useEffect(
    () => () => {
      if (entryFocus.current?.isConnected) entryFocus.current.focus();
    },
    [],
  );
  useEffect(() => {
    let disposed = false;
    api
      .command<{
        contentId: string;
        scoreRevision: number;
        route: WrittenRoute | null;
      }>({ type: "score" })
      .then((asset) => {
        if (disposed) return;
        if (
          !asset ||
          asset.contentId !== song.contentId ||
          asset.scoreRevision !== song.scoreRevision
        )
          throw new Error("曲目或谱面已切换，请重新打开原谱选段");
        if (!asset.route?.complete)
          throw new Error("谱面演奏顺序或音符对应需要校对");
        setRoute(asset.route);
      })
      .catch((e) => {
        if (!disposed) {
          setError(String(e.message));
          setReading(false);
        }
      });
    return () => {
      disposed = true;
    };
  }, [song.contentId, song.scoreRevision]);
  useEffect(() => {
    if (!route) return;
    let disposed = false;
    setPreview(null);
    setConfirmed(false);
    setPage(0);
    setReading(true);
    setError("");
    api
      .command<Preview>({
        type: "previewScoreRangeTrim",
        content_id: song.contentId,
        score_revision: song.scoreRevision,
        first,
        last,
        start_trim: startTrim,
        end_trim: endTrim,
      })
      .then((v) => {
        if (!disposed) {
          setPreview(v);
          limitsCache.current = {
            first,
            last,
            a: v.firstLimits,
            b: v.lastLimits,
          };
          setStartText(
            String(
              v.firstLimits.minBeat +
                (v.range.startTrim ?? 0) / v.firstLimits.ticksPerBeat,
            ),
          );
          setEndText(
            String(
              v.lastLimits.maxBeat -
                (v.range.endTrim ?? 0) / v.lastLimits.ticksPerBeat,
            ),
          );
          setBeatDirty(false);
        }
      })
      .catch((e) => {
        if (!disposed) setError(String(e.message));
      })
      .finally(() => {
        if (!disposed) setReading(false);
      });
    return () => {
      disposed = true;
    };
  }, [
    route,
    first,
    last,
    startTrim,
    endTrim,
    song.contentId,
    song.scoreRevision,
  ]);
  const measures = useMemo(
    () =>
      Array.from(
        new Map(route?.visits.map((v) => [v.measure, v]) ?? []).values(),
      ).sort((a, b) => a.measure - b.measure),
    [route],
  );
  const endpoint = (
    kind: "起点" | "终点",
    ordinal: number,
    update: (v: number) => void,
  ) => {
    const visit = route?.visits[ordinal];
    const visits =
      route?.visits.filter((v) => v.measure === visit?.measure) ?? [];
    return (
      <fieldset>
        <legend>{kind}</legend>
        <label>
          原谱小节
          <select
            aria-label={`${kind}原谱小节`}
            value={visit?.measure ?? -1}
            disabled={busy || !route}
            onChange={(e) => {
              const options = route!.visits.filter(
                (v) => v.measure === Number(e.target.value),
              );
              const preferred = kind === "终点" ? first : ordinal;
              const next = (
                options.find((v) => v.ordinal >= preferred) ?? options[0]
              ).ordinal;
              update(next);
              setStartTrim(0);
              setEndTrim(0);
              setDirty(true);
              if (kind === "起点" && next > last) setLast(next);
            }}
          >
            {!visit && <option value={-1}>原演奏位置已不存在</option>}
            {measures.map((m) => (
              <option key={m.measure} value={m.measure}>
                第 {m.number || m.measure + 1} 小节
              </option>
            ))}
          </select>
        </label>
        <label>
          演奏次数
          <select
            aria-label={`${kind}演奏次数`}
            value={ordinal}
            disabled={busy || !visit}
            onChange={(e) => {
              const next = Number(e.target.value);
              update(next);
              setStartTrim(0);
              setEndTrim(0);
              setDirty(true);
              if (kind === "起点" && next > last) setLast(next);
            }}
          >
            {visits.map((v, i) => (
              <option key={v.ordinal} value={v.ordinal}>
                第 {i + 1} 次 · 全曲第 {v.ordinal + 1} 段
                {v.partialStart || v.partialEnd ? " · 部分小节" : ""}
              </option>
            ))}
          </select>
        </label>
        <label>
          原谱拍位
          <input
            aria-label={`${kind}原谱拍位`}
            type="number"
            step="any"
            value={kind === "起点" ? startText : endText}
            disabled={
              busy ||
              !limitsCache.current ||
              limitsCache.current.first !== first ||
              limitsCache.current.last !== last
            }
            onChange={(e) => {
              if (kind === "起点") setStartText(e.target.value);
              else setEndText(e.target.value);
              setBeatDirty(true);
              setDirty(true);
              setConfirmed(false);
            }}
          />
        </label>
      </fieldset>
    );
  };
  const reviewBeats = () => {
    const limits = limitsCache.current;
    if (!limits || limits.first !== first || limits.last !== last) return;
    const a = Number(startText),
      b = Number(endText);
    if (
      !startText.trim() ||
      !endText.trim() ||
      !Number.isFinite(a) ||
      !Number.isFinite(b) ||
      a < limits.a.minBeat ||
      a > limits.a.maxBeat ||
      b < limits.b.minBeat ||
      b > limits.b.maxBeat
    ) {
      setError("拍位超出所选演奏次数的实际范围");
      return;
    }
    const x = Math.round((a - limits.a.minBeat) * limits.a.ticksPerBeat),
      y = Math.round((limits.b.maxBeat - b) * limits.b.ticksPerBeat);
    setStartTrim(x);
    setEndTrim(y);
    setBeatDirty(false);
    setError("");
  };
  const allowed =
    !!preview &&
    preview.range.firstVisit === first &&
    preview.range.lastVisit === last &&
    (preview.range.startTrim??0)===startTrim && (preview.range.endTrim??0)===endTrim &&
    !reading &&
    !beatDirty &&
    !busy &&
    (preview.crossings === 0 || confirmed);
  const work = async (save: boolean) => {
    if (!allowed || !preview || busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setError("");
    try {
      await api.command({
        type: save ? "saveScorePassage" : "applyScoreRange",
        content_id: song.contentId,
        score_revision: song.scoreRevision,
        range: preview.range,
        ...(save ? { id: preset?.id ?? null, name, notes } : {}),
      });
      await changed();
      close();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  };
  return (
    <div
      className="modal-backdrop"
      onClick={(e) => {
        e.stopPropagation();
        exit();
      }}
    >
      <section
        className="settings-dialog score-range-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="score-range-title"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Tab") {
            e.stopPropagation();
            const items = [
              ...e.currentTarget.querySelectorAll<HTMLElement>(
                "button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),a[href],[tabindex]",
              ),
            ].filter(
              (el) => el.tabIndex >= 0 && el.getClientRects().length > 0,
            );
            const first = items[0],
              last = items.at(-1);
            if (e.shiftKey && document.activeElement === first) {
              e.preventDefault();
              last?.focus();
            } else if (!e.shiftKey && document.activeElement === last) {
              e.preventDefault();
              first?.focus();
            }
          }
          if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            exit();
          }
        }}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="score-range-title">原谱选段练习</h2>
            <p>{song.title}</p>
          </div>
          <button
            autoFocus
            disabled={busy}
            aria-label="关闭原谱选段"
            onClick={exit}
          >
            <X size={18} />
          </button>
        </div>
        {preset && (
          <p className="score-range-original">
            原保存范围：{preset.scoreRange.description}
            。保存将使用下面重新审阅的范围。
          </p>
        )}
        <div className="score-range-endpoints">
          {endpoint("起点", first, setFirst)}
          {endpoint("终点", last, setLast)}
        </div>
        <p>
          拍位按原谱拍号的分母计拍，可用小数；6/8
          的一拍为八分音符。端点限定于所选演奏次数，终点的起音不计入。
        </p>
        <div className="file-actions">
          <button
            disabled={busy || reading || !limitsCache.current}
            onClick={reviewBeats}
          >
            审阅拍内范围
          </button>
          <button
            disabled={busy}
            onClick={() => {
              setStartTrim(0);
              setEndTrim(0);
              setStartText(String(route?.visits[first]?.startBeat ?? 1));
              setEndText(String(route?.visits[last]?.endBeat ?? 1));
              setBeatDirty(false);
              setDirty(true);
              setError("");
            }}
          >
            恢复该次完整范围
          </button>
        </div>
        {beatDirty && (
          <p role="status">拍位已修改，请审阅拍内范围后再保存或循环。</p>
        )}
        {reading && <p role="status">正在读取实际演奏路径…</p>}
        {error && (
          <p className="dialog-error" role="alert">
            {error}
          </p>
        )}
        {preview && (
          <div className="score-range-preview">
            <h3>{preview.range.description}</h3>
            <p>
              {preview.rows.length} 个演奏段 · {preview.notes} 个音符
              {preview.crossings > 0
                ? ` · 经过 ${preview.crossings} 处反复或跳转`
                : ""}
            </p>
            <div
              className="score-range-path"
              tabIndex={0}
              aria-label="选段实际演奏路径"
            >
              <table>
                <thead>
                  <tr>
                    <th>演奏位置</th>
                    <th>原谱小节</th>
                    <th>实际范围</th>
                  </tr>
                </thead>
                <tbody>
                  {preview.rows.slice(page * 30, page * 30 + 30).map((v) => (
                    <tr key={v.ordinal}>
                      <td>第 {v.ordinal + 1} 段</td>
                      <td>第 {v.number || v.measure + 1} 小节</td>
                      <td>
                        {v.partialStart
                          ? `第 ${Number(v.startBeat.toFixed(3))} 拍开始`
                          : "小节起点"}{" "}
                        ·{" "}
                        {v.partialEnd
                          ? `第 ${Number(v.endBeat.toFixed(3))} 拍前结束`
                          : "小节结束"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {preview.rows.length > 30 && (
              <div className="score-range-pages">
                <button
                  disabled={busy || page === 0}
                  onClick={() => setPage(page - 1)}
                >
                  上一页路径
                </button>
                <span>
                  {page + 1} / {Math.ceil(preview.rows.length / 30)}
                </span>
                <button
                  disabled={busy || (page + 1) * 30 >= preview.rows.length}
                  onClick={() => setPage(page + 1)}
                >
                  下一页路径
                </button>
              </div>
            )}
            {preview.crossings > 0 && (
              <label className="score-range-confirm">
                <input
                  type="checkbox"
                  disabled={busy}
                  checked={confirmed}
                  onChange={(e) => setConfirmed(e.target.checked)}
                />
                按上述实际路径练习，包含返回或跳过的小节
              </label>
            )}
          </div>
        )}
        <div className="score-range-save">
          <label>
            段落名称
            <input
              aria-label="原谱练习段名称"
              maxLength={80}
              disabled={busy}
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                setDirty(true);
              }}
              placeholder="例如：再现部转指、尾声连接"
            />
          </label>
          <label>
            练习备注
            <textarea
              aria-label="原谱练习段备注"
              disabled={busy}
              rows={2}
              value={notes}
              onChange={(e) => {
                setNotes(e.target.value);
                setDirty(true);
              }}
            />
          </label>
        </div>
        <p className="score-range-hint">
          保存时记住当前手别、音轨、速度、模式、预备拍与遍数。练习计划也可使用此段落；谱面或路径改变时需重新审阅。
        </p>
        <div className="file-actions">
          <button disabled={!allowed} onClick={() => void work(false)}>
            循环所选范围
          </button>
          <button
            className="primary-button"
            disabled={!allowed || !name.trim()}
            onClick={() => void work(true)}
          >
            {preset ? "更新原谱练习段" : "保存原谱练习段"}
          </button>
          <button disabled={busy} onClick={exit}>
            取消
          </button>
        </div>
      </section>
    </div>
  );
}
