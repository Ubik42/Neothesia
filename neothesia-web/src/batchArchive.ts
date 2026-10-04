import { invoke, isTauri } from "@tauri-apps/api/core";
import type { BatchData, BatchItem } from "./batchImportStore";
import { importKind, type PairTarget } from "./scorePairing";
const MAGIC = "NEOBAT01",
  CHUNK = 1_048_576,
  MAX = 512_000_000,
  META = 2_000_000;
type Entry = Omit<BatchItem, "file"> & {
  filename: string;
  mime: string;
  modified: number;
  size: number;
  hashes: string[];
};
type Manifest = {
  format: "neothesia-score-batch";
  version: 1;
  name: string;
  updated: number;
  items: Entry[];
};
const hash = async (b: Blob) =>
  Array.from(
    new Uint8Array(
      await crypto.subtle.digest("SHA-256", await b.arrayBuffer()),
    ),
  )
    .map((n) => n.toString(16).padStart(2, "0"))
    .join("");
export async function encodeBatch(
  batch: BatchData,
  progress: (text: string) => void,
  cancelled: () => boolean = () => false,
): Promise<Blob> {
  if (!batch.name.trim() || batch.name.trim().length > 120)
    throw new Error("批次名称需为 1 至 120 字");
  if (
    !batch.items.length ||
    batch.items.length > 200 ||
    batch.items.reduce((n, i) => n + i.file.size, 0) > MAX
  )
    throw new Error("批次需为 1 至 200 项，总文件大小不超过 512 MB");
  const items: Entry[] = [];
  for (let i = 0; i < batch.items.length; i++) {
    const { file, ...item } = batch.items[i],
      hashes: string[] = [];
    for (let at = 0; at < file.size; at += CHUNK) {
      if (cancelled()) throw new Error("已取消批次导出");
      progress(`核对文件 ${i + 1}/${batch.items.length} · ${file.name}`);
      hashes.push(await hash(file.slice(at, at + CHUNK)));
    }
    items.push({
      ...item,
      archiveSelected: item.selected,
      filename: file.name,
      mime: file.type,
      modified: file.lastModified,
      size: file.size,
      hashes,
    });
  }
  if (cancelled()) throw new Error("已取消批次导出");
  const manifest: Manifest = {
    format: "neothesia-score-batch",
    version: 1,
    name: batch.name.trim(),
    updated: Date.now(),
    items,
  };
  const metadata = new TextEncoder().encode(JSON.stringify(manifest));
  if (metadata.length > META) throw new Error("批次说明过大，请分批导出");
  const prefix = new Uint8Array(12);
  prefix.set(new TextEncoder().encode(MAGIC));
  new DataView(prefix.buffer).setUint32(8, metadata.length, true);
  return new Blob([prefix, metadata, ...batch.items.map((i) => i.file)], {
    type: "application/octet-stream",
  });
}
const text = (v: unknown, max: number, empty = true): v is string =>
  typeof v === "string" &&
  v.length <= max &&
  (empty || v.trim().length > 0) &&
  !/[\u0000-\u0008]/.test(v);
