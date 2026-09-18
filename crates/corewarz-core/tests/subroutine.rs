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
    // 1: DAT  $0, $500    ; the stack-pointer cell: stack starts at 500
    // 2: JMP  $0, $0      ; the return lands here and spins
    // 3: RET  $-2, $0     ; return, sp cell is two cells back
    let code = vec![
        i(Opcode::Call, AddrMode::Direct, 3, AddrMode::Direct, 1),
        Instruction {
            b: 500,
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
    assert_eq!(mars.core().get(1).b, 501, "stack pointer advanced");

    mars.step(&mut NullObserver); // RET
    assert_eq!(
        mars.current_pc(WarriorId(0)),
        Some(1),
        "returned to just after the CALL"
    );
    assert_eq!(mars.core().get(1).b, 500, "stack pointer restored");
}

#[test]
fn bombing_the_stack_pointer_hijacks_the_return() {
    let code = vec![
        i(Opcode::Call, AddrMode::Direct, 3, AddrMode::Direct, 1),
        Instruction {
            b: 500,
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

    mars.step(&mut NullObserver); // CALL pushes to 500, sp becomes 501

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
