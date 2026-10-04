import { ScoreHistoryReview } from "./ScoreHistoryReview";
import type {
  HistoryScoreFocus,
  HistoryScoreReview,
} from "./historyScoreFocus";
import { useEffect, useRef, useState, useMemo } from "react";
import {
  api,
  pitchName,
  type Note,
  type Snapshot,
  type LoadedSong,
} from "./api";
import { ScoreNoteEditor, type ScoreMapping } from "./ScoreNoteEditor";
import { ScoreExportDialog } from "./ScoreExportDialog";
import { ScoreRoute, type WrittenRoute } from "./ScoreRoute";
import { ScoreRangeDialog } from "./ScoreRangeDialog";
import { actionsForNote, actionUsable, fingerSequence } from "./fingerActions";
interface Asset {
  contentId: string;
  bytes: number[];
  name: string;
  compressed: boolean;
  coverage: number;
  confidence: number;
  readiness: string;
  diagnostics: string[];
  warnings: string[];
  route?: WrittenRoute | null;
}
type Mapping = ScoreMapping;
function cleanSvg(source: string) {
  const doc = new DOMParser().parseFromString(source, "image/svg+xml");
  doc
    .querySelectorAll("script,foreignObject,image,iframe")
    .forEach((e) => e.remove());
  doc.querySelectorAll("*").forEach((e) => {
    for (const a of Array.from(e.attributes))
      if (
        a.name.toLowerCase().startsWith("on") ||
        (["href", "xlink:href"].includes(a.name) && !a.value.startsWith("#"))
      )
        e.removeAttribute(a.name);
  });
  doc.querySelectorAll("g.note[data-id]").forEach((n) => {
    if (!n.id) n.setAttribute("id", n.getAttribute("data-id")!);
  });
  return new XMLSerializer().serializeToString(doc.documentElement);
}
export function Notation({
  song,
  state,
  seek,
  changed,
  detail,
  assignHands,
  edit,
  onEdit,
  canEdit,
  historyFocus,
  clearHistoryFocus,
  historyReview,
  clearHistoryReview,
  practiceHistoryReview,
  openHistoryPlan,
}: {
  song: LoadedSong;
  state: Snapshot;
  seek: (position: number) => void;
  changed: () => Promise<void>;
  detail: (note: Note) => void;
  assignHands: () => void;
  edit: boolean;
  canEdit: boolean;
  onEdit: (value: boolean) => void;
  historyFocus?: HistoryScoreFocus | null;
  clearHistoryFocus?: () => void;
  historyReview?: HistoryScoreReview | null;
  clearHistoryReview: () => void;
  openHistoryPlan: (id: string, day: string | null) => void;
  practiceHistoryReview: (
    reference: number,
    before: number,
    after: number,
  ) => Promise<void>;
}) {
  const [rangeSelection, setRangeSelection] = useState<{
    first: number;
    last: number;
  } | null>(null);
  const [pages, setPages] = useState<string[]>([]),
    [page, setPage] = useState(0),
    [asset, setAsset] = useState<Asset | null>(null),
    [mapping, setMapping] = useState<Mapping[]>([]),
    [follow, setFollow] = useState(true),
    [error, setError] = useState(""),
    [loading, setLoading] = useState(true),
    [ambiguities, setAmbiguities] = useState(0);
  const [showFingers, setShowFingers] = useState(true),
    [selected, setSelected] = useState<Mapping | null>(null),
    [selectionError, setSelectionError] = useState("");
  useEffect(() => {
    if (!edit) setSelected(null);
  }, [edit]);
  const [editorBusy, setEditorBusy] = useState(false);
  const [exporting, setExporting] = useState(false);
  useEffect(() => setExporting(false), [song.contentId, song.scoreRevision]);
  const [zoom, setZoom] = useState(0),
    [simpleFingers, setSimpleFingers] = useState(false),
    [paperWidth, setPaperWidth] = useState(900),
    [paperHeight, setPaperHeight] = useState(600);
  const scroll = useRef<HTMLDivElement>(null);
  useEffect(() => {
    try {
      const v = JSON.parse(
        localStorage.getItem(`notation-view:${song.contentId}`) || "{}",
      );
      setZoom([0, 75, 100, 125, 150, 200].includes(v.zoom) ? v.zoom : 0);
      setSimpleFingers(v.simple === true);
    } catch {
      setZoom(0);
      setSimpleFingers(false);
    }
  }, [song.contentId]);
  useEffect(() => {
    try {
      localStorage.setItem(
        `notation-view:${song.contentId}`,
        JSON.stringify({ zoom, simple: simpleFingers }),
      );
    } catch {}
  }, [song.contentId, zoom, simpleFingers]);
  useEffect(() => {
    const el = scroll.current;
    if (!el) return;
    const update = () => {
      setPaperWidth(Math.max(240, el.clientWidth - 24));
      const pane = el.closest(".notation-pane");
      if (pane)
        setPaperHeight(
          Math.max(
            160,
            pane.getBoundingClientRect().bottom -
              el.getBoundingClientRect().top -
              12,
          ),
        );
    };
    const ro = new ResizeObserver(update);
    ro.observe(el);
    const pane = el.closest(".notation-pane");
    if (pane) ro.observe(pane);
    update();
    return () => ro.disconnect();
  }, [edit, selected?.id, loading]);
  const important = useMemo(() => {
    const keep = new Set<string>();
    for (const a of song.fingerActions ?? []) keep.add(`${a.track}:${a.index}`);
    const byHand = new Map<string, Note[]>();
    for (const n of song.notes) {
      const key = `${n.track}:${n.part}`;
      const rows = byHand.get(key) || [];
      rows.push(n);
      byHand.set(key, rows);
    }
    for (const rows of byHand.values()) {
      rows.sort((a, b) => a.start - b.start || a.pitch - b.pitch);
      rows.forEach((n, i) => {
        const a = rows[i - 1],
          b = rows[i + 1];
        const smooth = (x: Note | undefined, y: Note | undefined) =>
          x &&
          y &&
          x.finger != null &&
          y.finger != null &&
          y.start > x.start &&
          y.start - x.start <= x.duration + 0.12 &&
          Math.abs(y.pitch - x.pitch) <= 2 &&
          Math.abs(y.finger - x.finger) === 1 &&
          Math.sign(y.pitch - x.pitch) ===
            Math.sign((y.finger - x.finger) * (n.part === "left" ? -1 : 1));
        if (!smooth(a, n) || !smooth(n, b)) keep.add(`${n.track}:${n.index}`);
      });
    }
    return keep;
  }, [song.notes, song.fingerActions]);
  const html = useMemo(() => ({ __html: pages[page] ?? "" }), [pages, page]);
  const notesById = useMemo(
    () => new Map(song.notes.map((n) => [`${n.track}:${n.index}`, n])),
    [song.notes],
  );
  const root = useRef<HTMLDivElement>(null),
    activeIds = useRef<string[]>([]);
  useEffect(() => {
    let disposed = false;
    const worker = new Worker("/verovio/worker.mjs", { type: "module" });
    setPages([]);
    setSelected(null);
    setSelectionError("");
    setMapping([]);
    setLoading(true);
    setError("");
    setPage(0);
    api
      .command<Asset | null>({ type: "score" })
      .then((data) => {
        if (disposed) return;
        if (!data) {
          setLoading(false);
          setError("请先为这首 MIDI 配对 MusicXML 或 MXL 乐谱");
          return;
        }
        setAsset(data);
        worker.postMessage({ bytes: data.bytes, compressed: data.compressed });
        worker.onmessage = async ({ data: result }) => {
          if (disposed) return;
          if (result.error) {
            setError(result.error);
            setLoading(false);
            return;
          }
          try {
            const midi = Array.from(atob(result.midi), (c: string) =>
              c.charCodeAt(0),
            );
            const correlated = await api.command<{
              mapping: Mapping[];
              ambiguities: number;
              following: boolean;
            }>({
              type: "scoreMap",
              score_revision: song.scoreRevision,
              midi,
              notes: result.notes,
              content_id: data.contentId,
            });
            if (disposed) return;
            setMapping(correlated.mapping);
            setAmbiguities(correlated.ambiguities);
            setPages(result.pages.map(cleanSvg));
            setFollow(correlated.following);
            setLoading(false);
          } catch (e) {
            if (!disposed) {
              setError(String(e instanceof Error ? e.message : e));
              setLoading(false);
            }
          }
        };
        worker.onerror = (e) => {
          if (!disposed) {
            setError(e.message || "乐谱渲染器启动失败");
            setLoading(false);
          }
        };
      })
      .catch((e) => {
        if (!disposed) {
          setError(String(e.message));
          setLoading(false);
        }
      });
    return () => {
      disposed = true;
      worker.terminate();
    };
  }, [song.contentId, song.hasScore, song.scoreRevision]);
  useEffect(() => {
    if (historyFocus && ["playing", "countIn"].includes(state.status))
      setFollow(true);
  }, [historyFocus?.request, state.status]);
  const focusApplied = useRef("");
  const focusTargets = useMemo(
    () =>
      historyFocus?.contentId === song.contentId
        ? mapping.filter(
            (m) =>
              m.track === historyFocus.track &&
              m.index === historyFocus.index &&
              m.pitch === historyFocus.pitch,
          )
        : [],
    [mapping, historyFocus, song.contentId],
  );
  const focusReliable =
    historyFocus?.scoreRevision === song.scoreRevision &&
    asset?.readiness === "Ready" &&
    new Set(focusTargets.map((m) => `${m.page}:${m.id}`)).size === 1;
  useEffect(() => {
    if (!historyFocus || !focusReliable || loading) return;
    const key = `${historyFocus.request}:${song.scoreRevision}`;
    if (focusApplied.current !== key) {
      focusApplied.current = key;
      setFollow(false);
      setPage(focusTargets[0].page);
    }
  }, [historyFocus, focusReliable, focusTargets, loading, song.scoreRevision]);
  useEffect(() => {
    const el = root.current;
    if (!el) return;
    el.querySelectorAll(".history-score-outline").forEach((n) => n.remove());
    el.querySelectorAll(".history-score-note").forEach((n) => {
      n.classList.remove("history-score-note");
      n.removeAttribute("data-history-grade");
    });
    if (!historyFocus || !focusReliable || loading) return;
    const note = focusTargets[0];
    if (note.page !== page) return;
    const target = el.querySelector<SVGGraphicsElement>(
      `[id="${CSS.escape(note.id)}"]`,
    );
    if (!target) return;
    target.classList.add("history-score-note");
    target.setAttribute("data-history-grade", historyFocus.kind);
    const head =
      target.querySelector<SVGGraphicsElement>(".notehead") ?? target;
    const svg = target.ownerSVGElement,
      ctm = svg?.getScreenCTM();
    if (svg && ctm) {
      const box = head.getBoundingClientRect(),
        inverse = ctm.inverse();
      const a = new DOMPoint(box.left - 7, box.top - 7).matrixTransform(
          inverse,
        ),
        b = new DOMPoint(box.right + 7, box.bottom + 7).matrixTransform(
          inverse,
        );
      const outline = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "rect",
      );
      outline.setAttribute("class", "history-score-outline");
      outline.setAttribute("x", String(a.x));
      outline.setAttribute("y", String(a.y));
      outline.setAttribute("width", String(b.x - a.x));
      outline.setAttribute("height", String(b.y - a.y));
      outline.setAttribute("rx", String(4 / Math.hypot(ctm.a, ctm.b)));
      outline.setAttribute("fill", "none");
      outline.setAttribute(
        "stroke",
        historyFocus.kind === "missed" ? "#bc4242" : "#b07912",
      );
      outline.setAttribute(
        "stroke-width",
        String(2 / Math.hypot(ctm.a, ctm.b)),
      );
      outline.setAttribute("pointer-events", "none");
      outline.style.fill = "none";
      outline.style.stroke =
        historyFocus.kind === "missed" ? "#bc4242" : "#b07912";
      outline.style.strokeWidth = String(2 / Math.hypot(ctm.a, ctm.b));
      svg.append(outline);
    }
    const frame = requestAnimationFrame(() =>
      target.scrollIntoView({ block: "center", inline: "center" }),
    );
    return () => cancelAnimationFrame(frame);
  }, [historyFocus, focusReliable, focusTargets, loading, page, html]);
  useEffect(() => {
    const current = mapping.filter(
      (n) => n.start <= state.position + 0.03 && n.end >= state.position,
    );
    if (follow) {
      const next = current[0] ?? mapping.find((n) => n.start >= state.position);
      if (next) setPage(next.page);
    }
    activeIds.current.forEach((id) =>
      root.current
        ?.querySelector(`[id="${CSS.escape(id)}"]`)
        ?.classList.remove("playing-note"),
    );
    activeIds.current = current.filter((n) => n.page === page).map((n) => n.id);
    activeIds.current.forEach((id) =>
      root.current
        ?.querySelector(`[id="${CSS.escape(id)}"]`)
        ?.classList.add("playing-note"),
    );
  }, [state.position, mapping, follow, page, pages]);
  useEffect(() => {
    const el = root.current;
    if (!el) return;
    el.querySelectorAll(".score-practice-finger").forEach((n) => n.remove());
    el.querySelectorAll("g.note").forEach((n) => {
      n.classList.remove("selected-score-note");
      n.removeAttribute("tabindex");
      n.removeAttribute("role");
    });
    const groups = new Map<string, Mapping[]>();
    for (const m of mapping.filter((m) => m.page === page)) {
      const old = groups.get(m.id) ?? [];
      if (!old.some((x) => x.track === m.track && x.index === m.index))
        old.push(m);
      groups.set(m.id, old);
    }
    const occupiedLabels: DOMRect[] = [];
    for (const [id, entries] of groups) {
      const target = el.querySelector<SVGGraphicsElement>(
        `[id="${CSS.escape(id)}"]`,
      );
      if (!target) continue;
      target.setAttribute("role", "button");
      target.setAttribute("tabindex", "0");
      target.setAttribute(
        "aria-label",
        `${pitchName(entries[0].pitch)} · ${entries.length} 个演奏位置 · ${edit ? "编辑指法" : "定位"}`,
      );
      if (selected?.id === id) target.classList.add("selected-score-note");
      if (!showFingers) continue;
      const notes = entries.map((m) => notesById.get(`${m.track}:${m.index}`));
      if (
        simpleFingers &&
        selected?.id !== id &&
        entries.every((m) => !important.has(`${m.track}:${m.index}`))
      )
        continue;
      const actions = entries.flatMap((m) => actionsForNote(song, m));
      const fingers = Array.from(
        new Set(
          notes
            .map((n) =>
              fingerSequence(n, n ? actionsForNote(song, n) : [], state.speed),
            )
            .filter(Boolean),
        ),
      );
      if (!fingers.length) continue;
      const head =
        target.querySelector<SVGGraphicsElement>(".notehead") ?? target;
      const svg = target.ownerSVGElement,
        ctm = svg?.getScreenCTM();
      if (!svg || !ctm) continue;
      const box = head.getBoundingClientRect();
      const point = new DOMPoint(
        box.left + box.width / 2,
        box.bottom + 13,
      ).matrixTransform(ctm.inverse());
      const label = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "text",
      );
      label.setAttribute("class", "score-practice-finger");
      if (actions.length) {
        label.classList.add("score-practice-substitution");
        label.setAttribute("data-score-note", id);
        label.setAttribute("tabindex", "0");
        label.setAttribute("role", "button");
        label.setAttribute(
          "aria-label",
          `${pitchName(entries[0].pitch)} 持音换指 ${fingers.join(" / ")}，查看演奏位置`,
        );
        target.setAttribute(
          "aria-label",
          `${pitchName(entries[0].pitch)} · ${entries.length} 个演奏位置 · 持音换指 ${fingers.join(" / ")}`,
        );
        label.setAttribute(
          "data-actions-valid",
          String(actions.every((a) => actionUsable(a, state.speed))),
        );
      }
      label.setAttribute("x", String(point.x));
      label.setAttribute("y", String(point.y));
      label.setAttribute("font-size", String(13 / Math.hypot(ctm.a, ctm.b)));
      label.setAttribute("text-anchor", "middle");
      label.setAttribute(
        "fill",
        actions.some((a) => !actionUsable(a, state.speed))
          ? "#9a4b08"
          : notes[0]?.part === "left"
            ? "#0765a3"
            : "#4b751c",
      );
      label.setAttribute("font-weight", "700");
      label.textContent = fingers.join("/");
      svg.append(label);
      // Keep longer substitution chains clear of adjacent personal fingering labels.
      for (let row = 1; row <= 10; row++) {
        const rect = label.getBoundingClientRect();
        if (
          !occupiedLabels.some(
            (old) =>
              rect.left < old.right + 3 &&
              rect.right + 3 > old.left &&
              rect.top < old.bottom + 3 &&
              rect.bottom + 3 > old.top,
          )
        )
          break;
        const shifted = new DOMPoint(
          box.left + box.width / 2,
          box.bottom + 13 + row * 15,
        ).matrixTransform(ctm.inverse());
        label.setAttribute("x", String(shifted.x));
        label.setAttribute("y", String(shifted.y));
      }
      occupiedLabels.push(label.getBoundingClientRect());
    }
  }, [
    mapping,
    pages,
    page,
    song.notes,
    song.fingerActions,
    state.speed,
    showFingers,
    selected,
    edit,
    simpleFingers,
    important,
    zoom,
    paperWidth,
  ]);
  const activate = (target: Element | null) => {
    if (!target || editorBusy) return;
    const choices = mapping.filter(
      (n) => n.id === target.id && n.page === page,
    );
    const unique = Array.from(
      new Map(choices.map((n) => [`${n.track}:${n.index}`, n])).values(),
    );
    const n = unique.find((n) => n.end >= state.position) ?? unique[0];
    if (!n) {
      if (edit)
        setSelectionError("此谱音没有可靠的演奏对应关系，请先核对乐谱配对。");
      return;
    }
    setSelectionError("");
    if (edit) {
      setFollow(false);
      setSelected(n);
    } else seek(n.start);
  };
  const activateLabel = (element: Element | null) => {
    const label = element?.closest(".score-practice-substitution");
    if (label) {
      const id = label.getAttribute("data-score-note");
      const choices = mapping
        .filter((m) => m.id === id && m.page === page)
        .sort((a, b) => a.start - b.start);
      const choice = choices.find((m) => m.end >= state.position) ?? choices[0];
      if (!choice || editorBusy) return;
      if (canEdit) {
        onEdit(true);
        setFollow(false);
        setSelected(choice);
      } else seek(choice.start);
    } else activate(element?.closest("g.note") ?? null);
  };
  const locate = (track: number, index: number) => {
    const next = mapping.find((m) => m.track === track && m.index === index);
    if (!next) return false;
    setFollow(false);
    setPage(next.page);
    setSelected(next);
    return true;
  };
  const positions = selected
    ? Array.from(
        new Map(
          mapping
            .filter((m) => m.id === selected.id && m.page === selected.page)
            .map((m) => [`${m.track}:${m.index}`, m]),
        ).values(),
      ).sort((a, b) => a.start - b.start)
    : [];
  return (
    <div className="notation-pane">
      <div className="notation-toolbar">
        <button
          disabled={page === 0}
          onClick={() => {
            setFollow(false);
            setPage(page - 1);
          }}
        >
          上一页
        </button>
        <span>{pages.length ? `${page + 1} / ${pages.length}` : "乐谱"}</span>
        <button
          disabled={page >= pages.length - 1}
          onClick={() => {
            setFollow(false);
            setPage(page + 1);
          }}
        >
          下一页
        </button>
        <label>
          <input
            type="checkbox"
            checked={follow}
            onChange={(e) => setFollow(e.target.checked)}
            disabled={!mapping.length}
          />
          跟随演奏
        </label>
        <button
          className={edit ? "selected" : ""}
          disabled={!mapping.length || editorBusy || !canEdit}
          onClick={() => {
            onEdit(!edit);
            setSelected(null);
            setSelectionError("");
          }}
        >
          谱面指法
        </button>
        <label>
          <input
            type="checkbox"
            checked={showFingers}
            onChange={(e) => setShowFingers(e.target.checked)}
          />
          练习指法
        </label>
        <label>
          显示
          <select
            aria-label="指法显示密度"
            value={simpleFingers ? "important" : "all"}
            disabled={!showFingers}
            onChange={(e) => setSimpleFingers(e.target.value === "important")}
            title="重点显示保留乐句首尾、换指、重复音与不连续的指法，省略连续顺指的中间标注"
          >
            <option value="all">全部指法</option>
            <option value="important">重点指法</option>
          </select>
        </label>
        <label>
          缩放
          <select
            aria-label="乐谱缩放"
            value={zoom}
            onChange={(e) => setZoom(Number(e.target.value))}
          >
            <option value={0}>适合宽度</option>
            {[75, 100, 125, 150, 200].map((z) => (
              <option key={z} value={z}>
                {z}%
              </option>
            ))}
          </select>
        </label>
        <button
          disabled={!asset || loading || editorBusy}
          onClick={() => setExporting(true)}
        >
          导出标记谱
        </button>
        <span>
          {asset
            ? `配对 ${asset.coverage}% · ${asset.readiness === "Ready" ? "可跟随" : asset.readiness === "Blocked" ? "导航未通过" : "需校对"}`
            : ""}
        </span>
      </div>
      {historyReview?.contentId === song.contentId && (
        <ScoreHistoryReview
          key={`${historyReview.historyId}:${historyReview.scoreRevision}`}
          review={historyReview}
          openPlan={openHistoryPlan}
          mapping={mapping}
          root={root}
          page={page}
          markup={pages[page] ?? ""}
          ready={
            !loading &&
            asset?.readiness === "Ready" &&
            historyReview.scoreRevision === song.scoreRevision
          }
          blocked={
            state.recording ||
            !!state.routine ||
            !!state.ladder?.active ||
            ["playing", "countIn"].includes(state.status) ||
            !!state.pressed.length
          }
          selectPage={setPage}
          pauseFollow={() => setFollow(false)}
          practice={practiceHistoryReview}
          close={clearHistoryReview}
        />
      )}
      {historyFocus?.contentId === song.contentId && (
        <div className="score-history-focus" role="status">
          <span>
            历史参考音 · 演奏第 {historyFocus.measure} 小节 ·{" "}
            {pitchName(historyFocus.pitch)} ·{" "}
            {(
              {
                onTime: "准时",
                early: "偏早",
                late: "偏晚",
                missed: "漏音",
                pending: "未计分",
                approximate: "近似对照",
              } as Record<string, string>
            )[historyFocus.kind] ?? historyFocus.kind}
          </span>
          <span>
            {loading
              ? "正在定位谱面…"
              : focusReliable
                ? "已标出当前关联乐谱中的对应音，反复位置按所选那次演奏定位。"
                : historyFocus.scoreRevision !== song.scoreRevision
                  ? "关联乐谱已更新，请回到历史重新定位；选段仍可练习。"
                  : "当前谱面尚无可靠的唯一对应，请先校对乐谱；选段仍可练习。"}
          </span>
          <button onClick={clearHistoryFocus}>清除历史标记</button>
        </div>
      )}
      {asset?.route && (
        <ScoreRoute
          key={`${song.contentId}:${song.scoreRevision}`}
          route={asset.route}
          position={state.position}
          seek={seek}
          disabled={editorBusy}
          selectRange={(first, last) => setRangeSelection({ first, last })}
          loop={async (start, end) => {
            await api.command({ type: "loop", start, end, enabled: true });
          }}
        />
      )}
      {rangeSelection && (
        <ScoreRangeDialog
          song={song}
          {...rangeSelection}
          close={() => setRangeSelection(null)}
          changed={changed}
        />
      )}
      {exporting && (
        <ScoreExportDialog song={song} close={() => setExporting(false)} />
      )}
      {loading && <p className="score-message">正在排版乐谱…</p>}
      {error && (
        <p className="score-message" role="alert">
          {error}
        </p>
      )}
      {asset && asset.diagnostics.length > 0 && (
        <p className="score-message">{asset.diagnostics.join("；")}</p>
      )}
      {ambiguities > 0 && (
        <p className="score-message">
          {ambiguities} 组音符身份不明确，已保留显示并跳过高亮。
        </p>
      )}
      {!!song.fingerActions?.length && (
        <p className="score-message score-held-legend">
          箭头表示保持琴键按下时换指；点选数字查看拍位与衔接音。⚠
          表示当前条件需要重新审阅。
          {song.fingerActions.some(
            (a) =>
              !mapping.some((m) => m.track === a.track && m.index === a.index),
          )
            ? "部分换指尚无可靠谱面对应，请在指法建议中查看。"
            : ""}
        </p>
      )}
      {selectionError && (
        <p className="score-message" role="alert">
          {selectionError}
        </p>
      )}
      {edit && !selected && (
        <p className="score-message">
          点选谱面音符编辑指法；有多次演奏位置时可分别修改。
        </p>
      )}
      <div className={`notation-body ${edit && selected ? "editing" : ""}`}>
        <div
          className="notation-scroll"
          ref={scroll}
          style={{ maxHeight: paperHeight }}
        >
          <div
            className="score-paper"
            style={{
              width: zoom ? `${(1050 * zoom) / 100}px` : `${paperWidth}px`,
              maxWidth: "none",
            }}
            ref={root}
            onClick={(e) => activateLabel(e.target as Element)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                const target = (e.target as Element).closest(
                  "g.note,.score-practice-substitution",
                );
                if (target) {
                  e.preventDefault();
                  e.stopPropagation();
                  activateLabel(target);
                }
              }
            }}
            dangerouslySetInnerHTML={html}
          />
        </div>
        {edit && selected && (
          <ScoreNoteEditor
            song={song}
            positions={positions}
            selected={selected}
            choose={setSelected}
            close={() => setSelected(null)}
            changed={changed}
            detail={detail}
            assignHands={assignHands}
            onBusy={setEditorBusy}
            seek={seek}
            locate={locate}
            rate={state.speed}
          />
        )}
      </div>
    </div>
  );
}
