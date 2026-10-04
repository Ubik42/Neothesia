import { useRef, useState } from "react";
import { X, FolderOpen } from "lucide-react";
import { api, type LoadedSong } from "./api";
import { BatchScoreImportDialog } from "./BatchScoreImportDialog";
type Item = {
  file: File;
  state: "pending" | "working" | "done" | "error";
  message: string;
  id?: string;
};
export function ImportDialog({
  close,
  changed,
  initialFiles = [],
}: {
  close: () => void;
  changed: () => Promise<void>;
  initialFiles?: File[];
}) {
  const [pairOpen, setPairOpen] = useState(
    initialFiles.some((f) => /\.(pdf|png|jpe?g|webp)$/i.test(f.name)),
  );
  const [pairFiles, setPairFiles] = useState<File[]>(initialFiles);
  const [items, setItems] = useState<Item[]>(
      initialFiles
        .filter((f) => /\.(mid|midi|musicxml|xml|mxl)$/i.test(f.name))
        .slice(0, 500)
        .map((file) => ({ file, state: "pending", message: "等待导入" })),
    ),
    [busy, setBusy] = useState(false),
    [bpm, setBpm] = useState(120),
    [error, setError] = useState(
      initialFiles.length > 500
        ? "一次最多导入 500 个文件，请分批添加"
        : initialFiles.some(
              (f) => !/\.(mid|midi|musicxml|xml|mxl)$/i.test(f.name),
            )
          ? "支持 MIDI、MusicXML 与 MXL 文件，其他文件未加入队列"
          : "",
    );
  const input = useRef<HTMLInputElement>(null),
    stop = useRef(false);
  const add = (files: File[]) => {
    if (busy) return;
    if (files.some((f) => /\.(pdf|png|jpe?g|webp)$/i.test(f.name))) {
      setPairFiles([...items.map((row) => row.file), ...files]);
      setPairOpen(true);
      return;
    }
    const valid = files.filter((f) =>
      /\.(mid|midi|musicxml|xml|mxl)$/i.test(f.name),
    );
    if (items.length + valid.length > 500)
      setError("一次最多导入 500 个文件，请分批添加");
    else if (valid.length !== files.length)
      setError("支持 MIDI、MusicXML 与 MXL 文件");
    setItems((old) =>
      [
        ...old,
        ...valid.map((file) => ({
          file,
          state: "pending" as const,
          message: "等待导入",
        })),
      ].slice(0, 500),
    );
  };
  const update = (index: number, value: Partial<Item>) =>
    setItems((old) =>
      old.map((v, i) => (i === index ? { ...v, ...value } : v)),
    );
  const importFiles = async () => {
    setBusy(true);
    setError("");
    stop.current = false;
    try {
      for (let i = 0; i < items.length && !stop.current; i++) {
        const item = items[i];
        if (item.state === "done") continue;
        update(i, { state: "working", message: "正在读取…" });
        try {
          const score = /\.(musicxml|xml|mxl)$/i.test(item.file.name),
            limit = score ? 4_000_000 : 32_000_000;
          if (
            item.file.size > limit ||
            (!api.desktop && item.file.size > 3_000_000)
          )
            throw new Error(
              api.desktop
                ? "文件超过导入大小限制"
                : "浏览器一次支持 3 MB，较大文件请在桌面版打开",
            );
          const bytes = Array.from(
            new Uint8Array(await item.file.arrayBuffer()),
          );
          const song = await api.command<
            LoadedSong & { importWarnings?: string[] }
          >({
            type: score ? "importScore" : "importMidi",
            bytes,
            name: item.file.name,
            ...(score ? { default_bpm: bpm } : {}),
          });
          update(i, {
            state: "done",
            id: song.contentId,
            message: song.importWarnings?.length
              ? song.importWarnings.join("；")
              : "已保存到本地曲库",
          });
        } catch (e) {
          update(i, {
            state: "error",
            message: e instanceof Error ? e.message : String(e),
          });
        }
      }
      await changed();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  if (pairOpen)
    return (
      <BatchScoreImportDialog
        initialFiles={pairFiles}
        close={close}
        changed={changed}
        back={() => setPairOpen(false)}
      />
    );
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!busy) close();
      }}
    >
      <section
        className="settings-dialog fingering-dialog import-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="import-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="import-title">导入曲目与乐谱</h2>
            <p>MIDI · MusicXML · MXL · PDF / 图片配对</p>
          </div>
          <button
            autoFocus
            disabled={busy}
            aria-label="关闭导入"
            onClick={close}
          >
            <X size={18} />
          </button>
        </div>
        <div className="batch-import-actions">
          <button
            disabled={busy}
            onClick={() => {
              setPairFiles(items.map((row) => row.file));
              setPairOpen(true);
            }}
          >
            批量配对谱面
          </button>
          <span>为多首曲目整理演奏乐谱、PDF 与连续图片</span>
        </div>
        <div
          className="import-drop"
          onDragOver={(e) => {
            e.preventDefault();
          }}
          onDrop={(e) => {
            e.preventDefault();
            e.stopPropagation();
            add(Array.from(e.dataTransfer.files));
          }}
        >
          <FolderOpen size={24} />
          <span>把文件拖到这里，或选择多个文件</span>
          <button disabled={busy} onClick={() => input.current?.click()}>
            选择导入文件
          </button>
          <input
            hidden
            multiple
            ref={input}
            aria-label="导入曲目文件"
            type="file"
            accept=".mid,.midi,.musicxml,.xml,.mxl,.pdf,.png,.jpg,.jpeg,.webp"
            onChange={(e) => {
              add(Array.from(e.target.files ?? []));
              e.target.value = "";
            }}
          />
        </div>
        <div className="fingering-options">
          <label>
            乐谱默认速度
            <input
              aria-label="乐谱默认速度"
              type="number"
              min={20}
              max={400}
              value={bpm}
              disabled={busy}
              onChange={(e) => setBpm(Number(e.target.value))}
            />
          </label>
          <p className="parameter-help">
            文件保存在本地曲库，原文件保留。乐谱中的速度标记优先；常规反复和一、二房会展开为演奏顺序。
          </p>
        </div>
        {error && (
          <p className="dialog-error" role="alert">
            {error}
          </p>
        )}
        <div className="fingering-table-wrap">
          <table className="fingering-table">
            <thead>
              <tr>
                <th>文件</th>
                <th>状态</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              {items.map((item, i) => (
                <tr key={i}>
                  <td>
                    {item.file.name}
                    <div className="parameter-help">
                      {(item.file.size / 1024).toFixed(0)} KB
                    </div>
                  </td>
                  <td className={item.state === "error" ? "dialog-error" : ""}>
                    {item.message}
                  </td>
                  <td>
                    {item.id ? (
                      <button
                        disabled={busy}
                        onClick={async () => {
                          await api.command({
                            type: "openRecent",
                            content_id: item.id,
                          });
                          await changed();
                          close();
                        }}
                      >
                        打开练习
                      </button>
                    ) : (
                      <button
                        disabled={busy}
                        onClick={() =>
                          setItems((old) =>
                            old.filter((_, index) => index !== i),
                          )
                        }
                      >
                        移除
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="dialog-actions">
          <span>
            已完成 {items.filter((v) => v.state === "done").length} /{" "}
            {items.length}
          </span>
          {busy ? (
            <button
              onClick={() => {
                stop.current = true;
              }}
            >
              完成当前文件后暂停
            </button>
          ) : (
            <button onClick={close}>完成</button>
          )}
          <button
            className="primary-button"
            disabled={busy || !items.some((i) => i.state !== "done")}
            onClick={() => void importFiles()}
          >
            开始导入
          </button>
        </div>
      </section>
    </div>
  );
}
