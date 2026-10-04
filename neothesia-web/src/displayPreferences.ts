import {
  normalizeShortcuts,
  shortcutDefaults,
  type Shortcuts,
} from "./shortcuts";
import { pitchName } from "./api";
export type DisplayPreferences = {
  shortcuts: Shortcuts;
  uiScale: number;
  notation: "pitch" | "solfege" | "degree";
  noteLabels: "auto" | "name" | "finger" | "both" | "none";
  keyLabels: "c" | "white" | "all" | "none";
  tonic: number;
  octave: boolean;
  fontSize: number;
  palette: "standard" | "accessible";
  range: 49 | 88;
  previewBars: number;
  libraryOpen: boolean;
  inspector: boolean;
  keyboardEnabled: boolean;
  keyboardOctave: number;
  keyCodes: string[];
  libraryWidth: number;
  inspectorWidth: number;
  keyboardHeight: number;
  paperPreviewHeight: number;
};
export const asdfCodes = [
  "KeyA",
  "KeyW",
  "KeyS",
  "KeyE",
  "KeyD",
  "KeyF",
  "KeyT",
  "KeyG",
  "KeyY",
  "KeyH",
  "KeyU",
  "KeyJ",
  "KeyK",
];
export const zxcvCodes = [
  "KeyZ",
  "KeyS",
  "KeyX",
  "KeyD",
  "KeyC",
  "KeyV",
  "KeyG",
  "KeyB",
  "KeyH",
  "KeyN",
  "KeyJ",
  "KeyM",
  "Comma",
];
export const displayDefaults: DisplayPreferences = {
  shortcuts: { ...shortcutDefaults },
  uiScale: 100,
  libraryWidth: 270,
  inspectorWidth: 218,
  keyboardHeight: 100,
  paperPreviewHeight: 150,
  notation: "pitch",
  noteLabels: "auto",
  keyLabels: "c",
  tonic: 0,
  octave: true,
  fontSize: 10,
  palette: "standard",
  range: 49,
  previewBars: 4,
  libraryOpen: true,
  inspector: true,
  keyboardEnabled: true,
  keyboardOctave: 4,
  keyCodes: asdfCodes,
};
export const displayStorage = "neothesia-display-v1";
export const allowedCode = (code: string) =>
  /^(Key[A-Z]|Digit[0-9]|Comma|Period|Semicolon)$/.test(code);
export const codeLabel = (code: string) =>
  ({ Comma: ",", Period: ".", Semicolon: ";" })[code] ??
  code.replace(/^(Key|Digit)/, "");
