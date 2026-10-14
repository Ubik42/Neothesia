import { useEffect, useRef, useState } from "react";
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
  missingTotal?: number;
  revision?: string;
  nextOffset?: number;
  hasMore?: boolean;
};
type AddedResult = { id: string; added: number; skipped: number; existing?: boolean };
type ReviewEdit = { speedPercent: number; passes: number; accuracy: number; onTime: number | null };
type Submission = {
  ids: string[];
  edits: { id: string; speed: number; passes: number; accuracy: number; onTime: number | null }[];
  routineId: string | null;
  day: string | null;
  name: string;
  requestId: string | null;
};
const pendingKey = (routineId: string | null, day: string | null) =>
  `neothesia-pending-review:${JSON.stringify([routineId, day])}`;
const readPending = (routineId: string | null, day: string | null): Submission | null => {
  try {
    const value = sessionStorage.getItem(pendingKey(routineId, day));
    if (!value) return null;
    const result: Submission = JSON.parse(value);
    if (result.routineId !== routineId || result.day !== day ||
        !Array.isArray(result.ids) || !result.ids.length ||
        !result.ids.every((id) => typeof id === "string") ||
        !Array.isArray(result.edits) || result.edits.length !== result.ids.length ||
        !result.edits.every((edit, index) => edit && edit.id === result.ids[index] &&
          typeof edit.speed === "number" && typeof edit.passes === "number" &&
          typeof edit.accuracy === "number" &&
          (edit.onTime === null || typeof edit.onTime === "number")) ||
        typeof result.name !== "string" ||
        (result.requestId !== null && typeof result.requestId !== "string")) return null;
    return result;
  } catch {
    return null;
  }
};
const suggestedEdit = (row: Suggestion): ReviewEdit => ({
  speedPercent: Math.round(row.item.speed * 100),
  passes: row.item.goal.passes,
  accuracy: row.item.goal.accuracy,
  onTime: row.item.goal.onTime,
});
const addedMessage = (result: AddedResult) =>
  result.existing
    ? "计划已保存；本次重试没有重复创建。"
    : `已加入 ${result.added} 项，跳过相同条件项目 ${result.skipped} 项。`;
