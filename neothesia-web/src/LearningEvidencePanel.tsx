import { useEffect, useState } from "react";
import { api } from "./api";
import "./learningEvidencePanel.css";

export type LearningEvidence = {
  contentId: string;
  generatedAt: number;
  week: {
    from: number;
    to: number;
    runningMs: number;
    targetMinutes: number | null;
    met: boolean;
    reasons: string[];
  };
  wholeSong: {
    recent: {
      sessionId: string;
      recordedAt: number;
      scope: string | null;
      accuracyPercent: number | null;
      effectiveBpm: number | null;
      meetsTargetBpm: boolean;
      readable: boolean;
    }[];
    comparableCount: number;
    targetBpmMet: boolean;
    reasons: string[];
  };
  routines: {
    items: {
      routineId: string;
      day: string;
      itemId: string;
      title: string;
      targetKind: string;
      completed: boolean;
      skipped: boolean;
      sessionIds: string[];
      readableSessionIds: string[];
    }[];
    completedCount: number;
    reasons: string[];
  };
  historicalRefs?: { sessionId: string; readable: boolean }[];
  reasons: string[];
};

type Detail = {
  id?: string;
  recordedAt?: number;
  title?: string;
  score?: { matched_notes: number; wrong_notes: number; missed_notes: number };
  tempo?: number | null;
  context?: { routine?: { routine_name: string; item_title: string; day: string } };
};

const minutes = (ms: number) => `${Math.round(ms / 60000)} 分钟`;
const date = (value: number) =>
  new Date(value < 1e11 ? value * 1000 : value).toLocaleString("zh-CN");

