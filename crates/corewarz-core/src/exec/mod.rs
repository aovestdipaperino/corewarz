//! Instruction execution: operand resolution and opcode dispatch.

pub mod classic_data;
pub mod classic_flow;
pub mod ext_bitwise;
pub mod ext_branch;
pub mod ext_compare;
pub mod ext_stack;
pub mod fields;
pub mod operand;

use crate::core::Core;
use crate::instruction::{Instruction, Opcode};
use crate::warrior::WarriorId;
use classic_data::{exec_binop, exec_mov, BinOp, StepResult};
use classic_flow::{
    exec_dat, exec_djn, exec_jmn, exec_jmp, exec_jmz, exec_nop, exec_skip_cmp, exec_spl, SkipCmp,
};
use ext_bitwise::{exec_bitop, BitOp};
use ext_branch::{exec_branch, Branch};
use ext_compare::exec_tst;
use ext_stack::{exec_call, exec_pop, exec_push, exec_ret};
use operand::{resolve, Operands};

/// Resolves operands and executes one instruction.
///
/// This is the single dispatch point for every opcode. It contains no dialect
/// branch: extended opcodes are rejected at assembly and warrior admission, so
/// by the time an instruction reaches here it is legal.
pub fn dispatch(core: &mut Core, pc: u32, owner: WarriorId) -> StepResult {
    let instr: Instruction = core.get(pc);
    let ops: Operands = resolve(core, pc, instr, owner);
    match instr.opcode {
        Opcode::Dat => exec_dat(),
        Opcode::Nop => exec_nop(),
        Opcode::Mov => exec_mov(core, instr, ops, owner),
        Opcode::Add => exec_binop(core, instr, ops, owner, BinOp::Add),
        Opcode::Sub => exec_binop(core, instr, ops, owner, BinOp::Sub),
        Opcode::Mul => exec_binop(core, instr, ops, owner, BinOp::Mul),
        Opcode::Div => exec_binop(core, instr, ops, owner, BinOp::Div),
        Opcode::Mod => exec_binop(core, instr, ops, owner, BinOp::Mod),
        Opcode::Jmp => exec_jmp(ops),
        Opcode::Jmz => exec_jmz(instr, ops),
        Opcode::Jmn => exec_jmn(instr, ops),
        Opcode::Djn => exec_djn(core, instr, ops, owner),
        Opcode::Spl => exec_spl(ops),
        Opcode::Seq => exec_skip_cmp(instr, ops, SkipCmp::Seq),
        Opcode::Sne => exec_skip_cmp(instr, ops, SkipCmp::Sne),
        Opcode::Slt => exec_skip_cmp(instr, ops, SkipCmp::Slt),
        Opcode::And => exec_bitop(core, instr, ops, owner, BitOp::And),
        Opcode::Or => exec_bitop(core, instr, ops, owner, BitOp::Or),
        Opcode::Xor => exec_bitop(core, instr, ops, owner, BitOp::Xor),
        Opcode::Shl => exec_bitop(core, instr, ops, owner, BitOp::Shl),
        Opcode::Shr => exec_bitop(core, instr, ops, owner, BitOp::Shr),
        Opcode::Not => exec_bitop(core, instr, ops, owner, BitOp::Not),
        Opcode::Tst => exec_tst(core, instr, ops, owner),
        Opcode::Jez => exec_branch(&*core, instr, ops, Branch::Jez),
        Opcode::Jnz => exec_branch(&*core, instr, ops, Branch::Jnz),
        Opcode::Jlz => exec_branch(&*core, instr, ops, Branch::Jlz),
        Opcode::Jgz => exec_branch(&*core, instr, ops, Branch::Jgz),
        Opcode::Call => exec_call(core, pc, ops, owner),
        Opcode::Ret => exec_ret(core, ops, owner),
        Opcode::Push => exec_push(core, ops, owner),
        Opcode::Pop => exec_pop(core, ops, owner),
    }
}

/// Drives every extended opcode through [`dispatch`] itself, not through its
/// `exec_*` function directly.
///
/// Each `exec_*` function has its own unit tests, but every one of them calls
/// its function directly, bypassing the `match` in `dispatch`. A mis-mapped
/// arm (say, `Opcode::Xor => BitOp::Or`, or `Opcode::Jlz => Branch::Jgz`)
/// would be invisible to the whole suite. These tests choose inputs where a
/// wrong mapping produces a different, observable result.
#[cfg(test)]
mod dispatch_wiring_tests {
    use super::*;
    use crate::config::Config;
    use crate::instruction::{AddrMode, Instruction, Modifier};

