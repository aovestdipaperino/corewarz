//! Extended bitwise opcodes.
//!
//! These behave exactly as [`crate::exec::classic_data::exec_binop`] does,
//! sharing the same operand evaluation and the same modifier table. Only the
//! arithmetic differs.

use crate::core::Core;
use crate::exec::classic_data::StepResult;
use crate::exec::fields::field_pairs;
use crate::exec::operand::Operands;
use crate::instruction::Instruction;
use crate::warrior::WarriorId;

/// Number of bits in a shift operand's word.
const WORD_BITS: u32 = 32;

/// An extended bitwise operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOp {
    And,
    Or,
    Xor,
    Shl,
    Shr,
    /// Complements the destination field. The source value is ignored.
    Not,
}

/// Executes `AND`, `OR`, `XOR`, `SHL`, `SHR`, or `NOT`.
pub fn exec_bitop(
    core: &mut Core,
    instr: Instruction,
    ops: Operands,
    owner: WarriorId,
    op: BitOp,
) -> StepResult {
    let mut dst = core.get(ops.b_ptr);
    for (src_f, dst_f) in field_pairs(instr.modifier) {
        let s = src_f.get(&ops.ira);
        let d = dst_f.get(&ops.irb);
        let raw: u64 = match op {
            BitOp::And => u64::from(d & s),
            BitOp::Or => u64::from(d | s),
            BitOp::Xor => u64::from(d ^ s),
            BitOp::Not => u64::from(!d),
            BitOp::Shl => {
                if s >= WORD_BITS {
                    0
                } else {
                    u64::from(d) << s
                }
            }
            BitOp::Shr => {
                if s >= WORD_BITS {
                    0
                } else {
                    u64::from(d >> s)
                }
            }
        };
        dst_f.set(&mut dst, core.fold(raw as i64));
    }
    core.set(ops.b_ptr, dst, owner);
    StepResult::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::exec::operand::resolve;
    use crate::instruction::{AddrMode, Modifier, Opcode};

    fn run(modifier: Modifier, op: BitOp, src: (u32, u32), dst: (u32, u32)) -> Instruction {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        let instr = Instruction {
            opcode: Opcode::And,
            modifier,
            a_mode: AddrMode::Direct,
            a: 1,
            b_mode: AddrMode::Direct,
            b: 2,
        };
        core.set(0, instr, w);
        core.set(
            1,
            Instruction {
                a: src.0,
                b: src.1,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        core.set(
            2,
            Instruction {
                a: dst.0,
                b: dst.1,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let ops = resolve(&mut core, 0, instr, w);
        assert_eq!(
            exec_bitop(&mut core, instr, ops, w, op),
            StepResult::Continue
        );
        core.get(2)
    }

    #[test]
    fn and_masks_the_destination() {
        assert_eq!(
            run(Modifier::B, BitOp::And, (0, 0b1100), (0, 0b1010)).b,
            0b1000
        );
    }

    #[test]
    fn or_sets_bits() {
        assert_eq!(
            run(Modifier::B, BitOp::Or, (0, 0b1100), (0, 0b1010)).b,
            0b1110
        );
    }

    #[test]
    fn xor_toggles_bits() {
        assert_eq!(
            run(Modifier::B, BitOp::Xor, (0, 0b1100), (0, 0b1010)).b,
            0b0110
        );
    }

    #[test]
    fn shl_shifts_the_destination_left_by_the_source() {
        assert_eq!(run(Modifier::B, BitOp::Shl, (0, 2), (0, 5)).b, 20);
    }

    #[test]
    fn shl_folds_into_the_core() {
        // 5000 << 1 = 10000, which folds to 2000 in an 8000-cell core.
        assert_eq!(run(Modifier::B, BitOp::Shl, (0, 1), (0, 5000)).b, 2000);
    }

    #[test]
    fn shr_shifts_the_destination_right() {
        assert_eq!(run(Modifier::B, BitOp::Shr, (0, 2), (0, 20)).b, 5);
    }

    #[test]
    fn a_shift_of_at_least_the_word_width_produces_zero() {
        assert_eq!(run(Modifier::B, BitOp::Shl, (0, 32), (0, 5)).b, 0);
        assert_eq!(run(Modifier::B, BitOp::Shr, (0, 64), (0, 5)).b, 0);
    }

    #[test]
    fn not_complements_the_destination_and_ignores_the_source() {
        // !7 as a 32-bit value is 0xFFFF_FFF8; folded into 8000 cells.
        let expected = Core::new(&Config::default()).fold(i64::from(!7u32));
        assert_eq!(run(Modifier::B, BitOp::Not, (0, 999), (0, 7)).b, expected);
    }

    #[test]
    fn not_complements_a_32_bit_word_not_the_core_size() {
        // NOT complements a full 32-bit word before folding, so on the
        // default 8000-cell core `NOT x` behaves as `7295 - x`, not
        // `7999 - x`: !7u32 folded into 8000 cells is 7288, matching
        // `7295 - 7`. This pins that behaviour so it cannot drift silently
        // (see docs/extended-redcode.md).
        assert_eq!(run(Modifier::B, BitOp::Not, (0, 0), (0, 7)).b, 7288);
    }

    #[test]
    fn modifiers_select_fields_exactly_as_add_does() {
        let r = run(Modifier::F, BitOp::Or, (0b0001, 0b0010), (0b0100, 0b1000));
        assert_eq!((r.a, r.b), (0b0101, 0b1010));

        let r = run(Modifier::X, BitOp::Or, (0b0001, 0b0010), (0b0100, 0b1000));
        assert_eq!((r.a, r.b), (0b0110, 0b1001));

        let r = run(Modifier::AB, BitOp::Or, (0b0001, 0), (0, 0b1000));
        assert_eq!(r.b, 0b1001);
    }
}
