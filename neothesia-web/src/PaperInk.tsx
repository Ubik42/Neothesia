import { api } from "./api";
import { InkMergeReview } from "./InkMergeReview";
import { reviewInkMerge, type InkReview } from "./paperInkMerge";
import { useEffect, useRef, useState, type PointerEvent } from "react";
import type { PaperAnnotation } from "./PaperAnnotations";
import { paperPoint } from "./PaperGeometry";
const colors: Record<string, string> = {
  yellow: "#c69a20",
  blue: "#3679b9",
  green: "#35845d",
  red: "#ba5050",
};
type Point = [number, number];
type Draft = { expected: PaperAnnotation[]; strokes: PaperAnnotation[] };
type Undo = {
  before: PaperAnnotation[];
  after: PaperAnnotation[];
  undone: boolean;
};
function validRows(v: unknown): v is PaperAnnotation[] {
  return (
    Array.isArray(v) &&
    v.length <= 2000 &&
    v.every(
      (n) =>
        n &&
        typeof n.id === "string" &&
        typeof n.assetId === "string" &&
        Number.isInteger(n.page) &&
        [n.x, n.y, n.width, n.height].every(Number.isFinite) &&
        n.ink &&
        n.ink.points.length >= 2 &&
        n.ink.points.length <= 4096 &&
        n.ink.points.every(
          (p: unknown) =>
            Array.isArray(p) &&
            p.length === 2 &&
            p.every(
              (x) =>
                typeof x === "number" && Number.isFinite(x) && x >= 0 && x <= 1,
            ),
        ) &&
        Number.isFinite(n.ink.thickness),
    )
  );
}
function readDraft(key: string): Draft | null {
  try {
    const v = JSON.parse(localStorage.getItem(key) || "null");
    return v && validRows(v.expected) && validRows(v.strokes) ? v : null;
  } catch {
    return null;
  }
}
function stroke(
  points: Point[],
  assetId: string,
  page: number,
  color: string,
  thickness: number,
): PaperAnnotation {
  const xs = points.map((p) => p[0]),
    ys = points.map((p) => p[1]),
    minx = Math.min(...xs),
    maxx = Math.max(...xs),
    miny = Math.min(...ys),
    maxy = Math.max(...ys),
    w = Math.max(0.002, maxx - minx),
    h = Math.max(0.002, maxy - miny),
    x = Math.min(minx, 1 - w),
    y = Math.min(miny, 1 - h);
  return {
    id: `ink-${crypto.randomUUID()}`,
    assetId,
    page,
    x,
    y,
    width: w,
    height: h,
    text: "",
    color,
    ink: {
      thickness,
      points: points.map((p) => [
        Math.max(0, Math.min(1, (p[0] - x) / w)),
        Math.max(0, Math.min(1, (p[1] - y) / h)),
      ]),
    },
  };
}
function segmentDistance(p: Point, a: Point, b: Point) {
  const dx = b[0] - a[0],
    dy = b[1] - a[1],
    t =
      dx || dy
        ? Math.max(
            0,
            Math.min(
              1,
              ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy),
            ),
          )
        : 0;
  return Math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy);
}
export function usePaperInk({
  contentId,
  book,
  assetId,
  page,
  rotation,
  width,
  height,
  notes,
  disabled,
  blocked,
  save,
  visible,
  preview,
}: {
  contentId: string;
  book: string;
  assetId: string;
  page: number;
  rotation: number;
  width: number;
  height: number;
  notes: PaperAnnotation[];
  disabled: boolean;
  blocked: boolean;
  save?: (
    expected: PaperAnnotation[],
    strokes: PaperAnnotation[],
  ) => Promise<boolean>;
  visible: boolean;
  preview?: PaperAnnotation | null;
}) {
  const key = `neothesia-paper-ink-draft:${contentId}:${book}:${assetId}:${page}`,
    undoKey = key + ":undo",
    current = notes.filter(
      (n) => n.assetId === assetId && n.page === page && n.ink,
    );
  const [initial] = useState(() => readDraft(key)),
    [active, setActive] = useState(!!initial),
    [expected, setExpected] = useState(initial?.expected ?? current),
    [rows, setRows] = useState(initial?.strokes ?? current),
    [tool, setTool] = useState<"pen" | "eraser">("pen"),
    [color, setColor] = useState("red"),
    [thickness, setThickness] = useState(0.0022),
    [message, setMessage] = useState(""),
    [busy, setBusy] = useState(false),
    [hasDraft, setHasDraft] = useState(!!initial),
    [undo, setUndo] = useState<Undo | null>(() => {
      try {
        const v = JSON.parse(localStorage.getItem(undoKey) || "null");
        return v && validRows(v.before) && validRows(v.after) ? v : null;
      } catch {
        return null;
      }
    });
  const [history, setHistory] = useState<PaperAnnotation[][]>([]),
    [future, setFuture] = useState<PaperAnnotation[][]>([]),
    [livePoints, setLivePoints] = useState<Point[] | null>(null),
    points = useRef<Point[] | null>(null),
    surface = useRef<SVGSVGElement>(null),
    drawing = useRef(false);
  const [merge, setMerge] = useState<InkReview | null>(null);
  const changed = JSON.stringify(rows) !== JSON.stringify(expected),
    locked = disabled || busy || blocked || !!merge || !save;
  const push = (next: PaperAnnotation[]) => {
    setHistory((h) => [...h, rows].slice(-100));
    setFuture([]);
    setRows(next);
  };
  useEffect(() => {
    if (!active) return;
    if (!changed) {
      try {
        localStorage.removeItem(key);
        setHasDraft(false);
      } catch {}
      return;
    }
    try {
      localStorage.setItem(key, JSON.stringify({ expected, strokes: rows }));
      setHasDraft(true);
    } catch {
      setMessage("手写草稿无法保存在本机，请保存笔迹后再离开。");
    }
  }, [active, changed, rows, expected, key]);
  const begin = () => {
    const restored = readDraft(key);
    setExpected(restored?.expected ?? current);
    setRows(restored?.strokes ?? current);
    setHistory([]);
    setFuture([]);
    setMessage("");
    setActive(true);
  };
  const discard = () => {
    try {
      localStorage.removeItem(key);
    } catch {}
    setHasDraft(false);
    setActive(false);
    setRows(current);
    setExpected(current);
    setMessage("已放弃本次手写修改。");
  };
  const persistUndo = (v: Undo) => {
    setUndo(v);
    try {
      localStorage.setItem(undoKey, JSON.stringify(v));
    } catch {
      setMessage("笔迹已保存，但本机无法保留保存撤销记录。");
    }
  };
  const commit = async () => {
    if (locked || !save || points.current) return;
    setBusy(true);
    try {
      if (await save(expected, rows)) {
        persistUndo({ before: expected, after: rows, undone: false });
        try {
          localStorage.removeItem(key);
        } catch {}
        setHasDraft(false);
        setActive(false);
        setMessage("笔迹已保存。");
      } else
        setMessage(
          "笔迹未保存，草稿保留。若本页笔迹已改变，请核对后重新编辑。",
        );
    } finally {
      setBusy(false);
    }
  };
  const undoSave = async () => {
    if (locked || !save || !undo) return;
    setBusy(true);
    try {
      const from = undo.undone ? undo.before : undo.after,
        to = undo.undone ? undo.after : undo.before;
      if (await save(from, to)) {
        persistUndo({ ...undo, undone: !undo.undone });
        setMessage(undo.undone ? "已重做笔迹保存。" : "已撤销上次笔迹保存。");
      } else setMessage("本页笔迹已改变，无法撤销这次保存。现有笔迹保留。");
    } finally {
      setBusy(false);
    }
  };
  const recheck = async () => {
    if (locked || points.current) return;
    setBusy(true);
    setMessage("");
    try {
      const result = await api.command<{
        attachments: { id: string; annotations?: PaperAnnotation[] }[];
      }>({ type: "scoreAttachments", content_id: contentId });
      const paper = result.attachments.find((a) => a.id === book);
      if (!paper) throw new Error("谱面版本已移除，请暂存草稿后重新打开");
      const saved = (paper.annotations ?? []).filter(
        (n) => n.assetId === assetId && n.page === page && n.ink,
      );
      setMerge(reviewInkMerge(expected, rows, saved));
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  };
  const position = (e: PointerEvent<SVGSVGElement>): Point => {
    const r = e.currentTarget.getBoundingClientRect();
    return paperPoint(
      Math.max(0, Math.min(1, (e.clientX - r.x) / r.width)),
      Math.max(0, Math.min(1, (e.clientY - r.y) / r.height)),
      (360 - rotation) % 360,
    );
  };
  const pixel = (p: Point): Point => {
    const v = paperPoint(p[0], p[1], rotation);
    return [v[0] * width, v[1] * height];
  };
  const inkPath = (n: PaperAnnotation) =>
    n
      .ink!.points.map((p, i) => {
        const [x, y] = pixel([n.x + p[0] * n.width!, n.y + p[1] * n.height!]);
        return `${i ? "L" : "M"}${x} ${y}`;
      })
      .join(" ");
  const baseWidth = rotation % 180 === 0 ? width : height;
  const erase = (p: Point) => {
    const point = pixel(p),
      removed = rows.find((n) => {
        const ps = n.ink!.points.map((p) =>
          pixel([n.x + p[0] * n.width!, n.y + p[1] * n.height!]),
        );
        return ps.some(
          (v, i) =>
            i > 0 &&
            segmentDistance(point, ps[i - 1], v) <=
              Math.max(7, (n.ink!.thickness * baseWidth) / 2 + 4),
        );
      });
    if (removed) push(rows.filter((n) => n.id !== removed.id));
  };
  const finish = () => {
    if (points.current) {
      const ps = points.current;
      if (ps.length === 1) ps.push([...ps[0]]);
      push([...rows, stroke(ps, assetId, page, color, thickness)]);
    }
    points.current = null;
    drawing.current = false;
    setLivePoints(null);
  };
  const tools = (
    <>
      {!active ? (
        <>
          <button disabled={locked} onClick={begin}>
            {hasDraft ? "继续手写草稿" : "手写标记"}
          </button>
          {undo && (
            <button disabled={locked} onClick={() => void undoSave()}>
              {undo.undone ? "重做笔迹保存" : "撤销上次笔迹保存"}
            </button>
          )}
        </>
      ) : (
        <>
          <button
            disabled={locked}
            aria-pressed={tool === "pen"}
            onClick={() => setTool("pen")}
          >
            画笔
          </button>
          <button
            disabled={locked}
            aria-pressed={tool === "eraser"}
            onClick={() => setTool("eraser")}
          >
            橡皮
          </button>
          <label>
            颜色
            <select
              aria-label="笔迹颜色"
              disabled={locked}
              value={color}
              onChange={(e) => setColor(e.target.value)}
            >
              {Object.keys(colors).map((c) => (
                <option key={c} value={c}>
                  {
                    {
                      yellow: "黄色",
                      blue: "蓝色",
                      green: "绿色",
                      red: "红色",
                    }[c]
                  }
                </option>
              ))}
            </select>
          </label>
          <label>
            粗细
            <select
              aria-label="笔迹粗细"
              disabled={locked}
              value={thickness}
              onChange={(e) => setThickness(Number(e.target.value))}
            >
              <option value={0.001}>细</option>
              <option value={0.0022}>中</option>
              <option value={0.004}>粗</option>
              <option value={0.007}>加粗</option>
            </select>
          </label>
          <button
            disabled={locked || !history.length}
            onClick={() => {
              setFuture((f) => [rows, ...f]);
              setRows(history.at(-1)!);
              setHistory((h) => h.slice(0, -1));
            }}
          >
            撤销笔画
          </button>
          <button
            disabled={locked || !future.length}
            onClick={() => {
              setHistory((h) => [...h, rows]);
              setRows(future[0]);
              setFuture((f) => f.slice(1));
            }}
          >
            重做笔画
          </button>
          <button
            disabled={locked || !changed || !!livePoints}
            onClick={() => void commit()}
          >
            保存笔迹
          </button>
          <button
            disabled={locked || !!livePoints}
            onClick={() => void recheck()}
          >
            核对已保存笔迹
          </button>
          <button
            disabled={locked || !!livePoints}
            onClick={() => {
              setActive(false);
              setMessage(changed ? "手写草稿已暂存。" : "已退出手写。");
            }}
          >
            暂存并退出手写
          </button>
          <button disabled={locked || !!livePoints} onClick={discard}>
            放弃笔迹修改
          </button>
          <span>
            {rows.length} 笔 ·{" "}
            {tool === "pen" ? "在谱页上书写" : "点击或划过整笔擦除"}
          </span>
        </>
      )}
      {message && <span role="status">{message}</span>}
      {merge && (
        <InkMergeReview
          review={merge}
          close={() => setMerge(null)}
          apply={(strokes) => {
            setExpected(merge.current);
            push(strokes);
            setMerge(null);
            setMessage("已合并到手写草稿，请查看后保存笔迹。");
          }}
        />
      )}
    </>
  );
  const overlay = (visible || active) && (
    <svg
      ref={surface}
      className={`paper-ink-layer ${active ? "active" : ""}`}
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      aria-label="谱页手写笔迹"
      onPointerDown={(e) => {
        if (!active || locked || e.button !== 0) return;
        e.preventDefault();
        e.stopPropagation();
        e.currentTarget.setPointerCapture(e.pointerId);
        drawing.current = true;
        const p = position(e);
        if (tool === "eraser") {
          erase(p);
          return;
        }
        if (rows.length >= 2000) {
          setMessage("已达本页笔迹容量，请先擦除部分笔迹。");
          return;
        }
        points.current = [p];
        setLivePoints([p]);
      }}
      onPointerMove={(e) => {
        if (!active || locked || !drawing.current) return;
        e.preventDefault();
        e.stopPropagation();
        const p = position(e);
        if (tool === "eraser") {
          erase(p);
          return;
        }
        const ps = points.current;
        if (!ps) return;
        if (ps.length >= 4096) {
          setMessage("这一笔已达点数上限，请抬起后继续书写。");
          return;
        }
        if (
          Math.hypot(p[0] - ps.at(-1)![0], p[1] - ps.at(-1)![1]) * baseWidth <
          0.7
        )
          return;
        ps.push(p);
        setLivePoints([...ps]);
      }}
      onPointerUp={(e) => {
        if (!drawing.current) return;
        e.preventDefault();
        e.stopPropagation();
        finish();
        if (e.currentTarget.hasPointerCapture(e.pointerId))
          e.currentTarget.releasePointerCapture(e.pointerId);
      }}
      onPointerCancel={() => {
        points.current = null;
        drawing.current = false;
        setLivePoints(null);
        setMessage("本笔已取消，其他笔迹保留。");
      }}
      onClick={(e) => {
        if (active) e.stopPropagation();
      }}
    >
      {(active
        ? rows
        : current.map((n) => (preview?.id === n.id ? preview : n))
      ).map((n) => (
        <path
          key={n.id}
          data-ink-id={n.id}
          d={inkPath(n)}
          fill="none"
          stroke={colors[n.color] ?? colors.red}
          strokeWidth={n.ink!.thickness * baseWidth}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      ))}
      {livePoints && (
        <path
          d={livePoints
            .map((p, i) => {
              const v = pixel(p);
              return `${i ? "L" : "M"}${v[0]} ${v[1]}`;
            })
            .join(" ")}
          fill="none"
          stroke={colors[color]}
          strokeWidth={thickness * baseWidth}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
    </svg>
  );
  return { tools, overlay, active };
}
