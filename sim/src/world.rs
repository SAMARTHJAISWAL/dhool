//! The grid: storage, painting, the tick loop and rendering.
//!
//! Per-material behaviour lives in `rules.rs`; this module owns the data and
//! the order in which cells are visited.

use crate::color;
use crate::material::{COUNT, Material};
use crate::rng::Rng;

/// Largest accepted width or height. Keeps a malicious save file from
/// asking for gigabytes.
pub const MAX_DIM: usize = 1024;

/// Largest accepted brush radius.
pub const MAX_BRUSH: i32 = 64;

/// Chance out of 256 that a loose-material brush fills a given cell.
const SPRINKLE: u8 = 140;

/// One grid cell. Four bytes, so a 320x200 grid is 250 KiB.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    /// What is in the cell.
    pub mat: Material,
    /// Remaining lifetime for fire, steam and smoke; unused otherwise.
    pub life: u8,
    /// Random per-cell value that gives each grain its own tint.
    pub shade: u8,
    /// The tick on which this cell was last updated. A cell whose clock
    /// equals the world clock has already acted this tick and is skipped,
    /// which is what stops a grain from falling twice in one pass.
    pub(crate) clock: u8,
}

/// The simulation state.
#[derive(Clone, Debug)]
pub struct World {
    width: usize,
    height: usize,
    pub(crate) cells: Vec<Cell>,
    pixels: Vec<u32>,
    histogram: [u32; COUNT],
    pub(crate) rng: Rng,
    pub(crate) clock: u8,
    ticks: u32,
}

impl World {
    /// Creates an empty world. Dimensions are clamped to `1..=MAX_DIM`.
    pub fn new(width: usize, height: usize, seed: u32) -> Self {
        let width = width.clamp(1, MAX_DIM);
        let height = height.clamp(1, MAX_DIM);
        let n = width * height;
        let mut histogram = [0; COUNT];
        histogram[Material::Empty as usize] = n as u32;
        let mut world = Self {
            width,
            height,
            cells: vec![Cell::default(); n],
            pixels: vec![0; n],
            histogram,
            rng: Rng::new(seed),
            clock: 0,
            ticks: 0,
        };
        world.render();
        world
    }

    /// Grid width in cells.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Grid height in cells.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Number of ticks run since creation.
    pub fn ticks(&self) -> u32 {
        self.ticks
    }

    /// All cells, row-major, top row first.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Index of `(x, y)` in the cell array, or `None` if out of bounds.
    #[inline]
    pub(crate) fn index(&self, x: i32, y: i32) -> Option<usize> {
        let (x, y) = (usize::try_from(x).ok()?, usize::try_from(y).ok()?);
        (x < self.width && y < self.height).then(|| y * self.width + x)
    }

    /// Material at `(x, y)`, or `None` if out of bounds.
    pub fn get(&self, x: i32, y: i32) -> Option<Material> {
        self.index(x, y).map(|i| self.cells[i].mat)
    }

    /// The full cell at `(x, y)`, or `None` if out of bounds.
    pub fn cell(&self, x: i32, y: i32) -> Option<Cell> {
        self.index(x, y).map(|i| self.cells[i])
    }

    /// A freshly created cell: random shade, material-specific lifetime,
    /// and stamped with the current clock so it does not act until the
    /// next tick.
    pub(crate) fn fresh(&mut self, mat: Material) -> Cell {
        Cell {
            mat,
            life: mat.initial_life(&mut self.rng),
            shade: self.rng.byte(),
            clock: self.clock,
        }
    }

    /// Replaces the cell at index `i` with a fresh cell of `mat`.
    #[inline]
    pub(crate) fn spawn(&mut self, i: usize, mat: Material) {
        self.cells[i] = self.fresh(mat);
    }

    /// Overwrites a single cell. Returns `false` if out of bounds.
    pub fn set(&mut self, x: i32, y: i32, mat: Material) -> bool {
        match self.index(x, y) {
            Some(i) => {
                self.spawn(i, mat);
                true
            }
            None => false,
        }
    }

