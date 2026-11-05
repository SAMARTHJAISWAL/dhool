import { describe, expect, it } from 'vitest';
import { spacingFor, strokePoints, type Point } from './stroke';

const gap = (a: Point, b: Point) => Math.max(Math.abs(a[0] - b[0]), Math.abs(a[1] - b[1]));

describe('strokePoints', () => {
  it('returns just the end point for a zero-length stroke', () => {
    expect(strokePoints([5, 5], [5, 5])).toEqual([[5, 5]]);
  });

  it('includes the end point but not the start', () => {
    const pts = strokePoints([0, 0], [4, 0]);
    expect(pts).toEqual([
      [1, 0],
      [2, 0],
      [3, 0],
      [4, 0],
    ]);
  });

  it('leaves no gaps on a fast diagonal drag', () => {
    const from: Point = [3, 7];
    const pts = strokePoints(from, [90, 41], 1);
    let prev = from;
    for (const p of pts) {
      expect(gap(prev, p)).toBeLessThanOrEqual(1);
      prev = p;
    }
    expect(pts.at(-1)).toEqual([90, 41]);
  });

  it('respects the spacing for big brushes', () => {
    const from: Point = [0, 0];
    const pts = strokePoints(from, [100, 0], 5);
    expect(pts).toHaveLength(20);
    let prev = from;
    for (const p of pts) {
      expect(gap(prev, p)).toBeLessThanOrEqual(5);
      prev = p;
    }
  });

  it('never emits the same cell twice in a row', () => {
    const pts = strokePoints([0, 0], [1, 1], 0.1);
    expect(pts).toEqual([[1, 1]]);
  });

  it('works backwards and with negative coordinates', () => {
    const pts = strokePoints([2, 2], [-2, -2]);
    expect(pts.at(-1)).toEqual([-2, -2]);
    expect(pts).toHaveLength(4);
  });
});

describe('spacingFor', () => {
  it('is at least one cell and about half the radius', () => {
    expect(spacingFor(0)).toBe(1);
    expect(spacingFor(1)).toBe(1);
    expect(spacingFor(8)).toBe(4);
    expect(spacingFor(9)).toBe(4);
  });
});
