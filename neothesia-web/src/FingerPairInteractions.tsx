import { useState } from "react";
import { pitchName } from "./api";
export interface PairInteraction {
  kind: "sameKeyHandoff" | "crossedRange";
  measure: number;
  fromMeasure: number;
  leftTrack: number;
  leftIndex: number;
  rightTrack: number;
  rightIndex: number;
  leftPitch: number;
  rightPitch: number;
  at: number;
  end: number;
  overlapMs: number;
  leftStartedBeforeRange: boolean;
  rightStartedBeforeRange: boolean;
}
export interface PairInteractionAnalysis {
  items: PairInteraction[];
  sameKey: number;
  crossedRange: number;
  truncated: boolean;
}
export const interactionInstruction = (item: PairInteraction) =>
  item.kind === "sameKeyHandoff"
    ? `两手 ${pitchName(item.leftPitch)} 同键持音重叠 ${item.overlapMs} 毫秒：确认实际由哪只手持音，练习先离键再由另一只手接替；需要修改分手时先完成分手审阅。`
    : `左手 ${pitchName(item.leftPitch)} 与右手 ${pitchName(item.rightPitch)} 同时演奏 ${item.overlapMs} 毫秒：左手音高高于右手，先慢练两手路径，核对分手、手位及交叉顺序。`;
export function FingerPairInteractions({
  analysis,
  focus,
  disabled,
}: {
  analysis: PairInteractionAnalysis;
  focus: (item: PairInteraction) => void;
  disabled: boolean;
}) {
  const [filter, setFilter] = useState("all"),
    [page, setPage] = useState(0);
  const rows = analysis.items.filter(
    (item) => filter === "all" || item.kind === filter,
  );
  return (
    <section
      className="finger-pair-interactions"
      aria-label="双手交接与音域审阅"
    >
      <h3>两手交接与音域</h3>
      <p>
        同键持音重叠 {analysis.sameKey} 处 · 左手高音与右手低音同时演奏{" "}
        {analysis.crossedRange} 处
        {analysis.truncated ? "（结果有截断，请缩小范围继续检查）" : ""}
      </p>
      <p>
        按 MIDI
        按键起止检查，离键后踏板延音不占用琴键。音域交叠位置用于核对分手、手位和两手路径，身体动作仍需人工判断。
      </p>
      {!analysis.items.length ? (
        <p>本次范围没有发现这两类交接位置。</p>
      ) : (
        <>
          <label>
            查看位置
            <select
              aria-label="双手交接筛选"
              value={filter}
              onChange={(e) => {
                setFilter(e.target.value);
                setPage(0);
              }}
            >
              <option value="all">全部</option>
              <option value="sameKeyHandoff">同键交接</option>
              <option value="crossedRange">音域交叠</option>
            </select>
          </label>
          {rows.slice(page * 10, page * 10 + 10).map((item, i) => (
            <div className="finger-interaction-row" key={page * 10 + i}>
              <strong>
                第{" "}
                {item.fromMeasure === item.measure
                  ? item.measure
                  : `${item.fromMeasure}→${item.measure}`}{" "}
                小节 ·{" "}
                {item.kind === "sameKeyHandoff" ? "同键交接" : "音域交叠"}
              </strong>
              <p>{interactionInstruction(item)}</p>
              <small>
                左手音轨 {item.leftTrack + 1} / 音符 {item.leftIndex + 1}
                ；右手音轨 {item.rightTrack + 1} / 音符 {item.rightIndex + 1}
              </small>
              {(item.leftStartedBeforeRange ||
                item.rightStartedBeforeRange) && (
                <p>包含选段开始前的持音；修改该音时请扩展推荐小节范围。</p>
              )}
              <button disabled={disabled} onClick={() => focus(item)}>
                查看两手指法位置
              </button>
            </div>
          ))}
          {!rows.length && <p>本次范围没有此类位置。</p>}
          <div className="finger-pair-selection">
            <button disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
              上一组交接
            </button>
            <span>
              {page + 1} / {Math.max(1, Math.ceil(rows.length / 10))}
            </span>
            <button
              disabled={(page + 1) * 10 >= rows.length}
              onClick={() => setPage((p) => p + 1)}
            >
              下一组交接
            </button>
          </div>
        </>
      )}
    </section>
  );
}
