import { useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  interfaceSnapshot,
  decodeInterfaceBackup,
  interfaceBaseline,
  restoreInterface,
  settingsGroups,
  groupNames,
  canonical,
  type InterfaceBackup,
  type Group,
} from "./interfaceBackup";
import { readLayouts, type SavedLayout } from "./LayoutSettings";
import { codeLabel } from "./displayPreferences";
import type { DisplayPreferences } from "./displayPreferences";
import { shortcutActions, shortcutLabel } from "./shortcuts";
type Preview = {
  backup: InterfaceBackup;
  baseline: string;
  current: DisplayPreferences;
  layouts: SavedLayout[];
};
const labels: Record<string, string> = {
  uiScale: "整体界面缩放",
  notation: "音符名称",
  noteLabels: "音符提示",
  keyLabels: "键上名称",
  tonic: "级数主音",
  octave: "音区数字",
  fontSize: "提示字号",
  palette: "声部配色",
  range: "键盘范围",
  previewBars: "预览小节",
  libraryOpen: "显示曲库",
  inspector: "显示练习参数",
  libraryWidth: "曲库宽度",
  inspectorWidth: "参数宽度",
  keyboardHeight: "琴键高度",
  paperPreviewHeight: "纸谱键盘区域",
  keyboardEnabled: "电脑键盘演奏",
  keyboardOctave: "电脑键盘音区",
  keyCodes: "弹奏按键",
};
const describe = (v: unknown) =>
  typeof v === "boolean"
    ? v
      ? "开启"
      : "关闭"
    : Array.isArray(v)
      ? v.map((k) => String(k).replace(/^(Key|Digit)/, "")).join(" ")
      : ((
          {
            pitch: "音名",
            solfege: "固定唱名",
            degree: "大调级数",
            auto: "指法优先",
            name: "名称",
            finger: "指法",
            both: "指法与名称",
            none: "隐藏",
            c: "C 键",
            white: "白键",
            all: "全部",
            standard: "曲目颜色",
            accessible: "左右手蓝橙",
          } as Record<string, string>
        )[String(v)] ?? String(v));
const settingText = (key: string, v: unknown) =>
  key === "uiScale"
    ? `${v}%`
    : key === "tonic"
      ? ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"][
          Number(v)
        ]
      : [
            "libraryWidth",
            "inspectorWidth",
            "keyboardHeight",
            "paperPreviewHeight",
            "fontSize",
          ].includes(key)
        ? `${v} 像素`
        : key === "range"
          ? `${v} 键`
          : key === "previewBars"
            ? `${v} 小节`
            : key === "keyboardOctave"
              ? `C${v}–C${Number(v) + 1}`
              : key === "keyCodes"
                ? (v as string[]).map(codeLabel).join(" ")
                : describe(v);
