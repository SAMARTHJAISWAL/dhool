//! # dhool-sim
//!
//! The simulation core of **dhool** (धूल, "dust"), a falling-sand sandbox.
//!
//! The crate is plain, dependency-free Rust. [`World`] holds a grid of
//! [`Cell`]s and advances it with [`World::tick`]; [`World::render`] writes
//! an RGBA pixel buffer the browser can blit directly. On `wasm32` a thin
//! `extern "C"` layer (`ffi.rs`) exposes a single global world to
//! JavaScript; everything else compiles and is tested on the host.
//!
//! ```
//! use dhool_sim::{Material, World};
//!
//! let mut world = World::new(32, 32, 7);
//! world.set(16, 0, Material::Sand);
//! for _ in 0..40 {
//!     world.tick();
//! }
//! assert_eq!(world.get(16, 31), Some(Material::Sand));
//! ```

mod color;
mod material;
mod rle;
mod rng;
mod rules;
mod world;

#[cfg(target_arch = "wasm32")]
mod ffi;

pub use material::{COUNT as MATERIAL_COUNT, Material, Phase};
pub use rle::DecodeError;
pub use rng::Rng;
pub use world::{Cell, MAX_BRUSH, MAX_DIM, World};
