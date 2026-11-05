//! The `extern "C"` surface the browser calls.
//!
//! There is no wasm-bindgen here: every export takes and returns plain
//! integers, and bulk data (the pixel buffer, save files) is exchanged by
//! handing JavaScript a pointer into linear memory plus a length. JS wraps
//! that range in a typed-array view, so reading a frame copies nothing.
//!
//! The module keeps one global [`World`]. WebAssembly without the threads
//! proposal is single-threaded and these functions never call back into
//! JavaScript, so there is never more than one live reference to it.

use core::cell::UnsafeCell;

use crate::{Material, World};

const DEFAULT_WIDTH: usize = 320;
const DEFAULT_HEIGHT: usize = 200;

struct State {
    world: World,
    /// Scratch buffer for save/load data crossing the boundary.
    io: Vec<u8>,
    seed: u32,
}

struct Global(UnsafeCell<Option<State>>);

// SAFETY: wasm32-unknown-unknown (without the atomics feature) runs on a
// single thread, so the cell can never be accessed concurrently.
unsafe impl Sync for Global {}

static STATE: Global = Global(UnsafeCell::new(None));

/// Returns the global state, creating a default world on first use.
fn state() -> &'static mut State {
    // SAFETY: single-threaded (see above), and every export calls this once
    // and drops the reference before returning, so no two `&mut` overlap.
    let slot = unsafe { &mut *STATE.0.get() };
    slot.get_or_insert_with(|| State {
        world: World::new(DEFAULT_WIDTH, DEFAULT_HEIGHT, 1),
        io: Vec::new(),
        seed: 1,
    })
}

fn material(id: u32) -> Option<Material> {
    u8::try_from(id).ok().and_then(Material::from_u8)
}

/// Replaces the world with an empty `width` x `height` grid.
#[unsafe(no_mangle)]
pub extern "C" fn init(width: u32, height: u32, seed: u32) {
    let s = state();
    s.seed = seed;
    s.world = World::new(width as usize, height as usize, seed);
}

/// Grid width in cells (may differ from the request if it was clamped).
#[unsafe(no_mangle)]
pub extern "C" fn width() -> u32 {
    state().world.width() as u32
}

/// Grid height in cells.
#[unsafe(no_mangle)]
pub extern "C" fn height() -> u32 {
    state().world.height() as u32
}

/// Advances one tick.
#[unsafe(no_mangle)]
pub extern "C" fn tick() {
    state().world.tick();
}

/// Advances `n` ticks in one call, saving FFI round trips at high speed.
#[unsafe(no_mangle)]
pub extern "C" fn tick_n(n: u32) {
    let world = &mut state().world;
    for _ in 0..n {
        world.tick();
    }
}

/// Number of ticks since the world was created.
#[unsafe(no_mangle)]
pub extern "C" fn ticks() -> u32 {
    state().world.ticks()
}

/// Redraws the pixel buffer and refreshes the material counts.
#[unsafe(no_mangle)]
pub extern "C" fn render() {
    state().world.render();
}

/// Pointer to `width * height` RGBA pixels in linear memory.
#[unsafe(no_mangle)]
pub extern "C" fn pixels_ptr() -> *const u32 {
    state().world.pixels().as_ptr()
}

/// Paints with the interactive brush (fills empty cells; `0` erases).
#[unsafe(no_mangle)]
pub extern "C" fn paint(x: i32, y: i32, radius: i32, mat: u32) {
    if let Some(m) = material(mat) {
        state().world.paint(x, y, radius, m);
    }
}

/// Fills a solid disc, overwriting what is there.
#[unsafe(no_mangle)]
pub extern "C" fn stamp(x: i32, y: i32, radius: i32, mat: u32) {
    if let Some(m) = material(mat) {
        state().world.stamp(x, y, radius, m);
    }
}

/// Fills a rectangle, overwriting what is there.
#[unsafe(no_mangle)]
pub extern "C" fn fill_rect(x: i32, y: i32, w: i32, h: i32, mat: u32) {
    if let Some(m) = material(mat) {
        state().world.fill_rect(x, y, w, h, m);
    }
}

/// Empties the grid.
#[unsafe(no_mangle)]
pub extern "C" fn clear() {
    state().world.clear();
}

/// Material id at `(x, y)`, or `-1` when out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn get(x: i32, y: i32) -> i32 {
    state().world.get(x, y).map_or(-1, |m| i32::from(m.id()))
}

/// Number of cells of a material as of the last `render`.
#[unsafe(no_mangle)]
pub extern "C" fn count(mat: u32) -> u32 {
    material(mat).map_or(0, |m| state().world.rendered_counts()[m as usize])
}

/// Number of material ids, so the front end can check its palette.
#[unsafe(no_mangle)]
pub extern "C" fn material_count() -> u32 {
    crate::MATERIAL_COUNT as u32
}

/// Encodes the world into the IO buffer and returns its length in bytes.
/// Read it from `io_ptr()`.
#[unsafe(no_mangle)]
pub extern "C" fn save() -> u32 {
    let s = state();
    s.io = s.world.save();
    s.io.len() as u32
}

/// Pointer to the IO buffer.
#[unsafe(no_mangle)]
pub extern "C" fn io_ptr() -> *mut u8 {
    state().io.as_mut_ptr()
}

/// Resizes the IO buffer to `len` bytes (zeroed) and returns a pointer for
/// JavaScript to write a snapshot into before calling `load`.
#[unsafe(no_mangle)]
pub extern "C" fn io_reserve(len: u32) -> *mut u8 {
    let s = state();
    s.io.clear();
    s.io.resize(len as usize, 0);
    s.io.as_mut_ptr()
}

/// Loads the first `len` bytes of the IO buffer as a snapshot. Returns `1`
/// on success; on failure returns `0` and leaves the world untouched.
#[unsafe(no_mangle)]
pub extern "C" fn load(len: u32) -> u32 {
    let s = state();
    let len = (len as usize).min(s.io.len());
    match World::load(&s.io[..len], s.seed) {
        Ok(world) => {
            s.world = world;
            1
        }
        Err(_) => 0,
    }
}
