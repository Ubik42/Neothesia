import { useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { practiceDayKey, practiceDuration, practiceIntervalDays, practiceWeek } from "./practiceTime";

type CalendarData = {
  intervals: { from: number; to: number; runningMs: number }[];
  saves?: { contentId: string; saveId: string; savedAt: number }[];
  earliestIntervalAt?: number | null;
  legacySessions: number;
  untrackedMs: number;
};

function weekStarts() {
  const monday = new Date(practiceWeek().from);
  return Array.from({ length: 12 }, (_, index) => {
    const date = new Date(monday);
    date.setDate(monday.getDate() - (11 - index) * 7);
    return date;
  });
}

export function PracticeTrend({
  refresh,
  disabled,
  openMonth,
}: {
  refresh: number;
  disabled: boolean;
  openMonth: (date: Date) => void;
}) {
  const starts = useMemo(weekStarts, [refresh]);
  const from = starts[0].getTime();
  const end = new Date(starts[11]);
  end.setDate(end.getDate() + 7);
  const to = end.getTime();
  const [data, setData] = useState<CalendarData | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");
    void api.command<CalendarData>({ type: "practiceCalendar", from, to })
      .then((result) => { if (active) setData(result); })
      .catch((cause) => { if (active) setError(cause instanceof Error ? cause.message : String(cause)); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [from, to, refresh]);
  const days = practiceIntervalDays(
    (data?.intervals ?? []).map((interval) => ({ start: interval.from, end: interval.to, runningMs: interval.runningMs })),
    from,
    to,
  );
  const weeks = starts.map((start) => {
    let ms = 0;
    let activeDays = 0;
    for (let day = 0; day < 7; day++) {
      const date = new Date(start);
      date.setDate(start.getDate() + day);
      const amount = days.get(practiceDayKey(date)) ?? 0;
      ms += amount;
      if (amount > 0) activeDays++;
    }
    const next = new Date(start);
    next.setDate(start.getDate() + 7);
    const saves = new Set((data?.saves ?? [])
      .filter((record) => record.savedAt >= start.getTime() && record.savedAt < next.getTime())
      .map((record) => `${record.contentId}:${record.saveId}`)).size;
    return { start, ms, activeDays, saves };
  });
  const maximum = Math.max(1, ...weeks.map((week) => week.ms));
  return (
    <section className="practice-trend" aria-label="最近十二周已保存练习">
      <p className="parameter-help">每根柱子是一周已保存的实际运行用时。选择某周可跳到该月的练习日历。</p>
      {error && <p role="alert">读取周趋势失败：{error}</p>}
      {loading && <p role="status">正在读取最近十二周…</p>}
      {!loading && data && <>
        <div className="practice-trend-weeks" role="group" aria-label="按周查看练习用时">
          {weeks.map((week) => {
            const label = `${week.start.getMonth() + 1}月${week.start.getDate()}日`;
            return <button
              key={week.start.getTime()}
              type="button"
              disabled={disabled}
              aria-label={`${label}起一周，已保存运行${practiceDuration(week.ms)}，练习${week.activeDays}天，保存${week.saves}次；打开该月日历`}
              onClick={() => openMonth(week.start)}
            >
              <span className="practice-trend-bar" style={{ height: `${Math.max(3, week.ms / maximum * 100)}%` }} aria-hidden="true" />
              <span className="practice-trend-date">{label}</span>
              <span className="practice-trend-time">{week.ms ? practiceDuration(week.ms) : "—"}</span>
            </button>;
          })}
        </div>
        <p className="parameter-help">这十二周共练习 {weeks.reduce((sum, week) => sum + week.activeDays, 0)} 天。旧记录和没有实际运行区间的用时不进入柱状图{data.legacySessions || data.untrackedMs ? "；这些资料仍保留在历史记录中。" : "。"}</p>
      </>}
    </section>
  );
}
