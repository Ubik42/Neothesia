import { useEffect, useState } from "react";
import { api } from "./api";
import {
  practiceDayKey,
  practiceDayRange,
  practiceDuration,
  practiceIntervalDays,
} from "./practiceTime";
import "./practiceCalendar.css";

type CalendarData = {
  intervals: {
    contentId: string;
    title: string;
    saveId: string;
    from: number;
    to: number;
    runningMs: number;
  }[];
  saves?: { contentId: string; saveId: string; savedAt: number }[];
  earliestIntervalAt?: number | null;
  totalMs: number;
  legacySessions: number;
  legacyTimedMs: number;
  untrackedMs: number;
};
export function PracticeCalendar({
  selected,
  choose,
  disabled,
}: {
  selected: number | null;
  choose: (range: { from: number; to: number } | null) => void;
  disabled: boolean;
}) {
  const [month, setMonth] = useState(() => {
    const d = new Date();
    return new Date(d.getFullYear(), d.getMonth(), 1);
  });
  const [data, setData] = useState<CalendarData | null>(null),
    [error, setError] = useState(""),
    [loading, setLoading] = useState(false),
    [refresh, setRefresh] = useState(0);
  const from = month.getTime(),
    to = new Date(month.getFullYear(), month.getMonth() + 1, 1).getTime();
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");
    setData(null);
    void api
      .command<CalendarData>({ type: "practiceCalendar", from, to })
      .then((v) => {
        if (active) setData(v);
      })
      .catch((e) => {
        if (active) setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [from, to, refresh]);
  const durations = practiceIntervalDays(
    (data?.intervals ?? []).map((i) => ({
      start: i.from,
      end: i.to,
      runningMs: i.runningMs,
    })),
    from,
    to,
  );
  const saves = new Map<string, Set<string>>();
  for (const s of data?.saves ?? []) {
    const key = practiceDayKey(new Date(s.savedAt));
    if (!saves.has(key)) saves.set(key, new Set());
    saves.get(key)!.add(`${s.contentId}:${s.saveId}`);
  }
  const days = new Date(month.getFullYear(), month.getMonth() + 1, 0).getDate(),
    blanks = (month.getDay() + 6) % 7;
  const selectedKey =
    selected === null ? null : practiceDayKey(new Date(selected));
  return (
    <details className="practice-calendar" open>
      <summary>练习日历</summary>
      <div className="practice-calendar-toolbar">
        <button
          disabled={disabled || loading}
          onClick={() =>
            setMonth(new Date(month.getFullYear(), month.getMonth() - 1, 1))
          }
        >
          上个月
        </button>
        <strong>
          {month.getFullYear()} 年 {month.getMonth() + 1} 月
        </strong>
        <button
          disabled={disabled || loading}
          onClick={() =>
            setMonth(new Date(month.getFullYear(), month.getMonth() + 1, 1))
          }
        >
          下个月
        </button>
        <button
          disabled={disabled || loading}
          onClick={() => {
            const now = new Date();
            setMonth(new Date(now.getFullYear(), now.getMonth(), 1));
          }}
        >
          本月
        </button>
        <button
          disabled={disabled || loading}
          onClick={() => setRefresh((n) => n + 1)}
        >
          刷新日历
        </button>
        {selected !== null && (
          <button disabled={disabled} onClick={() => choose(null)}>
            清除日期筛选
          </button>
        )}
      </div>
      {error && <p role="alert">{error}</p>}
      <div
        className="practice-calendar-grid"
        role="group"
        aria-label="按本机日期查看练习"
        aria-busy={loading}
      >
        {["一", "二", "三", "四", "五", "六", "日"].map((d) => (
          <span className="practice-calendar-weekday" key={d}>
            周{d}
          </span>
        ))}
        {Array.from({ length: blanks }, (_, i) => (
          <span key={`blank${i}`} aria-hidden="true" />
        ))}
        {Array.from({ length: days }, (_, i) => {
          const date = new Date(month.getFullYear(), month.getMonth(), i + 1),
            key = practiceDayKey(date),
            ms = durations.get(key) ?? 0,
            count = saves.get(key)?.size ?? 0;
          return (
            <button
              key={key}
              className={ms > 0 ? "has-practice" : ""}
              aria-label={`${key}，已保存运行 ${practiceDuration(ms)}，${count} 次保存`}
              aria-pressed={selectedKey === key}
              disabled={disabled || loading || !!error}
              onClick={() => choose(practiceDayRange(date))}
            >
              <b>{i + 1}</b>
              <span>{ms > 0 ? practiceDuration(ms) : "—"}</span>
              <small>{count ? `${count} 次保存` : ""}</small>
            </button>
          );
        })}
      </div>
      <p role="status">
        {loading
          ? "正在读取练习日历…"
          : data
            ? `本月已保存运行 ${practiceDuration(data.totalMs)} · ${data.saves?.length ?? 0} 次保存`
            : "日历尚未读取。"}
        {selected !== null
          ? ` 当前历史日期：${selectedKey}。`
          : " 点击日期可查看下方历史记录。"}
      </p>
      <p className="parameter-help">
        日历按本机时区划分日期。运行时间按实际区间跨日拆分，保存次数和下方历史按保存日期归属。下方的曲名、模式和手别筛选仅影响历史列表。历史每曲最多保留最近
        200 次成绩，日历保存次数可能多于可查看的成绩。
      </p>
      {data?.earliestIntervalAt != null && (
        <p className="parameter-help">
          计时账本最早记录：
          {new Date(data.earliestIntervalAt).toLocaleDateString("zh-CN")}
          。此前用时仅在已有旧资料中保留。
        </p>
      )}
      {data && (data.legacySessions > 0 || data.untrackedMs > 0) && (
        <p className="parameter-help">
          旧资料有 {data.legacySessions} 次保存未进入分段计时账本，已知旧用时{" "}
          {practiceDuration(data.legacyTimedMs)}；另有{" "}
          {practiceDuration(data.untrackedMs)}{" "}
          未能归入实际运行日期。日历仅显示有日期区间的已保存用时。
        </p>
      )}
    </details>
  );
}
