// Built-in scenes, drawn in code with a handful of primitives so they adapt
// to any grid size.

import { Mat, type MatId } from './materials';

export interface Painter {
  readonly width: number;
  readonly height: number;
  clear(): void;
  fillRect(x: number, y: number, w: number, h: number, mat: MatId): void;
  stamp(x: number, y: number, r: number, mat: MatId): void;
}

export interface Preset {
  readonly id: string;
  readonly name: string;
  readonly blurb: string;
  build(p: Painter): void;
}

function border(p: Painter): void {
  const { width: w, height: h } = p;
  p.fillRect(0, h - 2, w, 2, Mat.Wall);
  p.fillRect(0, 0, 2, h, Mat.Wall);
  p.fillRect(w - 2, 0, 2, h, Mat.Wall);
}

/** A filled triangle with apex (cx, top) and a base `halfWidth` either side at `bottom`. */
function triangle(p: Painter, cx: number, top: number, bottom: number, halfWidth: number, mat: MatId): void {
  const rows = Math.max(1, bottom - top);
  for (let y = top; y <= bottom; y++) {
    const half = Math.round(((y - top) / rows) * halfWidth);
    p.fillRect(cx - half, y, 2 * half + 1, 1, mat);
  }
}

/** A thick straight line made of discs. */
function line(p: Painter, x0: number, y0: number, x1: number, y1: number, r: number, mat: MatId): void {
  const n = Math.max(1, Math.ceil(Math.hypot(x1 - x0, y1 - y0)));
  for (let i = 0; i <= n; i++) {
    const t = i / n;
    p.stamp(Math.round(x0 + (x1 - x0) * t), Math.round(y0 + (y1 - y0) * t), r, mat);
  }
}

