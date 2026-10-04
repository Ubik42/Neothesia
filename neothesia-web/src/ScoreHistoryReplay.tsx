import { useState } from "react";
import type { HistoryScoreReview } from "./historyScoreFocus";
import { useHistoryReplay } from "./useHistoryReplay";
import { HistoryReplayTransport } from "./HistoryReplayTransport";
export function ScoreHistoryReplay({
  review,
  enabled,
  cursor,
  activity,
}: {
  review: HistoryScoreReview;
  enabled: boolean;
  cursor: (position: number | null) => void;
  activity: (active: boolean) => void;
}) {
  const [source, setSource] = useState("actual"),
    [rate, setRate] = useState(0.5);
  const replay = useHistoryReplay(
    review.contentId,
    review.historyId,
    enabled,
    cursor,
    activity,
  );
  const attached = replay.phase !== "idle",
    playing = replay.phase === "running" || replay.phase === "paused";
  return (
    <div aria-label="谱面历史声音回放">
      <div className="score-review-replay">
        <label>
          回放{" "}
          <select
            aria-label="谱面回放来源"
            value={source}
            disabled={attached || replay.busy}
            onChange={(e) => setSource(e.target.value)}
          >
            <option value="actual">实际演奏</option>
            <option value="reference">当轮参考音</option>
          </select>
        </label>
        <label>
          速度{" "}
          <select
            aria-label="谱面回放速度"
            value={rate}
            disabled={replay.busy}
            onChange={(e) => {
              const value = Number(e.target.value);
              setRate(value);
              if (attached) void replay.control("rate", value);
            }}
          >
            {[0.25, 0.5, 0.75, 1].map((n) => (
              <option key={n} value={n}>
                {n * 100}%
              </option>
            ))}
          </select>
        </label>
        <button
          disabled={!enabled || replay.busy || playing}
          onClick={() => void replay.play(source, rate)}
        >
          随谱回放
        </button>
        <button
          disabled={!attached && !replay.busy}
          onClick={() => void replay.stop()}
        >
          停止回放
        </button>
      </div>
      <HistoryReplayTransport
        replay={replay}
        prefix="谱面"
        references={review.notes}
        measures={review.measures}
      />
      {replay.error && <span role="alert">{replay.error}</span>}
    </div>
  );
}
