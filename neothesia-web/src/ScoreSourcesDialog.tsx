import { useEffect, useRef, useState } from "react";
import { X, RefreshCw } from "lucide-react";
import { api } from "./api";
import { ScoreSourceBatch } from "./ScoreSourceBatch";

export type Source = {
  id: string;
  contentId: string;
  versionId: string;
  name: string;
  songTitle: string;
  midiPath: string;
  path: string;
  status: string;
  pending: boolean;
  active: boolean;
  defaultBpm: number;
  fingerprint: string | null;
  error: string | null;
};
export type Preview = {
  id: string;
  fingerprint: string;
  name: string;
  title: string | null;
  newContentId: string;
  performanceReady: boolean;
  performanceError: string | null;
  writtenNotes: number;
  samePerformance: boolean | null;
  notes: number | null;
  duration: number | null;
  oldAvailable: boolean;
  pairing: {
    coverage: number;
    readiness: string;
    diagnostics: string[];
  } | null;
  warnings: string[];
};
const states: Record<string, string> = {
  ready: "原文件与保存版本一致",
  changed: "原文件有更新",
  acknowledged: "已保留当前版本",
  missing: "原文件缺失",
  unreadable: "原文件无法读取",
};
const durationText = (s: number) => {
  const total = Math.round(s);
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
};
const displayPath = (path: string) => path.replace(/^\\\\\?\\/, "");

