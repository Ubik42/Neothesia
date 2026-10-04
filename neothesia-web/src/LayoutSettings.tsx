import { useEffect, useState } from "react";
import {
  normalizeDisplayPreferences,
  type DisplayPreferences,
} from "./displayPreferences";
export const layoutKeys = [
  "uiScale",
  "libraryOpen",
  "inspector",
  "libraryWidth",
  "inspectorWidth",
  "keyboardHeight",
  "paperPreviewHeight",
] as const;
export type Layout = Pick<DisplayPreferences, (typeof layoutKeys)[number]>;
export type SavedLayout = { id: string; name: string; layout: Layout };
export const layoutStorage = "neothesia-layouts-v1";
export function extractLayout(value: DisplayPreferences): Layout {
  return Object.fromEntries(layoutKeys.map((k) => [k, value[k]])) as Layout;
}
export function readLayouts(): SavedLayout[] {
  try {
    const data = JSON.parse(localStorage.getItem(layoutStorage) ?? "[]");
    if (!Array.isArray(data)) return [];
    const ids = new Set<string>();
    return data
      .filter(
        (v) =>
          v &&
          typeof v.id === "string" &&
          !ids.has(v.id) &&
          typeof v.name === "string" &&
          v.name.trim().length > 0 &&
          v.name.length <= 40 &&
          v.layout &&
          typeof v.layout === "object" &&
          (ids.add(v.id), true),
      )
      .slice(0, 24)
      .map((v) => ({
        ...v,
        layout: extractLayout(normalizeDisplayPreferences(v.layout)),
      }));
  } catch {
    return [];
  }
}
export function LayoutSettings({
  value,
  change,
}: {
  value: DisplayPreferences;
  change: (v: DisplayPreferences) => void;
}) {
  const [saved, setSaved] = useState(readLayouts),
    [selected, setSelected] = useState(""),
    [name, setName] = useState(""),
    [error, setError] = useState(""),
    [status, setStatus] = useState("");
  useEffect(() => {
    const refresh = () => {
      setSaved(readLayouts());
      setSelected("");
      setName("");
    };
    window.addEventListener("neothesia-layouts-changed", refresh);
    return () =>
      window.removeEventListener("neothesia-layouts-changed", refresh);
  }, []);
  const update = (patch: Partial<DisplayPreferences>) =>
    change({ ...value, ...patch });
  const commit = (rows: SavedLayout[], message: string) => {
    try {
      localStorage.setItem(layoutStorage, JSON.stringify(rows));
      setSaved(rows);
      setStatus(message);
      setError("");
      return true;
    } catch {
      setError("本机存储未能写入，布局方案未保存，请保留当前设置后重试。");
      return false;
    }
  };
  const validName = () => {
    if (!name.trim() || name.trim().length > 40) {
      setError("布局名称需要 1～40 个字。");
      return false;
    }
    if (saved.some((s) => s.name === name.trim() && s.id !== selected)) {
      setError("已有同名布局，请使用不同名称。");
      return false;
    }
    return true;
  };
  return (
    <section className="layout-settings" aria-label="练习室布局">
      <h3>练习室布局</h3>
      <div className="interface-scale-controls">
        <label>
          整体缩放
          <select
            aria-label="整体界面缩放"
            value={value.uiScale}
            onChange={(e) => update({ uiScale: Number(e.target.value) })}
          >
            {Array.from(new Set([80, 90, 100, 110, 125, 150, value.uiScale]))
              .sort((a, b) => a - b)
              .map((n) => (
                <option key={n} value={n}>
                  {n}%
                </option>
              ))}
          </select>
        </label>
        <button onClick={() => update({ uiScale: 100 })}>恢复 100% 缩放</button>
        <button
          onClick={() =>
            update({
              uiScale: Math.min(
                100,
                Math.max(
                  80,
                  Math.round(
                    (Math.min(
                      window.innerWidth / 1100,
                      window.innerHeight / 800,
                    ) *
                      100) /
                      5,
                  ) * 5,
                ),
              ),
            })
          }
        >
          适应当前窗口
        </button>
      </div>
      <div className="layout-builtins">
        {[
          {
            name: "读谱",
            patch: {
              libraryOpen: false,
              inspector: true,
              keyboardHeight: 80,
              paperPreviewHeight: 110,
            },
          },
          {
            name: "选曲",
            patch: { libraryOpen: true, inspector: false, libraryWidth: 360 },
          },
          {
            name: "专注练习",
            patch: {
              libraryOpen: false,
              inspector: false,
              keyboardHeight: 150,
            },
          },
        ].map((p) => (
          <button
            key={p.name}
            onClick={() => {
              update(p.patch);
              setStatus(`已应用${p.name}布局`);
            }}
          >
            {p.name}
          </button>
        ))}
      </div>
      <div className="layout-dimensions">
        {[
          { key: "libraryWidth", label: "曲库宽度", min: 180, max: 440 },
          { key: "inspectorWidth", label: "练习参数宽度", min: 180, max: 360 },
          { key: "keyboardHeight", label: "琴键高度", min: 60, max: 220 },
          {
            key: "paperPreviewHeight",
            label: "纸谱上方键盘区域",
            min: 100,
            max: 360,
          },
        ].map((d) => (
          <label key={d.key}>
            {d.label}
            <input
              aria-label={d.label}
              type="range"
              min={d.min}
              max={d.max}
              step={10}
              value={value[d.key as keyof DisplayPreferences] as number}
              onChange={(e) => update({ [d.key]: Number(e.target.value) })}
            />
            <output>
              {value[d.key as keyof DisplayPreferences] as number} 像素
            </output>
          </label>
        ))}
      </div>
      <p className="parameter-help">
        尺寸立即生效。侧栏边缘可拖动；聚焦边缘后用方向键调整，Shift
        加快。窄窗口会自动限制面板尺寸，保留练习区域。
      </p>
      <div className="layout-saved">
        <label>
          已保存布局
          <select
            aria-label="已保存布局"
            value={selected}
            onChange={(e) => {
              setSelected(e.target.value);
              setName(saved.find((s) => s.id === e.target.value)?.name ?? "");
              setError("");
            }}
          >
            <option value="">新布局</option>
            {saved.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          名称
          <input
            aria-label="布局名称"
            maxLength={40}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <div className="dialog-actions">
          <button
            disabled={!selected}
            onClick={() => {
              const s = saved.find((s) => s.id === selected);
              if (s) {
                update(s.layout);
                setStatus(`已应用「${s.name}」`);
              }
            }}
          >
            应用布局
          </button>
          <button
            onClick={() => {
              if (!validName()) return;
              if (saved.length >= 24) {
                setError("最多保存 24 个布局，请先删除不需要的方案。");
                return;
              }
              if (saved.some((s) => s.name === name.trim())) {
                setError("已有同名布局，可更新当前方案或修改名称。");
                return;
              }
              const id = crypto.randomUUID();
              if (
                commit(
                  [
                    ...saved,
                    { id, name: name.trim(), layout: extractLayout(value) },
                  ],
                  "当前布局已另存",
                )
              )
                setSelected(id);
            }}
          >
            另存当前布局
          </button>
          <button
            disabled={!selected}
            onClick={() => {
              if (validName())
                commit(
                  saved.map((s) =>
                    s.id === selected
                      ? {
                          ...s,
                          name: name.trim(),
                          layout: extractLayout(value),
                        }
                      : s,
                  ),
                  "当前方案的名称和尺寸已更新",
                );
            }}
          >
            更新当前布局
          </button>
          <button
            disabled={!selected}
            onClick={() => {
              if (!validName()) return;
              commit(
                saved.map((s) =>
                  s.id === selected ? { ...s, name: name.trim() } : s,
                ),
                "布局已改名",
              );
            }}
          >
            仅改名
          </button>
          <button
            disabled={!selected}
            onClick={() => {
              if (
                commit(
                  saved.filter((s) => s.id !== selected),
                  "布局已删除；当前显示保持不变",
                )
              ) {
                setSelected("");
                setName("");
              }
            }}
          >
            删除布局
          </button>
        </div>
      </div>
      <p role="status">{status}</p>
      {error && (
        <p role="alert" className="dialog-error">
          {error}
        </p>
      )}
    </section>
  );
}
