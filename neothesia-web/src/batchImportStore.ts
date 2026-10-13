import type { PairTarget, ImportKind } from "./scorePairing";
export interface BatchItem {
  id: string;
  file: File;
  kind: ImportKind;
  selected: boolean;
  target?: PairTarget;
  manual: boolean;
  group: string;
  state: "pending" | "working" | "done" | "error" | "review";
  message: string;
  attemptTarget?: string;
  archiveSelected?: boolean;
}
export interface BatchData {
  id: string;
  name: string;
  revision: number;
  updated: number;
  items: BatchItem[];
  prepared: boolean;
  books: [string, string][];
  midiTargets: PairTarget[];
}
type Stored = Omit<BatchData, "items"> & {
  items: (Omit<BatchItem, "file"> & {
    filename: string;
    mime: string;
    modified: number;
    size: number;
  })[];
};
export type BatchSummary = Pick<
  BatchData,
  "id" | "name" | "revision" | "updated"
> & { total: number; done: number; bytes: number; interrupted: number };
const database = () =>
  new Promise<IDBDatabase>((resolve, reject) => {
    const r = indexedDB.open("neothesia-score-imports", 1);
    r.onupgradeneeded = () => {
      r.result.createObjectStore("batches", { keyPath: "id" });
      r.result.createObjectStore("files");
    };
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error);
    r.onblocked = () =>
      reject(new Error("另一个窗口正在更新批次存储，请关闭它后重试"));
  });
export async function listBatches(): Promise<BatchSummary[]> {
  const db = await database();
  try {
    return await new Promise((resolve, reject) => {
      const tx = db.transaction("batches", "readonly"),
        r = tx.objectStore("batches").getAll();
      r.onsuccess = () =>
        resolve(
          (r.result as Stored[])
            .map((b) => ({
              id: b.id,
              name: b.name,
              revision: b.revision,
              updated: b.updated,
              total: b.items.length,
              done: b.items.filter((i) => i.state === "done").length,
              bytes: b.items.reduce((n, i) => n + i.size, 0),
              interrupted: b.items.filter(
                (i) => i.state === "working" || i.state === "review",
              ).length,
            }))
            .sort((a, b) => b.updated - a.updated),
        );
      r.onerror = () => reject(r.error);
    });
  } finally {
    db.close();
  }
}
export async function saveBatch(value: BatchData): Promise<number> {
  if (!value.name.trim() || value.name.trim().length > 120)
    throw new Error("批次名称需为 1 至 120 字");
  if (
    value.items.length > 200 ||
    value.items.reduce((n, i) => n + i.file.size, 0) > 512_000_000
  )
    throw new Error("一个保存批次最多 200 项、512 MB，请分批整理");
  const db = await database();
  try {
    return await new Promise((resolve, reject) => {
      const tx = db.transaction(["batches", "files"], "readwrite"),
        batches = tx.objectStore("batches"),
        files = tx.objectStore("files");
      let reason = "",
        revision = 0;
      const r = batches.get(value.id);
      r.onsuccess = () => {
        const old = r.result as Stored | undefined;
        if ((old?.revision ?? 0) !== value.revision) {
          reason = "此批次已在另一个窗口更改，请重新打开保存版本后继续";
          tx.abort();
          return;
        }
        const ids = new Set(value.items.map((i) => i.id)),
          existing = new Set(old?.items.map((i) => i.id) ?? []);
        for (const item of old?.items ?? [])
          if (!ids.has(item.id)) files.delete([value.id, item.id]);
        for (const item of value.items)
          if (!existing.has(item.id)) files.put(item.file, [value.id, item.id]);
        revision = value.revision + 1;
        batches.put({
          ...value,
          name: value.name.trim(),
          revision,
          updated: Date.now(),
          items: value.items.map(({ file, ...item }) => ({
            ...item,
            filename: file.name,
            mime: file.type,
            modified: file.lastModified,
            size: file.size,
          })),
        });
      };
      tx.oncomplete = () => resolve(revision);
      tx.onabort = () =>
        reject(
          new Error(
            reason ||
              (tx.error?.name === "QuotaExceededError"
                ? "本机谱面副本存储空间不足，未保存此批次。请移除不需要的批次或分批整理"
                : tx.error?.message || "批次保存失败"),
          ),
        );
      tx.onerror = () => {};
    });
  } finally {
    db.close();
  }
}
export async function loadBatch(id: string): Promise<BatchData> {
  const db = await database();
  try {
    return await new Promise((resolve, reject) => {
      const tx = db.transaction(["batches", "files"], "readonly"),
        r = tx.objectStore("batches").get(id);
      let result: BatchData | undefined,
        reason = "";
      r.onsuccess = () => {
        const saved = r.result as Stored | undefined;
        if (!saved) {
          reason = "批次已移除";
          return;
        }
        const items: BatchItem[] = [];
        result = { ...saved, items };
        for (const row of saved.items) {
          const file = tx.objectStore("files").get([id, row.id]);
          file.onsuccess = () => {
            if (
              !(file.result instanceof Blob) ||
              file.result.size !== row.size
            ) {
              reason = `文件副本缺失：${row.filename}`;
              return;
            }
            const { filename, mime, modified, size: _size, ...item } = row;
            const interrupted = item.state === "working";
            items.push({
              ...item,
              file: new File([file.result], filename, {
                type: mime,
                lastModified: modified,
              }),
              state: interrupted
                ? item.kind === "midi"
                  ? "error"
                  : "review"
                : item.state,
              selected: interrupted ? false : item.selected,
              message: interrupted
                ? item.kind === "midi"
                  ? "上次导入中断，可继续读取候选"
                  : "上次保存中断，请核对已保存结果"
                : item.message,
            });
          };
        }
      };
      tx.oncomplete = () => {
        if (reason || !result) {
          reject(new Error(reason || "批次读取失败"));
          return;
        }
        // Requests run in insertion order; nevertheless use the saved order as
        // the authority rather than depending on callback scheduling.
        const order = (r.result as Stored).items.map((i) => i.id);
        result.items.sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id));
        resolve(result);
      };
      tx.onabort = () => reject(tx.error);
      tx.onerror = () => {};
    });
  } finally {
    db.close();
  }
}
export async function removeBatch(id: string, revision: number) {
  const db = await database();
  try {
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(["batches", "files"], "readwrite"),
        store = tx.objectStore("batches"),
        r = store.get(id);
      let reason = "";
      r.onsuccess = () => {
        const batch = r.result as Stored | undefined;
        if (!batch || batch.revision !== revision) {
          reason = "批次已变化，请刷新列表后再移除";
          tx.abort();
          return;
        }
        for (const item of batch.items)
          tx.objectStore("files").delete([id, item.id]);
        store.delete(id);
      };
      tx.oncomplete = () => resolve();
      tx.onabort = () =>
        reject(new Error(reason || tx.error?.message || "移除批次失败"));
      tx.onerror = () => {};
    });
  } finally {
    db.close();
  }
}

