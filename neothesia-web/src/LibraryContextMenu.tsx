import { useEffect, useLayoutEffect, useRef, useState } from "react";
export type LibraryMenuAction = {
  label: string;
  hint?: string;
  disabled?: boolean;
  run: () => void;
};
export function LibraryContextMenu({
  x,
  y,
  title,
  actions,
  close,
}: {
  x: number;
  y: number;
  title: string;
  actions: LibraryMenuAction[];
  close: (restore: boolean) => void;
}) {
  const box = useRef<HTMLDivElement>(null),
    [position, setPosition] = useState({ x, y });
  useLayoutEffect(() => {
    const rect = box.current!.getBoundingClientRect();
    setPosition({
      x: Math.max(8, Math.min(x, window.innerWidth - rect.width - 8)),
      y: Math.max(8, Math.min(y, window.innerHeight - rect.height - 8)),
    });
    box.current
      ?.querySelector<HTMLButtonElement>("button:not(:disabled)")
      ?.focus();
  }, [x, y]);
  useEffect(() => {
    const cancel = () => close(false);
    const outside = (e: PointerEvent) => {
      if (!box.current?.contains(e.target as Node)) close(false);
    };
    const key = (e: KeyboardEvent) => {
      const buttons = [
        ...box.current!.querySelectorAll<HTMLButtonElement>(
          "button:not(:disabled)",
        ),
      ];
      const index = buttons.indexOf(
        document.activeElement as HTMLButtonElement,
      );
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopImmediatePropagation();
        close(true);
      } else if (["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) {
        e.preventDefault();
        e.stopImmediatePropagation();
        const next =
          e.key === "Home"
            ? 0
            : e.key === "End"
              ? buttons.length - 1
              : (index + (e.key === "ArrowDown" ? 1 : -1) + buttons.length) %
                buttons.length;
        buttons[next]?.focus();
      } else if (e.key === "Tab") {
        close(false);
      }
    };
    window.addEventListener("keydown", key, true);
    window.addEventListener("resize", cancel);
    window.addEventListener("blur", cancel);
    document.addEventListener("pointerdown", outside, true);
    return () => {
      window.removeEventListener("keydown", key, true);
      window.removeEventListener("resize", cancel);
      window.removeEventListener("blur", cancel);
      document.removeEventListener("pointerdown", outside, true);
    };
  }, [close]);
  return (
    <div
      ref={box}
      className="library-context-menu"
      role="menu"
      aria-label="曲目操作"
      style={{ left: position.x, top: position.y }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <div className="library-context-title" title={title}>
        {title}
      </div>
      {actions.map((action, index) => (
        <button
          key={index}
          type="button"
          role="menuitem"
          disabled={action.disabled}
          onClick={() => {
            close(true);
            action.run();
          }}
        >
          <span>{action.label}</span>
          {action.hint && <small>{action.hint}</small>}
        </button>
      ))}
    </div>
  );
}
export async function copyLibraryPaths(paths: string[]) {
  const text = paths.join("\r\n");
  try {
    await navigator.clipboard.writeText(text);
    return;
  } catch {}
  const previous = document.activeElement as HTMLElement | null,
    field = document.createElement("textarea");
  field.value = text;
  field.style.cssText = "position:fixed;opacity:0;pointer-events:none";
  document.body.append(field);
  field.focus();
  field.select();
  const copied = document.execCommand("copy");
  field.remove();
  previous?.focus();
  if (!copied) throw new Error("无法复制文件位置，请在详情中选择路径复制。");
}
