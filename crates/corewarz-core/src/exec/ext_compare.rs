//! `TST`, the value-producing three-way comparison.
//!
//! Classic Redcode can only skip on a comparison. `TST` writes the result into
//! core, which is what makes the extended conditional branches sufficient: any
//! comparison-branch is `TST` followed by one branch.

use crate::core::Core;
use crate::exec::classic_data::StepResult;
use crate::exec::fields::field_pairs;
use crate::exec::operand::Operands;
use crate::instruction::Instruction;
use crate::warrior::WarriorId;

/// Executes `TST`.
///
/// Writes `0` when the compared fields are equal, `1` when the source field is
/// greater, and −1 (`CORESIZE-1`) when it is smaller.
pub fn exec_tst(
    core: &mut Core,
    instr: Instruction,
    ops: Operands,
    owner: WarriorId,
) -> StepResult {
    let mut dst = core.get(ops.b_ptr);
    for (src_f, dst_f) in field_pairs(instr.modifier) {
        let s = src_f.get(&ops.ira);
        let d = dst_f.get(&ops.irb);
        let value = match s.cmp(&d) {
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Less => core.negative_one(),
        };
        dst_f.set(&mut dst, value);
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

    fn run(modifier: Modifier, src: (u32, u32), dst: (u32, u32)) -> Instruction {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        let instr = Instruction {
            opcode: Opcode::Tst,
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
        assert_eq!(exec_tst(&mut core, instr, ops, w), StepResult::Continue);
        core.get(2)
    }

    #[test]
    fn equal_fields_produce_zero() {
        assert_eq!(run(Modifier::B, (0, 42), (0, 42)).b, 0);
    }

    #[test]
    fn a_greater_than_b_produces_one() {
        assert_eq!(run(Modifier::B, (0, 99), (0, 42)).b, 1);
    }

    #[test]
    fn a_less_than_b_produces_negative_one() {
        assert_eq!(run(Modifier::B, (0, 7), (0, 42)).b, 7999);
    }

    #[test]
    fn each_field_pair_is_compared_independently() {
        let r = run(Modifier::F, (10, 99), (10, 42));
        assert_eq!((r.a, r.b), (0, 1));
    }

    #[test]
    fn the_cross_modifier_compares_crosswise() {
        // .X compares source A against destination B, and source B against
        // destination A, writing each result into the destination field it
        // compared against.
        let r = run(Modifier::X, (10, 99), (50, 5));
        assert_eq!(r.b, 1, "source A 10 > destination B 5");
        assert_eq!(r.a, 1, "source B 99 > destination A 50");
    }

    #[test]
    fn tst_result_feeds_a_conditional_branch() {
        // The idiom this opcode exists for: TST then JLZ.
        let r = run(Modifier::B, (0, 3), (0, 9));
        let core = Core::new(&Config::default());
        assert!(core.is_negative(r.b), "3 < 9 must read as negative");
    }
}