/** Restores a selected set of portable batches together, or writes none of them. */
export async function saveImportedBatches(values: BatchData[]): Promise<void> {
  if (!Array.isArray(values) || values.length > 200)
    throw new Error("一次最多恢复 200 个整理批次");
  if (!values.length) return;
  const ids = new Set<string>();
  let total = 0;
  for (const value of values) {
    if (
      !value ||
      typeof value.id !== "string" ||
      !value.id.trim() ||
      value.id.length > 100 ||
      ids.has(value.id)
    )
      throw new Error("恢复批次的标识无效或重复，所有批次均未写入");
    ids.add(value.id);
    if (
      typeof value.name !== "string" ||
      !value.name.trim() ||
      value.name.trim().length > 120 ||
      /[\u0000-\u001f]/.test(value.name)
    )
      throw new Error("批次名称需为 1 至 120 字，且不能包含控制字符");
    if (
      !Array.isArray(value.items) ||
      !value.items.length ||
      value.items.length > 200
    )
      throw new Error("一个恢复批次需包含 1 至 200 项文件");
    const itemIds = new Set<string>();
    let bytes = 0;
    for (const item of value.items) {
      if (
        !item ||
        typeof item.id !== "string" ||
        !item.id.trim() ||
        item.id.length > 100 ||
        itemIds.has(item.id) ||
        !(item.file instanceof File) ||
        !item.file.name.trim() ||
        item.file.name.length > 255 ||
        /[\/\\\u0000-\u001f]/.test(item.file.name) ||
        !Number.isSafeInteger(item.file.lastModified) ||
        item.file.lastModified < 0
      )
        throw new Error("恢复批次的文件说明无效或重复，所有批次均未写入");
      itemIds.add(item.id);
      bytes += item.file.size;
    }
    total += bytes;
    if (bytes > 512_000_000 || total > 1_024_000_000)
      throw new Error("恢复批次的文件大小超过限制，请减少本次选择");
  }
  const db = await database();
  try {
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(["batches", "files"], "readwrite");
      const batches = tx.objectStore("batches"),
        files = tx.objectStore("files");
      let reason = "";
      tx.oncomplete = () => resolve();
      tx.onabort = () =>
        reject(
          new Error(
            reason ||
              (tx.error?.name === "QuotaExceededError"
                ? "本机存储空间不足，本次所有批次均未写入。请移除不需要的整理批次后重试"
                : tx.error?.name === "ConstraintError"
                  ? "这份备份中的批次已恢复到本机，本次所有批次均未写入"
                  : tx.error?.message || "批次恢复失败，本次所有批次均未写入"),
          ),
        );
      tx.onerror = () => {};
      try {
        for (const value of values) {
          // add, rather than put, also rejects an unexpected orphan file collision.
          batches.add({
            ...value,
            name: value.name.trim(),
            revision: 1,
            updated: Date.now(),
            items: value.items.map(({ file, ...item }) => ({
              ...item,
              filename: file.name,
              mime: file.type,
              modified: file.lastModified,
              size: file.size,
            })),
          });
          for (const item of value.items)
            files.add(item.file, [value.id, item.id]);
        }
      } catch (error) {
        reason =
          error instanceof DOMException && error.name === "QuotaExceededError"
            ? "本机存储空间不足，本次所有批次均未写入。请移除不需要的整理批次后重试"
            : error instanceof Error
              ? error.message
              : "批次恢复失败，本次所有批次均未写入";
        tx.abort();
      }
    });
  } finally {
    db.close();
  }
}
