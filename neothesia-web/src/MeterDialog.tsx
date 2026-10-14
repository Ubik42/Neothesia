import { useEffect, useRef, useState } from "react";
import { X, Plus, Trash2 } from "lucide-react";
import {
  api,
  type LoadedSong,
  type MeterMap,
  type MeterMapPreview,
  type MeterMapSegment,
} from "./api";
import "./meter.css";

function fromSong(song: LoadedSong): MeterMap {
  if (song.meterMap)
    return {
      ...structuredClone(song.meterMap),
      measure_lengths: song.meterMap.measure_lengths ?? [],
    };
  const measure_lengths = song.measures
    .filter((m, i) => i > 0 && i < song.measures.length - 1 && m.partial)
    .map((m) => ({ measure: m.number, ticks: m.endTick - m.startTick }));
  if (song.meterCorrection)
    return {
      pickup_ticks: song.meterCorrection.pickup_ticks,
      measure_lengths,
      segments: [
        {
          measure: 1,
          numerator: song.meterCorrection.numerator,
          denominator: song.meterCorrection.denominator,
          display_start: 1,
        },
      ],
    };
  const first = song.measures[0];
  const segments = song.measures
    .filter(
      (m, i, all) =>
        !i ||
        m.numerator !== all[i - 1].numerator ||
        m.denominator !== all[i - 1].denominator,
    )
    .map((m) => ({
      measure: m.number,
      numerator: m.numerator,
      denominator: m.denominator,
      display_start: null,
    }));
  return {
    measure_lengths,
    pickup_ticks:
      first?.partial &&
      first.endTick - first.startTick <
        (song.ppq * 4 * first.numerator) / first.denominator
        ? first.endTick - first.startTick
        : 0,
    segments: segments.length
      ? segments
      : [{ measure: 1, numerator: 4, denominator: 4, display_start: null }],
  };
}
const clock = (s: number) =>
  `${Math.floor(s / 60)}:${(s % 60).toFixed(3).padStart(6, "0")}`;
