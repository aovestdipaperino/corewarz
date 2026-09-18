use corewarz_core::mars::AdmissionError;
use corewarz_core::{AddrMode, Config, Dialect, Instruction, Mars, Modifier, Opcode, Warrior};

fn warrior(code: Vec<Instruction>) -> Warrior {
    Warrior {
        name: "w".into(),
        code,
        start: 0,
    }
}

fn and_instr() -> Instruction {
    Instruction {
        opcode: Opcode::And,
        modifier: Modifier::F,
        a_mode: AddrMode::Direct,
        a: 0,
        b_mode: AddrMode::Direct,
        b: 1,
    }
}

#[test]
fn an_extended_opcode_is_refused_admission_in_classic_mode() {
    let config = Config::default();
    let err = Mars::try_new(config, vec![warrior(vec![and_instr()])], &[0]).unwrap_err();
    assert!(matches!(
        err,
        AdmissionError::ExtendedOpcode {
            warrior: 0,
            offset: 0,
            opcode: Opcode::And
        }
    ));
}

#[test]
fn an_extended_opcode_is_admitted_in_extended_mode() {
    let config = Config {
        dialect: Dialect::Extended,
        ..Config::default()
    };
    assert!(Mars::try_new(config, vec![warrior(vec![and_instr()])], &[0]).is_ok());
}

#[test]
fn the_reported_offset_locates_the_offending_instruction() {
    let config = Config::default();
    let code = vec![Instruction::DAT_ZERO, Instruction::DAT_ZERO, and_instr()];
    let err = Mars::try_new(config, vec![warrior(code)], &[0]).unwrap_err();
    assert!(matches!(
        err,
        AdmissionError::ExtendedOpcode { offset: 2, .. }
    ));
}

#[test]
fn admission_rejects_overlong_warriors() {
    let config = Config {
        max_length: 2,
        ..Config::default()
    };
    let code = vec![Instruction::DAT_ZERO; 3];
    let err = Mars::try_new(config, vec![warrior(code)], &[0]).unwrap_err();
    assert!(matches!(
        err,
        AdmissionError::TooLong {
            length: 3,
            limit: 2,
            ..
        }
    ));
}

#[test]
fn admission_rejects_warriors_that_are_too_close_together() {
    let config = Config::default();
    let ws = vec![
        warrior(vec![Instruction::DAT_ZERO]),
        warrior(vec![Instruction::DAT_ZERO]),
    ];
    let err = Mars::try_new(config, ws, &[0, 50]).unwrap_err();
    assert!(matches!(err, AdmissionError::Separation(_)));
}
