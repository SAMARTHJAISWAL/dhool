//! Per-material update rules.
//!
//! Each rule is a method on [`World`] that runs for one cell, at most once
//! per tick. Movement goes through [`World::try_move`], which applies the
//! single density rule in [`can_displace`]; reactions spawn fresh cells via
//! [`World::spawn`], which stamps them with the current clock so they do not
//! act again until the next tick.

use crate::material::{Material, Phase};
use crate::world::World;

/// Direction of a proposed move, which decides how density is compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dir {
    /// Falling or sinking.
    Down,
    /// Rising (gases and buoyant fluids).
    Up,
    /// Spreading sideways (liquids and drifting gases).
    Side,
}

/// Orthogonal neighbours.
const N4: [(i32, i32); 4] = [(0, 1), (0, -1), (-1, 0), (1, 0)];
/// All eight neighbours.
const N8: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Chance out of 256 that a powder sinks into a liquid on a given try. Less
/// than certain, so sand drifts down through water instead of teleporting.
const LIQUID_DRAG: u8 = 128;
/// Chance out of 256 that lava moves on a given tick. Lava is viscous.
const LAVA_MOBILITY: u8 = 64;
/// Chance out of 256 that acid eats the neighbour it touches.
const ACID_BITE: u8 = 96;
/// Chance out of 256 that a plant tries to grow on a given tick.
const PLANT_GROWTH: u8 = 24;
/// Chance out of 256 that ice tries to freeze a neighbour on a given tick.
const ICE_FREEZE: u8 = 3;
/// Chance out of 256 that a flame melts touching ice on a given tick.
const FIRE_MELT: u8 = 32;
/// Chance out of 256 that a flame drifts upward on a given tick.
const FIRE_RISE: u8 = 140;
/// Chance out of 256 that open lava throws a spark into the air above it.
const LAVA_SPARK: u8 = 2;

/// Whether `mover` may swap into a cell holding `target` when moving in
/// `dir`. This one function is the whole density model:
///
/// * anything may move into empty space;
/// * solids, powders and fire are never displaced;
/// * moving down, the mover must be denser than the fluid it displaces;
/// * moving up, it must be lighter (bubbles rise through water);
/// * sideways, only a liquid may push a lighter fluid aside.
pub(crate) fn can_displace(mover: Material, target: Material, dir: Dir) -> bool {
    if target == Material::Empty {
        return true;
    }
    if !target.is_fluid() {
        return false;
    }
    match dir {
        Dir::Down => mover.density() > target.density(),
        Dir::Up => mover.density() < target.density(),
        Dir::Side => mover.phase() == Phase::Liquid && mover.density() > target.density(),
    }
}

impl World {
    /// Runs the rule for the cell at `(x, y)`, unless it already acted.
    pub(crate) fn update(&mut self, x: i32, y: i32) {
        let i = y as usize * self.width() + x as usize;
        let cell = self.cells[i];
        if cell.clock == self.clock {
            return;
        }
        self.cells[i].clock = self.clock;
        match cell.mat {
            Material::Empty | Material::Wall | Material::Wood | Material::Stone => {}
            Material::Sand | Material::Ash => self.powder(i, x, y),
            Material::Water | Material::Oil => self.liquid(i, x, y),
            Material::Acid => {
                if !self.acid(i, x, y) {
                    self.liquid(i, x, y);
                }
            }
            Material::Lava => self.lava(i, x, y),
            Material::Fire => self.fire(i, x, y),
            Material::Steam | Material::Smoke => self.gas(i, x, y),
            Material::Plant => self.plant(x, y),
            Material::Ice => self.ice(x, y),
        }
    }

    /// Swaps cells `a` and `b` and marks both as done for this tick.
    #[inline]
    fn swap(&mut self, a: usize, b: usize) {
        self.cells.swap(a, b);
        self.cells[a].clock = self.clock;
        self.cells[b].clock = self.clock;
    }

    /// Tries to move the cell at `from` to `(x, y)`. Returns whether it moved.
    pub(crate) fn try_move(&mut self, from: usize, x: i32, y: i32, dir: Dir) -> bool {
        let Some(to) = self.index(x, y) else {
            return false;
        };
        let mover = self.cells[from].mat;
        let target = self.cells[to].mat;
        if !can_displace(mover, target, dir) {
            return false;
        }
        if mover.phase() == Phase::Powder
            && target.phase() == Phase::Liquid
            && !self.rng.chance(LIQUID_DRAG)
        {
            return false;
        }
        self.swap(from, to);
        true
    }

