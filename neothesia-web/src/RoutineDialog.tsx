import { PassageSequenceDialog } from "./PassageSequenceDialog";
import {
  ExpressionGoalControl,
  expressionGoalText,
  type ExpressionGoal,
} from "./ExpressionGoalControl";
import { useEffect, useState } from "react";
import { WeakPracticePanel } from "./WeakPracticePanel";
import {
  RoutineScheduleEditor,
  scheduleText,
  type RoutineSchedule,
} from "./RoutineScheduleEditor";
import {
  api,
  type Snapshot,
  type LoadedSong,
  type SongRow,
  type LadderPreset,
  type CollectionSong,
} from "./api";
export interface RoutineGoal {
  expression?: ExpressionGoal | null;
  passes: number;
  accuracy: number;
  onTime: number | null;
  consecutive: boolean;
  attemptLimit: number;
}
interface Item {
  id: string;
  title: string;
  notes: string;
  songTitle: string;
  contentId: string;
  settings: { mode: string; hands: string; countIn: number };
  speed: number;
  goal: RoutineGoal;
  target: {
    kind: "whole" | "passage" | "scorePassage" | "precisePassage" | "ladder";
    range?: { description: string };
    start?: number;
    end?: number;
    preset?: LadderPreset;
  };
}
interface Plan {
  schedule?: RoutineSchedule | null;
  id: string;
  name: string;
  notes: string;
  items: Item[];
}
interface Progress {
  passed: number;
  attempts: number;
  streak: number;
  completed: boolean;
  skipped: boolean;
  limited: boolean;
  lastResult: string;
  bestAccuracy: number | null;
}
interface Run {
  day: string;
  routine: Plan;
  progress: Record<string, Progress>;
  lastItem: string | null;
}
export interface RoutineState {
  routineId: string;
  day: string;
  itemId: string;
  title: string;
  itemTitle: string;
  notes: string;
  index: number;
  total: number;
  goal: RoutineGoal;
  progress: Progress;
  nextItemId: string | null;
  kind: string;
}
const localDay = () => {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
};
const modes: Record<string, string> = {
  wait: "等音",
  flow: "连续",
  recital: "完整演奏",
  memory: "背谱",
};
const range = (item: Item) =>
  item.target.kind === "whole"
    ? "全曲"
    : item.target.kind === "ladder"
      ? `阶梯 · 第 ${item.target.preset?.start}–${item.target.preset?.end} 小节`
      : item.target.kind === "scorePassage" ||
          item.target.kind === "precisePassage"
        ? (item.target.range?.description ?? "原谱选段")
        : `第 ${item.target.start}–${item.target.end} 小节`;
