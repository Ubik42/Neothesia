import { encodeBatch, decodeBatch, downloadBatch } from "./batchArchive";
import { useEffect, useMemo, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { api, type LoadedSong } from "./api";
import {
  fileStem,
  imageBook,
  importKind,
  pairingCandidates,
  type ImportKind,
  type PairTarget,
} from "./scorePairing";
import { ScoreFilePreview } from "./ScoreFilePreview";
import "./batchScoreImport.css";
import {
  listBatches,
  saveBatch,
  loadBatch,
  removeBatch,
  type BatchItem,
  type BatchSummary,
  type BatchData,
} from "./batchImportStore";

type Item = BatchItem;

const labels = {
  midi: "MIDI 演奏",
  notation: "演奏乐谱",
  pdf: "PDF 纸谱",
  image: "图片谱页",
};
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
const itemsOf = (files: File[]): Item[] =>
  files
    .filter((f) => importKind(f.name))
    .slice(0, 200)
    .map((file) => ({
      id: crypto.randomUUID(),
      file,
      kind: importKind(file.name)!,
      selected: false,
      manual: false,
      group: imageBook(file.name),
      state: "pending",
      message: "等待读取候选",
    }));
const pathKey = (path: string) => path.replaceAll("\\", "/").toLowerCase();
function mergeTargets(rows: PairTarget[]) {
  const values = new Map<string, PairTarget>();
  for (const t of rows) {
    const key = pathKey(t.path),
      previous = values.get(key);
    values.set(
      key,
      previous
        ? {
            ...previous,
            ...t,
            aliases: [...new Set([...previous.aliases, ...t.aliases])],
          }
        : t,
    );
  }
  return [...values.values()];
}
async function actualTarget(target: PairTarget): Promise<PairTarget> {
  const value = await api.command<{
    available: boolean;
    contentId?: string;
    error?: string;
  }>({ type: "inspectSong", path: target.path, force: true });
  if (!value.available || !value.contentId)
    throw new Error(value.error || "演奏文件当前不可用，请重新选择曲目");
  if (target.contentId && value.contentId !== target.contentId)
    throw new Error("演奏文件内容已改变，请重新读取并核对配对");
  return { ...target, contentId: value.contentId };
}
export function BatchScoreImportDialog({
  initialFiles,
  close,
  back,
  changed,
}: {
  initialFiles: File[];
  close: () => void;
  back: () => void;
  changed: () => Promise<void>;
}) {
  const [items, setItemsReact] = useState<Item[]>(() => itemsOf(initialFiles));
  const itemsRef = useRef(items);
  const setItems = (value: Item[] | ((old: Item[]) => Item[])) => {
    const next = typeof value === "function" ? value(itemsRef.current) : value;
    itemsRef.current = next;
    setItemsReact(next);
  };
  const [targets, setTargets] = useState<PairTarget[]>([]),
    [prepared, setPreparedReact] = useState(false),
    [busy, setBusy] = useState(false);
  const [error, setError] = useState(
      initialFiles.length > 200
        ? "一次最多处理 200 个文件，超出部分请另建批次"
        : initialFiles.some((f) => !importKind(f.name))
          ? "部分文件格式不支持，未加入列表"
          : "",
    ),
    [page, setPage] = useState(0),
    [preview, setPreview] = useState<Item | null>(null);
  const [picker, setPicker] = useState<string | null>(null),
    [query, setQuery] = useState(""),
    [targetPage, setTargetPage] = useState(0),
    [picking, setPicking] = useState(false);
  const input = useRef<HTMLInputElement>(null),
    root = useRef<HTMLElement>(null),
    stop = useRef(false);
  const books = useRef(new Map<string, string>()),
    midiTargets = useRef<PairTarget[]>([]);
  const preparedRef = useRef(false);
  const setPrepared = (value: boolean) => {
    preparedRef.current = value;
    setPreparedReact(value);
  };
  const [batchName, setBatchName] = useState(
    "曲谱整理 " + new Date().toLocaleString("zh-CN"),
  );
  const nameRef = useRef(batchName);
  nameRef.current = batchName;
  const activeBatch = useRef<{ id: string; revision: number } | null>(null),
    saveChain = useRef<Promise<void>>(Promise.resolve());
  const [savedAt, setSavedAt] = useState(0),
    [storageError, setStorageError] = useState("");
  const [batchList, setBatchList] = useState<BatchSummary[]>([]),
    [batchesOpen, setBatchesOpen] = useState(false),
    [removing, setRemoving] = useState<BatchSummary | null>(null);
  const [receipt, setReceipt] = useState<{
    row: Item;
    matches: { id: string; name: string; kind: string; page: number }[];
  } | null>(null);
  const archiveInput = useRef<HTMLInputElement>(null),
    archiveCancel = useRef(false);
  const [archiveExporting, setArchiveExporting] = useState(false);
  const [archiveReview, setArchiveReview] = useState<BatchData | null>(null),
    [archiveProgress, setArchiveProgress] = useState(""),
    [archiveNotice, setArchiveNotice] = useState("");
  useEffect(() => {
    if (archiveReview)
      root.current
        ?.querySelector<HTMLInputElement>(".batch-archive-review input")
        ?.focus();
  }, [!!archiveReview]);
  const exportCurrent = async () => {
    archiveCancel.current = false;
    setArchiveExporting(true);
    setBusy(true);
    setError("");
    setArchiveNotice("");
    try {
      await checkpoint();
      const blob = await encodeBatch(
        {
          id: activeBatch.current?.id ?? "unsaved",
          revision: activeBatch.current?.revision ?? 0,
          name: nameRef.current,
          updated: Date.now(),
          items: itemsRef.current,
          prepared: preparedRef.current,
          books: [...books.current],
          midiTargets: midiTargets.current,
        },
        setArchiveProgress,
        () => archiveCancel.current,
      );
      const path = await downloadBatch(
        blob,
        nameRef.current,
        setArchiveProgress,
        () => archiveCancel.current,
      );
      if (path) setArchiveNotice("已导出曲谱整理批次：" + path);
    } catch (e) {
      setError(message(e));
    } finally {
      setArchiveExporting(false);
      setBusy(false);
      setArchiveProgress("");
    }
  };
  const readArchive = async (file: File) => {
    setBusy(true);
    setError("");
    setArchiveNotice("");
    try {
      const batch = await decodeBatch(file, setArchiveProgress);
      const workspace = await api.command<{
        entries: Record<string, { path: string; contentId?: string }>;
      }>({ type: "libraryWorkspace" });
      const resolved = new Map<string, PairTarget | undefined>();
      for (const row of batch.items) {
        const hint = row.target;
        if (!hint || row.kind === "midi") continue;
        const cid = hint.contentId;
        if (cid && !resolved.has(cid)) {
          const paths = [
            hint.path,
            ...Object.values(workspace.entries)
              .filter((e) => e.contentId === cid)
              .map((e) => e.path),
          ];
          let found: PairTarget | undefined;
          for (const path of [...new Set(paths)]) {
            try {
              found = await actualTarget({ ...hint, path });
              break;
            } catch {
              /* A path hint is never accepted without actual content verification. */
            }
          }
          resolved.set(cid, found);
        }
        const found = cid ? resolved.get(cid) : undefined;
        if (found) {
          row.target = found;
          row.selected = row.state !== "review" && !!row.archiveSelected;
          row.message =
            row.state === "review"
              ? "演奏已核对，请核对本机已保存谱面"
              : "已按内容核对演奏，请确认配对和勾选";
        } else
          row.message = "原演奏在本机未核对，请读取批内 MIDI 或重新选择曲目";
      }
      setArchiveReview(batch);
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
      setArchiveProgress("");
    }
  };
  const acceptArchive = async () => {
    if (!archiveReview) return;
    setBusy(true);
    setError("");
    try {
      await checkpoint();
      const batch = { ...archiveReview, name: archiveReview.name.trim() };
      const revision = await saveBatch(batch);
      activeBatch.current = { id: batch.id, revision };
      books.current = new Map();
      midiTargets.current = [];
      setItems(batch.items);
      setPrepared(false);
      setBatchName(batch.name);
      nameRef.current = batch.name;
      setSavedAt(Date.now());
      setStorageError("");
      setPage(0);
      setBatchesOpen(false);
      setArchiveReview(null);
      const library = await api.library();
      setTargets(
        library.songs.map((row) => ({
          path: row.path,
          title: row.title,
          composer: row.composer,
          aliases: [row.title, fileStem(row.path)],
        })),
      );
      await refreshBatches();
      setArchiveNotice(
        "已另存为本机新批次。请读取候选，再逐项核对待确认的谱面；原批次保留。",
      );
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  };
  const refreshBatches = async () => setBatchList(await listBatches());
  const checkpoint = async () => {
    const active = activeBatch.current;
    if (!active) return;
    const value = {
      id: active.id,
      name: nameRef.current,
      items: itemsRef.current,
      prepared: preparedRef.current,
      books: [...books.current.entries()],
      midiTargets: midiTargets.current,
      updated: Date.now(),
      revision: 0,
    };
    const work = saveChain.current
      .catch(() => {})
      .then(async () => {
        if (activeBatch.current?.id !== value.id)
          throw new Error("批次已切换，请重新打开");
        const revision = await saveBatch({
          ...value,
          revision: activeBatch.current.revision,
        });
        activeBatch.current = { id: value.id, revision };
        setSavedAt(Date.now());
        setStorageError("");
      });
    saveChain.current = work;
    try {
      await work;
    } catch (e) {
      stop.current = true;
      setStorageError(message(e));
      throw e;
    }
  };
  useEffect(() => {
    void refreshBatches().catch((e) => setStorageError(message(e)));
  }, []);
  useEffect(() => {
    if (
      busy ||
      batchesOpen ||
      archiveReview ||
      receipt ||
      !activeBatch.current ||
      storageError
    )
      return;
    const timer = window.setTimeout(() => {
      void checkpoint().catch(() => {});
    }, 400);
    return () => window.clearTimeout(timer);
  }, [
    items,
    prepared,
    batchName,
    busy,
    storageError,
    batchesOpen,
    archiveReview,
    receipt,
  ]);
  const saveCurrent = async () => {
    setBusy(true);
    setError("");
    const creating = !activeBatch.current;
    if (creating)
      activeBatch.current = { id: crypto.randomUUID(), revision: 0 };
    try {
      await checkpoint();
      await refreshBatches();
    } catch (e) {
      if (creating && activeBatch.current?.revision === 0)
        activeBatch.current = null;
      setError(message(e));
    } finally {
      setBusy(false);
    }
  };
  const leave = async (action: () => void) => {
    setBusy(true);
    try {
      await checkpoint();
      action();
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  };
  const restore = async (batch: BatchSummary) => {
    setBusy(true);
    setError("");
    try {
      await checkpoint();
      const value = await loadBatch(batch.id);
      const library = await api.library();
      activeBatch.current = { id: value.id, revision: value.revision };
      books.current = new Map(value.books);
      midiTargets.current = value.midiTargets;
      setItems(value.items);
      setPrepared(value.prepared);
      setBatchName(value.name);
      nameRef.current = value.name;
      setSavedAt(value.updated);
      setStorageError("");
      setPage(0);
      setBatchesOpen(false);
      setRemoving(null);
      setTargets(
        mergeTargets([
          ...library.songs.map((row) => ({
            path: row.path,
            title: row.title,
            composer: row.composer,
            aliases: [row.title, fileStem(row.path)],
          })),
          ...value.midiTargets,
        ]),
      );
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  };
  const checkInterrupted = async (row: Item) => {
    setBusy(true);
    setError("");
    try {
      if (!row.target) throw new Error("请先重新选择演奏曲目");
      const target = await actualTarget(row.target);
      const result = await api.checkLibraryScoreImport(
        target.path,
        target.contentId!,
        row.file,
        row.kind,
        row.attemptTarget,
      );
      setReceipt({ row: { ...row, target }, matches: result.matches });
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  };
  const resolveReceipt = async (match: { id: string; kind: string } | null) => {
    if (!receipt) return;
    setBusy(true);
    setError("");
    try {
      const row = receipt.row;
      const actual = await actualTarget(row.target!);
      const checked = await api.checkLibraryScoreImport(
        actual.path,
        actual.contentId!,
        row.file,
        row.kind,
        row.attemptTarget,
      );
      if (
        match
          ? !checked.matches.some((m) => m.id === match.id)
          : checked.matches.length > 0
      )
        throw new Error("保存结果已变化，请重新核对中断项");
      if (match && row.kind === "image")
        books.current.set(
          JSON.stringify([row.target!.contentId, row.group.trim()]),
          match.id,
        );
      update(row.id, {
        target: row.target,
        state: match ? "done" : "pending",
        selected: !match,
        message: match
          ? "已核对实际文件，采用已有保存结果"
          : "未找到已保存文件，可继续保存",
      });
      await checkpoint();
      setReceipt(null);
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  };
  const state = useRef({
    busy,
    picking,
    preview,
    picker,
    batchesOpen,
    archiveReview,
    receipt,
    close,
  });
  state.current = {
    busy,
    picking,
    preview,
    picker,
    batchesOpen,
    archiveReview,
    receipt,
    close: () => void leave(close),
  };
  useEffect(() => {
    const prior = document.activeElement;
    const key = (e: KeyboardEvent) => {
      const s = state.current;
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (s.busy || s.picking) return;
        if (s.preview) setPreview(null);
        else if (s.picker) setPicker(null);
        else if (s.archiveReview) setArchiveReview(null);
        else if (s.receipt) setReceipt(null);
        else if (s.batchesOpen) setBatchesOpen(false);
        else s.close();
      }
      if (e.key === "Tab") {
        const scope =
          root.current?.querySelector<HTMLElement>(
            ".batch-file-preview,.batch-target-picker,.batch-saved-list,.batch-receipt-review,.batch-archive-review",
          ) ?? root.current;
        const nodes = Array.from(
          scope?.querySelectorAll<HTMLElement>(
            "button:not(:disabled),input:not(:disabled),select:not(:disabled)",
          ) ?? [],
        ).filter((el) => el.getClientRects().length);
        if (!nodes.length) return;
        if (
          e.shiftKey &&
          (document.activeElement === nodes[0] ||
            !scope?.contains(document.activeElement))
        ) {
          e.preventDefault();
          nodes.at(-1)?.focus();
        } else if (
          !e.shiftKey &&
          (document.activeElement === nodes.at(-1) ||
            !scope?.contains(document.activeElement))
        ) {
          e.preventDefault();
          nodes[0].focus();
        }
      }
    };
    document.addEventListener("keydown", key, true);
    return () => {
      document.removeEventListener("keydown", key, true);
      if (prior instanceof HTMLElement && prior.isConnected) prior.focus();
    };
  }, []);
  const update = (id: string, values: Partial<Item>) =>
    setItems((old) =>
      old.map((row) => (row.id === id ? { ...row, ...values } : row)),
    );
  const add = (files: File[]) => {
    if (busy) return;
    const supported = files.filter((f) => importKind(f.name));
    const room = 200 - items.length;
    setItems((old) => [...old, ...itemsOf(supported.slice(0, room))]);
    setPrepared(false);
    setError(
      supported.length > room
        ? "一次最多处理 200 个文件，请分批添加"
        : supported.length !== files.length
          ? "支持 MIDI、MusicXML/MXL、PDF、PNG、JPEG 和 WebP"
          : "",
    );
  };
  const prepare = async () => {
    setBusy(true);
    setError("");
    stop.current = false;
    let wrote = false;
    try {
      const library = await api.library();
      let candidates = mergeTargets([
        ...library.songs.map((row) => ({
          path: row.path,
          title: row.title,
          composer: row.composer,
          aliases: [row.title, fileStem(row.path)],
        })),
        ...midiTargets.current,
      ]);
      for (const row of items.filter(
        (r) => r.kind === "midi" && r.state !== "done",
      )) {
        if (stop.current) break;
        update(row.id, { state: "working", message: "正在导入 MIDI" });
        try {
          await checkpoint();
          if (row.file.size > (isTauri() ? 32_000_000 : 3_000_000))
            throw new Error(
              isTauri()
                ? "MIDI 超过 32 MB"
                : "Web 模式 MIDI 上限为 3 MB，请使用桌面版导入",
            );
          const song = await api.command<LoadedSong>({
            type: "importMidi",
            name: row.file.name,
            bytes: Array.from(new Uint8Array(await row.file.arrayBuffer())),
          });
          wrote = true;
          const target = {
            path: song.sourcePath,
            contentId: song.contentId,
            title: fileStem(row.file.name),
            composer: "",
            aliases: [fileStem(row.file.name), song.title],
          };
          midiTargets.current = mergeTargets([...midiTargets.current, target]);
          candidates = mergeTargets([...candidates, target]);
          update(row.id, { state: "done", target, message: "MIDI 已导入曲库" });
          await checkpoint();
        } catch (e) {
          if (itemsRef.current.find((r) => r.id === row.id)?.state === "done") {
            setError("文件已保存，但批次进度更新失败：" + message(e));
            stop.current = true;
          } else update(row.id, { state: "error", message: message(e) });
        }
      }
      for (const row of itemsRef.current.filter(
        (r) => r.kind !== "midi" && r.manual && r.target?.contentId,
      )) {
        const same = candidates.find(
          (t) => t.contentId === row.target!.contentId,
        );
        if (same && same.path !== row.target!.path) {
          try {
            update(row.id, {
              target: await actualTarget(same),
              selected: row.state !== "review" && !!row.archiveSelected,
              message:
                row.state === "review"
                  ? "演奏已核对，请核对本机已保存谱面"
                  : "已按演奏内容恢复配对，请核对并勾选",
            });
          } catch (e) {
            update(row.id, { selected: false, message: message(e) });
          }
        }
      }
      setTargets(candidates);
      const inspected = new Map<string, PairTarget>();
      for (const row of items.filter(
        (r) =>
          r.kind !== "midi" &&
          r.state !== "done" &&
          r.state !== "review" &&
          !r.manual,
      )) {
        if (stop.current) break;
        const matched = pairingCandidates(row.file.name, candidates);
        if (matched.length !== 1) {
          update(row.id, {
            target: undefined,
            selected: false,
            state: "pending",
            message: matched.length
              ? "同名演奏有多份，请选择"
              : "未找到同名曲目，请选择",
          });
          continue;
        }
        try {
          let target = inspected.get(pathKey(matched[0].path));
          if (!target) {
            target = await actualTarget(matched[0]);
            inspected.set(pathKey(target.path), target);
          }
          update(row.id, {
            target,
            selected: true,
            state: "pending",
            message: "同名候选，请核对谱面版本",
          });
        } catch (e) {
          update(row.id, {
            target: undefined,
            selected: false,
            state: "error",
            message: message(e),
          });
        }
      }
      setPrepared(true);
      if (stop.current)
        setError("已暂停；已导入的 MIDI 保留，继续读取可处理剩余项");
    } catch (e) {
      setError(message(e));
    } finally {
      try {
        await checkpoint();
      } catch (e) {
        setError("批次进度未能保存：" + message(e));
      }
      if (wrote)
        try {
          await changed();
        } catch (e) {
          setError(`MIDI 已保存，刷新曲库失败：${message(e)}`);
        }
      setBusy(false);
    }
  };
  const choose = async (target: PairTarget) => {
    setPicking(true);
    setError("");
    try {
      const actual = await actualTarget(target);
      update(picker!, {
        target: actual,
        selected: true,
        manual: true,
        state:
          itemsRef.current.find((r) => r.id === picker)?.state === "review"
            ? "review"
            : "pending",
        message: "已人工指定曲目，请核对谱面",
      });
      setPicker(null);
    } catch (e) {
      setError(message(e));
    } finally {
      setPicking(false);
    }
  };
  const save = async () => {
    setBusy(true);
    setError("");
    stop.current = false;
    let wrote = false;
    try {
      for (const row of items.filter(
        (r) =>
          r.kind !== "midi" &&
          r.selected &&
          r.state !== "done" &&
          r.state !== "review",
      )) {
        if (stop.current) break;
        update(row.id, { state: "working", message: "正在保存配对" });
        try {
          if (!row.target) throw new Error("请先指定曲目");
          if (
            row.kind === "image" &&
            (!row.group.trim() || row.group.trim().length > 120)
          )
            throw new Error("图片纸谱名称需为 1 至 120 字");
          const target = await actualTarget(row.target);
          const key = JSON.stringify([target.contentId, row.group.trim()]);
          if (
            row.kind === "image" &&
            items
              .slice(items.indexOf(row) + 1)
              .some(
                (other) =>
                  other.kind === "image" &&
                  other.state === "done" &&
                  other.target?.contentId === target.contentId &&
                  other.group.trim() === row.group.trim(),
              )
          )
            throw new Error(
              "后面的谱页已保存。请将本页下移至已保存页之后，或使用另一本纸谱名称",
            );
          const append =
            row.kind === "image" ? books.current.get(key) : undefined;
          update(row.id, { attemptTarget: append });
          await checkpoint();
          const result = await api.addLibraryScore(
            target.path,
            target.contentId!,
            row.file,
            row.kind === "notation" ? "notation" : append ? "append" : "paper",
            append,
          );
          wrote = true;
          // The file is already persisted. A naming failure must not cause a retry
          // to insert the same page a second time.
          let warning = "";
          if (row.kind === "image" && !append && result.active) {
            books.current.set(key, result.active);
            try {
              await api.command({
                type: "manageLibraryScore",
                path: target.path,
                content_id: target.contentId,
                kind: "paper",
                id: result.active,
                action: "rename",
                name: row.group.trim(),
              });
            } catch (e) {
              warning = `；纸谱命名失败：${message(e)}`;
            }
          }
          update(row.id, {
            target,
            state: "done",
            message: `${append ? "已追加到图片纸谱" : "配对已保存"}${warning}`,
          });
          await checkpoint();
        } catch (e) {
          if (itemsRef.current.find((r) => r.id === row.id)?.state === "done") {
            setError("文件已保存，但批次进度更新失败：" + message(e));
            stop.current = true;
          } else update(row.id, { state: "error", message: message(e) });
        }
      }
      if (stop.current)
        setError("已暂停；已保存结果保留，可以继续保存剩余选中项");
    } finally {
      try {
        await checkpoint();
      } catch (e) {
        setError("批次进度未能保存：" + message(e));
      }
      if (wrote)
        try {
          await changed();
        } catch (e) {
          setError(`配对已保存，刷新曲库失败：${message(e)}`);
        }
      setBusy(false);
    }
  };
  const move = (id: string, offset: number) =>
    setItems((old) => {
      const index = old.findIndex((r) => r.id === id),
        next = index + offset;
      if (
        next < 0 ||
        next >= old.length ||
        old[index].state === "done" ||
        old[index].state === "review"
      )
        return old;
      const rows = [...old];
      [rows[index], rows[next]] = [rows[next], rows[index]];
      return rows;
    });
  const targetRows = useMemo(
    () =>
      targets.filter((t) =>
        `${t.title} ${t.composer} ${t.path}`
          .toLowerCase()
          .includes(query.toLowerCase()),
      ),
    [targets, query],
  );
  const visible = items.slice(page * 25, page * 25 + 25),
    count = items.filter(
      (r) =>
        r.selected &&
        r.kind !== "midi" &&
        r.state !== "done" &&
        r.state !== "review",
    ).length;
  const pendingMidi = items.some(
    (r) => r.kind === "midi" && r.state !== "done",
  );
  return (
    <div
      className="modal-backdrop"
      onClick={() => !busy && !picking && void leave(close)}
    >
      <section
        ref={root}
        className="settings-dialog batch-score-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="batch-score-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="batch-score-title">批量配对曲目与谱面</h2>
            <p>为多首演奏逐项整理 MusicXML、PDF 和图片纸谱</p>
          </div>
          <button
            autoFocus
            disabled={busy || picking}
            onClick={() => void leave(close)}
            aria-label="关闭批量配对"
          >
            关闭
          </button>
        </div>
        {!preview && !picker && !receipt && !batchesOpen && !archiveReview && (
          <div className="batch-checkpoint-bar">
            <label>
              批次名称
              <input
                aria-label="批次名称"
                maxLength={120}
                value={batchName}
                disabled={busy}
                onChange={(e) => setBatchName(e.target.value)}
              />
            </label>
            <button
              disabled={busy || !items.length}
              onClick={() => void saveCurrent()}
            >
              {activeBatch.current ? "保存批次进度" : "保存此批次"}
            </button>
            <button
              disabled={busy}
              onClick={() => {
                setBusy(true);
                void checkpoint()
                  .then(refreshBatches)
                  .then(() => {
                    setBatchesOpen(true);
                    setRemoving(null);
                  })
                  .catch((e) => setError(message(e)))
                  .finally(() => setBusy(false));
              }}
            >
              已保存批次（{batchList.length}）
            </button>
            <button
              disabled={busy || !items.length}
              onClick={() => void exportCurrent()}
            >
              导出整理批次
            </button>
            <button
              disabled={busy}
              onClick={() => archiveInput.current?.click()}
            >
              导入整理批次
            </button>
            <small>
              {savedAt
                ? "文件副本与进度已保存在本机 · " +
                  new Date(savedAt).toLocaleTimeString("zh-CN")
                : "保存批次后，文件副本与配对进度可在关闭或重开软件后继续"}
            </small>
          </div>
        )}
        <input
          ref={archiveInput}
          type="file"
          hidden
          accept=".neoscorebatch"
          aria-label="选择曲谱整理批次文件"
          onChange={(e) => {
            const f = e.target.files?.[0];
            e.target.value = "";
            if (f) void readArchive(f);
          }}
        />
        {archiveProgress && <p role="status">{archiveProgress}</p>}
        {archiveExporting && (
          <button
            onClick={() => {
              archiveCancel.current = true;
            }}
          >
            取消批次导出
          </button>
        )}
        {archiveNotice && <p role="status">{archiveNotice}</p>}
        {preview ? (
          <ScoreFilePreview
            file={preview.file}
            close={() => setPreview(null)}
          />
        ) : archiveReview ? (
          <section
            className="batch-archive-review"
            aria-label="整理批次导入预览"
          >
            <h3>导入整理批次</h3>
            <label>
              本机新批次名称
              <input
                aria-label="导入批次名称"
                value={archiveReview.name}
                maxLength={120}
                disabled={busy}
                onChange={(e) =>
                  setArchiveReview({ ...archiveReview, name: e.target.value })
                }
              />
            </label>
            <p>
              {archiveReview.items.length} 个文件 ·{" "}
              {(
                archiveReview.items.reduce((n, i) => n + i.file.size, 0) /
                1048576
              ).toFixed(1)}{" "}
              MB ·{" "}
              {archiveReview.items.filter((i) => i.state === "review").length}{" "}
              项需要核对已保存谱面。
            </p>
            <p>
              文件副本已逐段校验。将另存为新批次，保留原批次；谱页分组与顺序恢复，来源电脑的完成结果需要在本机重新核对。此文件携带整理材料，个人指法、批注和练习记录请使用曲目包或练习备份。
            </p>
            <div className="batch-archive-files">
              <table>
                <thead>
                  <tr>
                    <th>文件</th>
                    <th>谱页分组</th>
                    <th>演奏归属 / 本机核对</th>
                    <th>预览</th>
                  </tr>
                </thead>
                <tbody>
                  {archiveReview.items.map((row) => (
                    <tr key={row.id}>
                      <td>
                        {row.file.name}
                        <small>
                          {labels[row.kind]} · 来源
                          {row.archiveSelected ? "已勾选" : "未勾选"}
                        </small>
                      </td>
                      <td>{row.group}</td>
                      <td>
                        {row.target?.title ?? "尚未配对"}
                        <small>{row.message}</small>
                      </td>
                      <td>
                        <button
                          disabled={busy || row.kind === "midi"}
                          onClick={() => setPreview(row)}
                        >
                          预览文件
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="batch-import-actions">
              <button
                disabled={busy || !archiveReview.name.trim()}
                onClick={() => void acceptArchive()}
              >
                另存为本机新批次
              </button>
              <button disabled={busy} onClick={() => setArchiveReview(null)}>
                取消批次导入
              </button>
            </div>
          </section>
        ) : batchesOpen ? (
          <section className="batch-saved-list" aria-label="已保存的配对批次">
            <div className="batch-preview-heading">
              <h3>已保存批次</h3>
              <button
                autoFocus
                disabled={busy}
                onClick={() => {
                  setBatchesOpen(false);
                  setRemoving(null);
                }}
              >
                返回当前文件列表
              </button>
            </div>
            <p>
              打开批次会切换当前列表。批次保存文件副本；移除批次保留已经导入的曲目和谱面。
            </p>
            {!batchList.length && <p>还没有保存的批次。</p>}
            {batchList.map((batch) => (
              <div className="batch-saved-row" key={batch.id}>
                <div>
                  <strong>{batch.name}</strong>
                  <small>
                    {new Date(batch.updated).toLocaleString("zh-CN")} · 已完成{" "}
                    {batch.done} / {batch.total} ·{" "}
                    {(batch.bytes / 1048576).toFixed(1)} MB
                    {batch.interrupted
                      ? ` · ${batch.interrupted} 项待核对`
                      : ""}
                  </small>
                </div>
                <button disabled={busy} onClick={() => void restore(batch)}>
                  继续此批次
                </button>
                <button disabled={busy} onClick={() => setRemoving(batch)}>
                  移除批次
                </button>
              </div>
            ))}
            {removing && (
              <div className="batch-remove-review">
                <p>
                  移除“{removing.name}”的 {removing.total}{" "}
                  个文件副本和整理进度？已导入的曲目、谱面及原文件保留。
                </p>
                <button
                  disabled={busy}
                  onClick={() => {
                    setBusy(true);
                    void removeBatch(removing.id, removing.revision)
                      .then(() => {
                        if (activeBatch.current?.id === removing.id) {
                          activeBatch.current = null;
                          setSavedAt(0);
                        }
                        setRemoving(null);
                        return refreshBatches();
                      })
                      .catch((e) => setError(message(e)))
                      .finally(() => setBusy(false));
                  }}
                >
                  确认移除批次副本
                </button>
                <button disabled={busy} onClick={() => setRemoving(null)}>
                  取消移除
                </button>
              </div>
            )}
          </section>
        ) : receipt ? (
          <section
            className="batch-receipt-review"
            aria-label="核对中断保存结果"
          >
            <div className="batch-preview-heading">
              <h3>核对中断项</h3>
              <button
                autoFocus
                disabled={busy}
                onClick={() => setReceipt(null)}
              >
                返回配对列表
              </button>
            </div>
            <p>
              {receipt.row.file.name} → {receipt.row.target?.title}
            </p>
            <button disabled={busy} onClick={() => setPreview(receipt.row)}>
              预览待导入原谱
            </button>
            {receipt.matches.length ? (
              <>
                <p>
                  这些已登记谱面与待导入文件的实际字节一致。选择采用现有结果，避免再次添加。
                </p>
                {receipt.matches.map((match) => (
                  <div
                    className="batch-saved-row"
                    key={match.id + ":" + match.page}
                  >
                    <span>
                      {match.name}
                      {receipt.row.kind === "image"
                        ? ` · 第 ${match.page + 1} 页`
                        : ""}
                    </span>
                    <button
                      disabled={busy}
                      onClick={() => void resolveReceipt(match)}
                    >
                      采用此保存结果
                    </button>
                  </div>
                ))}
              </>
            ) : (
              <>
                <p>
                  指定演奏和纸谱下未找到同内容的已保存文件。可恢复为待保存项；这一步不会写入谱面。
                </p>
                <button
                  disabled={busy}
                  onClick={() => void resolveReceipt(null)}
                >
                  恢复为待保存项
                </button>
              </>
            )}
          </section>
        ) : picker ? (
          <section className="batch-target-picker" aria-label="选择配对曲目">
            <div className="batch-preview-heading">
              <h3>选择配对曲目</h3>
              <button disabled={picking} onClick={() => setPicker(null)}>
                返回文件列表
              </button>
            </div>
            <label>
              搜索曲名、作曲家或路径
              <input
                autoFocus
                value={query}
                disabled={picking}
                onChange={(e) => {
                  setQuery(e.target.value);
                  setTargetPage(0);
                }}
                aria-label="搜索配对曲目"
              />
            </label>
            <p>
              共 {targetRows.length} 首；同名曲目请根据作曲家及文件位置核对。
            </p>
            <div className="batch-target-list">
              {targetRows
                .slice(targetPage * 20, targetPage * 20 + 20)
                .map((t) => (
                  <button
                    key={pathKey(t.path)}
                    disabled={picking}
                    onClick={() => void choose(t)}
                  >
                    <strong>{t.title}</strong>
                    <span>{t.composer || "作曲家未填写"}</span>
                    <small>{t.path}</small>
                  </button>
                ))}
            </div>
            <div className="batch-preview-toolbar">
              <button
                disabled={picking || targetPage === 0}
                onClick={() => setTargetPage((p) => p - 1)}
              >
                上一组曲目
              </button>
              <span>
                {targetPage + 1} /{" "}
                {Math.max(1, Math.ceil(targetRows.length / 20))}
              </span>
              <button
                disabled={picking || (targetPage + 1) * 20 >= targetRows.length}
                onClick={() => setTargetPage((p) => p + 1)}
              >
                下一组曲目
              </button>
            </div>
          </section>
        ) : (
          <>
            <div
              className="batch-import-actions"
              onDragOver={(e) => e.preventDefault()}
              onDrop={(e) => {
                e.preventDefault();
                e.stopPropagation();
                add(Array.from(e.dataTransfer.files));
              }}
            >
              <button disabled={busy} onClick={() => input.current?.click()}>
                添加文件
              </button>
              <span>也可拖入文件，最多 200 项</span>
              <input
                hidden
                ref={input}
                aria-label="批量配对文件"
                type="file"
                multiple
                accept=".mid,.midi,.xml,.musicxml,.mxl,.pdf,.png,.jpg,.jpeg,.webp"
                onChange={(e) => {
                  add(Array.from(e.target.files ?? []));
                  e.target.value = "";
                }}
              />
              <button
                disabled={busy || !items.length}
                onClick={() => void prepare()}
              >
                {pendingMidi ? "导入 MIDI 并读取配对候选" : "读取曲库配对候选"}
              </button>
            </div>
            <p className="parameter-help">
              MIDI
              导入后保存到本地曲库。文件名相同只表示候选，不保证谱面版本一致，请预览核对。每份
              PDF
              和演奏乐谱分别保存；同一曲目、同一纸谱名称的图片按下表顺序组页。原文件保留。
            </p>
            <div className="batch-select-actions">
              <button
                disabled={busy || !prepared}
                onClick={() =>
                  setItems((old) =>
                    old.map((r) =>
                      r.target &&
                      r.kind !== "midi" &&
                      r.state !== "done" &&
                      r.state !== "review"
                        ? { ...r, selected: true }
                        : r,
                    ),
                  )
                }
              >
                选中已配对项
              </button>
              <button
                disabled={busy}
                onClick={() =>
                  setItems((old) =>
                    old.map((r) =>
                      r.state !== "done" ? { ...r, selected: false } : r,
                    ),
                  )
                }
              >
                取消待保存项
              </button>
              <span>
                选中 {count} 项 · 已完成{" "}
                {items.filter((r) => r.state === "done").length} /{" "}
                {items.length}
              </span>
            </div>
            <div className="batch-score-table-wrap">
              <table className="batch-score-table">
                <thead>
                  <tr>
                    <th>保存</th>
                    <th>文件与结果</th>
                    <th>归属曲目</th>
                    <th>纸谱与操作</th>
                  </tr>
                </thead>
                <tbody>
                  {visible.map((row) => {
                    const index = items.indexOf(row),
                      locked = busy || row.state === "done";
                    return (
                      <tr key={row.id} data-state={row.state}>
                        <td>
                          <input
                            type="checkbox"
                            aria-label={`保存 ${row.file.name}`}
                            checked={row.selected}
                            disabled={
                              locked ||
                              row.state === "review" ||
                              row.kind === "midi" ||
                              !row.target
                            }
                            onChange={(e) =>
                              update(row.id, { selected: e.target.checked })
                            }
                          />
                        </td>
                        <td>
                          <strong>{row.file.name}</strong>
                          <small>
                            {labels[row.kind]} ·{" "}
                            {(row.file.size / 1024).toFixed(0)} KB
                          </small>
                          <span
                            className={
                              row.state === "error" ? "batch-result-error" : ""
                            }
                            role={row.state === "error" ? "alert" : undefined}
                          >
                            {row.message}
                          </span>
                        </td>
                        <td>
                          {row.target ? (
                            <>
                              <strong>{row.target.title}</strong>
                              <small>{row.target.composer}</small>
                              <small title={row.target.path}>
                                {row.target.path}
                              </small>
                            </>
                          ) : (
                            <span>尚未指定</span>
                          )}
                          {row.kind !== "midi" && (
                            <button
                              disabled={locked || !prepared}
                              onClick={() => {
                                setPicker(row.id);
                                setQuery(row.target?.title ?? "");
                                setTargetPage(0);
                              }}
                            >
                              选择曲目
                            </button>
                          )}
                        </td>
                        <td>
                          {row.kind === "image" && (
                            <label>
                              图片纸谱名称
                              <input
                                value={row.group}
                                maxLength={120}
                                disabled={locked || row.state === "review"}
                                aria-label={`纸谱名称 ${row.file.name}`}
                                onChange={(e) =>
                                  update(row.id, { group: e.target.value })
                                }
                              />
                            </label>
                          )}
                          <div className="batch-row-actions">
                            {row.state === "review" && (
                              <button
                                disabled={busy}
                                onClick={() => void checkInterrupted(row)}
                              >
                                核对中断项
                              </button>
                            )}
                            {row.kind !== "midi" && (
                              <button
                                disabled={busy}
                                onClick={() => setPreview(row)}
                              >
                                预览
                              </button>
                            )}
                            {row.kind === "image" && (
                              <>
                                <button
                                  aria-label={`上移 ${row.file.name}`}
                                  disabled={
                                    locked ||
                                    row.state === "review" ||
                                    index === 0
                                  }
                                  onClick={() => move(row.id, -1)}
                                >
                                  上移
                                </button>
                                <button
                                  aria-label={`下移 ${row.file.name}`}
                                  disabled={
                                    locked ||
                                    row.state === "review" ||
                                    index + 1 === items.length
                                  }
                                  onClick={() => move(row.id, 1)}
                                >
                                  下移
                                </button>
                              </>
                            )}
                            {row.state !== "done" && (
                              <button
                                disabled={busy}
                                onClick={() => {
                                  setItems((old) =>
                                    old.filter((r) => r.id !== row.id),
                                  );
                                  setPage(0);
                                }}
                              >
                                移出
                              </button>
                            )}
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
              {!items.length && (
                <p>先添加 MIDI 与对应谱面，也可以只为已有曲目添加谱面。</p>
              )}
            </div>
            <div className="batch-preview-toolbar">
              <button
                disabled={page === 0}
                onClick={() => setPage((p) => p - 1)}
              >
                上一组文件
              </button>
              <span>
                {page + 1} / {Math.max(1, Math.ceil(items.length / 25))}
              </span>
              <button
                disabled={(page + 1) * 25 >= items.length}
                onClick={() => setPage((p) => p + 1)}
              >
                下一组文件
              </button>
            </div>
          </>
        )}
        {storageError && (
          <div className="batch-result-error" role="alert">
            <p>批次保存失败：{storageError}</p>
            <button disabled={busy} onClick={close}>
              保留上次存档并关闭
            </button>
          </div>
        )}
        {error && (
          <p role="alert" className="batch-result-error">
            {error}
          </p>
        )}
        {!preview && !picker && !receipt && !batchesOpen && !archiveReview && (
          <footer className="batch-import-footer">
            <button disabled={busy} onClick={() => void leave(back)}>
              返回直接导入
            </button>
            {busy ? (
              <button
                onClick={() => {
                  stop.current = true;
                }}
              >
                完成当前文件后暂停
              </button>
            ) : (
              <button
                className="primary"
                disabled={!prepared || !count}
                onClick={() => void save()}
              >
                保存选中配对（{count}）
              </button>
            )}
            <button disabled={busy} onClick={() => void leave(close)}>
              完成
            </button>
          </footer>
        )}
      </section>
    </div>
  );
}
