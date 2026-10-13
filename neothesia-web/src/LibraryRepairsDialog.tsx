import { useEffect, useRef, useState } from "react";
import { X, ChevronLeft, ChevronRight } from "lucide-react";
import { api } from "./api";
import { key } from "./libraryQuery";
type Location = {
  path: string;
  present: boolean;
  available: boolean;
  verified: boolean;
  reason: string;
  resolved: boolean;
  ignored: boolean;
  relinkedTo: string | null;
  primary: boolean;
  rating: number;
  hasScore: boolean;
};
type Group = {
  contentId: string;
  title: string;
  primaryPath: string | null;
  preferredPath: string | null;
  paths: Location[];
  candidates: string[];
  missing: number;
  unverified: number;
  available: number;
  resolved: number;
};
type Inventory = {
  groups: Group[];
  total: number;
  missingFiles: number;
  duplicateGroups: number;
  unverifiedFiles: number;
  resolvedFiles: number;
  replacementCount: number;
  offset: number;
  limit: number;
  revision: number;
};
type Replacement = {
  path: string; title: string; oldContentId: string; newContentId: string;
  oldNotes: number; newNotes: number; oldDuration: number; newDuration: number;
  oldTracks: number; newTracks: number; candidates: string[];
  reviewed: boolean; review?: { action: string; target: string | null } | null;
  available: boolean; error?: string | null;
};
type Replacements = { rows: Replacement[]; total: number; offset: number; limit: number; revision: number };
type ReplacementPreview = Replacement & {
  token: string; exact: { matched: number; oldOnly: number; newOnly: number } | null;
  oldSource: string | null;
};
type Item = { contentId: string; from: string; to: string };
export type RepairResult = {
  contentId: string;
  from: string;
  to: string;
  ok: boolean;
  error?: string;
  warnings?: string[];
};
const identity = (id: string, path: string) => `${id}|${key(path)}`;
export function LibraryRepairsDialog({
  close,
  changed,
  initialKind = "missing",
}: {
  close: () => void;
  changed: () => Promise<void>;
  initialKind?: string;
}) {
  const [indexProgress, setIndexProgress] = useState<{
    status: string;
    done: number;
    total: number;
    errors: number;
  } | null>(null);
  const [kind, setKind] = useState(initialKind),
    [query, setQuery] = useState(""),
    [resolved, setResolved] = useState(false),
    [offset, setOffset] = useState(0);
  const [data, setData] = useState<Inventory | null>(null),
    [replacementData, setReplacementData] = useState<Replacements | null>(null),
    [replacementPreview, setReplacementPreview] = useState<ReplacementPreview | null>(null),
    [replacementTarget, setReplacementTarget] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [results, setResults] = useState<RepairResult[]>([]);
  const [selected, setSelected] = useState<Record<string, Item>>({}),
    [choices, setChoices] = useState<Record<string, string>>({});
  const alive = useRef(true),
    requestVersion = useRef(0),
    working = useRef(false);
  const reload = async () => {
    const version = ++requestVersion.current;
    if (kind === "replacements") {
      const next = await api.command<Replacements>({ type: "midiReplacements", query, include_reviewed: resolved, offset, limit: 25 });
      if (alive.current && version === requestVersion.current) {
        setReplacementData(next);
        if (next.offset !== offset) setOffset(next.offset);
      }
      return;
    }
    const next = await api.command<Inventory>({
      type: "libraryRepairs",
      query,
      kind,
      include_resolved: resolved,
      offset,
      limit: 25,
    });
    if (alive.current && version === requestVersion.current) {
      setData(next);
      if (next.offset !== offset) setOffset(next.offset);
    }
  };
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      requestVersion.current++;
    };
  }, []);
  useEffect(() => {
    let stopped = false,
      timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      if (!working.current) {
        try {
          await reload();
          const progress = await api.command<{
            status: string;
            done: number;
            total: number;
            errors: number;
          }>({ type: "indexStatus" });
          if (!stopped) setIndexProgress(progress);
        } catch (e) {
          if (!stopped) setError(e instanceof Error ? e.message : String(e));
        }
      }
      if (!stopped) timer = setTimeout(poll, 3000);
    };
    void poll();
    return () => {
      stopped = true;
      clearTimeout(timer);
      requestVersion.current++;
    };
  }, [query, kind, resolved, offset]);
  useEffect(() => {
    setOffset(0);
    setSelected({});
    setChoices({});
    setResults([]);
    setReplacementPreview(null);
    setReplacementData(null);
  }, [kind, query, resolved]);
  useEffect(() => {
    const escape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (!working.current) close();
      }
    };
    window.addEventListener("keydown", escape, true);
    return () => window.removeEventListener("keydown", escape, true);
  }, [close]);
  const work = async (job: () => Promise<void>) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    try {
      await job();
      await reload();
      await changed();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      working.current = false;
      if (alive.current) setBusy(false);
    }
  };
  const choice = (g: Group, p: Location) => {
    const stored = choices[identity(g.contentId, p.path)];
    if (stored && g.candidates.includes(stored) && key(stored) !== key(p.path))
      return stored;
    if (g.candidates.length === 1 && key(g.candidates[0]) !== key(p.path))
      return g.candidates[0];
    if (
      kind === "duplicates" &&
      g.primaryPath &&
      key(g.primaryPath) !== key(p.path) &&
      g.candidates.includes(g.primaryPath)
    )
      return g.primaryPath;
    return "";
  };
  const select = (g: Group, p: Location, checked: boolean) => {
    const id = identity(g.contentId, p.path),
      to = choice(g, p);
    setSelected((old) => {
      const next = { ...old };
      if (checked && to) {
        if (Object.keys(next).length >= 100 && !next[id]) {
          setError("一次最多处理 100 个位置，请先处理已选项");
          return old;
        }
        next[id] = { contentId: g.contentId, from: p.path, to };
      } else delete next[id];
      return next;
    });
  };
  const choose = (g: Group, p: Location, to: string) => {
    const id = identity(g.contentId, p.path);
    setChoices((old) => ({ ...old, [id]: to }));
    setSelected((old) => {
      if (!old[id]) return old;
      const next = { ...old };
      if (to) next[id] = { contentId: g.contentId, from: p.path, to };
      else delete next[id];
      return next;
    });
  };
  const eligible = (p: Location) =>
    !p.resolved &&
    !p.ignored &&
    (kind === "duplicates" ? p.available : !p.available);
  const selectPage = () => {
    const next: Record<string, Item> = {};
    for (const g of data?.groups ?? [])
      for (const p of g.paths) {
        const to = choice(g, p);
        if (eligible(p) && to && Object.keys(next).length < 100)
          next[identity(g.contentId, p.path)] = {
            contentId: g.contentId,
            from: p.path,
            to,
          };
      }
    setSelected(next);
  };
  const repair = async (items: Item[]) => {
    const r = await api.command<{ results: RepairResult[] }>({
      type: "relinkLibrary",
      items,
    });
    setResults(r.results);
    setSelected({});
  };
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!busy) close();
      }}
    >
      <section
        className="settings-dialog library-repairs"
        role="dialog"
        aria-modal="true"
        aria-labelledby="library-repairs-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="library-repairs-title">文件位置</h2>
            <p>核对缺失文件和同内容的多个位置，保留原有练习记录。</p>
          </div>
          <button
            autoFocus
            aria-label="返回曲库管理"
            disabled={busy}
            onClick={close}
          >
            <X size={18} />
          </button>
        </div>
        <div
          className="library-repair-tabs"
          role="tablist"
          aria-label="文件位置分类"
        >
          {[
            ["missing", `缺失文件 ${data?.missingFiles ?? ""}`],
            ["replacements", `原路径内容变化 ${kind === "replacements" ? replacementData?.total ?? "" : data?.replacementCount ?? ""}`],
            ["duplicates", `重复路径 ${data?.duplicateGroups ?? ""}`],
            ["unverified", `待核对 ${data?.unverifiedFiles ?? ""}`],
            ["resolved", `已处理 ${data?.resolvedFiles ?? ""}`],
          ].map(([value, label]) => (
            <button
              key={value}
              role="tab"
              aria-selected={kind === value}
              disabled={busy}
              onClick={() => setKind(value)}
            >
              {label}
            </button>
          ))}
        </div>
        <div className="library-repair-toolbar">
          <input
            aria-label="文件位置搜索"
            placeholder="曲名或文件路径"
            value={query}
            disabled={busy}
            onChange={(e) => setQuery(e.target.value)}
          />
          <label>
            <input
              aria-label="显示已处理位置"
              type="checkbox"
              checked={resolved}
              disabled={busy}
              onChange={(e) => setResolved(e.target.checked)}
            />{" "}
            显示已处理位置
          </label>
          <button
            disabled={
              busy ||
              indexProgress?.status === "running" ||
              indexProgress?.status === "paused"
            }
            onClick={() =>
              void work(async () => {
                const paths = [
                  ...new Set(
                    data?.groups.flatMap((g) =>
                      g.paths.filter((p) => p.present).map((p) => p.path),
                    ) ?? [],
                  ),
                ];
                if (paths.length > 50000)
                  throw new Error("本页位置超过 50000 个，请按曲目逐项核对");
                if (paths.length) {
                  setIndexProgress(
                    await api.command({
                      type: "indexLibrary",
                      paths,
                      force: true,
                    }),
                  );
                }
              })
            }
          >
            检查本页位置
          </button>
        </div>
        {error && (
          <p role="alert" className="dialog-error">
            {error}
          </p>
        )}
        {indexProgress &&
          ["running", "paused"].includes(indexProgress.status) && (
            <p role="status" className="library-repair-help">
              正在核对位置 {indexProgress.done}/{indexProgress.total} ·{" "}
              {indexProgress.errors} 项需要处理
            </p>
          )}
        <p className="library-repair-help">
          {kind === "replacements"
            ? "文件在原路径换成新内容后，旧指法、批注和练习记录仍按旧 MIDI 内容保存。先预览，再选择旧曲目和新文件各自的去向。"
            : "关联前再次核对 MIDI 内容；相同名称不代表同一首曲目。新位置已有附加信息优先，原文件保留。"}
        </p>
        <div className="library-repair-scroll">
          {kind === "replacements" && replacementData?.rows.map((r) => {
            const preview = replacementPreview?.path === r.path && replacementPreview.oldContentId === r.oldContentId ? replacementPreview : null;
            const decided = r.review?.action === "keep" ? "保留旧曲目，暂不显示新文件" : r.review?.action === "new" ? "新文件作为新曲目显示" : r.review?.action === "relink" ? `旧曲目已关联至 ${r.review.target}` : "待审阅";
            return <section key={`${r.oldContentId}|${r.path}`} className="library-repair-group">
              <div className="library-repair-heading"><h3>{r.title}</h3><small>{decided}</small></div>
              <p title={r.path}>{r.path}</p>
              <p>旧 MIDI：{r.oldNotes} 个音符 · {r.oldTracks} 条声部 · {r.oldDuration.toFixed(1)} 秒 · 内容 {r.oldContentId.slice(0, 12)}</p>
              <p>现有 MIDI：{r.newNotes} 个音符 · {r.newTracks} 条声部 · {r.newDuration.toFixed(1)} 秒 · 内容 {r.newContentId.slice(0, 12)}</p>
              <button disabled={busy || !r.available} onClick={() => void work(async () => {
                const next = await api.command<ReplacementPreview>({type:"previewMidiReplacement",path:r.path,old_content_id:r.oldContentId,new_content_id:r.newContentId});
                setReplacementPreview(next);
                setReplacementTarget(next.candidates[0] ?? "");
              })}>核对音符与处理方式</button>
              {r.error && <p role="alert" className="dialog-error">{r.error}</p>}
              {preview && <div className="library-repair-path" aria-label="MIDI 内容变化预览">
                <div>
                  {preview.exact
                    ? <p>逐音核对：相同 {preview.exact.matched} 个，旧曲目独有 {preview.exact.oldOnly} 个，新文件独有 {preview.exact.newOnly} 个。核对所用旧文件：{preview.oldSource}</p>
                    : <p>旧 MIDI 原件暂不可用。当前只能比较索引记录中的音符数、声部数和时长；系统不会据此迁移旧指法或批注。</p>}
                  <p>旧内容和新内容的身份不同。练习历史继续归属旧内容。</p>
                  <label>旧曲目的正确位置 <select aria-label={`旧曲目的正确位置 ${r.path}`} value={replacementTarget} onChange={e=>setReplacementTarget(e.target.value)}>
                    <option value="">尚未找到</option>{preview.candidates.map(c=><option key={c} value={c}>{c}</option>)}
                  </select></label>
                </div>
                <div className="library-repair-path-actions">
                  <button disabled={busy} onClick={()=>void work(async()=>{await api.command({type:"decideMidiReplacement",path:r.path,old_content_id:r.oldContentId,new_content_id:r.newContentId,token:preview.token,action:"keep",target:null});setReplacementPreview(null);})}>保留旧曲目，隐藏新文件</button>
                  <button disabled={busy} onClick={()=>void work(async()=>{await api.command({type:"decideMidiReplacement",path:r.path,old_content_id:r.oldContentId,new_content_id:r.newContentId,token:preview.token,action:"new",target:null});setReplacementPreview(null);})}>新文件建立独立曲目</button>
                  <button disabled={busy || !replacementTarget} onClick={()=>void work(async()=>{await api.command({type:"decideMidiReplacement",path:r.path,old_content_id:r.oldContentId,new_content_id:r.newContentId,token:preview.token,action:"relink",target:replacementTarget});setReplacementPreview(null);})}>旧曲目重新关联到所选位置</button>
                </div>
              </div>}
            </section>;
          })}
          {kind === "replacements" && replacementData && !replacementData.rows.length && <p className="library-empty">{resolved ? "没有原路径内容变化记录" : "没有待审阅的原路径内容变化"}</p>}
          {kind !== "replacements" && <>
          {data?.groups.map((g) => (
            <section
              key={g.contentId}
              data-content-id={g.contentId}
              className="library-repair-group"
            >
              <div className="library-repair-heading">
                <h3>{g.title}</h3>
                {g.preferredPath && (
                  <button
                    disabled={busy}
                    onClick={() =>
                      void work(async () => {
                        await api.command({
                          type: "setLibraryPrimary",
                          content_id: g.contentId,
                          path: null,
                        });
                      })
                    }
                  >
                    使用默认位置
                  </button>
                )}
                <small>
                  {g.available} 个可用位置 · {g.missing} 个待修复
                  {g.resolved ? ` · ${g.resolved} 个已处理` : ""}
                </small>
              </div>
              {g.paths.map((p) => {
                const id = identity(g.contentId, p.path),
                  to = choice(g, p);
                return (
                  <div
                    key={id}
                    className={`library-repair-path ${p.resolved || p.ignored ? "resolved" : ""}`}
                  >
                    <div className="library-repair-original">
                      <input
                        aria-label={`处理位置 ${p.path}`}
                        type="checkbox"
                        checked={Boolean(selected[id])}
                        disabled={busy || !eligible(p) || !to}
                        onChange={(e) => select(g, p, e.target.checked)}
                      />
                      <div>
                        <strong>
                          {p.reason}
                          {p.primary ? " · 常用位置" : ""}
                          {p.resolved
                            ? " · 已关联"
                            : p.ignored
                              ? " · 已忽略旧位置"
                              : ""}
                        </strong>
                        <p title={p.path}>{p.path}</p>
                        {p.relinkedTo && <small>关联到 {p.relinkedTo}</small>}
                      </div>
                    </div>
                    <div className="library-repair-path-actions">
                      {p.resolved || p.ignored ? (
                        <button
                          disabled={busy}
                          onClick={() =>
                            void work(async () => {
                              await api.command({
                                type: "restoreLibraryPath",
                                content_id: g.contentId,
                                path: p.path,
                              });
                            })
                          }
                        >
                          恢复路径显示
                        </button>
                      ) : (
                        <>
                          {eligible(p) &&
                            g.candidates.some(
                              (c) => key(c) !== key(p.path),
                            ) && (
                              <>
                                <select
                                  aria-label={`新位置 ${p.path}`}
                                  value={to}
                                  disabled={busy}
                                  onChange={(e) => choose(g, p, e.target.value)}
                                >
                                  <option value="">选择可用位置</option>
                                  {g.candidates
                                    .filter((c) => key(c) !== key(p.path))
                                    .map((c) => (
                                      <option key={c} value={c}>
                                        {c}
                                      </option>
                                    ))}
                                </select>
                                <button
                                  disabled={busy || !to}
                                  onClick={() =>
                                    void work(() =>
                                      repair([
                                        {
                                          contentId: g.contentId,
                                          from: p.path,
                                          to,
                                        },
                                      ]),
                                    )
                                  }
                                >
                                  {p.available ? "合并显示" : "重新关联"}
                                </button>
                              </>
                            )}
                          {!p.available && p.verified && (
                            <button
                              disabled={busy}
                              onClick={() =>
                                void work(async () => {
                                  await api.command({
                                    type: "ignoreLibraryPath",
                                    content_id: g.contentId,
                                    path: p.path,
                                  });
                                })
                              }
                            >
                              忽略旧位置
                            </button>
                          )}
                          {!p.available && api.desktop && (
                            <button
                              disabled={busy}
                              onClick={() =>
                                void work(async () => {
                                  const r = await api.repairPath(
                                    g.contentId,
                                    p.path,
                                  );
                                  if (r) setResults(r.results);
                                })
                              }
                            >
                              选择文件关联
                            </button>
                          )}
                          {p.available && !p.primary && (
                            <button
                              disabled={busy}
                              onClick={() =>
                                void work(async () => {
                                  await api.command({
                                    type: "setLibraryPrimary",
                                    content_id: g.contentId,
                                    path: p.path,
                                  });
                                })
                              }
                            >
                              设为常用位置
                            </button>
                          )}
                          {p.present && !p.verified && (
                            <button
                              disabled={busy}
                              onClick={() =>
                                void work(async () => {
                                  await api.command({
                                    type: "inspectSong",
                                    path: p.path,
                                    force: true,
                                  });
                                })
                              }
                            >
                              核对内容
                            </button>
                          )}
                        </>
                      )}
                    </div>
                  </div>
                );
              })}
              {!g.candidates.length && (
                <p className="parameter-help">
                  未找到同内容的可用位置。可先导入原文件或添加所在文件夹，完成索引后再关联。
                </p>
              )}
            </section>
          ))}
          {data && !data.groups.length && (
            <p className="library-empty">
              {kind === "missing"
                ? "没有需要重新关联的文件"
                : kind === "duplicates"
                  ? "没有同内容的多个可用位置"
                  : kind === "unverified"
                    ? "文件位置已核对"
                    : "尚未关联文件位置"}
            </p>
          )}
          {!data && <p className="library-empty">正在读取文件位置…</p>}
          </>}
        </div>
        {results.length > 0 && (
          <div className="library-repair-results" role="status">
            <strong>
              本次处理：{results.filter((r) => r.ok).length} 个成功
              {results.some((r) => !r.ok)
                ? `，${results.filter((r) => !r.ok).length} 个未处理`
                : ""}
            </strong>
            {results.map((r, i) => (
              <p key={i} className={r.ok ? "" : "dialog-error"}>
                {r.from} → {r.to}
                <br />
                {r.ok ? "已关联，原文件保留" : r.error}
                {r.warnings?.map((w, j) => (
                  <small key={j}>{w}</small>
                ))}
              </p>
            ))}
          </div>
        )}
        <div className="library-repair-footer">
          <span>
            {kind === "replacements" ? `${replacementData?.total ?? 0} 处内容变化` : `${data?.total ?? 0} 首 · 已选择 ${Object.keys(selected).length} 个位置`}
          </span>
          {kind !== "replacements" && <><button disabled={busy || !data?.groups.length} onClick={selectPage}>
            选择本页可匹配项
          </button>
          <button
            disabled={busy || !Object.keys(selected).length}
            onClick={() => setSelected({})}
          >
            取消选择
          </button>
          <button
            className="primary-button"
            disabled={busy || !Object.keys(selected).length}
            onClick={() => void work(() => repair(Object.values(selected)))}
          >
            关联所选位置
          </button></>}
          <span className="menu-spacer" />
          <button
            aria-label="上一页文件位置"
            disabled={busy || !offset}
            onClick={() => {
              setOffset(Math.max(0, offset - 25));
              setSelected({});
            }}
          >
            <ChevronLeft size={14} />
          </button>
          <span>
            {Math.floor(offset / 25) + 1}/
            {Math.max(1, Math.ceil((kind === "replacements" ? replacementData?.total ?? 0 : data?.total ?? 0) / 25))}
          </span>
          <button
            aria-label="下一页文件位置"
            disabled={busy || offset + 25 >= (kind === "replacements" ? (replacementData?.total ?? 0) : (data?.total ?? 0))}
            onClick={() => {
              setOffset(offset + 25);
              setSelected({});
            }}
          >
            <ChevronRight size={14} />
          </button>
        </div>
      </section>
    </div>
  );
}