export function WeakPracticePanel({
  routineId,
  day,
  disabled,
  work,
  added,
  openLibrary,
  resumeRevision,
}: {
  routineId: string | null;
  day: string | null;
  disabled: boolean;
  work: (job: () => Promise<void>) => Promise<void>;
  added: (id: string) => Promise<void>;
  openLibrary: (focus: "missing" | "review") => void;
  resumeRevision: number;
}) {
  const lastResume = useRef(resumeRevision);
  const requestRevision = useRef(0);
  const draftRequestId = useRef<string | null>(null);
  const [rows, setRows] = useState<Suggestion[]>([]),
    [chosen, setChosen] = useState<string[]>([]),
    [edits, setEdits] = useState<Record<string, ReviewEdit>>({}),
    [note, setNote] = useState(""),
    [duePage, setDuePage] = useState<{
      dueTotal: number;
      eligibleTotal: number;
      missingTotal: number;
      revision: string;
      nextOffset: number;
      hasMore: boolean;
    } | null>(null),
    [name, setName] = useState("专项复习"),
    [message, setMessage] = useState("");
  const [pendingAdded, setPendingAdded] = useState<AddedResult | null>(null);
  const [pendingSubmission, setPendingSubmission] = useState<Submission | null>(
    () => readPending(routineId, day),
  );
  const rememberSubmission = (submission: Submission) => {
    setPendingSubmission(submission);
    try {
      sessionStorage.setItem(pendingKey(submission.routineId, submission.day), JSON.stringify(submission));
    } catch { /* The current dialog still keeps this retry. */ }
  };
  const clearSubmission = (submission: Submission) => {
    setPendingSubmission(null);
    try {
      sessionStorage.removeItem(pendingKey(submission.routineId, submission.day));
    } catch { /* Storage may be unavailable. */ }
  };
  useEffect(() => {
    setPendingSubmission(readPending(routineId, day));
  }, [routineId, day]);
  const locked = disabled || !!pendingSubmission || !!pendingAdded;
  const submit = async (submission: Submission) => {
    let result: AddedResult;
    try {
      result = await api.command<AddedResult>({
        type: "acceptWeakPractice",
        ids: submission.ids,
        edits: submission.edits,
        routine_id: submission.routineId,
        day: submission.day,
        name: submission.name,
        request_id: submission.requestId,
      });
    } catch (cause) {
      setMessage("上次加入结果尚未确认。请用原条件重试核对，避免新建重复计划。");
      throw cause;
    }
    clearSubmission(submission);
    draftRequestId.current = null;
    setPendingAdded(result);
    try {
      await added(result.id);
      setPendingAdded(null);
      setMessage(addedMessage(result));
    } catch (cause) {
      const detail = cause instanceof Error ? cause.message : String(cause);
      setMessage(`${addedMessage(result)} 计划已保存，但界面读取失败：${detail}。请重试显示计划。`);
    }
  };
  useEffect(() => {
    if (lastResume.current === resumeRevision) return;
    lastResume.current = resumeRevision;
    requestRevision.current++;
    draftRequestId.current = null;
    setRows([]);
    setChosen([]);
    setEdits({});
    setNote("");
    setDuePage(null);
    setMessage(pendingSubmission
      ? "上次加入结果尚未确认，请按原条件核对。"
      : pendingAdded
      ? "计划已经保存。曲库核对完成后，请重试显示已保存计划。"
      : "曲库资料可能已变化，请重新生成复习建议。");
  }, [resumeRevision, pendingAdded, pendingSubmission]);
  const generate = () =>
    void work(async () => {
      if (pendingSubmission) return;
      const request = ++requestRevision.current;
      draftRequestId.current = null;
      setRows([]);
      setChosen([]);
      setEdits({});
      setDuePage(null);
      setNote("");
      setMessage("");
      const [dueResult, weakResult] = await Promise.allSettled([
        api.command<SuggestionResult>({ type: "dueReview" }),
        api.command<SuggestionResult>({ type: "weakPractice" }),
      ]);
      if (request !== requestRevision.current) return;
      const due = dueResult.status === "fulfilled" ? dueResult.value : null;
      const weak = weakResult.status === "fulfilled" ? weakResult.value : null;
      const failure = [
        dueResult.status === "rejected" &&
          `到期复习读取失败：${String(dueResult.reason)}`,
        weakResult.status === "rejected" &&
          `薄弱项读取失败：${String(weakResult.reason)}`,
      ].filter((message): message is string => Boolean(message));
      if (!due && !weak) throw new Error(failure.join("；"));
      const suggestions = [
        ...(weak?.suggestions ?? []),
        ...(due?.suggestions ?? []),
      ];
      const balanced = [
        ...(weak?.suggestions.slice(0, 3) ?? []),
        ...(due?.suggestions.slice(0, 3) ?? []),
      ];
      const initial = [
        ...balanced,
        ...suggestions.filter((row) => !balanced.some((picked) => picked.id === row.id)),
      ].slice(0, 6);
      setRows(suggestions);
      setChosen(initial.map((row) => row.id));
      setNote([due?.note, weak?.note].filter(Boolean).join(" "));
      setDuePage(due ? {
        dueTotal: due.dueTotal ?? 0,
        eligibleTotal: due.eligibleTotal ?? 0,
        missingTotal: due.missingTotal ?? 0,
        revision: due.revision ?? "",
        nextOffset: due.nextOffset ?? due.suggestions.length,
        hasMore: due.hasMore === true,
      } : null);
      const resultMessage =
        suggestions.length
          ? ""
          : due?.dueTotal
            ? "待复习曲目的原条件或文件位置暂不能自动编排。请在曲库核对曲目，缺失文件先重新关联。"
            : "暂无足够的重复薄弱证据；完成几轮后再生成。";
      setMessage([resultMessage, ...failure].filter(Boolean).join(" "));
    });
  const loadMoreDue = () =>
    void work(async () => {
      if (pendingSubmission || !duePage?.hasMore) return;
      const request = ++requestRevision.current;
      try {
        const data = await api.command<SuggestionResult>({
          type: "dueReview",
          offset: duePage.nextOffset,
          expected: duePage.revision,
        });
        if (request !== requestRevision.current) return;
        setRows((current) => [
          ...current,
          ...data.suggestions.filter((row) => !current.some((item) => item.id === row.id)),
        ]);
        setDuePage({
          dueTotal: data.dueTotal ?? duePage.dueTotal,
          eligibleTotal: data.eligibleTotal ?? duePage.eligibleTotal,
          missingTotal: data.missingTotal ?? duePage.missingTotal,
          revision: data.revision ?? duePage.revision,
          nextOffset: data.nextOffset ?? duePage.nextOffset + data.suggestions.length,
          hasMore: data.hasMore === true,
        });
      } catch (cause) {
        if (request !== requestRevision.current) return;
        const detail = cause instanceof Error ? cause.message : String(cause);
        setRows((current) => current.filter((row) => row.kind !== "review"));
        setChosen((current) => current.filter((id) => !rows.some((row) => row.id === id && row.kind === "review")));
        setDuePage(null);
        setMessage(`到期复习分页读取失败：${detail}。旧到期建议已清除，请重新生成。`);
      }
    });
  const shownDue = rows.filter((row) => row.kind === "review").length;
  const editFor = (row: Suggestion) => edits[row.id] ?? suggestedEdit(row);
  const patchEdit = (row: Suggestion, change: Partial<ReviewEdit>) =>
    setEdits((current) => ({
      ...current,
      [row.id]: { ...(current[row.id] ?? suggestedEdit(row)), ...change },
    }));
  return (
    <details className="weak-practice-panel" open={pendingSubmission ? true : undefined}>
      <summary>从历史练习安排复习</summary>
      <button disabled={locked} onClick={generate}>
        生成复习建议
      </button>
      {pendingSubmission && (
        <div className="parameter-help" role="status">
          <p>“{pendingSubmission.name}”的 {pendingSubmission.ids.length} 个复习项目尚未确认。软件保留了当时的条件。改选前请先在计划列表核对已有项目。</p>
          <button type="button" disabled={disabled}
            onClick={() => void work(() => submit(pendingSubmission))}>
            按原条件核对上次加入
          </button>
          <button type="button" disabled={disabled}
            onClick={() => {
              clearSubmission(pendingSubmission);
              draftRequestId.current = null;
              setMessage("请核对计划列表后重新生成建议。");
            }}>
            已核对计划，重新选择
          </button>
        </div>
      )}
      <p className="parameter-help">
        预览后勾选加入
        {routineId
          ? day
            ? "当前日期安排"
            : "当前常用计划"
          : day
            ? "新计划与当前日期安排"
            : "新的常用计划"}
        ，不自动演奏。有两类建议时，系统默认各选最多三项；你可以逐项修改。
      </p>
      {duePage && duePage.dueTotal > 0 && (
        <div className="parameter-help">
          <p>
            曲库中 {duePage.dueTotal} 首曲目待复习；{duePage.eligibleTotal} 首可按原条件加入计划，当前已读取 {shownDue} 首。
            {duePage.missingTotal > 0 && ` ${duePage.missingTotal} 首文件缺失，请先重新关联。`}
            {duePage.dueTotal - duePage.eligibleTotal - duePage.missingTotal > 0 &&
              ` ${duePage.dueTotal - duePage.eligibleTotal - duePage.missingTotal} 首需核对练习条件。`}
          </p>
          {duePage.missingTotal > 0 && (
            <button type="button" disabled={locked} onClick={() => openLibrary("missing")}>
              查看缺失文件
            </button>
          )}
          {duePage.dueTotal - duePage.eligibleTotal - duePage.missingTotal > 0 && (
            <button type="button" disabled={locked} onClick={() => openLibrary("review")}>
              查看待复习曲目
            </button>
          )}
          {duePage.dueTotal > duePage.eligibleTotal &&
            <small>关闭曲库后会返回当前计划，保留已输入的内容；复习建议需要重新生成。</small>}
        </div>
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
            disabled={locked}
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
                  disabled={locked || (!chosen.includes(r.id) && chosen.length >= 60)}
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
                    {editFor(r).speedPercent}% · 连续{" "}
                    {editFor(r).passes} 轮达标，正确率 ≥ {editFor(r).accuracy}%
                    {r.item.goal.expression &&
                      `、${expressionGoalText(r.item.goal.expression)}`}
                    {editFor(r).onTime !== null
                      ? `、准时率 ≥ ${editFor(r).onTime}%`
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
              <details className="weak-practice-edit">
                <summary>调整加入计划的条件</summary>
                <div>
                  <label>练习速度
                    <input type="number" min={25} max={200} step={5} disabled={locked}
                      value={editFor(r).speedPercent}
                      onChange={(e) => patchEdit(r, { speedPercent: Number(e.target.value) })} />%
                  </label>
                  <label>达标轮数
                    <input type="number" min={1} max={100} disabled={locked}
                      value={editFor(r).passes}
                      onChange={(e) => patchEdit(r, { passes: Number(e.target.value) })} />
                  </label>
                  <label>正确率门槛
                    <input type="number" min={50} max={100} disabled={locked}
                      value={editFor(r).accuracy}
                      onChange={(e) => patchEdit(r, { accuracy: Number(e.target.value) })} />%
                  </label>
                  {r.item.settings.mode !== "wait" && (
                    <label><input type="checkbox" disabled={locked}
                      checked={editFor(r).onTime !== null}
                      onChange={(e) => patchEdit(r, { onTime: e.target.checked ? 80 : null })} />
                      准时率门槛
                      {editFor(r).onTime !== null && (
                        <><input type="number" min={50} max={100} disabled={locked}
                          value={editFor(r).onTime ?? 80}
                          onChange={(e) => patchEdit(r, { onTime: Number(e.target.value) })} />%</>
                      )}
                    </label>
                  )}
                  <button type="button" disabled={locked || !edits[r.id]}
                    onClick={() => setEdits((current) => {
                      const next = { ...current };
                      delete next[r.id];
                      return next;
                    })}>恢复建议值</button>
                </div>
              </details>
              </div>
            ))}
          </div>
          {duePage?.hasMore && (
            <button disabled={locked} onClick={loadMoreDue}>
              再读取 30 首到期曲目
            </button>
          )}
          <button
            disabled={
              locked || !chosen.length || (!routineId && !name.trim())
            }
            onClick={() =>
              void work(async () => {
                const selectedEdits = chosen.map((id) => {
                  const row = rows.find((item) => item.id === id);
                  if (!row) throw new Error("所选建议已变化，请重新生成");
                  const edit = editFor(row);
                  if (!Number.isFinite(edit.speedPercent) || edit.speedPercent < 25 || edit.speedPercent > 200 ||
                      !Number.isInteger(edit.passes) || edit.passes < 1 || edit.passes > 100 ||
                      !Number.isInteger(edit.accuracy) || edit.accuracy < 50 || edit.accuracy > 100 ||
                      (edit.onTime !== null && (!Number.isInteger(edit.onTime) || edit.onTime < 50 || edit.onTime > 100))) {
                    throw new Error(`“${row.title}”的练习条件超出范围，请核对速度、轮数和达标门槛`);
                  }
                  return { id, speed: edit.speedPercent / 100, passes: edit.passes,
                    accuracy: edit.accuracy, onTime: edit.onTime };
                });
                const submission: Submission = {
                  ids: [...chosen], edits: selectedEdits,
                  routineId, day, name,
                  requestId: routineId ? null : (draftRequestId.current ??= crypto.randomUUID()),
                };
                rememberSubmission(submission);
                await submit(submission);
              })
            }
          >
            加入勾选的复习项目
          </button>
        </>
      )}
      {pendingAdded && (
        <button
          disabled={disabled}
          onClick={() =>
            void work(async () => {
              try {
                await added(pendingAdded.id);
                setMessage(addedMessage(pendingAdded));
                setPendingAdded(null);
              } catch (cause) {
                const detail = cause instanceof Error ? cause.message : String(cause);
                setMessage(`计划已保存，但界面读取失败：${detail}。请重试显示计划。`);
              }
            })
          }
        >
          重试显示已保存计划
        </button>
      )}
      {message && <p role="status">{message}</p>}
    </details>
  );
}
