import {
  displayStorage,
  normalizeDisplayPreferences,
  type DisplayPreferences,
} from "./displayPreferences";
import {
  layoutStorage,
  layoutKeys,
  extractLayout,
  readLayouts,
  type SavedLayout,
} from "./LayoutSettings";
import { normalizeShortcuts } from "./shortcuts";
export const settingsGroups = {
  display: [
    "notation",
    "noteLabels",
    "keyLabels",
    "tonic",
    "octave",
    "fontSize",
    "palette",
    "range",
    "previewBars",
    ...layoutKeys,
  ],
  keyboard: ["keyboardEnabled", "keyboardOctave", "keyCodes"],
  shortcuts: ["shortcuts"],
} as const;
export const groupNames = {
  display: "显示与面板",
  keyboard: "电脑键盘弹奏",
  shortcuts: "操作快捷键",
};
export type Group = keyof typeof settingsGroups;
export type InterfaceBackup = {
  format: "neothesia-interface-settings";
  version: 1;
  createdAt: string;
  preferences: DisplayPreferences;
  layouts: SavedLayout[];
};
export function canonical(v: unknown): string {
  if (Array.isArray(v)) return `[${v.map(canonical).join(",")}]`;
  if (v && typeof v === "object")
    return `{${Object.keys(v)
      .sort()
      .map(
        (k) =>
          JSON.stringify(k) +
          ":" +
          canonical((v as Record<string, unknown>)[k]),
      )
      .join(",")}}`;
  return JSON.stringify(v) ?? "undefined";
}
const object = (v: unknown): v is Record<string, unknown> =>
  !!v && typeof v === "object" && !Array.isArray(v);
export function decodeInterfaceBackup(text: string): InterfaceBackup {
  if (new Blob([text]).size > 1_000_000)
    throw new Error("界面设置备份超过 1 MB");
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    throw new Error("文件不是有效的界面设置备份");
  }
  if (
    !object(raw) ||
    raw.format !== "neothesia-interface-settings" ||
    raw.version !== 1 ||
    !object(raw.preferences) ||
    !Array.isArray(raw.layouts) ||
    raw.layouts.length > 24
  )
    throw new Error("界面设置备份类型、版本或布局数量不支持");
  const sourcePrefs: Record<string, unknown> = {
    ...raw.preferences,
    shortcuts:
      typeof raw.preferences.shortcuts === "object" &&
      raw.preferences.shortcuts !== null
        ? { ...(raw.preferences.shortcuts as Record<string, unknown>) }
        : raw.preferences.shortcuts,
  };
  if (sourcePrefs.shortcuts && typeof sourcePrefs.shortcuts === "object") {
    for (const [id, chord] of Object.entries({
      zoomIn: "Alt+Shift+Equal",
      zoomOut: "Alt+Shift+Minus",
      zoomReset: "Alt+Shift+Digit0",
    }))
      if (!(id in sourcePrefs.shortcuts))
        (sourcePrefs.shortcuts as Record<string, unknown>)[id] = Object.values(
          sourcePrefs.shortcuts,
        ).includes(chord)
          ? null
          : chord;
  }
  const prefs = normalizeDisplayPreferences(
      sourcePrefs as Partial<DisplayPreferences>,
    ),
    fields = Object.values(settingsGroups).flat();
  for (const key of fields)
    if (
      !(key === "uiScale" && raw.preferences[key] === undefined) &&
      canonical(prefs[key]) !== canonical(sourcePrefs[key])
    )
      throw new Error("备份含有无效设置或相互冲突的快捷键，未应用");
  const ids = new Set(),
    names = new Set();
  const layouts = raw.layouts.map((v) => {
    if (
      !object(v) ||
      typeof v.id !== "string" ||
      !v.id ||
      v.id.length > 100 ||
      ids.has(v.id) ||
      typeof v.name !== "string" ||
      v.name !== v.name.trim() ||
      !v.name ||
      v.name.length > 40 ||
      names.has(v.name) ||
      !object(v.layout)
    )
      throw new Error("备份的布局名称或身份无效/重复");
    const normalized = extractLayout(normalizeDisplayPreferences(v.layout));
    for (const key of layoutKeys)
      if (
        !(key === "uiScale" && v.layout[key] === undefined) &&
        canonical(normalized[key]) !== canonical(v.layout[key])
      )
        throw new Error(`「${v.name}」的布局尺寸或开关无效`);
    ids.add(v.id);
    names.add(v.name);
    return { id: v.id, name: v.name, layout: normalized };
  });
  return {
    format: "neothesia-interface-settings",
    version: 1,
    createdAt: typeof raw.createdAt === "string" ? raw.createdAt : "",
    preferences: prefs,
    layouts,
  };
}
export function interfaceSnapshot(
  preferences: DisplayPreferences,
): InterfaceBackup {
  return {
    format: "neothesia-interface-settings",
    version: 1,
    createdAt: new Date().toISOString(),
    preferences,
    layouts: readLayouts(),
  };
}
export function interfaceBaseline(preferences: DisplayPreferences) {
  return canonical([
    preferences,
    localStorage.getItem(displayStorage),
    localStorage.getItem(layoutStorage),
  ]);
}
export function restoreInterface(
  backup: InterfaceBackup,
  current: DisplayPreferences,
  baseline: string,
  groups: Record<Group, boolean>,
  policies: Record<string, "keep" | "replace" | "skip">,
): DisplayPreferences {
  if (interfaceBaseline(current) !== baseline)
    throw new Error("本机设置或布局已变化，请重新核对后恢复");
  const result = { ...current };
  for (const [group, fields] of Object.entries(settingsGroups))
    if (groups[group as Group])
      for (const field of fields)
        (result as unknown as Record<string, unknown>)[field] =
          backup.preferences[field];
  if (
    canonical(normalizeShortcuts(result.shortcuts, result.keyCodes)) !==
    canonical(result.shortcuts)
  )
    throw new Error(
      "恢复后的操作快捷键与弹奏键冲突，请同时恢复两组或保留其中一组后重新核对",
    );
  const layouts = readLayouts();
  for (const source of backup.layouts) {
    const policy = policies[source.id] ?? "skip";
    if (policy !== "keep" && policy !== "replace") continue;
    const i = layouts.findIndex((v) => v.name === source.name);
    if (i >= 0) {
      if (policy === "replace") layouts[i] = { ...source, id: layouts[i].id };
    } else layouts.push({ ...source, id: crypto.randomUUID() });
  }
  if (layouts.length > 24)
    throw new Error("恢复后超过 24 个布局，请减少所选方案");
  const oldPrefs = localStorage.getItem(displayStorage),
    oldLayouts = localStorage.getItem(layoutStorage);
  try {
    localStorage.setItem(displayStorage, JSON.stringify(result));
    localStorage.setItem(layoutStorage, JSON.stringify(layouts));
  } catch {
    try {
      if (oldPrefs === null) localStorage.removeItem(displayStorage);
      else localStorage.setItem(displayStorage, oldPrefs);
      if (oldLayouts === null) localStorage.removeItem(layoutStorage);
      else localStorage.setItem(layoutStorage, oldLayouts);
    } catch {
      throw new Error(
        "写入与回退均未成功，请保留备份文件，重新打开设置核对本机资料",
      );
    }
    throw new Error("本机存储写入失败，已回退原设置");
  }
  window.dispatchEvent(new Event("neothesia-layouts-changed"));
  return result;
}
