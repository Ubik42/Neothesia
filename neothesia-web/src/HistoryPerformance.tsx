import { useEffect, useMemo, useRef, useState } from "react";
import { useHistoryReplay } from "./useHistoryReplay";
import type { HistoryMeasureSpan } from "./historyScoreFocus";
import { HistoryReplayTransport } from "./HistoryReplayTransport";
import { api } from "./api";
type Input = {
  at: number;
  judgedAt: number;
  songTime: number;
  bytes: [number, number, number];
  synthetic: boolean;
};
type Ref = {
  at: number;
  end: number;
  pitch: number;
  velocity: number;
  measure: number;
  songTime: number;
  track: number;
};
type Decision = {
  reference: number | null;
  input: number | null;
  kind: "onTime" | "early" | "late" | "missed" | "wrong";
  offsetMs: number | null;
  measure: number | null;
};
const gradeName = (kind: string) =>
  ({
    onTime: "准时",
    early: "偏早",
    late: "偏晚",
    missed: "漏音",
    pending: "未计分",
    approximate: "近似对照",
  })[kind] ?? kind;
type Archive = {
  measures?: HistoryMeasureSpan[] | null;
  judgments?: Decision[] | null;
  duration: number;
  inputs: Input[];
  references: Ref[];
  referencePedals: Input[];
  truncated: boolean;
};
type Actual = {
  input: number;
  at: number;
  end: number;
  judgedAt: number;
  pitch: number;
  velocity: number;
};
const pitch = (n: number) =>
  `${["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"][n % 12]}${Math.floor(n / 12) - 1}`;
