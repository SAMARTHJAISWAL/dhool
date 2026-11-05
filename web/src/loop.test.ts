import { describe, expect, it } from 'vitest';
import { FixedStep, RateMeter } from './loop';

describe('FixedStep', () => {
  it('runs one tick per 60 Hz frame at normal speed', () => {
    const s = new FixedStep(1000 / 60);
    let ticks = 0;
    for (let i = 0; i < 60; i++) ticks += s.advance(1000 / 60);
    expect(ticks).toBe(60);
  });

  it('is independent of refresh rate', () => {
    const s = new FixedStep(1000 / 60);
    let ticks = 0;
    for (let i = 0; i < 120; i++) ticks += s.advance(1000 / 120);
    expect(ticks).toBe(60);
  });

  it('scales with speed', () => {
    const fast = new FixedStep(10);
    const slow = new FixedStep(10);
    let f = 0;
    let sl = 0;
    for (let i = 0; i < 100; i++) {
      f += fast.advance(10, 4);
      sl += slow.advance(10, 0.25);
    }
    expect(f).toBe(400);
    expect(sl).toBe(25);
  });

  it('caps the catch-up after a long stall', () => {
    const s = new FixedStep(10, 8);
    expect(s.advance(10_000)).toBe(8);
    expect(s.advance(10)).toBe(1);
  });

  it('ignores nonsense input', () => {
    const s = new FixedStep(10);
    expect(s.advance(-5)).toBe(0);
    expect(s.advance(Number.NaN)).toBe(0);
    expect(s.advance(10, 0)).toBe(0);
  });
});

describe('RateMeter', () => {
  it('reports events per second over its window', () => {
    const m = new RateMeter();
    for (let t = 0; t <= 1000; t += 10) m.add(1, 1 + t, 500);
    expect(m.rate).toBeGreaterThan(95);
    expect(m.rate).toBeLessThan(105);
  });
});
