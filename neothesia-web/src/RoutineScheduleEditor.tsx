import { useEffect, useState } from "react";
export interface RoutineSchedule {
  enabled: boolean;
  weekdays: number[];
  start: string;
  end: string | null;
}
const labels = ["一", "二", "三", "四", "五", "六", "日"];
export const scheduleText = (s: RoutineSchedule | undefined | null) =>
  !s
    ? "手动安排"
    : `${s.enabled ? "每周" : "已暂停 · 周"}${s.weekdays.map((d) => labels[d]).join("、")}`;
export function RoutineScheduleEditor({
  value,
  day,
  disabled,
  save,
}: {
  value?: RoutineSchedule | null;
  day: string;
  disabled: boolean;
  save: (s: RoutineSchedule | null) => Promise<void>;
}) {
  const [draft, setDraft] = useState<RoutineSchedule>({
    enabled: true,
    weekdays: [0, 1, 2, 3, 4],
    start: day,
    end: null,
  });
  useEffect(() => {
    setDraft(
      value ?? {
        enabled: true,
        weekdays: [0, 1, 2, 3, 4],
        start: day,
        end: null,
      },
    );
  }, [value, day]);
  const valid =
    draft.weekdays.length > 0 &&
    !!draft.start &&
    (!draft.end || draft.end >= draft.start);
  return (
    <details className="routine-schedule">
      <summary>周期安排 · {scheduleText(value)}</summary>
      <fieldset disabled={disabled}>
        <legend>每周练习日</legend>
        <div className="routine-weekdays">
          {labels.map((label, d) => (
            <label key={d}>
              <input
                type="checkbox"
                aria-label={`星期${label}`}
                checked={draft.weekdays.includes(d)}
                onChange={(e) =>
                  setDraft((s) => ({
                    ...s,
                    weekdays: e.target.checked
                      ? [...s.weekdays, d].sort()
                      : s.weekdays.filter((v) => v !== d),
                  }))
                }
              />
              {label}
            </label>
          ))}
        </div>
        <div className="routine-schedule-dates">
          <label>
            开始日期
            <input
              aria-label="周期开始日期"
              type="date"
              min="2000-01-01"
              max="2200-12-31"
              value={draft.start}
              onChange={(e) =>
                setDraft((s) => ({ ...s, start: e.target.value }))
              }
            />
          </label>
          <label>
            结束日期（可留空）
            <input
              aria-label="周期结束日期"
              type="date"
              min={draft.start}
              max="2200-12-31"
              value={draft.end ?? ""}
              onChange={(e) =>
                setDraft((s) => ({ ...s, end: e.target.value || null }))
              }
            />
          </label>
        </div>
        <label className="routine-schedule-enable">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={(e) =>
              setDraft((s) => ({ ...s, enabled: e.target.checked }))
            }
          />
          启用周期安排
        </label>
        <div className="routine-meta-actions">
          <button disabled={!valid} onClick={() => void save(draft)}>
            保存周期安排
          </button>
          {value && (
            <>
              <button
                onClick={() => void save({ ...value, enabled: !value.enabled })}
              >
                {value.enabled ? "暂停周期安排" : "恢复周期安排"}
              </button>
              <button onClick={() => void save(null)}>取消周期安排</button>
            </>
          )}
        </div>
      </fieldset>
      <p className="parameter-help">
        打开符合规则的日期时建立当天安排，使用届时的模板。已有安排及成绩保留；暂停或取消只影响尚未建立的日期。不会自动开始演奏或补建未查看的过去日期。
      </p>
    </details>
  );
}
