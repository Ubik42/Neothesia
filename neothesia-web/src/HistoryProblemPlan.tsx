import {
  HistoryProblemCut,
  validCut,
  type ProblemCut,
  type CutBounds,
} from "./HistoryProblemCut";
import { useEffect, useRef, useState } from "react";
import { api, pitchName } from "./api";
import { historyGradeName, type HistoryScoreReview } from "./historyScoreFocus";
import type { RoutineGoal } from "./RoutineDialog";
type DraftEdit = { title: string; notes: string; cut?: ProblemCut | null };
type Row = {
  cutBounds: CutBounds;
  cut?: ProblemCut | null;
  cutDescription?: string;
  key: string;
  title: string;
  notes: string;
  start: number;
  end: number;
  references: number[];
  songTitle?: string;
  problems: {
    reference: number;
    measure: number;
    pitch: number;
    kind: string;
  }[];
};
type Preview = { rows: Row[]; revision: string; selected: number };
const today = () => {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
};
export function HistoryProblemPlan({
  review,
  disabled,
  openPlan,
  dirtyChanged,
  multi = false,
}: {
  multi?: boolean;
  review: HistoryScoreReview;
  disabled: boolean;
  openPlan: (id: string, day: string | null) => void;
  dirtyChanged: (dirty: boolean) => void;
}) {
  const cached = useRef<Record<string, any>>(
    (() => {
      if (!multi) return {};
      try {
        const v = JSON.parse(
          localStorage.getItem("neothesia-multi-plan-draft") || "{}",
        );
        return v && typeof v === "object" ? v : {};
      } catch {
        return {};
      }
    })(),
  );
  const [cutsChecked, setCutsChecked] = useState(true);
  const [open, setOpen] = useState(false),
    [chosen, setChosen] = useState<number[]>(
      (Array.isArray(cached.current.chosen) ? cached.current.chosen : [])
        .filter(
          (v: unknown) =>
            typeof v === "number" && Number.isSafeInteger(v) && v >= 0,
        )
        .slice(0, 60),
    ),
    [before, setBefore] = useState(
      Number.isInteger(cached.current.before) &&
        cached.current.before >= 0 &&
        cached.current.before <= 8
        ? cached.current.before
        : 0,
    ),
    [after, setAfter] = useState(
      Number.isInteger(cached.current.after) &&
        cached.current.after >= 0 &&
        cached.current.after <= 8
        ? cached.current.after
        : 0,
    ),
    [mode, setMode] = useState(
      cached.current.mode === "flow" ? "flow" : "wait",
    ),
    [speed, setSpeed] = useState(
      Number.isFinite(cached.current.speed) &&
        cached.current.speed >= 25 &&
        cached.current.speed <= 200
        ? cached.current.speed
        : 70,
    ),
    [preview, setPreview] = useState<Preview | null>(null),
    [edits, setEdits] = useState<Row[]>([]),
    [plans, setPlans] = useState<{ id: string; name: string }[]>([]),
    [target, setTarget] = useState(
      typeof cached.current.target === "string" ? cached.current.target : "",
    ),
    [dated, setDated] = useState(cached.current.dated === true),
    [day, setDay] = useState(() =>
      typeof cached.current.day === "string" &&
      /^\d{4}-\d{2}-\d{2}$/.test(cached.current.day)
        ? cached.current.day
        : today(),
    ),
    [name, setName] = useState(
      typeof cached.current.name === "string"
        ? cached.current.name
        : "问题段落复习",
    ),
    [notes, setNotes] = useState(
      typeof cached.current.notes === "string" ? cached.current.notes : "",
    ),
    [goal, setGoal] = useState<RoutineGoal>(
      cached.current.goal &&
        Number.isInteger(cached.current.goal.passes) &&
        Number.isInteger(cached.current.goal.accuracy) &&
        cached.current.goal.passes >= 1 &&
        cached.current.goal.passes <= 100 &&
        cached.current.goal.accuracy >= 50 &&
        cached.current.goal.accuracy <= 100 &&
        (cached.current.goal.onTime === null ||
          (Number.isInteger(cached.current.goal.onTime) &&
            cached.current.goal.onTime >= 50 &&
            cached.current.goal.onTime <= 100)) &&
        typeof cached.current.goal.consecutive === "boolean" &&
        Number.isInteger(cached.current.goal.attemptLimit) &&
        cached.current.goal.attemptLimit >= 1 &&
        cached.current.goal.attemptLimit <= 500
        ? cached.current.goal
        : {
            passes: 2,
            accuracy: 90,
            onTime: null,
            consecutive: false,
            attemptLimit: 20,
          },
    ),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [result, setResult] = useState<{
      id: string;
      day: string | null;
      added: number;
      skipped: number;
    } | null>(null),
    [search, setSearch] = useState(""),
    [limit, setLimit] = useState(30),
    [dirty, setDirty] = useState(cached.current.dirty === true);
  const alive = useRef(true),
    working = useRef(false),
    epoch = useRef(0),
    dialog = useRef<HTMLElement>(null),
    draftEdits = useRef<Record<string, DraftEdit>>(
      cached.current.edits && typeof cached.current.edits === "object"
        ? Object.fromEntries(
            Object.entries(cached.current.edits)
              .filter(
                ([, v]) =>
                  v &&
                  typeof v === "object" &&
                  typeof (v as DraftEdit).title === "string" &&
                  typeof (v as DraftEdit).notes === "string",
              )
              .map(([k, v]) => {
                const d = v as DraftEdit;
                return [
                  k,
                  {
                    title: d.title,
                    notes: d.notes,
                    cut: validCut(d.cut) ? d.cut : null,
                  },
                ];
              }),
          )
        : {},
    );
  useEffect(() => {
    if (!open) return;
    const previous = document.activeElement as HTMLElement | null;
    const el = dialog.current;
    if (!el) return;
    el.querySelector<HTMLElement>(
      "button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled)",
    )?.focus();
    const trap = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        if (!working.current) setOpen(false);
      }
      if (e.key === "Tab") {
        e.stopPropagation();
        const nodes = [
          ...el.querySelectorAll<HTMLElement>(
            "button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled)",
          ),
        ].filter((n) => n.offsetParent != null);
        const first = nodes[0],
          last = nodes.at(-1);
        if (!first) return;
        if (!el.contains(document.activeElement)) {
          e.preventDefault();
          (e.shiftKey ? last : first)?.focus();
        } else if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last?.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener("keydown", trap, true);
    return () => {
      document.removeEventListener("keydown", trap, true);
      if (previous?.isConnected) previous.focus();
    };
  }, [open]);
  const problems = review.notes.filter((n) =>
    ["early", "late", "missed"].includes(n.kind),
  );
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      epoch.current++;
    };
  }, []);
  useEffect(() => {
    dirtyChanged(dirty);
    const warn = (e: BeforeUnloadEvent) => {
      if (dirty) {
        e.preventDefault();
        e.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirty, dirtyChanged]);
  useEffect(() => {
    if (!multi) return;
    try {
      localStorage.setItem(
        "neothesia-multi-plan-draft",
        JSON.stringify({
          chosen,
          before,
          after,
          mode,
          speed,
          target,
          dated,
          day,
          name,
          notes,
          goal,
          dirty,
          edits: draftEdits.current,
        }),
      );
    } catch {}
  }, [
    multi,
    chosen,
    before,
    after,
    mode,
    speed,
    target,
    dated,
    day,
    name,
    notes,
    goal,
    dirty,
    edits,
  ]);
  const invalidate = () => {
    setPreview(null);
    setResult(null);
    setDirty(true);
  };
  const singleSpec = () => ({
    contentId: review.contentId,
    id: review.historyId,
    scoreRevision: review.scoreRevision,
    references: chosen,
    before,
    after,
    mode,
    speed: speed / 100,
    routineId: target || null,
    day: dated ? day : null,
  });
  const spec = () => {
    if (!multi) return singleSpec();
    const sources: { contentId: string; id: string; references: number[] }[] =
      [];
    const order = chosen.map((ref) => {
      const n = review.notes.find((n) => n.reference === ref);
      if (
        !n?.sourceContentId ||
        !n.sourceHistoryId ||
        n.sourceReference == null
      )
        throw new Error("所选记录已改变，请重新选择问题音");
      let source = sources.find(
        (s) => s.contentId === n.sourceContentId && s.id === n.sourceHistoryId,
      );
      if (!source) {
        source = {
          contentId: n.sourceContentId,
          id: n.sourceHistoryId,
          references: [],
        };
        sources.push(source);
      }
      source.references.push(n.sourceReference);
      return {
        contentId: n.sourceContentId,
        id: n.sourceHistoryId,
        references: [n.sourceReference],
      };
    });
    return {
      sources,
      order,
      before,
      after,
      mode,
      speed: speed / 100,
      routineId: target || null,
      day: dated ? day : null,
    };
  };
  async function work(job: () => Promise<void>) {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    try {
      await job();
    } catch (e) {
      if (alive.current) setError(e instanceof Error ? e.message : String(e));
    } finally {
      working.current = false;
      if (alive.current) setBusy(false);
    }
  }
  async function show() {
    setOpen(true);
    await work(async () => {
      const request = ++epoch.current;
      const data = await api.command<{ plans: { id: string; name: string }[] }>(
        { type: "routines", day },
      );
      if (alive.current && request === epoch.current) setPlans(data.plans);
    });
  }
  const toggle = (reference: number) => {
    invalidate();
    setChosen((old) =>
      old.includes(reference)
        ? old.filter((r) => r !== reference)
        : old.length < 60
          ? [...old, reference]
          : old,
    );
  };
  const move = (index: number, direction: number) => {
    invalidate();
    setChosen((old) => {
      const next = [...old];
      [next[index], next[index + direction]] = [
        next[index + direction],
        next[index],
      ];
      return next;
    });
  };
  const generate = () =>
    work(async () => {
      const request = ++epoch.current;
      const data = await api.command<Preview>({
        type: multi ? "previewMultiProblemPlan" : "previewHistoryProblemPlan",
        spec: spec(),
      });
      if (!alive.current || request !== epoch.current) return;
      setPreview(data);
      setEdits(
        data.rows.map((row) => ({ ...row, ...draftEdits.current[row.key] })),
      );
      setCutsChecked(
        !data.rows.some((row) => draftEdits.current[row.key]?.cut),
      );
      setResult(null);
    });
  const save = () =>
    work(async () => {
      if (!preview || !cutsChecked) return;
      const request = ++epoch.current;
      const data = await api.command<{
        id: string;
        day: string | null;
        added: number;
        skipped: number;
      }>({
        type: multi ? "saveMultiProblemPlan" : "saveHistoryProblemPlan",
        spec: spec(),
        expected: preview.revision,
        name,
        notes,
        goal,
        edits: edits.map(({ key, title, notes, cut }) => ({
          key,
          title,
          notes,
          cut: cut ?? null,
        })),
      });
      if (!alive.current || request !== epoch.current) return;
      setResult(data);
      setPreview(null);
      setDirty(false);
    });
  const changeCut = (row: Row, cut: ProblemCut | null) => {
    draftEdits.current[row.key] = { title: row.title, notes: row.notes, cut };
    setEdits((old) =>
      old.map((r) =>
        r.key === row.key ? { ...r, cut, cutDescription: undefined } : r,
      ),
    );
    setCutsChecked(false);
    setDirty(true);
    setResult(null);
  };
  const checkCuts = () =>
    work(async () => {
      if (!preview) return;
      const request = ++epoch.current;
      const source = spec();
      const data = await api.command<{ rows: Row[]; revision: string }>({
        type: "previewProblemCuts",
        single: multi ? null : source,
        multi: multi ? source : null,
        expected: preview.revision,
        edits: edits.map(({ key, title, notes, cut }) => ({
          key,
          title,
          notes,
          cut: cut ?? null,
        })),
      });
      if (!alive.current || request !== epoch.current) return;
      const rows = data.rows.map((row) => {
        const old = edits.find((e) => e.key === row.key)!;
        const updated = { ...row, title: old.title, notes: old.notes };
        draftEdits.current[row.key] = {
          title: old.title,
          notes: old.notes,
          cut: updated.cut,
        };
        return updated;
      });
      setEdits(rows);
      setCutsChecked(true);
      setPreview({ ...preview, revision: data.revision });
    });
  const filtered = problems.filter((n) =>
    `${n.sourceTitle ?? ""} ${n.measure} ${pitchName(n.pitch)} ${historyGradeName(n.kind)}`.includes(
      search.trim(),
    ),
  );
  const changeGoal = (patch: Partial<RoutineGoal>) => {
    setGoal({ ...goal, ...patch });
    setDirty(true);
    setResult(null);
  };
  return (
    <>
      <button
        disabled={disabled || !review.exact || !problems.length}
        onClick={() => void show()}
      >
        {multi ? "跨记录复习编排" : "编排问题段落"}
      </button>
      {open && (
        <div className="modal-backdrop">
          <section
            ref={dialog}
            className="settings-dialog history-problem-dialog"
            role="dialog"
            aria-modal="true"
            aria-label="历史问题段落编排"
          >
            <div className="dialog-heading">
              <div>
                <h2>{multi ? "跨记录复习编排" : "问题段落编排"}</h2>
                <p>
                  {review.title} ·{" "}
                  {new Date(review.recordedAt).toLocaleString("zh-CN")}
                </p>
              </div>
              <button disabled={busy} onClick={() => setOpen(false)}>
                关闭编排
              </button>
            </div>
            <p>
              挑选问题音并安排顺序，同一范围合成一项，重叠范围分别练习。关闭此窗口保留输入，离开批阅前请保存。
            </p>
            <div className="history-problem-layout">
              <div className="history-problem-source">
                <div className="history-range-controls">
                  <label>
                    查找问题{" "}
                    <input
                      aria-label="查找编排问题音"
                      value={search}
                      onChange={(e) => {
                        setSearch(e.target.value);
                        setLimit(30);
                      }}
                    />
                  </label>
                  <button
                    disabled={busy || disabled}
                    onClick={() => {
                      invalidate();
                      setChosen(filtered.slice(0, 60).map((n) => n.reference));
                    }}
                  >
                    选取前 60 个匹配问题
                  </button>
                  <button
                    disabled={busy || !chosen.length}
                    onClick={() => {
                      invalidate();
                      setChosen([]);
                    }}
                  >
                    清空选择
                  </button>
                </div>
                <table>
                  <thead>
                    <tr>
                      <th>选择</th>
                      <th>演奏小节 / 音</th>
                      <th>判定</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filtered.slice(0, limit).map((n) => (
                      <tr key={n.reference}>
                        <td>
                          <input
                            type="checkbox"
                            aria-label={`编排第${n.measure}小节${pitchName(n.pitch)}参考音${n.reference}`}
                            checked={chosen.includes(n.reference)}
                            disabled={
                              busy ||
                              disabled ||
                              (!chosen.includes(n.reference) &&
                                chosen.length >= 60)
                            }
                            onChange={() => toggle(n.reference)}
                          />
                        </td>
                        <td>
                          {n.sourceTitle && <span>{n.sourceTitle} · </span>}第{" "}
                          {n.measure} 小节 · {pitchName(n.pitch)}
                        </td>
                        <td>{historyGradeName(n.kind)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {!filtered.length && <p>没有匹配的问题音。</p>}
                {filtered.length > limit && (
                  <button onClick={() => setLimit(limit + 30)}>
                    再显示 30 个问题音
                  </button>
                )}
              </div>
              <div className="history-problem-editor">
                <h3>练习顺序 · 已选 {chosen.length}/60</h3>
                <ol className="history-problem-order">
                  {chosen.map((ref, index) => {
                    const n = review.notes.find((n) => n.reference === ref);
                    if (!n)
                      return (
                        <li key={ref}>
                          这个问题音尚未读取或记录已移出。
                          <button disabled={busy} onClick={() => toggle(ref)}>
                            移除未读取问题音
                          </button>
                        </li>
                      );
                    return (
                      <li key={ref}>
                        <span>
                          {n.sourceTitle && <span>{n.sourceTitle} · </span>}第{" "}
                          {n.measure} 小节 · {pitchName(n.pitch)} ·{" "}
                          {historyGradeName(n.kind)}
                        </span>
                        <button
                          aria-label={`问题音${ref}上移`}
                          disabled={busy || index === 0}
                          onClick={() => move(index, -1)}
                        >
                          上移
                        </button>
                        <button
                          aria-label={`问题音${ref}下移`}
                          disabled={busy || index === chosen.length - 1}
                          onClick={() => move(index, 1)}
                        >
                          下移
                        </button>
                        <button
                          aria-label={`移除问题音${ref}`}
                          disabled={busy}
                          onClick={() => toggle(ref)}
                        >
                          移除
                        </button>
                      </li>
                    );
                  })}
                </ol>
                <div className="history-problem-fields">
                  <label>
                    前面小节
                    <input
                      aria-label="编排前置小节"
                      type="number"
                      min={0}
                      max={8}
                      value={before}
                      disabled={busy}
                      onChange={(e) => {
                        invalidate();
                        setBefore(Number(e.target.value));
                      }}
                    />
                  </label>
                  <label>
                    后面小节
                    <input
                      aria-label="编排后置小节"
                      type="number"
                      min={0}
                      max={8}
                      value={after}
                      disabled={busy}
                      onChange={(e) => {
                        invalidate();
                        setAfter(Number(e.target.value));
                      }}
                    />
                  </label>
                  <label>
                    练习模式
                    <select
                      aria-label="编排练习模式"
                      value={mode}
                      disabled={busy}
                      onChange={(e) => {
                        invalidate();
                        setMode(e.target.value);
                        if (e.target.value === "wait")
                          setGoal({ ...goal, onTime: null });
                      }}
                    >
                      <option value="wait">等音</option>
                      <option value="flow">连续</option>
                    </select>
                  </label>
                  <label>
                    速度 %
                    <input
                      aria-label="编排速度"
                      type="number"
                      min={25}
                      max={200}
                      value={speed}
                      disabled={busy}
                      onChange={(e) => {
                        invalidate();
                        setSpeed(Number(e.target.value));
                      }}
                    />
                  </label>
                </div>
                <div className="history-problem-fields">
                  <label>
                    目标计划
                    <select
                      aria-label="编排目标计划"
                      disabled={busy}
                      value={target}
                      onChange={(e) => {
                        invalidate();
                        setTarget(e.target.value);
                      }}
                    >
                      <option value="">新建常用计划</option>
                      {plans.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    <input
                      type="checkbox"
                      aria-label="编排到指定日期"
                      checked={dated}
                      disabled={busy}
                      onChange={(e) => {
                        invalidate();
                        setDated(e.target.checked);
                      }}
                    />{" "}
                    指定日期
                  </label>
                  {dated && (
                    <label>
                      日期
                      <input
                        aria-label="编排日期"
                        type="date"
                        value={day}
                        disabled={busy}
                        onChange={(e) => {
                          invalidate();
                          setDay(e.target.value);
                        }}
                      />
                    </label>
                  )}
                </div>
                {!target && (
                  <>
                    <label>
                      计划名称
                      <input
                        aria-label="编排计划名称"
                        maxLength={80}
                        value={name}
                        disabled={busy}
                        onChange={(e) => {
                          setName(e.target.value);
                          setDirty(true);
                          setResult(null);
                        }}
                      />
                    </label>
                    <label>
                      计划备注
                      <textarea
                        aria-label="编排计划备注"
                        maxLength={2000}
                        value={notes}
                        disabled={busy}
                        onChange={(e) => {
                          setNotes(e.target.value);
                          setDirty(true);
                          setResult(null);
                        }}
                      />
                    </label>
                  </>
                )}
                <div className="history-problem-fields">
                  <label>
                    达标次数
                    <input
                      aria-label="编排达标次数"
                      type="number"
                      min={1}
                      max={100}
                      value={goal.passes}
                      disabled={busy}
                      onChange={(e) =>
                        changeGoal({ passes: Number(e.target.value) })
                      }
                    />
                  </label>
                  <label>
                    正确率 %
                    <input
                      aria-label="编排正确率"
                      type="number"
                      min={50}
                      max={100}
                      value={goal.accuracy}
                      disabled={busy}
                      onChange={(e) =>
                        changeGoal({ accuracy: Number(e.target.value) })
                      }
                    />
                  </label>
                  <label>
                    最多练习轮数
                    <input
                      aria-label="编排轮数上限"
                      type="number"
                      min={1}
                      max={500}
                      value={goal.attemptLimit}
                      disabled={busy}
                      onChange={(e) =>
                        changeGoal({ attemptLimit: Number(e.target.value) })
                      }
                    />
                  </label>
                </div>
                <div className="history-range-controls">
                  <label>
                    <input
                      type="checkbox"
                      aria-label="编排连续达标"
                      checked={goal.consecutive}
                      disabled={busy}
                      onChange={(e) =>
                        changeGoal({ consecutive: e.target.checked })
                      }
                    />{" "}
                    连续达标
                  </label>
                  {mode === "flow" && (
                    <>
                      <label>
                        <input
                          type="checkbox"
                          aria-label="编排准时率要求"
                          checked={goal.onTime != null}
                          disabled={busy}
                          onChange={(e) =>
                            changeGoal({ onTime: e.target.checked ? 85 : null })
                          }
                        />{" "}
                        准时率要求
                      </label>
                      {goal.onTime != null && (
                        <input
                          aria-label="编排准时率"
                          type="number"
                          min={50}
                          max={100}
                          value={goal.onTime}
                          disabled={busy}
                          onChange={(e) =>
                            changeGoal({ onTime: Number(e.target.value) })
                          }
                        />
                      )}
                    </>
                  )}
                </div>
                <button
                  disabled={busy || disabled || !chosen.length}
                  onClick={() => void generate()}
                >
                  预览练习编排
                </button>
                {preview && (
                  <div className="history-problem-preview">
                    <h3>
                      {preview.selected} 个问题音 → {edits.length} 个练习项目
                    </h3>
                    {edits.map((row, index) => (
                      <div key={row.key}>
                        <strong>
                          {row.songTitle ? `${row.songTitle} · ` : ""}
                          {index + 1}. 第 {row.start}–{row.end} 小节 ·{" "}
                          {row.references.length} 个所选问题
                        </strong>
                        <HistoryProblemCut
                          index={index + 1}
                          bounds={row.cutBounds}
                          cut={row.cut}
                          description={row.cutDescription}
                          disabled={busy}
                          change={(cut) => changeCut(row, cut)}
                        />
                        <label>
                          项目名称
                          <input
                            aria-label={`编排项目${index + 1}名称`}
                            maxLength={100}
                            value={row.title}
                            disabled={busy}
                            onChange={(e) => {
                              draftEdits.current[row.key] = {
                                cut: row.cut,
                                title: e.target.value,
                                notes: row.notes,
                              };
                              setEdits((old) =>
                                old.map((r) =>
                                  r.key === row.key
                                    ? { ...r, title: e.target.value }
                                    : r,
                                ),
                              );
                              setDirty(true);
                            }}
                          />
                        </label>
                        <label>
                          练习要求
                          <textarea
                            aria-label={`编排项目${index + 1}要求`}
                            maxLength={1700}
                            value={row.notes}
                            disabled={busy}
                            onChange={(e) => {
                              draftEdits.current[row.key] = {
                                cut: row.cut,
                                title: row.title,
                                notes: e.target.value,
                              };
                              setEdits((old) =>
                                old.map((r) =>
                                  r.key === row.key
                                    ? { ...r, notes: e.target.value }
                                    : r,
                                ),
                              );
                              setDirty(true);
                            }}
                          />
                        </label>
                      </div>
                    ))}
                    {!cutsChecked && (
                      <button
                        disabled={busy || disabled}
                        onClick={() => void checkCuts()}
                      >
                        核对拍内范围
                      </button>
                    )}
                    <button
                      disabled={
                        busy ||
                        disabled ||
                        !cutsChecked ||
                        edits.some((e) => !e.title.trim()) ||
                        (!target && !name.trim())
                      }
                      onClick={() => void save()}
                    >
                      保存练习编排
                    </button>
                  </div>
                )}
                {result && (
                  <p role="status">
                    已保存：新增 {result.added} 项，跳过完全相同项目{" "}
                    {result.skipped} 项。
                    <button
                      disabled={busy}
                      onClick={() => {
                        setOpen(false);
                        openPlan(result.id, result.day);
                      }}
                    >
                      打开已保存计划
                    </button>
                  </p>
                )}
                {error && <p role="alert">{error}</p>}
              </div>
            </div>
          </section>
        </div>
      )}
    </>
  );
}
