import { useEffect, useRef, useState } from "react";
import { X, Download } from "lucide-react";
import { api, type LoadedSong } from "./api";

type Options = {
  fingers: boolean;
  hands: boolean;
  substitutions: boolean;
  repeatPolicy: "consistent" | "first";
};
type Preview = {
  name: string;
  fingerprint: string;
  fingerNotes: number;
  handNotes: number;
  substitutionNotes: number;
  substitutionActions: number;
  invalidActions: number;
  ppq: number;
  writtenNotes: number;
  unmappedNotes: number;
  conflicts: {
    part: string;
    measure: number;
    note: number;
    fingers: boolean;
    hands: boolean;
    substitutions: boolean;
    positions: {
      track: number;
      index: number;
      seconds: number;
      finger: number | null;
      hand: string | null;
      changes: { offsetTick: number; from: number; to: number }[] | null;
    }[];
  }[];
  bytes?: number[];
};
export function ScoreExportDialog({
  song,
  close,
}: {
  song: LoadedSong;
  close: () => void;
}) {
  const [options, setOptions] = useState<Options>({
    fingers: true,
    hands: true,
    substitutions: true,
    repeatPolicy: "consistent",
  });
  const [preview, setPreview] = useState<Preview | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [message, setMessage] = useState(""),
    [revision, setRevision] = useState(0);
  const alive = useRef(true),
    saving = useRef(false);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  const command = {
    type: "exportAnnotatedScore",
    content_id: song.contentId,
    score_revision: song.scoreRevision,
    options,
  };
  useEffect(() => {
    let current = true;
    setPreview(null);
    setBusy(true);
    setError("");
    setMessage("");
    api
      .command<Preview>({
        type: "exportAnnotatedScore",
        content_id: song.contentId,
        score_revision: song.scoreRevision,
        options,
      })
      .then((data) => {
        if (current) setPreview(data);
      })
      .catch((e) => {
        if (current) setError(String(e));
      })
      .finally(() => {
        if (current) setBusy(false);
      });
    return () => {
      current = false;
    };
  }, [
    song.contentId,
    song.scoreRevision,
    song.fingerActionRevision,
    options,
    revision,
  ]);
  const save = async () => {
    if (!preview || busy || saving.current) return;
    saving.current = true;
    setBusy(true);
    setError("");
    try {
      const request = {
        ...command,
        download: true,
        fingerprint: preview.fingerprint,
      };
      if (api.desktop) {
        const path = await api.exportAnnotatedScore(request);
        if (alive.current) setMessage(path ? `已保存：${path}` : "已取消保存");
      } else {
        const result = await api.command<Preview>(request);
        if (!result.bytes)
          throw new Error("导出文件没有生成，请刷新预览后重试");
        const blob = new Blob([new Uint8Array(result.bytes)], {
          type: "application/vnd.recordare.musicxml+xml",
        });
        const url = URL.createObjectURL(blob),
          link = document.createElement("a");
        link.href = url;
        link.download = result.name;
        link.click();
        setTimeout(() => URL.revokeObjectURL(url), 30000);
        if (alive.current) setMessage("已生成个人标记谱，请查看下载文件");
      }
    } catch (e) {
      if (alive.current) {
        setError(String(e));
        setPreview(null);
      }
    } finally {
      saving.current = false;
      if (alive.current) setBusy(false);
    }
  };
  return (
    <div className="modal-backdrop">
      <section
        className="settings-dialog score-export-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="score-export-title"
      >
        <header className="dialog-heading">
          <h2 id="score-export-title">导出个人标记谱</h2>
          <button aria-label="关闭导出谱" onClick={close} disabled={busy}>
            <X size={18} />
          </button>
        </header>
        <p>
          生成当前谱面版本的 MusicXML
          副本，保留原谱的小节、反复、歌词与演奏记号。已保存的个人指法替换对应谱音的原指法。
        </p>
        <div className="score-export-options">
          <label>
            <input
              type="checkbox"
              checked={options.fingers}
              disabled={busy}
              onChange={(e) =>
                setOptions({ ...options, fingers: e.target.checked })
              }
            />
            个人指法
          </label>
          <label>
            <input
              type="checkbox"
              checked={options.hands}
              disabled={busy}
              onChange={(e) =>
                setOptions({ ...options, hands: e.target.checked })
              }
            />
            逐音分手标记
          </label>
          <label>
            <input
              type="checkbox"
              checked={options.substitutions}
              disabled={busy}
              onChange={(e) =>
                setOptions({ ...options, substitutions: e.target.checked })
              }
            />
            持音换指标记
          </label>
          <label>
            反复段落的不同标记
            <select
              aria-label="反复段落导出方式"
              value={options.repeatPolicy}
              disabled={busy}
              onChange={(e) =>
                setOptions({
                  ...options,
                  repeatPolicy: e.target.value as Options["repeatPolicy"],
                })
              }
            >
              <option value="consistent">保留原谱，跳过有分歧的标记</option>
              <option value="first">采用首次演奏的标记</option>
            </select>
          </label>
        </div>
        <p className="muted">
          分手以“左手 /
          右手”文字标注；原谱声部和谱表布局保持原样。尚未接受的推荐、纸谱批注不会写入此文件。持音换指同时写出对应起音指、接替指和起音后经过的四分音符拍数，按保存时的推荐速度校验动作；具体排版由制谱软件决定。
        </p>
        {busy && <p role="status">正在准备导出…</p>}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        {preview && (
          <>
            <dl className="score-export-summary">
              <div>
                <dt>指法谱音</dt>
                <dd>{preview.fingerNotes}</dd>
              </div>
              <div>
                <dt>分手谱音</dt>
                <dd>{preview.handNotes}</dd>
              </div>
              <div>
                <dt>持音换指</dt>
                <dd>
                  {preview.substitutionActions} 个动作 /{" "}
                  {preview.substitutionNotes} 个谱音
                </dd>
              </div>
              <div>
                <dt>反复分歧</dt>
                <dd>{preview.conflicts.length}</dd>
              </div>
              <div>
                <dt>未能可靠对应的个人标记</dt>
                <dd>{preview.unmappedNotes}</dd>
              </div>
            </dl>
            {preview.invalidActions > 0 && (
              <p role="status">
                {preview.invalidActions}{" "}
                个已保存动作需要重新审阅，本次跳过其所在音符的换指标记；起音指法和分手仍按所选方式处理。
              </p>
            )}
            {preview.writtenNotes === 0 && (
              <p>
                当前没有可写入的个人标记。保存时仍会生成原谱的 MusicXML 副本。
              </p>
            )}
            {preview.unmappedNotes > 0 && (
              <p>未能可靠对应的标记保留在练习记录中，本次导出跳过这些音符。</p>
            )}
            {preview.conflicts.length > 0 && (
              <details className="score-export-conflicts" open>
                <summary>查看反复分歧</summary>
                <p>
                  {options.repeatPolicy === "consistent"
                    ? "只跳过存在分歧的指法、分手或换指；同一谱音的其他一致标记仍会导出。"
                    : "同一谱音只保留首次演奏的标记，后续演奏的不同设置留在软件中。"}
                </p>
                {preview.conflicts.slice(0, 50).map((c) => (
                  <div key={`${c.part}:${c.measure}:${c.note}`}>
                    <strong>
                      {c.part} · 第 {c.measure} 小节 · 第 {c.note} 谱音
                    </strong>
                    <span>
                      {c.positions
                        .map(
                          (p, i) =>
                            `第 ${i + 1} 次：${[c.fingers ? `指法 ${p.finger ?? "沿用原谱"}` : null, c.hands ? (p.hand ?? "沿用原谱") : null, c.substitutions ? (p.changes === null ? "换指需重新审阅" : p.changes.length === 0 ? "无个人换指" : p.changes.map((a) => `${a.from}→${a.to}（起音后 ${(a.offsetTick / preview.ppq).toFixed(3)} 四分音符拍）`).join("、")) : null].filter(Boolean).join("，")}`,
                        )
                        .join("；")}
                    </span>
                  </div>
                ))}
                {preview.conflicts.length > 50 && (
                  <p>
                    另有 {preview.conflicts.length - 50}{" "}
                    个谱音分歧，按同一方式处理。
                  </p>
                )}
              </details>
            )}
          </>
        )}
        {message && <p role="status">{message}</p>}
        <footer>
          <button disabled={busy} onClick={() => setRevision((n) => n + 1)}>
            刷新预览
          </button>
          <button
            className="primary"
            disabled={busy || !preview}
            onClick={save}
          >
            <Download size={16} />
            保存 MusicXML
          </button>
        </footer>
      </section>
    </div>
  );
}
