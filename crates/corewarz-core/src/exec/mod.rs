//! Instruction execution: operand resolution and opcode dispatch.

pub mod classic_data;
pub mod classic_flow;
pub mod fields;
pub mod operand;

use crate::core::Core;
use crate::instruction::{Instruction, Opcode};
use crate::warrior::WarriorId;
use classic_data::{exec_binop, exec_mov, BinOp, StepResult};
use classic_flow::{
    exec_dat, exec_djn, exec_jmn, exec_jmp, exec_jmz, exec_nop, exec_skip_cmp, exec_spl, SkipCmp,
};
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
        other => unimplemented!("extended opcode {other:?} arrives in a later task"),
    }
}
