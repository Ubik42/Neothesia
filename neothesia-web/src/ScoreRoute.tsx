import { useMemo, useState } from "react";

export interface WrittenVisit {
  ordinal: number;
  measure: number;
  number: string;
  pass: number;
  start: number;
  end: number;
  startBeat: number;
  endBeat: number;
  partialStart: boolean;
  partialEnd: boolean;
}
export interface WrittenRoute {
  complete: boolean;
  visits: WrittenVisit[];
}
const beat = (value: number) => Number(value.toFixed(3)).toString();
const label = (visit: WrittenVisit) =>
  `原谱第 ${visit.number || visit.measure + 1} 小节${visit.partialStart ? `，第 ${beat(visit.startBeat)} 拍开始` : ""}${visit.partialEnd ? `，第 ${beat(visit.endBeat)} 拍前结束` : ""}`;

export function ScoreRoute({
  route,
  position,
  seek,
  disabled,
  loop,
  selectRange,
}: {
  route: WrittenRoute;
  position: number;
  seek: (position: number) => void;
  disabled: boolean;
  loop: (start: number, end: number) => Promise<void>;
  selectRange: (first: number, last: number) => void;
}) {
  let low = 0,
    high = route.visits.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    if (route.visits[middle].start <= position + 0.001) low = middle + 1;
    else high = middle;
  }
  const current = route.visits[Math.max(0, low - 1)];
  const [selected, setSelected] = useState<number | null>(null);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState("");
  const chosen =
    (selected === null ? current : route.visits[selected]) ?? current;
  const measures = useMemo(
    () => Array.from(new Map(route.visits.map((v) => [v.measure, v])).values()),
    [route],
  );
  const occurrences = useMemo(
    () => route.visits.filter((v) => v.measure === chosen?.measure),
    [route, chosen?.measure],
  );
  if (!current || !chosen) return null;
  const index = chosen.ordinal;
  const from = Math.max(0, index - 5);
  const visible = route.visits.slice(from, from + 12);
  const go = (v: WrittenVisit) => {
    setSelected(v.ordinal);
    seek(v.start);
  };
  const blocked = disabled || working || !route.complete;
  const startLoop = async () => {
    if (blocked) return;
    setWorking(true);
    setError("");
    try {
      await loop(chosen.start, chosen.end);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setWorking(false);
    }
  };
  return (
    <details className="score-route">
      <summary>
        演奏顺序 · {route.visits.length} 段 · 当前{label(current)} · 第{" "}
        {current.ordinal + 1} 段
      </summary>
      <div className="score-route-controls">
        <label>
          书面小节
          <select
            aria-label="原谱小节"
            value={chosen.measure}
            onChange={(e) => {
              const visits = route.visits.filter(
                (v) => v.measure === Number(e.target.value),
              );
              setSelected(
                (visits.find((v) => v.start >= position) ?? visits[0]).ordinal,
              );
            }}
          >
            {measures.map((v) => (
              <option key={v.measure} value={v.measure}>
                第 {v.number || v.measure + 1} 小节
              </option>
            ))}
          </select>
        </label>
        <label>
          演奏位置
          <select
            aria-label="原谱小节演奏位置"
            value={chosen.ordinal}
            onChange={(e) => setSelected(Number(e.target.value))}
          >
            {occurrences.map((v, i) => (
              <option key={v.ordinal} value={v.ordinal}>
                第 {i + 1} 次 · 全曲第 {v.ordinal + 1} 段
                {v.partialStart || v.partialEnd ? " · 部分小节" : ""}
              </option>
            ))}
          </select>
        </label>
        <button disabled={blocked} onClick={() => go(chosen)}>
          定位原谱小节
        </button>
        <button
          disabled={blocked || chosen.end - chosen.start < 0.1}
          onClick={() => void startLoop()}
        >
          循环这一段
        </button>
        <button
          disabled={blocked}
          onClick={() => selectRange(chosen.ordinal, chosen.ordinal)}
        >
          选段练习
        </button>
        <button
          disabled={disabled || working}
          onClick={() => setSelected(null)}
        >
          查看当前位置
        </button>
        {!route.complete && (
          <span role="status">演奏顺序需校对，暂不能按原谱定位</span>
        )}
      </div>
      {error && <p role="alert">{error}</p>}
      <nav className="score-route-sequence" aria-label="原谱演奏顺序">
        <button
          disabled={blocked || index === 0}
          onClick={() => go(route.visits[index - 1])}
        >
          上一段
        </button>
        {visible.map((v) => (
          <button
            key={v.ordinal}
            disabled={blocked}
            className={v.ordinal === current.ordinal ? "current" : ""}
            aria-current={v.ordinal === current.ordinal ? "step" : undefined}
            aria-label={`演奏第 ${v.ordinal + 1} 段，${label(v)}`}
            title={label(v)}
            onClick={() => go(v)}
          >
            <small>{v.ordinal + 1}</small>
            <span>
              {v.number || v.measure + 1}
              {v.partialStart || v.partialEnd ? "*" : ""}
            </span>
          </button>
        ))}
        <button
          disabled={blocked || index === route.visits.length - 1}
          onClick={() => go(route.visits[index + 1])}
        >
          下一段
        </button>
      </nav>
      <p>
        {label(chosen)}；相同书面小节的多次演奏可分别定位。带 *
        的位置只演奏该小节的一部分。
      </p>
    </details>
  );
}
