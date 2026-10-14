import {
  HistoryMultiProblemPlan,
  type ReviewSource,
} from "./HistoryMultiProblemPlan";
import type {
  HistoryScoreTarget,
  HistoryScoreReview,
} from "./historyScoreFocus";
import { HistoryPerformance } from "./HistoryPerformance";
export type HistoryPaperTarget = {
  contentId: string;
  measure: number;
  historyId: string;
  reference: number;
  pitch: number;
  track: number;
  songTime: number;
  index?: number | null;
  startMeasure: number;
  endMeasure: number;
};
import { practiceDuration } from "./practiceTime";
import { PracticeCalendar } from "./PracticeCalendar";
import { expressionGoalText, noteVelocityEvidenceText, notePedalEvidenceText, type NoteVelocityEvidence, type NotePedalEvidence } from "./ExpressionGoalControl";
import { useEffect, useState } from "react";
import {
  HistoryAnnotationEditor,
  type Annotation,
} from "./HistoryAnnotationEditor";
import { api, type Feedback, type Snapshot } from "./api";
interface Entry {
  annotation: Annotation | null;
  id: string;
  contentId: string;
  title: string;
  path: string | null;
  recordedAt: number;
  playingMs?: number | null;
  speed: number;
  hands: string;
  mode: string | null;
  coverage: { target: number; judged: number } | null;
  range: [number, number] | null;
  exactPassage?: boolean;
  score: Snapshot["score"];
  restorable: boolean;
  tempo: number | null;
}
interface Detail extends Entry {
  performanceAvailable?: boolean;
  summary: Feedback;
  expressionGoalResult?: { passed: boolean; message: string; noteVelocity?: NoteVelocityEvidence; notePedal?: NotePedalEvidence };
  context: {
    expression?: {
      velocity_difference: number | null;
      note_velocity_difference?: number | null;
      note_pedal_mismatch_count?: number | null;
      contour_percent: number | null;
      pedal_offset_ms: number | null;
    } | null;
    routine?: {
      expression?: {
        velocity_difference: number | null;
        note_velocity_difference?: number | null;
        note_pedal_mismatch_count?: number | null;
        contour_percent: number | null;
        pedal_offset_ms: number | null;
      } | null;
      item_notes?: string;
      consecutive?: boolean;
      accuracy_percent: number;
      on_time_percent: number | null;
      routine_name: string;
      day: string;
      item_title: string;
      passes_required: number;
    } | null;
    ladder?: {
      plan_name: string;
      stage: number;
      speed_percent: number;
      target_percent: number;
    } | null;
    tracks: { track_id: number; mode: string; visible: boolean }[];
    count_in: number;
    metronome: boolean;
    latency: number;
    pedal_latency_ms?: number | null;
    rounds: number;
    adaptive: boolean;
  } | null;
  comparison: {
    count: number;
    bestAccuracy: number | null;
    previous: Entry | null;
    recent: Entry[];
    reasonCode?: string | null;
    reason?: string | null;
    excludedRecent?: { entry: Entry; reasons: string[] }[];
    conditions?: {
      complete: boolean;
      range: [number, number] | null;
      passageBounds?: { start_tick: number; end_tick: number } | null;
      mode: string | null;
      hands: string;
      speed: number;
      effectiveTempoBpm: number | null;
      scope: string | null;
      targetNotes?: number;
      judgedNotes?: number;
      countIn?: number;
      metronome?: boolean;
      inputLatencyMs?: number;
      pedalLatencyMs?: number;
      pedalLatencySource?: string;
      rounds?: number;
      adaptive?: boolean;
      grid?: string;
      tracks?: { track_id: number; mode: string; visible: boolean }[];
      parts?: Record<string, string>;
      assignedHands?: string;
      exerciseSpec?: {
        tonic: number; tonality: string; minor_form?: string; pattern: string;
        direction: string; hands: string; octaves: number; repetitions: number;
        tempo_bpm: number;
      } | null;
      expressionGoal?: {
        velocity_difference: number | null;
        note_velocity_difference?: number | null;
        note_pedal_mismatch_count?: number | null;
        contour_percent: number | null;
        pedal_offset_ms: number | null;
      } | null;
    };
  };
}
const modeName = (m: string | null) =>
  ({
    wait: "等音",
    flow: "连续",
    listen: "聆听",
    recital: "完整演奏",
    memory: "背谱",
  })[m ?? ""] ?? "未记录模式";
const handName = (h: string) =>
  ({
    Left: "左手",
    Right: "右手",
    Both: "双手",
    Custom: "自定义声部",
    Unspecified: "未记录手别",
  })[h] ?? h;
const accuracy = (s: {
  matched_notes: number;
  wrong_notes: number;
  missed_notes: number;
}) => {
  const n = s.matched_notes + s.wrong_notes + s.missed_notes;
  return n ? s.matched_notes / n : null;
};
const percent = (n: number | null) =>
  n === null ? "—" : `${Math.round(n * 100)}%`;
