use corewarz_asm::{assemble, disassemble, disassemble_instruction};
use corewarz_core::{AddrMode, Config, Dialect, Instruction, Modifier, Opcode};

fn ext() -> Config {
    Config {
        dialect: Dialect::Extended,
        ..Config::default()
    }
}

#[test]
fn disassembles_in_canonical_form() {
    let i = Instruction {
        opcode: Opcode::Mov,
        modifier: Modifier::I,
        a_mode: AddrMode::Direct,
        a: 0,
        b_mode: AddrMode::Direct,
        b: 1,
    };
    assert_eq!(disassemble_instruction(i), "MOV.I $0, $1");
}

#[test]
fn every_addressing_mode_prints_its_sigil() {
    let modes = [
        (AddrMode::Immediate, '#'),
        (AddrMode::Direct, '$'),
        (AddrMode::IndirectA, '*'),
        (AddrMode::IndirectB, '@'),
        (AddrMode::PredecA, '{'),
        (AddrMode::PredecB, '<'),
        (AddrMode::PostincA, '}'),
        (AddrMode::PostincB, '>'),
    ];
    for (mode, sigil) in modes {
        let i = Instruction {
            a_mode: mode,
            ..Instruction::DAT_ZERO
        };
        let text = disassemble_instruction(i);
        assert!(text.contains(sigil), "{text} should contain {sigil}");
    }
}

/// Assembling a disassembly must reproduce the original instruction exactly,
/// for every opcode, every modifier that opcode accepts, and every addressing
/// mode.
#[test]
fn round_trip_is_exact_for_every_legal_combination() {
    use corewarz_asm::legal_modifiers;

    let opcodes = [
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
    let modes = [
        AddrMode::Immediate,
        AddrMode::Direct,
        AddrMode::IndirectA,
        AddrMode::IndirectB,
        AddrMode::PredecA,
        AddrMode::PredecB,
        AddrMode::PostincA,
        AddrMode::PostincB,
    ];

    for opcode in opcodes {
        let mods: Vec<Modifier> = if legal_modifiers(opcode).is_empty() {
            vec![Modifier::F]
        } else {
            legal_modifiers(opcode).to_vec()
        };
        for modifier in mods {
            for a_mode in modes {
                for b_mode in modes {
                    let original = Instruction {
                        opcode,
                        modifier,
                        a_mode,
                        a: 17,
                        b_mode,
                        b: 7993,
                    };
                    let text = disassemble_instruction(original);
                    let back = assemble(&text, &ext())
                        .unwrap_or_else(|e| panic!("`{text}` failed to reassemble: {e}"));
                    assert_eq!(back.code[0], original, "round trip changed `{text}`");
                }
            }
        }
    }
}

#[test]
fn a_whole_warrior_round_trips() {
    let src = "start MOV.I $0, $1\n      SPL.B $start, $0\n      JMP.B $-1, $0\n";
    let w = assemble(src, &ext()).unwrap();
    let text = disassemble(&w);
    let again = assemble(&text, &ext()).unwrap();
    assert_eq!(again.code, w.code);
}

#[test]
fn numeric_org_round_trips() {
    let src = "ORG 1\nDAT.F $0, $0\nMOV.I $0, $1\n";
    let w = assemble(src, &ext()).unwrap();
    assert_eq!(w.start, 1);
    let again = assemble(&disassemble(&w), &ext()).unwrap();
    assert_eq!(again.start, 1);
}