export function LearningEvidencePanel({
  contentId,
  targetBpm,
  targetMinutes,
  draftChanged,
}: {
  contentId: string;
  targetBpm: number | null;
  targetMinutes: number | null;
  draftChanged: boolean;
}) {
  const [evidence, setEvidence] = useState<LearningEvidence | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [detail, setDetail] = useState<Detail | null>(null);
  const [detailError, setDetailError] = useState("");
  const [selected, setSelected] = useState("");
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    let live = true;
    setLoading(true);
    setError("");
    setEvidence(null);
    void api
      .command<LearningEvidence>({ type: "songLearningEvidence", content_id: contentId })
      .then((value) => {
        if (live) setEvidence(value);
      })
      .catch((reason) => {
        if (live) setError(String(reason));
      })
      .finally(() => {
        if (live) setLoading(false);
      });
    return () => {
      live = false;
    };
  }, [contentId, revision]);

  const showSession = async (id: string) => {
    setSelected(id);
    setDetail(null);
    setDetailError("");
    try {
      const value = await api.command<Detail>({
        type: "historyDetail",
        content_id: contentId,
        id,
      });
      setDetail(value);
    } catch (reason) {
      setDetailError(String(reason));
    }
  };

  return (
    <section className="learning-evidence" aria-label="曲目练习证据">
      <header>
        <div>
          <h3>练习证据</h3>
          <p>这里汇总实际练习记录。你仍需自己确认学习阶段。</p>
        </div>
        <button type="button" disabled={loading} onClick={() => setRevision((n) => n + 1)}>
          重新读取
        </button>
      </header>
      {loading && <p role="status">正在读取练习证据…</p>}
      {error && <p role="alert" className="dialog-error">{error}</p>}
      {draftChanged && (
        <p className="learning-evidence-notice">
          你修改了目标。下方证据仍按已保存的目标计算；保存后会按新目标重新记录。
        </p>
      )}
      {evidence && (
        <>
          <div className="learning-evidence-overview">
            <article>
              <span>本周实际练习</span>
              <strong>{minutes(evidence.week.runningMs)}</strong>
              <small>
                {evidence.week.targetMinutes == null
                  ? "尚未设定每周时长"
                  : `已保存目标 ${evidence.week.targetMinutes} 分钟 · ${evidence.week.met ? "已达到" : "尚未达到"}`}
              </small>
            </article>
            <article>
              <span>整曲速度</span>
              <strong>{targetBpm == null ? "未设置目标速度" : evidence.wholeSong.targetBpmMet ? "有达到目标速度的整曲记录" : "等待目标速度记录"}</strong>
              <small>{targetBpm == null ? "尚未设定目标 BPM" : `当前表单目标 ${targetBpm} BPM`}</small>
            </article>
            <article>
              <span>日期计划</span>
              <strong>{evidence.routines.completedCount} 项达标</strong>
              <small>局部选段只证明该段，不代表整曲掌握</small>
            </article>
          </div>
          {targetMinutes !== evidence.week.targetMinutes && !draftChanged && (
            <p className="learning-evidence-notice">练习证据与当前表单的每周目标不同。请重新读取或保存资料。</p>
          )}
          {!!evidence.reasons.length && (
            <div className="learning-evidence-reasons">
              {evidence.reasons.map((reason, index) => <p key={index}>{reason}</p>)}
            </div>
          )}
          {!!evidence.historicalRefs?.some((reference) => !reference.readable) && (
            <p className="learning-evidence-notice">
              有 {evidence.historicalRefs.filter((reference) => !reference.readable).length} 条阶段证据引用的原成绩已清理。阶段变更时保存的统计仍可查看，原成绩无法重新打开。
            </p>
          )}
          <details>
            <summary>查看整曲成绩 · {evidence.wholeSong.recent.length} 条</summary>
            {!!evidence.wholeSong.reasons.length && (
              <p className="parameter-help">{evidence.wholeSong.reasons.join("；")}</p>
            )}
            <ol className="learning-evidence-list">
              {evidence.wholeSong.recent.map((row) => (
                <li key={row.sessionId}>
                  <div>
                    <strong>{date(row.recordedAt)}</strong>
                    <span>
                      {row.accuracyPercent == null ? "正确率未记录" : `正确率 ${row.accuracyPercent}%`}
                      {row.effectiveBpm == null ? " · 有效速度未知" : ` · ${row.effectiveBpm} BPM`}
                      {row.meetsTargetBpm ? " · 达到目标速度" : ""}
                    </span>
                  </div>
                  <button type="button" disabled={!row.readable} onClick={() => void showSession(row.sessionId)}>
                    {row.readable ? "查看成绩" : "原成绩已移除"}
                  </button>
                </li>
              ))}
            </ol>
            {!evidence.wholeSong.recent.length && <p className="parameter-help">目前没有可核验的整曲成绩。</p>}
          </details>
          <details>
            <summary>查看日期计划 · {evidence.routines.items.length} 项</summary>
            {!!evidence.routines.reasons.length && (
              <p className="parameter-help">{evidence.routines.reasons.join("；")}</p>
            )}
            <ol className="learning-evidence-list">
              {evidence.routines.items.map((item) => (
                <li key={`${item.routineId}-${item.day}-${item.itemId}`}>
                  <div>
                    <strong>{item.title}</strong>
                    <span>{item.day} · {({ whole: "整曲", passage: "选段", ladder: "速度阶梯" } as Record<string, string>)[item.targetKind] ?? "练习项目"} · {item.completed ? "已达标" : item.skipped ? "已跳过" : "尚未达标"} · 可读取成绩 {item.readableSessionIds.length}/{item.sessionIds.length} 条</span>
                  </div>
                  {!!item.readableSessionIds.length && (
                    <button type="button" onClick={() => void showSession(item.readableSessionIds.at(-1)!)}>
                      查看关联成绩
                    </button>
                  )}
                </li>
              ))}
            </ol>
            {!evidence.routines.items.length && <p className="parameter-help">这首曲目还没有日期计划项目。</p>}
          </details>
          {selected && (
            <aside className="learning-evidence-detail" aria-label="关联练习成绩">
              <button type="button" onClick={() => { setSelected(""); setDetail(null); }}>收起成绩</button>
              {detailError && <p role="alert">{detailError}</p>}
              {!detail && !detailError && <p role="status">正在读取原成绩…</p>}
              {detail && (
                <p>
                  {detail.title ?? "这首曲目"} · {detail.recordedAt ? date(detail.recordedAt) : "日期未知"}
                  {detail.tempo != null ? ` · ${detail.tempo} BPM` : ""}
                  {detail.score ? ` · 正确 ${detail.score.matched_notes}、错音 ${detail.score.wrong_notes}、漏音 ${detail.score.missed_notes}` : ""}
                  {detail.context?.routine ? ` · ${detail.context.routine.day} ${detail.context.routine.routine_name} / ${detail.context.routine.item_title}` : ""}
                </p>
              )}
            </aside>
          )}
        </>
      )}
    </section>
  );
}
