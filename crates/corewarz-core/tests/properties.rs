//! Invariants that must hold for every warrior, however nonsensical.

use corewarz_core::{
    AddrMode, Config, Dialect, Instruction, Mars, Modifier, NullObserver, Opcode, Warrior,
    WarriorId,
};
use proptest::prelude::*;

const ALL_OPCODES: [Opcode; 31] = [
    Opcode::Dat,
    Opcode::Mov,
    Opcode::Add,
    Opcode::Sub,
    Opcode::Mul,
    Opcode::Div,
    Opcode::Mod,
    Opcode::Jmp,
    Opcode::Jmz,
    Opcode::Jmn,
    Opcode::Djn,
    Opcode::Spl,
    Opcode::Seq,
    Opcode::Sne,
    Opcode::Slt,
    Opcode::Nop,
    Opcode::And,
    Opcode::Or,
    Opcode::Xor,
    Opcode::Shl,
    Opcode::Shr,
    Opcode::Not,
    Opcode::Tst,
    Opcode::Jez,
    Opcode::Jnz,
    Opcode::Jlz,
    Opcode::Jgz,
    Opcode::Call,
    Opcode::Ret,
    Opcode::Push,
    Opcode::Pop,
];

const ALL_MODIFIERS: [Modifier; 7] = [
    Modifier::A,
    Modifier::B,
    Modifier::AB,
    Modifier::BA,
    Modifier::F,
    Modifier::X,
    Modifier::I,
];

const ALL_MODES: [AddrMode; 8] = [
    AddrMode::Immediate,
    AddrMode::Direct,
    AddrMode::IndirectA,
    AddrMode::IndirectB,
    AddrMode::PredecA,
    AddrMode::PredecB,
    AddrMode::PostincA,
    AddrMode::PostincB,
];

const CORE_SIZE: u32 = 800;

fn any_instruction() -> impl Strategy<Value = Instruction> {
    (
        0..ALL_OPCODES.len(),
        0..ALL_MODIFIERS.len(),
        0..ALL_MODES.len(),
        0u32..CORE_SIZE,
        0..ALL_MODES.len(),
        0u32..CORE_SIZE,
    )
        .prop_map(|(op, md, am, a, bm, b)| Instruction {
            opcode: ALL_OPCODES[op],
            modifier: ALL_MODIFIERS[md],
            a_mode: ALL_MODES[am],
            a,
            b_mode: ALL_MODES[bm],
            b,
        })
}

fn any_warrior() -> impl Strategy<Value = Warrior> {
    proptest::collection::vec(any_instruction(), 1..20).prop_map(|code| Warrior {
        name: "random".into(),
        code,
        start: 0,
    })
}

fn config() -> Config {
    Config {
        core_size: CORE_SIZE,
        max_cycles: 2000,
        max_processes: 64,
        max_length: 20,
        min_separation: 20,
        dialect: Dialect::Extended,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// No sequence of instructions can drive a program counter, a process
    /// count, or a written address outside its legal range.
    #[test]
    fn invariants_hold_for_random_warriors(a in any_warrior(), b in any_warrior()) {
        let config = config();
        let mut mars = Mars::try_new(config, vec![a, b], &[0, 400])
            .expect("random warriors are admissible in extended mode");

        for _ in 0..config.max_cycles {
            if mars.outcome().is_some() {
                break;
            }
            mars.step(&mut NullObserver);

            for index in 0..2u8 {
                let id = WarriorId(index);
                prop_assert!(
                    mars.process_count(id) <= config.max_processes as usize,
                    "process count exceeded the limit"
                );
                if let Some(pc) = mars.process_pc(id) {
                    prop_assert!(pc < config.core_size, "program counter escaped the core");
                }
            }
        }
    }

    /// Every battle terminates. If this hangs, the run loop is wrong.
    #[test]
    fn every_battle_terminates(a in any_warrior(), b in any_warrior()) {
        let config = config();
        let mut mars = Mars::try_new(config, vec![a, b], &[0, 400]).unwrap();
        let _ = mars.run(&mut NullObserver);
        prop_assert!(mars.cycle() <= config.max_cycles);
    }

    /// Assembling a disassembly reproduces the instruction exactly, for any
    /// instruction the assembler would accept.
    #[test]
    fn disassembly_round_trips(instr in any_instruction()) {
        use corewarz_asm::{assemble, disassemble_instruction, legal_modifiers};

        let legal = legal_modifiers(instr.opcode);
        // Skip combinations the assembler legitimately rejects; Task 8 covers
        // the legal ones exhaustively, and this covers random field values.
        prop_assume!(legal.is_empty() || legal.contains(&instr.modifier));

        let text = disassemble_instruction(instr);
        let config = Config { dialect: Dialect::Extended, core_size: CORE_SIZE, ..Config::default() };
        let back = assemble(&text, &config).expect("canonical text must reassemble");
        let mut expected = instr;
        if legal.is_empty() {
            // Modifier-less opcodes disassemble without a modifier, so the
            // assembler supplies the default.
            expected.modifier = back.code[0].modifier;
        }
        prop_assert_eq!(back.code[0], expected);
    }
}

#[test]
fn a_warrior_of_every_single_opcode_runs_without_panicking() {
    // The narrowest possible smoke test for dispatch coverage: each opcode,
    // alone, executed once.
    for opcode in ALL_OPCODES {
        for modifier in ALL_MODIFIERS {
            let instr = Instruction {
                opcode,
                modifier,
                a_mode: AddrMode::Direct,
                a: 1,
                b_mode: AddrMode::Direct,
                b: 2,
            };
            let w = Warrior {
                name: "single".into(),
                code: vec![instr],
                start: 0,
            };
            let mut mars = Mars::try_new(config(), vec![w], &[0]).unwrap();
            mars.step(&mut NullObserver);
        }
    }
}
