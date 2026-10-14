export type DraftSource = { contentId: string; id: string; slot: number; revision?: string };

const prefix = "neothesia-multi-plan-draft:";
const unit = 1_000_000;
type SavedChoice = [string, string, number];

const object = (value: unknown): Record<string, unknown> | null =>
  value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;

const read = (key: string): Record<string, unknown> | null => {
  try {
    return object(JSON.parse(localStorage.getItem(key) || "null"));
  } catch {
    return null;
  }
};

export const draftScopeFor = (sources: DraftSource[]): string =>
  JSON.stringify(sources.map(({ contentId, id }) => [contentId, id]));

export const draftKeyFor = (scope: string): string => `${prefix}${scope}`;

export const choicesForStorage = (chosen: number[], sources: DraftSource[]): SavedChoice[] =>
  chosen.flatMap((value) => {
    const slot = Math.floor(value / unit);
    const reference = value - slot * unit;
    const source = sources.find((item) => item.slot === slot);
    return source && Number.isSafeInteger(reference) && reference >= 0 && reference < unit
      ? [[source.contentId, source.id, reference] as SavedChoice]
      : [];
  });

const currentChoices = (saved: unknown, sources: DraftSource[]): number[] =>
  Array.isArray(saved)
    ? saved.flatMap((value): number[] => {
      if (!Array.isArray(value) || value.length !== 3) return [];
      const [contentId, id, reference] = value;
      const source = sources.find((item) => item.contentId === contentId && item.id === id);
      const combined = source ? source.slot * unit + reference : NaN;
      return source && Number.isSafeInteger(reference) && reference >= 0 && reference < unit && Number.isSafeInteger(combined)
        ? [combined]
        : [];
    })
    : [];

const oldChoices = (saved: unknown, oldSources: unknown, sources: DraftSource[]): number[] => {
  if (!Array.isArray(saved) || !Array.isArray(oldSources)) return [];
  return saved.flatMap((value): number[] => {
    if (!Number.isSafeInteger(value) || value < 0) return [];
    const oldSlot = Math.floor(value / unit);
    const sourceRow = oldSources.find((item) =>
      Array.isArray(item) && item.length === 3 && item[2] === oldSlot);
    if (!sourceRow) return [];
    const source = sources.find((item) => item.contentId === sourceRow[0] && item.id === sourceRow[1]);
    const combined = source ? source.slot * unit + value % unit : NaN;
    return Number.isSafeInteger(combined) ? [combined] : [];
  });
};

export const readMultiProblemDraft = (sources: DraftSource[]): Record<string, unknown> => {
  const scope = draftScopeFor(sources);
  const saved = read(draftKeyFor(scope));
  if (saved) return { ...saved, chosen: currentChoices(saved.chosenRefs, sources) };

  let fallback: Record<string, unknown> | null = null;
  let fallbackSources: unknown = null;
  let exactSlots = false;
  try {
    for (let index = 0; index < localStorage.length; index++) {
      const key = localStorage.key(index);
      if (!key?.startsWith(prefix)) continue;
      let rows: unknown;
      try { rows = JSON.parse(key.slice(prefix.length)); } catch { continue; }
      if (!Array.isArray(rows) || rows.length !== sources.length ||
        !rows.every((row, position) => Array.isArray(row) && row.length === 3 &&
          row[0] === sources[position].contentId && row[1] === sources[position].id)) continue;
      const candidate = read(key);
      if (!candidate) continue;
      const isExact = rows.every((row, position) => row[2] === sources[position].slot);
      if (!fallback || (isExact && !exactSlots) || (!exactSlots && isExact === exactSlots &&
        (Array.isArray(candidate.chosen) ? candidate.chosen.length : 0) >
        (Array.isArray(fallback.chosen) ? fallback.chosen.length : 0))) {
        fallback = candidate;
        fallbackSources = rows;
        exactSlots = isExact;
      }
    }
  } catch {
    // A malformed old key does not invalidate a correctly saved draft.
  }
  return fallback
    ? { ...fallback, chosen: oldChoices(fallback.chosen, fallbackSources, sources) }
    : {};
};
