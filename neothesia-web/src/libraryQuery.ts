import type { CollectionSong, SongRow } from "./api";
export type Metadata = {
  title: string | null;
  composer: string | null;
  artist: string | null;
  collection: string | null;
  difficulty: string | null;
  tags: string[];
  notes: string | null;
};
export type Entry = {
  path: string;
  contentId: string | null;
  metadata: Metadata;
  groups: string[];
  rating: number;
  hasScore: boolean;
  notationCount?: number;
  paperCount?: number;
  paperNames?: string[];
  scoreName: string | null;
  notes: number;
  duration: number;
  tracks: number;
  measures: number;
  meter: string;
  available: boolean;
  error: string | null;
  size: number;
};
export const key = (p: string) => p.replaceAll("\\", "/").toLowerCase();
export const fields = [
  "all",
  "title",
  "composer",
  "artist",
  "collection",
  "tags",
  "difficulty",
  "notes",
  "path",
  "source",
] as const;
export type Field = (typeof fields)[number];
export type View = {
  query: string;
  field: Field;
  filter: string;
  sort: string;
  direction: "asc" | "desc";
  minRating: number;
  minDuration: string;
  maxDuration: string;
  practice: string;
  learning: string;
};
export const defaults: View = {
  query: "",
  field: "all",
  filter: "all",
  sort: "title",
  direction: "asc",
  minRating: 0,
  minDuration: "",
  maxDuration: "",
  practice: "all",
  learning: "all",
};
export const storage = "neothesia-library-view-v1";
export function readView(): View {
  try {
    const v = JSON.parse(localStorage.getItem(storage) ?? "null");
    if (!v || typeof v !== "object") return { ...defaults };
    return {
      query: typeof v.query === "string" ? v.query.slice(0, 4096) : "",
      field: fields.includes(v.field) ? v.field : "all",
      filter: [
        "all",
        "score",
        "missing",
        "duplicates",
        "unindexed",
        "errors",
      ].includes(v.filter)
        ? v.filter
        : "all",
      sort: [
        "title",
        "composer",
        "difficulty",
        "rating",
        "duration",
        "lastUsed",
        "sessions",
        "measures",
        "notes",
        "path",
        "learningStage",
        "learningDue",
      ].includes(v.sort)
        ? v.sort
        : "title",
      direction: v.direction === "desc" ? "desc" : "asc",
      minRating:
        Number.isInteger(v.minRating) && v.minRating >= 0 && v.minRating <= 5
          ? v.minRating
          : 0,
      minDuration:
        typeof v.minDuration === "string" &&
        /^(\d+(\.\d+)?)?$/.test(v.minDuration)
          ? v.minDuration
          : "",
      maxDuration:
        typeof v.maxDuration === "string" &&
        /^(\d+(\.\d+)?)?$/.test(v.maxDuration)
          ? v.maxDuration
          : "",
      learning: [
        "all",
        "untracked",
        "planned",
        "learning",
        "polishing",
        "maintaining",
        "mastered",
        "paused",
        "overdue",
        "today",
      ].includes(v.learning)
        ? v.learning
        : "all",
      practice: ["all", "played", "unplayed", "favorite", "review"].includes(
        v.practice,
      )
        ? v.practice
        : "all",
    };
  } catch {
    return { ...defaults };
  }
}
export function terms(q: string) {
  return [...q.matchAll(/(-?)(?:"([^"]+)"|([^\s"]+))/g)].map((m) => ({
    text: (m[2] ?? m[3]).toLocaleLowerCase(),
    exclude: m[1] === "-",
  }));
}
export function matches(
  r: SongRow,
  e: Entry | undefined,
  field: Field,
  tokens: ReturnType<typeof terms>,
) {
  const m = e?.metadata,
    f = {
      title: m?.title ?? r.title,
      composer: m?.composer ?? r.composer,
      artist: m?.artist ?? "",
      collection: m?.collection ?? "",
      tags: (m?.tags ?? r.tags ?? []).join(" "),
      difficulty: m?.difficulty ?? r.difficulty ?? "",
      notes: m?.notes ?? "",
      path: r.path,
      source: `${r.source} ${r.category}`,
    };
  const text = (
    field === "all" ? Object.values(f).join(" ") : f[field]
  ).toLocaleLowerCase();
  return tokens.every((t) =>
    t.exclude ? !text.includes(t.text) : text.includes(t.text),
  );
}
export function compare(
  a: SongRow,
  b: SongRow,
  x: Entry | undefined,
  y: Entry | undefined,
  h: CollectionSong | undefined,
  j: CollectionSong | undefined,
  sort: string,
  direction: "asc" | "desc",
) {
  const value = (
    r: SongRow,
    e: Entry | undefined,
    c: CollectionSong | undefined,
  ): string | number | null => {
    switch (sort) {
      case "composer":
        return e?.metadata.composer ?? r.composer;
      case "difficulty":
        return e?.metadata.difficulty ?? r.difficulty ?? null;
      case "rating":
        return e?.rating ?? r.rating ?? 0;
      case "duration":
        return e?.contentId ? e.duration : null;
      case "measures":
        return e?.contentId ? e.measures : null;
      case "notes":
        return e?.contentId ? e.notes : null;
      case "lastUsed":
        return c?.lastPracticed || null;
      case "sessions":
        return c?.sessions ?? 0;
      case "path":
        return r.path;
      default:
        return e?.metadata.title ?? r.title;
    }
  };
  const left = value(a, x, h),
    right = value(b, y, j);
  if (left == null || right == null) {
    if (left !== right) return left == null ? 1 : -1;
  } else {
    const n =
      typeof left === "number" && typeof right === "number"
        ? left - right
        : String(left).localeCompare(String(right), "zh", { numeric: true });
    if (n) return direction === "asc" ? n : -n;
  }
  return (
    (x?.metadata.title ?? a.title).localeCompare(
      y?.metadata.title ?? b.title,
      "zh",
      { numeric: true },
    ) || key(a.path).localeCompare(key(b.path))
  );
}
