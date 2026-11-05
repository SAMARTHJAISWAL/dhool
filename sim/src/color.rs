//! Cell colours.
//!
//! Each material has a base colour and a "grain" amplitude; a cell's random
//! `shade` byte nudges its brightness within that amplitude so sand looks
//! like sand rather than a flat fill. Fire, lava and water also shimmer: they
//! mix in a hash of the cell index and the tick count, so they move even
//! when the cells themselves do not. Rendering is a pure function of the
//! cell and the tick, and never draws from the simulation's RNG.

use crate::material::Material;
use crate::world::Cell;

/// Background colour for empty cells.
pub const EMPTY: [u8; 3] = [17, 14, 12];

/// Packs RGB into a little-endian RGBA `u32` with full alpha.
#[inline]
pub const fn rgba([r, g, b]: [u8; 3]) -> u32 {
    r as u32 | (g as u32) << 8 | (b as u32) << 16 | 0xFF00_0000
}

/// Base colour and grain amplitude for a material.
const fn look(mat: Material) -> ([u8; 3], i32) {
    match mat {
        Material::Empty => (EMPTY, 0),
        Material::Wall => ([86, 82, 80], 6),
        Material::Sand => ([222, 186, 118], 22),
        Material::Water => ([38, 104, 196], 10),
        Material::Oil => ([132, 96, 46], 10),
        Material::Fire => ([255, 140, 30], 0),
        Material::Steam => ([176, 186, 198], 14),
        Material::Wood => ([112, 70, 38], 14),
        Material::Plant => ([72, 168, 60], 20),
        Material::Acid => ([150, 236, 62], 16),
        Material::Smoke => ([84, 80, 78], 10),
        Material::Lava => ([238, 84, 22], 0),
        Material::Stone => ([108, 102, 98], 16),
        Material::Ice => ([168, 218, 244], 10),
        Material::Ash => ([150, 144, 138], 14),
    }
}

/// A cheap integer hash (a variant of the `lowbias32` finaliser).
#[inline]
const fn hash(a: u32, b: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^ (h >> 16)
}

#[inline]
fn add(c: [u8; 3], d: i32) -> [u8; 3] {
    c.map(|v| (i32::from(v) + d).clamp(0, 255) as u8)
}

/// Linear blend from `a` to `b`, with `t` in `0..=256`.
#[inline]
fn mix(a: [u8; 3], b: [u8; 3], t: i32) -> [u8; 3] {
    let mut out = [0; 3];
    for k in 0..3 {
        let (a, b) = (i32::from(a[k]), i32::from(b[k]));
        out[k] = (a + (b - a) * t / 256).clamp(0, 255) as u8;
    }
    out
}

/// Colour of one cell at a given tick. `index` is the cell's position in
/// the grid, used only to decorrelate the shimmer between neighbours.
pub fn cell_color(cell: Cell, index: u32, tick: u32) -> u32 {
    let (base, grain) = look(cell.mat);
    let shade = i32::from(cell.shade) - 128;
    let rgb = match cell.mat {
        Material::Fire => {
            // Young flames are yellow-white, old ones deep red.
            let heat = i32::from(cell.life.min(48)) * 256 / 48;
            let flicker = (hash(index, tick) & 63) as i32 - 32;
            let t = (heat + flicker).clamp(0, 256);
            if t > 128 {
                mix([255, 120, 24], [255, 226, 120], (t - 128) * 2)
            } else {
                mix([168, 30, 12], [255, 120, 24], t * 2)
            }
        }
        Material::Lava => {
            let glow = (hash(index / 3, tick / 6) & 127) as i32;
            mix(base, [255, 176, 52], glow + shade / 4)
        }
        Material::Water => {
            let ripple = (hash(index, tick / 10) & 7) as i32 - 4;
            add(base, shade * grain / 128 + ripple)
        }
        Material::Smoke => {
            // Fade into the background as the smoke runs out of life.
            let t = 256 - i32::from(cell.life.min(100)) * 256 / 100;
            mix(add(base, shade * grain / 128), EMPTY, t)
        }
        _ => add(base, shade * grain / 128),
    };
    rgba(rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(mat: Material, shade: u8, life: u8) -> Cell {
        Cell {
            mat,
            life,
            shade,
            ..Cell::default()
        }
    }

    #[test]
    fn packs_bytes_in_imagedata_order() {
        let px = rgba([1, 2, 3]);
        assert_eq!(px.to_le_bytes(), [1, 2, 3, 255]);
    }

    #[test]
    fn empty_is_the_background() {
        assert_eq!(cell_color(Cell::default(), 0, 0), rgba(EMPTY));
    }

    #[test]
    fn grain_varies_within_a_material() {
        let dark = cell_color(cell(Material::Sand, 0, 0), 0, 0);
        let light = cell_color(cell(Material::Sand, 255, 0), 0, 0);
        assert_ne!(dark, light);
    }

    #[test]
    fn fire_shimmers_over_time() {
        let c = cell(Material::Fire, 0, 20);
        let frames: Vec<_> = (0..8).map(|t| cell_color(c, 5, t)).collect();
        assert!(frames.windows(2).any(|w| w[0] != w[1]));
    }

    #[test]
    fn every_pixel_is_opaque() {
        for mat in Material::ALL {
            for shade in [0, 128, 255] {
                let px = cell_color(cell(mat, shade, 30), 9, 9);
                assert_eq!(px >> 24, 0xFF, "{mat:?}");
            }
        }
    }
}
