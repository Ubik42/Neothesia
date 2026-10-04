import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { annotatedPagePng } from "./PaperPageExport";
import { PaperPracticeBuilder } from "./PaperPracticeBuilder";
import { PaperSources } from "./PaperSources";
import { PaperTransferReview } from "./PaperTransferReview";
import type {
  PDFDocumentProxy,
  PDFDocumentLoadingTask,
  RenderTask,
} from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import {
  PaperAnnotations,
  paperAnnotationBox,
  type PaperRegion,
  type PaperAnnotation,
  type PaperAnnotationDraft,
} from "./PaperAnnotations";
export interface PaperView {
  page: number;
  zoom: number;
  fit: boolean;
  rotation: number;
}
export interface PaperAttachment {
  id: string;
  name: string;
  format: "pdf" | "images";
  pages: {
    id: string;
    name: string;
    kind: string;
    size: number;
    available: boolean;
    status?: string;
  }[];
  view: PaperView;
  mapping?: PaperMapping | null;
  annotations?: PaperAnnotation[];
  sources?: Record<string,{path:string;ignored:string|null}>;
}
interface PageAnchor {
  measure: number;
  page: number;
  region?: PaperRegion;
}
interface PaperMapping {
  grid: string;
  enabled: boolean;
  anchors: PageAnchor[];
}
interface PracticeContext {
  contentId: string | null;
  grid: string;
  measures: number;
  measure: number;
}
interface Papers {
  contentId: string;
  active: string | null;
  attachments: PaperAttachment[];
}
export function PaperScore({
  contentId,
  manage = false,
  revision = 0,
  onBusy,
  libraryPath,
  previewBook,
  onViewSaved,
  playback,
  contextKey,
}: {
  contentId: string;
  libraryPath?: string;
  previewBook?: string;
  onViewSaved?: () => void;
  manage?: boolean;
  revision?: number;
  onBusy?: (busy: boolean) => void;
  playback?: { measure: number };
  contextKey?: string;
}) {
  const [sourceBusy,setSourceBusy]=useState(false);
  const [transferBusy,setTransferBusy]=useState(false);
  const [practice, setPractice] = useState<PracticeContext | null>(null);
  const [data, setData] = useState<Papers | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [results, setResults] = useState<string[]>([]),
    [names, setNames] = useState<Record<string, string>>({});
  const input = useRef<HTMLInputElement>(null),
    append = useRef<string | undefined>(undefined),
    alive = useRef(true),
    working = useRef(false),
    currentContent = useRef(contentId);
  currentContent.current = contentId;
  useEffect(() => {
    onBusy?.(busy || sourceBusy || transferBusy);
    return () => onBusy?.(false);
  }, [busy, sourceBusy, transferBusy, onBusy]);
  const current =
    data?.attachments.find((a) => a.id === data.active) ?? data?.attachments[0];
  const refresh = async () => {
    const v = libraryPath
      ? (
          await api.command<{ papers: Papers }>({
            type: "libraryScores",
            content_id: contentId,
            path: libraryPath,
          })
        ).papers
      : await api.command<Papers>({
          type: "scoreAttachments",
          content_id: contentId,
        });
    if (previewBook) v.active = previewBook;
    if (alive.current && currentContent.current === contentId) setData(v);
  };
  useEffect(() => {
    let contextAlive = true;
    alive.current = true;
    setData(null);
    setError("");
    setPractice(null);
    void api
      .command<PracticeContext>({ type: "paperPracticeContext" })
      .then((v) => {
        if (
          contextAlive &&
          alive.current &&
          currentContent.current === contentId &&
          v.contentId === contentId
        )
          setPractice(v);
      })
      .catch(() => {});
    void refresh().catch((e) => alive.current && setError(String(e.message)));
    return () => {
      contextAlive = false;
      alive.current = false;
    };
  }, [contentId, revision, libraryPath, previewBook, contextKey]);
  const work = async (job: () => Promise<void>) => {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    try {
      await job();
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      working.current = false;
      if (alive.current) setBusy(false);
    }
  };
  const update = async (
    id: string | null,
    action: string,
    extra: Record<string, unknown> = {},
  ) => {
    await api.command({
      type: "updateScoreAttachment",
      content_id: contentId,
      id,
      action,
      ...extra,
    });
  };
  const importFiles = async (files: File[], into?: string) => {
    let target = into;
    const report: string[] = [];
    for (const f of files) {
      try {
        const image = /\.(png|jpe?g|webp)$/i.test(f.name);
        const v = await api.addScoreAttachment(
          contentId,
          f,
          into ? into : image ? target : undefined,
        );
        if (image && !target) target = v.active;
        report.push(`${f.name}：已添加`);
      } catch (e) {
        report.push(`${f.name}：${e instanceof Error ? e.message : String(e)}`);
      }
    }
    if (alive.current) setResults(report);
  };
  const pick = (into?: string) => {
    append.current = into;
    if (api.desktop) {
      void work(async () => {
        const r = await api.importScoreAttachments(contentId, into);
        if (r)
          setResults(
            r.results.map((v) => `${v.name}：${v.ok ? "已添加" : v.error}`),
          );
      });
    } else input.current?.click();
  };
  return (
    <section
      className={`paper-score ${manage ? "paper-manage" : ""}`}
      aria-label={manage ? "PDF 与图片谱面管理" : "谱页浏览"}
    >
      <input
        type="file"
        hidden
        multiple
        ref={input}
        accept=".pdf,.png,.jpg,.jpeg,.webp"
        aria-label="添加 PDF 或图片谱面"
        onChange={(e) => {
          const files = Array.from(e.target.files ?? []);
          e.target.value = "";
          if (files.length) void work(() => importFiles(files, append.current));
        }}
      />
      <div className="paper-selector">
        <label>
          谱面版本{" "}
          <select
            aria-label="PDF 与图片谱面版本"
            disabled={busy || sourceBusy || transferBusy || !!previewBook || !data?.attachments.length}
            value={current?.id ?? ""}
            onChange={(e) => void work(() => update(e.target.value, "select"))}
          >
            <option value="" disabled>
              选择谱面
            </option>
            {data?.attachments.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name} ·{" "}
                {a.format === "pdf" ? "PDF" : `${a.pages.length} 页图片`}
              </option>
            ))}
          </select>
        </label>
        {manage && (
          <>
            <button disabled={busy || sourceBusy || transferBusy} onClick={() => pick()}>
              添加 PDF / 图片
            </button>
            <button
              disabled={busy || sourceBusy || transferBusy || current?.format !== "images"}
              onClick={() => pick(current?.id)}
            >
              追加图片谱页
            </button>
          </>
        )}
      </div>
      {error && (
        <p role="alert" className="dialog-error">
          {error}
        </p>
      )}
      {manage && (
        <>
          <p className="parameter-help">
            谱页副本与曲目一起保存在本机。多张图片按导入顺序组成一份谱面；PDF
            独立保存。可设置小节与谱页对应，练习时自动翻页；反复按实际演奏小节分别设置。
          </p>
          {results.length > 0 && (
            <div className="paper-import-results" role="status">
              {results.map((r, i) => (
                <p key={i}>{r}</p>
              ))}
            </div>
          )}
          {current && (
            <div className="paper-version-actions">
              <input
                aria-label="附件谱面名称"
                value={names[current.id] ?? current.name}
                disabled={busy || sourceBusy || transferBusy}
                onChange={(e) =>
                  setNames({ ...names, [current.id]: e.target.value })
                }
              />
              <button
                disabled={
                  busy ||
                  !names[current.id]?.trim() ||
                  names[current.id] === current.name
                }
                onClick={() =>
                  void work(() =>
                    update(current.id, "rename", { name: names[current.id] }),
                  )
                }
              >
                保存名称
              </button>
              <button
                disabled={busy || sourceBusy || transferBusy}
                onClick={() => void work(() => update(current.id, "remove"))}
              >
                移除登记
              </button>
              <span>保留导入副本和原文件</span>
            </div>
          )}
          {current && <PaperSources key={`source:${contentId}:${current.id}`} contentId={contentId} attachment={current} disabled={busy || sourceBusy || transferBusy} changed={refresh} onBusy={setSourceBusy}/>}
          {current && data && <PaperTransferReview key={`transfer:${contentId}:${current.id}`} contentId={contentId} target={current} books={data.attachments}
            disabled={busy || sourceBusy || transferBusy} changed={refresh} onBusy={setTransferBusy}/>}
        </>
      )}
      {current ? (
        <PaperReader
          key={`${contentId}|${current.id}`}
          contentId={contentId}
          attachment={current}
          practice={practice}
          playback={playback}
          saveMapping={(mapping) =>
            work(async () => {
              await api.command({
                type: "setPaperMapping",
                content_id: contentId,
                id: current.id,
                mapping,
              });
            })
          }
          saveInk={async(assetId,page,expected,strokes)=>{
            let ok=false;await work(async()=>{await api.command({type:"editPaperInk",content_id:contentId,book:current.id,asset_id:assetId,page,expected,strokes});ok=true;});
            if(!ok)await refresh().catch(()=>{});else onViewSaved?.();return ok;
          }}
          saveAnnotation={async (id, draft, expected) => {
            let ok = false;
            await work(async () => {
              await api.command({
                type: "editPaperAnnotation",
                content_id: contentId,
                book: current.id,
                id,
                draft,
                expected,
              });
              ok = true;
            });
            if (!ok) await refresh().catch(() => {});
            else onViewSaved?.();
            return ok;
          }}
          onViewSaved={onViewSaved}
          disabled={busy || sourceBusy || transferBusy}
          manage={manage}
          update={(action, extra) =>
            work(() => update(current.id, action, extra))
          }
        />
      ) : (
        <div className="paper-empty">
          {data
            ? "尚无 PDF 或图片谱面。请在“谱面管理”添加。"
            : "正在读取谱面目录…"}
        </div>
      )}
    </section>
  );
}
export function PaperReader({
  contentId,
  attachment,
  disabled,
  manage,
  update,
  onViewSaved,
  practice,
  playback,
  saveMapping,
  saveAnnotation,
  saveInk,
  previewKind,
  readOnly=false,
  onPageCount,
}: {
  contentId: string;
  previewKind?: string;
  readOnly?: boolean;
  onPageCount?: (count:number)=>void;
  attachment: PaperAttachment;
  disabled: boolean;
  manage: boolean;
  update: (action: string, extra: Record<string, unknown>) => Promise<void>;
  onViewSaved?: () => void;
  practice: PracticeContext | null;
  playback?: { measure: number };
  saveMapping: (mapping: PaperMapping | null) => Promise<void>;
  saveInk?:(assetId:string,page:number,expected:PaperAnnotation[],strokes:PaperAnnotation[])=>Promise<boolean>;
  saveAnnotation: (
    id: string | null,
    draft: PaperAnnotationDraft | null,
    expected: PaperAnnotation | null,
  ) => Promise<boolean>;
}) {
  const [exportBusy, setExportBusy] = useState(false),
    [exportNotice, setExportNotice] = useState("");
  const exportLock = useRef(false),
    readerAlive = useRef(true);
  useEffect(() => {
    readerAlive.current = true;
    return () => {
      readerAlive.current = false;
    };
  }, []);
  const [annotationEditing, setAnnotationEditing] = useState(false),
    [annotationNotice, setAnnotationNotice] = useState(""),
    [sheetSize, setSheetSize] = useState({ width: 1, height: 1 });
  const [suspended, setSuspended] = useState(false),
    [followPending, setFollowPending] = useState<boolean | null>(null),
    [anchorMeasure, setAnchorMeasure] = useState(practice?.measure || 1);
  const [anchorRegion,setAnchorRegion]=useState<PaperRegion|null>(null),[anchorRegionPage,setAnchorRegionPage]=useState(attachment.view.page),[selectingRegion,setSelectingRegion]=useState(false);
  const [practiceAnchor,setPracticeAnchor]=useState<number|null>(null);
  const followedRegion=useRef("");
  const scrollRegion=(region:PaperRegion)=>{
    const viewport=frame.current, sheet=viewport?.querySelector<HTMLElement>(".paper-annotation-sheet");
    if(!viewport||!sheet)return;
    const r=paperAnnotationBox(region,latest.current.rotation);
    viewport.scrollTo({top:sheet.offsetTop+r.y*sheet.offsetHeight-Math.max(12,(viewport.clientHeight-r.height*sheet.offsetHeight)/2),left:sheet.offsetLeft+r.x*sheet.offsetWidth-Math.max(12,(viewport.clientWidth-r.width*sheet.offsetWidth)/2)});
  };
  const jumpAnchor=async(anchor:PageAnchor)=>{
    try{await api.command({type:"seekPaperMeasure",content_id:contentId,id:attachment.id,measure:anchor.measure});setExportNotice(`已定位第 ${anchor.measure} 小节。`);}catch(e){setExportNotice(String(e));}
  };
  const mapping = attachment.mapping;
  const stale = !!mapping && !!practice && mapping.grid !== practice.grid;
  const measured = playback?.measure ?? practice?.measure ?? 1;
  const [view, setView] = useState<PaperView>(attachment.view),
    [count, setCount] = useState(
      attachment.format === "images" ? attachment.pages.length : 0,
    ),
    [doc, setDoc] = useState<PDFDocumentProxy | null>(null),
    [width, setWidth] = useState(900),
    [error, setError] = useState(""),
    [loading, setLoading] = useState(true),
    [password, setPassword] = useState(""),
    [needsPassword, setNeedsPassword] = useState(false);
  const canvas = useRef<HTMLCanvasElement>(null),
    frame = useRef<HTMLDivElement>(null),
    passwordCallback = useRef<((v: string) => void) | null>(null),
    ready = useRef(false),
    latest = useRef(view);
  useEffect(()=>{if(count>0)onPageCount?.(count);},[count,onPageCount]);
  latest.current = view;
  const invalidPages =
    count > 0 && !!mapping?.anchors.some((a) => a.page > count);
  useEffect(() => {
    if (!annotationEditing) setAnnotationNotice("");
  }, [annotationEditing]);
  useEffect(()=>{
    if(anchorRegion&&mapping?.anchors.some(a=>a.measure===anchorMeasure&&a.page===anchorRegionPage&&a.region&&Math.abs(a.region.x-anchorRegion.x)<1e-6&&Math.abs(a.region.y-anchorRegion.y)<1e-6&&Math.abs(a.region.width-anchorRegion.width)<1e-6&&Math.abs(a.region.height-anchorRegion.height)<1e-6))setAnchorRegion(null);
  },[mapping]);
  const savedCallback = useRef(onViewSaved);
  savedCallback.current = onViewSaved;
  useEffect(() => {
    if (
      !playback ||
      !practice ||
      !mapping?.enabled ||
      stale ||
      suspended ||
      annotationEditing ||
      anchorRegion ||
      selectingRegion ||
      exportBusy ||
      invalidPages ||
      !count
    )
      return;
    try {
      const asset = (
        attachment.format === "pdf"
          ? attachment.pages[0]
          : attachment.pages[view.page - 1]
      )?.id;
      if (
        localStorage.getItem(
          `neothesia-paper-draft:${contentId}:${attachment.id}:${asset}:${attachment.format === "pdf" ? view.page : 1}`,
        )
      )
        return;
    } catch {}
    const anchor = mapping.anchors
      .slice()
      .reverse()
      .find((a) => a.measure <= measured);
    if (!anchor || anchor.page > count) return;
    if(latest.current.page!==anchor.page){
      followedRegion.current="";
      setView((v)=>(v.page===anchor.page?v:{...v,page:anchor.page}));
      frame.current?.scrollTo({top:0,left:0});
      return;
    }
    if(loading)return;
    const key=JSON.stringify([anchor,view.rotation,sheetSize]);
    if(followedRegion.current===key)return;
    followedRegion.current=key;
    if(anchor.region)scrollRegion(anchor.region);
  }, [
    measured,
    mapping,
    stale,
    suspended,
    annotationEditing,
    anchorRegion,selectingRegion,
    exportBusy,
    count,
    !!playback,
    practice?.grid,
    loading,view.page,view.rotation,sheetSize,
  ]);
  useEffect(
    () => () => {
      if (ready.current && !readOnly && !previewKind)
        void api
          .command({
            type: "updateScoreAttachment",
            content_id: contentId,
            id: attachment.id,
            action: "view",
            view: latest.current,
          })
          .then(() => savedCallback.current?.())
          .catch(() => {});
    },
    [contentId, attachment.id, readOnly, previewKind],
  );
  useEffect(() => {
    const el = frame.current;
    if (!el) return;
    const resize = new ResizeObserver(([entry]) =>
      setWidth(Math.max(200, entry.contentRect.width - 32)),
    );
    resize.observe(el);
    return () => resize.disconnect();
  }, []);
  useEffect(() => {
    let alive = true;
    let task: PDFDocumentLoadingTask | null = null;
    setError("");
    setLoading(true);
    if (attachment.format === "images") {
      setCount(attachment.pages.length);
      setView((v) => ({
        ...v,
        page: Math.min(v.page, attachment.pages.length),
      }));
      ready.current = true;
      setLoading(false);
      return;
    }
    void api
      .readScoreAttachment(contentId, attachment.id, 0, previewKind)
      .then(async (bytes) => {
        const pdfjs = await import("pdfjs-dist");
        if (!alive) return;
        pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;
        task = pdfjs.getDocument({
          data: bytes,
          enableXfa: false,
          cMapUrl: "/pdfjs/cmaps/",
          cMapPacked: true,
          standardFontDataUrl: "/pdfjs/standard_fonts/",
          wasmUrl: "/pdfjs/wasm/",
          maxImageSize: 64_000_000,
        });
        task.onPassword = (callback: (value: string) => void) => {
          if (alive) {
            passwordCallback.current = callback;
            setNeedsPassword(true);
            setError("该 PDF 需要密码，请输入后打开。");
          }
        };
        return task.promise;
      })
      .then((pdf) => {
        if (!alive || !pdf) return;
        if (pdf.numPages > 5000)
          throw new Error("PDF 超过 5000 页，请拆分后导入");
        setDoc(pdf);
        setCount(pdf.numPages);
        setView((v) => ({ ...v, page: Math.min(v.page, pdf.numPages) }));
        setNeedsPassword(false);
        setError("");
        ready.current = true;
      })
      .catch((e) => {
        if (alive) setError(`PDF 无法打开：${e.message ?? e}`);
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
      passwordCallback.current = null;
      void task?.destroy();
    };
  }, [contentId, attachment.id, attachment.format, attachment.pages.length]);
  useEffect(() => {
    if (!ready.current || readOnly || previewKind) return;
    const timer = setTimeout(() => {
      void api
        .command({
          type: "updateScoreAttachment",
          content_id: contentId,
          id: attachment.id,
          action: "view",
          view,
        })
        .then(() => savedCallback.current?.())
        .catch((e) => setError(`阅读位置未保存：${e.message}`));
    }, 450);
    return () => clearTimeout(timer);
  }, [view, contentId, attachment.id, readOnly, previewKind]);
  useEffect(() => {
    const target = canvas.current;
    if (!target || !count || (attachment.format === "pdf" && !doc)) return;
    let cancelled = false;
    let renderTask: RenderTask | null = null;
    let objectUrl: string | null = null;
    setLoading(true);
    if (!needsPassword) setError("");
    const draw = async () => {
      const ctx = target.getContext("2d");
      if (!ctx) throw new Error("无法创建谱页画布");
      const density = Math.min(window.devicePixelRatio || 1, 2);
      if (doc) {
        const p = await doc.getPage(view.page);
        if (cancelled) return;
        const base = p.getViewport({
          scale: 1,
          rotation: (p.rotate + view.rotation) % 360,
        });
        const scale = view.fit ? width / base.width : view.zoom / 100;
        const viewport = p.getViewport({
          scale,
          rotation: (p.rotate + view.rotation) % 360,
        });
        const d = Math.min(
          density,
          Math.sqrt(16_000_000 / (viewport.width * viewport.height)),
        );
        target.width = Math.max(1, Math.floor(viewport.width * d));
        target.height = Math.max(1, Math.floor(viewport.height * d));
        target.style.width = `${viewport.width}px`;
        target.style.height = `${viewport.height}px`;
        setSheetSize({ width: viewport.width, height: viewport.height });
        renderTask = p.render({
          canvas: target,
          canvasContext: ctx,
          viewport,
          transform: [d, 0, 0, d, 0, 0],
        });
        await renderTask.promise;
      } else {
        const index = view.page - 1;
        const asset = attachment.pages[index];
        if (!asset?.available) throw new Error("该谱页副本缺失，请重新添加");
        const bytes = await api.readScoreAttachment(
          contentId,
          attachment.id,
          index,
          previewKind,
        );
        if (cancelled) return;
        objectUrl = URL.createObjectURL(
          new Blob([bytes.slice().buffer], { type: `image/${asset.kind}` }),
        );
        const img = new Image();
        img.src = objectUrl;
        await img.decode();
        if (cancelled) return;
        if (img.naturalWidth * img.naturalHeight > 64_000_000)
          throw new Error("谱页图片过大，请缩小到 6400 万像素以内");
        const swap = view.rotation % 180 !== 0;
        const w = swap ? img.naturalHeight : img.naturalWidth,
          h = swap ? img.naturalWidth : img.naturalHeight;
        const scale = view.fit ? width / w : view.zoom / 100;
        const d = Math.min(
          density,
          Math.sqrt(16_000_000 / (w * h * scale * scale)),
        );
        target.width = Math.max(1, Math.floor(w * scale * d));
        target.height = Math.max(1, Math.floor(h * scale * d));
        target.style.width = `${w * scale}px`;
        target.style.height = `${h * scale}px`;
        setSheetSize({ width: w * scale, height: h * scale });
        ctx.fillStyle = "#fff";
        ctx.fillRect(0, 0, target.width, target.height);
        ctx.translate(target.width / 2, target.height / 2);
        ctx.rotate((view.rotation * Math.PI) / 180);
        ctx.scale(scale * d, scale * d);
        ctx.drawImage(img, -img.naturalWidth / 2, -img.naturalHeight / 2);
      }
    };
    void draw()
      .catch((e) => {
        if (!cancelled) setError(`谱页显示失败：${e.message ?? e}`);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
      renderTask?.cancel();
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [
    doc,
    count,
    invalidPages,
    view,
    width,
    contentId,
    attachment.id,
    attachment.pages,
    needsPassword,
  ]);
  const page = (n: number) => {
    if (exportLock.current) return;
    if (annotationEditing || selectingRegion) {
      setAnnotationNotice(selectingRegion?"请先完成或取消小节位置框选，再翻页。":"请先保存或取消当前批注，再翻页。");
      return;
    }
    setAnnotationNotice("");
    if (playback && mapping?.enabled) setSuspended(true);
    setView((v) => ({ ...v, page: Math.max(1, Math.min(count, n)) }));
    frame.current?.scrollTo({ top: 0, left: 0 });
  };
  const exportPage = async () => {
    if (
      exportLock.current ||
      annotationEditing ||
      selectingRegion ||
      loading ||
      error ||
      needsPassword ||
      !count
    )
      return;
    exportLock.current = true;
    setExportBusy(true);
    setExportNotice("");
    if (playback && mapping?.enabled) setSuspended(true);
    // Capture page/version/remarks before any asynchronous render or save dialog.
    const pageNumber = view.page,
      rotation = view.rotation,
      name = attachment.name;
    const asset =
      attachment.format === "pdf"
        ? attachment.pages[0]
        : attachment.pages[pageNumber - 1];
    const notes = (attachment.annotations ?? []).filter(
      (n) =>
        n.assetId === asset?.id &&
        n.page === (attachment.format === "pdf" ? pageNumber : 1),
    );
    let objectUrl: string | null = null;
    try {
      const source = document.createElement("canvas"),
        context = source.getContext("2d");
      if (!context || !asset?.available)
        throw new Error("谱页副本不可用，请重新添加或修复");
      if (doc) {
        const pdfPage = await doc.getPage(pageNumber);
        const base = pdfPage.getViewport({
          scale: 1,
          rotation: (pdfPage.rotate + rotation) % 360,
        });
        const scale = Math.min(
          1800 / base.width,
          Math.sqrt(12_000_000 / (base.width * base.height)),
        );
        const viewport = pdfPage.getViewport({
          scale,
          rotation: (pdfPage.rotate + rotation) % 360,
        });
        source.width = Math.max(1, Math.ceil(viewport.width));
        source.height = Math.max(1, Math.ceil(viewport.height));
        await pdfPage.render({
          canvas: source,
          canvasContext: context,
          viewport,
        }).promise;
      } else {
        const bytes = await api.readScoreAttachment(
          contentId,
          attachment.id,
          pageNumber - 1,
          previewKind,
        );
        objectUrl = URL.createObjectURL(
          new Blob([bytes.slice().buffer], { type: `image/${asset.kind}` }),
        );
        const image = new Image();
        image.src = objectUrl;
        await image.decode();
        if (image.naturalWidth * image.naturalHeight > 64_000_000)
          throw new Error("谱页图片过大，请缩小后导出");
        const swap = rotation % 180 !== 0,
          w = swap ? image.naturalHeight : image.naturalWidth,
          h = swap ? image.naturalWidth : image.naturalHeight;
        const scale = Math.min(1800 / w, 1, Math.sqrt(12_000_000 / (w * h)));
        source.width = Math.max(1, Math.ceil(w * scale));
        source.height = Math.max(1, Math.ceil(h * scale));
        context.fillStyle = "#fff";
        context.fillRect(0, 0, source.width, source.height);
        context.translate(source.width / 2, source.height / 2);
        context.rotate((rotation * Math.PI) / 180);
        context.scale(scale, scale);
        context.drawImage(
          image,
          -image.naturalWidth / 2,
          -image.naturalHeight / 2,
        );
      }
      const blob = await annotatedPagePng(
        source,
        notes,
        name,
        pageNumber,
        rotation,
      );
      const fileName = `${name.replace(/\.[^.]+$/, "").replace(/[<>:"/\\|?*\x00-\x1f]/g, "_")}-第${pageNumber}页-批注.png`;
      if (api.desktop) {
        const path = await api.exportPaperPage(
          fileName,
          new Uint8Array(await blob.arrayBuffer()),
        );
        if (readerAlive.current)
          setExportNotice(path ? `已保存：${path}` : "已取消保存");
      } else {
        const url = URL.createObjectURL(blob),
          link = document.createElement("a");
        link.href = url;
        link.download = fileName;
        link.click();
        setTimeout(() => URL.revokeObjectURL(url), 30000);
        if (readerAlive.current)
          setExportNotice("已导出本页图片，包含编号及完整批注说明。");
      }
    } catch (e) {
      if (readerAlive.current) setExportNotice(`导出失败：${String(e)}`);
    } finally {
      if (objectUrl) URL.revokeObjectURL(objectUrl);
      exportLock.current = false;
      if (readerAlive.current) setExportBusy(false);
    }
  };
  return (
    <div
      className="paper-reader"
      onKeyDown={(e) => {
        if (
          (e.target as HTMLElement).matches(
            "input,select,textarea,[contenteditable='true']",
          )
        )
          return;
        const delta =
          e.key === "PageDown" || e.key === "ArrowRight"
            ? 1
            : e.key === "PageUp" || e.key === "ArrowLeft"
              ? -1
              : 0;
        if (delta && count) {
          e.preventDefault();
          e.stopPropagation();
          page(view.page + delta);
        }
      }}
    >
      <fieldset className="paper-controls" disabled={exportBusy}>
        <button
          disabled={disabled || view.page <= 1}
          aria-label="上一谱页"
          onClick={() => page(view.page - 1)}
        >
          上一页
        </button>
        <label>
          第{" "}
          <input
            aria-label="谱页页码"
            type="number"
            min={1}
            max={count || 1}
            value={view.page}
            disabled={!count || disabled}
            onChange={(e) => {
              if (e.target.value) page(Number(e.target.value));
            }}
          />{" "}
          / {count || "…"} 页
        </label>
        <button
          aria-label="下一谱页"
          disabled={disabled || !count || view.page >= count}
          onClick={() => page(view.page + 1)}
        >
          下一页
        </button>
        <button
          className={view.fit ? "selected" : ""}
          onClick={() => setView((v) => ({ ...v, fit: true }))}
        >
          适合宽度
        </button>
        <select
          aria-label="谱页缩放"
          value={view.fit ? "fit" : String(view.zoom)}
          onChange={(e) =>
            setView((v) =>
              e.target.value === "fit"
                ? { ...v, fit: true }
                : { ...v, fit: false, zoom: Number(e.target.value) },
            )
          }
        >
          <option value="fit">适合宽度</option>
          {[25, 50, 75, 100, 125, 150, 200, 250, 300].map((z) => (
            <option key={z} value={z}>
              {z}%
            </option>
          ))}
        </select>
        <button
          onClick={() =>
            setView((v) => ({ ...v, rotation: (v.rotation + 90) % 360 }))
          }
        >
          旋转 90°
        </button>
        <button
          disabled={
            disabled ||
            loading ||
            !!error ||
            needsPassword ||
            annotationEditing ||
            exportBusy ||
            !count
          }
          onClick={() => void exportPage()}
          title="导出完整当前页、编号和批注说明，不受阅读缩放影响"
        >
          {exportBusy ? "正在导出…" : "导出本页批注"}
        </button>
        {manage && attachment.format === "images" && (
          <>
            <button
              disabled={disabled || view.page <= 1}
              onClick={() =>
                void update("movePage", {
                  page: view.page - 1,
                  direction: -1,
                }).then(() => page(view.page - 1))
              }
            >
              此页前移
            </button>
            <button
              disabled={disabled || view.page >= count}
              onClick={() =>
                void update("movePage", {
                  page: view.page - 1,
                  direction: 1,
                }).then(() => page(view.page + 1))
              }
            >
              此页后移
            </button>
            <button
              disabled={disabled || count <= 1}
              onClick={() => void update("removePage", { page: view.page - 1 })}
            >
              移除此页
              {(attachment.annotations ?? []).filter(
                (n) => n.assetId === attachment.pages[view.page - 1]?.id,
              ).length > 0
                ? `及 ${(attachment.annotations ?? []).filter((n) => n.assetId === attachment.pages[view.page - 1]?.id).length} 条批注`
                : ""}
            </button>
          </>
        )}
      </fieldset>
      {!readOnly && !previewKind && (practice || mapping) && (
        <div className="paper-follow-controls">
          <label>
            <input
              type="checkbox"
              aria-label="纸谱自动翻页"
              checked={followPending ?? !!mapping?.enabled}
              disabled={disabled || !practice || !mapping || stale}
              onChange={(e) => {
                const enabled = e.target.checked;
                setFollowPending(enabled);
                void saveMapping({ ...mapping!, enabled })
                  .then(() => setSuspended(false))
                  .finally(() => setFollowPending(null));
              }}
            />
            自动翻页
          </label>
          {playback && mapping?.enabled && !stale && (
            <span>
              {annotationEditing
                ? "批注编辑中，自动翻页暂停"
                : suspended
                  ? "已手动翻页，跟随暂停"
                  : `跟随第 ${measured} 小节`}
            </span>
          )}
          {playback && mapping?.enabled && suspended && (
            <button disabled={disabled} onClick={() => setSuspended(false)}>
              继续跟随
            </button>
          )}
          {stale && (
            <span role="status">小节位置已改变，请核对后重新确认对应。</span>
          )}
          {invalidPages && (
            <span role="status">
              部分对应页码不可用，请修改后继续自动翻页。
            </span>
          )}
          <details className="paper-mapping-editor">
            <summary>
              小节与谱页对应{mapping ? ` · ${mapping.anchors.length} 处` : ""}
            </summary>
            <p className="parameter-help">
              从指定演奏小节开始显示此页，直到下一个对应点。反复回到前页时，为展开后的演奏小节添加对应。可框选页内位置，随小节定位到具体谱行。
            </p>
            {practice && (
              <div className="paper-anchor-add">
                <label>
                  演奏小节{" "}
                  <input
                    aria-label="谱页对应小节"
                    type="number"
                    min={1}
                    max={practice.measures}
                    value={anchorMeasure}
                    disabled={disabled || stale}
                    onChange={(e) => setAnchorMeasure(Number(e.target.value))}
                  />
                </label>
                <span>→ 当前第 {view.page} 页{anchorRegion&&anchorRegionPage===view.page?" · 已选页内位置":""}</span>
                <button disabled={disabled||stale||annotationEditing||!count||loading||selectingRegion} onClick={()=>setSelectingRegion(true)}>框选小节位置</button>
                {selectingRegion&&<><span role="status">在谱页上拖选此小节或谱行的位置。</span><button onClick={()=>setSelectingRegion(false)}>取消框选</button></>}
                {anchorRegion&&!selectingRegion&&<button disabled={disabled} onClick={()=>setAnchorRegion(null)}>清除页内位置</button>}
                {playback && (
                  <button
                    disabled={disabled || stale}
                    onClick={() => setAnchorMeasure(measured)}
                  >
                    使用当前小节
                  </button>
                )}
                <button
                  disabled={
                    disabled ||
                    selectingRegion ||
                    stale ||
                    !count ||
                    !Number.isInteger(anchorMeasure) ||
                    anchorMeasure < 1 ||
                    anchorMeasure > practice.measures
                  }
                  onClick={() =>
                    void saveMapping({
                      grid: practice.grid,
                      enabled: mapping?.enabled ?? false,
                      anchors: [
                        ...(mapping?.anchors ?? []).filter(
                          (a) => a.measure !== anchorMeasure,
                        ),
                        { measure: anchorMeasure, page: view.page, ...(anchorRegion&&anchorRegionPage===view.page?{region:anchorRegion}:{}) },
                      ].sort((a, b) => a.measure - b.measure),
                    })
                  }
                >
                  保存此页对应
                </button>
              </div>
            )}
            {mapping?.anchors.map((a) => (
              <div className="paper-anchor-row" key={a.measure}>
                <span>
                  第 {a.measure} 小节 → 第 {a.page} 页{a.region?" · 页内位置":""}
                  {practice && a.measure > practice.measures
                    ? "（小节超出当前曲目）"
                    : ""}
                  {a.page > count && count > 0 ? "（页码不可用）" : ""}
                </span>
                <button disabled={disabled||annotationEditing} onClick={()=>{setAnchorMeasure(a.measure);setAnchorRegion(a.region??null);setAnchorRegionPage(a.page);page(a.page);followedRegion.current="";if(a.region&&a.page===view.page)scrollRegion(a.region);}}>查看 / 修改对应</button>
                <button disabled={disabled||!practice||stale||annotationEditing||a.measure>practice.measures} onClick={()=>void jumpAnchor(a)}>定位练习</button>
                <button disabled={disabled||!practice||stale||annotationEditing||a.measure>practice.measures} onClick={()=>setPracticeAnchor(a.measure)}>建立练习段</button>
                <button
                  disabled={disabled || !practice || stale}
                  aria-label={`移除第 ${a.measure} 小节对应`}
                  onClick={() => {
                    const anchors = mapping.anchors.filter(
                      (v) => v.measure !== a.measure,
                    );
                    void saveMapping(
                      anchors.length ? { ...mapping, anchors } : null,
                    );
                  }}
                >
                  移除
                </button>
              </div>
            ))}
            {stale && practice && mapping && (
              <button
                disabled={
                  disabled ||
                  mapping.anchors.some(
                    (a) => a.measure > practice.measures || a.page > count,
                  )
                }
                onClick={() =>
                  void saveMapping({
                    ...mapping,
                    grid: practice.grid,
                    enabled: false,
                  })
                }
              >
                按当前小节重新确认
              </button>
            )}
            {stale &&
              practice &&
              mapping &&
              mapping.anchors.some(
                (a) => a.measure > practice.measures || a.page > count,
              ) &&
              count > 0 && (
                <button
                  disabled={disabled}
                  onClick={() => {
                    const anchors = mapping.anchors.filter(
                      (a) => a.measure <= practice.measures && a.page <= count,
                    );
                    void saveMapping(
                      anchors.length
                        ? {
                            ...mapping,
                            grid: practice.grid,
                            enabled: false,
                            anchors,
                          }
                        : null,
                    );
                  }}
                >
                  移除超出范围的对应并重新确认
                </button>
              )}
            {mapping && practice && (
              <button
                disabled={disabled}
                onClick={() => void saveMapping(null)}
              >
                清除全部对应
              </button>
            )}
            {!practice && (
              <p className="parameter-help">
                打开该曲目练习后可以编辑小节对应。
              </p>
            )}
          </details>
        </div>
      )}
      {error && (
        <p className="dialog-error" role="alert">
          {error}
        </p>
      )}
      {exportNotice && (
        <p className="parameter-help" role="status">
          {exportNotice}
        </p>
      )}
      {needsPassword && (
        <form
          className="paper-password"
          onSubmit={(e) => {
            e.preventDefault();
            setLoading(true);
            passwordCallback.current?.(password);
          }}
        >
          <input
            type="password"
            aria-label="PDF 密码"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="off"
          />
          <button disabled={!password}>打开 PDF</button>
        </form>
      )}
      <div
        className="paper-sheet-scroll"
        ref={frame}
        tabIndex={0}
        aria-label="谱页画布"
      >
        {annotationNotice && <p role="status">{annotationNotice}</p>}
        <PaperAnnotations
          key={`${attachment.format === "pdf" ? attachment.pages[0]?.id : attachment.pages[view.page - 1]?.id}:${attachment.format === "pdf" ? view.page : 1}`}
          contentId={contentId}
          book={attachment.id}
          assetId={
            (attachment.format === "pdf"
              ? attachment.pages[0]
              : attachment.pages[view.page - 1]
            )?.id ?? ""
          }
          page={attachment.format === "pdf" ? view.page : 1}
          rotation={view.rotation}
          width={sheetSize.width}
          height={sheetSize.height}
          notes={attachment.annotations ?? []}
          disabled={
            readOnly || !!previewKind || disabled || loading || !!error || needsPassword || exportBusy
          }
          editing={setAnnotationEditing}
          regionSelection={selectingRegion}
          regionSelected={(region)=>{setAnchorRegion(region);setAnchorRegionPage(view.page);setSelectingRegion(false);}}
          save={saveAnnotation}
          saveInk={saveInk?(expected,strokes)=>saveInk((attachment.format==="pdf"?attachment.pages[0]:attachment.pages[view.page-1])?.id??"",attachment.format==="pdf"?view.page:1,expected,strokes):undefined}
        >
          <canvas
            ref={canvas}
            aria-label={`谱面第 ${view.page} 页`}
            hidden={!!error || needsPassword}
          />
          {mapping?.anchors.filter(a=>a.page===view.page&&a.region).map(a=>{
            const r=paperAnnotationBox(a.region!,view.rotation),active=!stale&&mapping.anchors.slice().reverse().find(n=>n.measure<=measured)?.measure===a.measure;
            return <button key={`anchor-${a.measure}`} className={`paper-measure-region ${active?"active":""}`} style={{left:`${r.x*100}%`,top:`${r.y*100}%`,width:`${r.width*100}%`,height:`${r.height*100}%`}} aria-label={`谱页第 ${a.measure} 小节位置`} title={`定位第 ${a.measure} 小节`} disabled={disabled||readOnly||!practice||stale||annotationEditing||selectingRegion||a.measure>practice.measures} onClick={e=>{e.stopPropagation();void jumpAnchor(a);}}><span>{a.measure}</span></button>;
          })}
          {anchorRegion&&anchorRegionPage===view.page&&(()=>{const r=paperAnnotationBox(anchorRegion,view.rotation);return <span className="paper-measure-region draft" style={{left:`${r.x*100}%`,top:`${r.y*100}%`,width:`${r.width*100}%`,height:`${r.height*100}%`}}>第 {anchorMeasure} 小节位置</span>})()}
        </PaperAnnotations>
        {loading && (
          <span className="paper-loading" role="status">
            正在载入谱页…
          </span>
        )}
      </div>
      {practiceAnchor!==null&&<PaperPracticeBuilder key={practiceAnchor} contentId={contentId} book={attachment.id} measure={practiceAnchor} close={()=>setPracticeAnchor(null)}/>}
    </div>
  );
}

export async function verifyPdfRenderer() {
  const pdfjs = await import("pdfjs-dist");
  pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;
  const content = "0 0 0 rg 10 10 80 80 re f\n";
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R >>",
    `<< /Length ${content.length} >>\nstream\n${content}endstream`,
  ];
  let text = "%PDF-1.4\n";
  const offsets = [0];
  objects.forEach((o, i) => {
    offsets.push(text.length);
    text += `${i + 1} 0 obj\n${o}\nendobj\n`;
  });
  const at = text.length;
  text += "xref\n0 5\n0000000000 65535 f \n";
  for (const n of offsets.slice(1))
    text += `${n.toString().padStart(10, "0")} 00000 n \n`;
  text += `trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n${at}\n%%EOF\n`;
  const task = pdfjs.getDocument({ data: new TextEncoder().encode(text) });
  try {
    const doc = await task.promise;
    const page = await doc.getPage(1);
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 100;
    const ctx = canvas.getContext("2d")!;
    await page.render({
      canvas,
      canvasContext: ctx,
      viewport: page.getViewport({ scale: 1 }),
    }).promise;
    return ctx.getImageData(50, 50, 1, 1).data[0] < 10;
  } finally {
    await task.destroy();
  }
}
(
  window as unknown as { __neothesiaPdfCheck: () => Promise<boolean> }
).__neothesiaPdfCheck = verifyPdfRenderer;
