import { annotatedPagePng } from "./PaperPageExport";
import type { PaperAnnotation } from "./PaperAnnotations";

export async function annotatedBookPdf({
  name,
  pages,
  rotation,
  render,
  annotations,
  progress,
  cancelled,
}: {
  name: string;
  pages: number;
  rotation: number;
  render: (page: number) => Promise<HTMLCanvasElement>;
  annotations: (page: number) => PaperAnnotation[];
  progress: (page: number, total: number) => void;
  cancelled: () => boolean;
}): Promise<Uint8Array> {
  if (!Number.isSafeInteger(pages) || pages < 1 || pages > 200)
    throw new Error("整本导出支持 1 至 200 页。请拆分较长的谱本后再导出。");
  const { PDFDocument } = await import("pdf-lib");
  const document = await PDFDocument.create();
  document.setTitle(`${name} · 个人批注`);
  for (let page = 1; page <= pages; page++) {
    if (cancelled()) throw new Error("用户已取消整本导出");
    progress(page, pages);
    const source = await render(page);
    const png = await annotatedPagePng(source, annotations(page), name, page, rotation);
    const image = await document.embedPng(await png.arrayBuffer());
    const width = 595;
    const height = (image.height * width) / image.width;
    const sheetHeight = 842;
    const slices = Math.ceil(height / sheetHeight);
    if (document.getPageCount() + slices > 500)
      throw new Error("批注说明较长，导出将超过 500 张打印页。请拆分谱本后再导出。");
    for (let slice = 0; slice < slices; slice++) {
      const sheet = document.addPage([width, sheetHeight]);
      sheet.drawImage(image, {
        x: 0,
        y: sheetHeight - height + slice * sheetHeight,
        width,
        height,
      });
    }
  }
  if (cancelled()) throw new Error("用户已取消整本导出");
  const bytes = await document.save();
  if (bytes.length > 64_000_000)
    throw new Error("整本批注 PDF 超过 64 MB。请拆分较长的谱本后导出。");
  return bytes;
}
