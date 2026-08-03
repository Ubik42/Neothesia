import fs from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";
import { createHash } from "node:crypto";

const packageRoot = process.env.VEROVIO_PACKAGE_ROOT;
if (!packageRoot) {
  throw new Error("Set VEROVIO_PACKAGE_ROOT to the installed verovio package directory.");
}
const modulePath = path.join(packageRoot, "dist", "verovio-module.mjs");
const toolkitPath = path.join(packageRoot, "dist", "verovio.mjs");
const { default: createVerovioModule } = await import(pathToFileURL(modulePath));
const { VerovioToolkit } = await import(pathToFileURL(toolkitPath));

const files = process.argv.slice(2);
if (files.length === 0) {
  throw new Error("Pass one or more MusicXML, XML, or MXL files.");
}

const moduleStarted = performance.now();
const module = await createVerovioModule();
const moduleInitMs = performance.now() - moduleStarted;
const results = [];
const outputRoot = process.env.VEROVIO_SVG_OUTPUT_ROOT;
if (outputRoot) fs.mkdirSync(outputRoot, { recursive: true });

function percentile(values, fraction) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor((sorted.length - 1) * fraction)];
}

function safeStem(file) {
  return path.basename(file, path.extname(file)).replace(/[^a-zA-Z0-9._-]+/g, "-");
}

for (const file of files) {
  const toolkit = new VerovioToolkit(module);
  toolkit.setOptions({
    breaks: "auto",
    pageHeight: 2970,
    pageWidth: 2100,
    scale: 40,
    svgHtml5: true,
    svgViewBox: true,
    xmlIdChecksum: true,
  });
  const bytes = fs.readFileSync(file);
  const loadStarted = performance.now();
  const loaded = path.extname(file).toLowerCase() === ".mxl"
    ? toolkit.loadZipDataBuffer(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength))
    : toolkit.loadData(bytes.toString("utf8"));
  const loadMs = performance.now() - loadStarted;
  if (!loaded) {
    results.push({ file: path.basename(file), loaded: false, loadMs });
    toolkit.destroy();
    continue;
  }

  const pageCount = toolkit.getPageCount();
  const renderStarted = performance.now();
  const pages = [];
  let svgBytes = 0;
  let noteIds = [];
  const pageByNoteId = new Map();
  const manifestPages = [];
  for (let page = 1; page <= pageCount; page += 1) {
    const started = performance.now();
    const svg = toolkit.renderToSVG(page);
    const renderMs = performance.now() - started;
    const ids = [...svg.matchAll(/<g\b[^>]*\bclass="[^"]*\bnote\b[^"]*"[^>]*>/g)]
      .map((match) => match[0].match(/\bid="([^"]+)"/))
      .filter(Boolean)
      .map((match) => match[1]);
    noteIds = noteIds.concat(ids);
    for (const id of ids) pageByNoteId.set(id, page - 1);
    const bytes = Buffer.byteLength(svg);
    const svgFile = `${safeStem(file)}-page-${page}.svg`;
    if (outputRoot) {
      fs.writeFileSync(path.join(outputRoot, svgFile), svg);
    }
    manifestPages.push({
      pageIndex: page - 1,
      svgFile,
      svgBytes: bytes,
      svgSha256: createHash("sha256").update(svg).digest("hex"),
    });
    svgBytes += bytes;
    pages.push({ page, renderMs, svgBytes: bytes, noteIds: ids.length });
  }
  const renderMs = performance.now() - renderStarted;
  const midiStarted = performance.now();
  toolkit.renderToMIDI();
  const midiIndexMs = performance.now() - midiStarted;
  const samples = noteIds.slice(0, 100);
  const lookupStarted = performance.now();
  let timedNotes = 0;
  for (const id of samples) {
    const time = toolkit.getTimeForElement(id);
    toolkit.getElementAttr(id);
    if (Number.isFinite(time) && time >= 0) timedNotes += 1;
  }
  const lookupMs = performance.now() - lookupStarted;
  const semanticKeys = new Map();
  const manifestNotes = [];
  let midiValueNotes = 0;
  for (const id of noteIds) {
    const values = toolkit.getMIDIValuesForElement(id);
    if (![values.time, values.pitch, values.duration].every(Number.isFinite)) continue;
    midiValueNotes += 1;
    manifestNotes.push({
      rendererId: id,
      pageIndex: pageByNoteId.get(id),
      onsetMillis: values.time,
      pitch: values.pitch,
      durationMillis: values.duration,
    });
    const key = `${values.time}:${values.pitch}:${values.duration}`;
    semanticKeys.set(key, (semanticKeys.get(key) ?? 0) + 1);
  }
  const ambiguousSemanticGroups = [...semanticKeys.values()].filter((count) => count > 1);
  let manifestFile = null;
  if (outputRoot) {
    manifestFile = `${safeStem(file)}.manifest.json`;
    fs.writeFileSync(
      path.join(outputRoot, manifestFile),
      `${JSON.stringify({
        schemaVersion: 2,
        rendererName: "verovio",
        rendererVersion: "6.1.0",
        sourceSha256: createHash("sha256").update(bytes).digest("hex"),
        sourceBytes: bytes.length,
        pageCount,
        pages: manifestPages,
        notes: manifestNotes,
      }, null, 2)}\n`,
    );
  }
  results.push({
    file: path.basename(file),
    loaded: true,
    sourceBytes: bytes.length,
    loadMs,
    pageCount,
    renderMs,
    svgBytes,
    noteIds: noteIds.length,
    midiIndexMs,
    sampledNoteLookups: samples.length,
    timedNotes,
    lookupMs,
    lookupMeanMs: samples.length ? lookupMs / samples.length : null,
    midiValueNotes,
    uniqueSemanticKeys: semanticKeys.size,
    ambiguousSemanticGroups: ambiguousSemanticGroups.length,
    maximumSemanticMultiplicity: Math.max(0, ...ambiguousSemanticGroups),
    manifestFile,
    pages,
  });
  toolkit.destroy();
}

const loadedResults = results.filter((result) => result.loaded);
const report = {
  verovioVersion: "6.1.0",
  moduleInitMs,
  wasmBundleBytes: fs.statSync(modulePath).size,
  summary: {
    inputs: results.length,
    loaded: loadedResults.length,
    failed: results.length - loadedResults.length,
    loadMedianMs: percentile(loadedResults.map((result) => result.loadMs), 0.5),
    loadMaxMs: percentile(loadedResults.map((result) => result.loadMs), 1),
    renderMedianMs: percentile(loadedResults.map((result) => result.renderMs), 0.5),
    renderMaxMs: percentile(loadedResults.map((result) => result.renderMs), 1),
    lookupMeanMaxMs: percentile(
      loadedResults.map((result) => result.lookupMeanMs).filter(Number.isFinite),
      1,
    ),
    totalSvgBytes: loadedResults.reduce((total, result) => total + result.svgBytes, 0),
  },
  results,
};
const json = JSON.stringify(report, null, 2);
if (process.env.VEROVIO_REPORT_PATH) {
  fs.writeFileSync(process.env.VEROVIO_REPORT_PATH, `${json}\n`);
}
console.log(json);