export function InterfaceBackupSettings({
  value,
  change,
}: {
  value: DisplayPreferences;
  change: (v: DisplayPreferences) => void;
}) {
  const [preview, setPreview] = useState<Preview | null>(null),
    [groups, setGroups] = useState<Record<Group, boolean>>({
      display: true,
      keyboard: true,
      shortcuts: true,
    }),
    [policies, setPolicies] = useState<
      Record<string, "keep" | "replace" | "skip">
    >({}),
    [error, setError] = useState(""),
    [status, setStatus] = useState(""),
    [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const load = async (file: File) => {
    setBusy(true);
    setError("");
    try {
      if (file.size > 1_000_000) throw new Error("界面设置备份超过 1 MB");
      const backup = decodeInterfaceBackup(await file.text());
      setPreview({
        backup,
        baseline: interfaceBaseline(value),
        current: value,
        layouts: readLayouts(),
      });
      setGroups({ display: true, keyboard: true, shortcuts: true });
      setPolicies(
        Object.fromEntries(backup.layouts.map((v) => [v.id, "keep"])),
      );
      setStatus("已读取备份，请核对恢复内容");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPreview(null);
    } finally {
      setBusy(false);
    }
  };
  const exportFile = async () => {
    setBusy(true);
    setError("");
    try {
      const contents = JSON.stringify(interfaceSnapshot(value), null, 2);
      if (isTauri()) {
        const path = await invoke<string | null>("export_interface_settings", {
          contents,
        });
        setStatus(path ? "界面设置备份已导出" : "已取消导出");
      } else {
        const url = URL.createObjectURL(
            new Blob([contents], { type: "application/json" }),
          ),
          a = document.createElement("a");
        a.href = url;
        a.download = "Neothesia-界面设置.neosettings";
        a.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
        setStatus("界面设置备份已下载");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="interface-backup-settings" aria-label="界面设置备份">
      <h3>界面设置备份</h3>
      <p className="parameter-help">
        导出显示、电脑键盘、操作快捷键和命名布局。导入后先核对，再选择恢复；练习成绩与曲目资料使用练习资料备份。
      </p>
      <div className="dialog-actions">
        <button disabled={busy} onClick={() => void exportFile()}>
          导出界面设置
        </button>
        <button disabled={busy} onClick={() => input.current?.click()}>
          读取界面设置备份
        </button>
      </div>
      <input
        ref={input}
        aria-label="界面设置备份文件"
        type="file"
        accept=".neosettings,.json"
        hidden
        onChange={(e) => {
          const file = e.target.files?.[0];
          e.target.value = "";
          if (file) void load(file);
        }}
      />
      {preview && (
        <div className="interface-backup-preview">
          <p>
            备份时间：
            {Number.isFinite(Date.parse(preview.backup.createdAt))
              ? new Date(preview.backup.createdAt).toLocaleString("zh-CN")
              : "未知"}{" "}
            · {preview.backup.layouts.length} 个命名布局
          </p>
          {(Object.keys(settingsGroups) as Group[]).map((group) => (
            <div key={group}>
              <label>
                <input
                  type="checkbox"
                  aria-label={`恢复${groupNames[group]}`}
                  checked={groups[group]}
                  onChange={(e) =>
                    setGroups({ ...groups, [group]: e.target.checked })
                  }
                />
                {groupNames[group]}
              </label>
              <details>
                <summary>核对{groupNames[group]}差异</summary>
                <table>
                  <thead>
                    <tr>
                      <th>设置</th>
                      <th>本机</th>
                      <th>备份</th>
                    </tr>
                  </thead>
                  <tbody>
                    {group === "shortcuts"
                      ? shortcutActions.map((a) => (
                          <tr key={a.id}>
                            <td>{a.label}</td>
                            <td>
                              {shortcutLabel(preview.current.shortcuts[a.id])}
                            </td>
                            <td>
                              {shortcutLabel(
                                preview.backup.preferences.shortcuts[a.id],
                              )}
                            </td>
                          </tr>
                        ))
                      : settingsGroups[group].map((key) => (
                          <tr
                            key={key}
                            className={
                              canonical(preview.current[key]) !==
                              canonical(preview.backup.preferences[key])
                                ? "settings-difference"
                                : ""
                            }
                          >
                            <td>{labels[key]}</td>
                            <td>{settingText(key, preview.current[key])}</td>
                            <td>
                              {settingText(
                                key,
                                preview.backup.preferences[key],
                              )}
                            </td>
                          </tr>
                        ))}
                  </tbody>
                </table>
              </details>
            </div>
          ))}
          {preview.backup.layouts.map((source) => {
            const existing = preview.layouts.find(
              (v) => v.name === source.name,
            );
            return (
              <div className="interface-layout-row" key={source.id}>
                <div>
                  <strong>{source.name}</strong>
                  <small>
                    {existing ? "本机已有同名方案" : "新方案"} · 曲库{" "}
                    {source.layout.libraryWidth} / 参数{" "}
                    {source.layout.inspectorWidth} / 琴键{" "}
                    {source.layout.keyboardHeight} 像素
                  </small>
                </div>
                <select
                  aria-label={`恢复布局：${source.name}`}
                  value={policies[source.id] ?? "skip"}
                  onChange={(e) =>
                    setPolicies({
                      ...policies,
                      [source.id]: e.target.value as
                        "keep" | "replace" | "skip",
                    })
                  }
                >
                  <option value="keep">
                    {existing ? "保留本机方案" : "加入新方案"}
                  </option>
                  {existing && <option value="replace">使用备份方案</option>}
                  <option value="skip">不恢复</option>
                </select>
              </div>
            );
          })}
          <div className="dialog-actions">
            <button
              onClick={() => {
                setPreview({
                  ...preview,
                  baseline: interfaceBaseline(value),
                  current: value,
                  layouts: readLayouts(),
                });
                setError("");
                setStatus("已重新读取本机设置，请核对差异");
              }}
            >
              重新核对本机设置
            </button>
            <button
              disabled={
                busy ||
                (!Object.values(groups).some(Boolean) &&
                  Object.values(policies).every((v) => v === "skip"))
              }
              onClick={() => {
                try {
                  const next = restoreInterface(
                    preview.backup,
                    value,
                    preview.baseline,
                    groups,
                    policies,
                  );
                  change(next);
                  setPreview(null);
                  setError("");
                  setStatus("所选界面设置已恢复");
                } catch (e) {
                  setError(e instanceof Error ? e.message : String(e));
                }
              }}
            >
              恢复所选设置
            </button>
            <button
              onClick={() => {
                setPreview(null);
                setError("");
                setStatus("已取消恢复");
              }}
            >
              取消恢复
            </button>
          </div>
        </div>
      )}
      <p role="status">{status}</p>
      {error && (
        <p className="dialog-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
