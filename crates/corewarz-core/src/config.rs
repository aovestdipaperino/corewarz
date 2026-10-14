//! Battle configuration.

/// Which Redcode dialect a battle uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dialect {
    /// Full ICWS'94 fidelity. Extended opcodes are rejected.
    #[default]
    Classic,
    /// ICWS'94 plus the CoreWarz extensions.
    Extended,
}

/// The parameters of a battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Number of cells in the core.
    pub core_size: u32,
    /// Cycles after which an undecided battle is a tie.
    pub max_cycles: u32,
    /// Maximum simultaneous processes per warrior.
    pub max_processes: u32,
    /// Maximum instructions in a loaded warrior.
    pub max_length: u32,
    /// Minimum distance between two warriors' load addresses.
    pub min_separation: u32,
    /// Which dialect this battle accepts.
    pub dialect: Dialect,
}

impl Config {
    /// The smallest core a battle accepts. Below 4 cells, folded signed
    /// reading degenerates and a size of 0 cannot fold at all.
    pub const MIN_CORE_SIZE: u32 = 4;
}

impl Default for Config {
    fn default() -> Self {
        Self {
            core_size: 8000,
            max_cycles: 80_000,
            max_processes: 8000,
            max_length: 100,
            min_separation: 100,
            dialect: Dialect::Classic,
        }
    }
}
