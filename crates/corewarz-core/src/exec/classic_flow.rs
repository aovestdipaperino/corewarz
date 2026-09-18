//! Classic control-flow, comparison, and process opcodes.

use crate::core::Core;
use crate::exec::classic_data::StepResult;
use crate::exec::fields::field_pairs;
use crate::exec::operand::Operands;
use crate::instruction::{Instruction, Modifier};
use crate::warrior::WarriorId;

/// A skip-style comparison opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipCmp {
    /// Skip if equal (`SEQ`, also spelled `CMP`).
    Seq,
    /// Skip if not equal (`SNE`).
    Sne,
    /// Skip if the A-operand is less than the B-operand (`SLT`).
    Slt,
}

/// Executes `DAT`. Always kills the process.
#[must_use]
pub const fn exec_dat() -> StepResult {
    StepResult::Die
}

/// Executes `NOP`. Always continues.
#[must_use]
pub const fn exec_nop() -> StepResult {
    StepResult::Continue
}

/// Executes `JMP`.
#[must_use]
pub const fn exec_jmp(ops: Operands) -> StepResult {
    StepResult::Jump(ops.a_ptr)
}

/// Executes `JMZ`: jumps when every examined field is zero.
#[must_use]
pub fn exec_jmz(instr: Instruction, ops: Operands) -> StepResult {
    match instr.modifier {
        Modifier::A | Modifier::BA => {
            if ops.irb.a == 0 {
                StepResult::Jump(ops.a_ptr)
            } else {
                StepResult::Continue
            }
        }
        Modifier::B | Modifier::AB => {
            if ops.irb.b == 0 {
                StepResult::Jump(ops.a_ptr)
            } else {
                StepResult::Continue
            }
        }
        Modifier::F | Modifier::X | Modifier::I => {
            if ops.irb.a == 0 && ops.irb.b == 0 {
                StepResult::Jump(ops.a_ptr)
            } else {
                StepResult::Continue
            }
        }
    }
}

/// Executes `JMN`: jumps when any examined field is non-zero.
#[must_use]
pub fn exec_jmn(instr: Instruction, ops: Operands) -> StepResult {
    match instr.modifier {
        Modifier::A | Modifier::BA => {
            if ops.irb.a != 0 {
                StepResult::Jump(ops.a_ptr)
            } else {
                StepResult::Continue
            }
        }
        Modifier::B | Modifier::AB => {
            if ops.irb.b != 0 {
                StepResult::Jump(ops.a_ptr)
            } else {
                StepResult::Continue
            }
        }
        Modifier::F | Modifier::X | Modifier::I => {
            if ops.irb.a != 0 || ops.irb.b != 0 {
                StepResult::Jump(ops.a_ptr)
            } else {
                StepResult::Continue
            }
        }
    }
}

/// Executes `DJN`: decrements the examined fields in core, then jumps when any
/// of them is non-zero.
pub fn exec_djn(
    core: &mut Core,
    instr: Instruction,
    ops: Operands,
    owner: WarriorId,
) -> StepResult {
    let mut dst = core.get(ops.b_ptr);
    let touch_a = matches!(
        instr.modifier,
        Modifier::A | Modifier::BA | Modifier::F | Modifier::X | Modifier::I
    );
    let touch_b = matches!(
        instr.modifier,
        Modifier::B | Modifier::AB | Modifier::F | Modifier::X | Modifier::I
    );
    let mut non_zero = false;
    if touch_a {
        dst.a = core.fold(i64::from(dst.a) - 1);
        non_zero |= dst.a != 0;
    }
    if touch_b {
        dst.b = core.fold(i64::from(dst.b) - 1);
        non_zero |= dst.b != 0;
    }
    core.set(ops.b_ptr, dst, owner);
    if non_zero {
        StepResult::Jump(ops.a_ptr)
    } else {
        StepResult::Continue
    }
}

/// Executes `SPL`.
#[must_use]
pub const fn exec_spl(ops: Operands) -> StepResult {
    StepResult::Split(ops.a_ptr)
}

