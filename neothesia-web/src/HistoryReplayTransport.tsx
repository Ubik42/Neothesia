import { useEffect, useState } from "react";
import { HistoryReplayRange, type ReplayReference } from "./HistoryReplayRange";
import type { HistoryMeasureSpan } from "./historyScoreFocus";
import type { HistoryReplay } from "./useHistoryReplay";
export function HistoryReplayTransport({
  replay,
  prefix,
  references = [],
  measures,
}: {
  replay: HistoryReplay;
  prefix: string;
  references?: ReplayReference[];
  measures?: HistoryMeasureSpan[] | null;
}) {
  const [target, setTarget] = useState(0),
    [dragging, setDragging] = useState(false);
  useEffect(() => {
    if (!dragging) setTarget(replay.position);
  }, [replay.position, dragging]);
  const attached = replay.phase !== "idle";
  const seek = () => {
    setDragging(false);
    if (attached)
      void replay.control(
        "seek",
        Math.max(
          replay.loop?.start ?? 0,
          Math.min(
            replay.loop
              ? Math.max(replay.loop.start, replay.loop.end - 0.001)
              : replay.end,
            target,
          ),
        ),
      );
  };
  return (
    <div className="history-transport">
      <button
        disabled={replay.busy || !attached || replay.phase === "finished"}
        onClick={() =>
          void replay.control(replay.phase === "paused" ? "resume" : "pause")
        }
      >
        {replay.phase === "paused" ? "继续回放" : "暂停回放"}
      </button>
      <input
        type="range"
        aria-label={`${prefix}回放进度`}
        min={replay.loop?.start ?? 0}
        max={replay.loop?.end ?? (replay.end || 1)}
        step={0.01}
        value={Math.min(target, replay.end || 1)}
        disabled={!attached || replay.busy}
        onChange={(e) => {
          setDragging(true);
          setTarget(Number(e.target.value));
        }}
        onPointerUp={seek}
        onKeyUp={seek}
      />
      <input
        type="number"
        aria-label={`${prefix}回放定位秒数`}
        min={replay.loop?.start ?? 0}
        max={replay.loop?.end ?? (replay.end || 1)}
        step={0.1}
        value={Number(target.toFixed(2))}
        disabled={!attached || replay.busy}
        onChange={(e) => {
          setDragging(true);
          setTarget(Number(e.target.value) || 0);
        }}
      />
      <button disabled={!attached || replay.busy} onClick={seek}>
        定位回放
      </button>
      <HistoryReplayRange
        replay={replay}
        prefix={prefix}
        references={references}
        measures={measures}
      />
      <span>
        {replay.position.toFixed(1)} / {replay.end.toFixed(1)} 秒 ·{" "}
        {
          {
            idle: "已停止",
            running: "正在回放",
            paused: "已暂停",
            finished: "回放结束",
          }[replay.phase]
        }
      </span>
    </div>
  );
}
