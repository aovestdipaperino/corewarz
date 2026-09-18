use corewarz_asm::{assemble, AsmErrorKind};
use corewarz_core::{AddrMode, Config, Dialect, Instruction, Modifier, Opcode};

fn cfg() -> Config {
    Config::default()
}

fn ext() -> Config {
    Config {
        dialect: Dialect::Extended,
        ..Config::default()
    }
}

#[test]
fn assembles_the_imp() {
    let w = assemble("MOV.I $0, $1", &cfg()).expect("imp must assemble");
    assert_eq!(w.code.len(), 1);
    assert_eq!(
        w.code[0],
        Instruction {
            opcode: Opcode::Mov,
            modifier: Modifier::I,
            a_mode: AddrMode::Direct,
            a: 0,
            b_mode: AddrMode::Direct,
            b: 1,
        }
    );
    assert_eq!(w.start, 0);
}

#[test]
fn strips_comments_and_blank_lines() {
    let w = assemble("; a comment\n\nMOV.I $0, $1 ; trailing\n", &cfg()).unwrap();
    assert_eq!(w.code.len(), 1);
}

#[test]
fn is_case_insensitive() {
    let a = assemble("mov.i $0, $1", &cfg()).unwrap();
    let b = assemble("MOV.I $0, $1", &cfg()).unwrap();
    assert_eq!(a.code, b.code);
}

#[test]
fn resolves_labels_as_relative_offsets() {
    let src = "start  MOV.I $0, $1\n       JMP  $start, $0\n";
    let w = assemble(src, &cfg()).unwrap();
    assert_eq!(w.code[1].a, 7999, "JMP back one cell is -1, folded");
}

#[test]
fn resolves_forward_labels() {
    let src = "       JMP  $end, $0\n       DAT  $0, $0\nend    MOV.I $0, $1\n";
    let w = assemble(src, &cfg()).unwrap();
    assert_eq!(w.code[0].a, 2);
}

#[test]
fn label_arithmetic_works() {
    let src = "start  MOV.I $0, $1\n       JMP  $start+1, $0\n";
    let w = assemble(src, &cfg()).unwrap();
    assert_eq!(w.code[1].a, 0, "start+1 from line 1 is offset 0");
}

#[test]
fn equ_substitutes_a_constant() {
    let src = "step EQU 4\nMOV.I $0, $step\n";
    let w = assemble(src, &cfg()).unwrap();
    assert_eq!(w.code[0].b, 4);
}

#[test]
fn org_sets_the_start_offset() {
    let src = "ORG go\n     DAT $0, $0\ngo   MOV.I $0, $1\n";
    let w = assemble(src, &cfg()).unwrap();
    assert_eq!(w.start, 1);
}

#[test]
fn end_with_a_label_sets_the_start_offset() {
    let src = "     DAT $0, $0\ngo   MOV.I $0, $1\n     END go\n";
    let w = assemble(src, &cfg()).unwrap();
    assert_eq!(w.start, 1);
    assert_eq!(w.code.len(), 2, "END is not an instruction");
}

#[test]
fn all_addressing_mode_sigils_parse() {
    let src = "MOV.I #0, $1\nMOV.I *0, @1\nMOV.I {0, <1\nMOV.I }0, >1\n";
    let w = assemble(src, &cfg()).unwrap();
    assert_eq!(
        (w.code[0].a_mode, w.code[0].b_mode),
        (AddrMode::Immediate, AddrMode::Direct)
    );
    assert_eq!(
        (w.code[1].a_mode, w.code[1].b_mode),
        (AddrMode::IndirectA, AddrMode::IndirectB)
    );
    assert_eq!(
        (w.code[2].a_mode, w.code[2].b_mode),
        (AddrMode::PredecA, AddrMode::PredecB)
    );
    assert_eq!(
        (w.code[3].a_mode, w.code[3].b_mode),
        (AddrMode::PostincA, AddrMode::PostincB)
    );
}

#[test]
fn a_missing_sigil_means_direct() {
    let w = assemble("MOV.I 0, 1", &cfg()).unwrap();
    assert_eq!(w.code[0].a_mode, AddrMode::Direct);
}