/// Executes `SEQ`, `SNE`, or `SLT`.
#[must_use]
pub fn exec_skip_cmp(instr: Instruction, ops: Operands, cmp: SkipCmp) -> StepResult {
    let condition = match (cmp, instr.modifier) {
        (SkipCmp::Seq, Modifier::I) => ops.ira == ops.irb,
        (SkipCmp::Sne, Modifier::I) => ops.ira != ops.irb,
        (SkipCmp::Seq, m) => field_pairs(m)
            .iter()
            .all(|(s, d)| s.get(&ops.ira) == d.get(&ops.irb)),
        (SkipCmp::Sne, m) => field_pairs(m)
            .iter()
            .any(|(s, d)| s.get(&ops.ira) != d.get(&ops.irb)),
        (SkipCmp::Slt, m) => field_pairs(m)
            .iter()
            .all(|(s, d)| s.get(&ops.ira) < d.get(&ops.irb)),
    };
    if condition {
        StepResult::Skip
    } else {
        StepResult::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::exec::operand::resolve;
    use crate::instruction::{AddrMode, Instruction, Modifier, Opcode};

    fn run(instr: Instruction, seed: &[(u32, Instruction)]) -> (Core, StepResult) {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        core.set(0, instr, w);
        for (addr, i) in seed {
            core.set(*addr, *i, w);
        }
        let ops = resolve(&mut core, 0, instr, w);
        let r = match instr.opcode {
            Opcode::Dat => exec_dat(),
            Opcode::Nop => exec_nop(),
            Opcode::Jmp => exec_jmp(ops),
            Opcode::Jmz => exec_jmz(instr, ops),
            Opcode::Jmn => exec_jmn(instr, ops),
            Opcode::Djn => exec_djn(&mut core, instr, ops, w),
            Opcode::Spl => exec_spl(ops),
            Opcode::Seq => exec_skip_cmp(instr, ops, SkipCmp::Seq),
            Opcode::Sne => exec_skip_cmp(instr, ops, SkipCmp::Sne),
            Opcode::Slt => exec_skip_cmp(instr, ops, SkipCmp::Slt),
            other => panic!("unsupported in this test: {other:?}"),
        };
        (core, r)
    }

    fn i(opcode: Opcode, modifier: Modifier, a: u32, b: u32) -> Instruction {
        Instruction {
            opcode,
            modifier,
            a_mode: AddrMode::Direct,
            a,
            b_mode: AddrMode::Direct,
            b,
        }
    }

    fn dat(a: u32, b: u32) -> Instruction {
        Instruction {
            a,
            b,
            ..Instruction::DAT_ZERO
        }
    }

    #[test]
    fn dat_kills_and_nop_continues() {
        assert_eq!(
            run(i(Opcode::Dat, Modifier::F, 0, 0), &[]).1,
            StepResult::Die
        );
        assert_eq!(
            run(i(Opcode::Nop, Modifier::F, 0, 0), &[]).1,
            StepResult::Continue
        );
    }

    #[test]
    fn jmp_jumps_to_the_a_operand() {
        assert_eq!(
            run(i(Opcode::Jmp, Modifier::B, 30, 0), &[]).1,
            StepResult::Jump(30)
        );
    }

    #[test]
    fn jmz_b_jumps_only_when_the_b_field_is_zero() {
        let taken = run(i(Opcode::Jmz, Modifier::B, 30, 1), &[(1, dat(9, 0))]).1;
        assert_eq!(taken, StepResult::Jump(30));
        let not_taken = run(i(Opcode::Jmz, Modifier::B, 30, 1), &[(1, dat(0, 9))]).1;
        assert_eq!(not_taken, StepResult::Continue);
    }

    #[test]
    fn jmz_f_requires_both_fields_zero() {
        assert_eq!(
            run(i(Opcode::Jmz, Modifier::F, 30, 1), &[(1, dat(0, 0))]).1,
            StepResult::Jump(30)
        );
        assert_eq!(
            run(i(Opcode::Jmz, Modifier::F, 30, 1), &[(1, dat(0, 5))]).1,
            StepResult::Continue
        );
    }

    #[test]
    fn jmn_f_jumps_when_either_field_is_non_zero() {
        assert_eq!(
            run(i(Opcode::Jmn, Modifier::F, 30, 1), &[(1, dat(0, 5))]).1,
            StepResult::Jump(30)
        );
        assert_eq!(
            run(i(Opcode::Jmn, Modifier::F, 30, 1), &[(1, dat(0, 0))]).1,
            StepResult::Continue
        );
    }

    #[test]
    fn djn_decrements_in_core_then_tests() {
        let (core, r) = run(i(Opcode::Djn, Modifier::B, 30, 1), &[(1, dat(0, 2))]);
        assert_eq!(core.get(1).b, 1);
        assert_eq!(r, StepResult::Jump(30));

        let (core, r) = run(i(Opcode::Djn, Modifier::B, 30, 1), &[(1, dat(0, 1))]);
        assert_eq!(core.get(1).b, 0);
        assert_eq!(r, StepResult::Continue);
    }

    #[test]
    fn djn_wraps_below_zero() {
        let (core, r) = run(i(Opcode::Djn, Modifier::B, 30, 1), &[(1, dat(0, 0))]);
        assert_eq!(core.get(1).b, 7999);
        assert_eq!(r, StepResult::Jump(30));
    }

    #[test]
    fn spl_splits_to_the_a_operand() {
        assert_eq!(
            run(i(Opcode::Spl, Modifier::B, 30, 0), &[]).1,
            StepResult::Split(30)
        );
    }

    #[test]
    fn seq_i_compares_whole_instructions() {
        let x = i(Opcode::Nop, Modifier::X, 5, 6);
        assert_eq!(
            run(i(Opcode::Seq, Modifier::I, 1, 2), &[(1, x), (2, x)]).1,
            StepResult::Skip
        );
        let y = i(Opcode::Nop, Modifier::X, 5, 7);
        assert_eq!(
            run(i(Opcode::Seq, Modifier::I, 1, 2), &[(1, x), (2, y)]).1,
            StepResult::Continue
        );
    }

    #[test]
    fn sne_is_the_negation_of_seq() {
        let x = i(Opcode::Nop, Modifier::X, 5, 6);
        assert_eq!(
            run(i(Opcode::Sne, Modifier::I, 1, 2), &[(1, x), (2, x)]).1,
            StepResult::Continue
        );
    }

    #[test]
    fn slt_skips_when_source_is_less_than_destination() {
        assert_eq!(
            run(
                i(Opcode::Slt, Modifier::B, 1, 2),
                &[(1, dat(0, 3)), (2, dat(0, 9))]
            )
            .1,
            StepResult::Skip
        );
        assert_eq!(
            run(
                i(Opcode::Slt, Modifier::B, 1, 2),
                &[(1, dat(0, 9)), (2, dat(0, 3))]
            )
            .1,
            StepResult::Continue
        );
    }

    #[test]
    fn slt_f_requires_all_pairs_to_hold() {
        assert_eq!(
            run(
                i(Opcode::Slt, Modifier::F, 1, 2),
                &[(1, dat(1, 9)), (2, dat(5, 3))]
            )
            .1,
            StepResult::Continue
        );
    }
}
