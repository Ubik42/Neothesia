import { useEffect, useState } from "react";
import { api, clock, type EngineCommand, type Snapshot } from "./api";

export function LoopControls({
  state,
  action,
}: {
  state: Snapshot;
  action: (command: EngineCommand) => void;
}) {
  const [start, setStart] = useState(0);
  const [end, setEnd] = useState(state.duration);
  useEffect(() => {
    setStart(state.passage?.start ?? 0);
    setEnd(state.passage?.end ?? state.duration);
  }, [state.duration, state.passage?.start, state.passage?.end]);
  return (
    <div className="loop-controls">
      <strong>分段循环</strong>
      <label>
        起点{" "}
        <input
          aria-label="循环起点（秒）"
          type="number"
          min={0}
          max={state.duration}
          step="0.1"
          value={start}
          onChange={(e) => setStart(Number(e.target.value))}
        />
      </label>
      <button
        disabled={!state.duration}
        onClick={() => setStart(Math.round(state.position * 10) / 10)}
      >
        当前设为起点
      </button>
      <label>
        终点{" "}
        <input
          aria-label="循环终点（秒）"
          type="number"
          min={0}
          max={state.duration}
          step="0.1"
          value={end}
          onChange={(e) => setEnd(Number(e.target.value))}
        />
      </label>
      <button
        disabled={!state.duration}
        onClick={() =>
          setEnd(Math.min(state.duration, Math.round(state.position * 10) / 10))
        }
      >
        当前设为终点
      </button>
      <span className="loop-unit">秒</span>
      <button
        className={state.passage ? "loop-active" : ""}
        disabled={!state.duration}
        onClick={() =>
          action({ type: "loop", enabled: !state.passage, start, end })
        }
      >
        {state.passage ? "关闭循环" : "开始循环"}
      </button>
      {state.passage && (
        <span aria-live="polite">
          已完成 {state.repetitions} 轮 · {clock(state.passage.start)}–
          {clock(state.passage.end)}
        </span>
      )}
    </div>
  );
}
