import { useState } from "react";
import { pitchName } from "./api";
import "./fingerCoverageReview.css";

export type FingerCoverageNote = {
  track: number;
  index: number;
  measure: number;
  pitch: number;
  contextIndex: number;
  target: boolean;
  before: FingerMarkedNeighbor | null;
  after: FingerMarkedNeighbor | null;
};

type FingerMarkedNeighbor = {
  track: number;
  index: number;
  measure: number;
  pitch: number;
  finger: number;
  contextIndex: number;
};

export function FingerCoverageReview({
  notes,
  total,
  truncated,
  trackName,
  locate,
  adopt,
  disabled,
}: {
  notes: FingerCoverageNote[];
  total: number;
  truncated: boolean;
  trackName: (track: number) => string;
  locate: (note: FingerCoverageNote) => void;
  adopt: (note: FingerCoverageNote) => void;
  disabled: boolean;
}) {
  const [page, setPage] = useState(0);
  const pages = Math.max(1, Math.ceil(notes.length / 12));
  const visible = notes.slice(page * 12, (page + 1) * 12);
  const neighbor = (n: FingerMarkedNeighbor | null) =>
    n
      ? `${trackName(n.track)} · 第 ${n.measure} 小节 · ${pitchName(n.pitch)}（${n.finger} 指）`
      : "无已标记音";
  return (
    <section className="finger-coverage" aria-label="未标记指法衔接清单">
      <strong>未标记位置与衔接 · {total} 个</strong>
      <p>此处列出保存后仍没有指法的音符。相邻指法只读取已保存或本次勾选的手指。</p>
      {visible.map((n) => (
        <div className="finger-coverage-row" key={`${n.track}:${n.index}`}>
          <div>
            <strong>
              {n.target ? "选段" : "同手上下文"} · {trackName(n.track)} · 第 {n.measure} 小节 · {pitchName(n.pitch)}
            </strong>
            <small>音符 {n.index + 1} · 前一个已标记：{neighbor(n.before)} · 后一个已标记：{neighbor(n.after)}</small>
          </div>
          <div className="finger-coverage-actions">
            <button type="button" disabled={disabled} onClick={() => locate(n)}>定位音符</button>
            {n.target && (
              <button type="button" disabled={disabled} onClick={() => adopt(n)}>采用这条建议</button>
            )}
          </div>
        </div>
      ))}
      {truncated && <p>清单显示前 256 个位置。请缩小选段，继续检查其余位置。</p>}
      {pages > 1 && (
        <div className="finger-coverage-pages">
          <button type="button" disabled={page === 0} onClick={() => setPage(page - 1)}>上一页</button>
          <span>{page + 1} / {pages}</span>
          <button type="button" disabled={page + 1 >= pages} onClick={() => setPage(page + 1)}>下一页</button>
        </div>
      )}
    </section>
  );
}
