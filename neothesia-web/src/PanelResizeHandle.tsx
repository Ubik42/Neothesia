import { useRef } from "react";
export function PanelResizeHandle({
  label,
  value,
  min,
  max,
  reverse = false,
  change,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  reverse?: boolean;
  change: (value: number) => void;
}) {
  const drag = useRef<{ x: number; width: number; scale: number } | null>(null);
  const update = (v: number) =>
    change(Math.min(max, Math.max(min, Math.round(v))));
  return (
    <div
      className={`panel-resize-handle ${reverse ? "resize-left" : "resize-right"}`}
      role="separator"
      aria-label={label}
      aria-orientation="vertical"
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      tabIndex={0}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        drag.current = {
          x: e.clientX,
          width: e.currentTarget.parentElement!.offsetWidth,
          scale:
            e.currentTarget.parentElement!.getBoundingClientRect().width /
            Math.max(1, e.currentTarget.parentElement!.offsetWidth),
        };
        e.currentTarget.setPointerCapture(e.pointerId);
      }}
      onPointerMove={(e) => {
        if (drag.current)
          update(
            drag.current.width +
              ((e.clientX - drag.current.x) / drag.current.scale) *
                (reverse ? -1 : 1),
          );
      }}
      onPointerUp={(e) => {
        drag.current = null;
        if (e.currentTarget.hasPointerCapture(e.pointerId))
          e.currentTarget.releasePointerCapture(e.pointerId);
      }}
      onPointerCancel={() => {
        drag.current = null;
      }}
      onKeyDown={(e) => {
        if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) {
          e.preventDefault();
          e.stopPropagation();
          update(
            e.key === "Home"
              ? min
              : e.key === "End"
                ? max
                : value +
                  (e.key === "ArrowRight" ? 1 : -1) *
                    (reverse ? -1 : 1) *
                    (e.shiftKey ? 40 : 10),
          );
        }
      }}
    />
  );
}