export function readDisplayPreferences(): DisplayPreferences {
  let raw: Partial<DisplayPreferences> = {};
  try {
    raw = JSON.parse(localStorage.getItem(displayStorage) ?? "{}") ?? {};
  } catch {}
  return normalizeDisplayPreferences(raw);
}
export function normalizeDisplayPreferences(
  raw: Partial<DisplayPreferences>,
): DisplayPreferences {
  const size = (value: unknown, low: number, high: number, fallback: number) =>
    typeof value === "number" && Number.isFinite(value)
      ? Math.max(low, Math.min(high, Math.round(value)))
      : fallback;
  const keys =
    Array.isArray(raw.keyCodes) &&
    raw.keyCodes.length === 13 &&
    raw.keyCodes.every(
      (code) => typeof code === "string" && allowedCode(code),
    ) &&
    new Set(raw.keyCodes).size === 13
      ? raw.keyCodes
      : [...asdfCodes];
  return {
    ...displayDefaults,
    keyCodes: keys,
    uiScale: size(raw.uiScale, 80, 150, 100),
    shortcuts: normalizeShortcuts(raw.shortcuts, keys),
    libraryWidth: size(raw.libraryWidth, 180, 440, 270),
    inspectorWidth: size(raw.inspectorWidth, 180, 360, 218),
    keyboardHeight: size(raw.keyboardHeight, 60, 220, 100),
    paperPreviewHeight: size(raw.paperPreviewHeight, 100, 360, 150),
    notation: ["pitch", "solfege", "degree"].includes(raw.notation ?? "")
      ? raw.notation!
      : "pitch",
    noteLabels: ["auto", "name", "finger", "both", "none"].includes(
      raw.noteLabels ?? "",
    )
      ? raw.noteLabels!
      : "auto",
    keyLabels: ["c", "white", "all", "none"].includes(raw.keyLabels ?? "")
      ? raw.keyLabels!
      : "c",
    palette: raw.palette === "accessible" ? "accessible" : "standard",
    tonic:
      Number.isInteger(raw.tonic) && raw.tonic! >= 0 && raw.tonic! < 12
        ? raw.tonic!
        : 0,
    octave: typeof raw.octave === "boolean" ? raw.octave : true,
    fontSize: [10, 12, 14, 16].includes(raw.fontSize ?? 0) ? raw.fontSize! : 10,
    range: raw.range === 88 ? 88 : 49,
    previewBars: [2, 4, 8, 12].includes(raw.previewBars ?? 0)
      ? raw.previewBars!
      : 4,
    libraryOpen: typeof raw.libraryOpen === "boolean" ? raw.libraryOpen : true,
    inspector: typeof raw.inspector === "boolean" ? raw.inspector : true,
    keyboardEnabled:
      typeof raw.keyboardEnabled === "boolean" ? raw.keyboardEnabled : true,
    keyboardOctave:
      Number.isInteger(raw.keyboardOctave) &&
      raw.keyboardOctave! >= 2 &&
      raw.keyboardOctave! <= 6
        ? raw.keyboardOctave!
        : 4,
  };
}
export function displayPitch(pitch: number, prefs: DisplayPreferences) {
  const pc = ((pitch % 12) + 12) % 12,
    register = Math.floor(pitch / 12) - 1;
  const name =
    prefs.notation === "pitch"
      ? pitchName(pitch)
          .replace(/-?\d+$/, " ")
          .trim()
      : prefs.notation === "solfege"
        ? [
            "Do",
            "Do♯",
            "Re",
            "Re♯",
            "Mi",
            "Fa",
            "Fa♯",
            "Sol",
            "Sol♯",
            "La",
            "La♯",
            "Si",
          ][pc]
        : ["1", "♯1", "2", "♯2", "3", "4", "♯4", "5", "♯5", "6", "♯6", "7"][
            (pc - prefs.tonic + 12) % 12
          ];
  const suffix =
    prefs.notation === "degree"
      ? String(register).replace(
          /[0-9-]/g,
          (digit) =>
            ({
              "-": "₋",
              "0": "₀",
              "1": "₁",
              "2": "₂",
              "3": "₃",
              "4": "₄",
              "5": "₅",
              "6": "₆",
              "7": "₇",
              "8": "₈",
              "9": "₉",
            })[digit]!,
        )
      : String(register);
  return name + (prefs.octave ? suffix : "");
}
export function noteLabel(
  pitch: number,
  finger: number | null | undefined,
  prefs: DisplayPreferences,
) {
  const name = displayPitch(pitch, prefs);
  return prefs.noteLabels === "none"
    ? ""
    : prefs.noteLabels === "finger"
      ? finger
        ? String(finger)
        : ""
      : prefs.noteLabels === "both"
        ? finger
          ? `${finger} · ${name}`
          : name
        : prefs.noteLabels === "auto" && finger
          ? String(finger)
          : name;
}
export const handColor = (part: string, prefs: DisplayPreferences) =>
  prefs.palette === "accessible"
    ? part === "left"
      ? "#56b4e9"
      : part === "right"
        ? "#e69f00"
        : "#cccccc"
    : part === "left"
      ? "#6cb7f3"
      : "#a9df74";
export const keyboardMap = (prefs: DisplayPreferences) =>
  Object.fromEntries(
    prefs.keyCodes.map((code, index) => [
      code,
      12 * (prefs.keyboardOctave + 1) + index,
    ]),
  );
