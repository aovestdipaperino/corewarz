//! Running battles.

use corewarz_core::mars::AdmissionError;
use corewarz_core::{Config, Mars, NullObserver, Outcome, Warrior};

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