    fn bitop_instr(opcode: Opcode) -> Instruction {
        Instruction {
            opcode,
            modifier: Modifier::B,
            a_mode: AddrMode::Direct,
            a: 1,
            b_mode: AddrMode::Direct,
            b: 2,
        }
    }

    /// Runs a two-operand extended opcode through `dispatch` with A-cell at 1
    /// and B-cell at 2, both holding the given B-field values, and returns the
    /// resulting B-field of the B-cell plus the `StepResult`.
    fn run_bitop(opcode: Opcode, src_b: u32, dst_b: u32) -> (u32, StepResult) {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        core.set(0, bitop_instr(opcode), w);
        core.set(
            1,
            Instruction {
                b: src_b,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        core.set(
            2,
            Instruction {
                b: dst_b,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let r = dispatch(&mut core, 0, w);
        (core.get(2).b, r)
    }

    #[test]
    fn and_computes_the_bitwise_and() {
        // 12 & 10 = 8; OR would give 14, XOR would give 6.
        let (result, r) = run_bitop(Opcode::And, 12, 10);
        assert_eq!(result, 8);
        assert_eq!(r, StepResult::Continue);
    }

    #[test]
    fn or_computes_the_bitwise_or() {
        // 12 | 10 = 14; AND would give 8, XOR would give 6.
        let (result, _) = run_bitop(Opcode::Or, 12, 10);
        assert_eq!(result, 14);
    }

    #[test]
    fn xor_computes_the_bitwise_xor() {
        // 12 ^ 10 = 6; AND would give 8, OR would give 14.
        let (result, _) = run_bitop(Opcode::Xor, 12, 10);
        assert_eq!(result, 6);
    }

    #[test]
    fn shl_shifts_left() {
        // 1 << 2 = 4; SHR of the same operands gives 0.
        let (result, _) = run_bitop(Opcode::Shl, 2, 1);
        assert_eq!(result, 4);
    }

    #[test]
    fn shr_shifts_right() {
        // 4 >> 1 = 2; SHL of the same operands gives 8.
        let (result, _) = run_bitop(Opcode::Shr, 1, 4);
        assert_eq!(result, 2);
    }

    #[test]
    fn not_complements_the_destination_and_ignores_the_source() {
        let core = Core::new(&Config::default());
        let expected = core.fold(i64::from(!0u32));
        let (result, _) = run_bitop(Opcode::Not, 999, 0);
        assert_eq!(result, expected);
        // Confirm this differs from AND/OR/XOR/SHL/SHR on the same inputs,
        // i.e. NOT genuinely ignores the source field.
        assert_ne!(result, 0);
    }

    #[test]
    fn tst_writes_the_three_way_comparison() {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        let instr = Instruction {
            opcode: Opcode::Tst,
            modifier: Modifier::B,
            a_mode: AddrMode::Direct,
            a: 1,
            b_mode: AddrMode::Direct,
            b: 2,
        };
        core.set(0, instr, w);
        core.set(
            1,
            Instruction {
                b: 5,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        core.set(
            2,
            Instruction {
                b: 3,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let r = dispatch(&mut core, 0, w);
        assert_eq!(r, StepResult::Continue);
        assert_eq!(core.get(2).b, 1, "source greater than dest produces 1");
    }

    fn branch_instr(opcode: Opcode) -> Instruction {
        Instruction {
            opcode,
            modifier: Modifier::B,
            a_mode: AddrMode::Direct,
            a: 30,
            b_mode: AddrMode::Direct,
            b: 2,
        }
    }

    /// Runs a branch opcode through `dispatch` with the tested B-field set to
    /// `value`, and returns the `StepResult`.
    fn run_branch(opcode: Opcode, value: u32) -> StepResult {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        core.set(0, branch_instr(opcode), w);
        core.set(
            2,
            Instruction {
                b: value,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        dispatch(&mut core, 0, w)
    }

    const TAKEN: StepResult = StepResult::Jump(30);

    // Truth table over {0, 1, 7999} is unique per opcode, so any pairwise
    // swap of branch arms (e.g. Jlz <-> Jgz) is caught by at least one of
    // these three assertions.
    #[test]
    fn jez_jumps_only_when_zero() {
        assert_eq!(run_branch(Opcode::Jez, 0), TAKEN);
        assert_eq!(run_branch(Opcode::Jez, 1), StepResult::Continue);
        assert_eq!(run_branch(Opcode::Jez, 7999), StepResult::Continue);
    }

    #[test]
    fn jnz_jumps_whenever_non_zero() {
        assert_eq!(run_branch(Opcode::Jnz, 0), StepResult::Continue);
        assert_eq!(run_branch(Opcode::Jnz, 1), TAKEN);
        assert_eq!(run_branch(Opcode::Jnz, 7999), TAKEN);
    }

    #[test]
    fn jlz_jumps_only_when_negative() {
        assert_eq!(run_branch(Opcode::Jlz, 0), StepResult::Continue);
        assert_eq!(run_branch(Opcode::Jlz, 1), StepResult::Continue);
        assert_eq!(run_branch(Opcode::Jlz, 7999), TAKEN);
    }

    #[test]
    fn jgz_jumps_only_when_positive() {
        assert_eq!(run_branch(Opcode::Jgz, 0), StepResult::Continue);
        assert_eq!(run_branch(Opcode::Jgz, 1), TAKEN);
        assert_eq!(run_branch(Opcode::Jgz, 7999), StepResult::Continue);
    }

    #[test]
    fn call_pushes_and_jumps_through_dispatch() {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        let instr = Instruction {
            opcode: Opcode::Call,
            modifier: Modifier::F,
            a_mode: AddrMode::Direct,
            a: 300,
            b_mode: AddrMode::Direct,
            b: 10,
        };
        core.set(0, instr, w);
        core.set(
            10,
            Instruction {
                b: 490,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let r = dispatch(&mut core, 0, w);
        assert_eq!(r, StepResult::Jump(300));
        assert_eq!(
            core.get(500).b,
            1,
            "pushed pc + 1, not RET's or PUSH's shape"
        );
        assert_eq!(core.get(10).b, 491, "stack pointer advanced");
    }

    #[test]
    fn ret_pops_and_jumps_through_dispatch() {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        let instr = Instruction {
            opcode: Opcode::Ret,
            modifier: Modifier::F,
            a_mode: AddrMode::Direct,
            a: 10,
            b_mode: AddrMode::Direct,
            b: 0,
        };
        core.set(0, instr, w);
        core.set(
            10,
            Instruction {
                b: 491,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        core.set(
            500,
            Instruction {
                b: 42,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let r = dispatch(&mut core, 0, w);
        assert_eq!(r, StepResult::Jump(42), "distinct from CALL's Jump(300)");
        assert_eq!(core.get(10).b, 490, "stack pointer retreated");
    }

    #[test]
    fn push_writes_a_value_and_advances_through_dispatch() {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        let instr = Instruction {
            opcode: Opcode::Push,
            modifier: Modifier::F,
            a_mode: AddrMode::Direct,
            a: 20,
            b_mode: AddrMode::Direct,
            b: 10,
        };
        core.set(0, instr, w);
        core.set(
            10,
            Instruction {
                b: 490,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        core.set(
            20,
            Instruction {
                b: 4321,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let r = dispatch(&mut core, 0, w);
        assert_eq!(r, StepResult::Continue, "unlike CALL/RET, PUSH never jumps");
        assert_eq!(
            core.get(500).b,
            4321,
            "pushed the A-operand's B-field value"
        );
        assert_eq!(core.get(10).b, 491);
    }

    #[test]
    fn pop_reads_and_retreats_through_dispatch() {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        core.set(
            10,
            Instruction {
                b: 491,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        core.set(
            500,
            Instruction {
                b: 777,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let instr = Instruction {
            opcode: Opcode::Pop,
            modifier: Modifier::F,
            a_mode: AddrMode::Direct,
            a: 21,
            b_mode: AddrMode::Direct,
            b: 10,
        };
        core.set(0, instr, w);
        let r = dispatch(&mut core, 0, w);
        assert_eq!(r, StepResult::Continue);
        assert_eq!(
            core.get(21).b,
            777,
            "popped value landed in the A-operand's cell"
        );
        assert_eq!(core.get(10).b, 490, "stack pointer retreated");
    }
}
