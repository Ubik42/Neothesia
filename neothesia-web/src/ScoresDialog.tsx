import { ScoreSourcesDialog } from "./ScoreSourcesDialog";
import { Fragment, useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { api, type LoadedSong } from "./api";
import { PaperScore, type PaperAttachment } from "./PaperScore";
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

async function webpForRecognition(bytes: Uint8Array, pageNumber: number, losslessBudget: number): Promise<{ bytes: Uint8Array; kind: "png" | "jpeg" }> {
  const url = URL.createObjectURL(new Blob([bytes.slice().buffer], { type: "image/webp" }));
  try {
    const source = new Image();
    source.src = url;
    await source.decode();
    if (!source.naturalWidth || !source.naturalHeight ||
      source.naturalWidth > 16_384 || source.naturalHeight > 16_384 ||
      source.naturalWidth * source.naturalHeight > 40_000_000)
      throw new Error(`第 ${pageNumber} 页图片尺寸过大。请缩小图片后重新导入。`);
    const canvas = document.createElement("canvas");
    canvas.width = source.naturalWidth;
    canvas.height = source.naturalHeight;
    const context = canvas.getContext("2d");
    if (!context) throw new Error(`第 ${pageNumber} 页无法转换 WebP 图片。`);
    context.fillStyle = "#fff";
    context.fillRect(0, 0, canvas.width, canvas.height);
    context.drawImage(source, 0, 0);
    const encode = (kind: "image/png" | "image/jpeg") =>
      new Promise<Blob>((resolve, reject) =>
        canvas.toBlob((result) => result ? resolve(result) : reject(new Error(`第 ${pageNumber} 页无法转换 WebP 图片。`)), kind, kind === "image/jpeg" ? 0.98 : undefined));
    if (losslessBudget > 0) {
      const png = await encode("image/png");
      if (png.type === "image/png" && png.size <= Math.min(8_000_000, losslessBudget))
        return { bytes: new Uint8Array(await png.arrayBuffer()), kind: "png" };
    }
    const jpeg = await encode("image/jpeg");
    if (jpeg.type !== "image/jpeg") throw new Error(`第 ${pageNumber} 页无法转换为识谱图片。`);
    return { bytes: new Uint8Array(await jpeg.arrayBuffer()), kind: "jpeg" };
  } finally {
    URL.revokeObjectURL(url);
  }
}

async function recognitionPdf(contentId: string, attachment: PaperAttachment, report: (message: string) => void): Promise<number[] | undefined> {
  if (attachment.format !== "images" || (attachment.pages.length === 1 && attachment.pages[0].kind !== "webp")) return undefined;
  const total = attachment.pages.reduce((size, page) => size + page.size, 0);
  const inputLimit = attachment.pages.length === 1 ? 48_000_000 : 32_000_000;
  if (total > inputLimit)
    throw new Error(`这份图片谱超过 ${inputLimit / 1_000_000} MB。请分成较小的谱本再识别。`);
  const { PDFDocument } = await import("pdf-lib");
  const document = await PDFDocument.create();
  document.setTitle(attachment.name);
  let embeddedImageBytes = 0;
  for (const [index, page] of attachment.pages.entries()) {
    report(`正在整理第 ${index + 1} / ${attachment.pages.length} 页图片…`);
    const bytes = await api.readScoreAttachment(contentId, attachment.id, index, page.kind);
    const converted = page.kind === "webp"
      ? await webpForRecognition(bytes, index + 1, Math.max(0, 28_000_000 - embeddedImageBytes))
      : { bytes, kind: page.kind };
    embeddedImageBytes += converted.bytes.length;
    if (embeddedImageBytes > 32_000_000)
      throw new Error("整理后的图片超过 32 MB。请分成较小的谱本再识别。");
    const image = converted.kind === "png"
      ? await document.embedPng(converted.bytes)
      : converted.kind === "jpeg"
        ? await document.embedJpg(converted.bytes)
        : null;
    if (!image) throw new Error(`第 ${index + 1} 页不是 PNG、JPEG 或 WebP 图片。`);
    const width = image.width * 72 / 300;
    const height = image.height * 72 / 300;
    if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0 || width > 14_400 || height > 14_400)
      throw new Error(`第 ${index + 1} 页的图片尺寸无法用于识谱 PDF。`);
    const sheet = document.addPage([width, height]);
    sheet.drawImage(image, { x: 0, y: 0, width, height });
  }
  const pdf = await document.save();
  if (pdf.length > 32_000_000)
    throw new Error("图片组页生成的 PDF 超过 32 MB。请分成较小的谱本再识别。");
  return Array.from(pdf);
}

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
  const currentContentId = useRef(song.contentId);
  currentContentId.current = song.contentId;
  const [sourcesOpen, setSourcesOpen] = useState(false);
  const [sourceFocusVersionId, setSourceFocusVersionId] = useState<string>();
  const canGenerateMuseScorePdf = api.desktop && /Windows/i.test(navigator.userAgent);
  const canGenerateMuseScoreScore = canGenerateMuseScorePdf && /\.midi?$/i.test(song.sourcePath);
  const [versionPreview, setVersionPreview] = useState<{
    id: string;
    path: string;
    evidence: ScoreRouteEvidence;
  } | null>(null);
  const [filePreview, setFilePreview] = useState<{
    name: string;
    bytes: number[];
    evidence: ScoreRouteEvidence;
    generated?: boolean;
    recognizedFrom?: string;
    previewToken?: string;
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
    setSourceFocusVersionId(undefined);
    void refresh().catch((e) => setError(String(e.message)));
  }, [song.contentId]);
  useEffect(() => {
    if (!api.desktop) return;
    const update = () => {
      if (document.visibilityState === "visible")
        void refresh().catch((e) => setError(String(e.message)));
    };
    window.addEventListener("focus", update);
    document.addEventListener("visibilitychange", update);
    return () => {
      window.removeEventListener("focus", update);
      document.removeEventListener("visibilitychange", update);
    };
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
  const generateScore = async () => {
    setBusy(true);
    setError("");
    setFilePreview(null);
    try {
      const draft = await api.generateMuseScoreScore(song.contentId);
      const evidence = await api.command<ScoreRouteEvidence>({
        type: "previewScoreBytesRoute",
        bytes: draft.bytes,
        name: draft.name,
        content_id: song.contentId,
      });
      setFilePreview({ ...draft, evidence, generated: true });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  };
  const recognizePaper = async (attachment: PaperAttachment, report: (message: string) => void) => {
    const contentId = song.contentId;
    setBusy(true);
    setError("");
    setFilePreview(null);
    try {
      const preparedPdf = await recognitionPdf(contentId, attachment, report);
      if (currentContentId.current !== contentId)
        throw new Error("当前曲目已经切换。请重新选择纸谱。");
      report("Audiveris 正在识别纸谱…");
      const draft = await api.recognizePaperWithAudiveris(
        contentId,
        attachment.id,
        attachment.pages.map((page) => ({ id: page.id, kind: page.kind })),
        preparedPdf,
      );
      if (currentContentId.current !== contentId)
        throw new Error("当前曲目已经切换。请重新选择纸谱。");
      if (draft.bytes.length > 4_000_000)
        throw new Error("识别结果超过 4 MB，无法加入谱面版本。请在 MuseScore 中分段整理后导入。");
      const evidence = await api.command<ScoreRouteEvidence>({
        type: "previewScoreBytesRoute",
        bytes: draft.bytes,
        name: draft.name,
        content_id: contentId,
      });
      if (currentContentId.current !== contentId)
        throw new Error("当前曲目已经切换。请重新选择纸谱。");
      setFilePreview({ ...draft, evidence, recognizedFrom: attachment.name });
      setTab("notation");
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
  const saveDraftForEdit = async () => {
    if (!filePreview?.previewToken || (!filePreview.generated && !filePreview.recognizedFrom)) return;
    const recognized = !!filePreview.recognizedFrom;
    setBusy(true);
    setError("");
    setEditMessage("");
    try {
      const fresh = await api.command<ScoreRouteEvidence>({
        type: "previewScoreBytesRoute",
        bytes: filePreview.bytes,
        name: filePreview.name,
        content_id: song.contentId,
      });
      if (
        !fresh.fingerprint ||
        fresh.fingerprint !== filePreview.evidence.fingerprint ||
        JSON.stringify(fresh.route) !== JSON.stringify(filePreview.evidence.route)
      ) {
        setFilePreview(null);
        throw new Error("谱面初稿或演奏顺序已变化。请重新识别或生成，并核对预览。");
      }
      const saved = await (recognized ? api.saveRecognizedScoreForEdit : api.saveGeneratedScoreForEdit)(
        song.contentId,
        filePreview.name,
        filePreview.bytes,
        filePreview.previewToken,
      );
      if (!saved) return;
      setFilePreview(null);
      setVersionPreview(null);
      setEditMessage(saved.opened
        ? "可编辑原谱已保存并在 MuseScore Studio 中打开。修改并保存后，请返回这里检查原谱更新。"
        : `可编辑原谱已保存：${saved.path}。${saved.warning ?? "请手动在 MuseScore Studio 中打开该文件。"} 修改并保存后，请返回这里检查原谱更新。`);
      setSourceFocusVersionId(saved.versionId);
      setSourcesOpen(saved.opened);
      await refresh();
      await changed();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  };
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
          <PaperScore contentId={song.contentId} manage onBusy={setPaperBusy} onRecognize={api.desktop ? recognizePaper : undefined} />
        ) : (
          <>
            <p className="parameter-help">
              同一首曲目可以保留不同版本的 MusicXML / MXL
              谱面。切换时重新核对音符对应关系；移除版本会从管理列表移除，文件仍保留。
            </p>
            {api.desktop && <p className="parameter-help">
              MuseScore Studio 可以把当前 MIDI 转成可编辑的 MusicXML 初稿。生成后先核对音符、节奏与演奏顺序，再决定保存或使用。已有原谱可以关联磁盘文件，在 MuseScore 中编辑并检查更新；原谱也可以排版成 PDF 谱页。
            </p>}
            <div className="dialog-actions">
              <button
                className="primary-button"
                disabled={busy}
                onClick={() => input.current?.click()}
              >
                添加谱面版本
              </button>
              {canGenerateMuseScoreScore && (
                <button disabled={busy} onClick={() => void generateScore()}>
                  从 MIDI 生成谱面初稿
                </button>
              )}
              <span>{versions.length} 份谱面</span>
              <button disabled={busy} onClick={() => {
                setSourceFocusVersionId(undefined);
                setSourcesOpen(true);
              }}>
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
                {filePreview.generated && (
                  <p className="parameter-help">
                    这是 MuseScore Studio 根据当前 MIDI 推出的谱面初稿。请检查谱号、左右手、时值和小节；MIDI 通常没有完整的记谱信息。
                  </p>
                )}
                {filePreview.recognizedFrom && (
                  <p className="parameter-help">
                    这份乐谱由 Audiveris 从“{filePreview.recognizedFrom}”识别。请逐小节核对音高、时值、拍号和反复记号；原纸谱仍保留在 PDF / 图片谱面中。
                  </p>
                )}
                <ScoreRoutePreview
                  evidence={filePreview.evidence}
                  title="这份谱面的演奏顺序"
                />
                <p className="parameter-help">
                  “添加并使用”会立即切换当前谱面。“仅保存谱面版本”会保留当前谱面和练习；以后可以在版本列表核对并启用。
                  {(filePreview.generated || filePreview.recognizedFrom) && "“保存可编辑原谱”会让你选一个新文件，随后在 MuseScore Studio 中打开。"}
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
                  {(filePreview.generated || filePreview.recognizedFrom) && filePreview.previewToken && (
                    <button disabled={busy} onClick={() => void saveDraftForEdit()}>
                      保存可编辑原谱并打开 MuseScore
                    </button>
                  )}
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
                            {canGenerateMuseScorePdf && original && (
                              <button
                                disabled={busy || original.status !== "ready"}
                                title={original.status !== "ready" ? "请先检查原谱更新" : "用 MuseScore Studio 排版 PDF，并作为新谱页版本加入当前曲目"}
                                onClick={() => void perform(async () => {
                                  await api.addMuseScorePdf(song.contentId, v.id);
                                  setEditMessage("PDF 已加入当前曲目。请在“PDF / 图片谱面”中核对页码和排版。");
                                  setTab("paper");
                                })}
                              >
                                生成 PDF 谱面
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
            focusVersionId={sourceFocusVersionId}
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
