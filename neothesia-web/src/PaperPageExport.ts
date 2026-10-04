import { paperAnnotationBox } from "./PaperGeometry";
import type { PaperAnnotation } from "./PaperAnnotations";

const colors: Record<string, string> = {
  yellow: "#d6ab32",
  blue: "#3679b9",
  green: "#35845d",
  red: "#ba5050",
};
const rotate = (x: number, y: number, r: number): [number, number] =>
  r === 90
    ? [1 - y, x]
    : r === 180
      ? [1 - x, 1 - y]
      : r === 270
        ? [y, 1 - x]
        : [x, y];

/** Compose an independent full-page snapshot with complete, numbered remarks. */
export async function annotatedPagePng(
  source: HTMLCanvasElement,
  notes: PaperAnnotation[],
  name: string,
  page: number,
  rotation: number,
): Promise<Blob> {
  const width = Math.max(1200, source.width + 100),
    margin = 50,
    header = 120,
    lineHeight = 42;
  const probe = document.createElement("canvas"),
    ctx = probe.getContext("2d");
  if (!ctx) throw new Error("无法创建导出画布");
  const font = '28px "Microsoft YaHei", "Noto Sans CJK SC", sans-serif';
  ctx.font = font;
  const wrap = (text: string, maxWidth: number) => {
    const lines: string[] = [];
    for (const paragraph of text.replace(/\t/g, "    ").split("\n")) {
      let line = "";
      for (const c of paragraph) {
        if (line && ctx.measureText(line + c).width > maxWidth) {
          lines.push(line);
          line = c;
        } else line += c;
      }
      lines.push(line);
    }
    return lines;
  };
  const title = wrap(name, width - 2 * margin - 230)[0];
  const entries = notes
    .filter((n) => !n.ink || n.text.trim())
    .map((n) => ({
      note: n,
      lines: wrap(
        n.text || (n.ink ? "手写标记" : n.width ? "高亮区域" : ""),
        width - 2 * margin - 75,
      ),
    }));
  const legendHeight = entries.length
    ? 95 + entries.reduce((h, e) => h + e.lines.length * lineHeight + 30, 0)
    : 70;
  const height = header + source.height + legendHeight + margin;
  if (height > 16000 || width * height > 32_000_000)
    throw new Error(
      "本页批注较多，无法合并为一张图片。可通过曲目包完整分享批注，或分散到其他谱页后导出。",
    );
  const output = document.createElement("canvas");
  output.width = width;
  output.height = height;
  const out = output.getContext("2d");
  if (!out) throw new Error("无法创建导出画布");
  out.fillStyle = "#fff";
  out.fillRect(0, 0, width, height);
  out.font = font;
  out.textBaseline = "top";
  out.fillStyle = "#24272d";
  out.fillText(title, margin, 38);
  out.fillText(`第 ${page} 页`, width - margin - 180, 38);
  out.fillStyle = "#626873";
  out.font = '20px "Microsoft YaHei", sans-serif';
  out.fillText(`个人批注 · ${notes.length} 条`, margin, 82);
  const paperMargin = (width - source.width) / 2;
  out.drawImage(source, paperMargin, header);
  const circle = (x: number, y: number, index: number, color: string) => {
    out.beginPath();
    out.arc(x, y, 21, 0, 2 * Math.PI);
    out.fillStyle = colors[color] ?? colors.yellow;
    out.fill();
    out.strokeStyle = "#fff";
    out.lineWidth = 3;
    out.stroke();
    out.fillStyle = "#fff";
    out.font = 'bold 23px "Microsoft YaHei", sans-serif';
    out.textAlign = "center";
    out.textBaseline = "middle";
    out.fillText(String(index + 1), x, y);
    out.textAlign = "left";
    out.textBaseline = "top";
  };
  for (const e of entries) {
    if (!e.note.width || e.note.ink) continue;
    const r = paperAnnotationBox(e.note, rotation);
    out.save();
    out.fillStyle = colors[e.note.color] ?? colors.yellow;
    out.globalAlpha = 0.25;
    out.fillRect(
      paperMargin + r.x * source.width,
      header + r.y * source.height,
      r.width * source.width,
      r.height * source.height,
    );
    out.globalAlpha = 0.8;
    out.strokeStyle = colors[e.note.color] ?? colors.yellow;
    out.lineWidth = 2;
    out.strokeRect(
      paperMargin + r.x * source.width,
      header + r.y * source.height,
      r.width * source.width,
      r.height * source.height,
    );
    out.restore();
  }
  for (const n of notes) {
    if (!n.ink) continue;
    out.save();
    out.strokeStyle = colors[n.color] ?? colors.red;
    out.lineWidth =
      n.ink.thickness * (rotation % 180 === 0 ? source.width : source.height);
    out.lineCap = "round";
    out.lineJoin = "round";
    out.beginPath();
    n.ink.points.forEach((p, i) => {
      const [x, y] = rotate(
        n.x + p[0] * n.width!,
        n.y + p[1] * n.height!,
        rotation,
      );
      if (i === 0)
        out.moveTo(paperMargin + x * source.width, header + y * source.height);
      else
        out.lineTo(paperMargin + x * source.width, header + y * source.height);
    });
    out.stroke();
    out.restore();
  }
  entries.forEach((e, i) => {
    const [x, y] = rotate(e.note.x, e.note.y, rotation);
    circle(
      paperMargin + x * source.width,
      header + y * source.height,
      i,
      e.note.color,
    );
  });
  let y = header + source.height + 38;
  out.fillStyle = "#24272d";
  out.font = 'bold 28px "Microsoft YaHei", sans-serif';
  out.fillText(
    entries.length
      ? "批注说明"
      : notes.some((n) => n.ink)
        ? "本页手写标记已绘入谱面"
        : "本页没有已保存的批注",
    margin,
    y,
  );
  y += 58;
  entries.forEach((e, i) => {
    circle(margin + 22, y + 18, i, e.note.color);
    out.font = font;
    out.fillStyle = "#24272d";
    for (const line of e.lines) {
      out.fillText(line, margin + 75, y);
      y += lineHeight;
    }
    y += 30;
  });
  const blob = await new Promise<Blob>((resolve, reject) =>
    output.toBlob(
      (b) => (b ? resolve(b) : reject(new Error("图片生成失败，请稍后重试"))),
      "image/png",
    ),
  );
  if (blob.size > 48_000_000)
    throw new Error("导出图片超过 48 MB，请减少本页批注或使用曲目包分享");
  return blob;
}
