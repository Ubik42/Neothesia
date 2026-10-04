export type BeatPoint = { measure: number; beat: number };
export type ProblemCut = { start: BeatPoint; end: BeatPoint };
export type CutBounds = {
  first: BeatPoint;
  last: BeatPoint;
  measures: {
    number: number;
    maxBeat: number;
    numerator: number;
    denominator: number;
  }[];
  anchors: BeatPoint[];
};
export function validCut(v: unknown): v is ProblemCut {
  if (!v || typeof v !== "object") return false;
  const c = v as ProblemCut;
  return [c.start, c.end].every(
    (p) =>
      p &&
      Number.isSafeInteger(p.measure) &&
      p.measure >= 1 &&
      Number.isFinite(p.beat) &&
      p.beat >= 1,
  );
}
export function HistoryProblemCut({
  index,
  bounds,
  cut,
  description,
  disabled,
  change,
}: {
  index: number;
  bounds: CutBounds;
  cut?: ProblemCut | null;
  description?: string;
  disabled: boolean;
  change: (v: ProblemCut | null) => void;
}) {
  const field = (which: "start" | "end", label: string) => {
    const p = cut![which],
      m = bounds.measures.find((m) => m.number === p.measure);
    return (
      <fieldset>
        <legend>{label}</legend>
        <label>
          小节
          <select
            aria-label={`项目${index}裁剪${label}小节`}
            value={p.measure}
            disabled={disabled}
            onChange={(e) =>
              change({
                ...cut!,
                [which]: { measure: Number(e.target.value), beat: 1 },
              })
            }
          >
            {bounds.measures.map((m) => (
              <option key={m.number} value={m.number}>
                {m.number} · {m.numerator}/{m.denominator}
              </option>
            ))}
          </select>
        </label>
        <label>
          拍位
          <input
            aria-label={`项目${index}裁剪${label}拍位`}
            type="number"
            min={1}
            max={m?.maxBeat ?? 1}
            step="any"
            value={p.beat}
            disabled={disabled}
            onChange={(e) =>
              change({
                ...cut!,
                [which]: { ...p, beat: Number(e.target.value) },
              })
            }
          />
        </label>
      </fieldset>
    );
  };
  return (
    <div className="history-problem-cut">
      <label>
        <input
          type="checkbox"
          aria-label={`项目${index}裁剪到拍内范围`}
          checked={!!cut}
          disabled={disabled}
          onChange={(e) =>
            change(
              e.target.checked
                ? { start: { ...bounds.first }, end: { ...bounds.last } }
                : null,
            )
          }
        />
        裁剪到拍内范围
      </label>
      {cut && (
        <>
          <div>
            {field("start", "起点")}
            {field("end", "终点")}
          </div>
          <p>
            所选问题起音：
            {bounds.anchors
              .map(
                (p) => `第 ${p.measure} 小节第 ${Number(p.beat.toFixed(3))} 拍`,
              )
              .join("、")}
            。终点起音不计入，全部所选问题须留在范围内。
          </p>
          {description ? (
            <p className="problem-cut-confirmed">已核对：{description}</p>
          ) : (
            <p>拍位已修改，保存前请核对拍内范围。</p>
          )}
        </>
      )}
    </div>
  );
}
