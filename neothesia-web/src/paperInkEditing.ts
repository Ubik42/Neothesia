import type { PaperAnnotation } from "./PaperAnnotations";

/** Translate a group in unrotated page coordinates, keeping its relative spacing. */
export function moveInk(
  rows: PaperAnnotation[],
  ids: string[],
  dx: number,
  dy: number,
): PaperAnnotation[] {
  const selected = new Set(ids);
  const group = rows.filter((n) => selected.has(n.id) && n.ink);
  if (!group.length || !Number.isFinite(dx) || !Number.isFinite(dy))
    return rows;
  const x = Math.max(
    -Math.min(...group.map((n) => n.x)),
    Math.min(dx, 1 - Math.max(...group.map((n) => n.x + (n.width ?? 0)))),
  );
  const y = Math.max(
    -Math.min(...group.map((n) => n.y)),
    Math.min(dy, 1 - Math.max(...group.map((n) => n.y + (n.height ?? 0)))),
  );
  if (Math.abs(x) < 1e-10 && Math.abs(y) < 1e-10) return rows;
  return rows.map((n) =>
    selected.has(n.id) && n.ink
      ? {
          ...n,
          x: Math.max(0, Math.min(1 - n.width!, n.x + x)),
          y: Math.max(0, Math.min(1 - n.height!, n.y + y)),
        }
      : n,
  );
}

type InkPoint = [number, number];
type Interval = [number, number];
// Return the part of segment p→q inside a round-ended eraser sweep a→b.
function sweptIntervals(
  p: InkPoint,
  q: InkPoint,
  a: InkPoint,
  b: InkPoint,
  radius: number,
): Interval[] {
  const dx = q[0] - p[0],
    dy = q[1] - p[1],
    length2 = dx * dx + dy * dy;
  const intervals: Interval[] = [];
  const circle = (c: InkPoint) => {
    const x = p[0] - c[0],
      y = p[1] - c[1];
    if (length2 < 1e-16) {
      if (x * x + y * y < radius * radius) intervals.push([0, 1]);
      return;
    }
    const dot = x * dx + y * dy,
      determinant = dot * dot - length2 * (x * x + y * y - radius * radius);
    if (determinant <= 0) return;
    const root = Math.sqrt(determinant),
      lo = Math.max(0, (-dot - root) / length2),
      hi = Math.min(1, (-dot + root) / length2);
    if (hi - lo > 1e-9) intervals.push([lo, hi]);
  };
  circle(a);
  circle(b);
  const ex = b[0] - a[0],
    ey = b[1] - a[1],
    length = Math.hypot(ex, ey);
  if (length > 1e-8) {
    const ux = ex / length,
      uy = ey / length;
    let lo = 0,
      hi = 1;
    const clip = (start: number, delta: number, min: number, max: number) => {
      if (Math.abs(delta) < 1e-12) {
        if (start < min || start > max) {
          lo = 1;
          hi = 0;
        }
        return;
      }
      const c = (min - start) / delta,
        d = (max - start) / delta;
      lo = Math.max(lo, Math.min(c, d));
      hi = Math.min(hi, Math.max(c, d));
    };
    clip((p[0] - a[0]) * ux + (p[1] - a[1]) * uy, dx * ux + dy * uy, 0, length);
    clip(
      -(p[0] - a[0]) * uy + (p[1] - a[1]) * ux,
      -dx * uy + dy * ux,
      -radius,
      radius,
    );
    if (hi - lo > 1e-9) intervals.push([lo, hi]);
  }
  intervals.sort((a, b) => a[0] - b[0]);
  const merged: Interval[] = [];
  for (const interval of intervals) {
    const last = merged.at(-1);
    if (last && interval[0] <= last[1] + 1e-9)
      last[1] = Math.max(last[1], interval[1]);
    else merged.push([...interval]);
  }
  return merged;
}

