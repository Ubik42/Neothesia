import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import type { PaperAnnotation } from "./PaperAnnotations";
import { applyInkMerge, type InkReview } from "./paperInkMerge";
const colors: Record<string, string> = {
  red: "#ba5050",
  yellow: "#c69a20",
  blue: "#3679b9",
  green: "#35845d",
};
function InkPreview({ note }: { note: PaperAnnotation | null }) {
  if (!note) return <p>已擦除此笔</p>;
  const w = note.width!,
    h = note.height!,
    scale = Math.min(160 / w, 80 / h);
  return (
    <>
      <svg
        width="180"
        height="100"
        viewBox="0 0 180 100"
        aria-label="笔迹形状预览"
      >
        <path
          d={note
            .ink!.points.map(
              (p, i) =>
                `${i ? "L" : "M"}${10 + p[0] * w * scale} ${10 + p[1] * h * scale}`,
            )
            .join(" ")}
          fill="none"
          stroke={colors[note.color]}
          strokeWidth={Math.max(1, Math.min(6, note.ink!.thickness * scale))}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
      {note.text && <p>{note.text}</p>}
    </>
  );
}
export function InkMergeReview({
  review,
  apply,
  close,
}: {
  review: InkReview;
  apply: (strokes: PaperAnnotation[]) => void;
  close: () => void;
}) {
  const [page, setPage] = useState(0);
  const [choices, setChoices] = useState<Record<string, "saved" | "local">>({}),
    root = useRef<HTMLElement>(null),
    prior = useRef(document.activeElement),
    closeRef = useRef(close);
  closeRef.current = close;
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopImmediatePropagation();
        closeRef.current();
      }
      if (e.key === "Tab") {
        const nodes = [
            ...(root.current?.querySelectorAll<HTMLElement>("button,input") ??
              []),
          ],
          first = nodes[0],
          last = nodes.at(-1);
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last?.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first?.focus();
        }
      }
    };
    window.addEventListener("keydown", key, true);
    return () => {
      window.removeEventListener("keydown", key, true);
      if (prior.current instanceof HTMLElement && prior.current.isConnected)
        prior.current.focus();
    };
  }, []);
  return (
    <div className="modal-backdrop">
      <section
        ref={root}
        className="settings-dialog ink-merge-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="ink-merge-title"
        onKeyDown={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="ink-merge-title">核对笔迹更改</h2>
            <p>
              已保存 {review.current.length} 笔 · 本机草稿合并后{" "}
              {applyInkMerge(review, choices).length} 笔
            </p>
          </div>
          <button autoFocus aria-label="关闭笔迹核对" onClick={close}>
            <X size={18} />
          </button>
        </div>
        <p>
          未冲突的新增、修改和擦除会合并。
          {review.conflicts.length
            ? `${review.conflicts.length} 笔有不同改动，请逐笔选择；默认保留已保存版本。`
            : "当前没有同笔冲突，可将更改合并回草稿。"}
          合并后先查看谱页，再保存。
        </p>
        {review.conflicts.length > 10 && (
          <div className="dialog-actions">
            <button disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
              上一页笔迹冲突
            </button>
            <span>
              {page + 1} / {Math.ceil(review.conflicts.length / 10)} 页
            </span>
            <button
              disabled={(page + 1) * 10 >= review.conflicts.length}
              onClick={() => setPage((p) => p + 1)}
            >
              下一页笔迹冲突
            </button>
          </div>
        )}
        <div className="ink-merge-conflicts">
          {review.conflicts.slice(page * 10, (page + 1) * 10).map((c, i) => (
            <fieldset key={c.id}>
              <legend>笔迹冲突 {page * 10 + i + 1}</legend>
              <div className="ink-merge-options">
                {(["saved", "local"] as const).map((kind) => (
                  <label key={kind}>
                    <input
                      type="radio"
                      name={`ink-conflict-${c.id}`}
                      aria-label={`${kind === "saved" ? "保留已保存" : "采用本机草稿"}笔迹 ${page * 10 + i + 1}`}
                      checked={(choices[c.id] ?? "saved") === kind}
                      onChange={() => setChoices({ ...choices, [c.id]: kind })}
                    />
                    {kind === "saved" ? "已保存版本" : "本机草稿"}
                    <InkPreview note={kind === "saved" ? c.saved : c.local} />
                  </label>
                ))}
              </div>
            </fieldset>
          ))}
        </div>
        <div className="dialog-actions">
          <button onClick={close}>返回手写草稿</button>
          <button
            className="primary"
            onClick={() => apply(applyInkMerge(review, choices))}
          >
            合并到草稿
          </button>
        </div>
      </section>
    </div>
  );
}
