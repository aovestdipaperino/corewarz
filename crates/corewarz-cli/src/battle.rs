//! Running battles and placing warriors.

use corewarz_core::mars::AdmissionError;
use corewarz_core::{Config, Mars, NullObserver, Outcome, Warrior};

/// A deterministic 64-bit pseudo-random generator (SplitMix64).
///
/// Battles must be reproducible from a seed, so the CLI never uses system
/// randomness.
pub struct Rng(u64);

impl Rng {
    /// Creates a generator from `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Returns the next value.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Places `count` warriors, the first at 0 and the rest at pseudo-random
/// positions respecting `config.min_separation`.
///
/// Placement is a rejection sample; with the default configuration it
/// terminates quickly. After 10,000 failed attempts it falls back to even
/// spacing, which always satisfies the separation rule for any sane
/// configuration.
#[must_use]
pub fn random_positions(config: &Config, count: usize, seed: u64) -> Vec<u32> {
    let mut rng = Rng::new(seed);
    let mut positions = vec![0u32];
    for _ in 1..count {
        let mut placed = None;
        for _ in 0..10_000 {
            let candidate = u32::try_from(rng.next_u64() % u64::from(config.core_size))
                .expect("core size fits in u32");
            let mut trial = positions.clone();
            trial.push(candidate);
            if Mars::check_positions(config, &trial).is_ok() {
                placed = Some(candidate);
                break;
            }
        }
        let next = placed.unwrap_or_else(|| {
            let step = config.core_size / u32::try_from(count).unwrap_or(1).max(1);
            step * u32::try_from(positions.len()).unwrap_or(1)
        });
        positions.push(next);
    }
    positions
}

/// Runs a single battle to completion.
///
/// # Errors
/// Returns an [`AdmissionError`] if the warriors cannot be admitted.
pub fn run_battle(
    warriors: &[Warrior],
    config: &Config,
    positions: &[u32],
) -> Result<Outcome, AdmissionError> {
    let mut mars = Mars::try_new(*config, warriors.to_vec(), positions)?;
    Ok(mars.run(&mut NullObserver))
}