const coverageName = (e: Entry) =>
  e.coverage && e.coverage.target > 0
    ? `${e.coverage.judged >= e.coverage.target ? "已完成" : "部分结果"} · ${e.coverage.judged}/${e.coverage.target} 个目标音符`
    : "完成范围未记录";
const rangeName = (e: Entry) =>
  e.range ? `第 ${e.range[0]}–${e.range[1]} 小节${e.exactPassage ? " · 原位置" : ""}` : "全曲";
const date = (n: number) =>
  new Date(n).toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
const delta = (a: number | null, b: number | null) =>
  a === null || b === null
    ? "—"
    : `${a >= b ? "+" : ""}${Math.round((a - b) * 100)} 个百分点`;
const trackModeName = (mode: string) =>
  ({ Human: "自己弹", Auto: "伴奏", Mute: "静音" })[mode] ?? mode;
const partName = (part: string) =>
  ({ LeftHand: "左手", RightHand: "右手", Other: "其他" })[part] ?? part;
function exerciseName(spec: NonNullable<NonNullable<Detail["comparison"]["conditions"]>["exerciseSpec"]>) {
  const key = ["C", "升 C", "D", "降 E", "E", "F", "升 F", "G", "降 A", "A", "降 B", "B"][spec.tonic] ?? `音级 ${spec.tonic}`;
  const kind = ({ Scale: "音阶", Arpeggio: "琶音", PrimaryChords: "主和弦" })[spec.pattern] ?? spec.pattern;
  const direction = ({ Ascending: "上行", Descending: "下行", UpAndDown: "上下行" })[spec.direction] ?? spec.direction;
  const hands = ({ Right: "右手", Left: "左手", Both: "双手" })[spec.hands] ?? spec.hands;
  const minor = spec.tonality === "Minor" ? `、${({ Natural: "自然", Harmonic: "和声", Melodic: "旋律" })[spec.minor_form ?? "Natural"] ?? spec.minor_form}小调` : "大调";
  return `${key}${minor} ${kind}；${direction}；${hands}；${spec.octaves} 个八度；${spec.repetitions} 次；${spec.tempo_bpm} BPM`;
}
function ComparisonConditions({ detail }: { detail: Detail }) {
  const c = detail.comparison.conditions;
  if (!c) return null;
  const parts = Object.entries(c.parts ?? {}).sort((a, b) => Number(a[0]) - Number(b[0]));
  return (
    <details className="history-comparison-conditions">
      <summary>查看本次比较条件</summary>
      <p className="parameter-help">软件只比较这首曲目中更早保存、且以下条件相同的成绩。曲名和文件路径可以改变；曲目内容必须相同。</p>
      <dl>
        <div><dt>曲目内容</dt><dd title={detail.contentId}>已按文件内容核对（{detail.contentId.slice(0, 12)}…）</dd></div>
        <div><dt>小节与练习音符</dt><dd>{c.grid ? `小节版本 ${c.grid}` : "小节版本未记录"}；{c.scope ? `练习音符版本 ${c.scope}` : "练习音符版本未记录"}</dd></div>
        <div><dt>范围与弹法</dt><dd>{c.range ? `第 ${c.range[0]}–${c.range[1]} 小节${c.passageBounds ? "（原起止拍位已保存）" : "（旧记录只保留小节）"}` : "全曲"}；{modeName(c.mode)}；{handName(c.hands)}；逐音分手 {c.assignedHands || "未记录"}</dd></div>
        <div><dt>速度</dt><dd>{Math.round(c.speed * 100)}%；{c.effectiveTempoBpm == null ? "有效速度未记录" : `${c.effectiveTempoBpm} BPM`}</dd></div>
        <div><dt>判断范围</dt><dd>{c.targetNotes == null || c.judgedNotes == null ? "目标音符与判断数未记录" : `${c.judgedNotes}/${c.targetNotes} 个目标音符已判断`}</dd></div>
        <div><dt>音轨安排</dt><dd>{c.tracks?.length ? c.tracks.map((t) => `音轨 ${t.track_id + 1} ${trackModeName(t.mode)}${t.visible ? "、显示" : "、隐藏"}`).join("；") : "未记录"}</dd></div>
        <div><dt>音轨分手</dt><dd>{parts.length ? parts.map(([track, part]) => `音轨 ${Number(track) + 1} ${partName(part)}`).join("；") : "没有单独指定的音轨分手"}</dd></div>
        <div><dt>练习来源</dt><dd>{c.exerciseSpec == null ? "导入曲目" : `生成的技术练习：${exerciseName(c.exerciseSpec)}`}</dd></div>
        <div><dt>预备与节拍</dt><dd>预备 {c.countIn ?? "未记录"} 小节；节拍器 {c.metronome == null ? "未记录" : c.metronome ? "开" : "关"}</dd></div>
        <div><dt>设备校准</dt><dd>输入 {c.inputLatencyMs ?? "未记录"} ms；踏板 {c.pedalLatencyMs ?? "未记录"} ms{c.pedalLatencySource === "legacyInputLatency" ? "（旧记录沿用输入校准值）" : ""}</dd></div>
        <div><dt>重复与自适应</dt><dd>{c.rounds == null ? "轮数未记录" : c.rounds === 0 ? "不限轮数" : `${c.rounds} 轮`}；自适应 {c.adaptive == null ? "未记录" : c.adaptive ? "开" : "关"}</dd></div>
        <div><dt>力度与踏板目标</dt><dd>{c.expressionGoal == null ? "无专项要求" : expressionGoalText({ velocityDifference: c.expressionGoal.velocity_difference, noteVelocityDifference: c.expressionGoal.note_velocity_difference, notePedalMismatchCount: c.expressionGoal.note_pedal_mismatch_count, contourPercent: c.expressionGoal.contour_percent, pedalOffsetMs: c.expressionGoal.pedal_offset_ms })}</dd></div>
      </dl>
      {c.range && !c.passageBounds &&
        <p className="parameter-help">这条旧记录只保存了起止小节。软件无法确认两次练习的拍内起止位置是否相同；同小节的历史比较只可按小节参考。</p>}
      {!c.complete && <p className="parameter-help">这条记录缺少可确认的完整条件或有效判断，因此软件不会把它当作同条件比较的依据。</p>}
    </details>
  );
}
function Trend({ detail }: { detail: Detail }) {
  const points = [...detail.comparison.recent, detail];
  const judged = points
    .map((e, i) => ({
      entry: e,
      x: 12 + (i * 296) / Math.max(1, points.length - 1),
      value: accuracy(e.score),
    }))
    .filter((p) => p.value !== null);
  if (judged.length < 2) return null;
  return (
    <figure className="history-trend">
      <svg
        viewBox="0 0 320 90"
        role="img"
        aria-label={`最近 ${points.length} 次同条件正确率走势`}
      >
        <path d="M12 8H308M12 44H308M12 80H308" className="trend-grid" />
        <polyline
          points={judged.map((p) => `${p.x},${80 - p.value! * 72}`).join(" ")}
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        />
        {judged.map((p) => (
          <circle
            key={p.entry.id}
            cx={p.x}
            cy={80 - p.value! * 72}
            r={p.entry.id === detail.id ? 4 : 2.5}
          >
            <title>
              {date(p.entry.recordedAt)} · {percent(p.value)}
            </title>
          </circle>
        ))}
      </svg>
      <figcaption>
        同曲目、选段、模式、手别、速度、评分范围及完成量；仅比较此前记录。
      </figcaption>
    </figure>
  );
}
export function HistoryDialog({
  close,
  changed,
  backup,
  score,
  paper,
  finger,
  review,
  openPlan,
}: {
  openPlan: (id: string, day: string | null) => void;
  close: () => void;
  changed: () => Promise<void>;
  backup: () => void;
  score: (focus: HistoryScoreTarget) => void;
  paper: (focus: HistoryPaperTarget) => void;
  finger: (focus: HistoryPaperTarget) => Promise<void>;
  review: (data: HistoryScoreReview) => void;
}) {
  const [sources, setSources] = useState<ReviewSource[]>(() => {
    try {
      const saved: unknown = JSON.parse(
        localStorage.getItem("neothesia-review-sources") || "[]",
      );
      if (!Array.isArray(saved)) return [];
      const restored: ReviewSource[] = [];
      const seen = new Set<string>();
      for (const value of saved) {
        if (!value || typeof value !== "object") continue;
        const source = value as Partial<ReviewSource>;
        if (typeof source.contentId !== "string" ||
          !/^[a-f0-9]{64}$/.test(source.contentId) ||
          typeof source.id !== "string" || !source.id ||
          typeof source.title !== "string" ||
          typeof source.recordedAt !== "number" || !Number.isFinite(source.recordedAt)) continue;
        const key = JSON.stringify([source.contentId, source.id]);
        if (seen.has(key)) continue;
        seen.add(key);
        restored.push({
          contentId: source.contentId,
          id: source.id,
          title: source.title,
          recordedAt: source.recordedAt,
          slot: restored.length,
        });
        if (restored.length === 20) break;
      }
      return restored;
    } catch {
      return [];
    }
  });
  useEffect(() => {
    try {
      localStorage.setItem("neothesia-review-sources", JSON.stringify(sources));
    } catch {}
  }, [sources]);
  const [, setMultiDirty] = useState(false);
  const [filters, setFilters] = useState({
    query: "",
    mode: "",
    hands: "",
    range: "",
    days: 0,
    from: null as number | null,
    to: null as number | null,
  });
  const [offset, setOffset] = useState(0),
    [entries, setEntries] = useState<Entry[]>([]),
    [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true),
    [error, setError] = useState(""),
    [selected, setSelected] = useState<Entry | null>(null);
  const [notesDirty, setNotesDirty] = useState(false);
  const [detail, setDetail] = useState<Detail | null>(null),
    [detailLoading, setDetailLoading] = useState(false),
    [busy, setBusy] = useState(false),
    [order, setOrder] = useState("errors");
  const changeFilter = (patch: Partial<typeof filters>) => {
    setLoading(true);
    setDetail(null);
    setFilters((f) => ({ ...f, ...patch }));
    setOffset(0);
    setSelected(null);
  };
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError("");
    const timer = setTimeout(
      () =>
        void api
          .command<{ entries: Entry[]; total: number }>({
            type: "historyQuery",
            ...filters,
            offset,
            limit: 50,
          })
          .then((data) => {
            if (!cancelled) {
              setEntries(data.entries);
              setTotal(data.total);
            }
          })
          .catch((e) => {
            if (!cancelled) setError(String(e.message ?? e));
          })
          .finally(() => {
            if (!cancelled) setLoading(false);
          }),
      180,
    );
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [filters, offset]);
  useEffect(() => {
    let cancelled = false;
    setDetail(null);
    setDetailLoading(false);
    if (!selected) return;
    setDetailLoading(true);
    setError("");
    void api
      .command<Detail>({
        type: "historyDetail",
        content_id: selected.contentId,
        id: selected.id,
      })
      .then((data) => {
        if (!cancelled) setDetail(data);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e.message ?? e));
      })
      .finally(() => {
        if (!cancelled) setDetailLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [selected]);
  useEffect(() => {
    const listener = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !busy && !notesDirty) close();
    };
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, [close, busy, notesDirty]);
  const reviewScore = async () => {
    if (!detail || busy || notesDirty) return;
    setBusy(true);
    setError("");
    try {
      const result = await api.command<{
        scoreReview?: HistoryScoreReview;
        restoreWarning?: string | null;
      }>({
        type: "historyScoreReview",
        content_id: detail.contentId,
        id: detail.id,
      });
      await changed();
      if (result.restoreWarning) {
        setError(result.restoreWarning);
        return;
      }
      if (!result.scoreReview) throw new Error("历史谱面资料未读取，请重试");
      review(result.scoreReview);
      close();
    } catch (e) {
      await changed();
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  const practicePerformance = async (
    reference: number,
    before: number,
    after: number,
    destination?: "score" | "paper" | "finger",
  ) => {
    if (!detail || busy || notesDirty) return;
    setBusy(true);
    setError("");
    try {
      const result = await api.command<{
        restoreWarning?: string | null;
        scoreFocus?: HistoryScoreTarget | null;
        paperFocus?: HistoryPaperTarget | null;
      }>({
        type: "historyPracticeRange",
        content_id: detail.contentId,
        id: detail.id,
        reference,
        before,
        after,
      });
      await changed();
      if (result.restoreWarning) {
        setError(result.restoreWarning);
        return;
      }
      if (destination === "paper") {
        if (!result.paperFocus ||
            result.paperFocus.contentId !== detail.contentId ||
            result.paperFocus.historyId !== detail.id ||
            result.paperFocus.reference !== reference ||
            !Number.isSafeInteger(result.paperFocus.measure) ||
            result.paperFocus.measure < 1 ||
            !Number.isSafeInteger(result.paperFocus.pitch) ||
            result.paperFocus.pitch < 0 || result.paperFocus.pitch > 127 ||
            !Number.isSafeInteger(result.paperFocus.track) ||
            result.paperFocus.track < 0 ||
            !Number.isFinite(result.paperFocus.songTime) ||
            result.paperFocus.songTime < 0 ||
            (result.paperFocus.index != null &&
              (!Number.isSafeInteger(result.paperFocus.index) || result.paperFocus.index < 0)) ||
            !Number.isSafeInteger(result.paperFocus.startMeasure) ||
            !Number.isSafeInteger(result.paperFocus.endMeasure) ||
            result.paperFocus.startMeasure < 1 ||
            result.paperFocus.startMeasure > result.paperFocus.measure ||
            result.paperFocus.endMeasure < result.paperFocus.measure) {
          setError("选段已准备好，但当前历史参考音无法对应到练习小节。请核对曲目后再从纸谱定位。");
          return;
        }
        paper(result.paperFocus);
      }
      if (destination === "finger") {
        const focus = result.paperFocus;
        if (!focus || focus.contentId !== detail.contentId ||
            focus.historyId !== detail.id || focus.reference !== reference ||
            !Number.isSafeInteger(focus.track) || focus.track < 0 ||
            !Number.isSafeInteger(focus.pitch) || focus.pitch < 0 || focus.pitch > 127 ||
            !Number.isSafeInteger(focus.index) || (focus.index ?? -1) < 0 ||
            !Number.isFinite(focus.songTime) || focus.songTime < 0) {
          setError("选段已准备好，但这处历史音符没有唯一的原曲身份。请在当前曲目的指法建议中人工定位。");
          return;
        }
        await finger(focus);
      }
      if (destination === "score") {
        if (!result.scoreFocus) {
          setError(
            "选段已准备好，但当前曲目没有关联乐谱或参考音身份不唯一，无法在谱面上标记。可以直接重练，或先关联和校对乐谱。",
          );
          return;
        }
        score(result.scoreFocus);
      }
      close();
    } catch (e) {
      await changed();
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  const open = async (restore: boolean, measure?: number) => {
    if (!detail || busy || notesDirty) return;
    setBusy(true);
    setError("");
    try {
      const result = await api.command<{ restoreWarning?: string | null }>({
        type: "historyOpen",
        content_id: detail.contentId,
        id: detail.id,
        restore,
      });
      if (result.restoreWarning) {
        await changed();
        setError(result.restoreWarning);
        return;
      }
      if (measure !== undefined) {
        await api.command({ type: "mode", value: "flow" });
        await api.command({
          type: "measureLoop",
          start: measure,
          end: measure,
          enabled: true,
        });
      }
      await changed();
      close();
    } catch (e) {
      await changed();
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setBusy(false);
    }
  };
  const bars = detail
    ? [...detail.summary.measures].sort((a, b) =>
        order === "measure"
          ? a.measure - b.measure
          : b.breakdown.wrong_notes +
              b.breakdown.missed_notes -
              (a.breakdown.wrong_notes + a.breakdown.missed_notes) ||
            a.measure - b.measure,
      )
    : [];
  const timed =
    detail && ["flow", "recital", "memory"].includes(detail.mode ?? "");
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!busy && !notesDirty) close();
      }}
    >
      <section
        className="settings-dialog history-dialog history-workspace"
        role="dialog"
        aria-modal="true"
        aria-labelledby="history-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <h2 id="history-title">练习历史</h2>
          <span className="menu-spacer" />
          <button disabled={busy || notesDirty} onClick={backup}>
            练习资料备份
          </button>
          <button
            autoFocus
            disabled={busy || notesDirty}
            onClick={close}
            aria-label="关闭练习历史"
          >
            关闭
          </button>
        </div>
        <HistoryMultiProblemPlan
          sources={sources}
          remove={(id, cid) =>
            setSources((old) =>
              old.filter((s) => s.id !== id || s.contentId !== cid),
            )
          }
          openPlan={openPlan}
          dirtyChanged={setMultiDirty}
        />
        <PracticeCalendar
          selected={filters.from}
          disabled={busy || notesDirty}
          choose={(range) =>
            changeFilter({
              from: range?.from ?? null,
              to: range?.to ?? null,
              days: 0,
            })
          }
        />
        <div className="history-filters">
          <input
            aria-label="查找历史曲目"
            disabled={busy || notesDirty}
            placeholder="曲名、路径或评语"
            value={filters.query}
            onChange={(e) => changeFilter({ query: e.target.value })}
          />
          <select
            disabled={busy || notesDirty}
            aria-label="历史练习模式"
            value={filters.mode}
            onChange={(e) => changeFilter({ mode: e.target.value })}
          >
            <option value="">全部模式</option>
            {["wait", "flow", "recital", "memory", "listen"].map((m) => (
              <option key={m} value={m}>
                {modeName(m)}
              </option>
            ))}
          </select>
          <select
            disabled={busy || notesDirty}
            aria-label="历史练习手别"
            value={filters.hands}
            onChange={(e) => changeFilter({ hands: e.target.value })}
          >
            <option value="">全部手别</option>
            {["Both", "Left", "Right", "Custom", "Unspecified"].map((h) => (
              <option key={h} value={h}>
                {handName(h)}
              </option>
            ))}
          </select>
          <select
            disabled={busy || notesDirty}
            aria-label="历史练习范围"
            value={filters.range}
            onChange={(e) => changeFilter({ range: e.target.value })}
          >
            <option value="">全部范围</option>
            <option value="whole">全曲</option>
            <option value="loop">选段</option>
          </select>
          <select
            disabled={busy || notesDirty}
            aria-label="历史日期范围"
            value={filters.days}
            onChange={(e) =>
              changeFilter({
                days: Number(e.target.value),
                from: null,
                to: null,
              })
            }
          >
            <option value="0">全部日期</option>
            <option value="1">最近 24 小时</option>
            <option value="7">最近 7 天</option>
            <option value="30">最近 30 天</option>
            <option value="90">最近 90 天</option>
          </select>
        </div>
        <p className="history-filter-help">上方选项只筛选记录列表。选中成绩后的“同条件比较”会核对保存时的曲目内容、小节与音符、练习范围、音轨、弹法、速度、判断数和设备设置；搜索词与日期筛选不会改变比较结果。</p>
        {error && (
          <p className="history-error" role="alert">
            {error}
          </p>
        )}
        <div className="history-split">
          <div className="history-records" aria-busy={loading}>
            <div className="history-pagination">
              <span>{loading ? "读取中…" : `${total} 条记录`}</span>
              <button
                disabled={busy || notesDirty || loading || offset === 0}
                onClick={() => setOffset(Math.max(0, offset - 50))}
              >
                上一页
              </button>
              <span>
                {Math.floor(offset / 50) + 1} /{" "}
                {Math.max(1, Math.ceil(total / 50))}
              </span>
              <button
                disabled={busy || notesDirty || loading || offset + 50 >= total}
                onClick={() => setOffset(offset + 50)}
              >
                下一页
              </button>
            </div>
            <div className="history-list">
              {!loading && !entries.length ? (
                <p className="history-empty">没有符合条件的记录。</p>
              ) : (
                entries.map((e) => (
                  <button
                    className={`history-record ${selected?.id === e.id && selected.contentId === e.contentId ? "selected" : ""}`}
                    key={`${e.contentId}-${e.id}`}
                    disabled={busy || notesDirty || loading}
                    onClick={() => setSelected(e)}
                  >
                    <span className="history-record-title">
                      <strong>
                        {e.title}
                        {e.annotation &&
                          [
                            e.annotation.note,
                            e.annotation.teacher,
                            e.annotation.next,
                          ].some(Boolean) && (
                            <small className="history-note-mark">
                              {" "}
                              · 有评语
                            </small>
                          )}
                      </strong>
                      <b>{percent(accuracy(e.score))}</b>
                    </span>
                    <span>
                      {date(e.recordedAt)} ·{" "}
                      {e.playingMs != null
                        ? practiceDuration(e.playingMs)
                        : "旧记录未计时"}{" "}
                      ·{" "}
                      {e.coverage &&
                      e.coverage.target > 0 &&
                      e.coverage.judged < e.coverage.target
                        ? "部分结果"
                        : ""}
                    </span>
                    <span>
                      {rangeName(e)} · {modeName(e.mode)} · {handName(e.hands)}{" "}
                      · {Math.round(e.speed * 100)}%
                    </span>
                  </button>
                ))
              )}
            </div>
            <p className="parameter-help">
              每首曲目保留最近 200 次已保存结果。循环按轮保存。
            </p>
          </div>
          <div className="history-detail" aria-busy={detailLoading}>
            {detailLoading ? (
              <p>正在读取详情…</p>
            ) : !detail ? (
              <p className="history-empty">
                选择一条记录，查看逐小节成绩、同条件比较，或恢复练习。
              </p>
            ) : (
              <>
                <h3>{detail.title}</h3>
                <button
                  disabled={
                    busy ||
                    notesDirty ||
                    !detail.performanceAvailable ||
                    sources.length >= 20 ||
                    sources.some(
                      (s) =>
                        s.id === detail.id && s.contentId === detail.contentId,
                    )
                  }
                  onClick={() =>
                    setSources((old) => {
                      if (old.length >= 20 || old.some((source) =>
                        source.id === detail.id && source.contentId === detail.contentId)) return old;
                      const occupied = new Set(old.map((source) => source.slot));
                      let slot = 0;
                      while (occupied.has(slot)) slot++;
                      return [...old, {
                        contentId: detail.contentId,
                        id: detail.id,
                        title: detail.title,
                        recordedAt: detail.recordedAt,
                        slot,
                      }];
                    })
                  }
                >
                  加入复习编排
                </button>
                <p className="history-date">
                  {date(detail.recordedAt)} · {rangeName(detail)}
                </p>
                <div className="history-condition">
                  {modeName(detail.mode)} · {handName(detail.hands)} ·{" "}
                  {Math.round(detail.speed * 100)}%
                  {detail.tempo !== null && ` · ${detail.tempo} BPM`}
                </div>
                <p className="parameter-help">
                  {detail.playingMs != null
                    ? `练习运行用时 ${practiceDuration(detail.playingMs)}（含等音等待，不含暂停和预备拍）`
                    : "旧记录未计时，不按曲长估算用时。"}
                </p>
                <p className="history-coverage">{coverageName(detail)}</p>
                <div className="history-result">
                  <strong>{percent(accuracy(detail.score))}</strong>
                  <span>
                    正确 {detail.score.matched_notes} · 错音{" "}
                    {detail.score.wrong_notes} · 漏音{" "}
                    {detail.score.missed_notes}
                  </span>
                </div>
                {detail.context ? (
                  <p className="parameter-help">
                    预备 {detail.context.count_in} 小节 · 节拍器
                    {detail.context.metronome ? "开" : "关"} · 自适应速度
                    {detail.context.adaptive ? "开" : "关"} ·{" "}
                    {detail.context.rounds || "不限"} 轮 · 输入校准{" "}
                    {detail.context.latency} ms
                  {detail.context.pedal_latency_ms != null
                      ? `（踏板同步 ${detail.context.pedal_latency_ms} ms）`
                      : `（旧记录踏板沿用输入校准 ${detail.context.latency} ms）`}
                    <br />
                    {detail.context.tracks
                      .map((t) => `音轨 ${t.track_id + 1}：${trackModeName(t.mode)}${t.visible ? "" : "（隐藏）"}`)
                      .join("；")}
                  </p>
                ) : (
                  <p className="parameter-help">
                    旧记录未保存完整声部与预备设置，无法精确恢复条件。
                  </p>
                )}
                {detail.path && (
                  <details className="history-source">
                    <summary>记录时的曲目文件</summary>
                    <span>{detail.path}</span>
                  </details>
                )}
                {detail.context?.routine && (
                  <p className="parameter-help">
                    练习计划：{detail.context.routine.routine_name} ·{" "}
                    {detail.context.routine.day} ·{" "}
                    {detail.context.routine.item_title}
                    <br />
                    {detail.context.routine.consecutive
                      ? "连续达标"
                      : "累计达标"}{" "}
                    {detail.context.routine.passes_required} 次 · 正确率 ≥
                    {detail.context.routine.accuracy_percent}%
                    {detail.context.routine.expression && (
                      <>
                        <br />
                        专项目标：
                        {expressionGoalText({
                          velocityDifference:
                            detail.context.routine.expression
                              .velocity_difference,
                          noteVelocityDifference:
                            detail.context.routine.expression.note_velocity_difference,
                          notePedalMismatchCount:
                            detail.context.routine.expression.note_pedal_mismatch_count,
                          contourPercent:
                            detail.context.routine.expression.contour_percent,
                          pedalOffsetMs:
                            detail.context.routine.expression.pedal_offset_ms,
                        })}
                      </>
                    )}
                    {detail.expressionGoalResult && (
                      <>
                        <br />
                        本轮专项：{detail.expressionGoalResult.message}
                        {detail.expressionGoalResult.noteVelocity && <> · {noteVelocityEvidenceText(detail.expressionGoalResult.noteVelocity)}</>}
                        {detail.expressionGoalResult.notePedal && <> · {notePedalEvidenceText(detail.expressionGoalResult.notePedal)}</>}
                      </>
                    )}
                    {detail.context.routine.item_notes && (
                      <>
                        <br />
                        {detail.context.routine.item_notes}
                      </>
                    )}
                  </p>
                )}
                {!detail.context?.routine && detail.context?.expression && (
                  <p className="parameter-help">
                    本次单曲目标：{expressionGoalText({
                      velocityDifference: detail.context.expression.velocity_difference,
                      noteVelocityDifference: detail.context.expression.note_velocity_difference,
                      notePedalMismatchCount: detail.context.expression.note_pedal_mismatch_count,
                      contourPercent: detail.context.expression.contour_percent,
                      pedalOffsetMs: detail.context.expression.pedal_offset_ms,
                    })}
                    {detail.expressionGoalResult && <><br />本轮目标：{detail.expressionGoalResult.message}{detail.expressionGoalResult.noteVelocity && <> · {noteVelocityEvidenceText(detail.expressionGoalResult.noteVelocity)}</>}{detail.expressionGoalResult.notePedal && <> · {notePedalEvidenceText(detail.expressionGoalResult.notePedal)}</>}</>}
                  </p>
                )}
                {detail.context?.ladder && (
                  <p className="parameter-help">
                    速度阶梯：{detail.context.ladder.plan_name} · 第{" "}
                    {detail.context.ladder.stage} 级 ·{" "}
                    {detail.context.ladder.speed_percent}% →{" "}
                    {detail.context.ladder.target_percent}%
                  </p>
                )}
                <div className="history-open-actions">
                  <button
                    disabled={busy || notesDirty}
                    onClick={() => void open(false)}
                  >
                    打开曲目
                  </button>
                  <button
                    className="primary"
                    disabled={busy || notesDirty || !detail.restorable}
                    onClick={() => void open(true)}
                  >
                    {busy ? "正在打开…" : "恢复这次练习"}
                  </button>
                </div>
                <HistoryPerformance
                  key={`performance:${detail.contentId}:${detail.id}`}
                  contentId={detail.contentId}
                  id={detail.id}
                  goalThreshold={detail.context?.routine?.expression?.note_velocity_difference ?? detail.context?.expression?.note_velocity_difference}
                  available={!!detail.performanceAvailable}
                  review={reviewScore}
                  practice={practicePerformance}
                  blocked={busy || notesDirty || !detail.restorable}
                />
                <HistoryAnnotationEditor
                  key={`${detail.contentId}:${detail.id}`}
                  value={detail.annotation}
                  locked={busy}
                  onLocked={setNotesDirty}
                  save={async (draft, expected) => {
                    const updated = await api.command<Detail>({
                      type: "annotateHistory",
                      content_id: detail.contentId,
                      id: detail.id,
                      ...draft,
                      expected,
                    });
                    setDetail(updated);
                    try {
                      const rows = await api.command<{
                        entries: Entry[];
                        total: number;
                      }>({
                        type: "historyQuery",
                        ...filters,
                        offset,
                        limit: 50,
                      });
                      setEntries(rows.entries);
                      setTotal(rows.total);
                    } catch (e) {
                      setError(
                        `评语已保存，但列表刷新失败：${String(e instanceof Error ? e.message : e)}`,
                      );
                    }
                  }}
                />
                <h4>同条件比较</h4>
                <ComparisonConditions detail={detail} />
                {detail.comparison.count ? (
                  <>
                    <p className="history-comparison">
                      相对此前一次：
                      {delta(
                        accuracy(detail.score),
                        detail.comparison.previous
                          ? accuracy(detail.comparison.previous.score)
                          : null,
                      )}
                      <br />
                      此前最佳：{percent(detail.comparison.bestAccuracy)} · 共{" "}
                      {detail.comparison.count} 次
                    </p>
                    <Trend detail={detail} />
                  </>
                ) : (
                  <p className="parameter-help">{detail.comparison.reason ?? "此前没有可比较记录。"}</p>
                )}
                {!!detail.comparison.excludedRecent?.length && (
                  <details className="history-excluded-attempts">
                    <summary>查看未纳入比较的旧成绩（最近 {detail.comparison.excludedRecent.length} 条）</summary>
                    <p className="parameter-help">软件按保存时间列出最近的不同条件成绩。每条原因来自本地引擎保存的练习条件。</p>
                    <ul>
                      {detail.comparison.excludedRecent.map(({ entry, reasons }) => (
                        <li key={`${entry.contentId}:${entry.id}`}>
                          <div><strong>{date(entry.recordedAt)} · {percent(accuracy(entry.score))}</strong><button disabled={busy || notesDirty} onClick={() => setSelected(entry)}>查看这次成绩</button></div>
                          <span>{rangeName(entry)} · {modeName(entry.mode)} · {handName(entry.hands)} · {Math.round(entry.speed * 100)}%</span>
                          <p>{reasons.join("；")}</p>
                        </li>
                      ))}
                    </ul>
                  </details>
                )}
                {timed ? (
                  <p className="history-timing">
                    节奏偏移 {detail.summary.timing.median_offset_ms ?? "—"} ms
                    · 波动 {detail.summary.timing.median_deviation_ms ?? "—"} ms
                    <br />
                    完整和弦 {detail.summary.chords.complete_chords} /{" "}
                    {detail.summary.chords.eligible_chords} · 起音跨度{" "}
                    {detail.summary.chords.median_attack_span_ms ?? "—"} ms
                    <br />
                    时值比例{" "}
                    {detail.summary.expression.articulation
                      .median_duration_ratio_percent ?? "—"}
                    % · 力度平均差{" "}
                    {detail.summary.expression.velocity.mean_abs_difference ??
                      "—"}
                  </p>
                ) : (
                  <p className="parameter-help">
                    等音记录只比较音符正确率，不评价节奏。
                  </p>
                )}
                {detail.summary.parts.length > 0 && (
                  <div className="history-parts">
                    {detail.summary.parts.map((p) => (
                      <span key={p.part}>
                        {{
                          LeftHand: "左手",
                          RightHand: "右手",
                          Other: "其他声部",
                        }[p.part] ?? p.part}{" "}
                        {percent(accuracy(p.breakdown))} · 错{" "}
                        {p.breakdown.wrong_notes} / 漏{" "}
                        {p.breakdown.missed_notes}
                      </span>
                    ))}
                  </div>
                )}
                <div className="history-bars-heading">
                  <h4>逐小节成绩</h4>
                  <select
                    disabled={busy || notesDirty}
                    aria-label="历史小节排序"
                    value={order}
                    onChange={(e) => setOrder(e.target.value)}
                  >
                    <option value="errors">错漏最多优先</option>
                    <option value="measure">按小节顺序</option>
                  </select>
                </div>
                <table className="history-bar-table">
                  <thead>
                    <tr>
                      <th>小节</th>
                      <th>正确率</th>
                      <th>错 / 漏</th>
                      <th>{timed ? "偏移 ms" : ""}</th>
                      <th>重练</th>
                    </tr>
                  </thead>
                  <tbody>
                    {bars.map((b) => (
                      <tr key={b.measure}>
                        <td>{b.measure}</td>
                        <td>{percent(accuracy(b.breakdown))}</td>
                        <td>
                          {b.breakdown.wrong_notes} / {b.breakdown.missed_notes}
                        </td>
                        <td>
                          {timed ? (b.timing.median_offset_ms ?? "—") : "—"}
                        </td>
                        <td>
                          <button
                            disabled={busy || notesDirty || !detail.restorable}
                            aria-label={`重练第 ${b.measure} 小节`}
                            onClick={() => void open(true, b.measure)}
                          >
                            重练
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {bars.length === 0 && (
                  <p className="parameter-help">这条记录没有逐小节数据。</p>
                )}
              </>
            )}
          </div>
        </div>
      </section>
    </div>
  );
}
