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
    ///
    /// # Panics
    /// Panics if `config.core_size` is below [`Config::MIN_CORE_SIZE`].
    /// [`crate::mars::Mars::try_new`] rejects such a config with an error.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        assert!(
            config.core_size >= Config::MIN_CORE_SIZE,
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

    /// Folds `a * b` without overflow for any two `u32` operands.
    #[must_use]
    pub fn fold_product(&self, a: u32, b: u32) -> u32 {
        let product = u64::from(a) * u64::from(b);
        // The remainder is below `size`, which fits in a u32.
        (product % u64::from(self.size)) as u32
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
    fn fold_product_does_not_overflow_for_any_u32_operands() {
        let c = core();
        assert_eq!(
            c.fold_product(u32::MAX, u32::MAX),
            ((u64::from(u32::MAX).pow(2)) % 8000) as u32
        );
        assert_eq!(c.fold_product(7999, 7999), 7999 * 7999 % 8000);
        assert_eq!(c.fold_product(0, u32::MAX), 0);
    }

    #[test]
    #[should_panic(expected = "below a core size")]
    fn new_rejects_a_core_below_the_minimum() {
        let _ = Core::new(&Config {
            core_size: 0,
            ..Config::default()
        });
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

#[cfg(kani)]
mod proofs {
    use super::*;

    fn core_of(size: u32) -> Core {
        Core {
            cells: Vec::new(),
            owners: Vec::new(),
            size,
            writes: Vec::new(),
        }
    }

    fn any_core() -> Core {
        let size: u32 = kani::any();
        kani::assume(size >= 4);
        core_of(size)
    }

    /// `fold` never panics or overflows and is exactly the Euclidean
    /// remainder. A symbolic 64-bit divisor is beyond what CBMC solves in
    /// reasonable time, so this is checked for every `i64` at the standard
    /// ICWS'94 core size of 8000 and at the size the step proofs use.
    fn check_fold(size: u32) {
        let core = core_of(size);
        let v: i64 = kani::any();
        let r = core.fold(v);
        assert!(r < size);
        assert_eq!(i64::from(r), v.rem_euclid(i64::from(size)));
    }

    #[kani::proof]
    fn fold_is_euclidean_remainder_at_8000() {
        check_fold(8000);
    }

    #[kani::proof]
    fn fold_is_euclidean_remainder_at_8() {
        check_fold(8);
    }

    /// For any core size, folding a value already in `0..size` (the
    /// commonest call, `fold(field)`) is the identity, and folding
    /// `addr + offset` for two in-range values lands in range.
    #[kani::proof]
    fn fold_of_an_address_sum_is_in_range_for_any_size() {
        let core = any_core();
        let (x, y): (u32, u32) = (kani::any(), kani::any());
        kani::assume(x < core.size() && y < core.size());
        assert_eq!(core.fold(i64::from(x)), x);
        let s = core.fold(i64::from(x) + i64::from(y));
        assert!(s < core.size());
        let sum = u64::from(x) + u64::from(y);
        let size = u64::from(core.size());
        let expected = if sum >= size { sum - size } else { sum };
        assert_eq!(u64::from(s), expected);
    }

    /// The signed reading splits the core cleanly: every folded value is
    /// exactly one of zero, positive, or negative, and -1 reads as negative.
    #[kani::proof]
    fn signed_reading_is_a_partition() {
        let core = any_core();
        let v: u32 = kani::any();
        kani::assume(v < core.size());
        let classes =
            u8::from(v == 0) + u8::from(core.is_positive(v)) + u8::from(core.is_negative(v));
        assert_eq!(classes, 1);
        assert!(core.is_negative(core.negative_one()));
    }
}
