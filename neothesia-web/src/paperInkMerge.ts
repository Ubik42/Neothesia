import type { PaperAnnotation } from "./PaperAnnotations";
export type InkConflict = {
  id: string;
  local: PaperAnnotation | null;
  saved: PaperAnnotation | null;
};
export type InkReview = {
  current: PaperAnnotation[];
  merged: PaperAnnotation[];
  conflicts: InkConflict[];
};
function signature(v: unknown): string {
  const normalize = (x: unknown): unknown =>
    typeof x === "number"
      ? Math.round(x * 1e6) / 1e6
      : Array.isArray(x)
        ? x.map(normalize)
        : x && typeof x === "object"
          ? Object.fromEntries(
              Object.entries(x)
                .sort(([a], [b]) => a.localeCompare(b))
                .map(([k, v]) => [k, normalize(v)]),
            )
          : x;
  return JSON.stringify(normalize(v ?? null));
}
export function reviewInkMerge(
  expected: PaperAnnotation[],
  draft: PaperAnnotation[],
  current: PaperAnnotation[],
): InkReview {
  const before = new Map(expected.map((n) => [n.id, n])),
    local = new Map(draft.map((n) => [n.id, n])),
    saved = new Map(current.map((n) => [n.id, n])),
    ids = [...new Set([...saved.keys(), ...local.keys(), ...before.keys()])],
    merged: PaperAnnotation[] = [],
    conflicts: InkConflict[] = [];
  for (const id of ids) {
    const a = before.get(id) ?? null,
      b = local.get(id) ?? null,
      c = saved.get(id) ?? null;
    if (signature(a) === signature(b) || signature(b) === signature(c)) {
      if (c) merged.push(c);
    } else if (signature(a) === signature(c)) {
      if (b) merged.push(b);
    } else {
      conflicts.push({ id, local: b, saved: c });
      if (c) merged.push(c);
    }
  }
  return { current, merged, conflicts };
}
export function applyInkMerge(
  review: InkReview,
  choices: Record<string, "saved" | "local">,
): PaperAnnotation[] {
  const values = new Map(review.merged.map((n) => [n.id, n]));
  for (const c of review.conflicts) {
    const n = choices[c.id] === "local" ? c.local : c.saved;
    if (n) values.set(c.id, n);
    else values.delete(c.id);
  }
  return [...values.values()];
}
