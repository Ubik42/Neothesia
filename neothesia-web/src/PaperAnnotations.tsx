import { usePaperInk } from "./PaperInk";
import { paperPoint as rotate, paperAnnotationBox } from "./PaperGeometry";
export { paperAnnotationBox } from "./PaperGeometry";
import { useEffect, useRef, useState, type ReactNode } from "react";

export interface PaperAnnotation {
  id: string;
  assetId: string;
  page: number;
  x: number;
  y: number;
  text: string;
  color: string;
  width?: number;
  height?: number;
  ink?: { points: [number, number][]; thickness: number };
}
export type PaperAnnotationDraft = Omit<PaperAnnotation, "id">;
export type PaperRegion = {
  x: number;
  y: number;
  width: number;
  height: number;
};
type Edit = {
  id: string | null;
  expected: PaperAnnotation | null;
  draft: PaperAnnotationDraft;
};
export function PaperAnnotations({
  contentId,
  book,
  assetId,
  page,
  rotation,
  width,
  height,
  notes,
  disabled,
  editing,
  save,
  children,
  regionSelection = false,
  regionSelected,
  saveInk,
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
  editing: (v: boolean) => void;
  save: (
    id: string | null,
    draft: PaperAnnotationDraft | null,
    expected: PaperAnnotation | null,
  ) => Promise<boolean>;
  children: ReactNode;
  regionSelection?: boolean;
  regionSelected?: (region: PaperRegion) => void;
  saveInk?: (
    expected: PaperAnnotation[],
    strokes: PaperAnnotation[],
  ) => Promise<boolean>;
}) {
  const key = `neothesia-paper-draft:${contentId}:${book}:${assetId}:${page}`;
  const [edit, setEdit] = useState<Edit | null>(() => {
    try {
      const v = JSON.parse(localStorage.getItem(key) || "null") as Edit | null;
      const valid =
        v &&
        v.draft.assetId === assetId &&
        v.draft.page === page &&
        typeof v.draft.text === "string" &&
        Number.isFinite(v.draft.x) &&
        Number.isFinite(v.draft.y)
          ? v
          : null;
      if (valid && !valid.id) {
        const existing = notes.find(
          (n) =>
            n.assetId === assetId &&
            n.page === page &&
            Math.abs(n.x - valid.draft.x) < 0.000001 &&
            Math.abs(n.y - valid.draft.y) < 0.000001 &&
            Math.abs((n.width ?? 0) - (valid.draft.width ?? 0)) < 0.000001 &&
            Math.abs((n.height ?? 0) - (valid.draft.height ?? 0)) < 0.000001 &&
            n.text === valid.draft.text.trim() &&
            n.color === valid.draft.color,
        );
        if (existing) {
          valid.id = existing.id;
          valid.expected = existing;
        }
      }
      return valid;
    } catch {
      return null;
    }
  });
  const [placing, setPlacing] = useState<boolean | "highlight">(false),
    [show, setShow] = useState(true),
    [message, setMessage] = useState("");
  const drag = useRef<{ x: number; y: number } | null>(null),
    [box, setBox] = useState<{
      x: number;
      y: number;
      width: number;
      height: number;
    } | null>(null);
  const position = (e: React.PointerEvent<HTMLDivElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    return {
      x: Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)),
      y: Math.max(0, Math.min(1, (e.clientY - r.top) / r.height)),
    };
  };
  const ink = usePaperInk({
    contentId,
    book,
    assetId,
    page,
    rotation,
    width,
    height,
    notes,
    disabled,
    blocked: !!edit || !!placing || regionSelection,
    save: saveInk,
    visible: show,
    preview: edit?.draft.ink
      ? { ...edit.draft, id: edit.id ?? "ink-preview" }
      : null,
  });
  const sheet = useRef<HTMLDivElement>(null);
  useEffect(() => {
    editing(!!edit || !!placing || regionSelection || ink.active);
    return () => editing(false);
  }, [!!edit, placing, regionSelection, ink.active, editing]);
  useEffect(() => {
    try {
      if (edit) localStorage.setItem(key, JSON.stringify(edit));
      else localStorage.removeItem(key);
    } catch {
      setMessage("草稿无法保存在此设备，请保存后再离开。");
    }
  }, [edit, key]);
  const current = notes.filter((n) => n.assetId === assetId && n.page === page);
  const clear = () => {
    try {
      localStorage.removeItem(key);
    } catch {}
    setEdit(null);
    setPlacing(false);
    setMessage("");
  };
  const choose = (note: PaperAnnotation) => {
    if (edit || placing || ink.active) {
      setMessage("请先保存或取消当前批注。");
      return;
    }
    setEdit({ id: note.id, expected: note, draft: { ...note } });
  };
  return (
    <>
      <div className="paper-annotation-tools">
        {ink.tools}
        <button
          disabled={
            disabled || !!edit || !!placing || regionSelection || ink.active
          }
          onClick={() => {
            setPlacing(true);
            setShow(true);
            setMessage("");
          }}
        >
          添加批注
        </button>
        <button
          disabled={
            disabled || !!edit || !!placing || regionSelection || ink.active
          }
          onClick={() => {
            setPlacing("highlight");
            setShow(true);
            setMessage("");
          }}
        >
          添加高亮区域
        </button>
        <button aria-pressed={show} onClick={() => setShow((v) => !v)}>
          {show ? "隐藏批注" : "显示批注"}
          {current.length ? ` · ${current.length}` : ""}
        </button>
        {placing && (
          <span role="status">
            {placing === "highlight"
              ? "在谱页上拖出要标记的区域。"
              : "点击谱页上的位置。"}
          </span>
        )}
        {(edit || placing) && (
          <button disabled={disabled} onClick={clear}>
            取消批注编辑
          </button>
        )}
        {message && <span role="status">{message}</span>}
      </div>
      {edit && (
        <div className="paper-annotation-editor">
          <label>
            批注内容{" "}
            <textarea
              autoFocus
              aria-label="谱页批注内容"
              maxLength={400}
              rows={2}
              value={edit.draft.text}
              disabled={disabled}
              onChange={(e) =>
                setEdit({
                  ...edit,
                  draft: { ...edit.draft, text: e.target.value },
                })
              }
            />
          </label>
          <label>
            颜色{" "}
            <select
              aria-label="谱页批注颜色"
              value={edit.draft.color}
              disabled={disabled}
              onChange={(e) =>
                setEdit({
                  ...edit,
                  draft: { ...edit.draft, color: e.target.value },
                })
              }
            >
              <option value="yellow">黄色</option>
              <option value="blue">蓝色</option>
              <option value="green">绿色</option>
              <option value="red">红色</option>
            </select>
          </label>
          <button
            disabled={disabled}
            onClick={() => setPlacing(edit.draft.width ? "highlight" : true)}
          >
            重新定位
          </button>
          <button
            disabled={
              disabled || (!edit.draft.text.trim() && !edit.draft.width)
            }
            onClick={() =>
              void save(edit.id, edit.draft, edit.expected).then((ok) => {
                if (ok) clear();
              })
            }
          >
            保存批注
          </button>
          {edit.id && (
            <button
              disabled={disabled}
              onClick={() =>
                void save(edit.id, null, edit.expected).then((ok) => {
                  if (ok) clear();
                })
              }
            >
              删除此批注
            </button>
          )}
          <small>尚未保存的内容会保留为本机草稿。</small>
        </div>
      )}
      {current.length > 0 && (
        <details className="paper-annotation-list">
          <summary>本页批注 · {current.length}</summary>
          {current.map((n, i) => (
            <button key={n.id} disabled={disabled} onClick={() => choose(n)}>
              {i + 1}. {n.ink ? "笔迹 · " : n.width ? "高亮 · " : ""}
              {n.text || (n.ink ? "手写标记" : "未填写说明")}
            </button>
          ))}
        </details>
      )}
      <div
        ref={sheet}
        className={`paper-annotation-sheet ${placing || regionSelection ? "placing" : ""}`}
        style={{ width, height }}
        onPointerDown={(e) => {
          if (
            (placing !== "highlight" && !regionSelection) ||
            disabled ||
            e.button !== 0
          )
            return;
          e.preventDefault();
          drag.current = position(e);
          setBox({ ...drag.current, width: 0, height: 0 });
          e.currentTarget.setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => {
          if (!drag.current) return;
          const p = position(e);
          setBox({
            x: Math.min(p.x, drag.current.x),
            y: Math.min(p.y, drag.current.y),
            width: Math.abs(p.x - drag.current.x),
            height: Math.abs(p.y - drag.current.y),
          });
        }}
        onPointerCancel={() => {
          drag.current = null;
          setBox(null);
        }}
        onPointerUp={(e) => {
          if (!drag.current) return;
          const p = position(e),
            a = rotate(drag.current.x, drag.current.y, (360 - rotation) % 360),
            b = rotate(p.x, p.y, (360 - rotation) % 360),
            rect = {
              x: Math.min(a[0], b[0]),
              y: Math.min(a[1], b[1]),
              width: Math.abs(a[0] - b[0]),
              height: Math.abs(a[1] - b[1]),
            };
          drag.current = null;
          setBox(null);
          if (e.currentTarget.hasPointerCapture(e.pointerId))
            e.currentTarget.releasePointerCapture(e.pointerId);
          if (rect.width < 0.002 || rect.height < 0.002) {
            setMessage("区域太小，请重新拖选。");
            return;
          }
          if (regionSelection) {
            regionSelected?.(rect);
            return;
          }
          setEdit(
            edit
              ? { ...edit, draft: { ...edit.draft, ...rect } }
              : {
                  id: null,
                  expected: null,
                  draft: { assetId, page, ...rect, text: "", color: "yellow" },
                },
          );
          setPlacing(false);
        }}
        onClick={(e) => {
          if (
            regionSelection ||
            !placing ||
            placing === "highlight" ||
            disabled ||
            !sheet.current
          )
            return;
          const rect = sheet.current.getBoundingClientRect();
          const x = Math.max(
              0,
              Math.min(1, (e.clientX - rect.left) / rect.width),
            ),
            y = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));
          const [cx, cy] = rotate(x, y, (360 - rotation) % 360);
          setEdit(
            edit
              ? { ...edit, draft: { ...edit.draft, x: cx, y: cy } }
              : {
                  id: null,
                  expected: null,
                  draft: {
                    assetId,
                    page,
                    x: cx,
                    y: cy,
                    text: "",
                    color: "yellow",
                  },
                },
          );
          setPlacing(false);
        }}
      >
        {children}
        {ink.overlay}
        {show &&
          current.map((n, i) => {
            if (n.ink) return null;
            const rect = paperAnnotationBox(n, rotation);
            const [x, y] = [rect.x, rect.y];
            return (
              <button
                key={n.id}
                className={n.width ? "paper-highlight" : "paper-note-pin"}
                data-color={n.color}
                style={{
                  left: `${x * 100}%`,
                  top: `${y * 100}%`,
                  ...(n.width
                    ? {
                        width: `${rect.width * 100}%`,
                        height: `${rect.height * 100}%`,
                      }
                    : {}),
                }}
                title={n.text || (n.width ? "高亮区域" : "批注")}
                aria-label={`批注 ${i + 1}：${n.text || (n.width ? "高亮区域" : "未填写说明")}`}
                disabled={disabled}
                onClick={(e) => {
                  e.stopPropagation();
                  if (!placing && !regionSelection) choose(n);
                }}
              >
                {i + 1}
              </button>
            );
          })}
        {box && (
          <span
            className="paper-highlight draft"
            style={{
              left: `${box.x * 100}%`,
              top: `${box.y * 100}%`,
              width: `${box.width * 100}%`,
              height: `${box.height * 100}%`,
            }}
          />
        )}
        {edit &&
          (() => {
            const rect = paperAnnotationBox(edit.draft, rotation);
            const [x, y] = [rect.x, rect.y];
            return (
              <span
                className={
                  edit.draft.width
                    ? "paper-highlight draft"
                    : "paper-note-pin draft"
                }
                data-color={edit.draft.color}
                style={{
                  left: `${x * 100}%`,
                  top: `${y * 100}%`,
                  ...(edit.draft.width
                    ? {
                        width: `${rect.width * 100}%`,
                        height: `${rect.height * 100}%`,
                      }
                    : {}),
                }}
              >
                ＋
              </span>
            );
          })()}
      </div>
    </>
  );
}
