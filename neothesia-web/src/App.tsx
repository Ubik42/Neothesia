import { eventShortcut, shortcutActions, shortcutLabel } from "./shortcuts";
import { PanelResizeHandle } from "./PanelResizeHandle";
import type {
  HistoryScoreFocus,
  HistoryScoreReview,
} from "./historyScoreFocus";
import { practiceDuration } from "./practiceTime";
import { DisplaySettingsDialog } from "./DisplaySettingsDialog";
import {
  readDisplayPreferences,
  displayStorage,
  keyboardMap,
  codeLabel,
  displayPitch,
  handColor,
} from "./displayPreferences";
import { TrackAppearanceControl } from "./TrackAppearanceControl";
import { TrackSoundControl } from "./TrackSoundControl";
import { PracticeBackupDialog } from "./PracticeBackupDialog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  FolderOpen,
  History,
  KeyboardMusic,
  ListMusic,
  Maximize2,
  Menu,
  Music2,
  Pause,
  Play,
  RotateCcw,
  Search,
  Settings2,
  Star,
  Volume2,
  X,
} from "lucide-react";
import {
  api,
  clock,
  emptyState,
  pitchName,
  type CollectionSong,
  type EngineCommand,
  type LoadedSong,
  type Snapshot,
  type SongRow,
} from "./api";
import { Notation } from "./Notation";
import { FingeringDialog } from "./FingeringDialog";
import { PackageDialog } from "./PackageDialog";
import { ImportDialog } from "./ImportDialog";
import { LibraryDialog } from "./LibraryDialog";
import { ScoresDialog } from "./ScoresDialog";
import { PaperScore } from "./PaperScore";
import { MeterDialog } from "./MeterDialog";
import { HandsDialog } from "./HandsDialog";
import { RoutineDialog, RoutineStatus } from "./RoutineDialog";
import { LadderDialog, LadderStatus } from "./LadderDialog";
import { PassageDialog } from "./PassageDialog";
import { BarNavigator } from "./BarNavigator";
import { Piano } from "./Piano";
import {
  ExerciseDialog,
  FeedbackDialog,
  MetadataDialog,
} from "./PracticeDialogs";
import { HistoryDialog } from "./HistoryDialog";
type Tab = "全部" | "收藏" | "最近" | "队列";
const normalize = (path: string | null | undefined) =>
  (path || "").replaceAll("\\", "/").toLowerCase();

