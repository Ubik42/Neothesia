import { InterfaceBackupSettings } from "./InterfaceBackupSettings";
import { ShortcutSettings } from "./ShortcutSettings";
import { LayoutSettings } from "./LayoutSettings";
import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { pitchName } from "./api";
import {
  asdfCodes,
  zxcvCodes,
  allowedCode,
  codeLabel,
  displayDefaults,
  type DisplayPreferences,
} from "./displayPreferences";
export function DisplaySettingsDialog({
  value,
  change,
  close,
}: {
  value: DisplayPreferences;
  change: (value: DisplayPreferences) => void;
  close: () => void;
}) {
  const shortcutCapturing = useRef(false);
  const root = useRef<HTMLElement>(null),
    prior = useRef(document.activeElement);
  useEffect(() => {
    return () => {
      if (prior.current instanceof HTMLElement && prior.current.isConnected)
        prior.current.focus();
    };
  }, []);
  const [capture, setCapture] = useState<number | null>(null),
    [error, setError] = useState("");
  const update = (patch: Partial<DisplayPreferences>) => {
    if (
      patch.keyCodes &&
      Object.values(value.shortcuts).some(
        (chord) => chord && patch.keyCodes!.includes(chord),
      )
    ) {
      setError("这些弹奏键与操作快捷键重复，请先修改对应操作快捷键。");
      return;
    }
    change({ ...value, ...patch });
  };
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        if (shortcutCapturing.current) return;
        event.preventDefault();
        event.stopImmediatePropagation();
        if (capture !== null) setCapture(null);
        else close();
        return;
      }
      if (event.key === "Tab" && capture === null) {
        const nodes = Array.from(
          root.current?.querySelectorAll<HTMLElement>(
            'button:not(:disabled),input:not(:disabled),select:not(:disabled),[tabindex="0"]',
          ) ?? [],
        ).filter((n) => n.getClientRects().length > 0);
        const first = nodes[0],
          last = nodes.at(-1);
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first?.focus();
        }
        return;
      }
      if (capture === null) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      if (
        event.repeat ||
        event.isComposing ||
        event.ctrlKey ||
        event.altKey ||
        event.metaKey ||
        !allowedCode(event.code)
      ) {
        setError("请按一个字母、数字或逗号/句号/分号键。");
        return;
      }
      if (
        value.keyCodes.some(
          (code, index) => index !== capture && code === event.code,
        )
      ) {
        setError("这个键已经分配给另一个音，请选择未使用的键。");
        return;
      }
      if (Object.values(value.shortcuts).includes(event.code)) {
        setError("这个键已用于操作快捷键，请先修改或清除对应快捷键。");
        return;
      }
      const keyCodes = [...value.keyCodes];
      keyCodes[capture] = event.code;
      update({ keyCodes });
      setCapture(null);
      setError("");
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [value, capture, close]);
  return (
    <div className="modal-backdrop" onClick={close}>
      <section
        ref={root}
        className="settings-dialog display-settings-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="display-settings-title"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="display-settings-title">显示与电脑键盘</h2>
            <p>立即生效，保存在本机</p>
          </div>
          <button autoFocus aria-label="关闭显示设置" onClick={close}>
            <X size={18} />
          </button>
        </div>
        <div className="display-settings-fields">
          <label>
            音符名称
            <select
              aria-label="音符名称格式"
              value={value.notation}
              onChange={(event) =>
                update({
                  notation: event.target
                    .value as DisplayPreferences["notation"],
                })
              }
            >
              <option value="pitch">音名 · C</option>
              <option value="solfege">固定唱名 · Do</option>
              <option value="degree">首调大调级数 · 1</option>
            </select>
          </label>
          <label>
            音符提示
            <select
              aria-label="音符提示内容"
              value={value.noteLabels}
              onChange={(event) =>
                update({
                  noteLabels: event.target
                    .value as DisplayPreferences["noteLabels"],
                })
              }
            >
              <option value="auto">指法优先，未标指法显示名称</option>
              <option value="name">仅名称</option>
              <option value="finger">仅指法</option>
              <option value="both">指法与名称</option>
              <option value="none">不显示文字</option>
            </select>
          </label>
          <label>
            键上名称
            <select
              aria-label="键上名称范围"
              value={value.keyLabels}
              onChange={(event) =>
                update({
                  keyLabels: event.target
                    .value as DisplayPreferences["keyLabels"],
                })
              }
            >
              <option value="c">仅 C 键</option>
              <option value="white">全部白键</option>
              <option value="all">全部键</option>
              <option value="none">不显示</option>
            </select>
          </label>
          <label>
            级数主音
            <select
              aria-label="级数主音"
              disabled={value.notation !== "degree"}
              value={value.tonic}
              onChange={(event) =>
                update({ tonic: Number(event.target.value) })
              }
            >
              {Array.from({ length: 12 }, (_, index) => (
                <option value={index} key={index}>
                  {pitchName(60 + index).slice(0, -1)}
                </option>
              ))}
            </select>
          </label>
          <label>
            提示字号
            <select
              aria-label="键盘提示字号"
              value={value.fontSize}
              onChange={(event) =>
                update({ fontSize: Number(event.target.value) })
              }
            >
              {[10, 12, 14, 16].map((size) => (
                <option key={size} value={size}>
                  {size}
                </option>
              ))}
            </select>
          </label>
          <label>
            声部配色
            <select
              aria-label="键盘声部配色"
              value={value.palette}
              onChange={(event) =>
                update({
                  palette: event.target.value as DisplayPreferences["palette"],
                })
              }
            >
              <option value="standard">曲目颜色</option>
              <option value="accessible">左右手蓝橙配色</option>
            </select>
          </label>
        </div>
        <div className="display-settings-checks">
          {[
            ["octave", "显示音区数字"],
            ["libraryOpen", "显示左侧曲库"],
            ["inspector", "显示练习参数"],
            ["keyboardEnabled", "启用电脑键盘演奏"],
          ].map(([key, label]) => (
            <label key={key}>
              <input
                type="checkbox"
                checked={Boolean(value[key as keyof DisplayPreferences])}
                onChange={(event) => update({ [key]: event.target.checked })}
              />
              {label}
            </label>
          ))}
        </div>
        <p className="parameter-help">
          固定唱名以 C 为 Do；级数按大调排列，以所选主音为
          1，右下数字表示音区，只改变提示，不移调演奏。窄键上的文字会缩小，放不下时隐藏；指法显示已有标记。
        </p>
        <InterfaceBackupSettings value={value} change={change} />
        <ShortcutSettings
          value={value}
          change={change}
          reportCapture={(active) => {
            shortcutCapturing.current = active;
          }}
          pianoCapture={capture}
          clearPiano={() => setCapture(null)}
        />
        <LayoutSettings value={value} change={change} />
        <section className="computer-key-settings">
          <h3>电脑键盘映射</h3>
          <div className="dialog-actions">
            <label>
              起始音区
              <select
                aria-label="电脑键盘起始音区"
                value={value.keyboardOctave}
                onChange={(event) =>
                  update({ keyboardOctave: Number(event.target.value) })
                }
              >
                {[2, 3, 4, 5, 6].map((octave) => (
                  <option value={octave} key={octave}>
                    C{octave}–C{octave + 1}
                  </option>
                ))}
              </select>
            </label>
            <button
              onClick={() => {
                update({ keyCodes: [...asdfCodes] });
                setCapture(null);
              }}
            >
              A S D 行
            </button>
            <button
              onClick={() => {
                update({ keyCodes: [...zxcvCodes] });
                setCapture(null);
              }}
            >
              Z X C 行
            </button>
          </div>
          <div className="computer-key-grid">
            {value.keyCodes.map((code, index) => (
              <button
                key={index}
                aria-label={`修改 ${pitchName(12 * (value.keyboardOctave + 1) + index)} 的电脑按键`}
                aria-pressed={capture === index}
                onClick={() => {
                  setCapture(index);
                  setError("");
                }}
              >
                <strong>{codeLabel(code)}</strong>
                <small>
                  {pitchName(12 * (value.keyboardOctave + 1) + index)}
                </small>
              </button>
            ))}
          </div>
          <p role="status">
            {capture !== null
              ? `请按要分配给 ${pitchName(12 * (value.keyboardOctave + 1) + capture)} 的键；Esc 取消。`
              : "点击一个按键修改。使用键盘物理位置，支持同时按多个键；输入框、对话框内和组合快捷键不会演奏。"}
          </p>
          {error && (
            <p role="alert" className="dialog-error">
              {error}
            </p>
          )}
        </section>
        <div className="dialog-actions">
          <button
            onClick={() => {
              change({ ...displayDefaults, keyCodes: [...asdfCodes] });
              setCapture(null);
              setError("");
            }}
          >
            恢复显示与按键默认值
          </button>
          <button className="primary-button" onClick={close}>
            完成
          </button>
        </div>
      </section>
    </div>
  );
}