    /// Material at an offset from `(x, y)` together with its index.
    #[inline]
    fn neighbour(&self, x: i32, y: i32, (dx, dy): (i32, i32)) -> Option<(usize, Material)> {
        let j = self.index(x + dx, y + dy)?;
        Some((j, self.cells[j].mat))
    }

    /// Sand and ash: fall, otherwise slide down a random diagonal.
    fn powder(&mut self, i: usize, x: i32, y: i32) {
        if self.try_move(i, x, y + 1, Dir::Down) {
            return;
        }
        let d = self.rng.sign();
        if !self.try_move(i, x + d, y + 1, Dir::Down) {
            self.try_move(i, x - d, y + 1, Dir::Down);
        }
    }

    /// Liquids: fall, slide diagonally, else flow up to `dispersion` cells
    /// sideways. The flow stops early above a hole so the liquid pours into
    /// it on the next tick instead of skating over it.
    fn liquid(&mut self, i: usize, x: i32, y: i32) {
        if self.try_move(i, x, y + 1, Dir::Down) {
            return;
        }
        let d = self.rng.sign();
        if self.try_move(i, x + d, y + 1, Dir::Down) || self.try_move(i, x - d, y + 1, Dir::Down) {
            return;
        }
        let mat = self.cells[i].mat;
        let reach = mat.dispersion();
        for dir in [d, -d] {
            let mut dest = None;
            for step in 1..=reach {
                let nx = x + dir * step;
                match self.index(nx, y) {
                    Some(j) if can_displace(mat, self.cells[j].mat, Dir::Side) => dest = Some(j),
                    _ => break,
                }
                if self
                    .get(nx, y + 1)
                    .is_some_and(|below| can_displace(mat, below, Dir::Down))
                {
                    break;
                }
            }
            if let Some(j) = dest {
                self.swap(i, j);
                return;
            }
        }
    }

    /// Steam and smoke: age, then rise with a random sideways wobble.
    /// Expired steam condenses into water; expired smoke vanishes.
    fn gas(&mut self, i: usize, x: i32, y: i32) {
        let cell = self.cells[i];
        if cell.life == 0 {
            let residue = if cell.mat == Material::Steam {
                Material::Water
            } else {
                Material::Empty
            };
            self.spawn(i, residue);
            return;
        }
        // Steam ages at half speed so clouds hang around for a while.
        if cell.mat != Material::Steam || self.rng.coin() {
            self.cells[i].life -= 1;
        }
        let dx = self.rng.below(3) as i32 - 1;
        if self.try_move(i, x + dx, y - 1, Dir::Up) || self.try_move(i, x, y - 1, Dir::Up) {
            return;
        }
        let d = self.rng.sign();
        self.try_move(i, x + d, y, Dir::Side);
    }

    /// Turns a flammable cell into a flame whose lifetime depends on fuel.
    fn ignite(&mut self, j: usize) {
        let fuel = self.cells[j].mat;
        let life = fuel.burn_time(&mut self.rng);
        self.spawn(j, Material::Fire);
        self.cells[j].life = life;
    }

    /// Fire: boils touching water (and goes out), melts ice, spreads to
    /// flammable neighbours, flickers upward, and when it burns out leaves
    /// smoke, a little ash, or nothing.
    fn fire(&mut self, i: usize, x: i32, y: i32) {
        for off in N4 {
            match self.neighbour(x, y, off) {
                Some((j, Material::Water)) => {
                    self.spawn(j, Material::Steam);
                    self.spawn(i, Material::Empty);
                    return;
                }
                Some((j, Material::Ice)) if self.rng.chance(FIRE_MELT) => {
                    self.spawn(j, Material::Water);
                }
                _ => {}
            }
        }
        for off in N8 {
            if let Some((j, m)) = self.neighbour(x, y, off) {
                let f = m.flammability();
                if f > 0 && self.rng.chance(f) {
                    self.ignite(j);
                }
            }
        }
        let life = self.cells[i].life;
        if life == 0 {
            let residue = match self.rng.below(10) {
                0..=3 => Material::Smoke,
                4 => Material::Ash,
                _ => Material::Empty,
            };
            self.spawn(i, residue);
            return;
        }
        self.cells[i].life = life - 1;
        if self.rng.chance(FIRE_RISE) {
            let dx = self.rng.below(3) as i32 - 1;
            if let Some((j, Material::Empty)) = self.neighbour(x, y, (dx, -1)) {
                self.swap(i, j);
            }
        }
    }

