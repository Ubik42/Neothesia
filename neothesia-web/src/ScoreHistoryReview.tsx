import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { HistoryProblemPlan } from "./HistoryProblemPlan";
import { ScoreHistoryReplay } from "./ScoreHistoryReplay";
import { pitchName } from "./api";
import type { ScoreMapping } from "./ScoreNoteEditor";
import {
  historyGradeName,
  type HistoryScoreReview,
  type HistoryReviewNote,
} from "./historyScoreFocus";
const noteKey = (n: { track: number; index: number | null }) =>
  `${n.track}:${n.index}`;
export function ScoreHistoryReview({
  review,
  mapping,
  root,
  page,
  markup,
  ready,
  blocked,
  selectPage,
  pauseFollow,
  practice,
  close,
  openPlan,
}: {
  review: HistoryScoreReview;
  mapping: ScoreMapping[];
  root: RefObject<HTMLDivElement | null>;
  page: number;
  markup: string;
  ready: boolean;
  blocked: boolean;
  selectPage: (page: number) => void;
  pauseFollow: () => void;
  practice: (reference: number, before: number, after: number) => Promise<void>;
  close: () => void;
  openPlan: (id: string, day: string | null) => void;
}) {
  const [occurrence, setOccurrence] = useState(1),
    [filter, setFilter] = useState("all"),
    [selected, setSelected] = useState<number | null>(null),
    [offset, setOffset] = useState(0),
    [before, setBefore] = useState(1),
    [after, setAfter] = useState(1),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [replaying, setReplaying] = useState(false),
    [replayPosition, setReplayPosition] = useState<number | null>(null);
  const [planDirty, setPlanDirty] = useState(false);
  const initialized = useRef(false),
    alive = useRef(true),
    working = useRef(false);
  useEffect(
    () => () => {
      alive.current = false;
    },
    [],
  );
  const groups = useMemo(() => {
    const result = new Map<string, ScoreMapping[]>();
    const seen = new Map<string, Set<string>>();
    for (const m of mapping) {
      const key = `${m.page}:${m.id}`,
        list = result.get(key) ?? [];
      const keys = seen.get(key) ?? new Set<string>();
      if (!keys.has(noteKey(m))) {
        list.push(m);
        keys.add(noteKey(m));
      }
      seen.set(key, keys);
      result.set(key, list);
    }
    for (const list of result.values())
      list.sort(
        (a, b) => a.start - b.start || a.track - b.track || a.index - b.index,
      );
    return result;
  }, [mapping]);
  const mappingByKey = useMemo(() => {
    const result = new Map<string, ScoreMapping | null>();
    for (const m of mapping) {
      const key = noteKey(m),
        old = result.get(key);
      if (!result.has(key)) result.set(key, m);
      else if (old && (old.page !== m.page || old.id !== m.id))
        result.set(key, null);
    }
    return result;
  }, [mapping]);
  const noteByKey = useMemo(() => {
    const result = new Map<string, HistoryReviewNote | null>();
    for (const n of review.notes) {
      if (n.index == null) continue;
      const key = noteKey(n);
      result.set(key, result.has(key) ? null : n);
    }
    return result;
  }, [review]);
  const timeline = useMemo(
    () =>
      [...review.notes].sort(
        (a, b) => a.at - b.at || a.reference - b.reference,
      ),
    [review],
  );
  const replayNotes = useMemo(() => {
    if (replayPosition == null) return [];
    let lo = 0,
      hi = timeline.length;
    while (lo < hi) {
      const mid = (lo + hi) >>> 1;
      if (timeline[mid].at <= replayPosition + 0.000001) lo = mid + 1;
      else hi = mid;
    }
    if (!lo) return [];
    const at = timeline[lo - 1].at;
    let start = lo - 1;
    while (start > 0 && Math.abs(timeline[start - 1].at - at) < 0.000001)
      start--;
    const notes = timeline.slice(start, lo);
    return review.measures?.length
      ? notes.filter((n) => n.end > replayPosition + 0.000001)
      : notes;
  }, [timeline, replayPosition]);
  const currentMeasure = useMemo(() => {
    const measures = review.measures;
    if (replayPosition == null || !measures?.length) return null;
    let low = 0,
      high = measures.length;
    while (low < high) {
      const mid = (low + high) >>> 1;
      if (measures[mid].at <= replayPosition + 0.000001) low = mid + 1;
      else high = mid;
    }
    const m = measures[low - 1];
    return m && m.end > replayPosition + 0.000001 ? m : null;
  }, [review.measures, replayPosition]);
  const replayReferences = new Set(replayNotes.map((n) => n.reference));
  const replayAnchor = replayNotes.find((n) => mappingByKey.get(noteKey(n)));
  const replayAnchorReference = replayAnchor?.reference;
  useEffect(() => {
    if (replayAnchor && ready) locate(replayAnchor);
  }, [replayAnchorReference, ready]);
  const chosen = useMemo(
    () =>
      [...groups.values()].flatMap((list) => {
        const m = list[occurrence - 1];
        if (
          !m ||
          !mappingByKey.get(noteKey(m)) ||
          list.some((v) => v !== m && Math.abs(v.start - m.start) < 0.000001)
        )
          return [];
        const n = noteByKey.get(noteKey(m));
        return n ? [{ m, n }] : [];
      }),
    [groups, noteByKey, occurrence, mappingByKey],
  );
  const maxOccurrence = [...groups.values()].reduce(
    (max, list) => Math.max(max, list.length),
    1,
  );
  const matches = (n: HistoryReviewNote) =>
    filter === "all" ||
    (filter === "problems"
      ? ["early", "late", "missed"].includes(n.kind)
      : n.kind === filter);
  const rows = chosen
    .filter(({ n }) => matches(n))
    .sort(
      (a, b) => a.n.songTime - b.n.songTime || a.n.reference - b.n.reference,
    );
  const problems = review.notes.filter(
    (n) =>
      ["early", "late", "missed"].includes(n.kind) &&
      !!mappingByKey.get(noteKey(n)),
  );
  const unmatched = review.notes.filter(
    (n) => n.index == null || !mappingByKey.get(noteKey(n)),
  ).length;
  const locate = (n: HistoryReviewNote) => {
    const target = mappingByKey.get(noteKey(n));
    if (!target || target.pitch !== n.pitch) {
      setError(
        "这个参考音尚无可靠唯一谱面位置，请先校对；仍可按历史小节重练。",
      );
      return;
    }
    const list = groups.get(`${target.page}:${target.id}`)!;
    setOccurrence(list.findIndex((m) => noteKey(m) === noteKey(n)) + 1);
    setSelected(n.reference);
    setOffset(0);
    pauseFollow();
    selectPage(target.page);
    setError("");
  };
  useEffect(() => {
    if (!initialized.current && ready && chosen.length) {
      initialized.current = true;
      pauseFollow();
      selectPage(chosen[0].m.page);
    }
  }, [ready, chosen]);
  useEffect(() => {
    const el = root.current;
    if (!el) return;
    el.querySelectorAll(".history-review-outline").forEach((n) => n.remove());
    el.querySelectorAll(".history-review-note").forEach((n) => {
      n.classList.remove("history-review-note");
      n.removeAttribute("data-review-grade");
      n.removeAttribute("data-review-reference");
    });
    if (!ready) return;
    const visible = new Map(
      chosen.map((row) => [`${row.m.page}:${row.m.id}`, row]),
    );
    for (const n of replayNotes) {
      const m = mappingByKey.get(noteKey(n));
      if (m && m.pitch === n.pitch) {
        const group = groups.get(`${m.page}:${m.id}`)!;
        if (
          !group.some((v) => v !== m && Math.abs(v.start - m.start) < 0.000001)
        )
          visible.set(`${m.page}:${m.id}`, { m, n });
      }
    }
    for (const { m, n } of visible.values()) {
      if (m.page !== page) continue;
      const target = el.querySelector<SVGGraphicsElement>(
        `[id="${CSS.escape(m.id)}"]`,
      );
      if (!target) continue;
      target.classList.add("history-review-note");
      target.setAttribute("data-review-grade", n.kind);
      target.setAttribute("data-review-reference", String(n.reference));
      const head =
          target.querySelector<SVGGraphicsElement>(".notehead") ?? target,
        svg = target.ownerSVGElement,
        ctm = svg?.getScreenCTM();
      if (!svg || !ctm) continue;
      const box = head.getBoundingClientRect(),
        inverse = ctm.inverse(),
        a = new DOMPoint(box.left - 6, box.top - 6).matrixTransform(inverse),
        b = new DOMPoint(box.right + 6, box.bottom + 6).matrixTransform(
          inverse,
        ),
        rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
      rect.setAttribute("class", "history-review-outline");
      rect.setAttribute("x", String(a.x));
      rect.setAttribute("y", String(a.y));
      rect.setAttribute("width", String(b.x - a.x));
      rect.setAttribute("height", String(b.y - a.y));
      rect.setAttribute("rx", String(3 / Math.hypot(ctm.a, ctm.b)));
      rect.style.fill = "none";
      rect.style.stroke =
        n.kind === "missed"
          ? "#bc4242"
          : ["early", "late"].includes(n.kind)
            ? "#b07912"
            : n.kind === "onTime"
              ? "#3e7f36"
              : n.kind === "approximate"
                ? "#397aaf"
                : "#87919b";
      rect.style.strokeWidth = String(
        (selected === n.reference ? 3 : 1.8) / Math.hypot(ctm.a, ctm.b),
      );
      rect.style.pointerEvents = "none";
      const title = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "title",
      );
      title.textContent = `各书面音第 ${occurrence} 次 · 演奏第 ${n.measure} 小节 · ${pitchName(n.pitch)} · ${historyGradeName(n.kind)}`;
      rect.append(title);
      svg.append(rect);
      if (replayReferences.has(n.reference)) {
        const cursor = rect.cloneNode(false) as SVGRectElement;
        cursor.setAttribute(
          "class",
          "history-review-outline history-replay-cursor",
        );
        cursor.setAttribute("data-replay-reference", String(n.reference));
        cursor.style.stroke = "#2479ba";
        cursor.style.strokeDasharray = `${5 / Math.hypot(ctm.a, ctm.b)} ${3 / Math.hypot(ctm.a, ctm.b)}`;
        cursor.style.strokeWidth = String(4 / Math.hypot(ctm.a, ctm.b));
        svg.append(cursor);
      }
      if (n.reference === selected)
        requestAnimationFrame(() => {
          if (alive.current)
            target.scrollIntoView({ block: "center", inline: "center" });
        });
    }
    return () => {
      el.querySelectorAll(".history-review-outline").forEach((n) => n.remove());
      el.querySelectorAll(".history-review-note").forEach((n) => {
        n.classList.remove("history-review-note");
        n.removeAttribute("data-review-grade");
        n.removeAttribute("data-review-reference");
      });
    };
  }, [
    chosen,
    page,
    markup,
    ready,
    selected,
    occurrence,
    replayAnchorReference,
    replaying,
  ]);
  async function train(reference: number) {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    try {
      await practice(reference, before, after);
    } catch (e) {
      if (alive.current) setError(e instanceof Error ? e.message : String(e));
    } finally {
      working.current = false;
      if (alive.current) setBusy(false);
    }
  }
  const nextProblem = (direction: number) => {
    if (!problems.length) return;
    const current = problems.findIndex((n) => n.reference === selected);
    const next = (current + direction + problems.length) % problems.length;
    locate(
      problems[current < 0 ? (direction < 0 ? problems.length - 1 : 0) : next],
    );
  };
  return (
    <section className="score-history-review" aria-label="整轮历史谱面批阅">
      <div className="score-review-heading">
        <strong>{review.title} · 历史谱面批阅</strong>
        <span>{new Date(review.recordedAt).toLocaleString("zh-CN")}</span>
        <button
          disabled={busy}
          onClick={() => {
            if (
              !planDirty ||
              window.confirm(
                "问题段落编排尚未保存，结束批阅会放弃输入。继续结束？",
              )
            )
              close();
          }}
        >
          结束谱面批阅
        </button>
      </div>
      <ScoreHistoryReplay
        review={review}
        enabled={ready && !blocked && !busy}
        cursor={setReplayPosition}
        activity={setReplaying}
      />
      {replaying && (
        <p>
          回放位置：
          {replayAnchor
            ? `演奏第 ${replayAnchor.measure} 小节 · ${pitchName(replayAnchor.pitch)}`
            : currentMeasure
              ? `演奏第 ${currentMeasure.measure} 小节 · 当前没有保留的参考持音`
              : "此处没有可靠谱面对应"}
          。蓝色虚线标记参考起音位置，原判定边框保持。
        </p>
      )}
      <p>
        {review.exact
          ? review.truncated
            ? "已保留判定："
            : "整轮："
          : "已保留参考音："}
        {review.exact
          ? ["onTime", "early", "late", "missed", "pending"]
              .map(
                (k) =>
                  `${historyGradeName(k)} ${review.notes.filter((n) => n.kind === k).length}`,
              )
              .join(" · ")
          : review.notes.length}
        {review.wrongNotes != null ? ` · 错音 ${review.wrongNotes}` : ""}。
        {review.exact
          ? "边框显示当轮真实判定，错音不套用到某个书面音。"
          : "旧记录没有逐音判定，仅标记保留的参考音。"}
      </p>
      <div className="score-review-controls">
        <label>
          查看次数{" "}
          <input
            type="number"
            min={1}
            max={maxOccurrence}
            aria-label="谱面历史演奏次数"
            value={occurrence}
            style={{ width: 64 }}
            disabled={busy || !ready || replaying}
            onChange={(e) => {
              const value = Math.max(
                1,
                Math.min(
                  maxOccurrence,
                  Math.trunc(Number(e.target.value) || 1),
                ),
              );
              setOccurrence(value);
              setOffset(0);
              setSelected(null);
              pauseFollow();
              const first = [...groups.values()].find(
                (list) =>
                  list[value - 1] && noteByKey.get(noteKey(list[value - 1])),
              );
              if (first) selectPage(first[value - 1].page);
            }}
          />
          / {maxOccurrence}
        </label>
        <select
          aria-label="谱面历史判定筛选"
          value={filter}
          onChange={(e) => {
            setFilter(e.target.value);
            setOffset(0);
          }}
        >
          {[
            ["all", "全部参考音"],
            ["problems", "偏早、偏晚与漏音"],
            ["missed", "漏音"],
            ["early", "偏早"],
            ["late", "偏晚"],
            ["onTime", "准时"],
            ["pending", "未计分"],
          ].map(([v, label]) => (
            <option key={v} value={v}>
              {label}
            </option>
          ))}
        </select>
        <button
          disabled={!problems.length || !ready || busy || replaying}
          onClick={() => nextProblem(-1)}
        >
          上一问题音
        </button>
        <button
          disabled={!problems.length || !ready || busy || replaying}
          onClick={() => nextProblem(1)}
        >
          下一问题音
        </button>
        <label>
          前面{" "}
          <select
            aria-label="谱面批阅重练前置小节"
            value={before}
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
            aria-label="谱面批阅重练后置小节"
            value={after}
            onChange={(e) => setAfter(Number(e.target.value))}
          >
            {[0, 1, 2, 4, 8].map((n) => (
              <option key={n} value={n}>
                {n} 小节
              </option>
            ))}
          </select>
        </label>
      </div>
      <p>
        按每个书面音各自的演奏次数显示，反复前后分开查看。当前次数没有保留的目标不当作漏音。
        {!ready
          ? "谱面还未完成可靠映射，或已经更新，请先校对或重新从历史批阅。"
          : unmatched
            ? `${unmatched} 个参考音没有唯一谱面对应，未着色。`
            : ""}
        {review.truncated ? "本轮资料不完整，只显示已保留证据。" : ""}
      </p>
      <HistoryProblemPlan
        review={review}
        disabled={blocked || busy || replaying || !ready}
        openPlan={openPlan}
        dirtyChanged={setPlanDirty}
      />
      {error && <p role="alert">{error}</p>}
      <details className="score-review-list">
        <summary>本次查看参考音 · {rows.length} 个</summary>
        <div className="score-review-table">
          <table>
            <thead>
              <tr>
                <th>演奏小节 / 音</th>
                <th>判定 / 起音</th>
                <th>实际 / 参考力度</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              {rows.slice(offset, offset + 40).map(({ n }) => (
                <tr key={n.reference}>
                  <td>
                    第 {n.measure} 小节 · {pitchName(n.pitch)}
                  </td>
                  <td>
                    {historyGradeName(n.kind)}
                    {n.offsetMs != null
                      ? ` · ${n.offsetMs > 0 ? "晚" : n.offsetMs < 0 ? "早" : "一致"} ${Math.abs(n.offsetMs)} ms`
                      : ""}
                  </td>
                  <td>
                    {n.actualVelocity ?? "—"} / {n.referenceVelocity}
                  </td>
                  <td>
                    <button
                      disabled={!ready || busy || replaying}
                      onClick={() => locate(n)}
                    >
                      查看谱音
                    </button>
                    <button
                      disabled={blocked || busy || replaying}
                      onClick={() => void train(n.reference)}
                    >
                      重练这段
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {!rows.length && <p>这次演奏没有保留当前次数与筛选的参考音。</p>}
        {rows.length > 40 && (
          <div className="score-review-controls">
            <button
              disabled={offset === 0}
              onClick={() => setOffset(Math.max(0, offset - 40))}
            >
              上一页
            </button>
            <span>
              {offset + 1}–{Math.min(rows.length, offset + 40)} / {rows.length}
            </span>
            <button
              disabled={offset + 40 >= rows.length}
              onClick={() => setOffset(offset + 40)}
            >
              下一页
            </button>
          </div>
        )}
      </details>
    </section>
  );
}
