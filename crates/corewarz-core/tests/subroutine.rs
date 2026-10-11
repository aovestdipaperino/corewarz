//! A warrior that actually calls a subroutine and returns from it.

use corewarz_core::{
    AddrMode, Config, Dialect, Instruction, Mars, Modifier, NullObserver, Opcode, Warrior,
    WarriorId,
};

fn i(opcode: Opcode, a_mode: AddrMode, a: u32, b_mode: AddrMode, b: u32) -> Instruction {
    Instruction {
        opcode,
        modifier: Modifier::F,
        a_mode,
        a,
        b_mode,
        b,
    }
}

#[test]
fn a_warrior_calls_a_subroutine_and_returns() {
    // 0: CALL $3, $1      ; call the routine at offset 3, sp cell at offset 1
    // 1: DAT  $0, $499    ; the stack-pointer cell: stack starts 499 cells on, at 500
    // 2: JMP  $0, $0      ; the return lands here and spins
    // 3: RET  $-2, $0     ; return, sp cell is two cells back
    let code = vec![
        i(Opcode::Call, AddrMode::Direct, 3, AddrMode::Direct, 1),
        Instruction {
            b: 499,
            ..Instruction::DAT_ZERO
        },
        i(Opcode::Jmp, AddrMode::Direct, 0, AddrMode::Direct, 0),
        i(Opcode::Ret, AddrMode::Direct, 7998, AddrMode::Direct, 0),
    ];
    let warrior = Warrior {
        name: "caller".into(),
        code,
        start: 0,
    };
    let config = Config {
        dialect: Dialect::Extended,
        ..Config::default()
    };
    let mut mars = Mars::new(config, vec![warrior], &[0]);

    mars.step(&mut NullObserver); // CALL
    assert_eq!(
        mars.current_pc(WarriorId(0)),
        Some(3),
        "jumped into the routine"
    );
    assert_eq!(mars.core().get(500).b, 1, "return address pushed");
    assert_eq!(mars.core().get(1).b, 500, "stack pointer advanced");

    mars.step(&mut NullObserver); // RET
    assert_eq!(
        mars.current_pc(WarriorId(0)),
        Some(1),
        "returned to just after the CALL"
    );
    assert_eq!(mars.core().get(1).b, 499, "stack pointer restored");
}

#[test]
fn bombing_the_stack_pointer_hijacks_the_return() {
    let code = vec![
        i(Opcode::Call, AddrMode::Direct, 3, AddrMode::Direct, 1),
        Instruction {
            b: 499,
            ..Instruction::DAT_ZERO
        },
        i(Opcode::Jmp, AddrMode::Direct, 0, AddrMode::Direct, 0),
        i(Opcode::Ret, AddrMode::Direct, 7998, AddrMode::Direct, 0),
    ];
    let warrior = Warrior {
        name: "caller".into(),
        code,
        start: 0,
    };
    let config = Config {
        dialect: Dialect::Extended,
        ..Config::default()
    };
    let mut mars = Mars::new(config, vec![warrior], &[0]);

    mars.step(&mut NullObserver); // CALL pushes to 500, sp field becomes 500

    // An enemy overwrites the pushed return address with a DAT cell's address.
    let mut hijacked = mars.core().get(500);
    hijacked.b = 6000;
    mars.core_mut().set(500, hijacked, WarriorId(0));

    mars.step(&mut NullObserver); // RET
    assert_eq!(
        mars.current_pc(WarriorId(0)),
        Some(6000),
        "the return went where the attacker chose"
    );
}

#[test]
fn a_warrior_with_an_out_of_range_stack_pointer_field_does_not_panic() {
    // Regression for FIX 3: `CALL $5, $1` with a stack-pointer cell whose
    // B-field holds 99_999 (from `DAT $0, $99999`, well past the 8000-cell
    // default core) is admitted by `Mars::try_new` -- admission checks
    // warrior length and dialect, not runtime cell contents -- and must
    // execute without panicking. It may do something useless; it must not
    // index out of bounds.
    let code = vec![
        i(Opcode::Call, AddrMode::Direct, 5, AddrMode::Direct, 1),
        Instruction {
            b: 99_999,
            ..Instruction::DAT_ZERO
        },
    ];
    let warrior = Warrior {
        name: "bad-sp".into(),
        code,
        start: 0,
    };
    let config = Config {
        dialect: Dialect::Extended,
        ..Config::default()
    };
    let mut mars = Mars::try_new(config, vec![warrior], &[0])
        .expect("warrior is admitted despite the bad field");

    mars.step(&mut NullObserver); // must not panic
}

#[test]
fn the_stack_pointer_is_relative_so_subroutines_work_anywhere() {
    // The stack-pointer field is read relative to its own cell, like `@`
    // indirection, so `sp DAT #0, #stack` finds the warrior's own stack
    // wherever the warrior is loaded, not an absolute cell near address 0.
    let source = "\
        CALL   subr, sp
        JMP    $0
subr    RET    sp
sp      DAT.F  #0, #stack
stack   DAT.F  #0, #0
";
    let config = Config {
        dialect: Dialect::Extended,
        ..Config::default()
    };
    let warrior = corewarz_asm::assemble(source, &config).unwrap();
    let base = 3000;
    let mut mars = Mars::new(config, vec![warrior], &[base]);

    mars.step(&mut NullObserver); // CALL
    assert_eq!(
        mars.core().get(base + 4).b,
        base + 1,
        "the return address lands on the warrior's own stack"
    );
    assert_eq!(mars.core().get(1).b, 0, "nothing is written near address 0");

    mars.step(&mut NullObserver); // RET
    assert_eq!(
        mars.current_pc(WarriorId(0)),
        Some(base + 1),
        "returned to just after the CALL"
    );
}
