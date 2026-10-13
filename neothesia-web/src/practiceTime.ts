export function practiceDuration(ms: number) {
  if (ms > 0 && ms < 1000) return `${Math.round(ms)} 毫秒`;
  if (ms >= 1000 && ms < 60000) return `${(ms / 1000).toFixed(1)} 秒`;
  const seconds = Math.floor(ms / 1000),
    minutes = Math.floor(seconds / 60),
    hours = Math.floor(minutes / 60);
  return hours
    ? `${hours}时 ${minutes % 60}分 ${seconds % 60}秒`
    : `${minutes}分 ${seconds % 60}秒`;
}
export function practiceWeek(now = new Date()) {
  const start = new Date(now);
  start.setHours(0, 0, 0, 0);
  start.setDate(start.getDate() - ((start.getDay() + 6) % 7));
  const end = new Date(start);
  end.setDate(end.getDate() + 7);
  return { from: start.getTime(), to: end.getTime() };
}

export function practiceDayKey(date: Date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}
export function practiceDayRange(date: Date) {
  const start = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  const end = new Date(date.getFullYear(), date.getMonth(), date.getDate() + 1);
  return { from: start.getTime(), to: end.getTime() };
}
/** Split recorded UTC intervals at local midnights, including daylight-saving days. */
export function practiceIntervalDays(
  intervals: { start: number; end: number; runningMs: number }[],
  from: number,
  to: number,
) {
  const result = new Map<string, number>();
  for (const interval of intervals) {
    let cursor = Math.max(from, interval.start);
    const end = Math.min(to, interval.end);
    if (!Number.isFinite(cursor) || !Number.isFinite(end) || cursor >= end)
      continue;
    while (cursor < end) {
      const day = new Date(cursor),
        boundary = Math.min(end, practiceDayRange(day).to);
      if (boundary <= cursor) break;
      const key = practiceDayKey(day);
      const duration = interval.end - interval.start;
      const elapsed =
        Math.floor(
          ((boundary - interval.start) / duration) * interval.runningMs,
        ) -
        Math.floor(((cursor - interval.start) / duration) * interval.runningMs);
      result.set(key, (result.get(key) ?? 0) + elapsed);
      cursor = boundary;
    }
  }
  return result;
}
