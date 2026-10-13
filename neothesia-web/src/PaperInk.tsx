import { api } from "./api";
import { InkMergeReview } from "./InkMergeReview";
import { reviewInkMerge, type InkReview } from "./paperInkMerge";
import { useEffect, useRef, useState, type PointerEvent } from "react";
import type { PaperAnnotation } from "./PaperAnnotations";
import { moveInk, eraseInk } from "./paperInkEditing";
import "./paperInkEditing.css";
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
    [tool, setTool] = useState<"pen" | "eraser" | "select">("pen"),
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
  const [eraserMode, setEraserMode] = useState<"whole" | "partial">("whole");
  const [eraserRadius, setEraserRadius] = useState(8);
  const [eraserCursor, setEraserCursor] = useState<Point | null>(null);
  const eraseGesture = useRef<{
    next: PaperAnnotation[];
    last: Point;
    failed: boolean;
  } | null>(null);
  const gesturePointer = useRef<number | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [moving, setMoving] = useState<PaperAnnotation[] | null>(null);
  const move = useRef<{
    start: Point;
    rows: PaperAnnotation[];
    ids: string[];
    next: PaperAnnotation[];
  } | null>(null);
  const selectedRows = rows.filter((n) => selected.includes(n.id));
  useEffect(() => {
    if (!drawing.current) return;
    points.current = null;
    drawing.current = false;
    gesturePointer.current = null;
    move.current = null;
    eraseGesture.current = null;
    setMoving(null);
    setLivePoints(null);
    setMessage("谱页尺寸已改变，当前操作已取消。其他笔迹保留。");
  }, [width, height, rotation]);
  const [merge, setMerge] = useState<InkReview | null>(null);
  const changed = JSON.stringify(rows) !== JSON.stringify(expected),
    locked = disabled || busy || blocked || !!merge || !save;
  const push = (next: PaperAnnotation[]) => {
    if (JSON.stringify(next) === JSON.stringify(rows)) return;
    setMessage("");
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
    setSelected([]);
    const restored = readDraft(key);
    setExpected(restored?.expected ?? current);
    setRows(restored?.strokes ?? current);
    setHistory([]);
    setFuture([]);
    setMessage("");
    setActive(true);
  };
  const discard = () => {
    setSelected([]);
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
    if (locked || !save || points.current || move.current) return;
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
    const gesture = eraseGesture.current;
    if (!gesture || gesture.failed) return;
    try {
      const rendered = surface.current?.getBoundingClientRect();
      const displayedWidth = rendered?.width || width;
      const displayedHeight = rendered?.height || height;
      const eraseWidth =
        rotation % 180 === 0 ? displayedWidth : displayedHeight;
      const baseHeight =
        rotation % 180 === 0 ? displayedHeight : displayedWidth;
      gesture.next = eraseInk(
        gesture.next,
        gesture.last,
        p,
        eraseWidth,
        baseHeight,
        eraserRadius,
        eraserMode,
        200000 -
          notes
            .filter((n) => n.assetId !== assetId || n.page !== page)
            .reduce((sum, n) => sum + (n.ink?.points.length ?? 0), 0),
        2000 -
          notes.filter(
            (n) => n.assetId !== assetId || n.page !== page || !n.ink,
          ).length,
      );
      gesture.last = p;
      setMoving(gesture.next);
    } catch (e) {
      gesture.failed = true;
      gesture.next = rows;
      setMoving(rows);
      setMessage(String(e instanceof Error ? e.message : e));
    }
  };
  const selectStroke = (id: string) => {
    if (locked || move.current || points.current) return;
    if (!active) begin();
    setTool("select");
    setSelected([id]);
    requestAnimationFrame(() => {
      surface.current?.focus({ preventScroll: true });
      surface.current?.scrollIntoView({ block: "nearest" });
    });
  };
  const changeSelected = (color?: string, thickness?: number) => {
    if (locked || move.current || !selectedRows.length) return;
    push(
      rows.map((n) =>
        selected.includes(n.id)
          ? {
              ...n,
              color: color ?? n.color,
              ink: { ...n.ink!, thickness: thickness ?? n.ink!.thickness },
            }
          : n,
      ),
    );
  };
  const deleteSelected = () => {
    if (locked || move.current || !selectedRows.length) return;
    push(rows.filter((n) => !selected.includes(n.id)));
    setSelected([]);
    setMessage("所选笔迹已从草稿删除。您可以撤销这次修改。");
  };
  const cancelMove = () => {
    gesturePointer.current = null;
    move.current = null;
    eraseGesture.current = null;
    setMoving(null);
    drawing.current = false;
  };
  const finish = () => {
    if (eraseGesture.current) {
      if (!eraseGesture.current.failed) push(eraseGesture.current.next);
      cancelMove();
    }
    if (move.current) {
      push(move.current.next);
      cancelMove();
    }
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
          <div
            className="paper-ink-main"
            role="toolbar"
            aria-label="笔迹常用工具"
          >
            <button
              disabled={locked || !!moving || !!livePoints}
              aria-pressed={tool === "pen"}
              onClick={() => setTool("pen")}
            >
              画笔
            </button>
            <button
              disabled={locked || !!moving || !!livePoints}
              aria-pressed={tool === "eraser"}
              onClick={() => setTool("eraser")}
            >
              橡皮
            </button>
            <button
              disabled={locked || !!moving || !!livePoints}
              aria-pressed={tool === "select"}
              onClick={() => {
                setTool("select");
                setSelected([]);
              }}
            >
              选择笔迹
            </button>
            {tool === "eraser" && (
              <>
                <label>
                  擦除
                  <select
                    aria-label="笔迹擦除方式"
                    value={eraserMode}
                    disabled={locked || !!moving}
                    onChange={(e) =>
                      setEraserMode(e.target.value as "whole" | "partial")
                    }
                  >
                    <option value="whole">整笔</option>
                    <option value="partial">局部</option>
                  </select>
                </label>
                <label>
                  大小
                  <select
                    aria-label="橡皮大小"
                    value={eraserRadius}
                    disabled={locked || !!moving}
                    onChange={(e) => setEraserRadius(Number(e.target.value))}
                  >
                    <option value={4}>小</option>
                    <option value={8}>中</option>
                    <option value={16}>大</option>
                  </select>
                </label>
              </>
            )}
            {tool === "select" ? (
              <>
                <label>
                  颜色
                  <select
                    aria-label="所选笔迹颜色"
                    disabled={locked || !!moving || !selectedRows.length}
                    value={
                      selectedRows.length &&
                      selectedRows.every(
                        (n) => n.color === selectedRows[0].color,
                      )
                        ? selectedRows[0].color
                        : ""
                    }
                    onChange={(e) => changeSelected(e.target.value)}
                  >
                    <option value="" disabled>
                      {selectedRows.length ? "多种颜色" : "未选择"}
                    </option>
                    <option value="yellow">黄色</option>
                    <option value="blue">蓝色</option>
                    <option value="green">绿色</option>
                    <option value="red">红色</option>
                  </select>
                </label>
                <label>
                  粗细
                  <select
                    aria-label="所选笔迹粗细"
                    disabled={locked || !!moving || !selectedRows.length}
                    value={
                      selectedRows.length &&
                      selectedRows.every(
                        (n) =>
                          n.ink!.thickness === selectedRows[0].ink!.thickness,
                      )
                        ? selectedRows[0].ink!.thickness
                        : ""
                    }
                    onChange={(e) =>
                      changeSelected(undefined, Number(e.target.value))
                    }
                  >
                    <option value="" disabled>
                      {selectedRows.length ? "多种粗细" : "未选择"}
                    </option>
                    <option value={0.001}>细</option>
                    <option value={0.0022}>中</option>
                    <option value={0.004}>粗</option>
                    <option value={0.007}>加粗</option>
                  </select>
                </label>
              </>
            ) : tool === "pen" ? (
              <>
                <label>
                  颜色
                  <select
                    aria-label="笔迹颜色"
                    disabled={locked || !!livePoints}
                    value={color}
                    onChange={(e) => setColor(e.target.value)}
                  >
                    <option value="yellow">黄色</option>
                    <option value="blue">蓝色</option>
                    <option value="green">绿色</option>
                    <option value="red">红色</option>
                  </select>
                </label>
                <label>
                  粗细
                  <select
                    aria-label="笔迹粗细"
                    disabled={locked || !!livePoints}
                    value={thickness}
                    onChange={(e) => setThickness(Number(e.target.value))}
                  >
                    <option value={0.001}>细</option>
                    <option value={0.0022}>中</option>
                    <option value={0.004}>粗</option>
                    <option value={0.007}>加粗</option>
                  </select>
                </label>
              </>
            ) : null}
            <button
              disabled={locked || !!moving || !!livePoints || !history.length}
              onClick={() => {
                setMessage("");
                setFuture((f) => [rows, ...f]);
                setRows(history.at(-1)!);
                setHistory((h) => h.slice(0, -1));
              }}
            >
              撤销笔画
            </button>
            <button
              disabled={locked || !changed || !!moving || !!livePoints}
              onClick={() => void commit()}
            >
              保存笔迹
            </button>
          </div>
          <div
            className="paper-ink-secondary"
            role="toolbar"
            aria-label="笔迹其他操作"
          >
            {tool === "select" && (
              <>
                <span role="status">已选 {selectedRows.length} 笔</span>
                <button
                  disabled={locked || !!moving || !rows.length}
                  onClick={() => setSelected(rows.map((n) => n.id))}
                >
                  全选本页笔迹
                </button>
                <button
                  disabled={locked || !!moving || !selectedRows.length}
                  onClick={() => setSelected([])}
                >
                  取消选择
                </button>
                <button
                  disabled={locked || !!moving || !selectedRows.length}
                  onClick={deleteSelected}
                >
                  删除所选笔迹
                </button>
              </>
            )}
            <button
              disabled={locked || !!moving || !!livePoints || !future.length}
              onClick={() => {
                setMessage("");
                setHistory((h) => [...h, rows]);
                setRows(future[0]);
                setFuture((f) => f.slice(1));
              }}
            >
              重做笔画
            </button>
            <button
              disabled={locked || !!moving || !!livePoints}
              onClick={() => void recheck()}
            >
              核对已保存笔迹
            </button>
            <button
              disabled={locked || !!moving || !!livePoints}
              onClick={() => {
                setActive(false);
                setMessage(changed ? "手写草稿已暂存。" : "已退出手写。");
              }}
            >
              暂存并退出手写
            </button>
            <button
              disabled={locked || !!moving || !!livePoints}
              onClick={discard}
            >
              放弃笔迹修改
            </button>
            <span
              className="paper-ink-help"
              title="您可以拖动所选笔迹。Shift 点击可多选，方向键可微调，Delete 可删除，Esc 可取消。Ctrl+Z 可撤销，Ctrl+Y 可重做。"
            >
              {rows.length} 笔 ·{" "}
              {tool === "pen"
                ? "在谱页上书写"
                : tool === "select"
                  ? "拖动可移动 · Shift 可多选"
                  : eraserMode === "whole"
                    ? "点击或划过整笔擦除"
                    : "拖动橡皮擦除经过的部分"}
            </span>
          </div>
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
      tabIndex={active ? 0 : undefined}
      style={{
        cursor:
          active && tool === "select"
            ? moving
              ? "grabbing"
              : "default"
            : undefined,
      }}
      onKeyDown={(e) => {
        if (!active || locked) return;
        if (e.key === "Escape") {
          e.preventDefault();
          e.stopPropagation();
          if (
            move.current ||
            eraseGesture.current ||
            points.current ||
            drawing.current
          ) {
            cancelMove();
            points.current = null;
            setLivePoints(null);
            setMessage("当前操作已取消。其他笔迹保留。");
          } else setSelected([]);
          return;
        }
        if (move.current || points.current) return;
        if (
          (e.ctrlKey || e.metaKey) &&
          ["z", "y"].includes(e.key.toLowerCase())
        ) {
          e.preventDefault();
          e.stopPropagation();
          if ((e.key.toLowerCase() === "y" || e.shiftKey) && future.length) {
            setHistory((h) => [...h, rows]);
            setRows(future[0]);
            setFuture((f) => f.slice(1));
          } else if (
            !e.shiftKey &&
            e.key.toLowerCase() === "z" &&
            history.length
          ) {
            setFuture((f) => [rows, ...f]);
            setRows(history.at(-1)!);
            setHistory((h) => h.slice(0, -1));
          }
          return;
        }
        if (tool !== "select") return;
        const directions: Record<string, Point> = {
          ArrowLeft: [-1, 0],
          ArrowRight: [1, 0],
          ArrowUp: [0, -1],
          ArrowDown: [0, 1],
        };
        if (e.key === "Delete" || e.key === "Backspace") {
          e.preventDefault();
          e.stopPropagation();
          deleteSelected();
        } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
          e.preventDefault();
          e.stopPropagation();
          setSelected(rows.map((n) => n.id));
        } else if (directions[e.key] && selectedRows.length) {
          e.preventDefault();
          e.stopPropagation();
          const d = directions[e.key],
            step = e.shiftKey ? 10 : 1;
          const a = paperPoint(0.5, 0.5, (360 - rotation) % 360);
          const b = paperPoint(
            0.5 + (d[0] * step) / width,
            0.5 + (d[1] * step) / height,
            (360 - rotation) % 360,
          );
          push(moveInk(rows, selected, b[0] - a[0], b[1] - a[1]));
        }
      }}
      onPointerDown={(e) => {
        if (
          !active ||
          locked ||
          e.button !== 0 ||
          gesturePointer.current !== null
        )
          return;
        gesturePointer.current = e.pointerId;
        e.preventDefault();
        e.stopPropagation();
        e.currentTarget.setPointerCapture(e.pointerId);
        drawing.current = true;
        e.currentTarget.focus({ preventScroll: true });
        const p = position(e);
        if (tool === "select") {
          const point = pixel(p);
          const hit = [...rows].reverse().find((n) => {
            const ps = n.ink!.points.map((v) =>
              pixel([n.x + v[0] * n.width!, n.y + v[1] * n.height!]),
            );
            return ps.some(
              (v, i) =>
                i > 0 &&
                segmentDistance(point, ps[i - 1], v) <=
                  Math.max(8, (n.ink!.thickness * baseWidth) / 2 + 4),
            );
          });
          if (!hit) {
            if (!e.shiftKey) setSelected([]);
            drawing.current = false;
            gesturePointer.current = null;
            e.currentTarget.releasePointerCapture(e.pointerId);
            return;
          }
          const ids = e.shiftKey
            ? selected.includes(hit.id)
              ? selected.filter((id) => id !== hit.id)
              : [...selected, hit.id]
            : selected.includes(hit.id)
              ? selected
              : [hit.id];
          setSelected(ids);
          if (ids.includes(hit.id)) {
            move.current = { start: p, rows, ids, next: rows };
            setMoving(rows);
          } else {
            drawing.current = false;
            gesturePointer.current = null;
            e.currentTarget.releasePointerCapture(e.pointerId);
          }
          return;
        }
        if (tool === "eraser") {
          setEraserCursor(p);
          eraseGesture.current = { next: rows, last: p, failed: false };
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
      onPointerLeave={() => setEraserCursor(null)}
      onPointerMove={(e) => {
        if (active && !locked && tool === "eraser")
          setEraserCursor(position(e));
        if (
          !active ||
          locked ||
          !drawing.current ||
          gesturePointer.current !== e.pointerId
        )
          return;
        e.preventDefault();
        e.stopPropagation();
        const p = position(e);
        if (tool === "select") {
          if (move.current) {
            move.current.next = moveInk(
              move.current.rows,
              move.current.ids,
              p[0] - move.current.start[0],
              p[1] - move.current.start[1],
            );
            setMoving(move.current.next);
          }
          return;
        }
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
        if (!drawing.current || gesturePointer.current !== e.pointerId) return;
        gesturePointer.current = null;
        e.preventDefault();
        e.stopPropagation();
        if (tool === "eraser") erase(position(e));
        finish();
        if (e.currentTarget.hasPointerCapture(e.pointerId))
          e.currentTarget.releasePointerCapture(e.pointerId);
      }}
      onLostPointerCapture={() => {
        if (drawing.current) {
          cancelMove();
          points.current = null;
          setLivePoints(null);
          setMessage("当前操作已取消。其他笔迹保留。");
        }
      }}
      onPointerCancel={() => {
        cancelMove();
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
        ? (moving ?? rows)
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
      {active &&
        tool === "select" &&
        (moving ?? rows)
          .filter((n) => selected.includes(n.id))
          .map((n) => {
            const a = pixel([n.x, n.y]),
              b = pixel([n.x + n.width!, n.y + n.height!]);
            return (
              <rect
                key={`selected-${n.id}`}
                data-selected-ink-id={n.id}
                x={Math.min(a[0], b[0]) - 5}
                y={Math.min(a[1], b[1]) - 5}
                width={Math.abs(a[0] - b[0]) + 10}
                height={Math.abs(a[1] - b[1]) + 10}
                fill="none"
                stroke="#1674cf"
                strokeWidth={1.5}
                strokeDasharray="5 3"
                pointerEvents="none"
              />
            );
          })}
      {active && tool === "eraser" && eraserCursor && (
        <circle
          cx={pixel(eraserCursor)[0]}
          cy={pixel(eraserCursor)[1]}
          r={
            (eraserRadius * width) /
            (surface.current?.getBoundingClientRect().width || width)
          }
          fill="#ffffff70"
          stroke="#52677b"
          strokeWidth={1}
          pointerEvents="none"
        />
      )}
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
  return { tools, overlay, active, selectStroke };
}
