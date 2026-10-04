import { PaperSourceUpdatesDialog } from "./PaperSourceUpdatesDialog";
import { LibraryMetadataHistoryDialog, type MetadataHistoryContext } from "./LibraryMetadataHistoryDialog";
import { LibraryMetadataBatchDialog, type MetadataBatchItem } from "./LibraryMetadataBatchDialog";
import { FileLocationsDialog } from "./FileLocationsDialog";
import {
  LibraryMetadataEditor,
  type MetadataEdit,
} from "./LibraryMetadataEditor";
import {
  LibraryContextMenu,
  copyLibraryPaths,
  type LibraryMenuAction,
} from "./LibraryContextMenu";
import { PracticeTimePanel } from "./PracticeTimePanel";
import { SongLearningDialog } from "./SongLearningDialog";
import {
  learningStages,
  learningMatches,
  compareLearning,
  activeDeadline,
  localDay,
  type SongLearning,
} from "./songLearning";
import { ScoreSourcesDialog } from "./ScoreSourcesDialog";
import { useEffect, useMemo, useRef, useState } from "react";
import { LibraryScoresPane } from "./LibraryScoresPane";
import { LibraryRepairsDialog } from "./LibraryRepairsDialog";
import { X, ChevronLeft, ChevronRight } from "lucide-react";
import { api, clock, type SongRow } from "./api";
import type { CollectionSong, LibraryMonitorStatus } from "./api";
import {
  compare,
  defaults,
  matches,
  key,
  readView,
  terms,
  storage,
  type Metadata,
  type Entry,
  type View,
} from "./libraryQuery";
type Group = { id: string; name: string; parent: string | null };
type IndexProgress = {
  status: string;
  done: number;
  total: number;
  errors: number;
  current: string;
  message: string;
};
type Workspace = {
  groups: Group[];
  entries: Record<string, Entry>;
  revision: number;
  learning?: Record<string, SongLearning>;
};
export function LibraryDialog({
  rows,
  close,
  changed,
  open,
  openVersion,
}: {
  rows: SongRow[];
  close: () => void;
  changed: () => Promise<void>;
  open: (row: SongRow) => Promise<void>;
  openVersion: (
    row: SongRow,
    contentId: string,
    kind: "notation" | "paper",
    id: string,
  ) => Promise<void>;
}) {
  const [data, setData] = useState<Workspace>({
      groups: [],
      entries: {},
      revision: 0,
    }),
    [folders, setFolders] = useState<{ path: string; available: boolean }[]>(
      [],
    );
  const [paperSourcesOpen,setPaperSourcesOpen]=useState(false);
  const [sourcesOpen, setSourcesOpen] = useState(false);
  const [metadataHistory, setMetadataHistory] = useState<MetadataHistoryContext | null>(null);
  const [locationsOpen, setLocationsOpen] = useState(false);
  const [metadataBatch, setMetadataBatch] = useState<MetadataBatchItem[] | null>(null);
  const [menu, setMenu] = useState<{
    row: SongRow;
    index: number;
    x: number;
    y: number;
    items: { row: SongRow; cid: string | null }[];
  } | null>(null);
  const [fileActionResult, setFileActionResult] = useState("");
  const [collectionWorking, setCollectionWorking] = useState(false);
  const collectionStop = useRef(false);
  const [batchStage, setBatchStage] = useState("learning"),
    [learningResult, setLearningResult] = useState("");
  const [learningEdit, setLearningEdit] = useState<{
    path: string;
    contentId: string;
    title: string;
    previous: SongLearning | null;
  } | null>(null);
  const today = localDay();
  const [view, setView] = useState(readView),
    [page, setPage] = useState(0),
    [group, setGroup] = useState("");
  const {
    query,
    field,
    filter,
    sort,
    direction,
    minRating,
    minDuration,
    maxDuration,
    practice,
    learning,
  } = view;
  const updateView = (v: Partial<View>) => setView((old) => ({ ...old, ...v }));
  const [collection, setCollection] = useState<CollectionSong[]>([]);
  useEffect(() => {
    try {
      localStorage.setItem(storage, JSON.stringify(view));
    } catch {}
  }, [view]);
  const [selected, setSelected] = useState<Set<string>>(new Set()),
    [active, setActive] = useState<string | null>(null),
    [name, setName] = useState(""),
    [parent, setParent] = useState("");
  const [target, setTarget] = useState(""),
    [rating, setRating] = useState("0"),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [background, setBackground] = useState<IndexProgress | null>(null),
    [edit, setEdit] = useState<MetadataEdit | null>(null);
  const [repairsOpen, setRepairsOpen] = useState(false);
  const [monitor, setMonitor] = useState<LibraryMonitorStatus | null>(null);
  const monitorChanging = useRef(false);
  const anchor = useRef<number | null>(null);
  const rangeBase = useRef<Set<string>>(new Set());
  const reload = async () => {
    const [d, f, c] = await Promise.all([
      api.command<Workspace>({ type: "libraryWorkspace" }),
      api.command<{ folders: typeof folders }>({ type: "libraryFolders" }),
      api.command<{ songs: CollectionSong[] }>({ type: "collection" }),
    ]);
    setCollection(c.songs);
    setData(d);
    setFolders(f.folders);
  };
  useEffect(() => {
    let stopped = false,
      timer: ReturnType<typeof setTimeout> | undefined,
      lastReload = 0,
      previous = "",
      monitorRevision = -1;
    const poll = async () => {
      try {
        const [progress, watch] = await Promise.all([
          api.command<IndexProgress>({ type: "indexStatus" }),
          api.command<LibraryMonitorStatus>({ type: "libraryMonitorStatus" }),
        ]);
        if (stopped) return;
        setBackground(progress);
        if (!monitorChanging.current) setMonitor(watch);
        if (
          previous !== progress.status ||
          ((progress.status === "running" ||
            watch.revision !== monitorRevision) &&
            Date.now() - lastReload > 2000)
        ) {
          await reload();
          lastReload = Date.now();
          monitorRevision = watch.revision;
        }
        previous = progress.status;
      } catch (e) {
        if (!stopped) setError(e instanceof Error ? e.message : String(e));
      }
      if (!stopped) timer = setTimeout(poll, 750);
    };
    void poll();
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, []);
  useEffect(() => {
    setPage(0);
    anchor.current = null;
    rangeBase.current = new Set();
  }, [view, group]);
  const work = async (job: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await job();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  const duplicates = useMemo(() => {
    const counts = new Map<string, number>();
    for (const e of Object.values(data.entries)) {
      if (e.contentId && e.available)
        counts.set(e.contentId, (counts.get(e.contentId) ?? 0) + 1);
    }
    return new Set([...counts].filter(([, n]) => n > 1).map(([id]) => id));
  }, [data]);
  const groupName = (id: string): string => {
    const parts: string[] = [],
      seen = new Set<string>();
    let g = data.groups.find((g) => g.id === id);
    while (g && !seen.has(g.id)) {
      seen.add(g.id);
      parts.unshift(g.name);
      g = data.groups.find((p) => p.id === g!.parent);
    }
    return parts.join(" / ");
  };
  const groups = [...data.groups].sort((a, b) =>
    groupName(a.id).localeCompare(groupName(b.id), "zh"),
  );
  const historyById = useMemo(
    () => new Map(collection.map((h) => [h.contentId, h])),
    [collection],
  );
  const historyByPath = useMemo(
    () =>
      new Map(collection.filter((h) => h.path).map((h) => [key(h.path!), h])),
    [collection],
  );
  const historyFor = (r: SongRow) => {
    const id = data.entries[key(r.path)]?.contentId;
    // An indexed replacement must never inherit grades from its old path.
    return id ? historyById.get(id) : historyByPath.get(key(r.path));
  };
  const filtered = useMemo(() => {
    const ids = new Set([group]);
    let expand = true;
    while (expand) {
      expand = false;
      for (const g of data.groups) {
        if (g.parent && ids.has(g.parent) && !ids.has(g.id)) {
          ids.add(g.id);
          expand = true;
        }
      }
    }
    const tokens = terms(query);
    return rows
      .filter((r) => {
        const e = data.entries[key(r.path)],
          m = e?.metadata;
        if (group && !(e?.groups ?? []).some((id) => ids.has(id))) return false;
        if (
          (filter === "score" &&
            !e?.hasScore &&
            !e?.notationCount &&
            !e?.paperCount) ||
          (filter === "missing" && e?.available !== false) ||
          (filter === "duplicates" &&
            (!e?.contentId || !duplicates.has(e.contentId))) ||
          (filter === "unindexed" && e?.contentId) ||
          (filter === "errors" && !e?.error)
        )
          return false;
        const h = historyFor(r);
        if (
          !learningMatches(
            e?.contentId ? data.learning?.[e.contentId] : undefined,
            learning,
            today,
          )
        )
          return false;
        if ((e?.rating ?? r.rating ?? 0) < minRating) return false;
        if (
          minDuration !== "" &&
          (!e?.contentId || e.duration < Number(minDuration) * 60)
        )
          return false;
        if (
          maxDuration !== "" &&
          (!e?.contentId || e.duration > Number(maxDuration) * 60)
        )
          return false;
        if (
          (practice === "played" && !h?.sessions) ||
          (practice === "unplayed" && !!h?.sessions) ||
          (practice === "favorite" && !h?.favorite) ||
          (practice === "review" && !h?.review?.due)
        )
          return false;
        return matches(r, e, field, tokens);
      })
      .sort(
        (a, b) =>
          (sort === "learningStage" || sort === "learningDue"
            ? compareLearning(
                data.learning?.[data.entries[key(a.path)]?.contentId ?? ""],
                data.learning?.[data.entries[key(b.path)]?.contentId ?? ""],
                sort,
                direction,
              )
            : 0) ||
          compare(
            a,
            b,
            data.entries[key(a.path)],
            data.entries[key(b.path)],
            historyFor(a),
            historyFor(b),
            sort,
            direction,
          ),
      );
  }, [rows, data, view, group, duplicates, historyById, historyByPath, today]);
  useEffect(() => {
    setPage((p) =>
      Math.min(p, Math.max(0, Math.ceil(filtered.length / 50) - 1)),
    );
  }, [filtered.length]);
  const shown = filtered.slice(page * 50, (page + 1) * 50),
    picked = rows.filter(
      (r) => selected.has(key(r.path)) && !r.path.startsWith("exercise:"),
    );
  const learningPicked = [
    ...new Map(
      picked.flatMap((row) => {
        const cid = data.entries[key(row.path)]?.contentId;
        return cid ? [[cid, { row, cid }] as const] : [];
      }),
    ).values(),
  ];
  const current = rows.find((r) => key(r.path) === active),
    entry = active ? data.entries[active] : null;
  const inspect = async (list: SongRow[], force = false) => {
    const paths = list
      .filter((r) => !r.path.startsWith("exercise:"))
      .map((r) => r.path);
    const progress = await api.command<IndexProgress>({
      type: "indexLibrary",
      paths,
      force,
    });
    setBackground(progress);
  };
  const choose = (
    row: SongRow,
    index: number,
    event: Pick<React.MouseEvent, "shiftKey" | "ctrlKey" | "metaKey">,
  ) => {
    if (busy) return;
    if (edit) {
      setError("请先保存或取消信息编辑，再选择曲目。");
      return;
    }
    setActive(key(row.path));
    setEdit(null);
    if (event.shiftKey && anchor.current !== null) {
      const a = Math.min(index, anchor.current),
        b = Math.max(index, anchor.current);
      setSelected(
        new Set([
          ...rangeBase.current,
          ...filtered.slice(a, b + 1).map((r) => key(r.path)),
        ]),
      );
    } else if (event.ctrlKey || event.metaKey) {
      setSelected((s) => {
        const n = new Set(s);
        n.has(key(row.path)) ? n.delete(key(row.path)) : n.add(key(row.path));
        rangeBase.current = new Set(n);
        return n;
      });
      anchor.current = index;
    } else {
      setSelected(new Set([key(row.path)]));
      rangeBase.current = new Set();
      anchor.current = index;
    }
  };
  const toggle = (row: SongRow) =>
    setSelected((s) => {
      const n = new Set(s);
      n.has(key(row.path)) ? n.delete(key(row.path)) : n.add(key(row.path));
      return n;
    });
  const mutate = async (value: Parameters<typeof api.command>[0]) => {
    await api.command(value);
    await reload();
    await changed();
  };
  const focusIndex = useRef<number | null>(null),
    tableRef = useRef<HTMLTableElement>(null);
  useEffect(() => {
    if (focusIndex.current !== null) {
      tableRef.current
        ?.querySelector<HTMLTableRowElement>(
          `tr[data-index="${focusIndex.current}"]`,
        )
        ?.focus();
      focusIndex.current = null;
    }
  }, [page]);
  const moveFocus = (index: number, event: React.KeyboardEvent) => {
    const next = Math.max(0, Math.min(filtered.length - 1, index));
    if (!filtered[next]) return;
    choose(filtered[next], next, event);
    if (Math.floor(next / 50) !== page) {
      focusIndex.current = next;
      setPage(Math.floor(next / 50));
    } else
      tableRef.current
        ?.querySelector<HTMLTableRowElement>(`tr[data-index="${next}"]`)
        ?.focus();
  };
  const ensureEntry = async (row: SongRow): Promise<Entry> => {
    const cached = data.entries[key(row.path)];
    if (cached?.contentId) return cached;
    const result = await api.command<Entry>({
      type: "inspectSong",
      path: row.path,
      force: true,
    });
    if (!result.contentId)
      throw new Error(result.error ?? "无法识别曲目，请检查文件。");
    return result;
  };
  const editRow = (row: SongRow) =>
    void work(async () => {
      const result = await api.command<{ contentId: string; value: Metadata }>({
        type: "readLibraryMetadata",
        path: row.path,
        content_id: data.entries[key(row.path)]?.contentId ?? null,
      });
      setActive(key(row.path));
      setEdit({
        path: row.path,
        contentId: result.contentId,
        value: result.value,
      });
      await reload();
      requestAnimationFrame(() =>
        document
          .querySelector<HTMLInputElement>(".library-metadata-editor input")
          ?.focus(),
      );
    });
  const learningRow = (row: SongRow) =>
    void work(async () => {
      const e = await ensureEntry(row);
      setLearningEdit({
        path: row.path,
        contentId: e.contentId!,
        title: e.metadata.title ?? row.title,
        previous: data.learning?.[e.contentId!] ?? null,
      });
    });
  const collectionRows = (
    items: { row: SongRow; cid: string | null }[],
    kind: "favorite" | "queued",
    enabled: boolean,
  ) =>
    void work(async () => {
      setCollectionWorking(true);
      collectionStop.current = false;
      let done = 0;
      const errors: string[] = [],
        seen = new Set<string>();
      try {
        for (const [index, { row, cid }] of items.entries()) {
          if (collectionStop.current) break;
          setFileActionResult(`正在处理 ${index + 1} / ${items.length} 首…`);
          try {
            const id = cid ?? (await ensureEntry(row)).contentId!;
            if (seen.has(id)) continue;
            seen.add(id);
            await api.command({
              type: "setLibraryCollection",
              path: row.path,
              content_id: id,
              [kind]: enabled,
            });
            done++;
          } catch (e) {
            errors.push(
              `${row.title}：${e instanceof Error ? e.message : String(e)}`,
            );
          }
        }
        await reload();
        await changed();
        setFileActionResult(
          `${collectionStop.current ? "已停止；" : ""}已${kind === "favorite" ? (enabled ? "收藏" : "取消收藏") : enabled ? "加入队列" : "移出队列"} ${done} 首${errors.length ? `；${errors.length} 首未完成：${errors.join("；")}` : ""}`,
        );
      } finally {
        setCollectionWorking(false);
      }
    });
  const showMenu = (row: SongRow, index: number, x: number, y: number) => {
    if (busy || edit) {
      if (edit) setError("请先保存或取消信息编辑，再打开曲目菜单。");
      return;
    }
    const keep = selected.has(key(row.path));
    if (!keep)
      choose(row, index, { shiftKey: false, ctrlKey: false, metaKey: false });
    else setActive(key(row.path));
    const chosen = keep ? rows.filter((r) => selected.has(key(r.path))) : [row];
    setMenu({
      row,
      index,
      x,
      y,
      items: chosen
        .filter((r) => !r.path.startsWith("exercise:"))
        .map((row) => ({
          row,
          cid: data.entries[key(row.path)]?.contentId ?? null,
        })),
    });
  };
  const closeMenu = (restore: boolean) => {
    const index = menu?.index;
    setMenu(null);
    if (restore && index !== undefined)
      tableRef.current
        ?.querySelector<HTMLTableRowElement>(`tr[data-index="${index}"]`)
        ?.focus();
  };
  useEffect(() => {
    setMenu(null);
  }, [view, group, page, data.revision]);
  useEffect(() => {
    const shortcut = (e: KeyboardEvent) => {
      if (
        learningEdit ||
        sourcesOpen || paperSourcesOpen ||
        locationsOpen ||
        metadataBatch ||
        metadataHistory ||
        repairsOpen ||
        menu ||
        busy ||
        edit ||
        document.querySelectorAll('[role="dialog"]').length > 1
      )
        return;
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        e.stopImmediatePropagation();
        const input = document.querySelector<HTMLInputElement>(
          '[aria-label="管理曲库搜索"]',
        );
        input?.focus();
        input?.select();
      }
    };
    window.addEventListener("keydown", shortcut, true);
    return () => window.removeEventListener("keydown", shortcut, true);
  }, [learningEdit, sourcesOpen, paperSourcesOpen, locationsOpen, metadataBatch, metadataHistory, repairsOpen, menu, busy, edit]);
  const menuActions: LibraryMenuAction[] = menu
    ? [
        { label: "打开练习", hint: "Enter", run: () => void open(menu.row) },
        {
          label: "编辑信息",
          hint: "F2",
          disabled: menu.row.path.startsWith("exercise:"),
          run: () => editRow(menu.row),
        },
        {
          label: "学习目标",
          disabled: menu.row.path.startsWith("exercise:"),
          run: () => learningRow(menu.row),
        },
        {
          label: `收藏${menu.items.length > 1 ? `所选 ${menu.items.length} 首` : "曲目"}`,
          disabled: !menu.items.length || menu.items.length > 100,
          run: () => collectionRows(menu.items, "favorite", true),
        },
        {
          label: "取消收藏",
          disabled: !menu.items.length || menu.items.length > 100,
          run: () => collectionRows(menu.items, "favorite", false),
        },
        {
          label: `加入队列${menu.items.length > 1 ? ` · ${menu.items.length} 首` : ""}`,
          disabled: !menu.items.length || menu.items.length > 100,
          run: () => collectionRows(menu.items, "queued", true),
        },
        {
          label: "移出队列",
          disabled: !menu.items.length || menu.items.length > 100,
          run: () => collectionRows(menu.items, "queued", false),
        },
        {
          label: "加入分组…",
          disabled: !menu.items.length,
          run: () =>
            document
              .querySelector<HTMLSelectElement>('[aria-label="批量目标分组"]')
              ?.focus(),
        },
        {
          label: "检查所选文件",
          disabled: !menu.items.length,
          run: () =>
            void work(() =>
              inspect(
                menu.items.map((i) => i.row),
                true,
              ),
            ),
        },
        {
          label: "打开文件位置",
          disabled: !api.desktop || menu.row.path.startsWith("exercise:"),
          run: () =>
            void work(async () => {
              await api.revealFile(menu.row.path);
              setFileActionResult(
                "已打开文件位置；原件缺失时打开可用的上级文件夹。",
              );
            }),
        },
        {
          label: `复制${menu.items.length > 1 ? "所选文件位置" : "文件位置"}`,
          hint: "Ctrl+C",
          disabled: !menu.items.length,
          run: () =>
            void work(async () => {
              await copyLibraryPaths(menu.items.map((i) => i.row.path));
              setFileActionResult(`已复制 ${menu.items.length} 个文件位置`);
            }),
        },
      ]
    : [];
  const sortHeader = (label: string, column: string) => (
    <th
      aria-sort={
        sort === column
          ? direction === "asc"
            ? "ascending"
            : "descending"
          : "none"
      }
    >
      <button
        className="library-sort-heading"
        onClick={() =>
          updateView({
            sort: column,
            direction:
              sort === column
                ? direction === "asc"
                  ? "desc"
                  : "asc"
                : ["rating", "lastUsed", "sessions"].includes(column)
                  ? "desc"
                  : "asc",
          })
        }
      >
        {label}
        {sort === column ? (direction === "asc" ? " ↑" : " ↓") : ""}
      </button>
    </th>
  );
  if (repairsOpen)
    return (
      <LibraryRepairsDialog
        initialKind={filter === "duplicates" ? "duplicates" : "missing"}
        close={() => setRepairsOpen(false)}
        changed={async () => {
          await reload();
          await changed();
        }}
      />
    );
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!busy && !edit) close();
        else if (edit) setError("请先保存或取消信息编辑，再关闭曲库。");
      }}
    >
      <section
        className="settings-dialog library-manager"
        role="dialog"
        aria-modal="true"
        aria-labelledby="library-manager-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="library-manager-title">曲库管理</h2>
            <p>
              {rows.length.toLocaleString()} 首 · 已索引{" "}
              {Object.values(data.entries)
                .filter((e) => e.contentId)
                .length.toLocaleString()}{" "}
              个文件
            </p>
          </div>
          <button
            autoFocus
            aria-label="关闭曲库管理"
            disabled={busy || !!edit}
            onClick={close}
          >
            <X size={18} />
          </button>
        </div>
        <div className="library-manager-toolbar">
          <select
            aria-label="曲库搜索字段"
            value={field}
            onChange={(e) =>
              updateView({ field: e.target.value as View["field"] })
            }
          >
            {[
              ["all", "全部信息"],
              ["title", "曲名"],
              ["composer", "作曲家"],
              ["artist", "演奏者"],
              ["collection", "曲集"],
              ["tags", "标签"],
              ["difficulty", "难度"],
              ["notes", "备注"],
              ["path", "文件路径"],
              ["source", "来源 / 类别"],
            ].map(([v, l]) => (
              <option key={v} value={v}>
                {l}
              </option>
            ))}
          </select>
          <input
            aria-label="管理曲库搜索"
            placeholder="曲名、作曲家、标签、难度或文件路径"
            value={query}
            onChange={(e) => updateView({ query: e.target.value })}
          />
          <select
            aria-label="曲库管理筛选"
            value={filter}
            onChange={(e) => updateView({ filter: e.target.value })}
          >
            {[
              ["all", "全部文件"],
              ["score", "已配乐谱"],
              ["missing", "文件缺失"],
              ["duplicates", "重复内容"],
              ["unindexed", "尚未索引"],
              ["errors", "需要处理"],
            ].map(([v, l]) => (
              <option key={v} value={v}>
                {l}
              </option>
            ))}
          </select>
          <select
            aria-label="曲库管理排序"
            value={sort}
            onChange={(e) =>
              updateView({
                sort: e.target.value,
                direction: ["lastUsed", "sessions", "rating"].includes(
                  e.target.value,
                )
                  ? "desc"
                  : "asc",
              })
            }
          >
            {[
              ["title", "按曲名"],
              ["composer", "按作曲家"],
              ["difficulty", "按难度"],
              ["rating", "按评级"],
              ["duration", "按时长"],
              ["lastUsed", "按最近练习"],
              ["sessions", "按记录次数"],
              ["measures", "按小节数"],
              ["notes", "按音符数"],
              ["path", "按路径"],
              ["learningStage", "按学习阶段"],
              ["learningDue", "按目标日期"],
            ].map(([v, l]) => (
              <option key={v} value={v}>
                {l}
              </option>
            ))}
          </select>
          <button
            aria-label="切换曲库排序方向"
            title="缺少信息的曲目始终排在最后"
            onClick={() =>
              updateView({ direction: direction === "asc" ? "desc" : "asc" })
            }
          >
            {direction === "asc" ? "升序 ↑" : "降序 ↓"}
          </button>
          <button
            disabled={busy}
            onClick={() => void work(() => mutate({ type: "refreshLibrary" }))}
          >
            立即检查
          </button>
          <button disabled={busy} onClick={() => setRepairsOpen(true)}>
            文件位置
          </button>
          <button disabled={busy} onClick={() => setSourcesOpen(true)}>
            原谱文件
            {monitor?.scoreUpdates
              ? ` · ${monitor.scoreUpdates} 个关联有更新`
              : ""}
          </button>
          <button disabled={busy} onClick={()=>setPaperSourcesOpen(true)}>纸谱原件{monitor?.paperUpdates?` · ${monitor.paperUpdates} 处更新`:""}{monitor?.paperUnavailable?` · ${monitor.paperUnavailable} 处无法读取`:""}</button>
        </div>
        <div className="library-query-options">
          <select
            aria-label="曲库学习阶段筛选"
            value={learning}
            onChange={(event) => updateView({ learning: event.target.value })}
          >
            <option value="all">全部学习阶段</option>
            <option value="untracked">未设置学习资料</option>
            <option value="overdue">目标已逾期</option>
            <option value="today">目标在今天</option>
            {Object.entries(learningStages).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
          <select
            aria-label="练习记录筛选"
            value={practice}
            onChange={(e) => updateView({ practice: e.target.value })}
          >
            {[
              ["all", "所有练习状态"],
              ["played", "已有成绩"],
              ["unplayed", "尚无成绩"],
              ["favorite", "收藏"],
              ["review", "待复习"],
            ].map(([v, l]) => (
              <option key={v} value={v}>
                {l}
              </option>
            ))}
          </select>
          <label>
            评级至少{" "}
            <select
              aria-label="最低评级"
              value={minRating}
              onChange={(e) =>
                updateView({ minRating: Number(e.target.value) })
              }
            >
              {[0, 1, 2, 3, 4, 5].map((n) => (
                <option key={n} value={n}>
                  {n ? `${n} 星` : "不限"}
                </option>
              ))}
            </select>
          </label>
          <label>
            时长{" "}
            <input
              aria-label="最短时长分钟"
              type="number"
              min="0"
              step="0.5"
              placeholder="不限"
              value={minDuration}
              onChange={(e) => updateView({ minDuration: e.target.value })}
            />{" "}
            至{" "}
            <input
              aria-label="最长时长分钟"
              type="number"
              min="0"
              step="0.5"
              placeholder="不限"
              value={maxDuration}
              onChange={(e) => updateView({ maxDuration: e.target.value })}
            />{" "}
            分钟
          </label>
          <button
            onClick={() => {
              setView({ ...defaults });
              setGroup("");
            }}
          >
            清除筛选
          </button>
          <small>多词同时匹配 · 双引号精确短语 · -词 排除</small>
          {minDuration !== "" &&
            maxDuration !== "" &&
            Number(minDuration) > Number(maxDuration) && (
              <span role="alert">最短时长超过最长时长</span>
            )}
        </div>
        {monitor && (
          <div className="library-monitor-status">
            <label>
              <input
                type="checkbox"
                aria-label="自动更新曲库"
                checked={monitor.enabled}
                disabled={busy}
                onChange={(e) =>
                  void work(async () => {
                    const next = await api.command<LibraryMonitorStatus>({
                      type: "setLibraryMonitor",
                      enabled: e.target.checked,
                    });
                    setMonitor(next);
                  })
                }
              />{" "}
              自动更新
            </label>
            <span>
              {monitor.phase === "updating"
                ? `正在更新 ${monitor.current} · 剩余 ${monitor.pending}`
                : monitor.phase === "scanning"
                  ? "正在检查文件变化"
                  : monitor.phase === "failed"
                    ? "更新需要处理"
                    : monitor.enabled
                      ? "文件变化后自动更新"
                      : "自动更新已暂停"}
            </span>
            {monitor.lastScan > 0 && (
              <small>
                上次检查{" "}
                {new Date(monitor.lastScan).toLocaleTimeString("zh-CN")}
              </small>
            )}
            {monitor.errors.length > 0 && (
              <details>
                <summary>{monitor.errors.length} 项需要处理</summary>
                {monitor.errors.map((message, i) => (
                  <p key={i}>{message}</p>
                ))}
              </details>
            )}
          </div>
        )}
        {error && (
          <p role="alert" className="dialog-error">
            {error}
          </p>
        )}
        <div className="library-manager-body">
          <nav className="library-manager-groups" aria-label="曲目分组">
            <button
              className={!group ? "selected" : ""}
              onClick={() => setGroup("")}
            >
              全部曲目
            </button>
            {groups.map((g) => (
              <button
                className={group === g.id ? "selected" : ""}
                key={g.id}
                title={groupName(g.id)}
                onClick={() => setGroup(g.id)}
              >
                {groupName(g.id)}
              </button>
            ))}
            <div className="group-editor">
              <input
                aria-label="新分组名称"
                placeholder="分组名称"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
              <select
                aria-label="分组上级"
                value={parent}
                onChange={(e) => setParent(e.target.value)}
              >
                <option value="">顶层分组</option>
                {groups.map((g) => (
                  <option key={g.id} value={g.id}>
                    {groupName(g.id)}
                  </option>
                ))}
              </select>
              <button
                disabled={busy || !name.trim()}
                onClick={() =>
                  void work(async () => {
                    const r = await api.command<{ id: string }>({
                      type: "libraryGroup",
                      id: null,
                      name,
                      parent: parent || null,
                    });
                    await reload();
                    setGroup(r.id);
                    setName("");
                  })
                }
              >
                新建分组
              </button>
              {group && (
                <>
                  <button
                    disabled={busy || !name.trim()}
                    onClick={() =>
                      void work(async () => {
                        await mutate({
                          type: "libraryGroup",
                          id: group,
                          name,
                          parent: parent || null,
                        });
                        setName("");
                      })
                    }
                  >
                    重命名 / 移动分组
                  </button>
                  <button
                    disabled={busy}
                    onClick={() =>
                      void work(async () => {
                        await mutate({ type: "removeLibraryGroup", id: group });
                        setGroup("");
                      })
                    }
                  >
                    取消此分组
                  </button>
                </>
              )}
            </div>
            <div className="folder-list">
              <h3>曲库文件夹</h3>
              <button disabled={busy || !!edit} onClick={() => setLocationsOpen(true)}>最近文件位置</button>
              {folders.map((f) => (
                <div key={f.path}>
                  <span title={f.path}>{f.path}</span>
                  {!f.available && <small>未连接</small>}
                  <button
                    aria-label={`移除文件夹 ${f.path}`}
                    disabled={busy}
                    onClick={() =>
                      void work(() =>
                        mutate({ type: "removeLibraryFolder", path: f.path }),
                      )
                    }
                  >
                    移除登记
                  </button>
                </div>
              ))}
              {api.desktop && (
                <button
                  disabled={busy}
                  onClick={() =>
                    void work(async () => {
                      await api.importFolder();
                      await changed();
                      await reload();
                    })
                  }
                >
                  添加文件夹
                </button>
              )}
              <p>移除登记只停止扫描，文件和历史仍保留。</p>
            </div>
          </nav>
          <div className="library-manager-center">
            <div className="library-manager-table-wrap">
              <table
                ref={tableRef}
                className="library-manager-table"
                onKeyDown={(event) => {
                  if (
                    (event.ctrlKey || event.metaKey) &&
                    event.key.toLowerCase() === "a"
                  ) {
                    event.preventDefault();
                    setSelected(new Set(filtered.map((r) => key(r.path))));
                  }
                }}
              >
                <thead>
                  <tr>
                    <th>
                      <input
                        type="checkbox"
                        aria-label="选择本页曲目"
                        checked={
                          shown.length > 0 &&
                          shown.every((r) => selected.has(key(r.path)))
                        }
                        disabled={busy}
                        onChange={(e) => {
                          const checked = e.target.checked;
                          setSelected((old) => {
                            const next = new Set(old);
                            for (const row of shown) {
                              if (checked) next.add(key(row.path));
                              else next.delete(key(row.path));
                            }
                            return next;
                          });
                        }}
                      />
                    </th>
                    {sortHeader("曲名 / 标签", "title")}
                    {sortHeader("作曲家", "composer")}
                    {sortHeader("难度", "difficulty")}
                    {sortHeader("时长", "duration")}
                    <th>谱面 / 状态</th>
                    {sortHeader("学习阶段 / 目标", "learningStage")}
                    {sortHeader("评级", "rating")}
                    <th title="每曲最多保留 200 次成绩；日期取最后一条成绩，不含仅打开曲目">
                      练习记录
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {shown.map((r, i) => {
                    const e = data.entries[key(r.path)],
                      m = e?.metadata;
                    return (
                      <tr
                        key={r.path}
                        className={active === key(r.path) ? "active" : ""}
                        data-index={page * 50 + i}
                        tabIndex={0}
                        aria-selected={selected.has(key(r.path))}
                        onClick={(event) => choose(r, page * 50 + i, event)}
                        onDoubleClick={() => {
                          if (!busy && !edit) void open(r);
                        }}
                        onContextMenu={(event) => {
                          event.preventDefault();
                          showMenu(
                            r,
                            page * 50 + i,
                            event.clientX,
                            event.clientY,
                          );
                        }}
                        onKeyDown={(event) => {
                          if (event.target !== event.currentTarget) return;
                          if (busy || edit) return;
                          if (
                            event.key === "ContextMenu" ||
                            (event.shiftKey && event.key === "F10")
                          ) {
                            event.preventDefault();
                            const rect =
                              event.currentTarget.getBoundingClientRect();
                            showMenu(
                              r,
                              page * 50 + i,
                              rect.left + 30,
                              rect.bottom,
                            );
                            return;
                          }
                          if (event.key === "F2") {
                            event.preventDefault();
                            if (!r.path.startsWith("exercise:")) editRow(r);
                            return;
                          }
                          if (
                            (event.ctrlKey || event.metaKey) &&
                            event.key.toLowerCase() === "a"
                          ) {
                            event.preventDefault();
                            setSelected(
                              new Set(filtered.map((row) => key(row.path))),
                            );
                            return;
                          }
                          if (
                            (event.ctrlKey || event.metaKey) &&
                            event.key.toLowerCase() === "c"
                          ) {
                            event.preventDefault();
                            const paths = (
                              selected.has(key(r.path))
                                ? rows.filter((row) =>
                                    selected.has(key(row.path)),
                                  )
                                : [r]
                            )
                              .filter(
                                (row) => !row.path.startsWith("exercise:"),
                              )
                              .map((row) => row.path);
                            void work(async () => {
                              await copyLibraryPaths(paths);
                              setFileActionResult(
                                `已复制 ${paths.length} 个文件位置`,
                              );
                            });
                            return;
                          }
                          const index = page * 50 + i,
                            movement: Record<string, number> = {
                              ArrowDown: index + 1,
                              ArrowUp: index - 1,
                              Home: event.ctrlKey ? 0 : page * 50,
                              End: event.ctrlKey
                                ? filtered.length - 1
                                : Math.min(filtered.length - 1, page * 50 + 49),
                              PageDown: index + 50,
                              PageUp: index - 50,
                            };
                          if (event.key in movement) {
                            event.preventDefault();
                            if (event.shiftKey && anchor.current === null)
                              anchor.current = index;
                            moveFocus(movement[event.key], event);
                            return;
                          }
                          if (event.key === "Enter") void open(r);
                          if (event.key === " ") {
                            event.preventDefault();
                            toggle(r);
                          }
                        }}
                      >
                        <td>
                          <input
                            type="checkbox"
                            aria-label={`选择 ${m?.title ?? r.title}`}
                            checked={selected.has(key(r.path))}
                            onClick={(event) => event.stopPropagation()}
                            onChange={() => toggle(r)}
                          />
                        </td>
                        <td title={r.path}>
                          <strong>{m?.title ?? r.title}</strong>
                          <small>
                            {(m?.tags ?? r.tags ?? []).join(" · ") ||
                              r.category ||
                              r.source}
                          </small>
                        </td>
                        <td>{m?.composer ?? r.composer}</td>
                        <td>{m?.difficulty ?? r.difficulty ?? "—"}</td>
                        <td>{e?.contentId ? clock(e.duration) : "—"}</td>
                        <td>
                          {e?.available === false
                            ? "文件缺失"
                            : e?.error
                              ? "需处理"
                              : e?.hasScore
                                ? "已配乐谱"
                                : e?.contentId
                                  ? "已索引"
                                  : "待索引"}
                          {!!e?.notationCount && (
                            <small>{e.notationCount} 份演奏乐谱</small>
                          )}
                          {!!e?.paperCount && (
                            <small title={e.paperNames?.join(" · ")}>
                              {e.paperCount} 份 PDF / 图片谱面
                            </small>
                          )}
                          {e?.contentId && duplicates.has(e.contentId) && (
                            <small>重复内容</small>
                          )}
                        </td>
                        <td>
                          {e?.contentId && data.learning?.[e.contentId] ? (
                            <>
                              <span>
                                {
                                  learningStages[
                                    data.learning[e.contentId].stage
                                  ]
                                }
                              </span>
                              <small
                                className={
                                  activeDeadline(data.learning[e.contentId]) &&
                                  data.learning[e.contentId].due! < today
                                    ? "learning-overdue"
                                    : ""
                                }
                              >
                                {data.learning[e.contentId].due ??
                                  "未设目标日期"}
                                {activeDeadline(data.learning[e.contentId]) &&
                                data.learning[e.contentId].due! < today
                                  ? " · 已逾期"
                                  : ""}
                              </small>
                            </>
                          ) : (
                            "未设置"
                          )}
                        </td>
                        <td>{e?.rating ? "★".repeat(e.rating) : "—"}</td>
                        <td>
                          {historyFor(r)?.sessions
                            ? `${historyFor(r)!.sessions} 次`
                            : "—"}
                          <small>
                            {historyFor(r)?.lastPracticed
                              ? new Date(
                                  historyFor(r)!.lastPracticed!,
                                ).toLocaleDateString("zh-CN")
                              : ""}
                            {historyFor(r)?.review?.due ? " · 待复习" : ""}
                          </small>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
              {!shown.length && (
                <p className="library-empty">没有符合条件的曲目</p>
              )}
            </div>
            <div className="library-manager-pagination">
              <span>
                {filtered.length.toLocaleString()} 首 · 已选择 {selected.size}{" "}
                首{" "}
                <button
                  onClick={() =>
                    setSelected(new Set(filtered.map((r) => key(r.path))))
                  }
                >
                  选择全部结果
                </button>
              </span>
              <button
                aria-label="上一页曲目"
                disabled={!page}
                onClick={() => setPage(page - 1)}
              >
                <ChevronLeft size={14} />
              </button>
              <span>
                {page + 1} / {Math.max(1, Math.ceil(filtered.length / 50))}
              </span>
              <button
                aria-label="下一页曲目"
                disabled={(page + 1) * 50 >= filtered.length}
                onClick={() => setPage(page + 1)}
              >
                <ChevronRight size={14} />
              </button>
            </div>
          </div>
          <aside className="library-file-details">
            {current ? (
              <>
                <h3>{entry?.metadata.title ?? current.title}</h3>
                <p className="file-path">{current.path}</p>
                <div className="file-actions">
                  <button
                    disabled={busy || !entry?.contentId}
                    onClick={() =>
                      entry?.contentId &&
                      setLearningEdit({
                        path: current.path,
                        contentId: entry.contentId,
                        title: entry.metadata.title ?? current.title,
                        previous: data.learning?.[entry.contentId] ?? null,
                      })
                    }
                  >
                    学习目标
                  </button>
                  <button disabled={busy} onClick={() => void open(current)}>
                    打开练习
                  </button>
                  <button
                    disabled={busy || current.path.startsWith("exercise:")}
                    onClick={() => void work(() => inspect([current], true))}
                  >
                    检查文件
                  </button>
                  <button
                    disabled={busy || current.path.startsWith("exercise:")}
                    onClick={() => editRow(current)}
                  >
                    编辑信息
                  </button>
                  <button
                    disabled={
                      busy ||
                      !api.desktop ||
                      current.path.startsWith("exercise:")
                    }
                    onClick={() =>
                      void work(async () => {
                        await api.revealFile(current.path);
                      })
                    }
                  >
                    打开文件位置
                  </button>
                  <button
                    disabled={busy || current.path.startsWith("exercise:")}
                    onClick={() =>
                      void work(async () => {
                        await copyLibraryPaths([current.path]);
                        setFileActionResult("已复制文件位置");
                      })
                    }
                  >
                    复制文件位置
                  </button>
                  {entry?.available === false && api.desktop && (
                    <button
                      disabled={busy || !entry.contentId}
                      onClick={() =>
                        void work(async () => {
                          const restored = await api.relocate(entry.contentId!);
                          if (restored) {
                            await api.command({
                              type: "inspectSong",
                              path: restored.sourcePath,
                              force: true,
                            });
                            setActive(key(restored.sourcePath));
                            await reload();
                            await changed();
                          }
                        })
                      }
                    >
                      重新定位 MIDI
                    </button>
                  )}
                </div>
                {entry?.contentId && current && <button disabled={busy || !!edit} onClick={() => setMetadataHistory({path:current.path,contentId:entry.contentId!,title:entry.metadata.title ?? current.title})}>资料变更历史</button>}
                {entry && (
                  <>
                    <dl>
                      {[
                        ["音符", entry.notes],
                        ["声部音轨", entry.tracks],
                        ["小节", entry.measures],
                        ["拍号", entry.meter || "—"],
                        ["文件大小", `${Math.round(entry.size / 1024)} KB`],
                      ].map(([label, value]) => (
                        <div key={label}>
                          <dt>{label}</dt>
                          <dd>{value}</dd>
                        </div>
                      ))}
                    </dl>
                    {entry.scoreName && (
                      <p className="file-path">乐谱：{entry.scoreName}</p>
                    )}
                    {entry.error && (
                      <p className="dialog-error">{entry.error}</p>
                    )}
                  </>
                )}
                {entry?.contentId && current && (
                  <LibraryScoresPane
                    key={`${current.path}:${entry.contentId}`}
                    row={current}
                    contentId={entry.contentId}
                    changed={async () => {
                      await reload();
                      await changed();
                    }}
                    onBusy={setBusy}
                    openVersion={openVersion}
                  />
                )}
                {entry?.contentId && data.learning?.[entry.contentId] && (
                  <section
                    className="library-learning-summary"
                    aria-label="当前曲目学习资料"
                  >
                    <h4>
                      {learningStages[data.learning[entry.contentId].stage]}
                    </h4>
                    <p>
                      {data.learning[entry.contentId].due
                        ? `目标日期：${data.learning[entry.contentId].due}`
                        : "未设置目标日期"}
                      {activeDeadline(data.learning[entry.contentId]) &&
                      data.learning[entry.contentId].due! < today
                        ? " · 已逾期"
                        : ""}
                    </p>
                    {data.learning[entry.contentId].weeklyMinutes && (
                      <p>
                        每周计划 {data.learning[entry.contentId].weeklyMinutes}{" "}
                        分钟
                      </p>
                    )}
                    {data.learning[entry.contentId].targetBpm && (
                      <p>
                        目标速度 {data.learning[entry.contentId].targetBpm} BPM
                      </p>
                    )}
                    {data.learning[entry.contentId].goal && (
                      <p className="learning-goal-text">
                        {data.learning[entry.contentId].goal}
                      </p>
                    )}
                  </section>
                )}
                {entry?.contentId && (
                  <PracticeTimePanel
                    contentId={entry.contentId}
                    targetMinutes={
                      data.learning?.[entry.contentId]?.weeklyMinutes ?? null
                    }
                  />
                )}
              </>
            ) : (
              <p>
                选择曲目查看文件、声部、拍号、乐谱与备注。Ctrl 多选，Shift
                连选，双击进入练习。
              </p>
            )}
            {edit && (
              <LibraryMetadataEditor
                key={`${edit.path}:${edit.contentId}`}
                edit={edit}
                onBusy={setBusy}
                cancel={() => {
                  setEdit(null);
                  setError("");
                }}
                saved={async (warnings) => {
                  await reload();
                  await changed();
                  if (warnings?.length) setError(warnings.join("；"));
                  setEdit(null);
                }}
              />
            )}
          </aside>
        </div>
        {learningResult && (
          <p className="library-learning-result" role="status">
            {learningResult}
          </p>
        )}
        <div className="library-manager-actions">
          <button disabled={busy || !!edit || !picked.length || picked.length > 100} onClick={() => setMetadataBatch(picked.map(row => ({path:row.path,contentId:data.entries[key(row.path)]?.contentId ?? null,title:row.title}))) }>批量修改资料 · {picked.length}</button>
          <select
            aria-label="批量学习阶段"
            value={batchStage}
            disabled={busy}
            onChange={(event) => setBatchStage(event.target.value)}
          >
            {Object.entries(learningStages).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
          <button
            disabled={
              busy || !learningPicked.length || learningPicked.length > 100
            }
            title="只处理已索引曲目，相同内容的副本合并为一首；每次最多 100 首。保留原目标与要求。"
            onClick={() =>
              void work(async () => {
                setLearningResult("");
                let done = 0;
                const errors: string[] = [];
                for (const { row, cid } of learningPicked) {
                  const previous = data.learning?.[cid] ?? null;
                  try {
                    await api.command({
                      type: "saveSongLearning",
                      path: row.path,
                      content_id: cid,
                      expected: previous,
                      value: {
                        stage: batchStage,
                        goal: previous?.goal ?? "",
                        due: previous?.due ?? null,
                        weeklyMinutes: previous?.weeklyMinutes ?? null,
                        targetBpm: previous?.targetBpm ?? null,
                      },
                    });
                    done++;
                  } catch (error) {
                    errors.push(`${row.title}：${String(error)}`);
                  }
                }
                await reload();
                await changed();
                setLearningResult(
                  `已设置 ${done} 首为${learningStages[batchStage]}${errors.length ? `；${errors.length} 首未完成：${errors.join("；")}` : "。原目标与要求已保留。"}`,
                );
              })
            }
          >
            设定学习阶段 · {learningPicked.length}
          </button>
          <button
            disabled={busy || !picked.length}
            onClick={() => void work(() => inspect(picked))}
          >
            索引所选 {picked.length || ""}
          </button>
          <button
            disabled={busy || !shown.length}
            onClick={() => void work(() => inspect(shown))}
          >
            索引本页
          </button>
          <button
            disabled={
              busy ||
              !rows.length ||
              background?.status === "running" ||
              background?.status === "paused"
            }
            onClick={() => void work(() => inspect(rows))}
          >
            后台索引全库
          </button>
          <select
            aria-label="批量目标分组"
            value={target}
            onChange={(e) => setTarget(e.target.value)}
          >
            <option value="">选择分组</option>
            {groups.map((g) => (
              <option key={g.id} value={g.id}>
                {groupName(g.id)}
              </option>
            ))}
          </select>
          <button
            disabled={busy || !target || !picked.length}
            onClick={() =>
              void work(() =>
                mutate({
                  type: "assignLibrary",
                  paths: picked.map((r) => r.path),
                  group: target,
                  rating: null,
                }),
              )
            }
          >
            加入分组
          </button>
          {group && (
            <button
              disabled={busy || !picked.length}
              onClick={() =>
                void work(() =>
                  mutate({
                    type: "unassignLibrary",
                    paths: picked.map((r) => r.path),
                    group,
                  }),
                )
              }
            >
              移出此分组
            </button>
          )}
          <select
            aria-label="批量曲目评级"
            value={rating}
            onChange={(e) => setRating(e.target.value)}
          >
            {[0, 1, 2, 3, 4, 5].map((r) => (
              <option key={r} value={r}>
                {r ? "★".repeat(r) : "未评级"}
              </option>
            ))}
          </select>
          <button
            disabled={busy || !picked.length}
            onClick={() =>
              void work(() =>
                mutate({
                  type: "assignLibrary",
                  paths: picked.map((r) => r.path),
                  group: null,
                  rating: Number(rating),
                }),
              )
            }
          >
            设定评级
          </button>
          {background && background.total > 0 && (
            <div className="index-status" aria-live="polite">
              <span>
                {(
                  {
                    running: "正在索引",
                    paused: "已暂停",
                    completed: "索引完成",
                    cancelled: "已取消",
                    failed: "索引失败",
                  } as Record<string, string>
                )[background.status] ?? background.status}{" "}
                {background.done.toLocaleString()} /{" "}
                {background.total.toLocaleString()}
                {background.errors > 0
                  ? ` · ${background.errors} 个文件需处理`
                  : ""}
              </span>
              {background.status === "running" && (
                <button
                  onClick={() =>
                    void work(async () => {
                      setBackground(
                        await api.command<IndexProgress>({
                          type: "pauseIndex",
                        }),
                      );
                    })
                  }
                >
                  暂停后台索引
                </button>
              )}
              {background.status === "paused" && (
                <button
                  onClick={() =>
                    void work(async () => {
                      setBackground(
                        await api.command<IndexProgress>({
                          type: "resumeIndex",
                        }),
                      );
                    })
                  }
                >
                  继续后台索引
                </button>
              )}
              {["running", "paused"].includes(background.status) && (
                <button
                  onClick={() =>
                    void work(async () => {
                      setBackground(
                        await api.command<IndexProgress>({
                          type: "cancelIndex",
                        }),
                      );
                    })
                  }
                >
                  取消索引任务
                </button>
              )}
              {background.status === "running" && (
                <span className="parameter-help" title={background.current}>
                  {background.current}
                </span>
              )}
              {background.message && (
                <span className="dialog-error">{background.message}</span>
              )}
            </div>
          )}
          <span className="menu-spacer" />
          <button
            disabled={!selected.size}
            onClick={() => {
              setSelected(new Set());
              anchor.current = null;
              rangeBase.current = new Set();
            }}
          >
            取消选择
          </button>
        </div>
        {collectionWorking && (
          <button
            onClick={() => {
              collectionStop.current = true;
            }}
          >
            停止收藏 / 队列处理
          </button>
        )}
        {fileActionResult && (
          <p role="status" className="library-learning-result">
            {fileActionResult}
          </p>
        )}
        {menu && (
          <LibraryContextMenu
            x={menu.x}
            y={menu.y}
            title={
              menu.items.length > 1
                ? `已选择 ${menu.items.length} 首 · ${menu.row.title}`
                : menu.row.title
            }
            actions={menuActions}
            close={closeMenu}
          />
        )}
        {learningEdit && (
          <SongLearningDialog
            {...learningEdit}
            close={() => setLearningEdit(null)}
            changed={async () => {
              await reload();
              await changed();
            }}
          />
        )}
        {metadataHistory && <LibraryMetadataHistoryDialog {...metadataHistory} close={() => setMetadataHistory(null)} changed={async () => {await reload();await changed();}} />}
        {metadataBatch && <LibraryMetadataBatchDialog items={metadataBatch} close={() => setMetadataBatch(null)} changed={async () => {await reload();await changed();}} />}
        {locationsOpen && <FileLocationsDialog close={() => setLocationsOpen(false)} />}
        {paperSourcesOpen && <PaperSourceUpdatesDialog close={()=>setPaperSourcesOpen(false)} changed={changed}/>}
        {sourcesOpen && (
          <ScoreSourcesDialog
            close={() => setSourcesOpen(false)}
            changed={changed}
          />
        )}
      </section>
    </div>
  );
}