export function HistoryPerformance({
  contentId,
  id,
  available,
  practice,
  review,
  blocked,
}: {
  contentId: string;
  id: string;
  available: boolean;
  practice: (
    reference: number,
    before: number,
    after: number,
    showScore?: boolean,
  ) => Promise<void>;
  blocked: boolean;
  review: () => Promise<void>;
}) {
  const [data, setData] = useState<Archive | null>(null),
    [error, setError] = useState(""),
    [loadBusy, setBusy] = useState(false),
    [rate, setRate] = useState(0.5),
    [source, setSource] = useState("actual"),
    [measure, setMeasure] = useState(0),
    [before, setBefore] = useState(1),
    [after, setAfter] = useState(1),
    [gradeFilter, setGradeFilter] = useState("all");
  const alive = useRef(true);
  const replay = useHistoryReplay(contentId, id, !blocked);
  const active = replay.phase !== "idle",
    position = replay.position,
    busy = loadBusy || replay.busy;
  const stop = replay.stop;
  const play = () => replay.play(source, rate);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  useEffect(() => {
    setData(null);
    setError("");
    setMeasure(0);
  }, [contentId, id]);
  async function load() {
    setBusy(true);
    setError("");
    try {
      const v = await api.command<Archive>({
        type: "historyPerformance",
        content_id: contentId,
        id,
      });
      if (alive.current) setData(v);
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      if (alive.current) setBusy(false);
    }
  }
  const actual = useMemo(() => {
    const notes: Actual[] = [],
      held = new Map<number, Actual>();
    for (const [input, e] of (data?.inputs ?? []).entries()) {
      const [kind, n, v] = e.bytes;
      if (kind === 144 && v > 0) {
        const old = held.get(n);
        if (old) old.end = e.at;
        const note = {
          input,
          at: e.at,
          end: data?.duration ?? e.at,
          judgedAt: e.judgedAt,
          pitch: n,
          velocity: v,
        };
        notes.push(note);
        held.set(n, note);
      } else if (kind === 128 || kind === 144) {
        const note = held.get(n);
        if (note) {
          note.end = Math.max(note.at, e.at);
          held.delete(n);
        }
      }
    }
    return notes;
  }, [data]);
  const refs =
    data?.references.filter((n) => !measure || n.measure === measure) ?? [];
  const start = measure && refs.length ? Math.min(...refs.map((n) => n.at)) : 0;
  const end =
    measure && refs.length
      ? Math.max(...refs.map((n) => n.end))
      : (data?.duration ?? 1);
  const seen = actual.filter((n) => n.end >= start - 0.5 && n.at <= end + 0.5);
  const exact = data?.judgments != null;
  const referenceIndexes = useMemo(
    () => new Map((data?.references ?? []).map((r, i) => [r, i])),
    [data],
  );
  const decisions = useMemo(
    () =>
      new Map(
        (data?.judgments ?? [])
          .filter((d) => d.reference != null)
          .map((d) => [d.reference!, d]),
      ),
    [data],
  );
  const inputDecisions = useMemo(
    () =>
      new Map(
        (data?.judgments ?? [])
          .filter((d) => d.input != null)
          .map((d) => [d.input!, d]),
      ),
    [data],
  );
  const pairs = useMemo(() => {
    if (data?.judgments != null) {
      const inputs = new Map(actual.map((n) => [n.input, n]));
      return data.references.map((r, index) => {
        const decision = decisions.get(index);
        return {
          reference: r,
          index,
          actual:
            decision?.input != null ? inputs.get(decision.input) : undefined,
          offset: decision?.offsetMs ?? null,
          grade: decision?.kind ?? "pending",
        };
      });
    }

    const grouped = new Map<number, { note: Actual; used: boolean }[]>();
    for (const n of actual) {
      const g = grouped.get(n.pitch) ?? [];
      g.push({ note: n, used: false });
      grouped.set(n.pitch, g);
    }
    for (const group of grouped.values())
      group.sort((a, b) => a.note.judgedAt - b.note.judgedAt);
    return (data?.references ?? []).map((r, index) => {
      const group = grouped.get(r.pitch) ?? [];
      let lo = 0,
        hi = group.length;
      while (lo < hi) {
        const m = (lo + hi) >> 1;
        if (group[m].note.judgedAt < r.at) lo = m + 1;
        else hi = m;
      }
      const candidates = group
        .slice(Math.max(0, lo - 4), lo + 5)
        .filter((n) => !n.used && Math.abs(n.note.judgedAt - r.at) <= 0.5)
        .sort(
          (a, b) =>
            Math.abs(a.note.judgedAt - r.at) - Math.abs(b.note.judgedAt - r.at),
        );
      const selected = candidates[0];
      if (selected) selected.used = true;
      return {
        reference: r,
        grade: "approximate",
        index,
        actual: selected?.note,
        offset: selected
          ? Math.round((selected.note.judgedAt - r.at) * 1000)
          : null,
      };
    });
  }, [actual, data, decisions]);
  const rows = pairs.filter(
    (p) =>
      (!measure || p.reference.measure === measure) &&
      (!exact ||
        gradeFilter === "all" ||
        (gradeFilter === "problems"
          ? ["early", "late", "missed"].includes(p.grade)
          : p.grade === gradeFilter)),
  );
  const wrong = (data?.judgments ?? []).filter(
    (d) => d.kind === "wrong" && (!measure || d.measure === measure),
  );
  const pitches = [
    ...new Set([...refs.map((n) => n.pitch), ...seen.map((n) => n.pitch)]),
  ].sort((a, b) => b - a);
  const width = Math.min(12000, Math.max(800, (end - start + 0.5) * 120));
  const x = (t: number) =>
    70 +
    (Math.max(0, t - start) / Math.max(0.1, end - start + 0.5)) * (width - 90);
  const y = (p: number) => 26 + pitches.indexOf(p) * 30;
  return (
    <details
      className="history-performance"
      onToggle={(e) => {
        if (!e.currentTarget.open) void stop();
      }}
    >
      <summary>演奏回放与起音对照</summary>
      {!available ? (
        <p>这条记录没有逐次按键数据。新保存的练习记录会保留演奏回放。</p>
      ) : (
        <>
          <button disabled={busy || active} onClick={() => void load()}>
            {data ? "重新读取演奏" : "查看这次演奏"}
          </button>
          {(error || replay.error) && (
            <p role="alert">{error || replay.error}</p>
          )}
          {data && (
            <>
              <p>
                {exact
                  ? "逐音结果来自当轮计分器：准时、偏早、偏晚、漏音及错音均保留实际判定；匹配音直接关联当次按键。红色参考音为漏音，橙色为偏早/偏晚，灰色为尚未计分。原成绩保持。"
                  : "这次记录没有保存逐音判定，仍按同音前后 0.5 秒就近对照；近似起音和未找到不能代替原计分结果。原成绩保持。"}
              </p>
              {exact && (
                <p>
                  本轮判定 · 准时{" "}
                  {pairs.filter((p) => p.grade === "onTime").length} · 偏早{" "}
                  {pairs.filter((p) => p.grade === "early").length} · 偏晚{" "}
                  {pairs.filter((p) => p.grade === "late").length} · 漏音{" "}
                  {pairs.filter((p) => p.grade === "missed").length} · 错音{" "}
                  {data?.judgments?.filter((d) => d.kind === "wrong").length ??
                    0}{" "}
                  · 未计分 {pairs.filter((p) => p.grade === "pending").length}
                </p>
              )}
              <p className="muted">
                演奏回放随“个人成绩与演奏回放”一起备份，可迁移到其他机器。
              </p>
              {data.truncated && (
                <p>本轮达到回放容量上限，后续数据未完整保留。</p>
              )}
              <div className="history-replay-controls">
                <select
                  aria-label="历史回放来源"
                  disabled={active || busy}
                  value={source}
                  onChange={(e) => setSource(e.target.value)}
                >
                  <option value="actual">实际演奏</option>
                  <option value="reference">当轮参考音</option>
                </select>
                <select
                  aria-label="历史回放速度"
                  disabled={busy}
                  value={rate}
                  onChange={(e) => {
                    const value = Number(e.target.value);
                    setRate(value);
                    if (active) void replay.control("rate", value);
                  }}
                >
                  {[0.25, 0.5, 0.75, 1].map((v) => (
                    <option key={v} value={v}>
                      {v * 100}%
                    </option>
                  ))}
                </select>
                <button
                  disabled={
                    busy ||
                    blocked ||
                    replay.phase === "running" ||
                    replay.phase === "paused"
                  }
                  onClick={() => void play()}
                >
                  回放声音
                </button>
                <button
                  disabled={busy || blocked}
                  onClick={() => void stop().then(review)}
                >
                  整轮谱面批阅
                </button>
                <button disabled={!active} onClick={() => void stop()}>
                  停止回放
                </button>
                <HistoryReplayTransport
                  replay={replay}
                  prefix="历史"
                  references={data.references}
                  measures={data.measures}
                />
                {exact && (
                  <select
                    aria-label="历史逐音判定筛选"
                    value={gradeFilter}
                    onChange={(e) => setGradeFilter(e.target.value)}
                  >
                    {[
                      ["all", "全部判定"],
                      ["problems", "偏早、偏晚与漏音"],
                      ["missed", "漏音"],
                      ["early", "偏早"],
                      ["late", "偏晚"],
                      ["onTime", "准时"],
                      ["pending", "未计分"],
                    ].map(([value, label]) => (
                      <option key={value} value={value}>
                        {label}
                      </option>
                    ))}
                  </select>
                )}
                <select
                  aria-label="演奏对照小节"
                  value={measure}
                  onChange={(e) => setMeasure(Number(e.target.value))}
                >
                  <option value={0}>全部演奏小节</option>
                  {[...new Set(data.references.map((n) => n.measure))]
                    .sort((a, b) => a - b)
                    .map((m) => (
                      <option key={m} value={m}>
                        第 {m} 小节
                      </option>
                    ))}
                </select>
              </div>
              <div className="history-performance-timeline">
                <svg
                  aria-label="参考与实际按键时间线"
                  width={width}
                  height={Math.max(90, pitches.length * 30 + 35)}
                >
                  {Array.from(
                    { length: Math.min(20, Math.ceil(end - start)) + 1 },
                    (_, i) => {
                      const t =
                        start +
                        ((end - start) * i) /
                          Math.max(1, Math.min(20, Math.ceil(end - start)));
                      return (
                        <text key={`t${i}`} x={x(t)} y={14} fill="#939aa6">
                          {t.toFixed(1)}s
                        </text>
                      );
                    },
                  )}
                  {pitches.map((p) => (
                    <g key={p}>
                      <text x={4} y={y(p) + 10} fill="#cdd2dc">
                        {pitch(p)}
                      </text>
                      <line
                        x1={65}
                        x2={width}
                        y1={y(p) + 25}
                        y2={y(p) + 25}
                        stroke="#343d46"
                      />
                    </g>
                  ))}
                  {refs.map((n, i) => (
                    <rect
                      key={`r${i}`}
                      x={x(n.at)}
                      y={y(n.pitch)}
                      width={Math.max(3, x(n.end) - x(n.at))}
                      height={9}
                      fill={
                        !exact
                          ? "#a9df74"
                          : decisions.get(referenceIndexes.get(n) ?? -1)
                                ?.kind === "missed"
                            ? "#f28d8d"
                            : ["early", "late"].includes(
                                  decisions.get(referenceIndexes.get(n) ?? -1)
                                    ?.kind ?? "",
                                )
                              ? "#eeb96d"
                              : decisions.get(referenceIndexes.get(n) ?? -1)
                                    ?.kind === "onTime"
                                ? "#a9df74"
                                : "#87919b"
                      }
                    >
                      <title>
                        {pitch(n.pitch)} · 第 {n.measure} 小节 · 参考力度{" "}
                        {n.velocity}
                      </title>
                    </rect>
                  ))}
                  {seen.map((n, i) => (
                    <rect
                      key={`a${i}`}
                      x={x(n.at)}
                      y={y(n.pitch) + 12}
                      width={Math.max(3, x(n.end) - x(n.at))}
                      height={9}
                      fill={
                        exact && inputDecisions.get(n.input)?.kind === "wrong"
                          ? "#f28d8d"
                          : "#70b7e8"
                      }
                    >
                      <title>
                        {pitch(n.pitch)} · 实际力度 {n.velocity}
                      </title>
                    </rect>
                  ))}
                  {active && position >= start && position <= end && (
                    <line
                      x1={x(position)}
                      x2={x(position)}
                      y1={0}
                      y2={pitches.length * 30 + 30}
                      stroke="#f5c15c"
                    />
                  )}
                </svg>
              </div>
              <div className="history-performance-controls">
                <label>
                  前面{" "}
                  <select
                    aria-label="重练前置小节"
                    value={before}
                    disabled={blocked}
                    onChange={(e) => setBefore(Number(e.target.value))}
                  >
                    {[0, 1, 2, 4, 8].map((n) => (
                      <option key={n} value={n}>
                        {n} 小节
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  后面{" "}
                  <select
                    aria-label="重练后置小节"
                    value={after}
                    disabled={blocked}
                    onChange={(e) => setAfter(Number(e.target.value))}
                  >
                    {[0, 1, 2, 4, 8].map((n) => (
                      <option key={n} value={n}>
                        {n} 小节
                      </option>
                    ))}
                  </select>
                </label>
                <button
                  disabled={busy || blocked || !measure}
                  onClick={() => {
                    const i = pairs.find(
                      (p) => p.reference.measure === measure,
                    )?.index;
                    if (i !== undefined)
                      void stop().then(() => practice(i, before, after));
                  }}
                >
                  重练所选小节
                </button>
              </div>
              <p>
                按当轮声部、速度和输入校准重练，可带前后小节连起来练。完整演奏/背谱改为连续练习，自适应速度关闭；准备好选段后按“开始练习”。
              </p>
              <div className="history-performance-table">
                <table>
                  <thead>
                    <tr>
                      <th>小节 / 音</th>
                      <th>判定</th>
                      <th>起音差值</th>
                      <th>力度对照</th>
                      <th>持音对照</th>
                      <th>练习</th>
                    </tr>
                  </thead>
                  <tbody>
                    {rows.slice(0, 200).map((p, i) => (
                      <tr key={i}>
                        <td>
                          第 {p.reference.measure} 小节 ·{" "}
                          {pitch(p.reference.pitch)}
                        </td>
                        <td>{gradeName(p.grade)}</td>
                        <td>
                          {p.offset === null
                            ? exact
                              ? "—"
                              : "附近未找到同音起音"
                            : `${p.offset > 0 ? "晚" : p.offset < 0 ? "早" : "一致"} ${Math.abs(p.offset)} ms`}
                        </td>
                        <td>
                          {p.actual
                            ? `${p.actual.velocity} / ${p.reference.velocity}`
                            : "—"}
                        </td>
                        <td>
                          {p.actual
                            ? `${(p.actual.end - p.actual.at).toFixed(2)} / ${(p.reference.end - p.reference.at).toFixed(2)} 秒`
                            : "—"}
                        </td>
                        <td>
                          <button
                            disabled={busy || blocked}
                            aria-label={`重练演奏第 ${p.reference.measure} 小节参考音 ${p.index + 1}`}
                            onClick={() =>
                              void stop().then(() =>
                                practice(p.index, before, after),
                              )
                            }
                          >
                            重练这里
                          </button>
                          <button
                            disabled={busy || blocked}
                            aria-label={`谱面定位演奏第 ${p.reference.measure} 小节参考音 ${p.index + 1}`}
                            onClick={() =>
                              void stop().then(() =>
                                practice(p.index, before, after, true),
                              )
                            }
                          >
                            在乐谱中重练
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {!rows.length && <p>该筛选范围没有对应参考音。</p>}
                {rows.length > 200 && (
                  <p>本页只显示前 200 个参考音，选择具体小节查看其余位置。</p>
                )}
              </div>
              {!!wrong.length && (
                <div className="history-wrong-inputs">
                  <h4>错音按键 · {wrong.length} 次</h4>
                  <p>
                    下列起音被当轮计分器判为错音。小节表示当时参考位置，不代表该按键匹配了那个参考音。
                  </p>
                  <table>
                    <thead>
                      <tr>
                        <th>音</th>
                        <th>实际起音</th>
                        <th>计分起音</th>
                        <th>当时参考小节</th>
                        <th>力度</th>
                      </tr>
                    </thead>
                    <tbody>
                      {wrong.slice(0, 200).map((d, i) => {
                        const n =
                          d.input != null ? data.inputs[d.input] : undefined;
                        return (
                          <tr key={i}>
                            <td>{n ? pitch(n.bytes[1]) : "—"}</td>
                            <td>{n?.at.toFixed(3)} 秒</td>
                            <td>{n?.judgedAt.toFixed(3)} 秒</td>
                            <td>
                              {d.measure
                                ? `第 ${d.measure} 小节`
                                : "尚无参考位置"}
                            </td>
                            <td>{n?.bytes[2]}</td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                  {wrong.length > 200 && (
                    <p>显示前 200 次，选择具体小节查看。</p>
                  )}
                </div>
              )}
              <p>
                实际踏板动作：
                {data.inputs
                  .filter(
                    (e) =>
                      e.bytes[0] === 176 && e.bytes[1] === 64 && !e.synthetic,
                  )
                  .map(
                    (e) =>
                      `${e.at.toFixed(2)} 秒 ${e.bytes[2] >= 64 ? "踩下" : "松开"}`,
                  )
                  .slice(0, 80)
                  .join("；") || "无实际踏板动作"}
              </p>
              <p>
                参考踏板动作：
                {(data.referencePedals ?? [])
                  .slice(0, 80)
                  .map(
                    (e) =>
                      `${e.at.toFixed(2)} 秒 ${e.bytes[2] >= 64 ? "踩下" : "松开"}`,
                  )
                  .join("；") || "本轮参考没有踏板动作"}
              </p>
            </>
          )}
        </>
      )}
    </details>
  );
}
