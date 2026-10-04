import type { RoutineState } from "./RoutineDialog";
import { invoke, isTauri } from "@tauri-apps/api/core";

export interface SongRow {
  id: string;
  title: string;
  composer: string;
  source: string;
  category: string;
  license: string;
  sourceUrl: string;
  path: string;
  tags?: string[];
  difficulty?: string | null;
  rating?: number;
}
export interface LibraryMonitorStatus {
  enabled: boolean;
  phase: string;
  revision: number;
  lastScan: number;
  pending: number;
  files: number;
  updated: number;
  errors: string[];
  current: string;
  paperUpdates?: number;
  paperUnavailable?: number;
  scoreUpdates?: number;
  scoreUnavailable?: number;
}
export interface Note {
  index: number;
  pitch: number;
  start: number;
  duration: number;
  velocity: number;
  track: number;
  tick: number;
  endTick: number;
  finger: number | null;
  part: "left" | "right" | "other";
  manualHand: boolean;
}
export interface CollectionSong {
  contentId: string;
  title: string;
  path: string | null;
  favorite: boolean;
  queuePosition: number | null;
  lastUsed: number;
  lastPracticed?: number | null;
  sessions: number;
  accuracy: number | null;
  generated: boolean;
  available: boolean;
  recommended: [number, number] | null;
  review: { due: boolean; days: number; streak: number } | null;
}
export interface TrackSound {
  volume: number;
  pan: number | null;
  program: number | null;
}
export interface LoadedSong {
  fingerActionRevision?: string;
  fingerActions?: {
    track: number;
    index: number;
    beforeTrack: number;
    beforeIndex: number;
    atTick: number;
    at: number;
    from: number;
    to: number;
    valid: boolean;
    practiceRate: number;
    validationRate: number;
  }[];
  trackSounds?: Record<string, TrackSound>;
  soundPresets?: { program: number; name: string }[];
  importWarnings?: string[];
  meterCorrection: {
    numerator: number;
    denominator: number;
    pickup_ticks: number;
  } | null;
  title: string;
  contentId: string;
  sourcePath: string;
  duration: number;
  notes: Note[];
  ppq: number;
  generated: boolean;
  hasScore: boolean;
  fingerUndoAvailable?: boolean;
  scoreRevision: number;
  measures: {
    number: number;
    start: number;
    end: number;
    startTick: number;
    endTick: number;
    numerator: number;
    denominator: number;
    explicit: boolean;
    partial: boolean;
  }[];
  tempo: { tick: number; seconds: number; bpm: number }[];
  tracks: {
    id: number;
    name: string;
    sourceName: string;
    color?: string | null;
    appearanceCustom?: boolean;
    notes: number;
    mode: string;
    part: string;
    visible: boolean;
    inferred: boolean;
  }[];
}
export interface SpeedLadderPlan {
  startPercent: number;
  targetPercent: number;
  stepPercent: number;
  passesRequired: number;
  accuracyPercent: number;
  onTimePercent: number | null;
  failuresBeforeStepBack: number;
  attemptLimit: number;
}
export interface LadderPreset {
  id: string;
  name: string;
  start: number;
  end: number;
  plan: SpeedLadderPlan;
  settings: {
    mode: string;
    hands: string;
    countIn: number;
    metronome: boolean;
  };
}
export interface LadderRun {
  preset: LadderPreset;
  active: boolean;
  progress: {
    stage: number;
    successStreak: number;
    failureStreak: number;
    totalRounds: number;
    batchRounds: number;
    completed: boolean;
    limited: boolean;
    lastResult: string;
    lastAccuracy: number | null;
    lastOnTime: number | null;
  };
}
export interface Snapshot {
  routine: RoutineState | null;
  ladder: LadderRun | null;
  status: string;
  position: number;
  duration: number;
  practiceMs: number;
  speed: number;
  wait: boolean;
  hands: string;
  required: number[];
  pressed: number[];
  score: {
    matched_notes: number;
    wrong_notes: number;
    missed_notes: number;
    on_time_notes: number;
    required_notes: number;
  };
  input: string | null;
  output: string;
  error: string | null;
  saved: boolean;
  passage: { start: number; end: number } | null;
  repetitions: number;
  volume: number;
  rounds: number;
  inputConnected: boolean;
  midiEvents: number;
  pedal: number;
  recording: boolean;
  recordDuration: number;
  mode: string;
  measure: number;
  beat: number;
  tick: number;
  bpm: number;
  countIn: number;
  countRemaining: number;
  metronome: boolean;
  latency: number;
  adaptive: boolean;
  summary: Feedback | null;
}
export const emptyState: Snapshot = {
  routine: null,
  ladder: null,
  status: "idle",
  position: 0,
  duration: 0,
  practiceMs: 0,
  speed: 1,
  wait: true,
  hands: "both",
  required: [],
  pressed: [],
  score: {
    matched_notes: 0,
    wrong_notes: 0,
    missed_notes: 0,
    on_time_notes: 0,
    required_notes: 0,
  },
  input: null,
  output: "内置钢琴",
  error: null,
  saved: false,
  passage: null,
  repetitions: 0,
  volume: 0.8,
  recording: false,
  recordDuration: 0,
  inputConnected: false,
  midiEvents: 0,
  pedal: 0,
  rounds: 0,
  mode: "wait",
  measure: 1,
  beat: 1,
  tick: 0,
  bpm: 120,
  countIn: 1,
  countRemaining: 0,
  metronome: false,
  latency: 0,
  adaptive: false,
  summary: null,
};
export type EngineCommand = { type: string; [key: string]: unknown };
const service = "http://127.0.0.1:32124";
async function http<T>(path: string, command?: EngineCommand): Promise<T> {
  const response = await fetch(service + path, {
    method: command ? "POST" : "GET",
    headers: command
      ? { "Content-Type": "application/json", "X-Neothesia-Client": "web" }
      : {},
    body: command ? JSON.stringify(command) : undefined,
    signal: AbortSignal.timeout(
      command &&
        [
          "importScore",
          "importMidi",
          "importPackage",
          "inspectPackage",
          "exportPackage",
          "libraryScores",
          "manageLibraryScore",
          "openLibraryScore",
        ].includes(command.type)
        ? 120000
        : 15000,
    ),
  });
  const data = await response.json();
  if (!response.ok) throw new Error(data.error || "本地音乐引擎请求失败");
  return data;
}
export const api = {
  checkLibraryScoreImport: async (
    path: string,
    contentId: string,
    file: File,
    kind: string,
    target?: string,
  ) => {
    if (file.size > (kind === "notation" ? 4_000_000 : 48_000_000))
      throw new Error("文件超过谱面容量上限");
    type Result = {
      matches: { id: string; name: string; kind: string; page: number }[];
    };
    if (isTauri())
      return invoke<Result>("engine_command", {
        command: {
          type: "checkLibraryScoreImport",
          path,
          content_id: contentId,
          bytes: Array.from(new Uint8Array(await file.arrayBuffer())),
          kind,
          target: target ?? null,
        },
      });
    const r = await fetch(service + "/api/library-score/check", {
      method: "POST",
      headers: {
        "X-Neothesia-Client": "web",
        "X-Neothesia-Path": encodeURIComponent(path),
        "X-Neothesia-Song": contentId,
        "X-Neothesia-Kind": kind,
        ...(target ? { "X-Neothesia-Target": target } : {}),
      },
      body: file,
      signal: AbortSignal.timeout(120000),
    });
    const value = await r.json();
    if (!r.ok) throw new Error(value.error || "保存结果核对失败");
    return value as Result;
  },
  desktop: isTauri(),
  library: () =>
    isTauri()
      ? invoke<{ songs: SongRow[] }>("library_list")
      : http<{ songs: SongRow[] }>("/api/library"),
  state: () =>
    isTauri() ? invoke<Snapshot>("engine_state") : http<Snapshot>("/api/state"),
  song: () =>
    isTauri()
      ? invoke<LoadedSong | null>("engine_song")
      : http<LoadedSong | null>("/api/song"),
  devices: () =>
    isTauri()
      ? invoke<{ inputs: string[]; outputs: string[]; vst3: string[] }>(
          "midi_devices",
        )
      : http<{ inputs: string[]; outputs: string[]; vst3: string[] }>(
          "/api/devices",
        ),
  command: <T = { ok: boolean }>(command: EngineCommand) =>
    isTauri()
      ? invoke<T>("engine_command", { command })
      : http<T>("/api/command", command),
  cancelLibraryScoreImport: (batchId: string) =>
    invoke("cancel_library_score_import", { batchId }),
  importLibraryScores: (
    batchId: string,
    path: string,
    contentId: string,
    kind: string,
    target?: string,
    page = 0,
  ) =>
    invoke<{
      results: { name: string; ok: boolean; error?: string }[];
      stopped?: boolean;
    } | null>("import_library_scores", {
      batchId,
      path,
      contentId,
      kind,
      target: target ?? null,
      page,
    }),
  addLibraryScore: async (
    path: string,
    contentId: string,
    file: File,
    kind: string,
    target?: string,
    page = 0,
  ) => {
    if (file.size > (kind === "notation" ? 4_000_000 : 48_000_000))
      throw new Error("文件超过当前谱面格式容量上限");
    if (isTauri())
      return invoke<{ active?: string }>("engine_command", {
        command: {
          type: "addLibraryScore",
          path,
          content_id: contentId,
          name: file.name,
          bytes: Array.from(new Uint8Array(await file.arrayBuffer())),
          kind,
          target: target ?? null,
          page,
        },
      });
    const r = await fetch(service + "/api/library-score/add", {
      method: "POST",
      headers: {
        "X-Neothesia-Client": "web",
        "X-Neothesia-Song": contentId,
        "X-Neothesia-Name": encodeURIComponent(file.name),
        "X-Neothesia-Path": encodeURIComponent(path),
        "X-Neothesia-Kind": kind,
        "X-Neothesia-Page": String(page),
        ...(target ? { "X-Neothesia-Target": target } : {}),
      },
      body: file,
      signal: AbortSignal.timeout(120000),
    });
    const value = await r.json();
    if (!r.ok) throw new Error(value.error || "谱面导入失败");
    return value as { active?: string };
  },
  addScoreAttachment: async (
    contentId: string,
    file: File,
    append?: string,
  ) => {
    if (file.size > 48_000_000) throw new Error("单份谱面超过 48 MB");
    if (isTauri())
      return invoke<{ active: string }>("engine_command", {
        command: {
          type: "addScoreAttachment",
          content_id: contentId,
          name: file.name,
          bytes: Array.from(new Uint8Array(await file.arrayBuffer())),
          append: append ?? null,
        },
      });
    const r = await fetch(service + "/api/score-attachment/add", {
      method: "POST",
      headers: {
        "X-Neothesia-Client": "web",
        "X-Neothesia-Song": contentId,
        "X-Neothesia-Name": encodeURIComponent(file.name),
        ...(append ? { "X-Neothesia-Append": append } : {}),
      },
      body: file,
      signal: AbortSignal.timeout(120000),
    });
    const data = await r.json();
    if (!r.ok) throw new Error(data.error || "谱面导入失败");
    return data as { active: string };
  },
  readScoreAttachment: async (
    contentId: string,
    id: string,
    page: number,
    previewKind?: string,
  ) => {
    if (isTauri()) {
      const bytes = await invoke<ArrayBuffer>("score_attachment_bytes", {
        contentId,
        id,
        page,
        previewKind: previewKind ?? null,
      });
      return new Uint8Array(bytes);
    }
    const r = await fetch(service + "/api/score-attachment/read", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Neothesia-Client": "web",
      },
      body: JSON.stringify({ contentId, id, page, previewKind }),
      signal: AbortSignal.timeout(30000),
    });
    if (!r.ok) {
      const v = await r.json();
      throw new Error(v.error || "谱页读取失败");
    }
    return new Uint8Array(await r.arrayBuffer());
  },
  importScoreAttachments: (contentId: string, append?: string) =>
    invoke<{ results: { name: string; ok: boolean; error?: string }[] } | null>(
      "import_score_attachments",
      { contentId, append: append ?? null },
    ),
  revealFile: (path: string) => invoke<void>("reveal_library_file", { path }),
  importFolder: () => invoke<number | null>("import_folder"),
  repairPath: (contentId: string, from: string) =>
    invoke<{ results: import("./LibraryRepairsDialog").RepairResult[] } | null>(
      "repair_midi",
      { contentId, from },
    ),
  relocate: (contentId: string) =>
    invoke<LoadedSong | null>("relocate_midi", { contentId }),
  export: () => invoke<string | null>("export_midi"),
  linkScoreSource: (midiPath: string, contentId: string, versionId: string) =>
    invoke<Record<string, unknown> | null>("link_score_source", {
      midiPath,
      contentId,
      versionId,
    }),
  exportPaperPage: (name: string, bytes: Uint8Array) =>
    invoke<string | null>("export_paper_page", {
      name,
      bytes: Array.from(bytes),
    }),
  exportAnnotatedScore: (request: Record<string, unknown>) =>
    invoke<string | null>("export_annotated_score", { request }),
  pickPracticeBackup: () =>
    invoke<Record<string, unknown> | null>("pick_practice_backup"),
  exportPracticeBackup: (selection: {
    plans: boolean;
    records: boolean;
    presets: boolean;
    days: number;
  }) => invoke<string | null>("export_practice_backup", { selection }),
  downloadPracticeBackup: async (selection: {
    plans: boolean;
    records: boolean;
    presets: boolean;
    days: number;
  }) => {
    const r = await fetch(service + "/api/practice-backup/export", {
      method: "POST",
      headers: {
        "X-Neothesia-Client": "web",
        "Content-Type": "application/json",
      },
      body: JSON.stringify(selection),
      signal: AbortSignal.timeout(120000),
    });
    if (!r.ok) {
      const v = await r.json();
      throw new Error(v.error || "备份导出失败");
    }
    return r.blob();
  },
  stagePracticeBackup: async (file: File) => {
    if (file.size > 256_000_000) throw new Error("练习备份超过 256 MB");
    const r = await fetch(service + "/api/practice-backup/stage", {
      method: "POST",
      headers: { "X-Neothesia-Client": "web" },
      body: file,
      signal: AbortSignal.timeout(120000),
    });
    const v = await r.json();
    if (!r.ok) throw new Error(v.error || "备份预览失败");
    return v as Record<string, unknown>;
  },
  pickPackage: () => invoke<Record<string, unknown> | null>("pick_package"),
  stagePackage: async (file: File) => {
    if (file.size > 256_000_000) throw new Error("曲目包超过 256 MB");
    if (isTauri())
      return invoke<Record<string, unknown>>("stage_package", {
        bytes: Array.from(new Uint8Array(await file.arrayBuffer())),
      });
    const r = await fetch(service + "/api/package/stage", {
      method: "POST",
      headers: { "X-Neothesia-Client": "web" },
      body: file,
      signal: AbortSignal.timeout(120000),
    });
    const v = await r.json();
    if (!r.ok) throw new Error(v.error || "曲目包预览失败");
    return v as Record<string, unknown>;
  },
  downloadPackage: async () => {
    const r = await fetch(service + "/api/package/export", {
      method: "POST",
      headers: { "X-Neothesia-Client": "web" },
      signal: AbortSignal.timeout(120000),
    });
    if (!r.ok) {
      const v = await r.json();
      throw new Error(v.error || "曲目包导出失败");
    }
    return r.blob();
  },
  exportPackage: () => invoke<string | null>("export_package"),
  import: () => invoke<LoadedSong | null>("import_midi"),
};
export const pitchName = (pitch: number) =>
  `${["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"][pitch % 12]}${Math.floor(pitch / 12) - 1}`;
