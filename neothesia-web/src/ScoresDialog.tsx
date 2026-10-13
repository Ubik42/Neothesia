import { ScoreSourcesDialog } from "./ScoreSourcesDialog";
import { Fragment, useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { api, type LoadedSong } from "./api";
import { PaperScore } from "./PaperScore";
import {
  ScoreRoutePreview,
  type ScoreRouteEvidence,
} from "./ScoreRoutePreview";
type Version = {
  id: string;
  name: string;
  path: string;
  active: boolean;
  available: boolean;
};
type OriginalSource = { versionId: string; status: string; path: string };
export function ScoresDialog({
  song,
  close,
  changed,
}: {
  song: LoadedSong;
  close: () => void;
  changed: () => Promise<void>;
}) {
  const [versions, setVersions] = useState<Version[]>([]),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const [names, setNames] = useState<Record<string, string>>({});
  const [originals, setOriginals] = useState<OriginalSource[]>([]);
  const [editMessage, setEditMessage] = useState("");
  const [tab, setTab] = useState("notation"),
    [paperBusy, setPaperBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const [sourcesOpen, setSourcesOpen] = useState(false);
  const [versionPreview, setVersionPreview] = useState<{
    id: string;
    path: string;
    evidence: ScoreRouteEvidence;
  } | null>(null);
  const [filePreview, setFilePreview] = useState<{
    name: string;
    bytes: number[];
    evidence: ScoreRouteEvidence;
  } | null>(null);
  const refresh = async () => {
    const data = await api.command<{ versions: Version[] }>({
      type: "scoreVersions",
    });
    setVersions(data.versions);
    if (api.desktop) {
      const sources = await api.command<{ entries: OriginalSource[] }>({
        type: "scoreSources",
        content_id: song.contentId,
      });
      setOriginals(sources.entries);
    }
  };
  useEffect(() => {
    setVersionPreview(null);
    setFilePreview(null);
    setEditMessage("");
    void refresh().catch((e) => setError(String(e.message)));
  }, [song.contentId]);
  const previewVersion = async (version: Version) => {
    setBusy(true);
    setError("");
    setVersionPreview(null);
    try {
      const evidence = await api.command<ScoreRouteEvidence>({
        type: "previewScoreVersionRoute",
        id: version.id,
        content_id: song.contentId,
      });
      setVersionPreview({ id: version.id, path: version.path, evidence });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  };
  const previewFile = async (file: File) => {
    setBusy(true);
    setError("");
    setFilePreview(null);
    try {
      if (file.size > 4_000_000) throw new Error("乐谱超过 4 MB");
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const evidence = await api.command<ScoreRouteEvidence>({
        type: "previewScoreBytesRoute",
        bytes,
        name: file.name,
        content_id: song.contentId,
      });
      setFilePreview({ name: file.name, bytes, evidence });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  };
  const saveFile = (mode: "use" | "save") =>
    void perform(async () => {
      if (!filePreview) return;
      const fresh = await api.command<ScoreRouteEvidence>({
        type: "previewScoreBytesRoute",
        bytes: filePreview.bytes,
        name: filePreview.name,
        content_id: song.contentId,
      });
      if (
        !fresh.fingerprint ||
        fresh.fingerprint !== filePreview.evidence.fingerprint ||
        JSON.stringify(fresh.route) !==
          JSON.stringify(filePreview.evidence.route)
      ) {
        setFilePreview(null);
        throw new Error(
          "所选文件的内容或演奏顺序已变化。请重新选择文件并核对预览。",
        );
      }
      if (mode === "use" && fresh.route?.complete !== true)
        throw new Error("当前谱面的演奏顺序尚未确认。可以仅保存这个版本。");
      await api.command({
        type: mode === "use" ? "pairScore" : "saveScoreVersion",
        bytes: filePreview.bytes,
        name: filePreview.name,
        content_id: song.contentId,
      });
      setFilePreview(null);
      setVersionPreview(null);
    });
  const perform = async (work: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await work();
      await refresh();
      await changed();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!busy && !paperBusy) close();
      }}
    >
      <section
        className="settings-dialog fingering-dialog scores-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="scores-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="dialog-heading">
          <div>
            <h2 id="scores-title">谱面管理</h2>
            <p>{song.title}</p>
          </div>
          <button
            autoFocus
            aria-label="关闭谱面管理"
            disabled={busy || paperBusy}
            onClick={close}
          >
            <X size={18} />
          </button>
        </div>
        <div className="paper-manager-tabs">
          <button
            className={tab === "notation" ? "selected" : ""}
            disabled={busy || paperBusy}
            onClick={() => setTab("notation")}
          >
            演奏乐谱
          </button>
          <button
            className={tab === "paper" ? "selected" : ""}
            disabled={busy || paperBusy}
            onClick={() => setTab("paper")}
          >
            PDF / 图片谱面
          </button>
        </div>
        {tab === "paper" ? (
          <PaperScore contentId={song.contentId} manage onBusy={setPaperBusy} />
        ) : (
          <>
            <p className="parameter-help">
              同一首曲目可以保留不同版本的 MusicXML / MXL
              谱面。切换时重新核对音符对应关系；移除版本会从管理列表移除，文件仍保留。
            </p>
            {api.desktop && <p className="parameter-help">
              编辑原谱时，先为版本关联磁盘上的原文件，再用 MuseScore Studio 打开。保存修改后，返回这里点“检查原谱更新”，预览差异并决定是否采用。当前练习不会因外部保存而自动切换谱面。
            </p>}
            <div className="dialog-actions">
              <button
                className="primary-button"
                disabled={busy}
                onClick={() => input.current?.click()}
              >
                添加谱面版本
              </button>
              <span>{versions.length} 份谱面</span>
              <button disabled={busy} onClick={() => setSourcesOpen(true)}>
                检查原谱更新
              </button>
              <button
                disabled={busy || !song.hasScore}
                onClick={() =>
                  void perform(async () => {
                    await api.command({
                      type: "detachScore",
                      content_id: song.contentId,
                    });
                  })
                }
              >
                解除当前谱面关联
              </button>
            </div>
            <input
              hidden
              ref={input}
              type="file"
              aria-label="添加谱面版本文件"
              accept=".musicxml,.xml,.mxl"
              onChange={(e) => {
                const f = e.target.files?.[0];
                e.target.value = "";
                if (f) void previewFile(f);
              }}
            />
            {filePreview && (
              <section
                className="score-source-preview"
                aria-label="待添加谱面预览"
              >
                <h3>{filePreview.name}</h3>
                <ScoreRoutePreview
                  evidence={filePreview.evidence}
                  title="这份谱面的演奏顺序"
                />
                <p className="parameter-help">
                  “添加并使用”会立即切换当前谱面。“仅保存谱面版本”会保留当前谱面和练习；以后可以在版本列表核对并启用。
                </p>
                <div className="dialog-actions">
                  <button
                    className="primary-button"
                    disabled={
                      busy || filePreview.evidence.route?.complete !== true
                    }
                    onClick={() => saveFile("use")}
                  >
                    添加并使用
                  </button>
                  <button disabled={busy} onClick={() => saveFile("save")}>
                    仅保存谱面版本
                  </button>
                  <button disabled={busy} onClick={() => setFilePreview(null)}>
                    取消添加
                  </button>
                </div>
                {filePreview.evidence.route?.complete !== true && (
                  <p className="score-route-warning">
                    这份谱面的演奏顺序尚未确认。保存版本后，当前练习仍使用原谱面。
                  </p>
                )}
              </section>
            )}
            {error && (
              <p role="alert" className="dialog-error">
                {error}
              </p>
            )}
            {editMessage && <p role="status" className="parameter-help">{editMessage}</p>}
            <div className="fingering-table-wrap">
              <table className="fingering-table">
                <thead>
                  <tr>
                    <th>谱面名称</th>
                    <th>状态</th>
                    <th>操作</th>
                  </tr>
                </thead>
                <tbody>
                  {versions.map((v) => {
                    const original = originals.find((entry) => entry.versionId === v.id);
                    const currentPreview =
                      versionPreview?.id === v.id &&
                      versionPreview.path === v.path &&
                      v.available
                        ? versionPreview.evidence
                        : null;
                    return (
                      <Fragment key={v.id}>
                        <tr>
                          <td>
                            <input
                              aria-label={`谱面名称 ${v.name}`}
                              value={names[v.id] ?? v.name}
                              disabled={busy}
                              onChange={(e) =>
                                setNames((old) => ({
                                  ...old,
                                  [v.id]: e.target.value,
                                }))
                              }
                            />
                            <div className="parameter-help" title={v.path}>
                              {v.path}
                            </div>
                          </td>
                          <td>
                            {v.active
                              ? "当前使用"
                              : v.available
                                ? "已保存"
                                : "文件丢失"}
                          </td>
                          <td>
                            <button
                              disabled={busy || !v.available}
                              aria-expanded={!!currentPreview}
                              onClick={() =>
                                currentPreview
                                  ? setVersionPreview(null)
                                  : void previewVersion(v)
                              }
                            >
                              {currentPreview ? "收起演奏顺序" : "预览演奏顺序"}
                            </button>
                            {api.desktop && (
                              <button
                                disabled={busy || !v.available}
                                onClick={() =>
                                  void perform(async () => {
                                    await api.linkScoreSource(
                                      song.sourcePath,
                                      song.contentId,
                                      v.id,
                                    );
                                  })
                                }
                              >
                                关联原谱文件
                              </button>
                            )}
                            {api.desktop && original && (
                              <button
                                disabled={busy || original.status !== "ready"}
                                title={original.status !== "ready" ? "原谱已变化或无法读取，请先检查原谱更新" : original.path}
                                onClick={() => void perform(async () => {
                                  await api.openScoreInMuseScore(song.contentId, v.id);
                                  setEditMessage("MuseScore Studio 已打开原谱。保存修改后，请返回点“检查原谱更新”，核对并采用新内容。");
                                })}
                              >
                                在 MuseScore 中编辑原谱
                              </button>
                            )}
                            <button
                              disabled={
                                busy ||
                                v.active ||
                                !v.available ||
                                !currentPreview
                              }
                              title={
                                !currentPreview
                                  ? "请先预览这个版本的演奏顺序"
                                  : undefined
                              }
                              onClick={() =>
                                void perform(async () => {
                                  let fresh: ScoreRouteEvidence;
                                  try {
                                    fresh =
                                      await api.command<ScoreRouteEvidence>({
                                        type: "previewScoreVersionRoute",
                                        id: v.id,
                                        content_id: song.contentId,
                                      });
                                  } catch (cause) {
                                    setVersionPreview(null);
                                    throw cause;
                                  }
                                  if (
                                    JSON.stringify(fresh) !==
                                    JSON.stringify(currentPreview)
                                  ) {
                                    setVersionPreview(null);
                                    throw new Error(
                                      "谱面内容或演奏顺序已变化。请重新预览后再使用。",
                                    );
                                  }
                                  await api.command({
                                    type: "activateScore",
                                    id: v.id,
                                    content_id: song.contentId,
                                  });
                                  setVersionPreview(null);
                                })
                              }
                            >
                              使用此谱面
                            </button>
                            <button
                              disabled={
                                busy ||
                                !names[v.id]?.trim() ||
                                names[v.id] === v.name
                              }
                              onClick={() =>
                                void perform(async () => {
                                  await api.command({
                                    type: "renameScoreVersion",
                                    id: v.id,
                                    content_id: song.contentId,
                                    name: names[v.id],
                                  });
                                })
                              }
                            >
                              保存名称
                            </button>
                            <button
                              disabled={busy || v.active}
                              onClick={() =>
                                void perform(async () => {
                                  await api.command({
                                    type: "removeScoreVersion",
                                    id: v.id,
                                    content_id: song.contentId,
                                  });
                                })
                              }
                            >
                              移除版本
                            </button>
                          </td>
                        </tr>
                        {currentPreview && (
                          <tr className="score-version-route-row">
                            <td colSpan={3}>
                              <ScoreRoutePreview
                                evidence={currentPreview}
                                title={`「${v.name}」的演奏顺序`}
                              />
                              <p className="parameter-help">
                                选择“使用此谱面”时，软件会再次检查版本和演奏顺序。
                              </p>
                            </td>
                          </tr>
                        )}
                      </Fragment>
                    );
                  })}
                </tbody>
              </table>
              {!versions.length && (
                <p className="parameter-help">
                  尚未保存谱面，添加与这首 MIDI 对应的 MusicXML 或 MXL 文件。
                </p>
              )}
            </div>
          </>
        )}
        <div className="dialog-actions">
          <button disabled={busy || paperBusy} onClick={close}>
            完成
          </button>
        </div>
        {sourcesOpen && (
          <ScoreSourcesDialog
            contentId={song.contentId}
            close={() => setSourcesOpen(false)}
            changed={async () => {
              await refresh();
              await changed();
            }}
          />
        )}
      </section>
    </div>
  );
}
