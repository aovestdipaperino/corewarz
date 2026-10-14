//! `MOV` and the classic arithmetic opcodes.

use crate::core::Core;
use crate::exec::fields::field_pairs;
use crate::exec::operand::Operands;
use crate::instruction::{Instruction, Modifier};
use crate::warrior::WarriorId;

/// What executing one instruction tells the scheduler to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    /// Continue at the next instruction.
    Continue,
    /// Continue at this folded absolute address.
    Jump(u32),
    /// Skip the next instruction.
    Skip,
    /// The process dies.
    Die,
    /// Continue at the next instruction and also spawn a process at this
    /// folded absolute address.
    Split(u32),
}

/// A classic arithmetic operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

/// Executes `MOV`.
pub fn exec_mov(
    core: &mut Core,
    instr: Instruction,
    ops: Operands,
    owner: WarriorId,
) -> StepResult {
    if instr.modifier == Modifier::I {
        core.set(ops.b_ptr, ops.ira, owner);
        return StepResult::Continue;
    }
    let mut dst = core.get(ops.b_ptr);
    for (src_f, dst_f) in field_pairs(instr.modifier) {
        dst_f.set(&mut dst, src_f.get(&ops.ira));
    }
    core.set(ops.b_ptr, dst, owner);
    StepResult::Continue
}

/// Executes `ADD`, `SUB`, `MUL`, `DIV`, or `MOD`.
///
/// Division or modulo by zero kills the process. Under `.F`, `.X`, and `.I`
/// the other field is still computed and written when its own divisor is
/// non-zero, whichever order the fields come in, as the ICWS'94 draft's
/// `emi94.c` and pMARS both do.
pub fn exec_binop(
    core: &mut Core,
    instr: Instruction,
    ops: Operands,
    owner: WarriorId,
    op: BinOp,
) -> StepResult {
    let mut dst = core.get(ops.b_ptr);
    let mut died = false;
    for (src_f, dst_f) in field_pairs(instr.modifier) {
        let s = i64::from(src_f.get(&ops.ira));
        let d = i64::from(dst_f.get(&ops.irb));
        let value = match op {
            BinOp::Add => core.fold(d + s),
            BinOp::Sub => core.fold(d - s),
            // Both fields are below the core size, so the product fits in a
            // u64 even where it would overflow an i64.
            BinOp::Mul => core.fold_product(dst_f.get(&ops.irb), src_f.get(&ops.ira)),
            BinOp::Div | BinOp::Mod if s == 0 => {
                died = true;
                continue;
            }
            BinOp::Div => core.fold(d / s),
            BinOp::Mod => core.fold(d % s),
        };
        dst_f.set(&mut dst, value);
    }
    core.set(ops.b_ptr, dst, owner);
    if died {
        StepResult::Die
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

    /// Runs one instruction at address 0 against a fresh core, returning the
    /// core afterwards along with the step result.
    fn run(instr: Instruction, seed: &[(u32, Instruction)]) -> (Core, StepResult) {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        core.set(0, instr, w);
        for (addr, i) in seed {
            core.set(*addr, *i, w);
        }
        let ops = resolve(&mut core, 0, instr, w);
        let r = match instr.opcode {
            Opcode::Mov => exec_mov(&mut core, instr, ops, w),
            Opcode::Add => exec_binop(&mut core, instr, ops, w, BinOp::Add),
            Opcode::Sub => exec_binop(&mut core, instr, ops, w, BinOp::Sub),
            Opcode::Mul => exec_binop(&mut core, instr, ops, w, BinOp::Mul),
            Opcode::Div => exec_binop(&mut core, instr, ops, w, BinOp::Div),
            Opcode::Mod => exec_binop(&mut core, instr, ops, w, BinOp::Mod),
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
    fn mov_i_copies_the_whole_instruction() {
        let src = i(Opcode::Nop, Modifier::X, 11, 22);
        let (core, r) = run(i(Opcode::Mov, Modifier::I, 1, 2), &[(1, src)]);
        assert_eq!(core.get(2), src);
        assert_eq!(r, StepResult::Continue);
    }

    #[test]
    fn mov_ab_copies_a_field_to_b_field() {
        let (core, _) = run(
            i(Opcode::Mov, Modifier::AB, 1, 2),
            &[(1, dat(11, 22)), (2, dat(33, 44))],
        );
        let d = core.get(2);
        assert_eq!((d.a, d.b), (33, 11));
    }

    #[test]
    fn mov_x_crosses_both_fields() {
        let (core, _) = run(
            i(Opcode::Mov, Modifier::X, 1, 2),
            &[(1, dat(11, 22)), (2, dat(33, 44))],
        );
        let d = core.get(2);
        assert_eq!((d.a, d.b), (22, 11));
    }

    #[test]
    fn add_f_adds_both_fields() {
        let (core, _) = run(
            i(Opcode::Add, Modifier::F, 1, 2),
            &[(1, dat(11, 22)), (2, dat(33, 44))],
        );
        let d = core.get(2);
        assert_eq!((d.a, d.b), (44, 66));
    }

    #[test]
    fn sub_wraps_below_zero() {
        let (core, _) = run(
            i(Opcode::Sub, Modifier::B, 1, 2),
            &[(1, dat(0, 50)), (2, dat(0, 10))],
        );
        assert_eq!(core.get(2).b, 7960);
    }

    #[test]
    fn mul_wraps_modulo_core_size() {
        let (core, _) = run(
            i(Opcode::Mul, Modifier::B, 1, 2),
            &[(1, dat(0, 1000)), (2, dat(0, 9))],
        );
        assert_eq!(core.get(2).b, 1000); // 9000 mod 8000
    }

    #[test]
    fn div_divides_destination_by_source() {
        let (core, _) = run(
            i(Opcode::Div, Modifier::B, 1, 2),
            &[(1, dat(0, 3)), (2, dat(0, 10))],
        );
        assert_eq!(core.get(2).b, 3);
    }

    #[test]
    fn div_by_zero_kills_the_process() {
        let (_, r) = run(
            i(Opcode::Div, Modifier::B, 1, 2),
            &[(1, dat(0, 0)), (2, dat(0, 10))],
        );
        assert_eq!(r, StepResult::Die);
    }

    #[test]
    fn mod_by_zero_kills_the_process() {
        let (_, r) = run(
            i(Opcode::Mod, Modifier::B, 1, 2),
            &[(1, dat(0, 0)), (2, dat(0, 10))],
        );
        assert_eq!(r, StepResult::Die);
    }

    #[test]
    fn div_f_kills_the_process_if_either_divisor_is_zero() {
        let (core, r) = run(
            i(Opcode::Div, Modifier::F, 1, 2),
            &[(1, dat(2, 0)), (2, dat(10, 10))],
        );
        assert_eq!(r, StepResult::Die);
        assert_eq!(
            core.get(2).a,
            5,
            "fields before the zero divisor are still written"
        );
    }

    #[test]
    fn div_f_still_writes_the_b_field_when_only_the_a_divisor_is_zero() {
        let (core, r) = run(
            i(Opcode::Div, Modifier::F, 1, 2),
            &[(1, dat(0, 2)), (2, dat(10, 10))],
        );
        assert_eq!(r, StepResult::Die);
        assert_eq!(core.get(2).a, 10, "zero divisor leaves its field alone");
        assert_eq!(
            core.get(2).b,
            5,
            "fields after the zero divisor are still written"
        );
    }
}