export default function App() {
  const [display, setDisplay] = useState(readDisplayPreferences),
    [displayDialog, setDisplayDialog] = useState(false);
  useEffect(() => {
    document.documentElement.style.zoom = String(display.uiScale / 100);
    document.documentElement.style.setProperty(
      "--ui-scale",
      String(display.uiScale / 100),
    );
    return () => {
      document.documentElement.style.zoom = "";
      document.documentElement.style.removeProperty("--ui-scale");
    };
  }, [display.uiScale]);
  const libraryOpen = display.libraryOpen,
    inspector = display.inspector,
    wideKeys = display.range === 88,
    seconds = display.previewBars;
  const setLibraryOpen = (value: boolean) =>
    setDisplay((previous) => ({ ...previous, libraryOpen: value }));
  const setInspector = (value: boolean) =>
    setDisplay((previous) => ({ ...previous, inspector: value }));
  const setWideKeys = (value: boolean) =>
    setDisplay((previous) => ({ ...previous, range: value ? 88 : 49 }));
  const setSeconds = (value: number) =>
    setDisplay((previous) => ({ ...previous, previewBars: value }));
  useEffect(() => {
    try {
      localStorage.setItem(displayStorage, JSON.stringify(display));
    } catch {}
  }, [display]);
  const [state, setState] = useState<Snapshot>(emptyState),
    [songs, setSongs] = useState<SongRow[]>([]),
    [song, setSong] = useState<LoadedSong | null>(null),
    [collection, setCollection] = useState<CollectionSong[]>([]);
  const [devices, setDevices] = useState({
    inputs: [] as string[],
    outputs: [] as string[],
    vst3: [] as string[],
  });
  const [connected, setConnected] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const [query, setQuery] = useState(""),
    [source, setSource] = useState("全部来源"),
    [tab, setTab] = useState<Tab>("全部"),
    [count, setCount] = useState(100);
  const [settings, setSettings] = useState(false),
    [history, setHistory] = useState(false),
    [view, setView] = useState("keyboard"),
    [editFingers, setEditFingers] = useState(false),
    [selectedNote, setSelectedNote] = useState<
      LoadedSong["notes"][number] | null
    >(null),
    [exerciseDialog, setExerciseDialog] = useState(false),
    [feedback, setFeedback] = useState(false),
    [metadata, setMetadata] = useState(false),
    [fingeringDialog, setFingeringDialog] = useState(false),
    [libraryDialog, setLibraryDialog] = useState(false),
    [passageDialog, setPassageDialog] = useState(false),
    [ladderDialog, setLadderDialog] = useState(false),
    [routineDialog, setRoutineDialog] = useState(false),
    [routineLaunch, setRoutineLaunch] = useState<{
      id: string;
      day: string | null;
    } | null>(null),
    [practiceBackup, setPracticeBackup] = useState(false),
    [handsDialog, setHandsDialog] = useState(false),
    [meterDialog, setMeterDialog] = useState(false),
    [scoresDialog, setScoresDialog] = useState(false),
    [paperRevision, setPaperRevision] = useState(0),
    [importDialog, setImportDialog] = useState(false),
    [packageDialog, setPackageDialog] = useState(false),
    [packageFile, setPackageFile] = useState<File | undefined>(),
    [droppedFiles, setDroppedFiles] = useState<File[]>([]),
    [notice, setNotice] = useState(""),
    [calibration, setCalibration] = useState<{
      message: string;
      suggested: number | null;
    } | null>(null);
  const [historyScoreReview, setHistoryScoreReview] =
    useState<HistoryScoreReview | null>(null);
  const [historyScoreFocus, setHistoryScoreFocus] =
    useState<HistoryScoreFocus | null>(null);
  useEffect(() => {
    if (historyScoreFocus && song?.contentId !== historyScoreFocus.contentId)
      setHistoryScoreFocus(null);
  }, [song?.contentId, historyScoreFocus]);
  useEffect(() => {
    if (historyScoreReview && song?.contentId !== historyScoreReview.contentId)
      setHistoryScoreReview(null);
  }, [song?.contentId, historyScoreReview]);
  const recital = ["recital", "memory"].includes(state.mode);
  const recitalLive = recital && state.status !== "finished";
  const ladderActive = Boolean(state.ladder?.active);
  const routineActive = Boolean(state.routine);
  const attemptLocked =
    routineActive ||
    ladderActive ||
    (recital && ["playing", "paused", "countIn"].includes(state.status));
  const completionShown = useRef(false);
  useEffect(() => {
    const finished = recital && state.status === "finished";
    if (finished && !completionShown.current && !history) setFeedback(true);
    completionShown.current = finished;
  }, [recital, state.status, history]);
  useEffect(() => {
    if (state.mode === "memory") {
      setSelectedNote(null);
      setEditFingers(false);
    }
  }, [state.mode]);
  const [loopStart, setLoopStart] = useState(1),
    [loopEnd, setLoopEnd] = useState(0);
  const scoreInput = useRef<HTMLInputElement>(null);
  const stateRef = useRef(state);
  stateRef.current = state;
  const noteQueue = useRef(Promise.resolve()),
    requestBusy = useRef(false);
  const updateCollection = useCallback(async () => {
    const data = await api.command<{ songs: CollectionSong[] }>({
      type: "collection",
    });
    setCollection(data.songs);
  }, []);
  const refresh = useCallback(async () => {
    try {
      const [library, current, inventory] = await Promise.all([
        api.library(),
        api.song(),
        api.devices(),
      ]);
      setSongs(library.songs);
      setSong(current);
      setDevices(inventory);
      setState(await api.state());
      await updateCollection();
      setConnected(true);
      setError("");
    } catch (e) {
      setConnected(false);
      setError(String(e instanceof Error ? e.message : e));
    }
  }, [updateCollection]);
  useEffect(() => {
    let stopped = false,
      lastRevision = -1,
      timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const status = await api.command<{ revision: number }>({
          type: "libraryMonitorStatus",
        });
        if (!stopped && status.revision !== lastRevision) {
          const library = await api.library();
          if (!stopped) {
            setSongs(library.songs);
            lastRevision = status.revision;
          }
        }
      } catch {
        /* The connection indicator is maintained by the engine poll. */
      }
      if (!stopped) timer = setTimeout(poll, 2000);
    };
    void poll();
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, []);
  useEffect(() => {
    void refresh();
  }, [refresh]);
  useEffect(() => {
    let stopped = false,
      timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await api.state();
        if (!stopped) {
          setState(next);
          setConnected(true);
        }
      } catch {
        if (!stopped) setConnected(false);
      }
      if (!stopped) timer = setTimeout(poll, 80);
    };
    void poll();
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, []);
  useEffect(() => {
    setLoopStart(
      song?.measures.find((m) => m.start === state.passage?.start)?.number ?? 1,
    );
    setLoopEnd(
      state.passage
        ? (song?.measures.find((m) => m.end >= state.passage!.end)?.number ?? 1)
        : (song?.measures.length ?? 1),
    );
  }, [
    song?.contentId,
    song?.duration,
    state.passage?.start,
    state.passage?.end,
  ]);
  useEffect(() => {
    setCount(100);
  }, [query, source, tab]);
  useEffect(() => {
    if (song) setWideKeys(song.notes.some((n) => n.pitch < 36 || n.pitch > 84));
  }, [song?.contentId]);
  useEffect(() => {
    if (!song?.fingerActions?.length) return;
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void api
        .song()
        .then((current) => {
          if (!cancelled && current?.contentId === song.contentId)
            setSong(current);
        })
        .catch(() => {});
    }, 120);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [state.speed, song?.contentId, !!song?.fingerActions?.length]);
  const run = useCallback(
    async (value: EngineCommand, changesSong = false) => {
      if (requestBusy.current) return;
      requestBusy.current = true;
      setBusy(true);
      try {
        await api.command(value);
        if (changesSong) setSong(await api.song());
        setState(await api.state());
        if (
          [
            "load",
            "openRecent",
            "favorite",
            "queue",
            "moveQueue",
            "save",
            "generate",
            "recording",
          ].includes(value.type)
        )
          await updateCollection();
        setError("");
      } catch (e) {
        setError(String(e instanceof Error ? e.message : e));
      } finally {
        requestBusy.current = false;
        setBusy(false);
      }
    },
    [updateCollection],
  );
  const action = (value: EngineCommand) => {
    if (["speed", "volume", "seek"].includes(value.type)) {
      pendingAdjust.current.set(value.type, value);
      void drainAdjust();
    } else
      void run(
        value,
        [
          "track",
          "trackSound",
          "trackAppearance",
          "hands",
          "finger",
          "pairScore",
        ].includes(value.type),
      );
  };
  const pendingAdjust = useRef(new Map<string, EngineCommand>());
  const adjusting = useRef(false);
  const drainAdjust = async () => {
    if (adjusting.current) return;
    adjusting.current = true;
    try {
      while (pendingAdjust.current.size) {
        const [key, value] = pendingAdjust.current.entries().next().value!;
        pendingAdjust.current.delete(key);
        await api.command(value);
        setState(await api.state());
      }
      setError("");
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      adjusting.current = false;
    }
  };
  const onNote = useCallback(
    (pitch: number, active: boolean) => {
      if (!connected) return;
      noteQueue.current = noteQueue.current
        .then(async () => {
          await api.command({
            type: "note",
            pitch,
            active,
            velocity: active ? 95 : 0,
          });
        })
        .catch((e) => setError(String(e.message ?? e)));
    },
    [connected],
  );
  const shortcutQueue = useRef<Promise<unknown>>(Promise.resolve());
  useEffect(() => {
    const keys = keyboardMap(display);
    const held = new Set<number>();
    const clear = () => {
      held.forEach((p) => onNote(p, false));
      held.clear();
    };
    const down = (e: KeyboardEvent) => {
      const zoomChord = eventShortcut(e),
        zoomAction = zoomChord
          ? shortcutActions.find(
              (a) =>
                display.shortcuts[a.id] === zoomChord &&
                ["zoomIn", "zoomOut", "zoomReset"].includes(a.id),
            )
          : undefined;
      if (zoomAction && !e.repeat && !e.isComposing && !e.defaultPrevented) {
        e.preventDefault();
        setDisplay((p) => ({
          ...p,
          uiScale:
            zoomAction.id === "zoomReset"
              ? 100
              : Math.max(
                  80,
                  Math.min(
                    150,
                    p.uiScale + (zoomAction.id === "zoomIn" ? 10 : -10),
                  ),
                ),
        }));
        return;
      }

      if (
        settings ||
        displayDialog ||
        history ||
        exerciseDialog ||
        feedback ||
        metadata ||
        fingeringDialog ||
        libraryDialog ||
        passageDialog ||
        ladderDialog ||
        routineDialog ||
        practiceBackup ||
        handsDialog ||
        meterDialog ||
        scoresDialog ||
        importDialog ||
        packageDialog ||
        !!document.querySelector(".history-problem-dialog") ||
        e.isComposing ||
        e.repeat
      )
        return;
      if (
        e.target instanceof HTMLElement &&
        (e.target.matches("input,select,textarea") ||
          (e.target.matches("button") &&
            !e.ctrlKey &&
            !e.altKey &&
            !e.shiftKey) ||
          e.target.isContentEditable)
      )
        return;
      const chord = eventShortcut(e),
        shortcut = chord
          ? shortcutActions.find((a) => display.shortcuts[a.id] === chord)
          : undefined;
      if (shortcut) {
        e.preventDefault();
        const current = stateRef.current;
        const disallowed = (c: Snapshot) =>
          [
            "previousMeasure",
            "nextMeasure",
            "slower",
            "faster",
            "metronome",
            "loop",
          ].includes(shortcut.id) &&
          (!!c.routine ||
            !!c.ladder?.active ||
            (shortcut.id !== "metronome" &&
              ["recital", "memory"].includes(c.mode)));
        if (disallowed(current)) {
          setError("当前完整演奏、计划或阶梯中，请用该练习的控制操作。");
          return;
        }
        const execute = (build: (latest: Snapshot) => EngineCommand) => {
          shortcutQueue.current = shortcutQueue.current
            .then(async () => {
              const latest = await api.state();
              if (disallowed(latest)) {
                setError("当前完整演奏、计划或阶梯中，请用该练习的控制操作。");
                return;
              }
              await run(build(latest));
            })
            .catch((e) => setError(e instanceof Error ? e.message : String(e)));
        };
        switch (shortcut.id) {
          case "playPause":
            execute((c) => ({
              type: ["playing", "countIn"].includes(c.status)
                ? "pause"
                : "play",
            }));
            break;
          case "restart":
            execute(() => ({ type: "restart" }));
            break;
          case "silence":
            clear();
            execute(() => ({ type: "pause" }));
            break;
          case "previousMeasure":
          case "nextMeasure":
            execute((c) => ({
              type: "seekMeasure",
              measure: Math.max(
                c.passage?.start ?? 1,
                Math.min(
                  c.passage?.end ?? song?.measures.length ?? 1,
                  c.measure + (shortcut.id === "nextMeasure" ? 1 : -1),
                ),
              ),
            }));
            break;
          case "slower":
          case "faster":
            execute((c) => ({
              type: "speed",
              value: Math.max(
                0.25,
                Math.min(
                  2,
                  Math.round(
                    (c.speed + (shortcut.id === "faster" ? 0.05 : -0.05)) * 100,
                  ) / 100,
                ),
              ),
            }));
            break;
          case "loop":
            execute((c) => ({
              type: "measureLoop",
              enabled: !c.passage,
              start: loopStart,
              end: loopEnd,
            }));
            break;
          case "metronome":
            execute((c) => ({ type: "metronome", enabled: !c.metronome }));
            break;
          case "view":
            setView((v) =>
              v === "keyboard" ? "score" : v === "score" ? "paper" : "keyboard",
            );
            break;
          case "library":
            setDisplay((p) => ({ ...p, libraryOpen: !p.libraryOpen }));
            break;
          case "inspector":
            setDisplay((p) => ({ ...p, inspector: !p.inspector }));
            break;
          case "display":
            setDisplayDialog(true);
            break;
          case "history":
            setFeedback(false);
            void run({ type: "pause" }).then(() => setHistory(true));
            break;
          case "passages":
            void run({ type: "pause" }).then(() => setPassageDialog(true));
            break;
          case "libraryManager":
            void run({ type: "pause" }).then(() => setLibraryDialog(true));
            break;
        }
        return;
      }
      if (e.ctrlKey || e.metaKey || e.altKey || e.shiftKey) return;
      const pitch = display.keyboardEnabled ? keys[e.code] : undefined;
      if (pitch !== undefined) {
        e.preventDefault();
        held.add(pitch);
        onNote(pitch, true);
      }
    };
    const up = (e: KeyboardEvent) => {
      const p = keys[e.code];
      if (p !== undefined && held.delete(p)) onNote(p, false);
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    window.addEventListener("blur", clear);
    return () => {
      clear();
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      window.removeEventListener("blur", clear);
    };
  }, [
    display.keyCodes,
    display.shortcuts,
    loopStart,
    loopEnd,
    song?.measures.length,
    display.keyboardOctave,
    display.keyboardEnabled,
    onNote,
    run,
    settings,
    displayDialog,
    history,
    exerciseDialog,
    feedback,
    metadata,
    fingeringDialog,
    libraryDialog,
    passageDialog,
    ladderDialog,
    routineDialog,
    practiceBackup,
    handsDialog,
    meterDialog,
    scoresDialog,
    importDialog,
    packageDialog,
  ]);
  useEffect(() => {
    if (
      !settings &&
      !displayDialog &&
      !history &&
      !exerciseDialog &&
      !feedback &&
      !metadata &&
      !fingeringDialog &&
      !libraryDialog &&
      !importDialog &&
      !packageDialog &&
      !scoresDialog &&
      !meterDialog &&
      !handsDialog &&
      !passageDialog &&
      !ladderDialog &&
      !routineDialog &&
      !practiceBackup
    )
      return;
    const previous = document.activeElement as HTMLElement | null;
    const trap = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (
          document.querySelector(
            '.practice-backup-dialog button[aria-label="关闭练习资料备份"]:disabled, .history-workspace button[aria-label="关闭练习历史"]:disabled, .ladder-dialog button[aria-label="关闭速度阶梯"]:disabled, .routine-dialog button[aria-label="关闭练习计划"]:disabled, .scores-dialog button[aria-label="关闭谱面管理"]:disabled, .library-manager button[aria-label="关闭曲库管理"]:disabled',
          )
        ) {
          e.preventDefault();
          return;
        }
        setSettings(false);
        setHistory(false);
        setExerciseDialog(false);
        setFeedback(false);
        setMetadata(false);
        setFingeringDialog(false);
        setLibraryDialog(false);
        setPassageDialog(false);
        setLadderDialog(false);
        setRoutineDialog(false);
        setRoutineLaunch(null);
        setPracticeBackup(false);
        setHandsDialog(false);
        setMeterDialog(false);
        setScoresDialog(false);
        if (
          !document.querySelector(
            '.package-dialog button[aria-label="关闭曲目包"]:disabled',
          )
        )
          setPackageDialog(false);
        if (
          !document.querySelector(
            '.import-dialog button[aria-label="关闭导入"]:disabled',
          )
        )
          setImportDialog(false);
      }
      if (e.key !== "Tab") return;
      const dialog = [
        ...document.querySelectorAll<HTMLElement>('[role="dialog"]'),
      ]
        .filter((element) => element.getClientRects().length > 0)
        .at(-1);
      const items = [
        ...(dialog?.querySelectorAll<HTMLElement>(
          "button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),a[href],summary,[tabindex]",
        ) ?? []),
      ].filter(
        (element) =>
          element.tabIndex >= 0 && element.getClientRects().length > 0,
      );
      const first = items[0],
        last = items.at(-1);
      if (!first) return;
      if (!dialog?.contains(document.activeElement)) {
        e.preventDefault();
        first.focus();
        return;
      }
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last?.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", trap);
    return () => {
      window.removeEventListener("keydown", trap);
      previous?.focus();
    };
  }, [
    settings,
    displayDialog,
    history,
    exerciseDialog,
    feedback,
    metadata,
    fingeringDialog,
    libraryDialog,
    passageDialog,
    ladderDialog,
    routineDialog,
    practiceBackup,
    handsDialog,
    meterDialog,
    scoresDialog,
    importDialog,
    packageDialog,
  ]);
  const selectedRow = useMemo(
    () => songs.find((r) => normalize(r.path) === normalize(song?.sourcePath)),
    [songs, song?.sourcePath],
  );
  const currentRecord = collection.find((r) => r.contentId === song?.contentId);
  const byPath = useMemo(
    () =>
      new Map(
        collection.map((r) => [
          normalize(r.path || `exercise:${r.contentId}`),
          r,
        ]),
      ),
    [collection],
  );
  const queue = useMemo(
    () =>
      collection
        .filter((r) => r.queuePosition !== null)
        .sort((a, b) => a.queuePosition! - b.queuePosition!),
    [collection],
  );
  const allLibraryRows = useMemo(() => {
    let rows = [...songs];
    const known = new Set(rows.map((r) => normalize(r.path)));
    for (const r of collection)
      if (
        (r.path || r.generated) &&
        !known.has(normalize(r.path || `exercise:${r.contentId}`))
      )
        rows.push({
          id: r.contentId,
          title: r.title,
          path: r.path || `exercise:${r.contentId}`,
          composer: r.generated ? "技术练习" : "本地导入",
          source: r.generated ? "技术练习" : "本地导入",
          category: "",
          license: "请确认文件使用权限",
          sourceUrl: "",
        });
    return rows;
  }, [songs, collection]);
  const libraryRows = useMemo(() => {
    let rows = allLibraryRows.filter((row) => {
      const r = byPath.get(normalize(row.path));
      if (tab === "收藏" && !r?.favorite) return false;
      if (tab === "最近" && !r) return false;
      if (tab === "队列" && r?.queuePosition == null) return false;
      return (
        (source === "全部来源" || row.source === source) &&
        query
          .toLowerCase()
          .trim()
          .split(/\s+/)
          .every((t) =>
            `${row.title} ${row.composer} ${row.source} ${row.category} ${(row.tags ?? []).join(" ")} ${row.difficulty ?? ""}`
              .toLowerCase()
              .includes(t),
          )
      );
    });
    if (tab === "队列")
      rows.sort(
        (a, b) =>
          byPath.get(normalize(a.path))!.queuePosition! -
          byPath.get(normalize(b.path))!.queuePosition!,
      );
    if (tab === "最近")
      rows.sort(
        (a, b) =>
          byPath.get(normalize(b.path))!.lastUsed -
          byPath.get(normalize(a.path))!.lastUsed,
      );
    return rows;
  }, [allLibraryRows, byPath, query, source, tab]);
  const sources = useMemo(
    () => [
      "全部来源",
      ...new Set([...songs.map((r) => r.source), "本地导入", "技术练习"]),
    ],
    [songs],
  );
  const load = async (row: SongRow) => {
    const record = byPath.get(normalize(row.path));
    if (record?.available === false && api.desktop) {
      try {
        const restored = await api.relocate(record.contentId);
        if (restored) {
          setSong(restored);
          setState(await api.state());
          await updateCollection();
          setError("");
        }
      } catch (e) {
        setError(String(e));
      }
      return;
    }
    await run(
      record
        ? { type: "openRecent", content_id: record.contentId }
        : { type: "load", path: row.path, title: row.title },
      true,
    );
    if (window.innerWidth < 1000) setLibraryOpen(false);
  };
  const importMidi = async () => {
    if (!api.desktop) {
      setImportDialog(true);
      return;
    }
    if (requestBusy.current) return;
    requestBusy.current = true;
    setBusy(true);
    try {
      const data = await api.import();
      if (data) {
        setSong(data);
        setState(await api.state());
        await updateCollection();
        setError("");
        setNotice(data.importWarnings?.join("；") ?? "");
      }
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      requestBusy.current = false;
      setBusy(false);
    }
  };
  const next =
    queue[queue.findIndex((r) => r.contentId === song?.contentId) + 1];
  const judged =
      state.score.matched_notes +
      state.score.wrong_notes +
      state.score.missed_notes,
    accuracy = judged
      ? Math.round((state.score.matched_notes / judged) * 100)
      : null;
  const status = !connected
    ? "引擎未连接"
    : state.recording
      ? `正在录制 ${clock(state.recordDuration)}`
      : state.status === "countIn"
        ? `预备拍 ${Math.ceil(state.countRemaining)}`
        : state.status === "finished"
          ? "本次练习完成"
          : state.status === "playing"
            ? state.wait && state.required.length
              ? "等待你弹奏"
              : "正在播放"
            : state.status === "paused"
              ? "已暂停"
              : song
                ? "准备开始"
                : "未选择曲目";
  const exportMidi = async () => {
    try {
      if (api.desktop) {
        await api.export();
        return;
      }
      const result = await api.command<{ bytes: number[]; title: string }>({
        type: "exportMidi",
      });
      const blob = new Blob([new Uint8Array(result.bytes)], {
        type: "audio/midi",
      });
      const url = URL.createObjectURL(blob),
        a = document.createElement("a");
      a.href = url;
      a.download = `${result.title}.mid`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    }
  };
  const fullscreen = () => {
    if (document.fullscreenElement) void document.exitFullscreen();
    else
      void document.documentElement
        .requestFullscreen()
        .catch(() => setError("当前窗口无法进入全屏"));
  };
  return (
    <div
      className={`workbench ${libraryOpen ? "with-library" : ""} ${inspector ? "with-inspector" : ""}`}
      onDragOver={(e) => {
        if (e.dataTransfer.types.includes("Files")) e.preventDefault();
      }}
      onDrop={(e) => {
        e.preventDefault();
        if (state.recording || busy) {
          setNotice(
            state.recording
              ? "请先停止录音，再导入文件"
              : "请等待当前操作完成，再导入文件",
          );
          return;
        }
        if (document.querySelector('[role="dialog"]')) {
          setNotice("请先关闭当前窗口，再拖入文件；导入窗口内可以继续添加文件");
          return;
        }
        const files = Array.from(e.dataTransfer.files);
        if (files.length === 1 && /\.neopiece$/i.test(files[0].name)) {
          setPackageFile(files[0]);
          setPackageDialog(true);
          return;
        }
        setDroppedFiles(files);
        setImportDialog(true);
      }}
    >
      <header className="menubar">
        <button
          className="app-name"
          onClick={() => setLibraryOpen(!libraryOpen)}
          title="显示或隐藏曲库"
        >
          <Music2 size={19} />
          Neothesia
        </button>
        <button onClick={() => setImportDialog(true)}>导入曲目</button>
        <button
          disabled={state.recording || busy}
          onClick={() => {
            setPackageFile(undefined);
            setPackageDialog(true);
          }}
        >
          曲目包
        </button>
        <button onClick={() => void importMidi()}>
          <FolderOpen size={15} />
          打开文件
        </button>
        <button
          onClick={() => {
            action({ type: "pause" });
            setLibraryDialog(true);
          }}
        >
          曲库管理
        </button>
        {api.desktop && (
          <button
            disabled={busy}
            onClick={() =>
              void api
                .importFolder()
                .then(refresh)
                .catch((e) => setError(String(e.message)))
            }
          >
            添加文件夹
          </button>
        )}
        <button
          disabled={busy || ladderActive || routineActive}
          onClick={() =>
            void run(
              { type: "recording", active: !state.recording },
              !state.recording ? false : true,
            )
          }
        >
          {state.recording
            ? `停止录音 ${clock(state.recordDuration)}`
            : "录制演奏"}
        </button>
        <button onClick={() => void exportMidi()} disabled={!song || busy}>
          导出 MIDI
        </button>
        <button onClick={() => setExerciseDialog(true)}>
          <Music2 size={15} />
          技术练习
        </button>
        <button
          disabled={busy || state.recording}
          onClick={() => {
            action({ type: "pause" });
            setRoutineDialog(true);
          }}
        >
          练习计划
        </button>
        <button
          onClick={() => {
            setFeedback(false);
            setHistory(true);
          }}
        >
          <History size={15} />
          练习历史
        </button>
        <button
          onClick={() => {
            setSettings(true);
            void api
              .devices()
              .then(setDevices)
              .catch((e) => setError(String(e.message)));
          }}
        >
          <Settings2 size={15} />
          设备设置
        </button>
        <span className="menu-spacer" />
        <span className={`engine-indicator ${connected ? "connected" : ""}`}>
          <i />
          {connected ? "本地引擎已连接" : "引擎未连接"}
        </span>
        <button className="icon-button" aria-label="全屏" onClick={fullscreen}>
          <Maximize2 size={16} />
        </button>
      </header>
      {notice && (
        <div className="app-notice" role="status">
          <span>{notice}</span>
          <button aria-label="关闭导入提示" onClick={() => setNotice("")}>
            关闭
          </button>
        </div>
      )}
      {(error || state.error) && (
        <div className="error-strip" role="alert">
          <span>{error || state.error}</span>
          <button onClick={() => void refresh()}>重新连接</button>
          <button aria-label="关闭错误提示" onClick={() => setError("")}>
            <X size={14} />
          </button>
        </div>
      )}
      <div
        className="workspace"
        style={
          {
            "--library-width": `${display.libraryWidth}px`,
            "--inspector-width": `${display.inspectorWidth}px`,
            "--keyboard-height": `${display.keyboardHeight}px`,
            "--paper-preview-height": `${display.paperPreviewHeight}px`,
          } as React.CSSProperties
        }
      >
        {libraryOpen && (
          <aside className="library-panel">
            <PanelResizeHandle
              label="调整曲库宽度"
              value={display.libraryWidth}
              min={180}
              max={440}
              change={(v) => setDisplay((p) => ({ ...p, libraryWidth: v }))}
            />
            <div className="panel-heading">
              <strong>曲库</strong>
              <span>{songs.length.toLocaleString()} 首</span>
              <button
                className="icon-button"
                aria-label="隐藏曲库"
                onClick={() => setLibraryOpen(false)}
              >
                <ChevronLeft size={15} />
              </button>
            </div>
            <div className="library-tabs">
              {(["全部", "收藏", "最近", "队列"] as Tab[]).map((value) => (
                <button
                  key={value}
                  className={tab === value ? "selected" : ""}
                  onClick={() => setTab(value)}
                >
                  {value}
                  {value === "队列" && queue.length > 0 ? (
                    <small>{queue.length}</small>
                  ) : null}
                </button>
              ))}
            </div>
            <div className="library-filters">
              <label className="search-box">
                <Search size={14} />
                <input
                  aria-label="搜索曲目"
                  placeholder="搜索曲名、作曲家"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                />
                {query && (
                  <button aria-label="清空搜索" onClick={() => setQuery("")}>
                    <X size={13} />
                  </button>
                )}
              </label>
              <select
                aria-label="曲库来源"
                value={source}
                onChange={(e) => setSource(e.target.value)}
              >
                {sources.map((s) => (
                  <option key={s}>{s}</option>
                ))}
              </select>
            </div>
            <div className="library-list" role="list" aria-label="曲目列表">
              {libraryRows.slice(0, count).map((row, index) => {
                const record = byPath.get(normalize(row.path));
                return (
                  <button
                    role="listitem"
                    key={row.id}
                    className={`song-row ${normalize(row.path) === normalize(song?.sourcePath || (song?.generated ? `exercise:${song.contentId}` : "")) ? "active" : ""}`}
                    disabled={busy}
                    onClick={() => void load(row)}
                    title={`${row.title}\n${row.composer}\n${row.source}`}
                  >
                    <span className="song-index">
                      {tab === "队列" ? index + 1 : <Music2 size={14} />}
                    </span>
                    <span className="song-text">
                      <strong>{record?.title || row.title}</strong>
                      <small>
                        {record?.available === false
                          ? "文件已丢失"
                          : record?.review?.due
                            ? "待复习 · " + (row.composer || row.source)
                            : row.composer || row.source}
                      </small>
                    </span>
                    {record?.favorite && <Star size={12} fill="currentColor" />}
                  </button>
                );
              })}
              {libraryRows.length === 0 && (
                <div className="list-empty">
                  {tab === "队列"
                    ? "还没有练习队列。选择曲目后点击「加入队列」。"
                    : tab === "收藏"
                      ? "还没有收藏。打开曲目后点击星标。"
                      : "没有找到曲目"}
                </div>
              )}
              {libraryRows.length > count && (
                <button
                  className="load-more"
                  onClick={() => setCount(count + 100)}
                >
                  继续加载（{libraryRows.length - count}）
                </button>
              )}
            </div>
            <div className="library-footer">
              <span>
                {libraryRows.length.toLocaleString()} 首
                {tab !== "全部" ? ` · ${tab}` : ""}
              </span>
              <button title="重新读取曲库" onClick={() => void refresh()}>
                <RotateCcw size={13} />
              </button>
            </div>
          </aside>
        )}
        <main className="practice-panel">
          <div className="song-toolbar">
            {!libraryOpen && (
              <button
                className="icon-button"
                aria-label="显示曲库"
                onClick={() => setLibraryOpen(true)}
              >
                <Menu size={18} />
              </button>
            )}
            <div className="current-song">
              <h1>{song?.title || "选择曲目"}</h1>
              <span>
                {selectedRow?.composer ||
                  (song ? "本地 MIDI" : "打开本地 MIDI 或选择内置练习")}
              </span>
            </div>
            <button
              disabled={!song || song.generated || busy}
              onClick={() => setMetadata(true)}
              title="编辑曲目信息"
            >
              信息
            </button>
            <button
              className={`icon-button ${currentRecord?.favorite ? "is-favorite" : ""}`}
              aria-label={currentRecord?.favorite ? "取消收藏" : "收藏曲目"}
              disabled={!song || busy}
              onClick={() =>
                action({ type: "favorite", enabled: !currentRecord?.favorite })
              }
            >
              <Star
                size={17}
                fill={currentRecord?.favorite ? "currentColor" : "none"}
              />
            </button>
            <button
              className={
                currentRecord?.queuePosition != null ? "is-queued" : ""
              }
              disabled={!song || busy}
              onClick={() =>
                action({
                  type: "queue",
                  enabled: currentRecord?.queuePosition == null,
                })
              }
            >
              <ListMusic size={16} />
              {currentRecord?.queuePosition != null ? "移出队列" : "加入队列"}
            </button>
            <button
              className="icon-button"
              aria-label="练习参数"
              onClick={() => setInspector(!inspector)}
            >
              <Settings2 size={17} />
            </button>
          </div>
          <div className="view-toolbar">
            <button
              className={view === "keyboard" ? "selected" : ""}
              onClick={() => setView("keyboard")}
            >
              键盘
            </button>
            <button
              className={view === "score" ? "selected" : ""}
              onClick={() => setView("score")}
            >
              乐谱
            </button>
            <button
              className={view === "paper" ? "selected" : ""}
              disabled={!song}
              onClick={() => setView("paper")}
            >
              谱页
            </button>
            <button
              disabled={!song || busy}
              onClick={() => scoreInput.current?.click()}
            >
              配对乐谱
            </button>
            <button
              disabled={!song || busy}
              onClick={() => setScoresDialog(true)}
            >
              谱面管理
            </button>
            <input
              ref={scoreInput}
              type="file"
              accept=".musicxml,.xml,.mxl"
              aria-label="配对乐谱文件"
              hidden
              onChange={async (e) => {
                const file = e.target.files?.[0],
                  id = song?.contentId;
                e.target.value = "";
                if (!file || !id) return;
                if (file.size > 4_000_000) {
                  setError("乐谱超过 4 MB");
                  return;
                }
                try {
                  const bytes = Array.from(
                    new Uint8Array(await file.arrayBuffer()),
                  );
                  await run(
                    {
                      type: "pairScore",
                      bytes,
                      name: file.name,
                      content_id: id,
                    },
                    true,
                  );
                  setView("score");
                } catch (err) {
                  setError(String(err));
                }
              }}
            />
            <button
              className={editFingers ? "selected" : ""}
              disabled={!song || recital || ladderActive || routineActive}
              onClick={() => {
                setEditFingers(!editFingers);
                setSelectedNote(null);
              }}
            >
              编辑指法
            </button>
            <button
              disabled={
                !song || busy || recital || ladderActive || routineActive
              }
              onClick={() => {
                action({ type: "pause" });
                setFingeringDialog(true);
              }}
            >
              指法建议
            </button>
            <span className="view-title">
              <KeyboardMusic size={15} />
              键盘提示
            </span>
            <span
              className="view-legend"
              title={song?.tracks
                .filter((t) => t.visible)
                .map((t) => t.name)
                .join(" · ")}
            >
              {(display.palette === "accessible" ||
                !song ||
                song.tracks.some((t) => t.visible && !t.color)) && (
                <>
                  <i
                    className="low"
                    style={{ background: handColor("left", display) }}
                  />
                  左手
                  <i
                    className="high"
                    style={{ background: handColor("right", display) }}
                  />
                  {display.palette === "accessible" ? "右手" : "右手 / 未分手"}
                  {display.palette === "accessible" && (
                    <>
                      <i style={{ background: handColor("other", display) }} />
                      未分手
                    </>
                  )}
                </>
              )}
              {display.palette === "standard" &&
                song?.tracks
                  .filter((t) => t.color && t.visible)
                  .slice(0, 2)
                  .map((t) => (
                    <span className="track-legend" key={t.id} title={t.name}>
                      <i style={{ background: t.color! }} />
                      {t.name}
                    </span>
                  ))}
            </span>
            <span className="menu-spacer" />
            <button
              aria-label="显示与电脑键盘设置"
              onClick={() => setDisplayDialog(true)}
            >
              显示设置
            </button>
            <label>
              显示
              <select
                aria-label="键盘显示范围"
                value={wideKeys ? "88" : "49"}
                onChange={(e) => setWideKeys(e.target.value === "88")}
              >
                <option value="49">49 键</option>
                <option value="88">88 键</option>
              </select>
            </label>
            <label>
              预览
              <select
                aria-label="预览小节数"
                value={seconds}
                onChange={(e) => setSeconds(Number(e.target.value))}
              >
                {[2, 4, 8, 12].map((n) => (
                  <option value={n} key={n}>
                    {n} 小节
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="piano-stage">
            <Piano
              song={state.mode === "memory" ? null : song}
              state={
                state.mode === "memory" ? { ...state, required: [] } : state
              }
              onNote={onNote}
              fullRange={wideKeys}
              previewBars={seconds}
              display={display}
              onSelect={editFingers && !recital ? setSelectedNote : undefined}
            />
            {view === "score" && song && state.mode !== "memory" && (
              <Notation
                openHistoryPlan={(id, day) => {
                  setRoutineLaunch({ id, day });
                  setHistoryScoreReview(null);
                  setRoutineDialog(true);
                }}
                historyReview={historyScoreReview}
                clearHistoryReview={() => setHistoryScoreReview(null)}
                practiceHistoryReview={async (reference, before, after) => {
                  if (!historyScoreReview) return;
                  try {
                    const result = await api.command<{
                      scoreFocus?: HistoryScoreFocus | null;
                      restoreWarning?: string | null;
                    }>({
                      type: "historyPracticeRange",
                      content_id: historyScoreReview.contentId,
                      id: historyScoreReview.historyId,
                      reference,
                      before,
                      after,
                    });
                    await refresh();
                    if (result.restoreWarning)
                      throw new Error(result.restoreWarning);
                    setHistoryScoreReview(null);
                    setHistoryScoreFocus(
                      result.scoreFocus
                        ? { ...result.scoreFocus, request: crypto.randomUUID() }
                        : null,
                    );
                    setFeedback(false);
                  } catch (e) {
                    await refresh();
                    throw e;
                  }
                }}
                historyFocus={historyScoreFocus}
                clearHistoryFocus={() => setHistoryScoreFocus(null)}
                song={song}
                state={state}
                seek={(position) => action({ type: "seek", position })}
                changed={async () => {
                  setSong(await api.song());
                  setState(await api.state());
                }}
                detail={(note) => {
                  setSelectedNote(note);
                  setFingeringDialog(true);
                }}
                assignHands={() => setHandsDialog(true)}
                edit={editFingers && !recital}
                canEdit={!recital}
                onEdit={setEditFingers}
              />
            )}
            {view === "paper" && song && state.mode !== "memory" && (
              <PaperScore
                contentId={song.contentId}
                revision={paperRevision}
                playback={{ measure: state.measure }}
                contextKey={JSON.stringify(song.meterCorrection)}
              />
            )}
            {selectedNote && view !== "score" && state.mode !== "memory" && (
              <div className="finger-editor">
                <span>{pitchName(selectedNote.pitch)} · 指法</span>
                <button
                  aria-label="关闭指法编辑"
                  onClick={() => setSelectedNote(null)}
                >
                  <X size={12} />
                </button>
                <div>
                  {[1, 2, 3, 4, 5].map((finger) => (
                    <button
                      key={finger}
                      onClick={() => {
                        void run(
                          {
                            type: "finger",
                            track: selectedNote.track,
                            index: selectedNote.index,
                            finger,
                          },
                          true,
                        );
                        setSelectedNote(null);
                      }}
                    >
                      {finger}
                    </button>
                  ))}
                  <button
                    onClick={() => {
                      void run(
                        {
                          type: "finger",
                          track: selectedNote.track,
                          index: selectedNote.index,
                          finger: null,
                        },
                        true,
                      );
                      setSelectedNote(null);
                    }}
                  >
                    清除
                  </button>
                </div>
              </div>
            )}
            <div className="stage-status">
              <span
                className={`transport-dot ${["playing", "countIn"].includes(state.status) ? "playing" : ""}`}
              />
              {status} · 第 {state.measure} 小节 · 第 {Math.floor(state.beat)}{" "}
              拍{state.passage && <span>循环 {state.repetitions + 1}</span>}
            </div>
            {state.mode === "memory" && (
              <div className="recital-caption">
                背谱：目标音符、谱面和指法提示已隐藏
              </div>
            )}
            {state.mode === "wait" && state.required.length > 0 && (
              <div className="required-notes">
                <span>请弹</span>
                <strong>
                  {state.required
                    .map((pitch) => displayPitch(pitch, display))
                    .join(" + ")}
                </strong>
              </div>
            )}
            {!song && (
              <div className="stage-empty">
                <Music2 size={34} />
                <p>从曲库选择一首曲目</p>
                <button
                  disabled={!songs.length || busy}
                  onClick={() => songs[0] && void load(songs[0])}
                >
                  五指热身 · 开始练习
                </button>
                <button onClick={() => void importMidi()}>
                  打开 MIDI 文件
                </button>
              </div>
            )}
          </div>
          {state.routine && (
            <RoutineStatus
              state={state}
              manage={() => setRoutineDialog(true)}
              end={() => action({ type: "stopRoutine" })}
              next={(r) => {
                void run({ type: "stopRoutine" }).then(() =>
                  run(
                    {
                      type: "openRoutineItem",
                      routine_id: r.routineId,
                      day: r.day,
                      item_id: r.nextItemId,
                      resume: true,
                    },
                    true,
                  ),
                );
              }}
            />
          )}
          {state.ladder && (
            <LadderStatus
              state={state}
              action={action}
              manage={() => setLadderDialog(true)}
            />
          )}
          {song && (
            <BarNavigator
              song={song}
              state={state}
              start={loopStart}
              end={loopEnd}
              disabled={
                busy || state.recording || ladderActive || routineActive
              }
              recital={recital}
              select={(a, b) => {
                setLoopStart(a);
                setLoopEnd(b);
              }}
              command={action}
            />
          )}
          <div className="timeline">
            <span
              aria-label="本轮运行用时"
              title="按真实运行时间，不含暂停和预备拍，含等音等待；保存结果后进入历史。"
            >
              {practiceDuration(state.practiceMs ?? 0)}
            </span>
            <span title={clock(state.position)}>第 {state.measure} 小节</span>
            <input
              aria-label="播放位置"
              type="range"
              min={state.passage ? loopStart : 1}
              max={state.passage ? loopEnd : (song?.measures.length ?? 1)}
              step="1"
              value={state.measure}
              disabled={
                !song || busy || recital || ladderActive || routineActive
              }
              onChange={(e) =>
                action({
                  type: "seekMeasure",
                  measure: Number(e.target.value),
                })
              }
            />
            <span title={clock(song?.duration || 0)}>
              {song?.measures.length || 0} 小节
            </span>
          </div>
          <div className="transport-bar">
            <button
              className="icon-button"
              title={`从头开始 · ${shortcutLabel(display.shortcuts.restart)}`}
              aria-label="从头开始"
              disabled={!song || busy}
              onClick={() => action({ type: "restart" })}
            >
              <RotateCcw size={18} />
            </button>
            <button
              className="play-button"
              title={`播放 / 暂停 · ${shortcutLabel(display.shortcuts.playPause)}`}
              disabled={!song || busy || !connected}
              onClick={() =>
                action({
                  type: ["playing", "countIn"].includes(state.status)
                    ? "pause"
                    : "play",
                })
              }
            >
              {["playing", "countIn"].includes(state.status) ? (
                <Pause size={17} fill="currentColor" />
              ) : (
                <Play size={17} fill="currentColor" />
              )}
              {["playing", "countIn"].includes(state.status)
                ? "暂停"
                : "开始练习"}
            </button>
            <button
              className="icon-button"
              aria-label="队列下一首"
              disabled={!next || busy}
              onClick={() =>
                next &&
                void run(
                  { type: "openRecent", content_id: next.contentId },
                  true,
                )
              }
            >
              <ChevronRight size={20} />
            </button>
            <span className="transport-divider" />
            <label className="speed-control">
              速度
              <input
                aria-label="播放速度"
                type="range"
                min="25"
                max="200"
                step="5"
                value={state.speed * 100}
                disabled={
                  busy ||
                  (recital &&
                    ["playing", "paused", "countIn"].includes(state.status))
                }
                onChange={(e) =>
                  action({ type: "speed", value: Number(e.target.value) / 100 })
                }
              />
              <output>{Math.round(state.speed * 100)}%</output>
            </label>
            <span className="menu-spacer" />
            <label
              className="volume-control"
              title={
                state.output === "内置钢琴"
                  ? "内置钢琴音量"
                  : "外部音源请在对应软件调整音量"
              }
            >
              <Volume2 size={17} />
              <input
                aria-label="钢琴音量"
                type="range"
                min="0"
                max="100"
                value={state.volume * 100}
                disabled={state.output !== "内置钢琴" || busy}
                onChange={(e) =>
                  action({
                    type: "volume",
                    value: Number(e.target.value) / 100,
                  })
                }
              />
            </label>
          </div>
        </main>
        {inspector && (
          <aside className="practice-inspector">
            <PanelResizeHandle
              label="调整练习参数宽度"
              value={display.inspectorWidth}
              min={180}
              max={360}
              reverse
              change={(v) => setDisplay((p) => ({ ...p, inspectorWidth: v }))}
            />
            <div className="panel-heading">
              <strong>练习</strong>
              <button
                className="icon-button"
                aria-label="隐藏练习参数"
                onClick={() => setInspector(false)}
              >
                <X size={14} />
              </button>
            </div>
            <section className="inspector-section">
              <h2>模式</h2>
              <div className="mode-buttons">
                {[
                  ["wait", "等音"],
                  ["flow", "连续"],
                  ["listen", "聆听"],
                  ["recital", "完整演奏"],
                  ["memory", "背谱"],
                ].map(([mode, label]) => (
                  <button
                    key={mode}
                    aria-pressed={state.mode === mode}
                    className={state.mode === mode ? "selected" : ""}
                    disabled={busy || ladderActive || routineActive}
                    onClick={() => action({ type: "mode", value: mode })}
                  >
                    {label}
                  </button>
                ))}
              </div>
              <p className="parameter-help">
                {state.mode === "wait"
                  ? "弹对后继续，不考核节奏"
                  : state.mode === "flow"
                    ? "按拍演奏，记录错音、漏音和节奏"
                    : state.mode === "recital"
                      ? "从曲首演奏一遍，结束后评分"
                      : state.mode === "memory"
                        ? "隐藏谱面、音符与指法提示，结束后评分"
                        : "自动演奏已启用的音轨"}
              </p>
            </section>
            <section className="inspector-section">
              <h2>声部与音轨</h2>
              <select
                aria-label="练习声部"
                value={state.hands}
                disabled={busy || !song || attemptLocked}
                onChange={(e) =>
                  action({ type: "hands", value: e.target.value })
                }
              >
                <option value="both">双手 / 全部练习音轨</option>
                <option value="left">左手 · 右手伴奏</option>
                <option value="right">右手 · 左手伴奏</option>
                <option value="custom" disabled>
                  自定义音轨
                </option>
              </select>
              {song?.tracks.map((t) => (
                <div
                  className="track-control"
                  key={`${song.contentId}:${t.id}`}
                >
                  <div>
                    <span className="track-name" title={t.name}>
                      <i
                        className="track-color-dot"
                        style={{
                          background:
                            t.color ??
                            (t.part === "left" ? "#6cb7f3" : "#a9df74"),
                        }}
                      />
                      {t.name}
                    </span>
                    <small>
                      {t.notes} 音符
                      {t.inferred && t.part !== "other" ? " · 自动分手" : ""}
                    </small>
                    <input
                      type="checkbox"
                      aria-label={`${t.name}显示`}
                      disabled={busy || attemptLocked}
                      checked={t.visible}
                      onChange={(e) =>
                        action({
                          type: "track",
                          ...t,
                          visible: e.target.checked,
                        })
                      }
                    />
                  </div>
                  <div>
                    <select
                      aria-label={`${t.name}模式`}
                      value={t.mode}
                      disabled={busy || attemptLocked}
                      onChange={(e) =>
                        action({ type: "track", ...t, mode: e.target.value })
                      }
                    >
                      <option value="human">自己弹</option>
                      <option value="auto">伴奏</option>
                      <option value="mute">静音</option>
                    </select>
                    <select
                      aria-label={`${t.name}声部`}
                      value={t.part}
                      disabled={busy || attemptLocked}
                      onChange={(e) =>
                        action({ type: "track", ...t, part: e.target.value })
                      }
                    >
                      <option value="other">未分手</option>
                      <option value="left">左手</option>
                      <option value="right">右手</option>
                    </select>
                  </div>
                  <TrackAppearanceControl
                    track={t}
                    disabled={busy}
                    action={action}
                  />
                  <TrackSoundControl
                    track={t}
                    sound={song.trackSounds?.[String(t.id)]}
                    presets={song.soundPresets}
                    disabled={busy || attemptLocked}
                    builtin={state.output === "内置钢琴"}
                    action={action}
                  />
                </div>
              ))}
              {song?.tracks.some((t) => t.part === "other") && (
                <p className="parameter-help">
                  单轨曲目可以逐音分手；音轨声部作为未单独标注音符的默认值。
                </p>
              )}
            </section>
            <button
              disabled={
                !song || busy || attemptLocked || state.mode === "memory"
              }
              onClick={() => setHandsDialog(true)}
            >
              逐音分手
            </button>
            <section className="inspector-section timing-controls">
              <h2>节拍</h2>
              <button
                disabled={!song || busy || attemptLocked}
                onClick={() => setMeterDialog(true)}
              >
                修正小节网格
              </button>
              <div className="tempo-readout">
                {Math.round(state.bpm * state.speed)}{" "}
                <small>
                  BPM · {song?.measures[state.measure - 1]?.numerator ?? 4}/
                  {song?.measures[state.measure - 1]?.denominator ?? 4}
                </small>
              </div>
              <label>
                预备拍
                <select
                  aria-label="预备拍"
                  disabled={attemptLocked}
                  value={state.countIn}
                  onChange={(e) =>
                    action({ type: "countIn", bars: Number(e.target.value) })
                  }
                >
                  {[0, 1, 2, 4].map((n) => (
                    <option key={n} value={n}>
                      {n === 0 ? "关闭" : `${n} 小节`}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <input
                  aria-label="节拍器"
                  disabled={ladderActive || routineActive}
                  type="checkbox"
                  checked={state.metronome}
                  onChange={(e) =>
                    action({ type: "metronome", enabled: e.target.checked })
                  }
                />
                节拍器
              </label>
              <label>
                <input
                  aria-label="自动调整速度"
                  disabled={recital || ladderActive || routineActive}
                  type="checkbox"
                  checked={state.adaptive}
                  onChange={(e) =>
                    action({ type: "adaptive", enabled: e.target.checked })
                  }
                />
                循环后自动调整速度
              </label>
              {song?.measures.some((m) => !m.explicit) && (
                <p className="parameter-help">
                  * 文件缺少拍号，暂按 4/4 显示。
                </p>
              )}
            </section>
            <section
              className="inspector-section loop-section"
              aria-disabled={recital || ladderActive || routineActive}
            >
              <div className="section-heading">
                <h2>分段循环</h2>
                <button
                  role="switch"
                  aria-label="分段循环"
                  aria-checked={Boolean(state.passage)}
                  className={`toggle ${state.passage ? "on" : ""}`}
                  disabled={
                    !song || busy || recital || ladderActive || routineActive
                  }
                  onClick={() =>
                    action({
                      type: "measureLoop",
                      enabled: !state.passage,
                      start: loopStart,
                      end: loopEnd,
                    })
                  }
                >
                  <i />
                </button>
              </div>
              <label>
                起点
                <span className="number-with-unit">
                  <input
                    aria-label="循环起始小节"
                    type="number"
                    step="1"
                    min="1"
                    max={song?.measures.length ?? 1}
                    value={loopStart}
                    onChange={(e) => setLoopStart(Number(e.target.value))}
                  />
                  <small>小节</small>
                </span>
                <button
                  disabled={!song || recital || ladderActive || routineActive}
                  title="使用当前位置作为起点"
                  onClick={() => setLoopStart(state.measure)}
                >
                  设 A
                </button>
              </label>
              <label>
                终点
                <span className="number-with-unit">
                  <input
                    aria-label="循环结束小节"
                    type="number"
                    step="1"
                    min="1"
                    max={song?.measures.length ?? 1}
                    value={loopEnd}
                    onChange={(e) => setLoopEnd(Number(e.target.value))}
                  />
                  <small>小节</small>
                </span>
                <button
                  disabled={!song || recital || ladderActive || routineActive}
                  title="使用当前位置作为终点"
                  onClick={() => setLoopEnd(state.measure)}
                >
                  设 B
                </button>
              </label>
              <button
                className="apply-loop"
                disabled={
                  !song || busy || recital || ladderActive || routineActive
                }
                onClick={() =>
                  action({
                    type: "measureLoop",
                    enabled: true,
                    start: loopStart,
                    end: loopEnd,
                  })
                }
              >
                {state.passage ? "应用范围" : "开始循环"}
              </button>
              <button
                disabled={
                  !song || busy || recital || ladderActive || routineActive
                }
                onClick={() => {
                  action({ type: "pause" });
                  setPassageDialog(true);
                }}
              >
                练习段落
              </button>
              <label>
                遍数
                <select
                  aria-label="循环遍数"
                  disabled={recital || ladderActive || routineActive}
                  value={state.rounds}
                  onChange={(e) =>
                    action({ type: "rounds", count: Number(e.target.value) })
                  }
                >
                  {[0, 1, 3, 5, 10].map((n) => (
                    <option key={n} value={n}>
                      {n === 0 ? "不限" : `${n} 遍`}
                    </option>
                  ))}
                </select>
              </label>
              {state.passage && (
                <span className="loop-progress" aria-live="polite">
                  已完成 {state.repetitions} 轮
                </span>
              )}
            </section>
            <section className="inspector-section">
              <h2>速度阶梯</h2>
              <button
                disabled={!song || busy || state.recording || recital}
                onClick={() => {
                  action({ type: "pause" });
                  setLadderDialog(true);
                }}
              >
                速度阶梯方案
              </button>
              <p className="parameter-help">
                按连续达标轮数逐级提速，保存每个方案的进度。
              </p>
            </section>
            {currentRecord?.recommended && (
              <section className="inspector-section">
                <h2>建议重练</h2>
                <button
                  disabled={busy || routineActive || ladderActive}
                  onClick={() =>
                    action({
                      type: "measureLoop",
                      start: currentRecord.recommended![0],
                      end: Math.min(
                        currentRecord.recommended![1],
                        song?.measures.length ?? 1,
                      ),
                      enabled: true,
                    })
                  }
                >
                  第 {currentRecord.recommended[0]}–
                  {Math.min(
                    currentRecord.recommended[1],
                    song?.measures.length ?? 1,
                  )}{" "}
                  小节
                </button>
                <p className="parameter-help">根据多次练习记录确定。</p>
              </section>
            )}
            {!recitalLive && (
              <section className="inspector-section results">
                <h2>本次结果</h2>
                <div className="accuracy">
                  <strong>{accuracy === null ? "—" : `${accuracy}%`}</strong>
                  <span>音符准确率</span>
                </div>
                <dl>
                  <div>
                    <dt>正确音符</dt>
                    <dd>{state.score.matched_notes}</dd>
                  </div>
                  <div>
                    <dt>错音</dt>
                    <dd>{state.score.wrong_notes}</dd>
                  </div>
                  <div>
                    <dt>漏音</dt>
                    <dd>{state.score.missed_notes}</dd>
                  </div>
                </dl>
                <button
                  disabled={
                    !judged ||
                    state.saved ||
                    busy ||
                    ladderActive ||
                    routineActive
                  }
                  onClick={() => action({ type: "save" })}
                >
                  {state.saved ? "已保存" : "保存结果"}
                </button>
                <button
                  disabled={!judged && !state.summary}
                  onClick={() => setFeedback(true)}
                >
                  逐小节反馈
                </button>
                {state.saved && (
                  <span className="saved-message">结果已保存在本机</span>
                )}
              </section>
            )}
            {recitalLive && (
              <section className="inspector-section">
                <h2>{state.mode === "memory" ? "背谱演奏" : "完整演奏"}</h2>
                <p className="parameter-help">
                  完成后显示准确率、节奏和逐小节反馈。
                </p>
                {state.mode === "memory" && <p>谱面与指法提示已隐藏</p>}
              </section>
            )}
            {currentRecord?.queuePosition != null && (
              <section className="inspector-section queue-controls">
                <h2>队列顺序</h2>
                <span>
                  第 {currentRecord.queuePosition + 1} 首 / {queue.length} 首
                </span>
                <div>
                  <button
                    disabled={currentRecord.queuePosition === 0 || busy}
                    onClick={() => action({ type: "moveQueue", direction: -1 })}
                  >
                    上移
                  </button>
                  <button
                    disabled={
                      currentRecord.queuePosition === queue.length - 1 || busy
                    }
                    onClick={() => action({ type: "moveQueue", direction: 1 })}
                  >
                    下移
                  </button>
                </div>
              </section>
            )}
          </aside>
        )}
      </div>
      <footer className="statusbar">
        <span>
          <KeyboardMusic size={13} />
          {state.input
            ? `${state.input}${state.inputConnected ? " · 已连接" : " · 未连接"}`
            : display.keyboardEnabled
              ? `电脑键盘 · ${display.keyCodes.map(codeLabel).join(" ")} · C${display.keyboardOctave}`
              : "电脑键盘演奏已关闭"}
        </span>
        <span>{state.output}</span>
        <span>{state.pedal >= 64 ? "延音踏板踩下" : ""}</span>
        {selectedRow?.source === "GiantMIDI-Piano" && (
          <span className="transcription-note">
            GiantMIDI · 自动转录，尚未校对
          </span>
        )}
        <span className="menu-spacer" />
        {selectedRow && (
          <details className="source-info">
            <summary>来源与许可</summary>
            <div>
              <strong>{selectedRow.source}</strong>
              <p>{selectedRow.license}</p>
              <p>{selectedRow.sourceUrl}</p>
            </div>
          </details>
        )}
        <button onClick={() => action({ type: "panic" })}>停止发声</button>
        <span>
          {display.shortcuts.playPause
            ? `${shortcutLabel(display.shortcuts.playPause)}：播放 / 暂停`
            : "播放快捷键未分配"}
        </span>
      </footer>
      {packageDialog && (
        <PackageDialog
          song={song}
          initialFile={packageFile}
          close={() => setPackageDialog(false)}
          changed={refresh}
        />
      )}
      {importDialog && (
        <ImportDialog
          initialFiles={droppedFiles}
          close={() => {
            setImportDialog(false);
            setDroppedFiles([]);
          }}
          changed={refresh}
        />
      )}
      {scoresDialog && song && (
        <ScoresDialog
          song={song}
          close={() => {
            setScoresDialog(false);
            setPaperRevision((v) => v + 1);
          }}
          changed={async () => {
            setSong(await api.song());
            setState(await api.state());
            await updateCollection();
          }}
        />
      )}
      {meterDialog && song && (
        <MeterDialog
          song={song}
          close={() => setMeterDialog(false)}
          changed={async () => {
            setSong(await api.song());
            setState(await api.state());
            setLoopStart(1);
            setLoopEnd(1);
            await updateCollection();
          }}
        />
      )}
      {handsDialog && song && (
        <HandsDialog
          song={song}
          start={loopStart}
          end={loopEnd}
          close={() => setHandsDialog(false)}
          changed={async () => {
            setSong(await api.song());
            setState(await api.state());
            await updateCollection();
          }}
        />
      )}
      {practiceBackup && (
        <PracticeBackupDialog
          close={() => setPracticeBackup(false)}
          changed={refresh}
        />
      )}
      {routineDialog && (
        <RoutineDialog
          initialPlan={routineLaunch}
          song={song}
          songs={songs}
          state={state}
          close={() => {
            setRoutineDialog(false);
            setRoutineLaunch(null);
          }}
          backup={() => {
            setRoutineDialog(false);
            setPracticeBackup(true);
          }}
          changed={refresh}
        />
      )}
      {ladderDialog && song && (
        <LadderDialog
          song={song}
          state={state}
          start={loopStart}
          end={loopEnd}
          close={() => setLadderDialog(false)}
          changed={refresh}
        />
      )}
      {passageDialog && song && (
        <PassageDialog
          song={song}
          state={state}
          start={loopStart}
          end={loopEnd}
          close={() => setPassageDialog(false)}
          changed={async () => {
            setSong(await api.song());
            setState(await api.state());
            await updateCollection();
          }}
        />
      )}
      {libraryDialog && (
        <LibraryDialog
          rows={allLibraryRows}
          close={() => setLibraryDialog(false)}
          changed={refresh}
          openVersion={async (row, contentId, kind, id) => {
            await api.command({
              type: "openLibraryScore",
              path: row.path,
              content_id: contentId,
              kind,
              id,
            });
            setSong(await api.song());
            setState(await api.state());
            setView(kind === "paper" ? "paper" : "score");
            setPaperRevision((v) => v + 1);
            await updateCollection();
            setLibraryDialog(false);
          }}
          open={async (row) => {
            await load(row);
            setLibraryDialog(false);
          }}
        />
      )}
      {fingeringDialog && song && (
        <FingeringDialog
          song={song}
          note={selectedNote}
          start={loopStart}
          end={loopEnd}
          close={() => setFingeringDialog(false)}
          changed={async () => {
            setSong(await api.song());
            setState(await api.state());
            await updateCollection();
          }}
        />
      )}
      {metadata && song && (
        <MetadataDialog
          close={() => setMetadata(false)}
          title={song.title}
          save={async (value) => {
            await run({ type: "setMetadata", value }, true);
            await updateCollection();
            setMetadata(false);
          }}
        />
      )}
      {exerciseDialog && (
        <ExerciseDialog
          close={() => setExerciseDialog(false)}
          generate={async (spec) => {
            await run({ type: "generate", spec }, true);
            setExerciseDialog(false);
          }}
        />
      )}
      {feedback && (
        <FeedbackDialog
          close={() => setFeedback(false)}
          state={state}
          retry={async () => {
            await run({ type: "restart" });
            setFeedback(false);
            await run({ type: "play" });
          }}
          practice={async (start, end) => {
            if (state.routine) await run({ type: "stopRoutine" });
            else if (ladderActive) await run({ type: "stopLadder" });
            if (recital) await run({ type: "mode", value: "flow" });
            await run({ type: "measureLoop", start, end, enabled: true });
            setFeedback(false);
          }}
        />
      )}
      {history && (
        <HistoryDialog
          openPlan={(id, day) => {
            setHistory(false);
            setRoutineLaunch({ id, day });
            setRoutineDialog(true);
          }}
          review={(data) => {
            setHistoryScoreReview(data);
            setHistoryScoreFocus(null);
            setView("score");
            setEditFingers(false);
            setSelectedNote(null);
            setFeedback(false);
          }}
          score={(focus) => {
            setHistoryScoreReview(null);
            setHistoryScoreFocus({ ...focus, request: crypto.randomUUID() });
            setView("score");
            setEditFingers(false);
            setSelectedNote(null);
          }}
          close={() => {
            setHistory(false);
            setFeedback(false);
          }}
          changed={async () => {
            await refresh();
            setFeedback(false);
          }}
          backup={() => {
            setHistory(false);
            setPracticeBackup(true);
          }}
        />
      )}
      {displayDialog && (
        <DisplaySettingsDialog
          value={display}
          change={setDisplay}
          close={() => setDisplayDialog(false)}
        />
      )}
      {settings && (
        <div className="modal-backdrop" onClick={() => setSettings(false)}>
          <section
            className="settings-dialog device-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="device-title"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="dialog-heading">
              <h2 id="device-title">设备设置</h2>
              <button
                autoFocus
                className="icon-button"
                aria-label="关闭设备设置"
                onClick={() => setSettings(false)}
              >
                <X size={19} />
              </button>
            </div>
            {state.output.startsWith("VST3") && (
              <button onClick={() => action({ type: "vstEditor" })}>
                打开音源窗口
              </button>
            )}
            <button
              onClick={() =>
                void api
                  .command<{ message: string; suggested: number | null }>({
                    type: "calibration",
                  })
                  .then(setCalibration)
                  .catch((e) => setError(String(e.message)))
              }
            >
              估计输入延迟
            </button>
            {calibration && (
              <p>
                {calibration.message}
                {calibration.suggested !== null && (
                  <button
                    onClick={() => {
                      action({
                        type: "latency",
                        milliseconds: calibration.suggested,
                      });
                      setCalibration(null);
                    }}
                  >
                    应用补偿
                  </button>
                )}
              </p>
            )}
            <label>
              输入延迟补偿（毫秒）
              <input
                aria-label="输入延迟补偿"
                type="number"
                min="-250"
                max="250"
                value={state.latency}
                onChange={(e) =>
                  action({
                    type: "latency",
                    milliseconds: Number(e.target.value),
                  })
                }
              />
            </label>
            <label>
              MIDI 输入
              <select
                aria-label="MIDI 输入"
                value={state.input || ""}
                disabled={busy}
                onChange={(e) =>
                  action({ type: "input", name: e.target.value || null })
                }
              >
                <option value="">电脑键盘 / 屏幕键盘</option>
                {state.input && !devices.inputs.includes(state.input) && (
                  <option value={state.input}>{state.input}（未连接）</option>
                )}
                {devices.inputs.map((n) => (
                  <option key={n}>{n}</option>
                ))}
              </select>
            </label>
            <label>
              演奏音源
              <select
                aria-label="演奏音源"
                value={
                  state.output.startsWith("VST3")
                    ? `vst3:${
                        devices.vst3.find(
                          (p) =>
                            state.output ===
                            `VST3 · ${p
                              .split(/[\\/]/)
                              .pop()
                              ?.replace(/\.vst3$/i, "")}`,
                        ) || ""
                      }`
                    : state.output === "内置钢琴"
                      ? ""
                      : state.output
                }
                disabled={busy}
                onChange={(e) =>
                  action({ type: "output", name: e.target.value || null })
                }
              >
                <option value="">内置钢琴</option>
                {devices.vst3.map((path) => (
                  <option key={path} value={`vst3:${path}`}>
                    VST3 ·{" "}
                    {path
                      .split(/[\\/]/)
                      .pop()
                      ?.replace(/\.vst3$/i, "")}
                  </option>
                ))}
                {devices.outputs.map((n) => (
                  <option key={n}>{n}</option>
                ))}
              </select>
            </label>
            <p>MIDI 键盘断开时会暂停播放。重新连接后，点击开始练习继续。</p>
            <p>
              可直接选择本机 VST3 乐器。选择 Pianoteq
              后，点击「打开音源窗口」调整音色。
            </p>
            <div className="dialog-actions">
              <button
                onClick={() =>
                  void api
                    .devices()
                    .then(setDevices)
                    .catch((e) => setError(String(e.message)))
                }
              >
                刷新设备
              </button>
              <button className="primary" onClick={() => setSettings(false)}>
                完成
              </button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
