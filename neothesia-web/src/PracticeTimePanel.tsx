import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { practiceDuration, practiceWeek } from "./practiceTime";
type Totals = {
  legacySessions: number;
  legacyTimedSessions: number;
  legacyTimedMs: number;
  periodLegacySessions: number;
  untrackedMs: number;
  earliestIntervalAt?: number | null;
  coverage: string;
  totalMs: number;
  periodMs: number;
  timedSessions: number;
  unknownSessions: number;
  periodUnknownSessions: number;
  retainedSessions: number;
  from: number;
  to: number;
};
export function PracticeTimePanel({
  contentId,
  targetMinutes,
}: {
  contentId: string;
  targetMinutes: number | null;
}) {
  const [totals, setTotals] = useState<Totals | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const epoch = useRef(0);
  const load = async () => {
    const token = ++epoch.current;
    setBusy(true);
    setError("");
    try {
      const result = (await api.command({
        type: "practiceTime",
        content_id: contentId,
        ...practiceWeek(),
      })) as Totals;
      if (token === epoch.current) setTotals(result);
    } catch (e) {
      if (token === epoch.current)
        setError(String(e instanceof Error ? e.message : e));
    } finally {
      if (token === epoch.current) setBusy(false);
    }
  };
  useEffect(() => {
    setTotals(null);
    void load();
    return () => {
      epoch.current++;
    };
  }, [contentId]);
  const target =
    targetMinutes && targetMinutes > 0 ? targetMinutes * 60000 : null;
  return (
    <section
      className="practice-time-panel"
      aria-label="已保存练习用时"
      aria-busy={busy}
    >
      <div className="practice-time-heading">
        <strong>已保存练习用时</strong>
        <button type="button" disabled={busy} onClick={() => void load()}>
          刷新用时
        </button>
      </div>
      {error ? (
        <p role="alert">{error}</p>
      ) : totals ? (
        <>
          <p>
            本周：{practiceDuration(totals.periodMs)}
            {target && ` / 计划 ${targetMinutes} 分钟`}
          </p>
          {target && (
            <progress
              aria-label="本周已保存练习用时进度"
              max={target}
              value={Math.min(target, totals.periodMs)}
            />
          )}
          <p>
            计时账本累计：{practiceDuration(totals.totalMs)} ·{" "}
            {totals.timedSessions} 次保存
          </p>
          {target && (
            <p>
              {totals.periodMs >= target
                ? "本周已保存运行用时达到计划。"
                : `本周距离计划还差 ${practiceDuration(target - totals.periodMs)}。`}{" "}
              学习阶段由你确认。
            </p>
          )}
          {totals.legacySessions > 0 && (
            <p>
              另有 {totals.legacySessions} 次旧记录未进入计时账本（本周保存{" "}
              {totals.periodLegacySessions} 次）。其中{" "}
              {totals.legacyTimedSessions} 次保留旧用时{" "}
              {practiceDuration(totals.legacyTimedMs)}，{totals.unknownSessions}{" "}
              次没有用时。
            </p>
          )}
          {totals.untrackedMs > 0 && (
            <p>
              软件另有 {practiceDuration(totals.untrackedMs)}{" "}
              未能归入实际运行日期。周目标仅使用有日期区间的账本用时。
            </p>
          )}
          {totals.earliestIntervalAt != null && (
            <p>
              本曲计时账本最早记录：
              {new Date(totals.earliestIntervalAt).toLocaleDateString("zh-CN")}
              。
            </p>
          )}
        </>
      ) : (
        <p>正在读取用时…</p>
      )}
      <details className="parameter-help">
        <summary>统计口径</summary>
        <p>
          软件累计本曲目已保存的运行时间。计时账本独立保存，历史成绩每首最多保留最近
          200 次。
          运行用时包含等音等待；软件会扣除暂停、预备拍、聆听和指法示范。未保存或被重置的练习不计入账本。
          本周从本机周一零点开始，到下周一零点结束。跨日练习按实际运行区间拆分。旧资料的用时单独列出。用时达标后，软件仍由你确认学习阶段。
        </p>
      </details>
    </section>
  );
}
