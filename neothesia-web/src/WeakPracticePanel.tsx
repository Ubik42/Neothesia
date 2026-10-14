import { useState } from "react";
import { api } from "./api";
import {
  expressionGoalText,
  type ExpressionGoal,
} from "./ExpressionGoalControl";
interface Suggestion {
  id: string;
  title: string;
  measure: number;
  accuracy: number;
  kind: "accuracy" | "rhythm" | "dynamics" | "pedal" | "review";
  range: string | null;
  expression: {
    contourPercent: number | null;
    contourSteps: number;
    samples: number;
    meanDifference: number | null;
    pedalOffsetMs: number | null;
    pedalAbsoluteOffsetMs: number | null;
    pedalDeviationMs: number | null;
    referenceTransitions: number;
    correspondenceFailures: number;
    calibratedLegacy: number;
  } | null;
  timing: {
    onTime: number;
    samples: number;
    offsetMs: number;
    deviationMs: number;
  } | null;
  attempts: number;
  judged: number;
  sourceAt: number;
  sourceSpeed: number;
  sourceMode: string;
  item: {
    notes: string;
    speed: number;
    settings: { mode: string; hands: string; metronome: boolean };
    goal: {
      passes: number;
      accuracy: number;
      onTime: number | null;
      expression?: ExpressionGoal | null;
    };
  };
}
type SuggestionResult = {
  suggestions: Suggestion[];
  note: string;
  dueTotal?: number;
  eligibleTotal?: number;
  revision?: string;
  nextOffset?: number;
  hasMore?: boolean;
};
export function WeakPracticePanel({
  routineId,
  day,
  disabled,
  work,
  added,
}: {
  routineId: string | null;
  day: string | null;
  disabled: boolean;
  work: (job: () => Promise<void>) => Promise<void>;
  added: (id: string) => Promise<void>;
}) {
  const [rows, setRows] = useState<Suggestion[]>([]),
    [chosen, setChosen] = useState<string[]>([]),
    [note, setNote] = useState(""),
    [duePage, setDuePage] = useState<{
      dueTotal: number;
      eligibleTotal: number;
      revision: string;
      nextOffset: number;
      hasMore: boolean;
    } | null>(null),
    [name, setName] = useState("专项复习"),
    [message, setMessage] = useState("");
  const generate = () =>
    void work(async () => {
      const [due, weak] = await Promise.all([
        api.command<SuggestionResult>({ type: "dueReview" }),
        api.command<SuggestionResult>({ type: "weakPractice" }),
      ]);
      const suggestions = [
        ...weak.suggestions,
        ...due.suggestions,
      ];
      setRows(suggestions);
      setChosen((weak.suggestions.length ? weak.suggestions : due.suggestions)
        .slice(0, 6).map((r) => r.id));
      setNote(`${due.note} ${weak.note}`);
      setDuePage({
        dueTotal: due.dueTotal ?? 0,
        eligibleTotal: due.eligibleTotal ?? 0,
        revision: due.revision ?? "",
        nextOffset: due.nextOffset ?? due.suggestions.length,
        hasMore: due.hasMore === true,
      });
      setMessage(
        suggestions.length
          ? ""
          : due.dueTotal
            ? "待复习曲目的原条件暂不能自动编排。请在曲库打开曲目核对，或完成几轮后重新生成。"
            : "暂无足够的重复薄弱证据；完成几轮后再生成。",
      );
    });
  const loadMoreDue = () =>
    void work(async () => {
      if (!duePage?.hasMore) return;
      const data = await api.command<SuggestionResult>({
        type: "dueReview",
        offset: duePage.nextOffset,
        expected: duePage.revision,
      });
      setRows((current) => [
        ...current,
        ...data.suggestions.filter((row) => !current.some((item) => item.id === row.id)),
      ]);
      setDuePage({
        dueTotal: data.dueTotal ?? duePage.dueTotal,
        eligibleTotal: data.eligibleTotal ?? duePage.eligibleTotal,
        revision: data.revision ?? duePage.revision,
        nextOffset: data.nextOffset ?? duePage.nextOffset + data.suggestions.length,
        hasMore: data.hasMore === true,
      });
    });
  const shownDue = rows.filter((row) => row.kind === "review").length;
  return (
    <details className="weak-practice-panel">
      <summary>从历史练习安排复习</summary>
      <button disabled={disabled} onClick={generate}>
        生成复习建议
      </button>
      <p className="parameter-help">
        预览后勾选加入
        {routineId
          ? day
            ? "当前日期安排"
            : "当前常用计划"
          : day
            ? "新计划与当前日期安排"
            : "新的常用计划"}
        ，不自动演奏。
      </p>
      {duePage && duePage.dueTotal > 0 && (
        <p className="parameter-help">
          曲库中 {duePage.dueTotal} 首曲目待复习；{duePage.eligibleTotal} 首可按原条件加入计划，当前已读取 {shownDue} 首。
          {duePage.dueTotal > duePage.eligibleTotal &&
            ` 其余 ${duePage.dueTotal - duePage.eligibleTotal} 首请在曲库直接打开核对。`}
        </p>
      )}
      {note && (
        <details className="weak-practice-method">
          <summary>建议依据与适用范围</summary>
          <p className="parameter-help">{note}</p>
        </details>
      )}
      {!routineId && (
        <label className="weak-plan-name">
          新计划名称
          <input
            aria-label="复习计划名称"
            maxLength={80}
            value={name}
            disabled={disabled}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
      )}
      {rows.length > 0 && (
        <>
          <div className="weak-practice-rows">
            {rows.map((r) => (
              <div className="weak-practice-row" key={r.id}>
              <label>
                <input
                  type="checkbox"
                  aria-label={
                    r.expression
                      ? `选择 ${r.title} ${r.range} ${r.kind === "dynamics" ? "力度起伏" : "踏板时机"}`
                      : r.kind === "review"
                        ? `选择 ${r.title} 全曲到期复习`
                      : `选择 ${r.title} 第 ${r.measure} 小节`
                  }
                  disabled={disabled || (!chosen.includes(r.id) && chosen.length >= 60)}
                  checked={chosen.includes(r.id)}
                  onChange={(e) =>
                    setChosen((v) =>
                      e.target.checked
                        ? [...v, r.id]
                        : v.filter((id) => id !== r.id),
                    )
                  }
                />
                <span>
                  <strong>
                    {r.title} · {r.range ?? `第 ${r.measure} 小节`} ·{" "}
                    {
                      {
                        rhythm: "节奏",
                        accuracy: "音符",
                        dynamics: "力度起伏",
                        pedal: "踏板时机",
                        review: "到期复习",
                      }[r.kind]
                    }
                  </strong>
                  <small>
                    {r.kind === "review" ? "最近正确率" : "正确率"} {Math.round(r.accuracy * 100)}% ·{" "}
                    {r.kind === "review"
                      ? `同条件连续达标 ${r.attempts} 次`
                      : `${r.attempts}${r.expression ? " 轮完整练习" : " 次完整小节"}`} ·{" "}
                    {r.judged} 次音符判断
                  </small>
                  {r.timing && (
                    <small>
                      准时率 {Math.round(r.timing.onTime * 100)}% ·{" "}
                      {r.timing.samples} 个节奏样本 · 偏移{" "}
                      {Math.round(r.timing.offsetMs)} ms · 波动{" "}
                      {Math.round(r.timing.deviationMs)} ms
                    </small>
                  )}
                  {r.expression && (
                    <small>
                      {r.kind === "dynamics" ? (
                        <>
                          起伏一致率{" "}
                          {Math.round(r.expression.contourPercent ?? 0)}% ·{" "}
                          {r.expression.contourSteps} 段参考变化 ·{" "}
                          {r.expression.samples} 个力度样本
                        </>
                      ) : (
                        <>
                          参考 {r.expression.referenceTransitions} 次踏板转换 ·{" "}
                          {r.expression.correspondenceFailures} 轮未完整对应
                          {r.expression.pedalOffsetMs != null &&
                            ` · 偏移汇总 ${Math.round(r.expression.pedalOffsetMs)} ms`}
                          {r.expression.pedalAbsoluteOffsetMs != null &&
                            ` · 绝对偏移汇总 ${Math.round(r.expression.pedalAbsoluteOffsetMs)} ms`}
                          {r.expression.pedalDeviationMs != null &&
                            ` · 波动汇总 ${Math.round(r.expression.pedalDeviationMs)} ms`}
                          {r.expression.calibratedLegacy > 0 &&
                            ` · ${r.expression.calibratedLegacy} 轮旧记录已换算输入延迟`}
                        </>
                      )}
                    </small>
                  )}
                  <small>
                    原条件：{Math.round(r.sourceSpeed * 100)}% ·{" "}
                    {
                      (
                        {
                          wait: "等音",
                          flow: "连续",
                          recital: "完整演奏",
                          memory: "背谱",
                        } as Record<string, string>
                      )[r.sourceMode]
                    }{" "}
                    ·{" "}
                    {(
                      {
                        both: "双手",
                        left: "左手",
                        right: "右手",
                        custom: "自定义声部",
                      } as Record<string, string>
                    )[r.item.settings.hands] ?? r.item.settings.hands}
                  </small>
                  <small>
                    {({ wait: "等音", flow: "连续", recital: "完整演奏", memory: "背谱" } as Record<string, string>)[r.item.settings.mode] ?? r.item.settings.mode} ·{" "}
                    {Math.round(r.item.speed * 100)}% · 连续{" "}
                    {r.item.goal.passes} 轮达标，正确率 ≥ {r.item.goal.accuracy}%
                    {r.item.goal.expression &&
                      `、${expressionGoalText(r.item.goal.expression)}`}
                    {r.item.goal.onTime !== null &&
                    r.item.goal.onTime !== undefined
                      ? `、准时率 ≥ ${r.item.goal.onTime}%`
                      : ""}
                    {r.item.settings.metronome ? "、节拍器开启" : ""}
                    ；打开时检查曲目版本与练习范围
                  </small>
                </span>
              </label>
              <details className="weak-practice-reason">
                <summary>查看建议依据与练习要求</summary>
                <p>{r.item.notes}</p>
                <p>最近一轮相关成绩：{new Date(r.sourceAt).toLocaleString("zh-CN")}</p>
              </details>
              </div>
            ))}
          </div>
          {duePage?.hasMore && (
            <button disabled={disabled} onClick={loadMoreDue}>
              再读取 30 首到期曲目
            </button>
          )}
          <button
            disabled={
              disabled || !chosen.length || (!routineId && !name.trim())
            }
            onClick={() =>
              void work(async () => {
                const result = await api.command<{
                  id: string;
                  added: number;
                  skipped: number;
                }>({
                  type: "acceptWeakPractice",
                  ids: chosen,
                  routine_id: routineId,
                  day,
                  name,
                });
                await added(result.id);
                setMessage(
                  `已加入 ${result.added} 项，跳过相同条件项目 ${result.skipped} 项。`,
                );
              })
            }
          >
            加入勾选的复习项目
          </button>
        </>
      )}
      {message && <p role="status">{message}</p>}
    </details>
  );
}
