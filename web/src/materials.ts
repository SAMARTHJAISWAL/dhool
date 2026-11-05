// Palette configuration. Ids must match `Material` in sim/src/material.rs;
// materials.test.ts parses the Rust enum to make sure they do.

/** Total number of material ids in the Rust core, including Empty. */
export const MATERIAL_COUNT = 15;

export const Mat = {
  Empty: 0,
  Wall: 1,
  Sand: 2,
  Water: 3,
  Oil: 4,
  Fire: 5,
  Steam: 6,
  Wood: 7,
  Plant: 8,
  Acid: 9,
  Smoke: 10,
  Lava: 11,
  Stone: 12,
  Ice: 13,
  Ash: 14,
} as const;

export type MatId = (typeof Mat)[keyof typeof Mat];

export interface MaterialInfo {
  readonly id: MatId;
  readonly name: string;
  /** Keyboard shortcut (lower case, as in KeyboardEvent.key). */
  readonly key: string;
  /** Swatch colour for the palette button. */
  readonly swatch: string;
  /** One-line description for the tooltip. */
  readonly hint: string;
}

/** Paintable materials, in palette order. */
export const PALETTE: readonly MaterialInfo[] = [
  { id: Mat.Sand, name: 'Sand', key: '1', swatch: '#deba76', hint: 'Falls and piles up; sinks in liquids' },
  { id: Mat.Water, name: 'Water', key: '2', swatch: '#2668c4', hint: 'Flows and levels out; douses fire' },
  { id: Mat.Wall, name: 'Wall', key: '3', swatch: '#565250', hint: 'Indestructible, even by acid' },
  { id: Mat.Wood, name: 'Wood', key: '4', swatch: '#704626', hint: 'Static; burns slowly' },
  { id: Mat.Fire, name: 'Fire', key: '5', swatch: '#ff8c1e', hint: 'Spreads to flammables, boils water' },
  { id: Mat.Oil, name: 'Oil', key: '6', swatch: '#84602e', hint: 'Floats on water; very flammable' },
  { id: Mat.Plant, name: 'Plant', key: '7', swatch: '#48a83c', hint: 'Grows into water' },
  { id: Mat.Acid, name: 'Acid', key: '8', swatch: '#96ec3e', hint: 'Eats solids (not wall), used up doing it' },
  { id: Mat.Lava, name: 'Lava', key: '9', swatch: '#ee5416', hint: 'Slow; ignites things, stone in water' },
  { id: Mat.Ice, name: 'Ice', key: '0', swatch: '#a8daf4', hint: 'Freezes water nearby; melts near heat' },
  { id: Mat.Steam, name: 'Steam', key: 'q', swatch: '#b0bac6', hint: 'Rises, then condenses into water' },
  { id: Mat.Stone, name: 'Stone', key: 'w', swatch: '#6c6662', hint: 'Static; acid can eat it' },
  { id: Mat.Smoke, name: 'Smoke', key: 'r', swatch: '#54504e', hint: 'Rises and fades' },
  { id: Mat.Ash, name: 'Ash', key: 't', swatch: '#96908a', hint: 'Light powder left by fire' },
];

/** Key that toggles the eraser. */
export const ERASER_KEY = 'e';

export function materialByKey(key: string): MaterialInfo | undefined {
  const k = key.toLowerCase();
  return PALETTE.find((m) => m.key === k);
}

export function materialName(id: number): string {
  const entry = Object.entries(Mat).find(([, v]) => v === id);
  return entry ? entry[0] : `#${id}`;
}
