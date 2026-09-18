//! Core-resident subroutines.
//!
//! The stack lives in the core and is therefore fully attackable: bombing a
//! stack-pointer cell redirects the victim's next `RET`, and the stack itself
//! can be read, corrupted, or overwritten with a chosen return address. This
//! is deliberate. No process has private state.
//!
//! A *stack-pointer cell* holds the current stack pointer in its B-field. The
//! stack grows upward and wraps like all core arithmetic.

use crate::core::Core;
use crate::exec::classic_data::StepResult;
use crate::exec::operand::Operands;
use crate::warrior::WarriorId;

/// Reads the stack pointer out of the cell at `sp_cell`.
fn load_sp(core: &Core, sp_cell: u32) -> u32 {
    core.get(sp_cell).b
}

/// Writes `value` back into the stack-pointer cell at `sp_cell`.
fn store_sp(core: &mut Core, sp_cell: u32, value: u32, owner: WarriorId) {
    let mut cell = core.get(sp_cell);
    cell.b = value;
    core.set(sp_cell, cell, owner);
}

/// Writes `value` into the B-field of the cell at `addr`.
fn store_b(core: &mut Core, addr: u32, value: u32, owner: WarriorId) {
    let mut cell = core.get(addr);
    cell.b = value;
    core.set(addr, cell, owner);
}

/// Executes `CALL target, sp`.
///
/// Pushes `pc + 1` onto the stack, advances the stack pointer, and jumps to
/// the A-operand.
pub fn exec_call(core: &mut Core, pc: u32, ops: Operands, owner: WarriorId) -> StepResult {
    let sp_cell = ops.b_ptr;
    let sp = load_sp(core, sp_cell);
    let return_address = core.fold(i64::from(pc) + 1);
    store_b(core, sp, return_address, owner);
    store_sp(core, sp_cell, core.fold(i64::from(sp) + 1), owner);
    StepResult::Jump(ops.a_ptr)
}

/// Executes `RET sp`.
///
/// Retreats the stack pointer and jumps to the address stored there. A
/// corrupted pointer or a corrupted stack simply sends the process somewhere
/// else; there is no error condition.
pub fn exec_ret(core: &mut Core, ops: Operands, owner: WarriorId) -> StepResult {
    let sp_cell = ops.a_ptr;
    let sp = core.fold(i64::from(load_sp(core, sp_cell)) - 1);
    store_sp(core, sp_cell, sp, owner);
    let target = core.get(sp).b;
    StepResult::Jump(target)
}

/// Executes `PUSH val, sp`.
///
/// Pushes the value in the B-field of the A-operand's cell and advances the
/// stack pointer.
///
/// `PUSH` pushes a value, not an address: that is what makes it useful for
/// saving data and symmetric with [`exec_pop`].
pub fn exec_push(core: &mut Core, ops: Operands, owner: WarriorId) -> StepResult {
    let sp_cell = ops.b_ptr;
    let sp = load_sp(core, sp_cell);
    store_b(core, sp, ops.ira.b, owner);
    store_sp(core, sp_cell, core.fold(i64::from(sp) + 1), owner);
    StepResult::Continue
}