    /// Paints a disc with the interactive brush. Materials only fill empty
    /// cells (so you can pour water onto sand without deleting it), loose
    /// materials are sprinkled for a natural look, and painting `Empty`
    /// erases whatever is there. Out-of-range coordinates and huge
    /// radii are safe: the disc is clipped to the grid.
    pub fn paint(&mut self, cx: i32, cy: i32, radius: i32, mat: Material) {
        let sprinkle = mat.is_loose() && radius >= 2;
        self.for_disc(cx, cy, radius, |world, i| {
            if mat == Material::Empty {
                world.spawn(i, Material::Empty);
            } else if world.cells[i].mat == Material::Empty
                && (!sprinkle || world.rng.chance(SPRINKLE))
            {
                world.spawn(i, mat);
            }
        });
    }

    /// Fills a disc solid, overwriting whatever is there. Used by presets.
    pub fn stamp(&mut self, cx: i32, cy: i32, radius: i32, mat: Material) {
        self.for_disc(cx, cy, radius, |world, i| world.spawn(i, mat));
    }

    /// Fills the rectangle `[x, x + w) x [y, y + h)`, clipped to the grid.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, mat: Material) {
        let x0 = x.clamp(0, self.width as i32);
        let y0 = y.clamp(0, self.height as i32);
        let x1 = x.saturating_add(w.max(0)).clamp(0, self.width as i32);
        let y1 = y.saturating_add(h.max(0)).clamp(0, self.height as i32);
        for yy in y0..y1 {
            for xx in x0..x1 {
                let i = yy as usize * self.width + xx as usize;
                self.spawn(i, mat);
            }
        }
    }

    fn for_disc(&mut self, cx: i32, cy: i32, radius: i32, mut f: impl FnMut(&mut Self, usize)) {
        let r = radius.clamp(0, MAX_BRUSH);
        // r*r + r gives a rounder disc than r*r at small radii.
        let r2 = r * r + r;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                if let Some(i) = self.index(cx.saturating_add(dx), cy.saturating_add(dy)) {
                    f(self, i);
                }
            }
        }
    }

    /// Empties the whole grid.
    pub fn clear(&mut self) {
        self.cells.fill(Cell::default());
    }

    /// Advances the simulation by one step.
    ///
    /// Rows are visited bottom to top so that a falling grain lands in a
    /// row that has already been processed. The horizontal direction
    /// alternates each tick, otherwise liquids would drift and piles would
    /// lean towards the side scanned first. Each cell is stamped with the
    /// tick's clock as it acts (and so is any cell it swaps with), so
    /// nothing moves more than once per tick.
    pub fn tick(&mut self) {
        self.clock = self.clock.wrapping_add(1);
        let w = self.width as i32;
        let left_to_right = self.clock & 1 == 0;
        for y in (0..self.height as i32).rev() {
            for i in 0..w {
                let x = if left_to_right { i } else { w - 1 - i };
                self.update(x, y);
            }
        }
        self.ticks = self.ticks.wrapping_add(1);
    }

    /// Writes every cell's colour into the pixel buffer and refreshes the
    /// per-material counts. Rendering never touches the simulation RNG, so
    /// calling it (or not) cannot change how the world evolves.
    pub fn render(&mut self) {
        let mut histogram = [0u32; COUNT];
        for (i, (px, cell)) in self.pixels.iter_mut().zip(&self.cells).enumerate() {
            *px = color::cell_color(*cell, i as u32, self.ticks);
            histogram[cell.mat as usize] += 1;
        }
        self.histogram = histogram;
    }

    /// The RGBA pixel buffer from the last [`render`](Self::render), one
    /// `u32` per cell in little-endian `0xAABBGGRR` order, which is the
    /// byte order `ImageData` expects.
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    /// Per-material cell counts as of the last [`render`](Self::render).
    pub fn rendered_counts(&self) -> &[u32; COUNT] {
        &self.histogram
    }

    /// Counts cells of a material right now.
    pub fn count(&self, mat: Material) -> usize {
        self.cells.iter().filter(|c| c.mat == mat).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_world_is_empty() {
        let w = World::new(10, 5, 1);
        assert_eq!((w.width(), w.height()), (10, 5));
        assert_eq!(w.count(Material::Empty), 50);
        assert_eq!(w.pixels().len(), 50);
    }

    #[test]
    fn dimensions_are_clamped() {
        let w = World::new(0, 5000, 1);
        assert_eq!((w.width(), w.height()), (1, MAX_DIM));
    }

    #[test]
    fn get_and_set_respect_bounds() {
        let mut w = World::new(4, 4, 1);
        assert!(w.set(3, 3, Material::Sand));
        assert_eq!(w.get(3, 3), Some(Material::Sand));
        assert!(!w.set(4, 0, Material::Sand));
        assert!(!w.set(-1, 0, Material::Sand));
        assert_eq!(w.get(0, 4), None);
        assert_eq!(w.get(i32::MIN, i32::MAX), None);
    }

    #[test]
    fn out_of_bounds_paint_is_safe() {
        let mut w = World::new(16, 16, 1);
        w.paint(-100, -100, 5, Material::Sand);
        w.paint(i32::MAX, i32::MIN, 1000, Material::Wall);
        w.paint(i32::MIN, i32::MAX, -3, Material::Water);
        w.stamp(i32::MAX, i32::MAX, i32::MAX, Material::Stone);
        w.fill_rect(i32::MIN, i32::MIN, i32::MAX, i32::MAX, Material::Wall);
        w.fill_rect(10, 10, -5, -5, Material::Sand);
        assert_eq!(w.count(Material::Sand), 0);
        // A disc partly off the grid paints only the visible part.
        w.stamp(0, 0, 2, Material::Wall);
        assert!(w.count(Material::Wall) > 0);
    }

    #[test]
    fn huge_brush_is_clamped_not_slow() {
        let mut w = World::new(200, 200, 1);
        w.stamp(100, 100, i32::MAX, Material::Wall);
        let r = MAX_BRUSH as usize;
        assert!(w.count(Material::Wall) <= (2 * r + 1) * (2 * r + 1));
    }

    #[test]
    fn paint_fills_only_empty_cells_and_empty_erases() {
        let mut w = World::new(9, 9, 1);
        w.stamp(4, 4, 4, Material::Wall);
        let walls = w.count(Material::Wall);
        w.paint(4, 4, 4, Material::Sand);
        assert_eq!(w.count(Material::Wall), walls, "brush must not overwrite");
        w.paint(4, 4, 4, Material::Empty);
        assert_eq!(w.count(Material::Wall), 0);
    }

    #[test]
    fn single_cell_brush_is_not_sprinkled() {
        let mut w = World::new(9, 9, 1);
        for x in 0..9 {
            w.paint(x, 0, 0, Material::Water);
        }
        assert_eq!(w.count(Material::Water), 9);
    }

    #[test]
    fn fill_rect_is_clipped() {
        let mut w = World::new(10, 10, 1);
        w.fill_rect(8, 8, 5, 5, Material::Stone);
        assert_eq!(w.count(Material::Stone), 4);
    }

    #[test]
    fn clear_empties_everything() {
        let mut w = World::new(8, 8, 1);
        w.fill_rect(0, 0, 8, 8, Material::Sand);
        w.clear();
        assert_eq!(w.count(Material::Empty), 64);
    }

    #[test]
    fn render_counts_sum_to_cell_total() {
        let mut w = World::new(20, 10, 3);
        w.fill_rect(0, 5, 20, 5, Material::Water);
        w.render();
        let counts = w.rendered_counts();
        assert_eq!(counts.iter().sum::<u32>(), 200);
        assert_eq!(counts[Material::Water as usize], 100);
    }
}
