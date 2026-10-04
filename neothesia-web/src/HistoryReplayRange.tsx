import { useEffect, useMemo, useState } from "react";
import type { HistoryMeasureSpan } from "./historyScoreFocus";
import type { HistoryReplay } from "./useHistoryReplay";
import { HistoryReplayClips } from "./HistoryReplayClips";
export type ReplayReference = {
  at: number;
  end: number;
  measure: number;
  songTime?: number;
  pitch?: number;
  track?: number;
};
export function HistoryReplayRange({
  replay,
  prefix,
  references,
  measures,
}: {
  replay: HistoryReplay;
  prefix: string;
  references: ReplayReference[];
  measures?: HistoryMeasureSpan[] | null;
}) {
  const exact = !!measures?.length;
  const spans = useMemo(() => {
    if (exact)
      return measures!.map((m) => ({
        ...m,
        start: m.at,
        partial:
          m.songStart > m.measureStart + 0.000001 ||
          m.songEnd < m.measureEnd - 0.000001,
      }));
    const ordered = [...references].sort((a, b) => a.at - b.at),
      counts = new Map<number, number>();
    const result: {
      measure: number;
      occurrence: number;
      start: number;
      end: number;
      partial?: boolean;
    }[] = [];
    let previous: ReplayReference | undefined;
    const seen = new Set<string>();
    for (const n of ordered) {
      const last = result.at(-1);
      const key =
        n.songTime == null
          ? ""
          : `${n.track}:${n.pitch}:${Math.round(n.songTime * 1000000)}`;
      const rewind =
        previous &&
        n.songTime != null &&
        previous.songTime != null &&
        (n.songTime < previous.songTime - 0.000001 ||
          (seen.has(key) && n.at > previous.at + 0.000001));
      previous = n;
      if (last?.measure === n.measure && !rewind) {
        seen.add(key);
        last.end = Math.max(last.end, n.end);
        continue;
      }
      seen.clear();
      seen.add(key);
      const occurrence = (counts.get(n.measure) ?? 0) + 1;
      counts.set(n.measure, occurrence);
      result.push({ measure: n.measure, occurrence, start: n.at, end: n.end });
    }
    return result;
  }, [references, measures]);
  const [measure, setMeasure] = useState(1),
    [occurrence, setOccurrence] = useState(1),
    [start, setStart] = useState(0),
    [end, setEnd] = useState(0),
    [error, setError] = useState("");
  useEffect(() => {
    setMeasure(spans[0]?.measure ?? 1);
    setOccurrence(1);
    setError("");
  }, [spans]);
  const selectedIndex = spans.findIndex(
      (n) => n.measure === measure && n.occurrence === occurrence,
    ),
    selected = spans[selectedIndex];
  const attached = replay.phase !== "idle",
    disabled = !attached || replay.busy;
  const pick = () => {
    if (!selected) {
      setError("这个演奏小节的所选次数没有保存可定位的范围。");
      return;
    }
    setStart(selected.start);
    setEnd(
      exact ? selected.end : (spans[selectedIndex + 1]?.start ?? replay.end),
    );
    setError("");
  };
  return (
    <details className="history-replay-range">
      <summary>
        小节定位与循环回放
        {replay.loop
          ? ` · ${replay.loop.start.toFixed(2)}–${replay.loop.end.toFixed(2)} 秒 · 已循环 ${replay.loopRounds} 次`
          : ""}
      </summary>
      <div className="history-range-controls">
        <label>
          演奏第{" "}
          <input
            type="number"
            min={1}
            step={1}
            aria-label={`${prefix}回放小节`}
            value={measure}
            onChange={(e) => {
              setMeasure(Math.max(1, Math.trunc(Number(e.target.value) || 1)));
              setError("");
            }}
          />{" "}
          小节
        </label>
        <label>
          第{" "}
          <input
            type="number"
            min={1}
            step={1}
            aria-label={`${prefix}回放小节次数`}
            value={occurrence}
            onChange={(e) =>
              setOccurrence(
                Math.max(1, Math.trunc(Number(e.target.value) || 1)),
              )
            }
          />{" "}
          次
        </label>
        <button
          disabled={disabled || !selected}
          onClick={() => void replay.control("seek", selected!.start)}
        >
          {exact ? "跳到小节起点" : "跳到参考起音"}
        </button>
        <button disabled={disabled || !selected} onClick={pick}>
          {exact ? "采用小节范围" : "采用小节参考范围"}
        </button>
      </div>
      <div className="history-range-controls">
        <label>
          起点{" "}
          <input
            type="number"
            min={0}
            max={replay.end}
            step={0.01}
            aria-label={`${prefix}回放循环起点`}
            value={Number(start.toFixed(3))}
            onChange={(e) => setStart(Number(e.target.value) || 0)}
          />{" "}
          秒
        </label>
        <button
          disabled={disabled}
          onClick={() => setStart(Number(replay.position.toFixed(3)))}
        >
          设为起点
        </button>
        <label>
          终点{" "}
          <input
            type="number"
            min={0}
            max={replay.end}
            step={0.01}
            aria-label={`${prefix}回放循环终点`}
            value={Number(end.toFixed(3))}
            onChange={(e) => setEnd(Number(e.target.value) || 0)}
          />{" "}
          秒
        </label>
        <button
          disabled={disabled}
          onClick={() => setEnd(Number(replay.position.toFixed(3)))}
        >
          设为终点
        </button>
        <button
          disabled={
            disabled || end - start < 0.1 || start < 0 || end > replay.end
          }
          onClick={() =>
            void replay.control("loop", start, { end, enabled: true })
          }
        >
          循环回放选段
        </button>
        <button
          disabled={disabled || !replay.loop}
          onClick={() =>
            void replay.control("loop", 0, { end: replay.end, enabled: false })
          }
        >
          关闭回放循环
        </button>
      </div>
      <p>
        {exact
          ? "按当轮保存的小节边界定位，休止小节也可选择；等音等待包含在该小节时长内。"
          : "旧记录没有小节边界，按首个保留参考起音定位，范围到下一小节首个参考起音。"}
        循环终点不播放，跨界持音在起点重新建立。
      </p>
      {selected?.partial && (
        <p>本次只演奏了所选小节的一部分，采用范围仅包含已演奏部分。</p>
      )}
      {!selected && <p>所选小节次数没有保存范围，仍可用回放进度设置范围。</p>}
      {error && <p role="alert">{error}</p>}
      <HistoryReplayClips
        key={`${replay.contentId}:${replay.historyId}`}
        replay={replay}
        start={start}
        end={end}
        pick={(a, b) => {
          setStart(a);
          setEnd(b);
          setError("");
        }}
      />
    </details>
  );
}