function target(v: unknown): v is PairTarget {
  if (v === undefined) return true;
  const t = v as PairTarget;
  return (
    !!t &&
    text(t.path, 32768, false) &&
    text(t.title, 512) &&
    text(t.composer, 512) &&
    Array.isArray(t.aliases) &&
    t.aliases.length <= 64 &&
    t.aliases.every((a) => text(a, 512)) &&
    (t.contentId === undefined || text(t.contentId, 200, false))
  );
}
export async function decodeBatch(
  file: Blob,
  progress: (text: string) => void,
): Promise<BatchData> {
  if (file.size < 12 || file.size > MAX + META + 12)
    throw new Error("批次文件大小无效");
  const header = new Uint8Array(await file.slice(0, 12).arrayBuffer());
  if (new TextDecoder().decode(header.slice(0, 8)) !== MAGIC)
    throw new Error("这不是 Neothesia 曲谱整理批次");
  const length = new DataView(header.buffer).getUint32(8, true);
  if (length < 1 || length > META || 12 + length > file.size)
    throw new Error("批次说明长度无效");
  let m: Manifest;
  try {
    m = JSON.parse(await file.slice(12, 12 + length).text());
  } catch {
    throw new Error("批次说明损坏");
  }
  if (
    !m ||
    m.format !== "neothesia-score-batch" ||
    m.version !== 1 ||
    !text(m.name, 120, false) ||
    !Array.isArray(m.items) ||
    !m.items.length ||
    m.items.length > 200 ||
    !Number.isSafeInteger(m.updated) ||
    m.updated < 0
  )
    throw new Error("批次格式或版本不支持");
  let offset = 12 + length,
    total = 0;
  const ids = new Set<string>(),
    items: BatchItem[] = [];
  for (let i = 0; i < m.items.length; i++) {
    const r = m.items[i];
    if (
      !r ||
      !text(r.id, 100, false) ||
      ids.has(r.id) ||
      !text(r.filename, 255, false) ||
      /[\/\\]/.test(r.filename) ||
      importKind(r.filename) !== r.kind ||
      !text(r.mime, 200) ||
      !Number.isSafeInteger(r.modified) ||
      r.modified < 0 ||
      !Number.isSafeInteger(r.size) ||
      r.size < 0 ||
      !text(r.group, 240) ||
      !text(r.message, 2000) ||
      typeof r.selected !== "boolean" ||
      typeof r.manual !== "boolean" ||
      (r.archiveSelected !== undefined &&
        typeof r.archiveSelected !== "boolean") ||
      !["pending", "working", "done", "error", "review"].includes(r.state) ||
      !target(r.target) ||
      !Array.isArray(r.hashes) ||
      r.hashes.length !== Math.ceil(r.size / CHUNK) ||
      !r.hashes.every((h) => typeof h === "string" && /^[a-f0-9]{64}$/.test(h))
    )
      throw new Error(`第 ${i + 1} 项文件说明无效`);
    ids.add(r.id);
    total += r.size;
    if (total > MAX || offset + r.size > file.size)
      throw new Error("批次文件副本长度无效");
    const blob = file.slice(offset, offset + r.size);
    for (let at = 0; at < r.size; at += CHUNK) {
      progress(`核对文件 ${i + 1}/${m.items.length} · ${r.filename}`);
      if (
        (await hash(blob.slice(at, at + CHUNK))) !==
        r.hashes[Math.floor(at / CHUNK)]
      )
        throw new Error(`文件副本损坏：${r.filename}`);
    }
    const {
      filename,
      mime,
      modified,
      size: _size,
      hashes: _hashes,
      ...row
    } = r;
    // Portable state never reuses the source machine's sheet IDs or assumes a write completed here.
    items.push({
      ...row,
      id: crypto.randomUUID(),
      file: new File([blob], filename, { type: mime, lastModified: modified }),
      selected: false,
      archiveSelected: row.archiveSelected ?? row.selected,
      manual: !!row.target,
      state:
        r.kind === "midi"
          ? "pending"
          : ["done", "working", "review"].includes(r.state)
            ? "review"
            : "pending",
      message:
        r.kind === "midi"
          ? "待在本机导入或核对演奏文件"
          : ["done", "working", "review"].includes(r.state)
            ? "请核对本机是否已有此谱面，再采用结果或继续保存"
            : "请核对本机配对后继续",
      attemptTarget: undefined,
    });
    offset += r.size;
  }
  if (offset !== file.size) throw new Error("批次包含无法解释的多余字节");
  return {
    id: crypto.randomUUID(),
    revision: 0,
    name: m.name,
    updated: Date.now(),
    items,
    prepared: false,
    books: [],
    midiTargets: [],
  };
}
export async function downloadBatch(
  blob: Blob,
  name: string,
  progress: (text: string) => void,
  cancelled: () => boolean = () => false,
): Promise<string | null> {
  const filename =
    name.replace(/[<>:\"/\\|?*\u0000-\u001f]/g, "_") + ".neoscorebatch";
  if (cancelled()) throw new Error("已取消批次导出");
  if (!isTauri()) {
    const url = URL.createObjectURL(blob),
      a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 30000);
    return filename;
  }
  const session = await invoke<string | null>("begin_score_batch_export", {
    name: filename,
    size: blob.size,
  });
  if (!session) return null;
  try {
    for (let at = 0; at < blob.size; at += CHUNK) {
      if (cancelled()) throw new Error("已取消批次导出");
      progress(`正在导出 ${Math.round((at / blob.size) * 100)}%`);
      await invoke("write_score_batch_export", {
        session,
        offset: at,
        bytes: Array.from(
          new Uint8Array(await blob.slice(at, at + CHUNK).arrayBuffer()),
        ),
      });
    }
    if (cancelled()) throw new Error("已取消批次导出");
    return await invoke<string>("finish_score_batch_export", { session });
  } catch (e) {
    await invoke("cancel_score_batch_export", { session }).catch(() => {});
    throw e;
  }
}