export const clock = (seconds: number) =>
  `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;

export interface Feedback {
  mode?: string;
  trend?: {
    attempts: number;
    accuracyDelta: number | null;
    speedDelta: number | null;
    tempoDelta: number | null;
  };
  overall: Snapshot["score"];
  measures: {
    measure: number;
    breakdown: {
      matched_notes: number;
      wrong_notes: number;
      missed_notes: number;
    };
    timing: {
      median_offset_ms: number | null;
      median_deviation_ms: number | null;
    };
  }[];
  parts: {
    part: string;
    breakdown: {
      matched_notes: number;
      wrong_notes: number;
      missed_notes: number;
    };
  }[];
  timing: {
    median_offset_ms: number | null;
    median_deviation_ms: number | null;
  };
  chords: {
    eligible_chords: number;
    complete_chords: number;
    incomplete_chords: number;
    median_attack_span_ms: number | null;
  };
  expression: {
    velocity: {
      matched_samples: number;
      mean_abs_difference: number | null;
      contour_steps: number;
      contour_aligned: number;
    };
    pedal: {
      user_used: boolean;
      target_present: boolean;
      timing_samples: number;
      median_offset_ms: number | null;
    };
    articulation: {
      matched_samples: number;
      median_duration_ratio_percent: number | null;
    };
  };
}
