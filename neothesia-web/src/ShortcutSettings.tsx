import { useEffect, useState } from "react";
import {
  shortcutActions,
  shortcutDefaults,
  eventShortcut,
  shortcutError,
  shortcutLabel,
  type ShortcutAction,
} from "./shortcuts";
import type { DisplayPreferences } from "./displayPreferences";
export function ShortcutSettings({
  value,
  change,
  pianoCapture,
  clearPiano,
  reportCapture,
}: {
  value: DisplayPreferences;
  change: (v: DisplayPreferences) => void;
  pianoCapture: number | null;
  clearPiano: () => void;
  reportCapture: (active: boolean) => void;
}) {
  const [capture, setCaptureState] = useState<ShortcutAction | null>(null),
    [error, setError] = useState("");
  const setCapture = (next: ShortcutAction | null) => {
    setCaptureState(next);
    reportCapture(next !== null);
  };
  useEffect(() => () => reportCapture(false), []);
  useEffect(() => {
    if (pianoCapture !== null) setCapture(null);
  }, [pianoCapture]);
  useEffect(() => {
    if (!capture) return;
    const down = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (e.key === "Escape") {
        setCapture(null);
        setError("");
        return;
      }
      if (e.repeat || ["Control", "Alt", "Shift", "Meta"].includes(e.key))
        return;
      const chord = eventShortcut(e);
      const message = chord
        ? shortcutError(chord, value.keyCodes)
        : "请使用 Ctrl、Alt 或 Shift，不使用系统键。";
      if (message) {
        setError(message);
        return;
      }
      const other = shortcutActions.find(
        (a) => a.id !== capture && value.shortcuts[a.id] === chord,
      );
      if (other) {
        setError(
          `这个组合已用于「${other.label}」，请先清除原绑定或选择其他按键。`,
        );
        return;
      }
      change({ ...value, shortcuts: { ...value.shortcuts, [capture]: chord } });
      setCapture(null);
      setError("");
    };
    window.addEventListener("keydown", down, true);
    return () => window.removeEventListener("keydown", down, true);
  }, [capture, value, change]);
  return (
    <section className="shortcut-settings" aria-label="操作快捷键">
      <h3>操作快捷键</h3>
      <p className="parameter-help">
        点击按键录入新组合，Esc
        取消。输入文字和打开窗口时暂停全局操作；电脑键盘弹奏键单独设置。
      </p>
      <div className="shortcut-list">
        {shortcutActions.map((a) => (
          <div key={a.id}>
            <span>{a.label}</span>
            <button
              aria-label={`修改快捷键：${a.label}`}
              aria-pressed={capture === a.id}
              onClick={() => {
                clearPiano();
                setCapture(a.id);
                setError("");
              }}
            >
              {capture === a.id
                ? "请按组合键…"
                : shortcutLabel(value.shortcuts[a.id])}
            </button>
            <button
              aria-label={`清除快捷键：${a.label}`}
              disabled={!value.shortcuts[a.id]}
              onClick={() => {
                change({
                  ...value,
                  shortcuts: { ...value.shortcuts, [a.id]: null },
                });
                setCapture(null);
                setError("");
              }}
            >
              清除
            </button>
          </div>
        ))}
      </div>
      <button
        onClick={() => {
          change({ ...value, shortcuts: { ...shortcutDefaults } });
          setCapture(null);
          setError("");
        }}
      >
        恢复操作快捷键默认值
      </button>
      <p role="status">
        {capture
          ? `正在设置「${shortcutActions.find((a) => a.id === capture)?.label}」`
          : "操作快捷键自动保存在本机"}
      </p>
      {error && (
        <p role="alert" className="dialog-error">
          {error}
        </p>
      )}
    </section>
  );
}