export function MeterDialog({
  song,
  close,
  changed,
}: {
  song: LoadedSong;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const initial = useRef(
    song.meterMap
      ? fromSong(song)
      : song.meterCorrection
        ? fromSong(song)
        : null,
  );
  const [draft, setDraft] = useState<MeterMap | null>(initial.current);
  const [form, setForm] = useState(() => fromSong(song));
  const [preview, setPreview] = useState<
    (MeterMapPreview & { key: string }) | null
  >(null);
  const [busy, setBusy] = useState(false),
    [previewing, setPreviewing] = useState(false),
    [saved, setSaved] = useState(false),
    [error, setError] = useState("");
  const [page, setPage] = useState(0),
    [jump, setJump] = useState(1),
    [located, setLocated] = useState<number | null>(null);
  const root = useRef<HTMLElement>(null),
    request = useRef(0),
    saving = useRef(false);
  const key = JSON.stringify(draft),
    dirty = key !== JSON.stringify(initial.current);
  const validPreview = preview?.key === key ? preview : null;
  const edit = (value: MeterMap) => {
    request.current++;
    setPreviewing(false);
    setForm(value);
    setLocated(null);
    setDraft(value);
    setPreview(null);
    setError("");
    setPage(0);
  };
  const segment = (index: number, changes: Partial<MeterMapSegment>) =>
    edit({
      ...form,
      segments: form.segments.map((s, i) =>
        i === index ? { ...s, ...changes } : s,
      ),
    });
  const unitTicksAt = (measure: number) => {
    const active =
      [...form.segments].reverse().find((s) => s.measure <= measure) ??
      form.segments[0];
    return (song.ppq * 4) / active.denominator;
  };
  const exit = () => {
    if (
      !saving.current &&
      (saved || !dirty || window.confirm("小节网格尚未保存，放弃本次修改？"))
    )
      close();
  };
  useEffect(() => {
    const unload = (e: BeforeUnloadEvent) => {
      if ((!saved && dirty) || saving.current) {
        e.preventDefault();
        e.returnValue = "";
      }
    };
    const escape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopImmediatePropagation();
        exit();
      }
      if (e.key === "Tab") {
        const nodes = [
          ...(root.current?.querySelectorAll<HTMLElement>(
            "button:not(:disabled),input:not(:disabled),select:not(:disabled)",
          ) ?? []),
        ].filter((n) => n.getClientRects().length);
        if (
          nodes.length &&
          ((e.shiftKey && document.activeElement === nodes[0]) ||
            (!e.shiftKey && document.activeElement === nodes.at(-1)))
        ) {
          e.preventDefault();
          (e.shiftKey ? nodes.at(-1) : nodes[0])?.focus();
        }
      }
    };
    document.addEventListener("keydown", escape, true);
    window.addEventListener("beforeunload", unload);
    return () => {
      document.removeEventListener("keydown", escape, true);
      window.removeEventListener("beforeunload", unload);
    };
  }, [dirty, saved, close]);
  useEffect(
    () => () => {
      request.current++;
    },
    [],
  );
  const inspect = async (value = draft) => {
    const id = ++request.current;
    setPreviewing(true);
    setError("");
    try {
      if (
        value &&
        (!Number.isFinite(value.pickup_ticks) ||
          value.measure_lengths.some(
            (m) =>
              !Number.isInteger(m.measure) ||
              !Number.isInteger(m.ticks) ||
              m.measure < 2 ||
              m.ticks < 1,
          ) ||
          value.segments.some(
            (s) =>
              !Number.isInteger(s.measure) ||
              !Number.isInteger(s.numerator) ||
              (s.display_start !== null && !Number.isInteger(s.display_start)),
          ))
      )
        throw new Error("请填写完整的小节位置、拍号和显示编号。");
      const result = await api.command<MeterMapPreview>({
        type: "previewMeterMap",
        content_id: song.contentId,
        value,
      });
      if (id === request.current) {
        setPreview({ ...result, key: JSON.stringify(value) });
        setPage(0);
        if (value === null)
          setForm(
            fromSong({
              ...song,
              meterMap: null,
              meterCorrection: null,
              measures: result.measures,
            }),
          );
      }
    } catch (e) {
      if (id === request.current) {
        setPreview(null);
        setError(e instanceof Error ? e.message : String(e));
      }
    } finally {
      if (id === request.current) setPreviewing(false);
    }
  };
  const restore = () => {
    request.current++;
    setDraft(null);
    setLocated(null);
    setPreview(null);
    setPreviewing(false);
    setError("");
    void inspect(null);
  };
  const save = async () => {
    if (saving.current || saved || !validPreview) return;
    saving.current = true;
    setBusy(true);
    setError("");
    let didSave = false;
    try {
      await api.command({
        type: "saveMeterMap",
        content_id: song.contentId,
        value: draft,
        proof: validPreview.proof,
      });
      didSave = true;
      setSaved(true);
      await changed();
      close();
    } catch (e) {
      const detail = e instanceof Error ? e.message : String(e);
      setError(didSave
        ? `小节网格已保存，但界面刷新失败：${detail}。你可以重试刷新，或关闭窗口后重新打开曲目。`
        : detail);
      setPreview(null);
    } finally {
      saving.current = false;
      setBusy(false);
    }
  };
  const retryRefresh = async () => {
    setBusy(true);
    setError("");
    try {
      await changed();
      close();
    } catch (e) {
      const detail = e instanceof Error ? e.message : String(e);
      setError(`小节网格已保存，但界面刷新失败：${detail}。你可以重试刷新，或关闭窗口后重新打开曲目。`);
    } finally {
      setBusy(false);
    }
  };
  const locate = async (measure: LoadedSong["measures"][number]) => {
    try {
      await api.command({ type: "seek", position: measure.start });
      setLocated(measure.number);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };
  const bars = validPreview?.measures ?? [],
    pageCount = Math.max(1, Math.ceil(bars.length / 20));
  return (
    <div className="modal-backdrop" onClick={exit}>
      <section
        ref={root}
        className="settings-dialog meter-map-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="meter-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="meter-title">修正小节网格</h2>
            <p>{song.title}</p>
          </div>
          <button
            autoFocus
            aria-label="关闭小节网格"
            disabled={busy}
            onClick={exit}
          >
            <X size={18} />
          </button>
        </div>
        <p className="parameter-help">
          导航小节从 1 开始，按演奏顺序排列。显示编号用于对应纸谱，可以从 0
          开始，也可以在新段落重新编号。练习选段仍使用导航小节。
        </p>
        <fieldset disabled={busy || saved} className="meter-map-editor">
          <legend>拍号变化</legend>
          <div className="meter-table-wrap">
            <table className="meter-map-table">
              <thead>
                <tr>
                  <th>从导航小节</th>
                  <th>每小节拍数</th>
                  <th>单位音符</th>
                  <th>显示起始编号</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                {form.segments.map((s, i) => (
                  <tr key={i}>
                    <td>
                      <input
                        aria-label={`第 ${i + 1} 段起始小节`}
                        type="number"
                        min={1}
                        step={1}
                        value={s.measure || ""}
                        disabled={!i}
                        onChange={(e) =>
                          segment(i, { measure: Number(e.target.value) })
                        }
                      />
                    </td>
                    <td>
                      <input
                        aria-label={
                          i ? `第 ${i + 1} 段每小节拍数` : "网格每小节拍数"
                        }
                        type="number"
                        min={1}
                        max={32}
                        value={s.numerator || ""}
                        onChange={(e) =>
                          segment(i, { numerator: Number(e.target.value) })
                        }
                      />
                    </td>
                    <td>
                      <select
                        aria-label={
                          i ? `第 ${i + 1} 段拍号分母` : "网格拍号分母"
                        }
                        value={s.denominator}
                        onChange={(e) =>
                          segment(i, { denominator: Number(e.target.value) })
                        }
                      >
                        {[1, 2, 4, 8, 16, 32].map((n) => (
                          <option key={n} value={n}>
                            {n} 分音符
                          </option>
                        ))}
                      </select>
                    </td>
                    <td>
                      <input
                        aria-label={`第 ${i + 1} 段显示起始编号`}
                        type="number"
                        min={0}
                        step={1}
                        placeholder={i ? "接续上一段" : "从 1 开始"}
                        value={s.display_start ?? ""}
                        onChange={(e) =>
                          segment(i, {
                            display_start:
                              e.target.value === ""
                                ? null
                                : Number(e.target.value),
                          })
                        }
                      />
                    </td>
                    <td>
                      {i > 0 && (
                        <button
                          aria-label={`移除第 ${i + 1} 段拍号`}
                          onClick={() =>
                            edit({
                              ...form,
                              segments: form.segments.filter(
                                (_, index) => index !== i,
                              ),
                            })
                          }
                        >
                          <Trash2 size={15} />
                          移除
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="meter-map-toolbar">
            <button
              onClick={() => {
                const last = form.segments.at(-1)!;
                edit({
                  ...form,
                  segments: [
                    ...form.segments,
                    { ...last, measure: last.measure + 1, display_start: null },
                  ],
                });
              }}
            >
              <Plus size={16} />
              添加拍号变化
            </button>
            <label>
              首小节弱起拍数
              <input
                aria-label="网格弱起拍数"
                type="number"
                min={0}
                step="any"
                value={Number(
                  (
                    form.pickup_ticks /
                    ((song.ppq * 4) / form.segments[0].denominator)
                  ).toFixed(6),
                )}
                onChange={(e) =>
                  edit({
                    ...form,
                    pickup_ticks: Math.round(
                      (Number(e.target.value) * song.ppq * 4) /
                        form.segments[0].denominator,
                    ),
                  })
                }
              />
            </label>
          </div>
          <p className="parameter-help">
            弱起拍数使用首段的单位音符。0
            表示完整首小节。各段起始小节必须从小到大排列；上方拍号变化会影响后续小节的时间位置。
          </p>
        </fieldset>
        <fieldset disabled={busy} className="meter-map-editor">
          <legend>中途不完整的小节</legend>
          <p className="parameter-help">
            MIDI
            文件可能在一个小节尚未结束时切换拍号。这里填写该小节实际有多少拍，下一小节会从实际结束位置开始；其后恢复当前拍号的正常长度。
          </p>
          <div className="meter-table-wrap">
            <table className="meter-map-table">
              <thead>
                <tr>
                  <th>导航小节</th>
                  <th>实际拍数</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                {form.measure_lengths.map((item, i) => (
                  <tr key={i}>
                    <td>
                      <input
                        aria-label={`第 ${i + 1} 个短小节位置`}
                        type="number"
                        min={2}
                        step={1}
                        value={item.measure || ""}
                        onChange={(e) =>
                          edit({
                            ...form,
                            measure_lengths: form.measure_lengths.map((m, j) =>
                              j === i
                                ? { ...m, measure: Number(e.target.value) }
                                : m,
                            ),
                          })
                        }
                      />
                    </td>
                    <td>
                      <input
                        aria-label={`第 ${i + 1} 个短小节实际拍数`}
                        type="number"
                        min={0.001}
                        step="any"
                        value={Number(
                          (item.ticks / unitTicksAt(item.measure)).toFixed(6),
                        )}
                        onChange={(e) =>
                          edit({
                            ...form,
                            measure_lengths: form.measure_lengths.map((m, j) =>
                              j === i
                                ? {
                                    ...m,
                                    ticks: Math.round(
                                      Number(e.target.value) *
                                        unitTicksAt(m.measure),
                                    ),
                                  }
                                : m,
                            ),
                          })
                        }
                      />
                    </td>
                    <td>
                      <button
                        aria-label={`移除第 ${i + 1} 个短小节`}
                        onClick={() =>
                          edit({
                            ...form,
                            measure_lengths: form.measure_lengths.filter(
                              (_, j) => j !== i,
                            ),
                          })
                        }
                      >
                        <Trash2 size={15} />
                        移除
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="meter-map-toolbar">
            <button
              onClick={() => {
                const next = Math.max(
                  2,
                  ...form.measure_lengths.map((m) => m.measure + 1),
                );
                const active =
                  [...form.segments].reverse().find((s) => s.measure <= next) ??
                  form.segments[0];
                const fullTicks = active.numerator * unitTicksAt(next);
                edit({
                  ...form,
                  measure_lengths: [
                    ...form.measure_lengths,
                    {
                      measure: next,
                      ticks: Math.max(
                        1,
                        Math.round(
                          fullTicks -
                            (active.numerator > 1
                              ? unitTicksAt(next)
                              : unitTicksAt(next) / 2),
                        ),
                      ),
                    },
                  ],
                });
              }}
            >
              <Plus size={16} />
              添加不完整小节
            </button>
          </div>
          <p className="parameter-help">
            实际拍数使用该小节拍号的单位音符。例如 4/4 小节提前一拍结束时，请填
            3。
          </p>
        </fieldset>
        <div className="meter-map-toolbar">
          <button disabled={busy || saved || previewing} onClick={() => void inspect()}>
            {previewing ? "正在计算…" : "预览小节网格"}
          </button>
          <span role="status">
            {validPreview
              ? `${draft ? "修改后的" : "文件原始"}网格，共 ${bars.length} 小节`
              : "请预览后核对小节位置。"}
            {located !== null ? ` 已定位到导航小节 ${located}。` : ""}
          </span>
        </div>
        {validPreview && (
          <section aria-label="小节网格预览">
            <div className="meter-table-wrap">
              <table className="meter-map-table">
                <thead>
                  <tr>
                    <th>导航小节</th>
                    <th>显示编号</th>
                    <th>拍号</th>
                    <th>开始—结束</th>
                    <th>小节长度</th>
                    <th>操作</th>
                  </tr>
                </thead>
                <tbody>
                  {bars.slice(page * 20, page * 20 + 20).map((m) => (
                    <tr key={m.number}>
                      <td>{m.number}</td>
                      <td>{m.displayNumber ?? m.number}</td>
                      <td>
                        {m.numerator}/{m.denominator}
                      </td>
                      <td>
                        {clock(m.start)}—{clock(m.end)}
                      </td>
                      <td>{m.partial ? "不完整小节" : "完整小节"}</td>
                      <td>
                        <button
                          disabled={busy}
                          onClick={() => void locate(m)}
                          aria-label={`定位导航小节 ${m.number}`}
                        >
                          定位
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="meter-map-toolbar">
              <button disabled={!page} onClick={() => setPage((p) => p - 1)}>
                上一页
              </button>
              <span>
                {page + 1} / {pageCount} 页
              </span>
              <button
                disabled={page + 1 >= pageCount}
                onClick={() => setPage((p) => p + 1)}
              >
                下一页
              </button>
              <label>
                导航小节
                <input
                  aria-label="预览跳转小节"
                  type="number"
                  min={1}
                  max={bars.length}
                  value={jump}
                  onChange={(e) => setJump(Number(e.target.value))}
                />
              </label>
              <button
                disabled={
                  !Number.isInteger(jump) || jump < 1 || jump > bars.length
                }
                onClick={() => setPage(Math.floor((jump - 1) / 20))}
              >
                查看小节
              </button>
            </div>
          </section>
        )}
        <p className="parameter-help">
          保存后，软件会清除当前循环和本轮成绩。历史记录会保留。音符和速度保持原样。
        </p>
        {error && (
          <p role="alert" className="dialog-error">
            {error}
          </p>
        )}
        <div className="dialog-actions">
          <button disabled={busy || saved || previewing} onClick={restore}>
            恢复文件原始网格
          </button>
          <button disabled={busy} onClick={exit}>
            {saved ? "关闭" : "取消"}
          </button>
          {saved && (
            <button disabled={busy} onClick={() => void retryRefresh()}>
              重试刷新界面
            </button>
          )}
          <button
            className="primary-button"
            disabled={busy || saved || !validPreview}
            onClick={() => void save()}
          >
            {busy ? "正在保存…" : "保存小节网格"}
          </button>
        </div>
      </section>
    </div>
  );
}
