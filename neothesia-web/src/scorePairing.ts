export type ImportKind = "midi" | "notation" | "pdf" | "image";
export interface PairTarget {
  path: string;
  title: string;
  composer: string;
  aliases: string[];
  contentId?: string;
}
export function importKind(name: string): ImportKind | null {
  if (/\.(mid|midi)$/i.test(name)) return "midi";
  if (/\.(musicxml|xml|mxl)$/i.test(name)) return "notation";
  if (/\.pdf$/i.test(name)) return "pdf";
  if (/\.(png|jpe?g|webp)$/i.test(name)) return "image";
  return null;
}
export const fileStem = (name: string) =>
  name.replace(/^.*[\\/]/, "").replace(/\.[^.]+$/, "");
const normalized = (s: string) =>
  s
    .normalize("NFKC")
    .toLocaleLowerCase()
    .replace(/[\s_\-.,，。()[\]（）]+/g, "");
export function imageBook(name: string) {
  return (
    fileStem(name).replace(
      /(?:[-_\s](?:page|p|页)?\s*\d{1,4}|第\d{1,4}页)$/i,
      "",
    ) || fileStem(name)
  );
}
export function pairingCandidates(
  name: string,
  targets: PairTarget[],
): PairTarget[] {
  const exact = normalized(fileStem(name));
  const match = (value: string) =>
    targets.filter((t) => t.aliases.some((a) => normalized(a) === value));
  const direct = match(exact);
  // Work numbers are significant. Only try a page suffix on image files,
  // and only after the complete name fails to match a work.
  return direct.length || importKind(name) !== "image"
    ? direct
    : match(normalized(imageBook(name)));
}
