import { ScoreSourcesDialog } from "./ScoreSourcesDialog";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { X, FileMusic, FileText, RefreshCw } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { isTauri } from "@tauri-apps/api/core";
import { api, type SongRow } from "./api";
import { PaperScore, type PaperAttachment } from "./PaperScore";
type Version = {
  id: string;
  name: string;
  path: string;
  active: boolean;
  status: string;
  size: number;
  copies: number;
  error?: string;
};
type Data = {
  contentId: string;
  path: string;
  songAvailable: boolean;
  versions: Version[];
  papers: { active: string | null; attachments: PaperAttachment[] };
};
const statuses: Record<string, string> = {
  ready: "文件可用",
  changed: "内容已改变",
  missing: "文件缺失",
  unreadable: "无法读取",
};
const fileSize = (size: number) =>
  size < 1024
    ? `${size} B`
    : size < 1_048_576
      ? `${(size / 1024).toFixed(1)} KB`
      : `${(size / 1_048_576).toFixed(1)} MB`;
export function LibraryScoresPane({
  row,
  contentId,
  changed,
  onBusy,
  openVersion,
}: {
  row: SongRow;
  contentId: string;
  changed: () => Promise<void>;
  onBusy: (busy: boolean) => void;
  openVersion: (
    row: SongRow,
    contentId: string,
    kind: "notation" | "paper",
    id: string,
  ) => Promise<void>;
}) {
  const [data, setData] = useState<Data | null>(null),
    [loading, setLoading] = useState(true),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [names, setNames] = useState<Record<string, string>>({}),
    [preview, setPreview] = useState<string | null>(null),
    [remove, setRemove] = useState<{
      id: string;
      kind: string;
      name: string;
    } | null>(null);
  const [imports, setImports] = useState<
    { name: string; ok: boolean; error?: string }[]
  >([]);
  const [progress, setProgress] = useState("");
  const [sourcesOpen, setSourcesOpen] = useState(false);
  const nativeBatch = useRef<string | null>(null);
  const [nativeRunning, setNativeRunning] = useState(false);
  const [pageRemoval, setPageRemoval] = useState<{
    id: string;
    page: number;
    name: string;
  } | null>(null);
  const filesInput = useRef<HTMLInputElement>(null),
    importContext = useRef<{ kind: string; target?: string; page: number }>({
      kind: "paper",
      page: 0,
    }),
    stopImport = useRef(false);
  const importFiles = async (
    files: File[],
    ctx: { kind: string; target?: string; page: number },
  ) => {
    await work(async () => {
      if (files.length > 100) throw new Error("一次最多添加 100 份文件");
      if (
        (ctx.kind === "repairPaper" ||
          (ctx.kind === "notation" && ctx.target)) &&
        files.length !== 1
      )
        throw new Error("修复时请选择一份原文件");
      stopImport.current = false;
      setImports([]);
      let append = ctx.target;
      const results: { name: string; ok: boolean; error?: string }[] = [];
      for (let i = 0; i < files.length; i++) {
        if (stopImport.current) {
          setProgress(`已停止；处理 ${i} / ${files.length} 份`);
          break;
        }
        const file = files[i];
        setProgress(`正在处理 ${i + 1} / ${files.length}：${file.name}`);
        try {
          const prefix = new Uint8Array(await file.slice(0, 5).arrayBuffer());
          const image =
            ctx.kind === "paper" && String.fromCharCode(...prefix) !== "%PDF-";
          const value = await api.addLibraryScore(
            row.path,
            contentId,
            file,
            image && append ? "append" : ctx.kind,
            image ? append : ctx.target,
            ctx.page,
          );
          if (image && !append) append = value.active;
          results.push({ name: file.name, ok: true });
        } catch (e) {
          results.push({
            name: file.name,
            ok: false,
            error: e instanceof Error ? e.message : String(e),
          });
        }
        setImports([...results]);
        if (i === files.length - 1)
          setProgress(
            `处理完成：成功 ${results.filter((r) => r.ok).length}，失败 ${results.filter((r) => !r.ok).length}`,
          );
      }
    });
  };
  const pick = (kind: string, target?: string, page = 0) => {
    const ctx = { kind, target, page };
    if (isTauri())
      void work(async () => {
        setProgress("选择谱面文件…");
        const batchId = crypto.randomUUID();
        nativeBatch.current = batchId;
        setNativeRunning(false);
        setImports([]);
        const unlisten = await listen<{
          batchId: string;
          current: number;
          total: number;
          name: string;
          results: { name: string; ok: boolean; error?: string }[];
        }>("library-score-import-progress", (e) => {
          if (e.payload.batchId !== nativeBatch.current) return;
          setNativeRunning(true);
          setImports(e.payload.results);
          setProgress(
            `正在处理 ${e.payload.current} / ${e.payload.total}：${e.payload.name}`,
          );
        });
        let value;
        try {
          value = await api.importLibraryScores(
            batchId,
            row.path,
            contentId,
            kind,
            target,
            page,
          );
        } finally {
          unlisten();
          nativeBatch.current = null;
          setNativeRunning(false);
        }
        if (value) {
          setImports(value.results);
          setProgress(
            `${value.stopped ? "已停止" : "处理完成"}：成功 ${value.results.filter((r) => r.ok).length}，失败 ${value.results.filter((r) => !r.ok).length}`,
          );
        } else setProgress("");
      });
    else {
      importContext.current = ctx;
      const input = filesInput.current!;
      input.accept =
        kind === "notation"
          ? ".musicxml,.xml,.mxl"
          : kind === "append"
            ? ".png,.jpg,.jpeg,.webp"
            : ".pdf,.png,.jpg,.jpeg,.webp";
      input.multiple = !(
        kind === "repairPaper" ||
        (kind === "notation" && target)
      );
      input.click();
    }
  };
  const arrange = (id: string, page: number, action: string, direction = 0) =>
    void work(async () => {
      await api.command({
        type: "updateLibraryPaper",
        path: row.path,
        content_id: contentId,
        id,
        page,
        action,
        direction,
      });
    });
  const generation = useRef(0),
    working = useRef(false),
    alive = useRef(true);
  const refresh = async () => {
    if (!alive.current) return;
    const g = ++generation.current;
    setLoading(true);
    try {
      const v = await api.command<Data>({
        type: "libraryScores",
        path: row.path,
        content_id: contentId,
      });
      if (g === generation.current) {
        setData(v);
        setError("");
      }
    } catch (e) {
      if (g === generation.current) {
        setData(null);
        setError(e instanceof Error ? e.message : String(e));
      }
    } finally {
      if (g === generation.current) setLoading(false);
    }
  };
  useEffect(() => {
    alive.current = true;
    setData(null);
    setNames({});
    setPreview(null);
    setRemove(null);
    setImports([]);
    setProgress("");
    setPageRemoval(null);
    void refresh();
    return () => {
      alive.current = false;
      generation.current++;
    };
  }, [row.path, contentId]);
  useEffect(() => {
    onBusy(busy);
    return () => onBusy(false);
  }, [busy, onBusy]);
  useEffect(() => {
    if (!preview) return;
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopImmediatePropagation();
        setPreview(null);
      }
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [preview]);
  const work = async (job: () => Promise<void>) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    try {
      await job();
      await refresh();
      await changed();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      working.current = false;
      setBusy(false);
    }
  };
  const mutate = (kind: string, id: string, action: string, name?: string) =>
    api.command({
      type: "manageLibraryScore",
      path: row.path,
      content_id: contentId,
      kind,
      id,
      action,
      name: name ?? null,
    });
  const nameEditor = (kind: string, id: string, name: string) => {
    const key = `${kind}:${id}`;
    return (
      <div className="library-score-name">
        <input
          aria-label={`版本名称 ${name}`}
          disabled={busy}
          value={names[key] ?? name}
          onChange={(e) => setNames((v) => ({ ...v, [key]: e.target.value }))}
        />
        <button
          disabled={
            busy ||
            (names[key] ?? name).trim() === name ||
            !(names[key] ?? name).trim()
          }
          onClick={() =>
            void work(async () => {
              await mutate(kind, id, "rename", names[key]);
              setNames((v) => {
                const n = { ...v };
                delete n[key];
                return n;
              });
            })
          }
        >
          保存名称
        </button>
      </div>
    );
  };
  return (
    <section className="library-score-versions" aria-label="曲目谱面版本">
      <input
        type="file"
        hidden
        ref={filesInput}
        aria-label="曲库添加谱面文件"
        onChange={(e) => {
          const files = Array.from(e.target.files ?? []);
          e.target.value = "";
          if (files.length) void importFiles(files, importContext.current);
        }}
      />
      <div className="library-score-heading">
        <h4>谱面资料</h4>
        <button
          aria-label="重新检查谱面资料"
          disabled={busy || loading}
          onClick={() => void refresh()}
        >
          <RefreshCw size={13} />
        </button>
      </div>
      {loading && <p role="status">正在核对谱面文件…</p>}
      {error && (
        <p role="alert" className="dialog-error">
          {error}
        </p>
      )}
      {data && (
        <>
          <p className="parameter-help">
            {data.versions.length} 份演奏乐谱 · {data.papers.attachments.length}{" "}
            份 PDF / 图片。查看和改名不会切换当前曲目。
          </p>
          {!data.songAvailable && (
            <p className="dialog-error">
              MIDI 缺失；仍可管理资料和预览谱页，进入练习前需重新定位文件。
            </p>
          )}
          <div className="library-score-actions">
            <button disabled={busy} onClick={() => pick("notation")}>
              添加演奏乐谱
            </button>
            <button disabled={busy} onClick={() => pick("paper")}>
              添加 PDF / 图片
            </button>
          </div>
          <p className="parameter-help">
            可多选文件；同批图片合并为一份多页谱，PDF
            各自保留。新演奏乐谱添加后可选择关联。
          </p>
          {progress && (
            <p role="status" aria-label="谱面文件处理进度">
              {progress}
              {busy && (!isTauri() || nativeRunning) && (
                <button
                  onClick={() => {
                    stopImport.current = true;
                    if (nativeBatch.current)
                      void api
                        .cancelLibraryScoreImport(nativeBatch.current)
                        .catch((e) => setError(String(e.message ?? e)));
                  }}
                >
                  处理完此份后停止
                </button>
              )}
            </p>
          )}
          {!!imports.length && (
            <details
              className="library-import-results"
              open={imports.some((r) => !r.ok)}
            >
              <summary>
                导入结果 · 成功 {imports.filter((r) => r.ok).length} /{" "}
                {imports.length}
              </summary>
              {imports.map((r, i) => (
                <p key={i} className={r.ok ? "parameter-help" : "dialog-error"}>
                  {r.name}：{r.ok ? "已保存" : r.error}
                </p>
              ))}
            </details>
          )}
          <h5>
            <FileMusic size={14} />
            演奏乐谱
          </h5>
          <button disabled={busy} onClick={() => setSourcesOpen(true)}>
            检查原谱更新
          </button>
          {!data.versions.length && (
            <p className="parameter-help">尚未添加 MusicXML / MXL 乐谱。</p>
          )}
          {data.versions.map((v) => (
            <article key={v.id} className="library-score-version">
              {nameEditor("notation", v.id, v.name)}
              <div className="library-score-state">
                <span className={v.status === "ready" ? "" : "unavailable"}>
                  {statuses[v.status] || v.status}
                </span>
                {v.active && <span className="score-default">当前关联</span>}
                <span>{v.status === "missing" ? "—" : fileSize(v.size)}</span>
                {v.copies > 1 && <span>{v.copies} 份相同登记</span>}
              </div>
              <details className="library-score-location">
                <summary title={v.path}>文件位置</summary>
                <p className="file-path">{v.path}</p>
              </details>
              {v.error && <small>{v.error}</small>}
              <div className="library-score-actions">
                {api.desktop && (
                  <button
                    disabled={busy || v.status !== "ready"}
                    onClick={() =>
                      void work(async () => {
                        await api.linkScoreSource(row.path, contentId, v.id);
                      })
                    }
                  >
                    关联原谱文件
                  </button>
                )}
                {v.status !== "ready" && (
                  <button
                    disabled={busy}
                    onClick={() => pick("notation", v.id)}
                  >
                    用原文件修复
                  </button>
                )}
                <button
                  disabled={busy || v.status !== "ready" || !data.songAvailable}
                  onClick={() =>
                    void work(() =>
                      openVersion(row, contentId, "notation", v.id),
                    )
                  }
                >
                  打开乐谱练习
                </button>
                {v.active ? (
                  <button
                    disabled={busy}
                    onClick={() =>
                      void work(async () => {
                        await mutate("notation", v.id, "detach");
                      })
                    }
                  >
                    解除关联
                  </button>
                ) : (
                  <button
                    disabled={
                      busy || v.status !== "ready" || !data.songAvailable
                    }
                    onClick={() =>
                      void work(async () => {
                        await mutate("notation", v.id, "select");
                      })
                    }
                  >
                    设为关联谱面
                  </button>
                )}
                {v.copies > 1 && (
                  <button
                    disabled={busy || v.status !== "ready"}
                    onClick={() =>
                      void work(async () => {
                        await mutate("notation", v.id, "deduplicate");
                      })
                    }
                  >
                    保留此版，合并重复登记
                  </button>
                )}
                <button
                  disabled={busy || v.active}
                  onClick={() =>
                    setRemove({ kind: "notation", id: v.id, name: v.name })
                  }
                >
                  移除登记
                </button>
              </div>
            </article>
          ))}
          <h5>
            <FileText size={14} />
            PDF / 图片谱页
          </h5>
          {!data.papers.attachments.length && (
            <p className="parameter-help">尚未添加 PDF 或图片。</p>
          )}
          {data.papers.attachments.map((a) => {
            const bad = a.pages.filter((p) => p.status !== "ready"),
              active = data.papers.active === a.id;
            return (
              <article key={a.id} className="library-score-version">
                {nameEditor("paper", a.id, a.name)}
                <div className="library-score-state">
                  <span>
                    {a.format === "pdf" ? "PDF" : `${a.pages.length} 页图片`}
                  </span>
                  {active && <span className="score-default">常用谱页</span>}
                  <span>
                    {fileSize(a.pages.reduce((n, p) => n + p.size, 0))}
                  </span>
                </div>
                <small>
                  阅读位置：第 {a.view.page} 页 ·{" "}
                  {a.view.fit ? "适合宽度" : `${a.view.zoom}%`} ·{" "}
                  {a.view.rotation}°
                </small>
                {bad.map((p) => (
                  <p key={p.id} className="dialog-error">
                    {p.name}：{statuses[p.status ?? "missing"]}
                  </p>
                ))}
                <details className="library-paper-pages">
                  <summary>
                    {a.format === "pdf"
                      ? "源文件与修复"
                      : `页序与修复 · ${a.pages.length} 页`}
                  </summary>
                  {a.pages.map((p, i) => (
                    <div key={p.id} className="library-paper-page">
                      <span>
                        {a.format === "images" ? `${i + 1}. ` : ""}
                        {p.name}
                      </span>
                      <small>{statuses[p.status ?? "missing"]}</small>
                      {p.status !== "ready" && (
                        <button
                          disabled={busy}
                          onClick={() => pick("repairPaper", a.id, i)}
                        >
                          修复此页原文件
                        </button>
                      )}
                      {a.format === "images" && (
                        <>
                          <button
                            disabled={busy || a.pages.length <= 1}
                            onClick={() =>
                              setPageRemoval({
                                id: a.id,
                                page: i,
                                name: `${p.name}${(a.annotations ?? []).filter((n) => n.assetId === p.id).length ? `（含 ${(a.annotations ?? []).filter((n) => n.assetId === p.id).length} 条批注，随页登记一起移除）` : ""}`,
                              })
                            }
                          >
                            移除页登记
                          </button>
                          <button
                            aria-label={`上移谱页 ${i + 1}`}
                            disabled={busy || i === 0}
                            onClick={() => arrange(a.id, i, "movePage", -1)}
                          >
                            ↑
                          </button>
                          <button
                            aria-label={`下移谱页 ${i + 1}`}
                            disabled={busy || i === a.pages.length - 1}
                            onClick={() => arrange(a.id, i, "movePage", 1)}
                          >
                            ↓
                          </button>
                        </>
                      )}
                    </div>
                  ))}
                </details>
                <div className="library-score-actions">
                  {a.format === "images" && (
                    <button
                      disabled={busy}
                      onClick={() => pick("append", a.id)}
                    >
                      追加图片页
                    </button>
                  )}
                  <button
                    disabled={busy || !!bad.length}
                    onClick={() => setPreview(a.id)}
                  >
                    预览谱页
                  </button>
                  <button
                    disabled={busy || !!bad.length || !data.songAvailable}
                    onClick={() =>
                      void work(() =>
                        openVersion(row, contentId, "paper", a.id),
                      )
                    }
                  >
                    打开谱页练习
                  </button>
                  {!active && (
                    <button
                      disabled={busy || !!bad.length}
                      onClick={() =>
                        void work(async () => {
                          await mutate("paper", a.id, "select");
                        })
                      }
                    >
                      设为常用
                    </button>
                  )}
                  <button
                    disabled={busy}
                    onClick={() =>
                      setRemove({ kind: "paper", id: a.id, name: a.name })
                    }
                  >
                    移除登记
                  </button>
                </div>
              </article>
            );
          })}
          {pageRemoval && (
            <div
              className="library-score-removal"
              role="group"
              aria-label="确认移除图片页"
            >
              <p>
                移除第 {pageRemoval.page + 1} 页“{pageRemoval.name}
                ”的登记？文件副本保留。
              </p>
              <button
                disabled={busy}
                onClick={() =>
                  void work(async () => {
                    await api.command({
                      type: "updateLibraryPaper",
                      path: row.path,
                      content_id: contentId,
                      id: pageRemoval.id,
                      page: pageRemoval.page,
                      action: "removePage",
                      direction: 0,
                    });
                    setPageRemoval(null);
                  })
                }
              >
                确认移除图片页
              </button>
              <button disabled={busy} onClick={() => setPageRemoval(null)}>
                取消移除页
              </button>
            </div>
          )}
          {remove && (
            <div
              className="library-score-removal"
              role="group"
              aria-label="确认移除谱面登记"
            >
              <p>移除“{remove.name}”的登记？文件副本仍保留。</p>
              <button
                disabled={busy}
                onClick={() =>
                  void work(async () => {
                    await mutate(remove.kind, remove.id, "remove");
                    setRemove(null);
                  })
                }
              >
                确认移除登记
              </button>
              <button disabled={busy} onClick={() => setRemove(null)}>
                取消
              </button>
            </div>
          )}
        </>
      )}
      {preview &&
        createPortal(
          <div
            className="modal-backdrop library-paper-backdrop"
            onClick={() => setPreview(null)}
          >
            <section
              role="dialog"
              aria-modal="true"
              aria-label="曲库谱页预览"
              className="settings-dialog library-paper-preview"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="dialog-heading">
                <div>
                  <h2>谱页预览</h2>
                  <p>
                    {row.title} ·{" "}
                    {
                      data?.papers.attachments.find((a) => a.id === preview)
                        ?.name
                    }
                  </p>
                </div>
                <button
                  autoFocus
                  aria-label="关闭谱页预览"
                  onClick={() => {
                    setPreview(null);
                    void refresh();
                  }}
                >
                  <X size={18} />
                </button>
              </div>
              <PaperScore
                contentId={contentId}
                libraryPath={row.path}
                previewBook={preview}
                onViewSaved={() => void refresh()}
              />
            </section>
          </div>,
          document.body,
        )}
      {sourcesOpen &&
        createPortal(
          <ScoreSourcesDialog
            contentId={contentId}
            close={() => setSourcesOpen(false)}
            changed={async () => {
              await refresh();
              await changed();
            }}
          />,
          document.body,
        )}
    </section>
  );
}
