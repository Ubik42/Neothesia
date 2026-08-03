import fs from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import { pathToFileURL } from "node:url";

function argumentsByName(values) {
  const parsed = new Map();
  for (let index = 0; index < values.length; index += 2) {
    const name = values[index];
    const value = values[index + 1];
    if (!name?.startsWith("--") || value == null || parsed.has(name)) {
      throw new Error("Expected unique --package-root, --source and --output arguments.");
    }
    parsed.set(name, value);
  }
  return parsed;
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

const args = argumentsByName(process.argv.slice(2));
const packageRoot = args.get("--package-root");
const sourcePath = args.get("--source");
const outputRoot = args.get("--output");
if (!packageRoot || !sourcePath || !outputRoot || args.size !== 3) {
  throw new Error("Expected --package-root, --source and --output exactly once.");
}
if (fs.existsSync(outputRoot)) {
  throw new Error(`Output directory already exists: ${outputRoot}`);
}
fs.mkdirSync(outputRoot);

const modulePath = path.join(packageRoot, "dist", "verovio-module.mjs");
const toolkitPath = path.join(packageRoot, "dist", "verovio.mjs");
const { default: createVerovioModule } = await import(pathToFileURL(modulePath));
const { VerovioToolkit } = await import(pathToFileURL(toolkitPath));
const module = await createVerovioModule();
const toolkit = new VerovioToolkit(module);

try {
  toolkit.setOptions({
    breaks: "auto",
    pageHeight: 2970,
    pageWidth: 2100,
    scale: 40,
    svgHtml5: true,
    svgViewBox: true,
    xmlIdChecksum: true,
  });
  const source = fs.readFileSync(sourcePath);
  const loaded = path.extname(sourcePath).toLowerCase() === ".mxl"
    ? toolkit.loadZipDataBuffer(source.buffer.slice(source.byteOffset, source.byteOffset + source.byteLength))
    : toolkit.loadData(source.toString("utf8"));
  if (!loaded) throw new Error("Verovio rejected the score.");

  const pageCount = toolkit.getPageCount();
  if (pageCount < 1) throw new Error("Verovio produced no pages.");
  const pages = [];
  const notePage = new Map();
  for (let page = 1; page <= pageCount; page += 1) {
    const svg = toolkit.renderToSVG(page);
    const svgFile = `page-${page}.svg`;
    const svgBytes = Buffer.byteLength(svg);
    const tags = [...svg.matchAll(/<g\b[^>]*\bclass="[^"]*\bnote\b[^"]*"[^>]*>/g)];
    for (const tag of tags) {
      const id = tag[0].match(/\bid="([^"]+)"/)?.[1];
      if (id) notePage.set(id, page - 1);
    }
    fs.writeFileSync(path.join(outputRoot, svgFile), svg);
    pages.push({
      pageIndex: page - 1,
      svgFile,
      svgBytes,
      svgSha256: sha256(svg),
    });
  }

  toolkit.renderToMIDI();
  const notes = [];
  for (const [rendererId, pageIndex] of notePage) {
    const values = toolkit.getMIDIValuesForElement(rendererId);
    if (![values.time, values.pitch, values.duration].every(Number.isFinite)) continue;
    notes.push({
      rendererId,
      pageIndex,
      onsetMillis: values.time,
      pitch: values.pitch,
      durationMillis: values.duration,
    });
  }

  const manifest = {
    schemaVersion: 2,
    rendererName: "verovio",
    rendererVersion: "6.1.0",
    sourceSha256: sha256(source),
    sourceBytes: source.length,
    pageCount,
    pages,
    notes,
  };
  const temporaryManifest = path.join(outputRoot, "manifest.json.partial");
  fs.writeFileSync(temporaryManifest, `${JSON.stringify(manifest, null, 2)}\n`);
  fs.renameSync(temporaryManifest, path.join(outputRoot, "manifest.json"));
  process.stdout.write(`${JSON.stringify({ pageCount, noteCount: notes.length })}\n`);
} finally {
  toolkit.destroy();
}
