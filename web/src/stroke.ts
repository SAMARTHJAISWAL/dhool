// Stroke interpolation: pointer events arrive tens of milliseconds apart, so
// a fast drag would otherwise paint a dotted line. We fill the gap with
// evenly spaced brush positions.

export type Point = readonly [x: number, y: number];

/**
 * Integer grid points from `from` (exclusive) to `to` (inclusive), spaced at
 * most `spacing` cells apart. Returns just `[to]` when the two coincide.
 */
export function strokePoints(from: Point, to: Point, spacing = 1): Point[] {
  const step = Math.max(1, spacing);
  const dx = to[0] - from[0];
  const dy = to[1] - from[1];
  const dist = Math.hypot(dx, dy);
  const n = Math.max(1, Math.ceil(dist / step));
  const out: Point[] = [];
  let last: Point | undefined;
  for (let i = 1; i <= n; i++) {
    const t = i / n;
    const p: Point = [Math.round(from[0] + dx * t), Math.round(from[1] + dy * t)];
    if (!last || last[0] !== p[0] || last[1] !== p[1]) out.push(p);
    last = p;
  }
  return out;
}

/** Brush spacing for a radius: overlap discs by half so strokes are solid. */
export function spacingFor(radius: number): number {
  return Math.max(1, Math.floor(radius / 2));
}