/// Executes `POP dst, sp`.
///
/// Retreats the stack pointer and writes the value there into the B-field of
/// the A-operand's cell.
pub fn exec_pop(core: &mut Core, ops: Operands, owner: WarriorId) -> StepResult {
    let sp_cell = ops.b_ptr;
    let sp = core.fold(i64::from(load_sp(core, sp_cell)) - 1);
    store_sp(core, sp_cell, sp, owner);
    let value = core.get(sp).b;
    store_b(core, ops.a_ptr, value, owner);
    StepResult::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::exec::operand::resolve;
    use crate::instruction::{AddrMode, Instruction, Modifier, Opcode};

    const SP_CELL: u32 = 10;
    const STACK_BASE: u32 = 500;

    /// Builds a core with a stack-pointer cell at [`SP_CELL`] pointing at
    /// [`STACK_BASE`], executes `instr` at address 0, and returns the core and
    /// step result.
    fn run(opcode: Opcode, a: u32, b: u32, sp_value: u32) -> (Core, StepResult) {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        let instr = Instruction {
            opcode,
            modifier: Modifier::F,
            a_mode: AddrMode::Direct,
            a,
            b_mode: AddrMode::Direct,
            b,
        };
        core.set(0, instr, w);
        core.set(
            SP_CELL,
            Instruction {
                b: sp_value,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let ops = resolve(&mut core, 0, instr, w);
        let r = match opcode {
            Opcode::Call => exec_call(&mut core, 0, ops, w),
            Opcode::Ret => exec_ret(&mut core, ops, w),
            Opcode::Push => exec_push(&mut core, ops, w),
            Opcode::Pop => exec_pop(&mut core, ops, w),
            other => panic!("not a stack opcode: {other:?}"),
        };
        (core, r)
    }

    #[test]
    fn call_pushes_the_return_address_and_jumps() {
        let (core, r) = run(Opcode::Call, 300, SP_CELL, STACK_BASE);
        assert_eq!(r, StepResult::Jump(300), "jumps to the A-operand");
        assert_eq!(core.get(STACK_BASE).b, 1, "return address is pc + 1");
        assert_eq!(
            core.get(SP_CELL).b,
            STACK_BASE + 1,
            "stack pointer advanced"
        );
    }

    #[test]
    fn ret_pops_the_return_address_and_jumps_there() {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        let instr = Instruction {
            opcode: Opcode::Ret,
            modifier: Modifier::F,
            a_mode: AddrMode::Direct,
            a: SP_CELL,
            b_mode: AddrMode::Direct,
            b: 0,
        };
        core.set(0, instr, w);
        core.set(
            SP_CELL,
            Instruction {
                b: STACK_BASE + 1,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        core.set(
            STACK_BASE,
            Instruction {
                b: 42,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let ops = resolve(&mut core, 0, instr, w);
        assert_eq!(exec_ret(&mut core, ops, w), StepResult::Jump(42));
        assert_eq!(core.get(SP_CELL).b, STACK_BASE, "stack pointer retreated");
    }

    #[test]
    fn call_and_ret_are_inverses() {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        let call = Instruction {
            opcode: Opcode::Call,
            modifier: Modifier::F,
            a_mode: AddrMode::Direct,
            a: 300,
            b_mode: AddrMode::Direct,
            b: SP_CELL,
        };
        core.set(0, call, w);
        core.set(
            SP_CELL,
            Instruction {
                b: STACK_BASE,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let ops = resolve(&mut core, 0, call, w);
        assert_eq!(exec_call(&mut core, 0, ops, w), StepResult::Jump(300));

        let ret = Instruction {
            opcode: Opcode::Ret,
            modifier: Modifier::F,
            a_mode: AddrMode::Direct,
            a: core.fold(i64::from(SP_CELL) - 300),
            b_mode: AddrMode::Direct,
            b: 0,
        };
        core.set(300, ret, w);
        let ops = resolve(&mut core, 300, ret, w);
        assert_eq!(
            exec_ret(&mut core, ops, w),
            StepResult::Jump(1),
            "returns after the CALL"
        );
        assert_eq!(
            core.get(SP_CELL).b,
            STACK_BASE,
            "pointer is back where it started"
        );
    }

    #[test]
    fn push_and_pop_move_a_value_through_the_stack() {
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        core.set(
            SP_CELL,
            Instruction {
                b: STACK_BASE,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        core.set(
            20,
            Instruction {
                b: 1234,
                ..Instruction::DAT_ZERO
            },
            w,
        );

        let push = Instruction {
            opcode: Opcode::Push,
            modifier: Modifier::F,
            a_mode: AddrMode::Direct,
            a: 20,
            b_mode: AddrMode::Direct,
            b: SP_CELL,
        };
        core.set(0, push, w);
        let ops = resolve(&mut core, 0, push, w);
        assert_eq!(exec_push(&mut core, ops, w), StepResult::Continue);
        assert_eq!(core.get(STACK_BASE).b, 1234);
        assert_eq!(core.get(SP_CELL).b, STACK_BASE + 1);

        let pop = Instruction {
            opcode: Opcode::Pop,
            modifier: Modifier::F,
            a_mode: AddrMode::Direct,
            a: 21,
            b_mode: AddrMode::Direct,
            b: SP_CELL,
        };
        core.set(0, pop, w);
        let ops = resolve(&mut core, 0, pop, w);
        assert_eq!(exec_pop(&mut core, ops, w), StepResult::Continue);
        assert_eq!(core.get(21).b, 1234, "the value came back");
        assert_eq!(core.get(SP_CELL).b, STACK_BASE);
    }

    #[test]
    fn the_stack_pointer_wraps() {
        let (core, _) = run(Opcode::Call, 300, SP_CELL, 7999);
        assert_eq!(core.get(SP_CELL).b, 0, "advancing past the end wraps");
    }

    #[test]
    fn a_bombed_stack_pointer_sends_ret_somewhere_harmless_to_the_vm() {
        // The stack pointer has been overwritten with garbage. RET must not
        // panic; it simply jumps wherever the garbage points.
        let mut core = Core::new(&Config::default());
        let w = WarriorId(0);
        let instr = Instruction {
            opcode: Opcode::Ret,
            modifier: Modifier::F,
            a_mode: AddrMode::Direct,
            a: SP_CELL,
            b_mode: AddrMode::Direct,
            b: 0,
        };
        core.set(0, instr, w);
        core.set(
            SP_CELL,
            Instruction {
                b: 0,
                ..Instruction::DAT_ZERO
            },
            w,
        );
        let ops = resolve(&mut core, 0, instr, w);
        let r = exec_ret(&mut core, ops, w);
        assert!(matches!(r, StepResult::Jump(_)), "no panic, just a jump");
        assert_eq!(core.get(SP_CELL).b, 7999, "retreating below zero wraps");
    }
}
