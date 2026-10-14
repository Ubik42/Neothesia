import { useEffect, useMemo, useRef, useState } from "react";
import { api, pitchName, type LoadedSong, type Note } from "./api";
import { annotatedPagePng } from "./PaperPageExport";
import { annotatedBookPdf } from "./PaperBookExport";
import { PaperPracticeBuilder } from "./PaperPracticeBuilder";
import { PaperSources } from "./PaperSources";
import { PaperTransferReview } from "./PaperTransferReview";
import "./paper-navigation.css";
import "./history-paper.css";
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
  noteAnchors?: PaperNoteAnchor[];
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
interface PaperNoteAnchor {
  track: number;
  index: number;
  pitch: number;
  measure: number;
  assetId: string;
  page: number;
  grid: string;
  region: PaperRegion;
}
type HistoryPaperFocus = {
  contentId: string;
  measure: number;
  historyId: string;
  reference: number;
  pitch: number;
  track: number;
  songTime: number;
  index?: number | null;
  startMeasure: number;
  endMeasure: number;
  request: string;
};
function paperNoteAnchor(
  attachment: PaperAttachment,
  practice: PracticeContext,
  focus: HistoryPaperFocus,
): PaperNoteAnchor | null {
  if (focus.index == null || !Number.isSafeInteger(focus.index)) return null;
  const anchor = attachment.noteAnchors?.find((item) =>
    item.track === focus.track && item.index === focus.index &&
    item.pitch === focus.pitch && item.measure === focus.measure &&
    item.grid === practice.grid,
  );
  if (!anchor) return null;
  const asset = attachment.format === "pdf" ? attachment.pages[0] : attachment.pages[anchor.page - 1];
  return asset?.id === anchor.assetId && asset.available ? anchor : null;
}
function historyMappingIssue(
  attachment: PaperAttachment,
  practice: PracticeContext,
  focus: HistoryPaperFocus,
): string | null {
  if (paperNoteAnchor(attachment, practice, focus)) return null;
  if (!attachment.mapping?.anchors.length) return "尚未设置小节与谱页对应";
  if (attachment.mapping.grid !== practice.grid)
    return "小节位置已改变，映射需要重新核对";
  const anchor = attachment.mapping.anchors
    .filter((item) => item.measure <= focus.measure)
    .sort((left, right) => right.measure - left.measure)[0];
  if (!anchor) return `第 ${focus.measure} 小节之前没有纸谱对应点`;
  if (attachment.format === "images" && !attachment.pages[anchor.page - 1]?.available)
    return `第 ${anchor.page} 页图片不可用`;
  if (attachment.format === "pdf" && !attachment.pages[0]?.available)
    return "PDF 文件不可用";
  return null;
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
  historyFocus,
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
  historyFocus?: HistoryPaperFocus | null;
}) {
  const [sourceBusy,setSourceBusy]=useState(false);
  const [transferBusy,setTransferBusy]=useState(false);
  const [practice, setPractice] = useState<PracticeContext | null>(null);
  const [song, setSong] = useState<LoadedSong | null>(null);
  const [practiceLoaded, setPracticeLoaded] = useState(false);
  const [focusBook, setFocusBook] = useState<string | null>(null);
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
  const focus = historyFocus?.contentId === contentId ? historyFocus : null;
  const eligible = focus && practice
    ? data?.attachments.filter((item) => !historyMappingIssue(item, practice, focus)) ?? []
    : [];
  const preferred = focus && eligible.length
    ? eligible.find((item) => item.id === focusBook) ??
      eligible.find((item) => practice && paperNoteAnchor(item, practice, focus)) ??
      eligible.find((item) => item.id === data?.active) ??
      (eligible.length === 1 ? eligible[0] : null)
    : null;
  const current = focus
    ? preferred ?? data?.attachments.find((item) => item.id === data.active) ?? data?.attachments[0]
    : data?.attachments.find((item) => item.id === data.active) ?? data?.attachments[0];
  const locating = !!focus && !!current && preferred?.id === current.id;
  useEffect(() => setFocusBook(null), [focus?.request]);
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
    setSong(null);
    setPracticeLoaded(false);
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
      .catch(() => {})
      .finally(() => {
        if (contextAlive && alive.current) setPracticeLoaded(true);
      });
    if (!libraryPath) void api.command<LoadedSong>({ type: "currentSong" })
      .then((value) => {
        if (contextAlive && alive.current && currentContent.current === contentId && value.contentId === contentId)
          setSong(value);
      }).catch(() => {});
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
            onChange={(e) => {
              if (focus) setFocusBook(e.target.value);
              else void work(() => update(e.target.value, "select"));
            }}
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
      {focus && (
        <div className="paper-history-location" role="status">
          {!data ? "正在读取这首曲目的纸谱…" : !data.attachments.length
            ? "练习段已准备好。这首曲目还没有纸谱，请先在谱面管理中添加 PDF 或图片。"
            : !practiceLoaded ? "正在核对练习小节与纸谱位置…"
            : !practice || practice.contentId !== contentId
              ? "练习段已准备好，但当前曲目小节资料无法读取，纸谱没有自动定位。"
              : !Number.isSafeInteger(focus.measure) || focus.measure < 1 || focus.measure > practice.measures
                ? "练习段已准备好，但目标小节超出当前曲目范围，纸谱没有自动定位。"
                : eligible.length > 1 && !preferred
                  ? "多份纸谱都能定位这一小节，请选择要使用的纸谱。"
                  : preferred
                    ? `目标音在第 ${focus.measure} 小节；本次重练第 ${focus.startMeasure}–${focus.endMeasure} 小节。${preferred.name} 将定位到目标小节对应的谱页。`
                    : "练习段已准备好，但还没有可定位这一小节的纸谱。请在谱面管理中核对对应点。"}
          {practice && !preferred && !!data?.attachments.length && (
            <ul>
              {data.attachments.map((item) => (
                <li key={item.id}>
                  {item.name}：{historyMappingIssue(item, practice, focus) ?? "可定位，请选择"}
                </li>
              ))}
            </ul>
          )}
          {practice && eligible.length > 1 && (
            <label>
              选择纸谱{" "}
              <select
                aria-label="选择重练纸谱"
                value={preferred?.id ?? ""}
                onChange={(event) => setFocusBook(event.target.value)}
              >
                <option value="">选择一份纸谱</option>
                {eligible.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
              </select>
            </label>
          )}
        </div>
      )}
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
          song={song}
          playback={playback}
          historyFocus={locating ? focus : null}
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
          saveNoteAnchor={(track, index, draft, expected) =>
            work(async () => {
              await api.command({
                type: "editPaperNoteAnchor",
                content_id: contentId,
                book: current.id,
                track,
                index,
                draft,
                expected,
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
  song,
  playback,
  historyFocus,
  saveMapping,
  saveNoteAnchor,
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
  song?: LoadedSong | null;
  playback?: { measure: number };
  historyFocus?: HistoryPaperFocus | null;
  saveMapping: (mapping: PaperMapping | null) => Promise<void>;
  saveNoteAnchor?: (
    track: number,
    index: number,
    draft: Pick<PaperNoteAnchor, "pitch" | "measure" | "assetId" | "page" | "region"> | null,
    expected: PaperNoteAnchor | null,
  ) => Promise<void>;
  saveInk?:(assetId:string,page:number,expected:PaperAnnotation[],strokes:PaperAnnotation[])=>Promise<boolean>;
  saveAnnotation: (
    id: string | null,
    draft: PaperAnnotationDraft | null,
    expected: PaperAnnotation | null,
  ) => Promise<boolean>;
}) {
  const [exportBusy, setExportBusy] = useState(false),
    [bookBusy, setBookBusy] = useState(false),
    [exportNotice, setExportNotice] = useState("");
  const exportLock = useRef(false),
    exportCancelled = useRef(false),
    readerAlive = useRef(true);
  useEffect(() => {
    readerAlive.current = true;
    return () => {
      readerAlive.current = false;
    };
  }, []);
  const [annotationEditing, setAnnotationEditing] = useState(false),
    [annotationNotice, setAnnotationNotice] = useState(""),
    [annotationQuery, setAnnotationQuery] = useState(""),
    [sheetSize, setSheetSize] = useState({ width: 1, height: 1 });
  const [renderedPage, setRenderedPage] = useState(0);
  const [suspended, setSuspended] = useState(!!historyFocus),
    [followPending, setFollowPending] = useState<boolean | null>(null),
    [anchorMeasure, setAnchorMeasure] = useState(practice?.measure || 1);
  const [anchorRegion,setAnchorRegion]=useState<PaperRegion|null>(null),[anchorRegionPage,setAnchorRegionPage]=useState(attachment.view.page),[selectingRegion,setSelectingRegion]=useState(false);
  const [noteMeasure, setNoteMeasure] = useState(practice?.measure || 1);
  const [noteKey, setNoteKey] = useState("");
  const [noteRegion, setNoteRegion] = useState<PaperRegion | null>(null);
  const [noteRegionPage, setNoteRegionPage] = useState(attachment.view.page);
  const [selectingNoteRegion, setSelectingNoteRegion] = useState(false);
  const [practiceAnchor,setPracticeAnchor]=useState<number|null>(null);
  const followedRegion=useRef("");
  const pendingAnnotation=useRef<{page:number;region:PaperRegion}|null>(null);
  const handledHistoryFocus = useRef("");
  const [historyNotice, setHistoryNotice] = useState("");
  const [historyTarget, setHistoryTarget] = useState<{
    request: string; measure: number; page: number; region: PaperRegion;
    note?: { track: number; index: number; pitch: number; assetId: string; grid: string };
  } | null>(null);
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
  const measureForNote = song?.measures[noteMeasure - 1];
  const notesForMeasure = measureForNote
    ? song!.notes.filter((note) => note.tick >= measureForNote.startTick && note.tick < measureForNote.endTick)
        .sort((left, right) => left.tick - right.tick || left.track - right.track || left.index - right.index)
    : [];
  const selectedNote: Note | undefined = notesForMeasure.find((note) => `${note.track}:${note.index}` === noteKey) ?? notesForMeasure[0];
  const savedNoteAnchor = selectedNote && attachment.noteAnchors?.find((anchor) =>
    anchor.track === selectedNote.track && anchor.index === selectedNote.index);
  const noteCoverage = useMemo(() => {
    if (!song || !practice || song.contentId !== contentId) return { notes: [], marked: new Set<string>() };
    const measureAt = (tick: number) => {
      let low = 0, high = song.measures.length - 1;
      while (low <= high) {
        const middle = (low + high) >> 1;
        const measure = song.measures[middle];
        if (tick < measure.startTick) high = middle - 1;
        else if (tick >= measure.endTick) low = middle + 1;
        else return middle + 1;
      }
      return 0;
    };
    const notes = song.notes.flatMap((note) => {
      const measure = measureAt(note.tick);
      return measure === 0 ? [] : [{ note, measure }];
    }).sort((a, b) => a.measure - b.measure || a.note.tick - b.note.tick ||
      a.note.track - b.note.track || a.note.index - b.note.index);
    const valid = new Map(notes.map(({ note, measure }) => [`${note.track}:${note.index}`, { note, measure }]));
    const marked = new Set<string>();
    for (const anchor of attachment.noteAnchors ?? []) {
      const key = `${anchor.track}:${anchor.index}`;
      const target = valid.get(key);
      const asset = attachment.format === "pdf" ? attachment.pages[0] : attachment.pages[anchor.page - 1];
      if (target && target.note.pitch === anchor.pitch && target.measure === anchor.measure &&
          anchor.grid === practice.grid && asset?.id === anchor.assetId && asset.available) marked.add(key);
    }
    return { notes, marked };
  }, [song, practice?.grid, contentId, attachment.noteAnchors, attachment.pages, attachment.format]);
  const unmarkedNote = useMemo(() => {
    const position = noteCoverage.notes.findIndex(({ note }) =>
      note.track === selectedNote?.track && note.index === selectedNote?.index);
    return noteCoverage.notes.slice(position + 1).concat(noteCoverage.notes.slice(0, position + 1))
      .find(({ note }) => !noteCoverage.marked.has(`${note.track}:${note.index}`));
  }, [noteCoverage, selectedNote?.track, selectedNote?.index]);
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
  const currentAsset = attachment.format === "pdf" ? attachment.pages[0] : attachment.pages[view.page - 1];
  const canvas = useRef<HTMLCanvasElement>(null),
    frame = useRef<HTMLDivElement>(null),
    passwordCallback = useRef<((v: string) => void) | null>(null),
    ready = useRef(false),
    latest = useRef(view);
  useEffect(()=>{if(count>0)onPageCount?.(count);},[count,onPageCount]);
  latest.current = view;
  const annotationLocations=(attachment.annotations??[])
    .filter(note=>!note.ink && (note.text.trim() || note.width || note.height))
    .map(note=>({note,page:attachment.format==="pdf"?note.page:attachment.pages.findIndex(asset=>asset.id===note.assetId)+1}))
    .filter(entry=>entry.page>0 && entry.page<=count)
    .sort((left,right)=>left.page-right.page || left.note.y-right.note.y || left.note.x-right.note.x);
  const matchingAnnotations=annotationLocations.filter(({note,page})=>
    !annotationQuery.trim() || note.text.toLocaleLowerCase("zh-CN").includes(annotationQuery.trim().toLocaleLowerCase("zh-CN")) || String(page)===annotationQuery.trim(),
  );
  useEffect(()=>{
    const pending=pendingAnnotation.current;
    if(!pending || loading || renderedPage !== pending.page || view.page!==pending.page)return;
    pendingAnnotation.current=null;
    scrollRegion(pending.region);
  },[loading,renderedPage,view.page,view.rotation,sheetSize]);
  const invalidPages =
    count > 0 && !!mapping?.anchors.some((a) => a.page > count);
  const highlightedHistory = historyFocus && historyTarget?.request === historyFocus.request &&
    practice?.contentId === contentId && view.page === historyTarget.page &&
    (historyTarget.note
      ? attachment.noteAnchors?.some((anchor) => anchor.track === historyTarget.note!.track &&
          anchor.index === historyTarget.note!.index && anchor.pitch === historyTarget.note!.pitch &&
          anchor.assetId === historyTarget.note!.assetId && anchor.grid === historyTarget.note!.grid &&
          anchor.page === historyTarget.page &&
          JSON.stringify(anchor.region) === JSON.stringify(historyTarget.region))
      : !stale && mapping?.anchors.some((anchor) => anchor.measure === historyTarget.measure &&
          anchor.page === historyTarget.page && anchor.region &&
          JSON.stringify(anchor.region) === JSON.stringify(historyTarget.region)))
      ? historyTarget : null;
  useEffect(() => {
    pendingAnnotation.current = null;
    setHistoryTarget(null);
    setHistoryNotice("");
  }, [historyFocus?.request]);
  useEffect(() => {
    if (!historyTarget || !historyFocus || historyTarget.request !== historyFocus.request) return;
    const stillMapped = practice?.contentId === contentId && (historyTarget.note
      ? attachment.noteAnchors?.some((anchor) => anchor.track === historyTarget.note!.track &&
          anchor.index === historyTarget.note!.index && anchor.assetId === historyTarget.note!.assetId &&
          anchor.grid === historyTarget.note!.grid && anchor.page === historyTarget.page &&
          JSON.stringify(anchor.region) === JSON.stringify(historyTarget.region))
      : !stale && mapping?.anchors.some((anchor) => anchor.measure === historyTarget.measure &&
          anchor.page === historyTarget.page &&
          JSON.stringify(anchor.region) === JSON.stringify(historyTarget.region)));
    if (!stillMapped) {
      pendingAnnotation.current = null;
      setHistoryTarget(null);
      setHistoryNotice("纸谱小节区域已经改变，历史高亮已清除。请重新从历史成绩定位。");
    }
  }, [historyTarget, historyFocus?.request, stale, practice?.contentId, contentId, mapping, attachment.noteAnchors]);
  useEffect(() => {
    if (!historyFocus || handledHistoryFocus.current === historyFocus.request || !practice)
      return;
    const fail = (message: string) => {
      handledHistoryFocus.current = historyFocus.request;
      pendingAnnotation.current = null;
      setHistoryTarget(null);
      setHistoryNotice(message);
    };
    if (historyFocus.contentId !== contentId || practice.contentId !== contentId) {
      fail("当前曲目与历史记录不一致，纸谱没有自动定位。");
      return;
    }
    if (!Number.isSafeInteger(historyFocus.measure) || historyFocus.measure < 1 || historyFocus.measure > practice.measures) {
      fail("目标小节超出当前曲目范围，纸谱没有自动定位。");
      return;
    }
    const issue = historyMappingIssue(attachment, practice, historyFocus);
    if (issue) {
      fail(`${attachment.name}：${issue}，纸谱没有自动定位。`);
      return;
    }
    if (!count && loading) return;
    const exactNote = paperNoteAnchor(attachment, practice, historyFocus);
    const anchor = exactNote ?? mapping!.anchors
      .filter((item) => item.measure <= historyFocus.measure)
      .sort((left, right) => right.measure - left.measure)[0];
    if (!count || anchor.page > count) {
      fail(`第 ${anchor.page} 页不可用，纸谱没有自动定位。`);
      return;
    }
    handledHistoryFocus.current = historyFocus.request;
    setSuspended(true);
    setView((previous) => previous.page === anchor.page ? previous : { ...previous, page: anchor.page });
    frame.current?.scrollTo({ top: 0, left: 0 });
    const exactRegion = anchor.measure === historyFocus.measure ? anchor.region : undefined;
    setHistoryTarget(exactRegion ? {
      request: historyFocus.request, measure: historyFocus.measure,
      page: anchor.page, region: exactRegion,
      ...(exactNote ? { note: {
        track: exactNote.track, index: exactNote.index, pitch: exactNote.pitch,
        assetId: exactNote.assetId, grid: exactNote.grid,
      } } : {}),
    } : null);
    if (anchor.region) {
      pendingAnnotation.current = { page: anchor.page, region: anchor.region };
      if (!loading && renderedPage === anchor.page && latest.current.page === anchor.page) {
        scrollRegion(anchor.region);
        pendingAnnotation.current = null;
      }
    }
    const location = exactNote
      ? "已标出目标音符位置"
      : exactRegion
      ? "已标出目标小节区域；这个区域不是单个音符的精确位置"
      : anchor.region
        ? `按第 ${anchor.measure} 小节的区域定位谱行；目标小节没有独立的页内区域`
        : "只定位到谱页；目标小节没有独立的页内区域";
    setHistoryNotice(`已定位第 ${historyFocus.measure} 小节的参考音 ${pitchName(historyFocus.pitch)}：第 ${anchor.page} 页。${location}。自动翻页已暂停，练习段已准备好。`);
  }, [historyFocus?.request, practice, attachment, mapping, count, loading, renderedPage, contentId]);
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
    if(loading || renderedPage !== view.page)return;
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
    loading,renderedPage,view.page,view.rotation,sheetSize,
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
    setRenderedPage(0);
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
    let rendered = false;
    void draw().then(() => { rendered = true; })
      .catch((e) => {
        if (!cancelled) setError(`谱页显示失败：${e.message ?? e}`);
      })
      .finally(() => {
        if (!cancelled) {
          setRenderedPage(rendered ? view.page : 0);
          setLoading(false);
        }
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
  const locateAnnotation=(note:PaperAnnotation,pageNumber:number)=>{
    if(annotationEditing || selectingRegion || exportLock.current || disabled || !count)return;
    pendingAnnotation.current={page:pageNumber,region:{x:note.x,y:note.y,width:Math.max(note.width??0,0.035),height:Math.max(note.height??0,0.035)}};
    page(pageNumber);
    if(pageNumber===view.page && !loading && renderedPage===pageNumber){
      const pending=pendingAnnotation.current;
      pendingAnnotation.current=null;
      if(pending)scrollRegion(pending.region);
    }
  };
  const renderForExport = async (pageNumber: number, rotation: number) => {
    const asset = attachment.format === "pdf" ? attachment.pages[0] : attachment.pages[pageNumber - 1];
    if (!asset?.available) throw new Error(`第 ${pageNumber} 页副本不可用，请先修复谱页`);
    const source = document.createElement("canvas");
    const context = source.getContext("2d");
    if (!context) throw new Error("无法创建导出画布");
    if (attachment.format === "pdf") {
      if (!doc) throw new Error("PDF 尚未加载完成");
      const pdfPage = await doc.getPage(pageNumber);
      const base = pdfPage.getViewport({ scale: 1, rotation: (pdfPage.rotate + rotation) % 360 });
      const scale = Math.min(1800 / base.width, Math.sqrt(12_000_000 / (base.width * base.height)));
      const viewport = pdfPage.getViewport({ scale, rotation: (pdfPage.rotate + rotation) % 360 });
      source.width = Math.max(1, Math.ceil(viewport.width));
      source.height = Math.max(1, Math.ceil(viewport.height));
      await pdfPage.render({ canvas: source, canvasContext: context, viewport }).promise;
    } else {
      const bytes = await api.readScoreAttachment(contentId, attachment.id, pageNumber - 1, previewKind);
      const url = URL.createObjectURL(new Blob([bytes.slice().buffer], { type: `image/${asset.kind}` }));
      try {
        const image = new Image();
        image.src = url;
        await image.decode();
        if (image.naturalWidth * image.naturalHeight > 64_000_000)
          throw new Error(`第 ${pageNumber} 页图片过大，请缩小后导出`);
        const swap = rotation % 180 !== 0;
        const width = swap ? image.naturalHeight : image.naturalWidth;
        const height = swap ? image.naturalWidth : image.naturalHeight;
        const scale = Math.min(1800 / width, 1, Math.sqrt(12_000_000 / (width * height)));
        source.width = Math.max(1, Math.ceil(width * scale));
        source.height = Math.max(1, Math.ceil(height * scale));
        context.fillStyle = "#fff";
        context.fillRect(0, 0, source.width, source.height);
        context.translate(source.width / 2, source.height / 2);
        context.rotate((rotation * Math.PI) / 180);
        context.scale(scale, scale);
        context.drawImage(image, -image.naturalWidth / 2, -image.naturalHeight / 2);
      } finally {
        URL.revokeObjectURL(url);
      }
    }
    return source;
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
    try {
      const source = await renderForExport(pageNumber, rotation);
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
      exportLock.current = false;
      if (readerAlive.current) setExportBusy(false);
    }
  };
  const exportBook = async () => {
    if (exportLock.current || disabled || annotationEditing || selectingRegion ||
        loading || error || needsPassword || !count) return;
    exportLock.current = true;
    exportCancelled.current = false;
    setExportBusy(true);
    setBookBusy(true);
    setExportNotice("");
    if (playback && mapping?.enabled) setSuspended(true);
    const name = attachment.name;
    const rotation = view.rotation;
    try {
      const bytes = await annotatedBookPdf({
        name,
        pages: count,
        rotation,
        render: async (pageNumber) => {
          if (!readerAlive.current || exportCancelled.current) throw new Error("整本导出已停止");
          return renderForExport(pageNumber, rotation);
        },
        cancelled: () => exportCancelled.current || !readerAlive.current,
        annotations: (pageNumber) => {
          const asset = attachment.format === "pdf" ? attachment.pages[0] : attachment.pages[pageNumber - 1];
          return (attachment.annotations ?? []).filter((note) =>
            note.assetId === asset?.id && note.page === (attachment.format === "pdf" ? pageNumber : 1));
        },
        progress: (pageNumber, total) => {
          if (readerAlive.current) setExportNotice(`正在制作第 ${pageNumber} / ${total} 页批注 PDF…`);
        },
      });
      const fileName = `${name.replace(/\.[^.]+$/, "").replace(/[<>:"/\\|?*\x00-\x1f]/g, "_")}-整本批注.pdf`;
      if (api.desktop) {
        const path = await api.exportPaperBook(fileName, bytes);
        if (readerAlive.current) setExportNotice(path ? `已保存：${path}` : "已取消保存");
      } else {
        const url = URL.createObjectURL(new Blob([bytes.slice().buffer], { type: "application/pdf" }));
        const link = document.createElement("a");
        link.href = url;
        link.download = fileName;
        link.click();
        setTimeout(() => URL.revokeObjectURL(url), 30000);
        if (readerAlive.current) setExportNotice(`已导出整本批注 PDF，共 ${count} 页原谱。`);
      }
    } catch (e) {
      if (readerAlive.current) setExportNotice(exportCancelled.current ? "已取消整本导出" : `整本导出失败：${String(e)}`);
    } finally {
      exportLock.current = false;
      if (readerAlive.current) { setExportBusy(false); setBookBusy(false); }
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
        <button
          disabled={disabled || loading || !!error || needsPassword || annotationEditing || selectingRegion || exportBusy || !count}
          onClick={() => void exportBook()}
          title="按当前旋转方向导出整份谱本和已保存的批注；长批注会自动续到下一张打印页"
        >
          导出整本批注 PDF
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
      {historyNotice && <p role="status" className="paper-history-result">{historyNotice}</p>}
      {!!annotationLocations.length && !previewKind && (
        <details className="paper-annotation-navigation">
          <summary>批注位置 · {annotationLocations.length} 处</summary>
          <label>
            查找批注或页码
            <input aria-label="查找纸谱批注或页码" value={annotationQuery} onChange={event=>setAnnotationQuery(event.target.value)} placeholder="文字或页码" />
          </label>
          <div className="paper-annotation-navigation-list">
            {matchingAnnotations.map(({note,page:pageNumber})=>(
              <button key={note.id} type="button" disabled={disabled || annotationEditing || selectingRegion || exportBusy} onClick={()=>locateAnnotation(note,pageNumber)} title={note.text || "高亮位置"}>
                <span>第 {pageNumber} 页</span>
                <span>{note.text.trim().slice(0,80) || "高亮位置"}</span>
              </button>
            ))}
            {!matchingAnnotations.length && <p>没有找到对应的批注。</p>}
          </div>
        </details>
      )}
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
          {practice && song?.contentId === contentId && saveNoteAnchor && (
            <details className="paper-note-editor">
              <summary>音符在纸谱上的位置 · {attachment.noteAnchors?.length ?? 0} 处</summary>
              <p className="parameter-help">
                先选实际演奏小节和音符，再在纸谱上框选它的位置。同一音高在反复段出现多次时，请逐次标记。
              </p>
              <p className="parameter-help">当前有效位置 {noteCoverage.marked.size} / {noteCoverage.notes.length} 个演奏音符。小节网格或谱页变化后，旧位置需重新核对。</p>
              <div className="paper-note-editor-controls">
                <label>
                  演奏小节
                  <input aria-label="音符位置演奏小节" type="number" min={1} max={practice.measures}
                    value={noteMeasure} disabled={disabled || selectingNoteRegion}
                    onChange={(event) => { setNoteMeasure(Number(event.target.value)); setNoteKey(""); setNoteRegion(null); }} />
                </label>
                <label>
                  目标音符
                  <select aria-label="纸谱位置目标音" value={selectedNote ? `${selectedNote.track}:${selectedNote.index}` : ""}
                    disabled={disabled || selectingNoteRegion || !notesForMeasure.length}
                    onChange={(event) => { setNoteKey(event.target.value); setNoteRegion(null); }}>
                    {!notesForMeasure.length && <option value="">这一小节没有可标记的音符</option>}
                    {notesForMeasure.map((note) => (
                      <option key={`${note.track}:${note.index}`} value={`${note.track}:${note.index}`}>
                        {noteCoverage.marked.has(`${note.track}:${note.index}`) ? "已标记 · " : "未标记 · "}{pitchName(note.pitch)} · {song.tracks.find((track) => track.id === note.track)?.name ?? `音轨 ${note.track}`} · 第 {((note.tick - measureForNote!.startTick) / song.ppq + 1).toFixed(2)} 拍
                      </option>
                    ))}
                  </select>
                </label>
                <button type="button" disabled={disabled || selectingNoteRegion || !unmarkedNote}
                  onClick={() => {
                    if (!unmarkedNote) return;
                    setNoteMeasure(unmarkedNote.measure);
                    setNoteKey(`${unmarkedNote.note.track}:${unmarkedNote.note.index}`);
                    setNoteRegion(null);
                  }}>下一处未标记音符</button>
                <span>当前第 {view.page} 页{noteRegion && noteRegionPage === view.page ? " · 已框选" : ""}</span>
                <button disabled={disabled || !selectedNote || !count || loading || annotationEditing || selectingRegion || selectingNoteRegion}
                  onClick={() => setSelectingNoteRegion(true)}>框选音符位置</button>
                {selectingNoteRegion && <>
                  <span role="status">请在谱页上拖选这个音符所在的小区域。</span>
                  <button onClick={() => setSelectingNoteRegion(false)}>取消框选</button>
                </>}
                {noteRegion && !selectingNoteRegion && <button disabled={disabled} onClick={() => setNoteRegion(null)}>清除本次框选</button>}
                <button disabled={disabled || !selectedNote || !noteRegion || noteRegionPage !== view.page || !currentAsset?.available || selectingNoteRegion}
                  onClick={() => {
                    if (!selectedNote || !noteRegion || !currentAsset) return;
                    void saveNoteAnchor(selectedNote.track, selectedNote.index, {
                      pitch: selectedNote.pitch, measure: noteMeasure, assetId: currentAsset.id,
                      page: view.page, region: noteRegion,
                    }, savedNoteAnchor ?? null);
                  }}>
                  {savedNoteAnchor ? "更新音符位置" : "保存音符位置"}
                </button>
              </div>
              {savedNoteAnchor && <div className="paper-note-anchor-row">
                <span>{pitchName(savedNoteAnchor.pitch)} 已标在第 {savedNoteAnchor.page} 页{savedNoteAnchor.grid !== practice.grid ? " · 小节网格已变化" : ""}</span>
                <button disabled={disabled || selectingNoteRegion} onClick={() => {
                  setNoteRegion(savedNoteAnchor.region); setNoteRegionPage(savedNoteAnchor.page);
                  page(savedNoteAnchor.page); pendingAnnotation.current = { page: savedNoteAnchor.page, region: savedNoteAnchor.region };
                }}>查看 / 修改位置</button>
                <button disabled={disabled || selectingNoteRegion} onClick={() =>
                  void saveNoteAnchor(savedNoteAnchor.track, savedNoteAnchor.index, null, savedNoteAnchor)
                }>移除音符位置</button>
              </div>}
              {!notesForMeasure.length && <p className="parameter-help">请选有演奏音符的小节。</p>}
            </details>
          )}
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
      {bookBusy && <button type="button" onClick={() => { exportCancelled.current = true; setExportNotice("正在停止整本导出…"); }}>取消整本导出</button>}
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
          regionSelection={selectingRegion || selectingNoteRegion}
          regionSelected={(region)=>{
            if (selectingNoteRegion) { setNoteRegion(region); setNoteRegionPage(view.page); setSelectingNoteRegion(false); }
            else { setAnchorRegion(region); setAnchorRegionPage(view.page); setSelectingRegion(false); }
          }}
          save={saveAnnotation}
          saveInk={saveInk?(expected,strokes)=>saveInk((attachment.format==="pdf"?attachment.pages[0]:attachment.pages[view.page-1])?.id??"",attachment.format==="pdf"?view.page:1,expected,strokes):undefined}
        >
          <canvas
            ref={canvas}
            aria-label={`谱面第 ${view.page} 页`}
            hidden={!!error || needsPassword}
          />
          {mapping?.anchors.filter(a=>a.page===view.page&&a.region).map(a=>{
            const r=paperAnnotationBox(a.region!,view.rotation),historical=!!highlightedHistory&&!highlightedHistory.note&&a.measure===highlightedHistory.measure&&a.page===highlightedHistory.page,active=!stale&&!historyFocus&&mapping.anchors.slice().reverse().find(n=>n.measure<=measured)?.measure===a.measure;
            return <button key={`anchor-${a.measure}`} className={`paper-measure-region ${historical?"history-target":""} ${active?"active":""}`} style={{left:`${r.x*100}%`,top:`${r.y*100}%`,width:`${r.width*100}%`,height:`${r.height*100}%`}} aria-label={historical?`历史参考音 ${pitchName(historyFocus!.pitch)} 所在第 ${a.measure} 小节区域`:`谱页第 ${a.measure} 小节位置`} title={historical?"历史问题音所在小节区域，不代表音符精确位置":`定位第 ${a.measure} 小节`} disabled={disabled||readOnly||!practice||stale||annotationEditing||selectingRegion||a.measure>practice.measures} onClick={e=>{e.stopPropagation();void jumpAnchor(a);}}><span>{a.measure}</span></button>;
          })}
          {attachment.noteAnchors?.filter((anchor) => anchor.page === view.page &&
            anchor.grid === practice?.grid && anchor.assetId === currentAsset?.id).map((anchor) => {
            const r = paperAnnotationBox(anchor.region, view.rotation);
            const historical = !!highlightedHistory?.note && highlightedHistory.note.track === anchor.track &&
              highlightedHistory.note.index === anchor.index;
            return <button key={`note-${anchor.track}-${anchor.index}`}
              className={`paper-note-region ${historical ? "history-target" : ""}`}
              style={{left:`${r.x*100}%`,top:`${r.y*100}%`,width:`${r.width*100}%`,height:`${r.height*100}%`}}
              aria-label={historical ? `历史参考音 ${pitchName(anchor.pitch)} 的纸谱位置` : `${pitchName(anchor.pitch)} 在第 ${anchor.measure} 小节的纸谱位置`}
              title={`${pitchName(anchor.pitch)} · 第 ${anchor.measure} 小节 · 音轨 ${anchor.track}`}
              disabled={disabled || readOnly || annotationEditing || selectingRegion || selectingNoteRegion}
              onClick={(event) => { event.stopPropagation(); setNoteMeasure(anchor.measure); setNoteKey(`${anchor.track}:${anchor.index}`); setNoteRegion(anchor.region); setNoteRegionPage(anchor.page); }}>
              <span>{pitchName(anchor.pitch)}</span>
            </button>;
          })}
          {anchorRegion&&anchorRegionPage===view.page&&(()=>{const r=paperAnnotationBox(anchorRegion,view.rotation);return <span className="paper-measure-region draft" style={{left:`${r.x*100}%`,top:`${r.y*100}%`,width:`${r.width*100}%`,height:`${r.height*100}%`}}>第 {anchorMeasure} 小节位置</span>})()}
          {noteRegion && noteRegionPage === view.page && selectedNote && (()=>{const r=paperAnnotationBox(noteRegion,view.rotation);return <span className="paper-note-region draft" style={{left:`${r.x*100}%`,top:`${r.y*100}%`,width:`${r.width*100}%`,height:`${r.height*100}%`}}>{pitchName(selectedNote.pitch)}</span>})()}
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
