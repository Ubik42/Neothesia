import { useEffect, useRef, useState } from "react";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import type { PDFDocumentProxy } from "pdfjs-dist";
import { importKind } from "./scorePairing";

function cleanSvg(source: string) {
  const doc = new DOMParser().parseFromString(source, "image/svg+xml");
  doc
    .querySelectorAll("script,foreignObject,image,iframe")
    .forEach((e) => e.remove());
  doc.querySelectorAll("*").forEach((e) =>
    Array.from(e.attributes).forEach((a) => {
      if (
        a.name.toLowerCase().startsWith("on") ||
        (["href", "xlink:href"].includes(a.name) && !a.value.startsWith("#"))
      )
        e.removeAttribute(a.name);
    }),
  );
  return new XMLSerializer().serializeToString(doc.documentElement);
}
export function ScoreFilePreview({
  file,
  close,
}: {
  file: File;
  close: () => void;
}) {
  const [pages, setPages] = useState<string[]>([]),
    [pdf, setPdf] = useState<PDFDocumentProxy | null>(null);
  const [page, setPage] = useState(0),
    [url, setUrl] = useState(""),
    [error, setError] = useState("");
  const [ready, setReady] = useState(false),
    canvas = useRef<HTMLCanvasElement>(null);
  const kind = importKind(file.name);
  useEffect(() => {
    let disposed = false,
      worker: Worker | undefined,
      objectUrl = "";
    let task: import("pdfjs-dist").PDFDocumentLoadingTask | undefined;
    const load = async () => {
      if (file.size > (kind === "notation" ? 4_000_000 : 48_000_000))
        throw new Error("文件超过谱面容量上限");
      if (kind === "pdf") {
        const lib = await import("pdfjs-dist");
        if (disposed) return;
        lib.GlobalWorkerOptions.workerSrc = workerUrl;
        task = lib.getDocument({
          data: new Uint8Array(await file.arrayBuffer()),
          cMapUrl: "/pdfjs/cmaps/",
          cMapPacked: true,
          standardFontDataUrl: "/pdfjs/standard_fonts/",
          wasmUrl: "/pdfjs/wasm/",
        });
        const document = await task.promise;
        if (!disposed) setPdf(document);
      } else if (kind === "image") {
        objectUrl = URL.createObjectURL(file);
        const image = new Image();
        image.src = objectUrl;
        await image.decode();
        if (image.naturalWidth * image.naturalHeight > 64_000_000)
          throw new Error("图片尺寸过大，请缩小后导入");
        if (!disposed) {
          setUrl(objectUrl);
          setReady(true);
        }
      } else {
        const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
        if (disposed) return;
        worker = new Worker("/verovio/worker.mjs", { type: "module" });
        worker.onmessage = ({ data }) => {
          if (disposed) return;
          if (data.error) setError(data.error);
          else {
            setPages(data.pages.map(cleanSvg));
            setReady(true);
          }
        };
        worker.onerror = () =>
          !disposed && setError("乐谱排版失败，请检查文件");
        worker.postMessage({ bytes, compressed: /\.mxl$/i.test(file.name) });
      }
    };
    void load().catch((e) => !disposed && setError(String(e)));
    return () => {
      disposed = true;
      worker?.terminate();
      void task?.destroy();
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [file, kind]);
  useEffect(() => {
    if (!pdf) return;
    let disposed = false,
      task: import("pdfjs-dist").RenderTask | undefined;
    setReady(false);
    void pdf
      .getPage(page + 1)
      .then((p) => {
        if (disposed || !canvas.current) return;
        const base = p.getViewport({ scale: 1 });
        const view = p.getViewport({
          scale: Math.min(1100 / base.width, 1700 / base.height),
        });
        const el = canvas.current;
        el.width = view.width;
        el.height = view.height;
        task = p.render({ canvas: el, viewport: view });
        return task.promise;
      })
      .then(() => !disposed && setReady(true))
      .catch((e) => !disposed && setError(String(e)));
    return () => {
      disposed = true;
      task?.cancel();
    };
  }, [pdf, page]);
  const count = pdf?.numPages ?? (pages.length || 1);
  return (
    <section className="batch-file-preview" aria-label="待导入谱面预览">
      <div className="batch-preview-heading">
        <strong>{file.name}</strong>
        <button autoFocus onClick={close}>
          返回配对列表
        </button>
      </div>
      <div className="batch-preview-toolbar">
        <button disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
          上一页
        </button>
        <span>
          第 {page + 1} / {count} 页
        </span>
        <button
          disabled={page + 1 >= count}
          onClick={() => setPage((p) => p + 1)}
        >
          下一页
        </button>
      </div>
      {error ? (
        <p role="alert">{error}</p>
      ) : (
        <>
          {!ready && <p role="status">正在读取实际谱面…</p>}
          <div className="batch-preview-sheet">
            {kind === "pdf" ? (
              <canvas ref={canvas} aria-label="PDF 谱页" />
            ) : kind === "image" ? (
              url && <img src={url} alt={file.name} />
            ) : (
              pages[page] && (
                <div dangerouslySetInnerHTML={{ __html: pages[page] }} />
              )
            )}
          </div>
        </>
      )}
    </section>
  );
}
