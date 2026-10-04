import { useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { X } from "lucide-react";
import { api } from "./api";
import { PaperReader } from "./PaperScore";
type Row = {
  contentId: string;
  book: string;
  bookName: string;
  assetId: string;
  page: number;
  name: string;
  path: string;
  status: string;
  fingerprint: string | null;
  baseline: string;
  error: string | null;
};
type Snapshot = {
  rows: Row[];
  errors: string[];
  lastScan: number;
  revision: string;
};
type Preview = {
  baseline: string;
  fingerprint: string;
  asset: { id: string; name: string; kind: string; size: number };
  row: Row;
};
const labels: Record<string, string> = {
  same: "与副本一致",
  changed: "原件已更新",
  ignored: "已忽略这次更新",
  missing: "原件位置缺失",
  unavailable: "原件无法读取",
};
const identity = (r: Row) => `${r.contentId}:${r.book}:${r.assetId}`;
export function PaperSourceUpdatesDialog({
  close,
  changed,
}: {
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [data, setData] = useState<Snapshot>({
      rows: [],
      errors: [],
      lastScan: 0,
      revision: "",
    }),
    [selected, setSelected] = useState<string | null>(null),
    [query, setQuery] = useState(""),
    [filter, setFilter] = useState("needs"),
    [page, setPage] = useState(0),
    [path, setPath] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [message, setMessage] = useState(""),
    [preview, setPreview] = useState<Preview | null>(null);
  const alive = useRef(true),
    working = useRef(false);
  useEffect(() => {
    alive.current = true;
    let polling = false;
    const refresh = async () => {
      if (polling) return;
      polling = true;
      try {
        const v = await api.command<Snapshot>({ type: "paperSourceUpdates" });
        if (alive.current) setData(v);
      } catch (e) {
        if (alive.current) setError(String(e));
      } finally {
        polling = false;
      }
    };
    void refresh();
    const id = setInterval(() => void refresh(), 1500);
    return () => {
      alive.current = false;
      clearInterval(id);
    };
  }, []);
  const rows = data.rows.filter(
    (r) =>
      (filter === "all" ||
        (filter === "needs"
          ? ["changed", "missing", "unavailable"].includes(r.status)
          : r.status === filter)) &&
      [r.bookName, r.name, r.path, r.contentId].some((v) =>
        v.toLowerCase().includes(query.toLowerCase()),
      ),
  );
  const current = data.rows.find((r) => identity(r) === selected);
  const count = Math.max(1, Math.ceil(rows.length / 50));
  useEffect(() => setPage(0), [filter, query]);
  useEffect(() => setPage((p) => Math.min(p, count - 1)), [count]);
  useEffect(() => {
    if (!working.current) {
      setPreview(null);
      setPath(current?.path ?? "");
    }
  }, [selected, current?.baseline, current?.fingerprint]);
  async function work(fn: () => Promise<void>) {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await fn();
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      working.current = false;
      if (alive.current) setBusy(false);
    }
  }
  const check = async () => {
    await api.command({ type: "refreshLibrary" });
    if (alive.current)
      setMessage(
        "已请求重新检查，结果会自动更新；自动更新暂停时也可手动检查。",
      );
  };
  async function link(value: string | null) {
    if (!current) return;
    await api.command({
      type: "linkPaperSource",
      content_id: current.contentId,
      book: current.book,
      asset: current.assetId,
      path: value,
      baseline: current.baseline,
    });
    await changed();
    await api.command({ type: "refreshLibrary" });
    if (alive.current) {
      setPreview(null);
      setMessage(
        value
          ? "原件位置已保存，正在重新检查。"
          : "已解除原件关联，导入副本和练习资料保留。",
      );
    }
  }
  async function apply(action: string) {
    if (!preview) return;
    await api.command({
      type: "applyPaperSource",
      content_id: preview.row.contentId,
      book: preview.row.book,
      asset: preview.row.assetId,
      fingerprint: preview.fingerprint,
      baseline: preview.baseline,
      action,
    });
    await changed();
    await api.command({ type: "refreshLibrary" });
    if (alive.current) {
      setPreview(null);
      setMessage(
        action === "version"
          ? "已保存新版本。旧版批注和小节对应保留，可在谱面管理中审阅迁移。"
          : "已忽略当前内容；原件再次变化时会重新提示。",
      );
    }
  }
  return (
    <div className="modal-backdrop">
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="paper-updates-title"
        className="settings-dialog paper-updates-dialog"
      >
        <header className="dialog-heading">
          <h2 id="paper-updates-title">纸谱原件</h2>
          <button aria-label="关闭纸谱原件" disabled={busy} onClick={close}>
            <X size={18} />
          </button>
        </header>
        <p>
          集中检查已关联的
          PDF、图片原件。曲库的“自动更新”开关同时控制此检查；采用更新先核对谱页，原副本和资料保留。
        </p>
        <div className="paper-update-toolbar">
          <input
            aria-label="搜索纸谱原件"
            placeholder="搜索谱面、页名或原件位置"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <select
            aria-label="纸谱原件状态"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          >
            <option value="needs">需要处理</option>
            <option value="all">全部关联</option>
            {Object.entries(labels).map(([k, v]) => (
              <option key={k} value={k}>
                {v}
              </option>
            ))}
          </select>
          <button disabled={busy} onClick={() => void work(check)}>
            重新检查原件
          </button>
        </div>
        <small>
          {data.lastScan
            ? `上次检查 ${new Date(data.lastScan).toLocaleTimeString("zh-CN")} · ${data.rows.length} 处关联`
            : "尚无检查结果，可点击“重新检查原件”。"}
        </small>
        {data.errors.length > 0 && (
          <details>
            <summary>{data.errors.length} 个谱页目录需要处理</summary>
            {data.errors.slice(0, 50).map((e, i) => (
              <p key={i}>{e}</p>
            ))}
          </details>
        )}
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        {message && <p role="status">{message}</p>}
        <div className="paper-update-list">
          <table>
            <thead>
              <tr>
                <th>谱面 / 页</th>
                <th>状态</th>
                <th>原件位置</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              {rows.slice(page * 50, page * 50 + 50).map((r) => (
                <tr
                  key={identity(r)}
                  className={identity(r) === selected ? "selected" : ""}
                >
                  <td>
                    {r.bookName}
                    <small>
                      第 {r.page} 页 · {r.name}
                    </small>
                  </td>
                  <td>
                    {labels[r.status]}
                    {r.error && <small>{r.error}</small>}
                  </td>
                  <td title={r.path}>{r.path}</td>
                  <td>
                    <button
                      disabled={busy}
                      onClick={() => {
                        setSelected(identity(r));
                        setPreview(null);
                      }}
                    >
                      查看与处理
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {!rows.length && (
            <p>
              当前筛选没有原件需要处理。可切换“全部关联”，或在谱面管理中关联原文件。
            </p>
          )}
        </div>
        <div className="paper-update-toolbar">
          <button disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
            上一页
          </button>
          <span>
            {page + 1} / {count} · {rows.length} 处
          </span>
          <button
            disabled={page + 1 >= count}
            onClick={() => setPage((p) => p + 1)}
          >
            下一页
          </button>
        </div>
        {current && (
          <section className="paper-update-detail" aria-label="纸谱原件处理">
            <strong>
              {current.bookName} · 第 {current.page} 页 ·{" "}
              {labels[current.status]}
            </strong>
              <label>
              原件完整位置
              <input
                aria-label="纸谱原件新位置"
                disabled={busy}
                value={path}
                onChange={(e) => setPath(e.target.value)}
              />
            </label>
            <div className="paper-update-toolbar">
              {isTauri() && (
                <button
                  disabled={busy}
                  onClick={() =>
                    void work(async () => {
                      const p = await invoke<string | null>(
                        "pick_paper_source",
                      );
                      if (p) await link(p);
                    })
                  }
                >
                  选择原件
                </button>
              )}
              <button
                disabled={busy || !path.trim()}
                onClick={() => void work(() => link(path.trim()))}
              >
                关联相同内容的新位置
              </button>
              <button
                disabled={busy}
                onClick={() => void work(() => link(null))}
              >
                解除原件关联
              </button>
              {current.fingerprint &&
                ["changed", "ignored"].includes(current.status) && (
                  <button
                    disabled={busy}
                    onClick={() =>
                      void work(async () => {
                        const v = await api.command<Omit<Preview, "row">>({
                          type: "previewPaperSource",
                          content_id: current.contentId,
                          book: current.book,
                          asset: current.assetId,
                          fingerprint: current.fingerprint,
                        });
                        if (alive.current) setPreview({ ...v, row: current });
                      })
                    }
                  >
                    预览原件更新
                  </button>
                )}
            </div>
          </section>
        )}
        {preview && (
          <section
            className="paper-source-preview"
            aria-label="集中原件更新预览"
          >
            <strong>
              {preview.row.bookName} · {preview.asset.name}
            </strong>
            <PaperReader
              key={preview.fingerprint}
              contentId={preview.row.contentId}
              attachment={{
                id: preview.asset.id,
                name: preview.asset.name,
                format: preview.asset.kind === "pdf" ? "pdf" : "images",
                pages: [{ ...preview.asset, available: true }],
                view: { page: 1, zoom: 100, fit: true, rotation: 0 },
              }}
              previewKind={preview.asset.kind}
              disabled={busy}
              manage={false}
              practice={null}
              update={async () => {}}
              saveMapping={async () => {}}
              saveAnnotation={async () => false}
            />
            <div className="paper-update-toolbar">
              <button
                disabled={busy}
                onClick={() => void work(() => apply("version"))}
              >
                保存为新版本
              </button>
              <button
                disabled={busy}
                onClick={() => void work(() => apply("ignore"))}
              >
                忽略这次更新
              </button>
              <button disabled={busy} onClick={() => setPreview(null)}>
                关闭更新预览
              </button>
            </div>
          </section>
        )}
      </section>
    </div>
  );
}
