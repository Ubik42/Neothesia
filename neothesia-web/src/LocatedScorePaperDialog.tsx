import { useEffect, useState } from "react";
import { api, type LoadedSong } from "./api";
import type { ScoreMapping } from "./ScoreNoteEditor";
import { buildLocatedPaperPlan, svgPagePng, type LocatedPaperPlan } from "./locatedScorePaper";
import "./located-score-paper.css";

export function LocatedScorePaperDialog({
  song, pages, mapping, sourceFingerprint, mappingToken, close,
}: {
  song: LoadedSong;
  pages: string[];
  mapping: ScoreMapping[];
  sourceFingerprint: string;
  mappingToken: string;
  close: () => void;
}) {
  const [plan, setPlan] = useState<LocatedPaperPlan | null>(null);
  const [name, setName] = useState(`${song.title} · 可定位谱`);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState("");
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  useEffect(() => {
    try {
      setPlan(buildLocatedPaperPlan(pages, mapping, song));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, [pages, mapping, song]);
  const save = async () => {
    if (!plan?.notes.length || busy || !name.trim()) return;
    setBusy(true);
    setError("");
    try {
      const context = await api.command<{ contentId: string | null; grid: string }>({ type: "paperPracticeContext" });
      if (context.contentId !== song.contentId || !context.grid)
        throw new Error("当前曲目已改变，请重新打开关联乐谱");
      const current = await api.command<{ sourceFingerprint: string; readiness: string } | null>({ type: "score" });
      if (current?.sourceFingerprint !== sourceFingerprint || current.readiness !== "Ready")
        throw new Error("关联乐谱已改变，请重新打开后核对定位");
      const rendered: Blob[] = [];
      let payloadBytes = 0;
      for (let i = 0; i < pages.length; i++) {
        setProgress(`正在生成第 ${i + 1} / ${pages.length} 页…`);
        const png = await svgPagePng(pages[i]);
        payloadBytes += png.size;
        if (payloadBytes > 48_000_000)
          throw new Error("这份谱的图片数据超过 48 MB，请降低谱页分辨率或分段生成。");
        rendered.push(png);
      }
      setProgress("正在保存谱页和逐音位置…");
      await api.createLocatedScorePaper({
        content_id: song.contentId,
        score_revision: song.scoreRevision,
        source_fingerprint: sourceFingerprint,
        mapping_token: mappingToken,
        name: name.trim(),
        anchors: plan.notes,
      }, rendered);
      setSaved(true);
      setProgress("已保存。纸谱管理中可以查看这份谱，历史问题可定位到已核对的音符。");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setProgress("");
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="modal-backdrop">
      <section className="settings-dialog located-paper-dialog" role="dialog" aria-modal="true" aria-labelledby="located-paper-title">
        <header className="dialog-heading">
          <h2 id="located-paper-title">生成可定位纸谱</h2>
          <button aria-label="关闭可定位纸谱" disabled={busy} onClick={close}>关闭</button>
        </header>
        <p>软件会使用当前已核对的关联乐谱生成本机谱页。逐音位置只采用唯一对应的演奏音符。原有 PDF、图片和手工标记会保留。</p>
        <label>谱面名称 <input aria-label="可定位谱面名称" value={name} disabled={busy || saved} onChange={(event) => setName(event.target.value)} /></label>
        {plan && (
          <div role="status" className="located-paper-summary">
            <p>共 {pages.length} 页；可定位 {plan.notes.length} 个演奏音符；跳过 {plan.skipped} 个。</p>
            {plan.reasons.length > 0 && <p>{plan.reasons.join("；")}。</p>}
          </div>
        )}
        {pages[0] && <div className="located-paper-preview" aria-label="生成谱页预览" dangerouslySetInnerHTML={{ __html: pages[0] }} />}
        {progress && <p role="status">{progress}</p>}
        {error && <p className="dialog-error" role="alert">{error}</p>}
        <footer className="dialog-actions">
          <button disabled={busy} onClick={close}>{saved ? "完成" : "取消"}</button>
          {!saved && <button disabled={busy || !plan?.notes.length || !name.trim()} onClick={() => void save()}>采用并保存</button>}
        </footer>
      </section>
    </div>
  );
}
