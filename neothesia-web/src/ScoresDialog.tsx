import { ScoreSourcesDialog } from "./ScoreSourcesDialog";
import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { api, type LoadedSong } from "./api";
import { PaperScore } from "./PaperScore";
type Version = {
  id: string;
  name: string;
  path: string;
  active: boolean;
  available: boolean;
};
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
  const [tab, setTab] = useState("notation"),
    [paperBusy, setPaperBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const [sourcesOpen, setSourcesOpen] = useState(false);
  const refresh = async () => {
    const data = await api.command<{ versions: Version[] }>({
      type: "scoreVersions",
    });
    setVersions(data.versions);
  };
  useEffect(() => {
    void refresh().catch((e) => setError(String(e.message)));
  }, [song.contentId]);
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
                if (f)
                  void perform(async () => {
                    if (f.size > 4_000_000) throw new Error("乐谱超过 4 MB");
                    await api.command({
                      type: "pairScore",
                      bytes: Array.from(new Uint8Array(await f.arrayBuffer())),
                      name: f.name,
                      content_id: song.contentId,
                    });
                  });
              }}
            />
            {error && (
              <p role="alert" className="dialog-error">
                {error}
              </p>
            )}
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
                  {versions.map((v) => (
                    <tr key={v.id}>
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
                        <button
                          disabled={busy || v.active || !v.available}
                          onClick={() =>
                            void perform(async () => {
                              await api.command({
                                type: "activateScore",
                                id: v.id,
                                content_id: song.contentId,
                              });
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
                  ))}
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