#[test]
fn negative_field_values_fold() {
    let w = assemble("DAT $-1, $0", &cfg()).unwrap();
    assert_eq!(w.code[0].a, 7999);
}

#[test]
fn a_missing_second_operand_defaults_to_direct_zero() {
    let w = assemble("JMP $5", &cfg()).unwrap();
    assert_eq!((w.code[0].b_mode, w.code[0].b), (AddrMode::Direct, 0));
}

#[test]
fn default_modifiers_follow_icws94() {
    // MOV with a non-immediate A-operand defaults to .I
    assert_eq!(
        assemble("MOV $0, $1", &cfg()).unwrap().code[0].modifier,
        Modifier::I
    );
    // MOV with an immediate A-operand defaults to .AB
    assert_eq!(
        assemble("MOV #0, $1", &cfg()).unwrap().code[0].modifier,
        Modifier::AB
    );
    // ADD with a non-immediate A-operand defaults to .F
    assert_eq!(
        assemble("ADD $0, $1", &cfg()).unwrap().code[0].modifier,
        Modifier::F
    );
    // ADD with an immediate A-operand defaults to .AB
    assert_eq!(
        assemble("ADD #1, $1", &cfg()).unwrap().code[0].modifier,
        Modifier::AB
    );
    // DAT always defaults to .F
    assert_eq!(
        assemble("DAT #0, #0", &cfg()).unwrap().code[0].modifier,
        Modifier::F
    );
    // JMP and friends always default to .B
    assert_eq!(
        assemble("JMP $0, $0", &cfg()).unwrap().code[0].modifier,
        Modifier::B
    );
    // SEQ with a non-immediate A-operand defaults to .I
    assert_eq!(
        assemble("SEQ $0, $1", &cfg()).unwrap().code[0].modifier,
        Modifier::I
    );
}

#[test]
fn unknown_opcodes_are_rejected_with_a_line_number() {
    let err = assemble("MOV.I $0, $1\nFROB $0, $0\n", &cfg()).unwrap_err();
    assert_eq!(err.line, 2);
    assert!(matches!(err.kind, AsmErrorKind::UnknownOpcode(_)));
}

#[test]
fn extended_opcodes_are_rejected_in_classic_mode() {
    let err = assemble("AND.F $0, $1", &cfg()).unwrap_err();
    assert!(matches!(
        err.kind,
        AsmErrorKind::ExtendedOpcodeInClassic(Opcode::And)
    ));
    assert!(assemble("AND.F $0, $1", &ext()).is_ok());
}

#[test]
fn illegal_modifiers_are_rejected() {
    // CALL takes no modifier.
    let err = assemble("CALL.F $0, $1", &ext()).unwrap_err();
    assert!(matches!(err.kind, AsmErrorKind::IllegalModifier { .. }));
    // Branches do not accept .AB or .BA.
    let err = assemble("JEZ.AB $0, $1", &ext()).unwrap_err();
    assert!(matches!(err.kind, AsmErrorKind::IllegalModifier { .. }));
    assert!(assemble("JEZ.B $0, $1", &ext()).is_ok());
}

#[test]
fn unresolved_labels_are_rejected() {
    let err = assemble("JMP $nowhere, $0", &cfg()).unwrap_err();
    assert!(matches!(err.kind, AsmErrorKind::UnresolvedLabel(_)));
}

#[test]
fn duplicate_labels_are_rejected() {
    let err = assemble("a DAT $0, $0\na DAT $0, $0\n", &cfg()).unwrap_err();
    assert!(matches!(err.kind, AsmErrorKind::DuplicateLabel(_)));
}

#[test]
fn warriors_longer_than_the_limit_are_rejected() {
    let cfg = Config {
        max_length: 3,
        ..Config::default()
    };
    let src = "DAT $0, $0\n".repeat(4);
    let err = assemble(&src, &cfg).unwrap_err();
    assert!(matches!(err.kind, AsmErrorKind::TooLong { .. }));
}
