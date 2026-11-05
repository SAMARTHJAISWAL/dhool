//! Materials and their static properties.
//!
//! Every behaviour in the simulation is driven by a handful of numbers per
//! material: its [`Phase`], its [`density`](Material::density) (which decides
//! who sinks through whom), how far a liquid spreads sideways in one tick
//! ([`dispersion`](Material::dispersion)) and how readily it catches fire
//! ([`flammability`](Material::flammability)). The per-material rules in
//! `rules.rs` read these tables rather than hard-coding pairs, so adding a
//! material is mostly a matter of filling in a row here.

use crate::rng::Rng;

/// Broad state of matter, used to pick a movement rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Nothing there.
    Empty,
    /// Never moves on its own (wall, wood, stone, plant, ice).
    Solid,
    /// Falls straight down, then slides diagonally (sand, ash).
    Powder,
    /// Falls, slides diagonally, then flows sideways (water, oil, acid, lava).
    Liquid,
    /// Rises and drifts (steam, smoke).
    Gas,
    /// Fire: short-lived, rises only into empty space, never displaced.
    Plasma,
}

/// A cell's material. The discriminant is the id used across the FFI
/// boundary, in save files and by the front end's palette.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Material {
    /// Empty space.
    #[default]
    Empty = 0,
    /// Indestructible boundary. Nothing affects it, acid included.
    Wall = 1,
    /// Heavy powder. Piles at 45 degrees and sinks through liquids.
    Sand = 2,
    /// The reference liquid. Puts out fire, cools lava, feeds plants.
    Water = 3,
    /// Liquid lighter than water, so it floats on it. Burns fast.
    Oil = 4,
    /// Burns for a short while, spreads to flammables, boils water.
    Fire = 5,
    /// Rises, then condenses back into water.
    Steam = 6,
    /// Static and flammable; burns for a long time.
    Wood = 7,
    /// Static; grows into neighbouring water. Flammable.
    Plant = 8,
    /// Dense liquid that eats most solids and is used up doing it.
    Acid = 9,
    /// Rises and fades away.
    Smoke = 10,
    /// Slow, hot liquid. Ignites flammables; turns to stone in water.
    Lava = 11,
    /// What lava becomes when it is quenched. Static, but acid eats it.
    Stone = 12,
    /// Static. Slowly freezes neighbouring water; melts near heat.
    Ice = 13,
    /// Light powder left behind by fire.
    Ash = 14,
}

/// Number of materials, including [`Material::Empty`].
pub const COUNT: usize = 15;

impl Material {
    /// All materials in id order, so `ALL[m as usize] == m`.
    pub const ALL: [Material; COUNT] = [
        Material::Empty,
        Material::Wall,
        Material::Sand,
        Material::Water,
        Material::Oil,
        Material::Fire,
        Material::Steam,
        Material::Wood,
        Material::Plant,
        Material::Acid,
        Material::Smoke,
        Material::Lava,
        Material::Stone,
        Material::Ice,
        Material::Ash,
    ];

    /// Material for an id, or `None` if the id is unknown.
    pub const fn from_u8(id: u8) -> Option<Self> {
        if (id as usize) < COUNT {
            Some(Self::ALL[id as usize])
        } else {
            None
        }
    }

    /// The id of this material.
    pub const fn id(self) -> u8 {
        self as u8
    }

