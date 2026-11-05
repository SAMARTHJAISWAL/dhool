import { describe, expect, it } from 'vitest';
import { Mat, type MatId } from './materials';
import { PRESETS, presetById, type Painter } from './presets';

/** A tiny software grid that applies fills the way the Rust core does. */
class Grid implements Painter {
  readonly cells: Uint8Array;
  ops = 0;
  constructor(
    readonly width: number,
    readonly height: number,
  ) {
    this.cells = new Uint8Array(width * height);
  }
  clear(): void {
    this.cells.fill(0);
  }
  private put(x: number, y: number, m: MatId): void {
    if (x >= 0 && y >= 0 && x < this.width && y < this.height) this.cells[y * this.width + x] = m;
  }
  fillRect(x: number, y: number, w: number, h: number, m: MatId): void {
    this.ops++;
    for (let yy = y; yy < y + h; yy++) for (let xx = x; xx < x + w; xx++) this.put(xx, yy, m);
  }
  stamp(cx: number, cy: number, r: number, m: MatId): void {
    this.ops++;
    for (let dy = -r; dy <= r; dy++)
      for (let dx = -r; dx <= r; dx++) if (dx * dx + dy * dy <= r * r + r) this.put(cx + dx, cy + dy, m);
  }
  count(m: MatId): number {
    return this.cells.filter((c) => c === m).length;
  }
}

describe('presets', () => {
  it('have unique ids and can be found by id', () => {
    const ids = PRESETS.map((p) => p.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(PRESETS.length).toBeGreaterThanOrEqual(3);
    for (const id of ids) expect(presetById(id)?.id).toBe(id);
    expect(presetById('nope')).toBeUndefined();
  });

  for (const [w, h] of [
    [320, 200],
    [200, 240],
  ] as const) {
    it(`each builds a non-trivial scene at ${w}x${h}`, () => {
      for (const p of PRESETS) {
        const g = new Grid(w, h);
        p.build(g);
        const filled = g.cells.filter((c) => c !== Mat.Empty).length;
        expect(filled, p.id).toBeGreaterThan(w * h * 0.05);
        expect(g.count(Mat.Wall), p.id).toBeGreaterThan(0);
      }
    });
  }

  it('give each scene its signature material', () => {
    const built = Object.fromEntries(
      PRESETS.map((p) => {
        const g = new Grid(320, 200);
        p.build(g);
        return [p.id, g];
      }),
    );
    expect(built.hourglass?.count(Mat.Sand)).toBeGreaterThan(1000);
    expect(built.volcano?.count(Mat.Lava)).toBeGreaterThan(100);
    expect(built['oil-fire']?.count(Mat.Oil)).toBeGreaterThan(1000);
    expect(built['oil-fire']?.count(Mat.Fire)).toBeGreaterThan(0);
    expect(built.garden?.count(Mat.Plant)).toBeGreaterThan(0);
  });
});
