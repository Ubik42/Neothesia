export const paperPoint = (
  x: number,
  y: number,
  r: number,
): [number, number] =>
  r === 90
    ? [1 - y, x]
    : r === 180
      ? [1 - x, 1 - y]
      : r === 270
        ? [y, 1 - x]
        : [x, y];
export function paperAnnotationBox(
  n: { x: number; y: number; width?: number; height?: number },
  r: number,
) {
  const a = paperPoint(n.x, n.y, r),
    b = paperPoint(n.x + (n.width ?? 0), n.y + (n.height ?? 0), r);
  return {
    x: Math.min(a[0], b[0]),
    y: Math.min(a[1], b[1]),
    width: Math.abs(a[0] - b[0]),
    height: Math.abs(a[1] - b[1]),
  };
}