/** The unrotated page dimensions are in displayed pixels, so the eraser is circular at every rotation and zoom. */
export function eraseInk(
  rows: PaperAnnotation[],
  from: InkPoint,
  to: InkPoint,
  pageWidth: number,
  pageHeight: number,
  radius: number,
  mode: "whole" | "partial",
  pointBudget = 200000,
  annotationBudget = 2000,
): PaperAnnotation[] {
  if (pageWidth <= 0 || pageHeight <= 0 || radius <= 0) return rows;
  const a: InkPoint = [from[0] * pageWidth, from[1] * pageHeight],
    b: InkPoint = [to[0] * pageWidth, to[1] * pageHeight];
  const output: PaperAnnotation[] = [];
  for (const n of rows) {
    if (!n.ink) {
      output.push(n);
      continue;
    }
    const reach = radius + (n.ink.thickness * pageWidth) / 2;
    if (
      (n.x + n.width!) * pageWidth < Math.min(a[0], b[0]) - reach ||
      n.x * pageWidth > Math.max(a[0], b[0]) + reach ||
      (n.y + n.height!) * pageHeight < Math.min(a[1], b[1]) - reach ||
      n.y * pageHeight > Math.max(a[1], b[1]) + reach
    ) {
      output.push(n);
      continue;
    }
    const points = n.ink.points.map((p): InkPoint => [
      (n.x + p[0] * n.width!) * pageWidth,
      (n.y + p[1] * n.height!) * pageHeight,
    ]);
    const fragments: InkPoint[][] = [];
    let fragment: InkPoint[] = [];
    let hit = false;
    const flush = () => {
      if (fragment.length >= 2) fragments.push(fragment);
      fragment = [];
    };
    for (let i = 1; i < points.length; i++) {
      const p = points[i - 1],
        q = points[i];
      const cuts = sweptIntervals(
        p,
        q,
        a,
        b,
        radius + (n.ink.thickness * pageWidth) / 2,
      );
      if (cuts.length) hit = true;
      if (hit && mode === "whole") break;
      const at = (t: number): InkPoint => [
        p[0] + (q[0] - p[0]) * t,
        p[1] + (q[1] - p[1]) * t,
      ];
      let start = 0;
      for (const [lo, hi] of cuts) {
        if (lo > start + 1e-9) {
          if (!fragment.length) fragment.push(at(start));
          fragment.push(at(lo));
        }
        flush();
        start = hi;
      }
      if (start < 1 - 1e-9) {
        if (!fragment.length) fragment.push(at(start));
        fragment.push(q);
      }
      if (fragment.length >= 4096) {
        const end = fragment.at(-1)!;
        flush();
        fragment = [end];
      }
      if (fragments.length > 128)
        throw new Error(
          "这次擦除产生的片段过多。当前擦除已取消，请缩短擦除范围。",
        );
    }
    flush();
    if (fragments.length > 128)
      throw new Error(
        "这次擦除产生的片段过多。当前擦除已取消，请缩短擦除范围。",
      );
    if (!hit) {
      output.push(n);
      continue;
    }
    if (mode === "whole") continue;
    for (const ps of fragments) {
      const normalized = ps.map((p): InkPoint => [
        p[0] / pageWidth,
        p[1] / pageHeight,
      ]);
      const minX = Math.min(...normalized.map((p) => p[0])),
        maxX = Math.max(...normalized.map((p) => p[0]));
      const minY = Math.min(...normalized.map((p) => p[1])),
        maxY = Math.max(...normalized.map((p) => p[1]));
      const w = Math.max(0.002, maxX - minX),
        h = Math.max(0.002, maxY - minY),
        x = Math.max(0, Math.min(minX, 1 - w)),
        y = Math.max(0, Math.min(minY, 1 - h));
      output.push({
        ...n,
        id: `ink-${crypto.randomUUID()}`,
        x,
        y,
        width: w,
        height: h,
        ink: {
          ...n.ink,
          points: normalized.map((p) => [
            Math.max(0, Math.min(1, (p[0] - x) / w)),
            Math.max(0, Math.min(1, (p[1] - y) / h)),
          ]),
        },
      });
    }
    if (output.length > annotationBudget)
      throw new Error(
        "这次擦除会超过整份谱面 2000 条批注的容量。当前擦除已取消，原草稿保留。",
      );
  }
  if (output.length > annotationBudget)
    throw new Error(
      "这次擦除会超过整份谱面 2000 条批注的容量。当前擦除已取消，原草稿保留。",
    );
  if (
    output.reduce((count, n) => count + (n.ink?.points.length ?? 0), 0) >
    pointBudget
  )
    throw new Error(
      "这次擦除会超过整份谱面 20 万点的容量。当前擦除已取消，原草稿保留。",
    );
  return output;
}
