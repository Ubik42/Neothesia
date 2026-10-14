import { useEffect, useRef, useState } from "react";
import { api, pitchName, type LoadedSong } from "./api";
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
  const [previewPage, setPreviewPage] = useState(0);
  const [previewZoom, setPreviewZoom] = useState(0);
  const [excluded, setExcluded] = useState<Set<string>>(() => new Set());
  const [focused, setFocused] = useState("");
  const previewRef = useRef<HTMLDivElement>(null);
  const identity = (note: LocatedPaperPlan["notes"][number]) => `${note.track}:${note.index}`;
  useEffect(() => {
    try {
      setPlan(null);
      setExcluded(new Set());
      setPreviewPage(0);
      setFocused("");
      setPlan(buildLocatedPaperPlan(pages, mapping, song));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, [pages, mapping, song]);
  const accepted = plan?.notes.filter(note => !excluded.has(identity(note))) ?? [];
  const pageNotes = plan?.notes.filter(note => note.page === previewPage + 1) ?? [];
  const selected = pageNotes.find(note => identity(note) === focused);
  useEffect(() => {
    if (!selected || !previewRef.current) return;
    const viewport = previewRef.current;
    const svg = viewport.querySelector("svg");
    if (!svg) return;
    const viewportRect = viewport.getBoundingClientRect();
    const svgRect = svg.getBoundingClientRect();
    viewport.scrollTo({
      top: viewport.scrollTop + svgRect.top - viewportRect.top + selected.region.y * svgRect.height - viewport.clientHeight / 2,
      left: viewport.scrollLeft + svgRect.left - viewportRect.left + selected.region.x * svgRect.width - viewport.clientWidth / 2,
      behavior: "smooth",
    });
  }, [focused, previewPage, selected]);
  const setPageNotes = (keep: boolean) => {
    setExcluded(current => {
      const next = new Set(current);
      for (const note of pageNotes) keep ? next.delete(identity(note)) : next.add(identity(note));
      return next;
    });
  };
  const save = async () => {
    if (!accepted.length || busy || !name.trim()) return;
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
        anchors: accepted,
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
        <p>软件会用当前已核对的关联乐谱生成本机谱页。请逐页查看音符位置；您可以在保存前跳过需要重新核对的音符。原有 PDF、图片和手工标记会保留。</p>
        <label>谱面名称 <input aria-label="可定位谱面名称" value={name} disabled={busy || saved} onChange={(event) => setName(event.target.value)} /></label>
        {plan && (
          <div role="status" className="located-paper-summary">
            <p>共 {pages.length} 页；可定位 {plan.notes.length} 个演奏音符；本次保留 {accepted.length} 个；跳过 {plan.skipped + excluded.size} 个。</p>
            {plan.reasons.length > 0 && <p>{plan.reasons.join("；")}。</p>}
            {excluded.size > 0 && <p>您手动跳过 {excluded.size} 个位置。这些音符以后仍可在纸谱中手工标记。</p>}
          </div>
        )}
        {pages.length > 0 && <>
          <div className="located-paper-pages">
            <button disabled={busy || previewPage === 0} onClick={() => { setPreviewPage(page => page - 1); setFocused(""); }}>上一页</button>
            <label>预览页码 <input aria-label="预览页码" type="number" min={1} max={pages.length} value={previewPage + 1}
              disabled={busy} onChange={event => { const next = Number(event.target.value); if (Number.isInteger(next) && next >= 1 && next <= pages.length) { setPreviewPage(next - 1); setFocused(""); } }} /> / {pages.length} 页</label>
            <button disabled={busy || previewPage >= pages.length - 1} onClick={() => { setPreviewPage(page => page + 1); setFocused(""); }}>下一页</button>
            <label>预览缩放 <select aria-label="谱页预览缩放" value={previewZoom} disabled={busy} onChange={event => setPreviewZoom(Number(event.target.value))}>
              <option value={0}>适合宽度</option><option value={1100}>原尺寸</option><option value={1650}>放大</option>
            </select></label>
            <span>本页保留 {pageNotes.filter(note => !excluded.has(identity(note))).length} / {pageNotes.length} 个位置</span>
          </div>
          <div className="located-paper-preview" aria-label={`生成谱页预览 · 第 ${previewPage + 1} 页`} ref={previewRef}>
            <div className="located-paper-sheet" style={{ width: previewZoom ? `${previewZoom}px` : "100%" }}>
              <div className="located-paper-svg" dangerouslySetInnerHTML={{ __html: pages[previewPage] }} />
              {selected && <span className={`located-paper-highlight ${excluded.has(identity(selected)) ? "excluded" : ""}`}
                style={{ left: `${selected.region.x * 100}%`, top: `${selected.region.y * 100}%`, width: `${selected.region.width * 100}%`, height: `${selected.region.height * 100}%` }} />}
            </div>
          </div>
          {plan && <div className="located-paper-notes">
            <div className="located-paper-notes-heading">
              <strong>本页音符位置</strong>
              <button disabled={busy || saved || !pageNotes.length} onClick={() => setPageNotes(true)}>保留本页</button>
              <button disabled={busy || saved || !pageNotes.length} onClick={() => setPageNotes(false)}>跳过本页</button>
            </div>
            <div className="located-paper-note-list">
              {pageNotes.map(note => <div className="located-paper-note-row" key={identity(note)}>
                <input type="checkbox" aria-label={`保留音轨 ${note.track} 音符 ${note.index} 的位置`}
                  checked={!excluded.has(identity(note))} disabled={busy || saved}
                  onChange={event => { const keep = event.target.checked; setExcluded(current => { const next = new Set(current); keep ? next.delete(identity(note)) : next.add(identity(note)); return next; }); }} />
                <button className={focused === identity(note) ? "selected" : ""} onClick={() => setFocused(identity(note))}
                  aria-label={`查看第 ${note.measure} 小节 ${pitchName(note.pitch)} 的位置`}>第 {note.measure} 小节 · {pitchName(note.pitch)} · {song.tracks.find(track => track.id === note.track)?.name ?? `音轨 ${note.track}`}</button>
              </div>)}
              {!pageNotes.length && <p>这一页没有可靠的逐音位置。谱页仍会保存。</p>}
            </div>
          </div>}
        </>}
        {progress && <p role="status">{progress}</p>}
        {error && <p className="dialog-error" role="alert">{error}</p>}
        <footer className="dialog-actions">
          <button disabled={busy} onClick={close}>{saved ? "完成" : "取消"}</button>
          {!saved && <button disabled={busy || !accepted.length || !name.trim()} onClick={() => void save()}>采用并保存</button>}
        </footer>
      </section>
    </div>
  );
}
