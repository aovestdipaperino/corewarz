//! Extended conditional branches.
//!
//! Each branch tests the B-operand against zero and jumps to the A-operand.
//! Testing against zero rather than comparing two operands is what keeps these
//! instructions to two operands; [`crate::exec::ext_compare::exec_tst`] makes
//! that sufficient.

use crate::core::Core;
use crate::exec::classic_data::StepResult;
use crate::exec::operand::Operands;
use crate::instruction::{Instruction, Modifier};

/// An extended conditional branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    /// Jump if zero.
    Jez,
    /// Jump if non-zero.
    Jnz,
    /// Jump if negative under the signed reading.
    Jlz,
    /// Jump if positive under the signed reading.
    Jgz,
}

/// Executes `JEZ`, `JNZ`, `JLZ`, or `JGZ`.
///
/// Under `.F` and `.X` the condition must hold for both fields. The assembler
/// rejects `.AB`, `.BA`, and `.I` on these opcodes, so those modifiers cannot
/// reach here; they are treated as `.F` defensively.
#[must_use]
pub fn exec_branch(core: &Core, instr: Instruction, ops: Operands, branch: Branch) -> StepResult {
    let values: &[u32] = match instr.modifier {
        Modifier::A => &[ops.irb.a],
        Modifier::B => &[ops.irb.b],
        _ => &[ops.irb.a, ops.irb.b],
    };
    let holds = |v: u32| match branch {
        Branch::Jez => v == 0,
        Branch::Jnz => v != 0,
        Branch::Jlz => core.is_negative(v),
        Branch::Jgz => core.is_positive(v),
    };
    if values.iter().copied().all(holds) {
        StepResult::Jump(ops.a_ptr)
    } else {
        StepResult::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::exec::operand::resolve;
    use crate::instruction::{AddrMode, Modifier, Opcode};
    use crate::warrior::WarriorId;

    fn run(branch: Branch, modifier: Modifier, test: (u32, u32)) -> StepResult {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        let instr = Instruction {
            opcode: Opcode::Jez,
            modifier,
            a_mode: AddrMode::Direct,
            a: 30,
            b_mode: AddrMode::Direct,
            b: 2,
        };
        core.set(0, instr, w);
        core.set(
            2,
            Instruction {
                a: test.0,
                b: test.1,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let ops = resolve(&mut core, 0, instr, w);
        exec_branch(&core, instr, ops, branch)
    }

    const TAKEN: StepResult = StepResult::Jump(30);

    #[test]
    fn jez_jumps_when_the_tested_field_is_zero() {
        assert_eq!(run(Branch::Jez, Modifier::B, (9, 0)), TAKEN);
        assert_eq!(run(Branch::Jez, Modifier::B, (0, 9)), StepResult::Continue);
    }

    #[test]
    fn jnz_jumps_when_the_tested_field_is_non_zero() {
        assert_eq!(run(Branch::Jnz, Modifier::B, (0, 9)), TAKEN);
        assert_eq!(run(Branch::Jnz, Modifier::B, (9, 0)), StepResult::Continue);
    }

    #[test]
    fn jlz_uses_the_signed_reading() {
        assert_eq!(
            run(Branch::Jlz, Modifier::B, (0, 7999)),
            TAKEN,
            "-1 is negative"
        );
        assert_eq!(run(Branch::Jlz, Modifier::B, (0, 4001)), TAKEN);
        assert_eq!(
            run(Branch::Jlz, Modifier::B, (0, 4000)),
            StepResult::Continue
        );
        assert_eq!(run(Branch::Jlz, Modifier::B, (0, 0)), StepResult::Continue);
    }

    #[test]
    fn jgz_requires_non_zero_and_not_negative() {
        assert_eq!(run(Branch::Jgz, Modifier::B, (0, 1)), TAKEN);
        assert_eq!(run(Branch::Jgz, Modifier::B, (0, 4000)), TAKEN);
        assert_eq!(run(Branch::Jgz, Modifier::B, (0, 0)), StepResult::Continue);
        assert_eq!(
            run(Branch::Jgz, Modifier::B, (0, 7999)),
            StepResult::Continue
        );
    }

    #[test]
    fn the_a_modifier_examines_the_a_field() {
        assert_eq!(run(Branch::Jez, Modifier::A, (0, 9)), TAKEN);
        assert_eq!(run(Branch::Jez, Modifier::A, (9, 0)), StepResult::Continue);
    }

    #[test]
    fn f_and_x_require_the_condition_on_both_fields() {
        assert_eq!(run(Branch::Jez, Modifier::F, (0, 0)), TAKEN);
        assert_eq!(run(Branch::Jez, Modifier::F, (0, 9)), StepResult::Continue);
        assert_eq!(run(Branch::Jnz, Modifier::X, (3, 4)), TAKEN);
        assert_eq!(run(Branch::Jnz, Modifier::X, (3, 0)), StepResult::Continue);
    }
}