export function RoutineStatus({
  state,
  manage,
  next,
  end,
}: {
  state: Snapshot;
  manage: () => void;
  next: (r: RoutineState) => void;
  end: () => void;
}) {
  const r = state.routine;
  if (!r) return null;
  return (
    <div className="routine-status" aria-live="polite">
      <div>
        <strong>
          {r.title} · {r.day}
        </strong>
        <span>
          {r.index}/{r.total} · {r.itemTitle} ·{" "}
          {r.progress.completed
            ? "本项已达标"
            : r.progress.skipped
              ? "本项已跳过"
              : r.progress.limited
                ? "本次轮数到上限"
                : `达标 ${r.goal.consecutive ? r.progress.streak : r.progress.passed}/${r.goal.passes} 次 · 已练 ${r.progress.attempts} 轮`}
        </span>
        {r.goal.expression && (
          <span className="routine-expression-target">
            要求：{expressionGoalText(r.goal.expression)}
          </span>
        )}
        {r.progress.lastResult && (
          <span className="routine-last-result">
            上一轮：{r.progress.lastResult}
          </span>
        )}
        {r.notes && <span title={r.notes}>{r.notes}</span>}
      </div>
      <button onClick={manage}>查看安排</button>
      <button
        disabled={!r.progress.completed || !r.nextItemId}
        onClick={() => next(r)}
      >
        打开下一项
      </button>
      <button onClick={end}>结束本项</button>
    </div>
  );
}
export function RoutineDialog({
  initialPlan,
  song,
  songs,
  state,
  close,
  changed,
  backup,
}: {
  initialPlan?: { id: string; day: string | null } | null;
  song: LoadedSong | null;
  songs: SongRow[];
  state: Snapshot;
  close: () => void;
  changed: () => Promise<void>;
  backup: () => void;
}) {
  const [day, setDay] = useState(
      () => initialPlan?.day ?? state.routine?.day ?? localDay(),
    ),
    [template, setTemplate] = useState(
      !!initialPlan && initialPlan.day == null,
    ),
    [plans, setPlans] = useState<Plan[]>([]),
    [runs, setRuns] = useState<Run[]>([]);
  const [selected, setSelected] = useState<string | null>(
      initialPlan?.id ?? state.routine?.routineId ?? null,
    ),
    [name, setName] = useState(state.routine?.title ?? ""),
    [notes, setNotes] = useState("");
  const [itemId, setItemId] = useState<string | null>(
      state.routine?.itemId ?? null,
    ),
    [title, setTitle] = useState(state.routine?.itemTitle ?? ""),
    [requirement, setRequirement] = useState(state.routine?.notes ?? ""),
    [source, setSource] = useState(state.routine ? "" : "current");
  const [goal, setGoal] = useState<RoutineGoal>(
    state.routine?.goal ?? {
      passes: 1,
      accuracy: 90,
      onTime: null,
      consecutive: false,
      attemptLimit: 20,
    },
  );
  const [passages, setPassages] = useState<{ id: string; name: string }[]>([]),
    [ladders, setLadders] = useState<LadderPreset[]>([]),
    [generated, setGenerated] = useState<CollectionSong[]>([]);
  const [compose, setCompose] = useState(false);
  const [skipReason, setSkipReason] = useState("");
  const [search, setSearch] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const run = runs.find((r) => r.routine.id === selected),
    plan = template
      ? plans.find((p) => p.id === selected)
      : (run?.routine ?? plans.find((p) => p.id === selected));
  const editDay = template ? null : day,
    active = Boolean(
      state.routine &&
      state.routine.routineId === selected &&
      state.routine.day === day &&
      !template,
    ),
    locked = busy || active;
  const progress = (id: string) =>
    state.routine?.routineId === selected &&
    state.routine.day === day &&
    state.routine.itemId === id
      ? state.routine.progress
      : run?.progress[id];
  const currentItem = plan?.items.find((i) => i.id === itemId),
    ladderItem =
      source.startsWith("ladder:") ||
      (source === "" && currentItem?.target.kind === "ladder");
  const reload = async () => {
    const data = await api.command<{ plans: Plan[]; runs: Run[] }>({
      type: "routines",
      day,
    });
    setPlans(data.plans);
    setRuns(data.runs);
  };
  useEffect(() => {
    let cancelled = false;
    void api
      .command<{ plans: Plan[]; runs: Run[] }>({ type: "routines", day })
      .then((data) => {
        if (!cancelled) {
          setPlans(data.plans);
          setRuns(data.runs);
          const p =
            data.runs.find((r) => r.routine.id === selected)?.routine ??
            data.plans.find((p) => p.id === selected);
          if (p && !template) {
            setName(p.name);
            setNotes(p.notes);
          }
        }
      })
      .catch((e) => {
        if (!cancelled) setError(String(e.message));
      });
    return () => {
      cancelled = true;
    };
  }, [day]);
  useEffect(() => {
    if (!song) {
      setPassages([]);
      setLadders([]);
      return;
    }
    let cancelled = false;
    void Promise.all([
      api.command<{ passages: { id: string; name: string }[] }>({
        type: "passages",
      }),
      api.command<{ presets: LadderPreset[] }>({ type: "ladderPresets" }),
      api.command<{ songs: CollectionSong[] }>({ type: "collection" }),
    ])
      .then(([p, l, c]) => {
        if (!cancelled) {
          setPassages(p.passages);
          setLadders(l.presets);
          setGenerated(c.songs.filter((s) => s.generated));
        }
      })
      .catch((e) => {
        if (!cancelled) setError(String(e.message));
      });
    return () => {
      cancelled = true;
    };
  }, [song?.contentId]);
  useEffect(() => {
    const escape = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !e.defaultPrevented && !busy && !compose)
        close();
    };
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [close, busy, compose]);
  const work = async (job: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await job();
    } catch (e) {
      await changed();
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setBusy(false);
    }
  };
  const choosePlan = (p: Plan) => {
    setSelected(p.id);
    setName(p.name);
    setNotes(p.notes);
    setItemId(null);
    setTitle("");
    setRequirement("");
    setSource("current");
    setError("");
  };
  const chooseItem = (item: Item) => {
    setItemId(item.id);
    setTitle(item.title);
    setRequirement(item.notes);
    setGoal(item.goal);
    setSource("");
    setSkipReason("");
    setError("");
  };
  const newItem = () => {
    setItemId(null);
    setTitle(song?.title ?? "");
    setRequirement("");
    setGoal({
      passes: 1,
      accuracy: 90,
      onTime: null,
      consecutive: false,
      attemptLimit: 20,
    });
    setSource("current");
  };
  const patch = (value: Partial<RoutineGoal>) =>
    setGoal((g) => ({ ...g, ...value }));
  const itemCommand = (type: string, extra: Record<string, unknown> = {}) => ({
    type,
    routine_id: selected,
    day: editDay,
    item_id: itemId,
    ...extra,
  });
  const openItem = (item: Item) =>
    void work(async () => {
      await api.command({
        type: "openRoutineItem",
        routine_id: selected,
        day,
        item_id: item.id,
        resume: true,
      });
      await changed();
      close();
    });
  const allPlans = template
    ? plans
    : [
        ...plans.map(
          (p) => runs.find((r) => r.routine.id === p.id)?.routine ?? p,
        ),
        ...runs
          .filter((r) => !plans.some((p) => p.id === r.routine.id))
          .map((r) => r.routine),
      ];
  const completed =
      plan?.items.filter((i) => progress(i.id)?.completed).length ?? 0,
    skipped = plan?.items.filter((i) => progress(i.id)?.skipped).length ?? 0;
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!busy) close();
      }}
    >
      <section
        className="settings-dialog routine-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="routine-title"
        onClick={(e) => e.stopPropagation()}
      >
        {compose && plan && (
          <PassageSequenceDialog
            contentId={song?.contentId ?? ""}
            songTitle={song?.title ?? "跨曲目段落"}
            routineId={plan.id}
            day={editDay}
            planName={plan.name}
            close={() => setCompose(false)}
            added={async () => {
              await reload();
              await changed();
            }}
          />
        )}
        <div className="dialog-heading">
          <h2 id="routine-title">练习计划</h2>
          <span className="menu-spacer" />
          <button disabled={busy} onClick={backup}>
            练习资料备份
          </button>
          <button
            autoFocus
            disabled={busy}
            aria-label="关闭练习计划"
            onClick={close}
          >
            关闭
          </button>
        </div>
        <div className="routine-toolbar">
          <button
            aria-pressed={!template}
            onClick={() => {
              setTemplate(false);
              setItemId(null);
              setTitle("");
              setRequirement("");
              setSource("current");
              const p = run?.routine ?? plans.find((p) => p.id === selected);
              if (p) {
                setName(p.name);
                setNotes(p.notes);
              }
            }}
          >
            当天安排
          </button>
          <button
            aria-pressed={template}
            onClick={() => {
              setTemplate(true);
              setItemId(null);
              setTitle("");
              setRequirement("");
              setSource("current");
              const p = plans.find((p) => p.id === selected);
              if (p) {
                setName(p.name);
                setNotes(p.notes);
              }
            }}
          >
            常用计划
          </button>
          <input
            aria-label="练习计划日期"
            type="date"
            value={day}
            disabled={busy || template}
            onChange={(e) => {
              setDay(e.target.value);
              setItemId(null);
              setTitle("");
              setRequirement("");
              setSource("current");
            }}
          />
          <span>
            {template
              ? "编辑模板，不改变已建立的当天安排"
              : "实际结果记录达标；跳过单独标记"}
          </span>
        </div>
        {error && (
          <p role="alert" className="routine-error">
            {error}
          </p>
        )}
        <div className="routine-layout">
          <aside className="routine-plans">
            <button
              disabled={busy}
              onClick={() => {
                setSelected(null);
                setName("");
                setNotes("");
                setItemId(null);
              }}
            >
              新建常用计划
            </button>
            {allPlans.map((p) => (
              <button
                className={`routine-plan ${selected === p.id ? "selected" : ""}`}
                key={p.id}
                disabled={busy}
                onClick={() => choosePlan(p)}
              >
                <strong>{p.name}</strong>
                <span>
                  {p.items.length} 个项目 ·{" "}
                  {scheduleText(plans.find((v) => v.id === p.id)?.schedule)}
                  {!template && runs.some((r) => r.routine.id === p.id)
                    ? " · 已建立当天安排"
                    : ""}
                </span>
              </button>
            ))}
          </aside>
          <div className="routine-main">
            <WeakPracticePanel
              routineId={selected}
              day={editDay}
              disabled={locked}
              work={work}
              added={async (id) => {
                setSelected(id);
                await reload();
                const data = await api.command<{ plans: Plan[]; runs: Run[] }>({
                  type: "routines",
                  day,
                });
                const p =
                  (template
                    ? data.plans.find((p) => p.id === id)
                    : data.runs.find((r) => r.routine.id === id)?.routine) ??
                  data.plans.find((p) => p.id === id);
                if (p) {
                  setName(p.name);
                  setNotes(p.notes);
                }
              }}
            />
            <details className="routine-plan-meta" open={!plan}>
              <summary>
                {plan ? `计划信息：${plan.name}` : "创建常用计划"}
              </summary>
              <label>
                计划名称
                <input
                  aria-label="练习计划名称"
                  value={name}
                  maxLength={80}
                  disabled={locked}
                  placeholder="例如：每日基本功与曲目练习"
                  onChange={(e) => setName(e.target.value)}
                />
              </label>
              <label>
                计划备注
                <textarea
                  aria-label="练习计划备注"
                  value={notes}
                  disabled={locked}
                  rows={2}
                  onChange={(e) => setNotes(e.target.value)}
                />
              </label>
              <div className="routine-meta-actions">
                <button
                  disabled={locked || !name.trim()}
                  onClick={() =>
                    void work(async () => {
                      const data = await api.command<{ id: string }>({
                        type: "saveRoutine",
                        id: selected,
                        day: selected ? editDay : null,
                        name,
                        notes,
                        copy_of: null,
                      });
                      setSelected(data.id);
                      await reload();
                    })
                  }
                >
                  {selected ? "保存计划信息" : "创建计划"}
                </button>
                {selected && (
                  <>
                    <button
                      disabled={busy || !name.trim()}
                      onClick={() =>
                        void work(async () => {
                          const data = await api.command<{ id: string }>({
                            type: "saveRoutine",
                            id: null,
                            day: editDay,
                            name: name + "（副本）",
                            notes,
                            copy_of: selected,
                          });
                          setSelected(data.id);
                          setTemplate(true);
                          setName(name + "（副本）");
                          setItemId(null);
                          await reload();
                        })
                      }
                    >
                      复制为常用计划
                    </button>
                    {template && (
                      <button
                        disabled={locked}
                        onClick={() =>
                          void work(async () => {
                            await api.command({
                              type: "removeRoutine",
                              id: selected,
                            });
                            setSelected(null);
                            setName("");
                            await reload();
                          })
                        }
                      >
                        移除常用计划
                      </button>
                    )}
                  </>
                )}
              </div>
            </details>
            {plan && template && (
              <RoutineScheduleEditor
                value={plan.schedule}
                day={day}
                disabled={busy}
                save={async (schedule) => {
                  await work(async () => {
                    await api.command({
                      type: "saveRoutineSchedule",
                      id: plan.id,
                      schedule,
                    });
                    await reload();
                  });
                }}
              />
            )}
            {plan &&
              !template &&
              plans.find((p) => p.id === plan.id)?.schedule && (
                <p className="parameter-help">
                  {scheduleText(plans.find((p) => p.id === plan.id)?.schedule)}
                  ；本日
                  {run ? "已保存独立安排" : "不符合周期规则，仍可手动打开项目"}
                  。修改周期请进入常用计划。
                </p>
              )}
            {plan && (
              <>
                <div className="routine-items-heading">
                  <strong>
                    {template
                      ? "项目顺序"
                      : `${completed}/${plan.items.length} 项达标${skipped ? ` · ${skipped} 项跳过` : ""}`}
                  </strong>
                  <button disabled={locked || !song} onClick={newItem}>
                    添加当前练习
                  </button>
                  <button disabled={locked} onClick={() => setCompose(true)}>
                    编排多个段落
                  </button>
                  {active && (
                    <button
                      disabled={busy}
                      onClick={() =>
                        void work(async () => {
                          await api.command({ type: "stopRoutine" });
                          await changed();
                          await reload();
                        })
                      }
                    >
                      结束本项，保留进度
                    </button>
                  )}
                </div>
                <div className="routine-items">
                  {plan.items.map((item, index) => {
                    const p = progress(item.id);
                    return (
                      <div
                        className={`routine-item ${itemId === item.id ? "selected" : ""}`}
                        key={item.id}
                      >
                        <button
                          className="routine-item-select"
                          disabled={busy}
                          onClick={() => chooseItem(item)}
                        >
                          <span className="routine-item-index">
                            {index + 1}
                          </span>
                          <span>
                            <strong>{item.title}</strong>
                            <small>
                              {item.songTitle} · {range(item)} ·{" "}
                              {modes[item.settings.mode] ?? item.settings.mode}{" "}
                              · {Math.round(item.speed * 100)}%
                            </small>
                            {item.notes && <small>{item.notes}</small>}
                          </span>
                          <b>
                            {template
                              ? ""
                              : p?.completed
                                ? "已达标"
                                : p?.skipped
                                  ? "已跳过"
                                  : p?.limited
                                    ? "本次到上限"
                                    : `${(item.goal.consecutive ? p?.streak : p?.passed) ?? 0}/${item.goal.passes}`}
                          </b>
                        </button>
                        <button
                          className="routine-quick-open"
                          disabled={
                            busy || template || p?.completed || p?.skipped
                          }
                          aria-label={`练习项目：${item.title}`}
                          onClick={() => openItem(item)}
                        >
                          练习
                        </button>
                      </div>
                    );
                  })}
                  {plan.items.length === 0 && (
                    <p className="parameter-help">
                      添加当前曲目、命名选段或速度阶梯，然后按顺序练习。
                    </p>
                  )}
                </div>
                {(itemId || title) && (
                  <div className="routine-item-editor">
                    <h3>{itemId ? "编辑项目" : "添加项目"}</h3>
                    <label>
                      项目名称
                      <input
                        aria-label="计划项目名称"
                        value={title}
                        disabled={locked}
                        maxLength={100}
                        onChange={(e) => setTitle(e.target.value)}
                      />
                    </label>
                    <label>
                      练习要求
                      <textarea
                        aria-label="计划项目练习要求"
                        value={requirement}
                        rows={2}
                        disabled={locked}
                        onChange={(e) => setRequirement(e.target.value)}
                      />
                    </label>
                    <label>
                      练习条件来源
                      <select
                        aria-label="计划项目条件来源"
                        disabled={locked}
                        value={source}
                        onChange={(e) => {
                          setSource(e.target.value);
                          if (e.target.value.startsWith("ladder:"))
                            patch({ expression: null });
                        }}
                      >
                        {itemId && (
                          <option value="">保留已保存的曲目与条件</option>
                        )}
                        <option value="current">当前曲目及当前练习条件</option>
                        {passages.map((p) => (
                          <option key={p.id} value={`passage:${p.id}`}>
                            命名选段：{p.name}
                          </option>
                        ))}
                        {ladders.map((p) => (
                          <option key={p.id} value={`ladder:${p.id}`}>
                            速度阶梯：{p.name}
                          </option>
                        ))}
                      </select>
                    </label>
                    <div className="routine-goal-grid">
                      <label>
                        达标次数
                        <input
                          aria-label="计划项目达标次数"
                          type="number"
                          min={1}
                          max={100}
                          disabled={locked || ladderItem}
                          value={goal.passes}
                          onChange={(e) =>
                            patch({ passes: Number(e.target.value) })
                          }
                        />
                      </label>
                      <label>
                        正确率至少 %
                        <input
                          aria-label="计划项目正确率"
                          type="number"
                          min={50}
                          max={100}
                          disabled={locked || ladderItem}
                          value={goal.accuracy}
                          onChange={(e) =>
                            patch({ accuracy: Number(e.target.value) })
                          }
                        />
                      </label>
                      <label>
                        本次轮数上限
                        <input
                          aria-label="计划项目轮数上限"
                          type="number"
                          min={1}
                          max={500}
                          disabled={locked || ladderItem}
                          value={goal.attemptLimit}
                          onChange={(e) =>
                            patch({ attemptLimit: Number(e.target.value) })
                          }
                        />
                      </label>
                    </div>
                    <div className="routine-goal-options">
                      <label>
                        <input
                          aria-label="计划项目要求连续达标"
                          type="checkbox"
                          checked={goal.consecutive}
                          disabled={locked || ladderItem}
                          onChange={(e) =>
                            patch({ consecutive: e.target.checked })
                          }
                        />
                        要求连续达标
                      </label>
                      <label>
                        <input
                          aria-label="计划项目要求准时率"
                          type="checkbox"
                          checked={goal.onTime !== null}
                          disabled={locked || ladderItem}
                          onChange={(e) =>
                            patch({ onTime: e.target.checked ? 80 : null })
                          }
                        />
                        准时率门槛
                      </label>
                      {goal.onTime !== null && (
                        <input
                          aria-label="计划项目准时率"
                          type="number"
                          min={50}
                          max={100}
                          disabled={locked || ladderItem}
                          value={goal.onTime}
                          onChange={(e) =>
                            patch({ onTime: Number(e.target.value) })
                          }
                        />
                      )}
                    </div>
                    {!ladderItem && (
                      <ExpressionGoalControl
                        value={goal.expression}
                        wait={
                          source === "current"
                            ? state.mode === "wait"
                            : source === ""
                              ? currentItem?.settings.mode === "wait"
                              : false
                        }
                        disabled={locked}
                        change={(expression) => patch({ expression })}
                      />
                    )}
                    <p className="parameter-help">
                      {ladderItem
                        ? "阶梯项目使用所选方案的完整晋级规则，达到目标级才完成。"
                        : "完整练完目标音符后，按正确率、准时率和已选力度/踏板要求计达标；等音不支持准时率门槛。"}{" "}
                      改变目标或重新捕获条件会重置当天本项进度；历史结果保留。
                    </p>
                    {currentItem && !template && (
                      <label>
                        本次跳过原因（可选）
                        <input
                          aria-label="计划项目跳过原因"
                          disabled={busy}
                          value={skipReason}
                          onChange={(e) => setSkipReason(e.target.value)}
                        />
                      </label>
                    )}
                    <div className="routine-item-actions">
                      <button
                        disabled={
                          locked || !title.trim() || (!song && source !== "")
                        }
                        onClick={() =>
                          void work(async () => {
                            const [kind, ...parts] = source.split(":");
                            const data = await api.command<{ id: string }>({
                              type: "saveRoutineItem",
                              routine_id: selected,
                              day: editDay,
                              id: itemId,
                              source: source ? kind : null,
                              source_id: parts.join(":") || null,
                              title,
                              notes: requirement,
                              goal,
                            });
                            setItemId(data.id);
                            setSource("");
                            await reload();
                          })
                        }
                      >
                        保存项目
                      </button>
                      {currentItem && (
                        <>
                          <button
                            disabled={
                              busy ||
                              template ||
                              progress(currentItem.id)?.completed ||
                              progress(currentItem.id)?.skipped
                            }
                            onClick={() => openItem(currentItem)}
                          >
                            打开本项练习
                          </button>
                          <button
                            disabled={locked}
                            onClick={() =>
                              void work(async () => {
                                await api.command(
                                  itemCommand("moveRoutineItem", {
                                    direction: -1,
                                  }),
                                );
                                await reload();
                              })
                            }
                          >
                            上移
                          </button>
                          <button
                            disabled={locked}
                            onClick={() =>
                              void work(async () => {
                                await api.command(
                                  itemCommand("moveRoutineItem", {
                                    direction: 1,
                                  }),
                                );
                                await reload();
                              })
                            }
                          >
                            下移
                          </button>
                          <button
                            disabled={locked}
                            onClick={() =>
                              void work(async () => {
                                await api.command(
                                  itemCommand("removeRoutineItem"),
                                );
                                setItemId(null);
                                setTitle("");
                                await reload();
                              })
                            }
                          >
                            移除项目
                          </button>
                          {!template && (
                            <>
                              <button
                                disabled={busy}
                                onClick={() =>
                                  void work(async () => {
                                    await api.command(
                                      itemCommand("skipRoutineItem", {
                                        reason: skipReason || "本次跳过",
                                      }),
                                    );
                                    await changed();
                                    await reload();
                                  })
                                }
                              >
                                本次跳过
                              </button>
                              <button
                                disabled={locked}
                                onClick={() =>
                                  void work(async () => {
                                    await api.command(
                                      itemCommand("resetRoutineItem"),
                                    );
                                    await reload();
                                  })
                                }
                              >
                                重置本项进度
                              </button>
                            </>
                          )}
                        </>
                      )}
                    </div>
                    {currentItem && progress(currentItem.id) && (
                      <p className="routine-evidence">
                        {progress(currentItem.id)?.lastResult} · 已练{" "}
                        {progress(currentItem.id)?.attempts} 轮 · 最佳正确率{" "}
                        {progress(currentItem.id)?.bestAccuracy === null
                          ? "—"
                          : Math.round(
                              (progress(currentItem.id)?.bestAccuracy ?? 0) *
                                100,
                            ) + "%"}
                      </p>
                    )}
                  </div>
                )}
                <details className="routine-song-picker">
                  <summary>
                    加入另一首曲目（当前：{song?.title ?? "未打开曲目"}）
                  </summary>
                  <input
                    aria-label="计划添加曲目搜索"
                    disabled={busy || active}
                    placeholder="搜索曲名或作曲家"
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                  />
                  <div>
                    {generated
                      .filter((s) => !search || s.title.includes(search))
                      .slice(0, 8)
                      .map((row) => (
                        <button
                          disabled={busy || active}
                          key={row.contentId}
                          onClick={() =>
                            void work(async () => {
                              await api.command({
                                type: "openRecent",
                                content_id: row.contentId,
                              });
                              await changed();
                              setItemId(null);
                              setTitle(row.title);
                              setRequirement("");
                              setSource("current");
                            })
                          }
                        >
                          {row.title} · 生成练习
                        </button>
                      ))}
                    {songs
                      .filter(
                        (s) =>
                          search &&
                          (s.title + s.composer)
                            .toLowerCase()
                            .includes(search.toLowerCase()),
                      )
                      .slice(0, 20)
                      .map((row) => (
                        <button
                          key={row.id}
                          disabled={busy || active}
                          onClick={() =>
                            void work(async () => {
                              await api.command({
                                type: "load",
                                path: row.path,
                                title: row.title,
                              });
                              await changed();
                              setItemId(null);
                              setTitle(row.title);
                              setRequirement("");
                              setSource("current");
                            })
                          }
                        >
                          {row.title}
                          {row.composer && ` · ${row.composer}`}
                        </button>
                      ))}
                  </div>
                  <p className="parameter-help">
                    打开曲目后确认主界面声部或选段，再加入。生成练习也可从最近练习选择。
                  </p>
                </details>
              </>
            )}
          </div>
        </div>
        <p className="routine-footnote">
          常用计划是模板。当天安排首次使用时复制模板；修改某一天不会改动其他日期或模板。打开项目后按主界面“开始练习”，达标后再选择下一项。
        </p>
      </section>
    </div>
  );
}
