//! A tiny, seedable xorshift32 generator.
//!
//! The simulation needs a lot of cheap coin flips (which diagonal to try,
//! whether fire spreads this tick) and it needs them to be reproducible so
//! tests can assert exact outcomes. Xorshift32 is three shifts and three
//! xors per number, keeps its whole state in one `u32` (native on wasm32),
//! and has a period of 2^32 - 1, which is plenty for a toy.

/// Xorshift32 pseudo-random number generator (Marsaglia, 2003).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u32,
}

impl Rng {
    /// Creates a generator from a seed. A zero seed would make xorshift
    /// stick at zero forever, so the seed is first scrambled with a
    /// SplitMix-style finaliser and zero is replaced by a fixed constant.
    pub fn new(seed: u32) -> Self {
        let mut z = seed.wrapping_add(0x9E37_79B9);
        z = (z ^ (z >> 16)).wrapping_mul(0x85EB_CA6B);
        z = (z ^ (z >> 13)).wrapping_mul(0xC2B2_AE35);
        z ^= z >> 16;
        Self {
            state: if z == 0 { 0x6D2B_79F5 } else { z },
        }
    }

    /// Next 32 random bits.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform integer in `0..n`. Uses the multiply-shift trick instead of
    /// `%`, which is faster and avoids most of the modulo bias.
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        ((u64::from(self.next_u32()) * u64::from(n)) >> 32) as u32
    }

    /// True with probability `p / 256`.
    #[inline]
    pub fn chance(&mut self, p: u8) -> bool {
        (self.next_u32() >> 24) < u32::from(p)
    }

    /// A fair coin.
    #[inline]
    pub fn coin(&mut self) -> bool {
        self.next_u32() & 0x8000_0000 != 0
    }

    /// Either `-1` or `1`.
    #[inline]
    pub fn sign(&mut self) -> i32 {
        if self.coin() { 1 } else { -1 }
    }

    /// A random byte, used for per-cell colour variation.
    #[inline]
    pub fn byte(&mut self) -> u8 {
        (self.next_u32() >> 24) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        let same = (0..100).filter(|_| a.next_u32() == b.next_u32()).count();
        assert!(same < 3);
    }

    #[test]
    fn zero_seed_does_not_stick() {
        let mut r = Rng::new(0);
        let first = r.next_u32();
        assert_ne!(first, 0);
        assert_ne!(first, r.next_u32());
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut r = Rng::new(7);
        let mut seen = [false; 6];
        for _ in 0..1000 {
            let v = r.below(6);
            assert!(v < 6);
            seen[v as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn chance_is_roughly_calibrated() {
        let mut r = Rng::new(9);
        let hits = (0..100_000).filter(|_| r.chance(64)).count();
        // Expect 25%.
        assert!((23_000..27_000).contains(&hits), "{hits}");
        assert!(!(0..1000).any(|_| r.chance(0)));
    }
}