export function ScoreSourcesDialog({
  contentId,
  close,
  changed,
}: {
  contentId?: string;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [entries, setEntries] = useState<Source[]>([]),
    [preview, setPreview] = useState<Preview | null>(null),
    [selected, setSelected] = useState<string | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [message, setMessage] = useState(""),
    [query, setQuery] = useState(""),
    [filter, setFilter] = useState("all");
  const [page, setPage] = useState(0);
  const [chosen, setChosen] = useState<Set<string>>(new Set());
  const [batch, setBatch] = useState(false);
  useEffect(() => setPage(0), [query, filter]);
  const working = useRef(false),
    alive = useRef(true),
    generation = useRef(0);
  const refresh = async () => {
    const ticket = ++generation.current;
    const data = await api.command<{ entries: Source[] }>({
      type: "scoreSources",
      content_id: contentId ?? null,
    });
    if (alive.current && ticket === generation.current)
      setEntries(data.entries);
  };
  useEffect(() => {
    alive.current = true;
    setPreview(null);
    setSelected(null);
    void refresh().catch((e) => alive.current && setError(String(e)));
    const timer = setInterval(() => {
      if (!working.current)
        void refresh().catch((e) => alive.current && setError(String(e)));
    }, 5000);
    return () => {
      alive.current = false;
      generation.current++;
      clearInterval(timer);
    };
  }, [contentId]);
  const work = async (job: () => Promise<void>) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await job();
      await refresh();
    } catch (e) {
      if (alive.current) {
        setError(String(e));
        setPreview(null);
      }
    } finally {
      working.current = false;
      if (alive.current) setBusy(false);
    }
  };
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopImmediatePropagation();
        if (!working.current) close();
      }
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [close]);
  const source = entries.find((s) => s.id === selected);
  const currentPreview =
    preview && source?.fingerprint === preview.fingerprint ? preview : null;
  const apply = (mode: "notation" | "performance") =>
    void work(async () => {
      if (!currentPreview) return;
      const result = await api.command<{
        mode: string;
        contentId: string;
        warnings: string[];
      }>({
        type: "applyScoreSource",
        id: currentPreview.id,
        fingerprint: currentPreview.fingerprint,
        mode,
      });
      if (alive.current) {
        setPreview(null);
        setMessage(
          [
            mode === "notation"
              ? "已添加新的谱面版本。原版本和演奏保持；需要时再选择使用新谱面。"
              : "已生成并打开新谱演奏，旧曲目和成绩仍保留。",
            ...result.warnings,
          ].join(" "),
        );
      }
      await changed();
    });
  const rows = entries.filter(
    (s) =>
      (filter === "all" ||
        (filter === "changed" && s.pending) ||
        (filter === "unavailable" &&
          ["missing", "unreadable"].includes(s.status))) &&
      `${s.songTitle} ${s.name} ${s.path}`
        .toLowerCase()
        .includes(query.toLowerCase()),
  );
  useEffect(
    () =>
      setPage((n) => Math.min(n, Math.max(0, Math.ceil(rows.length / 50) - 1))),
    [rows.length],
  );
  return (
    <div
      className="modal-backdrop score-sources-backdrop"
      onClick={(e) => {
        e.stopPropagation();
        if (!busy) close();
      }}
    >
      <section
        className="settings-dialog score-sources-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="score-sources-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="score-sources-title">原谱文件</h2>
            <p>保存版本与外部 MusicXML / MXL 原文件的对应关系</p>
          </div>
          <button
            aria-label="关闭原谱文件"
            autoFocus
            disabled={busy}
            onClick={close}
          >
            <X size={18} />
          </button>
        </div>
        <p className="parameter-help">
          原文件修改后，保存的谱面、演奏和个人资料继续保留。先预览，再选择添加谱面版本或生成新演奏。这里的文件位置仅保存在本机。
        </p>
        <div className="score-source-query">
          <input
            aria-label="查找原谱文件"
            placeholder="查找版本名称或原文件位置"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <select
            aria-label="原谱文件状态筛选"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          >
            <option value="all">全部关联</option>
            <option value="changed">有更新</option>
            <option value="unavailable">缺失或无法读取</option>
          </select>
          <button disabled={busy} onClick={() => void work(refresh)}>
            <RefreshCw size={14} />
            立即检查
          </button>
        </div>
        {error && (
          <p className="dialog-error" role="alert">
            {error}
          </p>
        )}
        {message && <p role="status">{message}</p>}
        {!entries.length && (
          <p className="parameter-help">
            尚未关联原谱。在桌面版直接打开乐谱或在曲库添加演奏乐谱时会建立关联；已保存版本也可手动关联与其内容一致的原文件。
          </p>
        )}
        <div className="score-source-batch-toolbar">
          <button
            disabled={busy || batch}
            onClick={() =>
              setChosen((previous) => {
                const next = new Set(previous);
                for (const row of rows.filter(
                  (row) => row.pending && row.fingerprint,
                )) {
                  if (next.size >= 100) break;
                  next.add(row.id);
                }
                return next;
              })
            }
          >
            选择筛选结果中的更新
          </button>
          <button
            disabled={busy || batch || !chosen.size}
            onClick={() => setChosen(new Set())}
          >
            清空选择
          </button>
          <button
            className="primary-button"
            disabled={
              busy ||
              batch ||
              !entries.some((row) => chosen.has(row.id) && row.pending)
            }
            onClick={() => {
              setPreview(null);
              setBatch(true);
            }}
          >
            批量预览 ·{" "}
            {entries.filter((row) => chosen.has(row.id) && row.pending).length}
          </button>
          <span className="parameter-help">
            每批最多 100 个版本，跨页保留选择
          </span>
        </div>
        {batch && (
          <ScoreSourceBatch
            sources={entries.filter((row) => chosen.has(row.id) && row.pending)}
            run={work}
            busy={busy}
            changed={changed}
            close={() => {
              setBatch(false);
              setChosen(new Set());
            }}
          />
        )}
        <div className="score-source-list" hidden={batch}>
          {rows.slice(page * 50, (page + 1) * 50).map((s) => (
            <article
              data-source-id={s.id}
              className={selected === s.id ? "selected" : ""}
              key={s.id}
            >
              <header>
                <label className="score-source-choice">
                  <input
                    type="checkbox"
                    aria-label={`选择更新：${s.songTitle} · ${s.name}`}
                    disabled={
                      busy ||
                      batch ||
                      !s.pending ||
                      !s.fingerprint ||
                      (!chosen.has(s.id) && chosen.size >= 100)
                    }
                    checked={chosen.has(s.id)}
                    onChange={(event) =>
                      setChosen((previous) => {
                        const next = new Set(previous);
                        if (event.target.checked) next.add(s.id);
                        else next.delete(s.id);
                        return next;
                      })
                    }
                  />
                  <strong>
                    {s.songTitle} · {s.name}
                    {s.active ? " · 当前关联" : ""}
                  </strong>
                </label>
                <span className={s.pending ? "source-updated" : "muted"}>
                  {states[s.status]}
                </span>
              </header>
              <p className="source-path" title={displayPath(s.path)}>
                {displayPath(s.path)}
              </p>
              {s.error && <p className="parameter-help">{s.error}</p>}
              <div className="dialog-actions">
                <button
                  disabled={
                    busy || batch || !s.fingerprint || s.status === "ready"
                  }
                  onClick={() =>
                    void work(async () => {
                      setSelected(s.id);
                      const value = await api.command<Preview>({
                        type: "previewScoreSource",
                        id: s.id,
                        fingerprint: s.fingerprint,
                      });
                      if (alive.current) setPreview(value);
                    })
                  }
                >
                  预览更新
                </button>
                <button
                  disabled={
                    busy || batch || !s.fingerprint || s.status !== "changed"
                  }
                  onClick={() =>
                    void work(async () => {
                      await api.command({
                        type: "acknowledgeScoreSource",
                        id: s.id,
                        fingerprint: s.fingerprint,
                      });
                      setPreview(null);
                      setMessage(
                        "已保留当前版本；原文件再次修改时会重新显示更新。",
                      );
                    })
                  }
                >
                  保留当前版本
                </button>
                {api.desktop && (
                  <button
                    disabled={busy || batch}
                    onClick={() =>
                      void work(async () => {
                        const result = await api.linkScoreSource(
                          s.midiPath,
                          s.contentId,
                          s.versionId,
                        );
                        if (result) {
                          setPreview(null);
                          setMessage("已重新关联原文件。");
                        }
                      })
                    }
                  >
                    重新关联原文件
                  </button>
                )}
                <button
                  disabled={busy || batch}
                  onClick={() =>
                    void work(async () => {
                      await api.command({
                        type: "unlinkScoreSource",
                        id: s.id,
                        path: s.path,
                      });
                      setPreview(null);
                      setMessage("已取消原文件关联，保存谱面和曲目继续保留。");
                    })
                  }
                >
                  取消原文件关联
                </button>
              </div>
            </article>
          ))}
        </div>
        <div className="score-source-pagination" hidden={batch}>
          <button disabled={page === 0} onClick={() => setPage((n) => n - 1)}>
            上一页
          </button>
          <span>
            {rows.length
              ? `${page + 1} / ${Math.ceil(rows.length / 50)}`
              : "0 / 0"}{" "}
            · {rows.length} 个关联
          </span>
          <button
            disabled={(page + 1) * 50 >= rows.length}
            onClick={() => setPage((n) => n + 1)}
          >
            下一页
          </button>
        </div>
        {preview && !currentPreview && (
          <p role="status">原文件或关联已变化，请重新预览后应用。</p>
        )}
        {currentPreview && (
          <section className="score-source-preview" aria-label="原谱更新预览">
            <h3>{currentPreview.title || currentPreview.name}</h3>
            <p>
              {currentPreview.performanceReady
                ? `${currentPreview.notes} 个演奏音符 · ${durationText(currentPreview.duration ?? 0)}`
                : `${currentPreview.writtenNotes} 个书面谱音 · 演奏暂无法生成`}{" "}
              · 缺省速度 {source?.defaultBpm} BPM
            </p>
            <p>
              {!currentPreview.performanceReady
                ? "可以保存新谱面版本；演奏生成不可用的原因见下方，不自动改变当前演奏。"
                : currentPreview.samePerformance
                  ? "演奏内容身份相同，重新生成时继续使用已有曲目资料和成绩。"
                  : "演奏内容身份不同，将建立另一首曲目。旧成绩、逐音指法、分手、练习片段和计划不会自动迁到新音符。"}
            </p>
            <p>
              {currentPreview.pairing
                ? `与旧演奏对应 ${currentPreview.pairing.coverage}%${currentPreview.pairing.readiness === "Blocked" ? "，反复导航尚未通过" : ""}。添加谱面后仍需选择版本并核对跟随。`
                : "旧演奏文件不可用，仍可保存新谱面或生成新的演奏曲目。"}
            </p>
            {currentPreview.performanceError && (
              <p className="dialog-error">{currentPreview.performanceError}</p>
            )}
            {currentPreview.pairing?.diagnostics.map((w, i) => (
              <p className="parameter-help" key={`d${i}`}>
                {w}
              </p>
            ))}
            {currentPreview.warnings.map((w, i) => (
              <p className="parameter-help" key={`w${i}`}>
                {w}
              </p>
            ))}
            <div className="dialog-actions">
              <button disabled={busy} onClick={() => apply("notation")}>
                添加为新谱面版本
              </button>
              <button
                className="primary-button"
                disabled={busy || !currentPreview.performanceReady}
                onClick={() => apply("performance")}
              >
                {!currentPreview.performanceReady
                  ? "演奏生成不可用"
                  : currentPreview.samePerformance
                    ? "更新谱面并打开演奏"
                    : "建立并打开新演奏曲目"}
              </button>
              <button disabled={busy} onClick={() => setPreview(null)}>
                收起预览
              </button>
            </div>
          </section>
        )}
        <footer className="dialog-actions">
          <button
            disabled={busy}
            onClick={() =>
              void work(async () => {
                await api.command({ type: "stopRoutine" });
                await api.command({ type: "stopLadder" });
                await api.command({ type: "pause" });
                setMessage(
                  "已停止当前练习，可重新预览并应用更新。录音需先在录制窗口结束。",
                );
                await changed();
              })
            }
          >
            停止当前练习
          </button>
          <span className="parameter-help">
            {entries.length} 个版本关联 · 每 5 秒检查；应用前再次核对文件内容
          </span>
          <button disabled={busy} onClick={close}>
            完成
          </button>
        </footer>
      </section>
    </div>
  );
}
