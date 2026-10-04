import {
  HistoryMultiProblemPlan,
  type ReviewSource,
} from "./HistoryMultiProblemPlan";
import type {
  HistoryScoreTarget,
  HistoryScoreReview,
} from "./historyScoreFocus";
import { HistoryPerformance } from "./HistoryPerformance";
import { practiceDuration } from "./practiceTime";
import { expressionGoalText } from "./ExpressionGoalControl";
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
  score: Snapshot["score"];
  restorable: boolean;
  tempo: number | null;
}
interface Detail extends Entry {
  performanceAvailable?: boolean;
  summary: Feedback;
  expressionGoalResult?: { passed: boolean; message: string };
  context: {
    routine?: {
      expression?: {
        velocity_difference: number | null;
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
  e.range ? `第 ${e.range[0]}–${e.range[1]} 小节` : "全曲";
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
  review,
  openPlan,
}: {
  openPlan: (id: string, day: string | null) => void;
  close: () => void;
  changed: () => Promise<void>;
  backup: () => void;
  score: (focus: HistoryScoreTarget) => void;
  review: (data: HistoryScoreReview) => void;
}) {
  const [sources, setSources] = useState<ReviewSource[]>(() => {
    try {
      return (
        JSON.parse(
          localStorage.getItem("neothesia-review-sources") || "[]",
        ) as ReviewSource[]
      )
        .filter(
          (s) =>
            typeof s.contentId === "string" &&
            /^[a-f0-9]{64}$/.test(s.contentId) &&
            typeof s.id === "string" &&
            typeof s.title === "string" &&
            Number.isSafeInteger(s.slot) &&
            s.slot >= 0,
        )
        .slice(0, 20);
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
    showScore = false,
  ) => {
    if (!detail || busy || notesDirty) return;
    setBusy(true);
    setError("");
    try {
      const result = await api.command<{
        restoreWarning?: string | null;
        scoreFocus?: HistoryScoreTarget | null;
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
      if (showScore) {
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
            onChange={(e) => changeFilter({ days: Number(e.target.value) })}
          >
            <option value="0">全部日期</option>
            <option value="1">最近 24 小时</option>
            <option value="7">最近 7 天</option>
            <option value="30">最近 30 天</option>
            <option value="90">最近 90 天</option>
          </select>
        </div>
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
                    setSources((old) => [
                      ...old,
                      {
                        contentId: detail.contentId,
                        id: detail.id,
                        title: detail.title,
                        recordedAt: detail.recordedAt,
                        slot: Math.max(-1, ...old.map((s) => s.slot)) + 1,
                      },
                    ])
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
                      : "（旧记录踏板未校准）"}
                    <br />
                    {detail.context.tracks
                      .map(
                        (t) =>
                          `音轨 ${t.track_id + 1}：${{ Human: "自己弹", Auto: "伴奏", Mute: "静音" }[t.mode] ?? t.mode}${t.visible ? "" : "（隐藏）"}`,
                      )
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
                  <p className="parameter-help">
                    此前没有可比较记录。不同选段、手别、模式、速度、评分范围或完成量分别比较。
                  </p>
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