export const PRESETS: readonly Preset[] = [
  {
    id: 'hourglass',
    name: 'Hourglass',
    blurb: 'Sand through a narrow neck',
    build(p) {
      const { width: w, height: h } = p;
      border(p);
      const cx = Math.floor(w / 2);
      const top = Math.round(h * 0.08);
      const bottom = h - 3;
      const mid = Math.round((top + bottom) / 2);
      const half = Math.round(Math.min(w * 0.3, h * 0.42));
      const neck = Math.max(3, Math.round(half * 0.06));
      // Inner half-width of the glass at row y.
      const halfAt = (y: number): number =>
        y <= mid
          ? half + ((neck - half) * (y - top)) / (mid - top)
          : neck + ((half - neck) * (y - mid)) / (bottom - mid);
      // Fill most of the upper bulb with sand.
      for (let y = top + 2; y <= mid - 6; y++) {
        const span = Math.floor(halfAt(y)) - 2;
        if (span > 0) p.fillRect(cx - span, y, 2 * span + 1, 1, Mat.Sand);
      }
      line(p, cx - half, top, cx - neck, mid, 1, Mat.Wall);
      line(p, cx + half, top, cx + neck, mid, 1, Mat.Wall);
      line(p, cx - neck, mid, cx - half, bottom, 1, Mat.Wall);
      line(p, cx + neck, mid, cx + half, bottom, 1, Mat.Wall);
      p.fillRect(cx - half - 1, top - 2, 2 * half + 3, 2, Mat.Wall);
    },
  },
  {
    id: 'volcano',
    name: 'Volcano',
    blurb: 'Lava meets the sea',
    build(p) {
      const { width: w, height: h } = p;
      border(p);
      const cx = Math.round(w * 0.38);
      const peak = Math.round(h * 0.3);
      const base = h - 3;
      const half = Math.round(w * 0.3);
      triangle(p, cx, peak, base, half, Mat.Stone);
      // Magma chamber and vent.
      p.stamp(cx, Math.round(h * 0.78), Math.round(h * 0.12), Mat.Lava);
      p.fillRect(cx - 3, peak, 7, Math.round(h * 0.78) - peak, Mat.Lava);
      // An eruption already under way: a blob of lava sitting on the crater
      // that spills down both flanks, towards the trees and into the sea.
      p.stamp(cx, peak - 6, 9, Mat.Lava);
      // The sea on the right, with a wooden pier.
      const sea = Math.round(h * 0.72);
      p.fillRect(cx + half - 10, sea, w - (cx + half - 10) - 2, base - sea + 1, Mat.Water);
      p.fillRect(Math.round(w * 0.8), sea - 6, Math.round(w * 0.16), 2, Mat.Wood);
      for (const x of [0.82, 0.88, 0.94]) p.fillRect(Math.round(w * x), sea - 4, 2, base - sea + 4, Mat.Wood);
      // A few trees on the left slope.
      for (const x of [0.12, 0.18]) {
        const tx = Math.round(w * x);
        const ty = Math.round(peak + ((cx - tx) / half) * (base - peak)) - 1;
        p.fillRect(tx, ty - 10, 2, 10, Mat.Wood);
        p.stamp(tx, ty - 12, 4, Mat.Plant);
      }
    },
  },
  {
    id: 'oil-fire',
    name: 'Oil fire',
    blurb: 'A slick on a harbour, and a spark',
    build(p) {
      const { width: w, height: h } = p;
      border(p);
      const floor = h - 3;
      const basinTop = Math.round(h * 0.55);
      // Stone basin with water, oil floating on top.
      p.fillRect(Math.round(w * 0.1), basinTop, 3, floor - basinTop + 1, Mat.Stone);
      p.fillRect(Math.round(w * 0.9) - 3, basinTop, 3, floor - basinTop + 1, Mat.Stone);
      const x0 = Math.round(w * 0.1) + 3;
      const x1 = Math.round(w * 0.9) - 3;
      p.fillRect(x0, basinTop + 14, x1 - x0, floor - basinTop - 13, Mat.Water);
      p.fillRect(x0, basinTop + 4, x1 - x0, 10, Mat.Oil);
      // Wooden jetty and crates over the slick.
      p.fillRect(Math.round(w * 0.05), basinTop - 4, Math.round(w * 0.35), 3, Mat.Wood);
      p.fillRect(Math.round(w * 0.2), basinTop - 16, 12, 12, Mat.Wood);
      p.fillRect(Math.round(w * 0.6), basinTop - 30, 4, 26, Mat.Wood);
      p.fillRect(Math.round(w * 0.6) - 12, basinTop - 30, 28, 3, Mat.Wood);
      // An ice shelf on the far side.
      p.fillRect(Math.round(w * 0.9), basinTop - 2, Math.round(w * 0.1) - 2, floor - basinTop + 3, Mat.Ice);
      // The spark.
      p.stamp(Math.round(w * 0.45), basinTop + 2, 2, Mat.Fire);
    },
  },
  {
    id: 'garden',
    name: 'Garden',
    blurb: 'Rain on seedlings',
    build(p) {
      const { width: w, height: h } = p;
      border(p);
      const floor = h - 3;
      // Rolling sand ground with a pond in the middle.
      for (let x = 2; x < w - 2; x++) {
        const ground = Math.round(h * 0.78 + Math.sin(x / 18) * 4 + Math.sin(x / 7) * 1.5);
        p.fillRect(x, ground, 1, floor - ground + 1, Mat.Sand);
      }
      const pondX = Math.round(w * 0.42);
      const pondW = Math.round(w * 0.22);
      p.fillRect(pondX, Math.round(h * 0.74), pondW, Math.round(h * 0.16), Mat.Empty);
      p.fillRect(pondX, Math.round(h * 0.86), pondW, 2, Mat.Stone);
      p.fillRect(pondX, Math.round(h * 0.76), pondW, Math.round(h * 0.1), Mat.Water);
      // Seedlings in and around the pond.
      for (const x of [0.46, 0.52, 0.58]) p.stamp(Math.round(w * x), Math.round(h * 0.84), 1, Mat.Plant);
      // Planter boxes with wet soil.
      for (const x of [0.12, 0.76]) {
        const bx = Math.round(w * x);
        const by = Math.round(h * 0.58);
        p.fillRect(bx, by, 30, 2, Mat.Wood);
        p.fillRect(bx, by - 12, 2, 12, Mat.Wood);
        p.fillRect(bx + 28, by - 12, 2, 12, Mat.Wood);
        p.fillRect(bx + 2, by - 6, 26, 6, Mat.Water);
        p.stamp(bx + 15, by - 2, 1, Mat.Plant);
      }
      // A rain cloud of steam.
      for (let i = 0; i < 9; i++) {
        p.stamp(Math.round(w * (0.3 + i * 0.05)), Math.round(h * (0.12 + (i % 3) * 0.03)), 5, Mat.Steam);
      }
    },
  },
];

export function presetById(id: string): Preset | undefined {
  return PRESETS.find((p) => p.id === id);
}
