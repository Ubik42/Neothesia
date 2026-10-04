import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { practiceDuration, practiceWeek } from "./practiceTime";
type Totals = {
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
            保留记录合计：{practiceDuration(totals.totalMs)} ·{" "}
            {totals.timedSessions} 次有用时记录
          </p>
          {totals.unknownSessions > 0 && (
            <p>
              另有 {totals.unknownSessions} 次旧记录未计时（本周{" "}
              {totals.periodUnknownSessions} 次），不按曲长估算。
            </p>
          )}
        </>
      ) : (
        <p>正在读取用时…</p>
      )}
      <details className="parameter-help">
        <summary>统计口径</summary>
        <p>
          统计本曲目已保存结果，每首最多保留最近 200
          次。运行用时包含等音等待，不含暂停、预备拍、聆听和指法示范；未保存或被重置的练习不计。本周按本机周一至周日及保存时间归属，不代表持续实弹。阶段仍由你确认。
        </p>
      </details>
    </section>
  );
}
