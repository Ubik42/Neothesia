import { useEffect, useState } from "react";
import "./scoreRoutePreview.css";

export type ScoreRouteVisit = {
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
};

export type ScoreRouteEvidence = {
  route: { complete: boolean; visits: ScoreRouteVisit[] } | null;
  diagnostics: string[];
  warnings: string[];
  readiness: string;
  coverage: number;
  versionId?: string;
  contentId?: string;
  fingerprint?: string;
};

const readinessName: Record<string, string> = {
  Ready: "可以跟随演奏",
  Review: "对应关系需要人工核对",
  Poor: "谱音与演奏对应很少",
  Partial: "部分音符可以跟随",
  Blocked: "演奏跟随不可用",
};

const time = (seconds: number) => {
  if (!Number.isFinite(seconds)) return "时间未知";
  const tenths = Math.round(Math.max(0, seconds) * 10);
  return `${Math.floor(tenths / 600)}:${String(Math.floor((tenths % 600) / 10)).padStart(2, "0")}.${tenths % 10}`;
};
const beat = (value: number) =>
  Number.isFinite(value) ? Number(value.toFixed(2)).toString() : "?";

export function ScoreRoutePreview({
  evidence,
  title = "演奏顺序",
  initialVisible = 80,
}: {
  evidence: ScoreRouteEvidence | null | undefined;
  title?: string;
  initialVisible?: number;
}) {
  const [visible, setVisible] = useState(initialVisible);
  useEffect(() => setVisible(initialVisible), [evidence, initialVisible]);
  if (!evidence) {
    return (
      <section className="score-route-preview" aria-label={title}>
        <h4>{title}</h4>
        <p role="status">当前谱面没有可核对的演奏顺序。</p>
      </section>
    );
  }
  const route = evidence.route;
  const visits = Array.isArray(route?.visits) ? route.visits : [];
  const complete = route?.complete === true;
  const totals = new Map<number, number>();
  for (const visit of visits)
    totals.set(visit.measure, (totals.get(visit.measure) ?? 0) + 1);
  const occurrences = new Map<number, number>();
  return (
    <section className="score-route-preview" aria-label={title}>
      <header>
        <h4>{title}</h4>
        <span
          className={complete ? "score-route-ready" : "score-route-warning"}
        >
          {complete ? "顺序完整" : "顺序尚未确认"}
        </span>
      </header>
      <p>
        {visits.length} 次小节经过 · 与演奏音符对应 {evidence.coverage}% ·{" "}
        {readinessName[evidence.readiness] ?? evidence.readiness}
      </p>
      {!complete && (
        <p className="score-route-warning" role="status">
          当前预览不能证明整首曲目的演奏顺序。请查看下方问题并校对谱面与演奏对应，再使用这条路线。
        </p>
      )}
      {visits.length ? (
        <ol className="score-route-visits">
          {visits.slice(0, visible).map((visit, index) => {
            const occurrence = (occurrences.get(visit.measure) ?? 0) + 1;
            occurrences.set(visit.measure, occurrence);
            return (
              <li key={`${visit.ordinal}-${index}`}>
                <span className="score-route-position">{index + 1}</span>
                <strong>第 {visit.number || visit.measure + 1} 小节</strong>
                {(totals.get(visit.measure) ?? 0) > 1 && (
                  <span>本小节第 {occurrence} 次</span>
                )}
                {visit.pass > 1 && <span>第 {visit.pass} 遍</span>}
                <span
                  className={
                    visit.partialStart || visit.partialEnd
                      ? "score-route-partial"
                      : undefined
                  }
                >
                  第 {beat(visit.startBeat)}–{beat(visit.endBeat)} 拍
                  {visit.partialStart || visit.partialEnd ? " · 局部" : ""}
                </span>
                <time>
                  {time(visit.start)}–{time(visit.end)}
                </time>
              </li>
            );
          })}
        </ol>
      ) : (
        <p className="parameter-help" role="status">
          这份谱面没有可显示的小节经过。请检查谱面内容和下方问题。
        </p>
      )}
      {visible < visits.length && (
        <button onClick={() => setVisible((count) => count + 80)}>
          继续显示后续小节 · 剩余 {visits.length - visible} 次
        </button>
      )}
      {evidence.diagnostics.map((item, index) => (
        <p className="score-route-diagnostic" key={`diagnostic-${index}`}>
          需校对：{item}
        </p>
      ))}
      {evidence.warnings.map((item, index) => (
        <p className="score-route-diagnostic" key={`warning-${index}`}>
          谱面提示：{item}
        </p>
      ))}
    </section>
  );
}
