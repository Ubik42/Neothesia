import { invoke, isTauri } from "@tauri-apps/api/core";

/** Downloads a prepared backup; desktop writes use an atomic temporary file. */
export async function downloadWorkspaceBackup(
  blob: Blob,
  name: string,
  progress: (text: string) => void,
  cancelled: () => boolean = () => false,
): Promise<string | null> {
  if (blob.size < 12 || blob.size > 1_073_741_824)
    throw new Error("资料备份大小无效，最大支持 1 GiB。");
  const safeName = name.replace(/[<>:\"/\\|?*\u0000-\u001f]/g, "_").trim();
  const filename =
    (safeName.replace(/\.neoworkspace$/i, "") || "Neothesia资料备份") +
    ".neoworkspace";
  const checkCancelled = () => {
    if (cancelled()) throw new Error("已取消资料备份导出");
  };
  checkCancelled();
  if (!isTauri()) {
    progress("正在下载资料备份…");
    checkCancelled();
    const url = URL.createObjectURL(blob);
    try {
      const link = document.createElement("a");
      link.href = url;
      link.download = filename;
      link.click();
    } finally {
      window.setTimeout(() => URL.revokeObjectURL(url), 30_000);
    }
    return filename;
  }
  const session = await invoke<string | null>("begin_workspace_export", {
    name: filename,
    size: blob.size,
  });
  if (!session) return null;
  try {
    const chunkSize = 1_048_576;
    for (let offset = 0; offset < blob.size; offset += chunkSize) {
      checkCancelled();
      const bytes = new Uint8Array(
        await blob.slice(offset, offset + chunkSize).arrayBuffer(),
      );
      checkCancelled();
      progress(`正在导出资料备份 ${Math.floor((offset / blob.size) * 100)}%`);
      await invoke("write_score_batch_export", {
        session,
        offset,
        bytes: Array.from(bytes),
      });
    }
    checkCancelled();
    progress("正在完成资料备份…");
    return await invoke<string>("finish_score_batch_export", { session });
  } catch (error) {
    await invoke("cancel_score_batch_export", { session }).catch(() => {});
    throw error;
  }
}
