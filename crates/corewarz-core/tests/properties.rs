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

/// Opcodes that don't reliably kill a process within one or two steps.
///
/// `DAT` always dies immediately, and `DIV`/`MOD` die on a zero divisor,
/// which is common under uniform random field values. Excluding just these
/// three from an otherwise uniform pool makes warriors survive long enough
/// to matter for boundary-crossing tests, without hand-picking "friendly"
/// behavior for every opcode.
const SAFE_OPCODES: [Opcode; 27] = [
    Opcode::Mov,
    Opcode::Add,
    Opcode::Sub,
    Opcode::Mul,
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
];

fn safe_instruction() -> impl Strategy<Value = Instruction> {
    (
        0..SAFE_OPCODES.len(),
        0..ALL_MODIFIERS.len(),
        0..ALL_MODES.len(),
        0u32..CORE_SIZE,
        0..ALL_MODES.len(),
        0u32..CORE_SIZE,
    )
        .prop_map(|(op, md, am, a, bm, b)| Instruction {
            opcode: SAFE_OPCODES[op],
            modifier: ALL_MODIFIERS[md],
            a_mode: ALL_MODES[am],
            a,
            b_mode: ALL_MODES[bm],
            b,
        })
}

/// A warrior built from `SAFE_OPCODES`, more likely to survive many cycles
/// than `any_warrior`'s fully-uniform pool.
fn safe_warrior() -> impl Strategy<Value = Warrior> {
    proptest::collection::vec(safe_instruction(), 1..20).prop_map(|code| Warrior {
        name: "safe".into(),
        code,
        start: 0,
    })
}

/// A warrior whose body is deliberately non-lethal and ends with an
/// unconditional `JMP` back to its own first instruction, so it loops for
/// as long as the battle runs instead of dying in a handful of cycles. This
/// gives the PC many chances to advance and, combined with a position near
/// the top of the core, to actually cross the `core_size` fold boundary
/// rather than merely being asserted near it.
fn looping_warrior() -> impl Strategy<Value = Warrior> {
    proptest::collection::vec(safe_instruction(), 1..19).prop_map(|mut body| {
        let len = body.len() as u32;
        let jmp = Instruction {
            opcode: Opcode::Jmp,
            modifier: Modifier::A,
            a_mode: AddrMode::Direct,
            // Folds to `pc - len`, i.e. back to the first body instruction,
            // however large `pc` has grown (see `Core::fold`).
            a: CORE_SIZE - len,
            b_mode: AddrMode::Direct,
            b: 0,
        };
        body.push(jmp);
        Warrior {
            name: "looping".into(),
            code: body,
            start: 0,
        }
    })
}

/// A start offset within `min_separation` of the very top of the core, so a
/// warrior placed at `core_size - offset` reaches the fold boundary (`pc ==
/// core_size - 1`) within a handful of `Continue`/`Skip` steps instead of
/// needing hundreds of cycles of incidental drift.
fn near_top_offset() -> impl Strategy<Value = u32> {
    1u32..20
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

    /// Places a warrior within a handful of instructions of the top of the
    /// core (`core_size - offset`) so its program counter is forced across
    /// the `core_size` fold boundary almost immediately, exercising both the
    /// `Continue` (`pc + 1`) and `Skip` (`pc + 2`) folds directly rather than
    /// relying on hundreds of cycles of incidental drift. This is what
    /// gives the core-bounds assertion below real power: mutating either
    /// fold to an unfolded raw add now escapes the core within a few steps.
    #[test]
    fn invariants_hold_near_core_boundary(
        top in safe_warrior(),
        offset in near_top_offset(),
        other in any_warrior(),
    ) {
        let config = config();
        let position = config.core_size - offset;
        let mut mars = Mars::try_new(config, vec![top, other], &[position, 400])
            .expect("safe warriors near the top are admissible in extended mode");

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

    /// A warrior that loops on itself via a trailing `JMP` survives for the
    /// full battle instead of dying in a few cycles, so across many random
    /// starting positions the PC eventually approaches and crosses the
    /// `core_size` boundary through ordinary `Continue` stepping, not just a
    /// contrived starting position.
    #[test]
    fn invariants_hold_for_looping_warriors(
        looper in looping_warrior(),
        other in safe_warrior(),
        position in 0u32..CORE_SIZE,
    ) {
        let config = config();
        let raw = (i64::from(position) - 400).rem_euclid(i64::from(config.core_size));
        let distance = raw.min(i64::from(config.core_size) - raw);
        prop_assume!(distance >= i64::from(config.min_separation));

        let mut mars = Mars::try_new(config, vec![looper, other], &[position, 400])
            .expect("looping warriors are admissible in extended mode");

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
