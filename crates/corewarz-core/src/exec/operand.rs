//! ICWS'94 operand evaluation.
//!
//! Resolution runs before every opcode dispatch and is shared by every
//! instruction, classic and extended. It is also the only place where the
//! pre-decrement and post-increment addressing modes mutate the core.

use crate::core::Core;
use crate::instruction::{AddrMode, Instruction};
use crate::warrior::WarriorId;

/// The result of evaluating an instruction's two operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operands {
    /// Folded absolute address the A-operand designates.
    pub a_ptr: u32,
    /// Folded absolute address the B-operand designates.
    pub b_ptr: u32,
    /// Snapshot of the instruction at `a_ptr`, taken during resolution.
    pub ira: Instruction,
    /// Snapshot of the instruction at `b_ptr`, taken during resolution.
    pub irb: Instruction,
}

/// A post-increment deferred until both operands have been resolved.
#[derive(Clone, Copy)]
struct Deferred {
    addr: u32,
    a_field: bool,
}

fn eval_one(
    core: &mut Core,
    pc: u32,
    mode: AddrMode,
    field: u32,
    owner: WarriorId,
    deferred: &mut Option<Deferred>,
) -> u32 {
    match mode {
        AddrMode::Immediate => pc,
        AddrMode::Direct => core.fold(i64::from(pc) + i64::from(field)),
        AddrMode::IndirectA | AddrMode::IndirectB => {
            let ptr = core.fold(i64::from(pc) + i64::from(field));
            let cell = core.get(ptr);
            let off = if mode == AddrMode::IndirectA {
                cell.a
            } else {
                cell.b
            };
            core.fold(i64::from(ptr) + i64::from(off))
        }
        AddrMode::PredecA | AddrMode::PredecB => {
            let ptr = core.fold(i64::from(pc) + i64::from(field));
            let mut cell = core.get(ptr);
            if mode == AddrMode::PredecA {
                cell.a = core.fold(i64::from(cell.a) - 1);
            } else {
                cell.b = core.fold(i64::from(cell.b) - 1);
            }
            core.set(ptr, cell, owner);
            let off = if mode == AddrMode::PredecA {
                cell.a
            } else {
                cell.b
            };
            core.fold(i64::from(ptr) + i64::from(off))
        }
        AddrMode::PostincA | AddrMode::PostincB => {
            let ptr = core.fold(i64::from(pc) + i64::from(field));
            let cell = core.get(ptr);
            let off = if mode == AddrMode::PostincA {
                cell.a
            } else {
                cell.b
            };
            *deferred = Some(Deferred {
                addr: ptr,
                a_field: mode == AddrMode::PostincA,
            });
            core.fold(i64::from(ptr) + i64::from(off))
        }
    }
}

fn apply(core: &mut Core, d: Option<Deferred>, owner: WarriorId) {
    if let Some(d) = d {
        let mut cell = core.get(d.addr);
        if d.a_field {
            cell.a = core.fold(i64::from(cell.a) + 1);
        } else {
            cell.b = core.fold(i64::from(cell.b) + 1);
        }
        core.set(d.addr, cell, owner);
    }
}

/// Evaluates both operands of `instr` executing at `pc`.
///
/// Applies pre-decrement side effects immediately and post-increment side
/// effects after both operands have been evaluated, in A-then-B order, as
/// ICWS'94 requires.
pub fn resolve(core: &mut Core, pc: u32, instr: Instruction, owner: WarriorId) -> Operands {
    let mut da = None;
    let mut db = None;
    let a_ptr = eval_one(core, pc, instr.a_mode, instr.a, owner, &mut da);
    let ira = core.get(a_ptr);
    let b_ptr = eval_one(core, pc, instr.b_mode, instr.b, owner, &mut db);
    let irb = core.get(b_ptr);
    apply(core, da, owner);
    apply(core, db, owner);
    Operands {
        a_ptr,
        b_ptr,
        ira,
        irb,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::instruction::{AddrMode, Instruction, Modifier, Opcode};

    fn setup() -> (Core, WarriorId) {
        (Core::new(&Config::default()), WarriorId(0))
    }

    fn instr(a_mode: AddrMode, a: u32, b_mode: AddrMode, b: u32) -> Instruction {
        Instruction {
            opcode: Opcode::Mov,
            modifier: Modifier::I,
            a_mode,
            a,
            b_mode,
            b,
        }
    }

    #[test]
    fn immediate_points_at_the_instruction_itself() {
        let (mut core, w) = setup();
        let i = instr(AddrMode::Immediate, 7, AddrMode::Immediate, 9);
        let ops = resolve(&mut core, 100, i, w);
        assert_eq!(ops.a_ptr, 100);
        assert_eq!(ops.b_ptr, 100);
    }

    #[test]
    fn direct_is_relative_to_the_program_counter_and_wraps() {
        let (mut core, w) = setup();
        let i = instr(AddrMode::Direct, 5, AddrMode::Direct, 7990);
        let ops = resolve(&mut core, 100, i, w);
        assert_eq!(ops.a_ptr, 105);
        assert_eq!(ops.b_ptr, 90); // 100 + 7990 mod 8000
    }

    #[test]
    fn indirect_b_follows_the_b_field_of_the_pointed_at_cell() {
        let (mut core, w) = setup();
        let mut target = Instruction::DAT_ZERO;
        target.b = 20;
        core.set(110, target, w);
        let i = instr(AddrMode::IndirectB, 10, AddrMode::Direct, 0);
        let ops = resolve(&mut core, 100, i, w);
        assert_eq!(ops.a_ptr, 130); // 100 + 10 -> cell 110, b=20 -> 130
    }

    #[test]
    fn predecrement_b_decrements_before_following() {
        let (mut core, w) = setup();
        let mut target = Instruction::DAT_ZERO;
        target.b = 20;
        core.set(110, target, w);
        let i = instr(AddrMode::PredecB, 10, AddrMode::Direct, 0);
        let ops = resolve(&mut core, 100, i, w);
        assert_eq!(core.get(110).b, 19, "decrement must happen in core");
        assert_eq!(ops.a_ptr, 129);
    }

    #[test]
    fn predecrement_wraps_below_zero() {
        let (mut core, w) = setup();
        let i = instr(AddrMode::PredecB, 10, AddrMode::Direct, 0);
        let _ = resolve(&mut core, 100, i, w);
        assert_eq!(core.get(110).b, 7999);
    }

    #[test]
    fn postincrement_follows_the_old_value_then_increments() {
        let (mut core, w) = setup();
        let mut target = Instruction::DAT_ZERO;
        target.b = 20;
        core.set(110, target, w);
        let i = instr(AddrMode::PostincB, 10, AddrMode::Direct, 0);
        let ops = resolve(&mut core, 100, i, w);
        assert_eq!(ops.a_ptr, 130, "must use the pre-increment value");
        assert_eq!(core.get(110).b, 21, "increment must happen in core");
    }

    #[test]
    fn instruction_registers_are_snapshots_taken_at_resolution() {
        let (mut core, w) = setup();
        let mut src = Instruction::DAT_ZERO;
        src.a = 77;
        core.set(105, src, w);
        let i = instr(AddrMode::Direct, 5, AddrMode::Direct, 5);
        let ops = resolve(&mut core, 100, i, w);
        assert_eq!(ops.ira.a, 77);
        assert_eq!(ops.irb.a, 77);
    }
}
