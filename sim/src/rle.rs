//! Compact run-length encoded snapshots, small enough for a URL hash.
//!
//! Layout (all integers little-endian):
//!
//! ```text
//! "DHL" | version: u8 | width: u16 | height: u16 | runs...
//! run = material id: u8 | run length - 1: LEB128 varint
//! ```
//!
//! Only materials are stored. Shades and lifetimes are regenerated on load,
//! which costs nothing visually and keeps a typical scene to a few hundred
//! bytes. Runs cross row boundaries, so an empty sky is a single run.

use crate::material::Material;
use crate::world::{MAX_DIM, World};

const MAGIC: &[u8; 3] = b"DHL";
const VERSION: u8 = 1;
const HEADER: usize = 8;

/// Why a snapshot could not be decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The data does not start with the `DHL` magic bytes.
    BadMagic,
    /// The snapshot was written by an unknown format version.
    BadVersion(u8),
    /// Width or height is zero or larger than [`MAX_DIM`].
    BadSize,
    /// A run names a material id that does not exist.
    BadMaterial(u8),
    /// The data ends in the middle of a run or header.
    Truncated,
    /// The runs do not add up to exactly `width * height` cells.
    LengthMismatch,
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadMagic => f.write_str("not a dhool snapshot"),
            Self::BadVersion(v) => write!(f, "unsupported snapshot version {v}"),
            Self::BadSize => f.write_str("snapshot dimensions out of range"),
            Self::BadMaterial(m) => write!(f, "unknown material id {m}"),
            Self::Truncated => f.write_str("snapshot is truncated"),
            Self::LengthMismatch => f.write_str("snapshot runs do not cover the grid"),
        }
    }
}

impl std::error::Error for DecodeError {}

fn push_varint(out: &mut Vec<u8>, mut v: u32) {
    while v >= 0x80 {
        out.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn read_varint(bytes: &[u8], pos: &mut usize) -> Result<u32, DecodeError> {
    let mut value = 0u32;
    for shift in (0..35).step_by(7) {
        let b = *bytes.get(*pos).ok_or(DecodeError::Truncated)?;
        *pos += 1;
        value |= u32::from(b & 0x7F).checked_shl(shift).unwrap_or(0);
        if b & 0x80 == 0 {
            return Ok(value);
        }
    }
    // More than five continuation bytes cannot be a valid u32.
    Err(DecodeError::LengthMismatch)
}

impl World {
    /// Encodes the grid's materials as a run-length snapshot.
    pub fn save(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64);
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.extend_from_slice(&(self.width() as u16).to_le_bytes());
        out.extend_from_slice(&(self.height() as u16).to_le_bytes());
        let mut cells = self.cells().iter().map(|c| c.mat);
        let Some(mut current) = cells.next() else {
            return out;
        };
        let mut run = 1u32;
        for mat in cells {
            if mat == current {
                run += 1;
            } else {
                out.push(current.id());
                push_varint(&mut out, run - 1);
                current = mat;
                run = 1;
            }
        }
        out.push(current.id());
        push_varint(&mut out, run - 1);
        out
    }

    /// Decodes a snapshot into a new world seeded with `seed`.
    ///
    /// # Errors
    ///
    /// Returns a [`DecodeError`] if the data is not a well-formed snapshot.
    /// Decoding never panics and never allocates more than
    /// `MAX_DIM * MAX_DIM` cells, whatever the input.
    pub fn load(bytes: &[u8], seed: u32) -> Result<World, DecodeError> {
        if bytes.len() < HEADER {
            return Err(if bytes.starts_with(&MAGIC[..bytes.len().min(3)]) {
                DecodeError::Truncated
            } else {
                DecodeError::BadMagic
            });
        }
        if &bytes[..3] != MAGIC {
            return Err(DecodeError::BadMagic);
        }
        if bytes[3] != VERSION {
            return Err(DecodeError::BadVersion(bytes[3]));
        }
        let width = usize::from(u16::from_le_bytes([bytes[4], bytes[5]]));
        let height = usize::from(u16::from_le_bytes([bytes[6], bytes[7]]));
        if !(1..=MAX_DIM).contains(&width) || !(1..=MAX_DIM).contains(&height) {
            return Err(DecodeError::BadSize);
        }
        let total = width * height;
        let mut world = World::new(width, height, seed);
        let mut pos = HEADER;
        let mut filled = 0usize;
        while pos < bytes.len() {
            let id = bytes[pos];
            pos += 1;
            let mat = Material::from_u8(id).ok_or(DecodeError::BadMaterial(id))?;
            let run = read_varint(bytes, &mut pos)? as usize + 1;
            if run > total - filled {
                return Err(DecodeError::LengthMismatch);
            }
            if mat != Material::Empty {
                for i in filled..filled + run {
                    world.spawn(i, mat);
                }
            }
            filled += run;
        }
        if filled != total {
            return Err(DecodeError::LengthMismatch);
        }
        world.render();
        Ok(world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_round_trips() {
        for v in [0, 1, 127, 128, 300, 16_383, 16_384, u32::MAX] {
            let mut buf = Vec::new();
            push_varint(&mut buf, v);
            let mut pos = 0;
            assert_eq!(read_varint(&buf, &mut pos), Ok(v));
            assert_eq!(pos, buf.len());
        }
    }

    #[test]
    fn overlong_varint_is_rejected() {
        let mut pos = 0;
        assert!(read_varint(&[0xFF; 6], &mut pos).is_err());
    }
}