    /// Human-readable name.
    pub const fn name(self) -> &'static str {
        match self {
            Material::Empty => "Empty",
            Material::Wall => "Wall",
            Material::Sand => "Sand",
            Material::Water => "Water",
            Material::Oil => "Oil",
            Material::Fire => "Fire",
            Material::Steam => "Steam",
            Material::Wood => "Wood",
            Material::Plant => "Plant",
            Material::Acid => "Acid",
            Material::Smoke => "Smoke",
            Material::Lava => "Lava",
            Material::Stone => "Stone",
            Material::Ice => "Ice",
            Material::Ash => "Ash",
        }
    }

    /// State of matter, which selects the movement rule.
    pub const fn phase(self) -> Phase {
        match self {
            Material::Empty => Phase::Empty,
            Material::Wall | Material::Wood | Material::Plant | Material::Stone | Material::Ice => {
                Phase::Solid
            }
            Material::Sand | Material::Ash => Phase::Powder,
            Material::Water | Material::Oil | Material::Acid | Material::Lava => Phase::Liquid,
            Material::Steam | Material::Smoke => Phase::Gas,
            Material::Fire => Phase::Plasma,
        }
    }

    /// Relative density. A falling cell may swap with a liquid or gas below
    /// it only if it is denser; a rising gas may swap with anything fluid
    /// above it that is denser. Solids report `u8::MAX` and never move.
    pub const fn density(self) -> u8 {
        match self {
            Material::Empty => 0,
            Material::Fire => 1,
            Material::Smoke => 2,
            Material::Steam => 3,
            Material::Oil => 40,
            Material::Water => 50,
            Material::Acid => 60,
            Material::Ash => 70,
            Material::Lava => 80,
            Material::Sand => 90,
            Material::Wall | Material::Wood | Material::Plant | Material::Stone | Material::Ice => {
                u8::MAX
            }
        }
    }

    /// How many cells a liquid may travel sideways in one tick. Zero for
    /// anything that is not a liquid.
    pub const fn dispersion(self) -> i32 {
        match self {
            Material::Water => 5,
            Material::Oil => 4,
            Material::Acid => 3,
            Material::Lava => 1,
            _ => 0,
        }
    }

    /// Chance out of 256, per tick and per touching flame, of catching fire.
    pub const fn flammability(self) -> u8 {
        match self {
            Material::Oil => 48,
            Material::Plant => 32,
            Material::Wood => 24,
            _ => 0,
        }
    }

    /// Whether acid can eat this material.
    pub const fn dissolvable(self) -> bool {
        matches!(
            self,
            Material::Sand
                | Material::Wood
                | Material::Plant
                | Material::Stone
                | Material::Ice
                | Material::Ash
        )
    }

    /// Whether a brush stroke of this material is sprinkled (painted with
    /// gaps) rather than laid down solid. Loose materials look and behave
    /// better when they start out slightly scattered.
    pub const fn is_loose(self) -> bool {
        matches!(
            self.phase(),
            Phase::Powder | Phase::Liquid | Phase::Gas | Phase::Plasma
        )
    }

    /// Liquids and gases: things that can be pushed aside by denser matter.
    pub const fn is_fluid(self) -> bool {
        matches!(self.phase(), Phase::Liquid | Phase::Gas)
    }

    /// Initial lifetime in ticks for materials that expire, zero otherwise.
    pub(crate) fn initial_life(self, rng: &mut Rng) -> u8 {
        match self {
            Material::Fire => 20 + rng.below(30) as u8,
            Material::Steam => 100 + rng.below(120) as u8,
            Material::Smoke => 40 + rng.below(80) as u8,
            _ => 0,
        }
    }

    /// Lifetime of the flame that replaces this material when it ignites.
    pub(crate) fn burn_time(self, rng: &mut Rng) -> u8 {
        match self {
            Material::Oil => 16 + rng.below(32) as u8,
            Material::Plant => 30 + rng.below(40) as u8,
            Material::Wood => 60 + rng.below(80) as u8,
            _ => 20 + rng.below(30) as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for (i, m) in Material::ALL.iter().enumerate() {
            assert_eq!(m.id() as usize, i);
            assert_eq!(Material::from_u8(i as u8), Some(*m));
        }
    }

    #[test]
    fn unknown_ids_are_rejected() {
        assert_eq!(Material::from_u8(COUNT as u8), None);
        assert_eq!(Material::from_u8(u8::MAX), None);
    }

    #[test]
    fn density_ordering_matches_the_story() {
        use Material::*;
        assert!(Oil.density() < Water.density(), "oil must float on water");
        assert!(Water.density() < Sand.density(), "sand must sink in water");
        assert!(Steam.density() < Water.density());
        assert!(Smoke.density() < Steam.density());
        assert!(Lava.density() > Water.density());
    }

    #[test]
    fn only_liquids_disperse() {
        for m in Material::ALL {
            assert_eq!(m.dispersion() > 0, m.phase() == Phase::Liquid, "{m:?}");
        }
    }

    #[test]
    fn wall_is_immune_to_everything() {
        assert!(!Material::Wall.dissolvable());
        assert_eq!(Material::Wall.flammability(), 0);
        assert_eq!(Material::Wall.phase(), Phase::Solid);
    }

    #[test]
    fn names_are_unique() {
        let mut names: Vec<_> = Material::ALL.iter().map(|m| m.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), COUNT);
    }
}
