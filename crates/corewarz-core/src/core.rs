//! Core memory.

use crate::config::Config;
use crate::instruction::Instruction;
use crate::warrior::WarriorId;

/// The circular memory array that warriors fight inside.
///
/// Every address arithmetic operation in CoreWarz goes through [`Core::fold`].
/// This is the only place in the codebase where modular wrapping happens.
#[derive(Debug, Clone)]
pub struct Core {
    cells: Vec<Instruction>,
    owners: Vec<Option<WarriorId>>,
    size: u32,
    writes: Vec<(u32, WarriorId)>,
}

impl Core {
    /// Creates a core of `config.core_size` cells, every cell `DAT.F $0, $0`
    /// and unowned.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        debug_assert!(
            config.core_size >= 4,
            "below a core size of 4, folded signed reading degenerates: \
             negative_one() stops satisfying is_negative()"
        );
        let n = config.core_size as usize;
        Self {
            cells: vec![Instruction::DAT_ZERO; n],
            owners: vec![None; n],
            size: config.core_size,
            writes: Vec::new(),
        }
    }

    /// The number of cells in the core.
    #[must_use]
    pub const fn size(&self) -> u32 {
        self.size
    }

    /// Reduces any integer to a valid core address, wrapping in both
    /// directions.
    #[must_use]
    pub fn fold(&self, value: i64) -> u32 {
        let m = i64::from(self.size);
        (((value % m) + m) % m) as u32
    }

    /// Reads the instruction at `addr`.
    ///
    /// # Panics
    /// Panics if `addr >= self.size()`. Callers must fold first.
    #[must_use]
    pub fn get(&self, addr: u32) -> Instruction {
        self.cells[addr as usize]
    }

    /// Writes `instr` at `addr` and records `warrior` as the cell's owner.
    ///
    /// # Panics
    /// Panics if `addr >= self.size()`. Callers must fold first.
    pub fn set(&mut self, addr: u32, instr: Instruction, warrior: WarriorId) {
        self.cells[addr as usize] = instr;
        self.owners[addr as usize] = Some(warrior);
        self.writes.push((addr, warrior));
    }

    /// The warrior that last wrote `addr`, if any.
    ///
    /// The ownership map has no effect on the rules; it exists so that a
    /// visualizer can colour the battlefield.
    #[must_use]
    pub fn owner(&self, addr: u32) -> Option<WarriorId> {
        self.owners[addr as usize]
    }

    /// Addresses written since [`Core::take_writes`] was last called.
    ///
    /// The VM drains this once per executed instruction to notify observers,
    /// which keeps `Core` free of any observer dependency.
    pub fn take_writes(&mut self) -> Vec<(u32, WarriorId)> {
        std::mem::take(&mut self.writes)
    }

    /// The core's representation of −1.
    #[must_use]
    pub const fn negative_one(&self) -> u32 {
        self.size - 1
    }

    /// Returns `true` if `value` reads as negative under the conventional
    /// signed interpretation: strictly greater than half the core size.
    #[must_use]
    pub const fn is_negative(&self, value: u32) -> bool {
        value > self.size / 2
    }

    /// Returns `true` if `value` reads as positive: non-zero and not negative.
    #[must_use]
    pub const fn is_positive(&self, value: u32) -> bool {
        value != 0 && value <= self.size / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::instruction::Instruction;
    use crate::warrior::WarriorId;

    fn core() -> Core {
        Core::new(&Config {
            core_size: 8000,
            ..Config::default()
        })
    }

    #[test]
    fn new_core_is_all_dat_zero_and_unowned() {
        let c = core();
        assert_eq!(c.size(), 8000);
        for addr in [0u32, 1, 4000, 7999] {
            assert_eq!(c.get(addr), Instruction::DAT_ZERO);
            assert_eq!(c.owner(addr), None);
        }
    }

    #[test]
    fn fold_wraps_both_directions() {
        let c = core();
        assert_eq!(c.fold(0), 0);
        assert_eq!(c.fold(7999), 7999);
        assert_eq!(c.fold(8000), 0);
        assert_eq!(c.fold(8001), 1);
        assert_eq!(c.fold(-1), 7999);
        assert_eq!(c.fold(-8001), 7999);
        assert_eq!(c.fold(24_000), 0);
    }

    #[test]
    fn set_records_the_writing_warrior() {
        let mut c = core();
        let mut i = Instruction::DAT_ZERO;
        i.a = 42;
        c.set(10, i, WarriorId(1));
        assert_eq!(c.get(10).a, 42);
        assert_eq!(c.owner(10), Some(WarriorId(1)));
        assert_eq!(c.owner(11), None);
    }

    #[test]
    fn the_signed_reading_splits_the_core_in_half() {
        let c = core();
        assert_eq!(c.negative_one(), 7999);
        assert!(c.is_negative(7999));
        assert!(c.is_negative(4001));
        assert!(!c.is_negative(4000), "exactly half is not negative");
        assert!(!c.is_negative(0));
        assert!(c.is_positive(1));
        assert!(c.is_positive(4000));
        assert!(!c.is_positive(0), "zero is neither positive nor negative");
        assert!(!c.is_positive(4001));
    }
}
