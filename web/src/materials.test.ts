import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { ERASER_KEY, MATERIAL_COUNT, Mat, PALETTE, materialByKey, materialName } from './materials';

// Parse `Name = 3,` lines out of the Rust enum so the two sides cannot drift.
const rustSource = readFileSync(fileURLToPath(new URL('../../sim/src/material.rs', import.meta.url)), 'utf8');
const enumBody = /pub enum Material \{([\s\S]*?)\n\}/.exec(rustSource)?.[1] ?? '';
const rustIds = Object.fromEntries([...enumBody.matchAll(/^\s*(\w+) = (\d+),/gm)].map((m) => [m[1], Number(m[2])]));

describe('palette config', () => {
  it('matches the Rust material ids exactly', () => {
    expect(Object.keys(rustIds)).toHaveLength(MATERIAL_COUNT);
    expect(rustIds).toEqual(Mat);
    expect(rustSource).toContain(`pub const COUNT: usize = ${MATERIAL_COUNT};`);
  });

  it('offers every material except Empty, once each', () => {
    const ids = PALETTE.map((m) => m.id).sort((a, b) => a - b);
    expect(ids).toEqual(Object.values(Mat).filter((id) => id !== Mat.Empty).sort((a, b) => a - b));
  });

  it('has unique shortcut keys that do not clash with other controls', () => {
    const keys = PALETTE.map((m) => m.key);
    expect(new Set(keys).size).toBe(keys.length);
    for (const reserved of [ERASER_KEY, ' ', '.', '[', ']']) expect(keys).not.toContain(reserved);
    expect(keys.slice(0, 10)).toEqual(['1', '2', '3', '4', '5', '6', '7', '8', '9', '0']);
  });

  it('has valid swatch colours and hints', () => {
    for (const m of PALETTE) {
      expect(m.swatch).toMatch(/^#[0-9a-f]{6}$/);
      expect(m.hint.length).toBeGreaterThan(5);
    }
  });

  it('looks materials up by key, ignoring case', () => {
    expect(materialByKey('1')?.name).toBe('Sand');
    expect(materialByKey('Q')?.name).toBe('Steam');
    expect(materialByKey('x')).toBeUndefined();
  });

  it('names ids', () => {
    expect(materialName(Mat.Lava)).toBe('Lava');
    expect(materialName(99)).toBe('#99');
  });
});