    /// Lava: quenched by water into stone (boiling the water), melts ice,
    /// ignites flammables, throws the odd spark, and flows slowly.
    fn lava(&mut self, i: usize, x: i32, y: i32) {
        for off in N4 {
            match self.neighbour(x, y, off) {
                Some((j, Material::Water)) => {
                    self.spawn(j, Material::Steam);
                    self.spawn(i, Material::Stone);
                    return;
                }
                Some((j, Material::Ice)) => self.spawn(j, Material::Water),
                Some((j, m)) if m.flammability() > 0 && self.rng.chance(m.flammability()) => {
                    self.ignite(j);
                }
                _ => {}
            }
        }
        if self.rng.chance(LAVA_SPARK) {
            if let Some((j, Material::Empty)) = self.neighbour(x, y, (0, -1)) {
                self.spawn(j, Material::Fire);
                self.cells[j].life = 4 + self.rng.below(8) as u8;
            }
        }
        if self.rng.chance(LAVA_MOBILITY) {
            self.liquid(i, x, y);
        }
    }

    /// Acid: bites a solid below or beside it. Both cells disappear, the
    /// acid sometimes leaving a wisp of smoke. Returns whether it reacted.
    fn acid(&mut self, i: usize, x: i32, y: i32) -> bool {
        let off = if self.rng.coin() {
            (0, 1)
        } else {
            N4[self.rng.below(4) as usize]
        };
        let Some((j, target)) = self.neighbour(x, y, off) else {
            return false;
        };
        if !target.dissolvable() || !self.rng.chance(ACID_BITE) {
            return false;
        }
        self.spawn(j, Material::Empty);
        let fumes = self.rng.below(4) == 0;
        self.spawn(
            i,
            if fumes {
                Material::Smoke
            } else {
                Material::Empty
            },
        );
        true
    }

    /// Plant: now and then, turns a random neighbouring water cell into plant.
    fn plant(&mut self, x: i32, y: i32) {
        if !self.rng.chance(PLANT_GROWTH) {
            return;
        }
        let off = N8[self.rng.below(8) as usize];
        if let Some((j, Material::Water)) = self.neighbour(x, y, off) {
            self.spawn(j, Material::Plant);
        }
    }

    /// Ice: very slowly freezes a random orthogonal water neighbour.
    /// Melting is handled by fire and lava.
    fn ice(&mut self, x: i32, y: i32) {
        if !self.rng.chance(ICE_FREEZE) {
            return;
        }
        let off = N4[self.rng.below(4) as usize];
        if let Some((j, Material::Water)) = self.neighbour(x, y, off) {
            self.spawn(j, Material::Ice);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Material::*;

    #[test]
    fn anything_moves_into_empty() {
        for m in Material::ALL {
            assert!(can_displace(m, Empty, Dir::Down));
        }
    }

    #[test]
    fn solids_powders_and_fire_are_never_displaced() {
        for target in [Wall, Wood, Stone, Sand, Ash, Fire, Ice, Plant] {
            for mover in Material::ALL {
                for dir in [Dir::Down, Dir::Up, Dir::Side] {
                    assert!(
                        !can_displace(mover, target, dir),
                        "{mover:?} into {target:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn density_decides_vertical_swaps() {
        assert!(can_displace(Sand, Water, Dir::Down));
        assert!(can_displace(Water, Oil, Dir::Down));
        assert!(!can_displace(Oil, Water, Dir::Down));
        assert!(can_displace(Steam, Water, Dir::Up));
        assert!(!can_displace(Water, Steam, Dir::Up));
    }

    #[test]
    fn gases_do_not_push_sideways() {
        assert!(!can_displace(Steam, Smoke, Dir::Side));
        assert!(can_displace(Water, Oil, Dir::Side));
        assert!(!can_displace(Oil, Water, Dir::Side));
    }
}
