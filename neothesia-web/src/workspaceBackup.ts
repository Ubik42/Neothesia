import { encodeBatch, decodeBatch } from "./batchArchive";
import type { BatchData } from "./batchImportStore";

const MAGIC = "NEOWRK01";
const CHUNK = 1_048_576;
const MAX_TOTAL = 1_024_000_000;
const MAX_PRACTICE = 256_000_000;
const MAX_BATCH = 514_000_012;
const MAX_META = 2_000_000;
type Entry = { name: string; size: number; hashes: string[] };
type Manifest = {
  format: "neothesia-workspace";
  version: 1;
  createdAt: number;
  practice: Entry;
  batches: Entry[];
};
const digest = async (blob: Blob): Promise<string> =>
  Array.from(
    new Uint8Array(
      await crypto.subtle.digest("SHA-256", await blob.arrayBuffer()),
    ),
  )
    .map((v) => v.toString(16).padStart(2, "0"))
    .join("");
const batchName = (index: number) =>
  `batch-${String(index + 1).padStart(4, "0")}.neoscorebatch`;
async function describe(
  blob: Blob,
  name: string,
  progress: (text: string) => void,
): Promise<Entry> {
  const hashes: string[] = [];
  for (let at = 0; at < blob.size; at += CHUNK) {
    progress(
      `正在核对备份内容 · ${name} · ${Math.floor(at / CHUNK) + 1}/${Math.ceil(blob.size / CHUNK)}`,
    );
    hashes.push(await digest(blob.slice(at, at + CHUNK)));
  }
  return { name, size: blob.size, hashes };
}
function validEntry(value: unknown, name: string, max: number): value is Entry {
  if (!value || typeof value !== "object") return false;
  const entry = value as Entry;
  return (
    entry.name === name &&
    Number.isSafeInteger(entry.size) &&
    entry.size > 0 &&
    entry.size <= max &&
    Array.isArray(entry.hashes) &&
    entry.hashes.length === Math.ceil(entry.size / CHUNK) &&
    entry.hashes.every((h) => typeof h === "string" && /^[a-f0-9]{64}$/.test(h))
  );
}
function validatePortableNames(batch: BatchData) {
  if (
    /[\u0000-\u001f\u007f]/.test(batch.name) ||
    batch.items.some(
      (item) =>
        /[\u0000-\u001f\u007f]/.test(item.file.name) ||
        item.file.name === "." ||
        item.file.name === "..",
    )
  )
    throw new Error("整理批次名称或文件名称包含无效字符");
}
export async function encodeWorkspaceBackup(
  practice: Blob,
  batches: BatchData[],
  progress: (text: string) => void,
): Promise<Blob> {
  if (
    !(practice instanceof Blob) ||
    practice.size < 1 ||
    practice.size > MAX_PRACTICE
  )
    throw new Error("练习备份大小必须在 1 字节至 256 MB 之间");
  if (
    !Array.isArray(batches) ||
    batches.length > 200 ||
    new Set(batches.map((b) => b.id)).size !== batches.length
  )
    throw new Error("完整备份最多包含 200 个不同的整理批次");
  const parts: Blob[] = [practice];
  const entries: Entry[] = [];
  let total = practice.size;
  const practiceEntry = await describe(
    practice,
    "practice.neopractice",
    progress,
  );
  for (let i = 0; i < batches.length; i++) {
    progress(
      `正在打包整理批次 ${i + 1}/${batches.length} · ${batches[i].name}`,
    );
    const blob = await encodeBatch(batches[i], progress);
    total += blob.size;
    if (total + 12 > MAX_TOTAL)
      throw new Error("完整备份超过 1.024 GB，请减少本次包含的整理批次");
    // The existing batch decoder validates filenames, metadata and portable states.
    validatePortableNames(await decodeBatch(blob, progress));
    entries.push(await describe(blob, batchName(i), progress));
    parts.push(blob);
  }
  const manifest: Manifest = {
    format: "neothesia-workspace",
    version: 1,
    createdAt: Date.now(),
    practice: practiceEntry,
    batches: entries,
  };
  const metadata = new TextEncoder().encode(JSON.stringify(manifest));
  if (
    metadata.byteLength > MAX_META ||
    total + metadata.byteLength + 12 > MAX_TOTAL
  )
    throw new Error("完整备份的说明或总大小超过限制");
  const prefix = new Uint8Array(12);
  prefix.set(new TextEncoder().encode(MAGIC));
  new DataView(prefix.buffer).setUint32(8, metadata.byteLength, true);
  return new Blob([prefix, metadata, ...parts], {
    type: "application/octet-stream",
  });
}
export async function decodeWorkspaceBackup(
  file: Blob,
  progress: (text: string) => void,
): Promise<{ practice: File; batches: BatchData[] }> {
  if (!(file instanceof Blob) || file.size < 13 || file.size > MAX_TOTAL)
    throw new Error("完整备份文件大小无效");
  const prefix = new Uint8Array(await file.slice(0, 12).arrayBuffer());
  if (new TextDecoder().decode(prefix.slice(0, 8)) !== MAGIC)
    throw new Error("这个文件不是 Neothesia 完整备份");
  const length = new DataView(prefix.buffer).getUint32(8, true);
  if (length < 1 || length > MAX_META || 12 + length > file.size)
    throw new Error("完整备份说明长度无效");
  const metadata = file.slice(12, 12 + length);
  let manifest: Manifest;
  try {
    manifest = JSON.parse(
      new TextDecoder("utf-8", { fatal: true }).decode(
        await metadata.arrayBuffer(),
      ),
    );
  } catch {
    throw new Error("完整备份说明损坏");
  }
  if (
    !manifest ||
    manifest.format !== "neothesia-workspace" ||
    manifest.version !== 1 ||
    !Number.isSafeInteger(manifest.createdAt) ||
    manifest.createdAt < 0 ||
    !validEntry(manifest.practice, "practice.neopractice", MAX_PRACTICE) ||
    !Array.isArray(manifest.batches) ||
    manifest.batches.length > 200 ||
    !manifest.batches.every((entry, i) =>
      validEntry(entry, batchName(i), MAX_BATCH),
    )
  )
    throw new Error("完整备份格式、文件名称或内容说明无效");
  const entries = [manifest.practice, ...manifest.batches];
  const expectedSize = entries.reduce(
    (size, entry) => size + entry.size,
    12 + length,
  );
  if (expectedSize !== file.size)
    throw new Error("完整备份内容缺失或包含多余字节");
  const parts: Blob[] = [];
  let offset = 12 + length;
  for (const entry of entries) {
    const part = file.slice(offset, offset + entry.size);
    for (let at = 0; at < entry.size; at += CHUNK) {
      progress(
        `正在核对备份内容 · ${entry.name} · ${Math.floor(at / CHUNK) + 1}/${entry.hashes.length}`,
      );
      if (
        (await digest(part.slice(at, at + CHUNK))) !==
        entry.hashes[Math.floor(at / CHUNK)]
      )
        throw new Error(`完整备份内容损坏：${entry.name}`);
    }
    parts.push(part);
    offset += entry.size;
  }
  const fingerprint = await digest(metadata);
  const batches: BatchData[] = [];
  for (let i = 1; i < parts.length; i++) {
    const batch = await decodeBatch(parts[i], progress);
    validatePortableNames(batch);
    // Reopening the same backup must not silently create another local copy.
    batch.id = `workspace-${fingerprint}-${i}`;
    batches.push(batch);
  }
  return {
    practice: new File([parts[0]], "practice.neopractice", {
      type: "application/octet-stream",
      lastModified: manifest.createdAt,
    }),
    batches,
  };
}
