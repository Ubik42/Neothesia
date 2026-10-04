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
  kind: "accuracy" | "rhythm" | "dynamics" | "pedal";
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
    speed: number;
    settings: { mode: string; hands: string };
    goal: {
      passes: number;
      accuracy: number;
      onTime: number | null;
      expression?: ExpressionGoal | null;
    };
  };
}
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
    [name, setName] = useState("专项复习"),
    [message, setMessage] = useState("");
  const generate = () =>
    void work(async () => {
      const data = await api.command<{
        suggestions: Suggestion[];
        note: string;
      }>({ type: "weakPractice" });
      setRows(data.suggestions);
      setChosen(data.suggestions.slice(0, 6).map((r) => r.id));
      setNote(data.note);
      setMessage(
        data.suggestions.length
          ? ""
          : "暂无足够的重复薄弱证据；完成几轮后再生成。",
      );
    });
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
              <label key={r.id}>
                <input
                  type="checkbox"
                  aria-label={
                    r.expression
                      ? `选择 ${r.title} ${r.range} ${r.kind === "dynamics" ? "力度起伏" : "踏板时机"}`
                      : `选择 ${r.title} 第 ${r.measure} 小节`
                  }
                  disabled={disabled}
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
                      }[r.kind]
                    }
                  </strong>
                  <small>
                    正确率 {Math.round(r.accuracy * 100)}% · {r.attempts}
                    {r.expression ? " 轮完整练习" : " 次完整小节"} · {r.judged}{" "}
                    次音符判断
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
                    {r.item.settings.mode === "wait" ? "等音" : "连续"} ·{" "}
                    {Math.round(r.item.speed * 100)}% · 连续{" "}
                    {r.item.goal.passes} 轮达标
                    {r.item.goal.expression &&
                      `，正确率 ≥ ${r.item.goal.accuracy}%、${expressionGoalText(r.item.goal.expression)}`}
                    {r.item.goal.onTime !== null &&
                    r.item.goal.onTime !== undefined
                      ? `，正确率 ≥ ${r.item.goal.accuracy}%、准时率 ≥ ${r.item.goal.onTime}%、节拍器开启`
                      : ""}
                    ；打开时检查曲目版本与练习范围
                  </small>
                </span>
              </label>
            ))}
          </div>
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
