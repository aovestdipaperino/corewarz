//! Two-pass assembly: collect labels and `EQU`s, then emit instructions.

use std::collections::HashMap;

use corewarz_core::{AddrMode, Config, Dialect, Instruction, Opcode, Warrior};

use crate::error::{AsmError, AsmErrorKind};
use crate::lexer::{lex, Line};
use crate::modifier::{default_modifier, legal_modifiers, parse_modifier, parse_opcode};
use crate::parser::{parse_operand, RawOperand};

fn is_pseudo(text: &str) -> bool {
    matches!(text.to_ascii_uppercase().as_str(), "EQU" | "ORG" | "END")
}

/// Assembles Redcode `source` into a loadable warrior.
///
/// # Errors
/// Returns the first [`AsmError`], carrying the offending source line.
pub fn assemble(source: &str, config: &Config) -> Result<Warrior, AsmError> {
    let is_mnemonic =
        |t: &str| parse_opcode(t.split('.').next().unwrap_or(t)).is_some() || is_pseudo(t);
    let lines = lex(source, &is_mnemonic);

    // --- pass one: labels, EQUs, ORG ---
    let mut labels: HashMap<String, i64> = HashMap::new();
    let mut equs: HashMap<String, String> = HashMap::new();
    let mut org: Option<(String, usize)> = None;
    let mut offset: i64 = 0;
    for line in &lines {
        let mnemonic = line.mnemonic.as_deref().unwrap_or("").to_ascii_uppercase();
        match mnemonic.as_str() {
            "EQU" => {
                let name = line.label.clone().ok_or(AsmError {
                    line: line.number,
                    kind: AsmErrorKind::MissingOperand,
                })?;
                let value = line.operands.first().cloned().ok_or(AsmError {
                    line: line.number,
                    kind: AsmErrorKind::MissingOperand,
                })?;
                check_not_reserved(&name, &is_mnemonic, line.number)?;
                equs.insert(name.to_ascii_lowercase(), value);
            }
            "ORG" | "END" => {
                if let Some(target) = line.operands.first() {
                    org = Some((target.clone(), line.number));
                }
            }
            "" => {
                if let Some(label) = &line.label {
                    check_not_reserved(label, &is_mnemonic, line.number)?;
                    insert_label(&mut labels, label, offset, line.number)?;
                }
            }
            _ => {
                if let Some(label) = &line.label {
                    check_not_reserved(label, &is_mnemonic, line.number)?;
                    insert_label(&mut labels, label, offset, line.number)?;
                }
                offset += 1;
            }
        }
    }

    let length = u32::try_from(offset).unwrap_or(u32::MAX);
    if length > config.max_length {
        let line = lines.last().map_or(1, |l| l.number);
        return Err(AsmError {
            line,
            kind: AsmErrorKind::TooLong {
                length,
                limit: config.max_length,
            },
        });
    }

    // --- pass two: emit ---
    let capacity = usize::try_from(offset).unwrap_or(0);
    let mut code = Vec::with_capacity(capacity);
    let mut address: i64 = 0;
    for line in &lines {
        let Some(raw_mnemonic) = &line.mnemonic else {
            continue;
        };
        if is_pseudo(raw_mnemonic) {
            continue;
        }
        let instr = emit(line, raw_mnemonic, address, &labels, &equs, config)?;
        code.push(instr);
        address += 1;
    }

    let start = match org {
        Some((target, org_line)) => {
            let resolved = resolve_expr(&target, 0, &labels, &equs).map_err(|kind| AsmError {
                line: org_line,
                kind,
            })?;
            u32::try_from(resolved.rem_euclid(i64::from(config.core_size))).unwrap_or(0)
        }
        None => 0,
    };

    Ok(Warrior {
        name: String::from("warrior"),
        code,
        start,
    })
}

/// Rejects a label/EQU name that parses as a mnemonic (opcode or
/// pseudo-op), matching pMARS treating mnemonics as reserved words.
fn check_not_reserved(
    name: &str,
    is_mnemonic: &dyn Fn(&str) -> bool,
    line: usize,
) -> Result<(), AsmError> {
    if is_mnemonic(name) {
        return Err(AsmError {
            line,
            kind: AsmErrorKind::ReservedWord(name.to_string()),
        });
    }
    Ok(())
}

fn insert_label(
    labels: &mut HashMap<String, i64>,
    label: &str,
    offset: i64,
    line: usize,
) -> Result<(), AsmError> {
    let key = label.to_ascii_lowercase();
    if labels.insert(key, offset).is_some() {
        return Err(AsmError {
            line,
            kind: AsmErrorKind::DuplicateLabel(label.to_string()),
        });
    }
    Ok(())
}

/// Resolves an operand expression to a value relative to `address`.
fn resolve_expr(
    text: &str,
    address: i64,
    labels: &HashMap<String, i64>,
    equs: &HashMap<String, String>,
) -> Result<i64, AsmErrorKind> {
    let operand = parse_operand(text)?;
    resolve_operand(&operand, address, labels, equs).map(|(_, v)| v)
}

fn resolve_operand(
    operand: &RawOperand,
    address: i64,
    labels: &HashMap<String, i64>,
    equs: &HashMap<String, String>,
) -> Result<(AddrMode, i64), AsmErrorKind> {
    let Some(label) = &operand.label else {
        return Ok((operand.mode, operand.offset));
    };
    let key = label.to_ascii_lowercase();
    if let Some(target) = labels.get(&key) {
        return Ok((operand.mode, target - address + operand.offset));
    }
    if let Some(text) = equs.get(&key) {
        let inner = parse_operand(text)?;
        let (_, value) = resolve_operand(&inner, address, labels, equs)?;
        return Ok((operand.mode, value + operand.offset));
    }
    Err(AsmErrorKind::UnresolvedLabel(label.clone()))
}

fn emit(
    line: &Line,
    raw_mnemonic: &str,
    address: i64,
    labels: &HashMap<String, i64>,
    equs: &HashMap<String, String>,
    config: &Config,
) -> Result<Instruction, AsmError> {
    let at = |kind: AsmErrorKind| AsmError {
        line: line.number,
        kind,
    };

    let opcode = parse_opcode(raw_mnemonic)
        .ok_or_else(|| at(AsmErrorKind::UnknownOpcode(raw_mnemonic.to_string())))?;

    if config.dialect == Dialect::Classic && opcode.is_extended() {
        return Err(at(AsmErrorKind::ExtendedOpcodeInClassic(opcode)));
    }

    let explicit_modifier = match &line.modifier {
        Some(text) => Some(
            parse_modifier(text).ok_or_else(|| at(AsmErrorKind::MalformedOperand(text.clone())))?,
        ),
        None => None,
    };

    let mut operands = Vec::new();
    for text in &line.operands {
        operands.push(parse_operand(text).map_err(at)?);
    }
    if operands.is_empty() {
        if opcode == Opcode::Dat || opcode == Opcode::Nop || opcode == Opcode::Ret {
            operands.push(RawOperand::direct_zero());
        } else {
            return Err(at(AsmErrorKind::MissingOperand));
        }
    }
    while operands.len() < 2 {
        operands.push(RawOperand::direct_zero());
    }

    let (a_mode, a_value) = resolve_operand(&operands[0], address, labels, equs).map_err(at)?;
    let (b_mode, b_value) = resolve_operand(&operands[1], address, labels, equs).map_err(at)?;

    let modifier = match explicit_modifier {
        Some(m) => {
            if !legal_modifiers(opcode).contains(&m) {
                return Err(at(AsmErrorKind::IllegalModifier {
                    opcode,
                    modifier: m,
                }));
            }
            m
        }
        None => default_modifier(opcode, a_mode, b_mode),
    };

    let size = i64::from(config.core_size);
    Ok(Instruction {
        opcode,
        modifier,
        a_mode,
        a: u32::try_from(a_value.rem_euclid(size)).unwrap_or(0),
        b_mode,
        b: u32::try_from(b_value.rem_euclid(size)).unwrap_or(0),
    })
}

#[cfg(test)]
mod reserved_word_tests {
    use super::check_not_reserved;
    use crate::error::AsmErrorKind;

    fn is_mnemonic(t: &str) -> bool {
        crate::modifier::parse_opcode(t.split('.').next().unwrap_or(t)).is_some()
            || matches!(t.to_ascii_uppercase().as_str(), "EQU" | "ORG" | "END")
    }

    // The public lexer/assembler surface can no longer construct a `Line`
    // whose label equals a mnemonic (see `lexer::lex`'s doc comment): any
    // leading token that is a known mnemonic is unconditionally lexed as
    // the instruction, never a label. `check_not_reserved` is therefore a
    // structural invariant guard exercised directly here rather than
    // through `assemble()`, proving the `ReservedWord` variant and its
    // message are correct should that invariant ever be violated (e.g. by
    // a future lexer change).
    #[test]
    fn rejects_names_that_parse_as_a_mnemonic_case_insensitively() {
        for name in ["mov", "MOV", "End", "org", "dat", "Jmp"] {
            let err = check_not_reserved(name, &is_mnemonic, 3).unwrap_err();
            assert_eq!(err.line, 3);
            assert!(matches!(err.kind, AsmErrorKind::ReservedWord(ref s) if s == name));
        }
    }

    #[test]
    fn accepts_names_that_are_not_mnemonics() {
        assert!(check_not_reserved("start", &is_mnemonic, 1).is_ok());
        assert!(check_not_reserved("dat1", &is_mnemonic, 1).is_ok());
    }

    #[test]
    fn reserved_word_error_message_names_the_word() {
        let msg = AsmErrorKind::ReservedWord("mov".to_string()).to_string();
        assert!(msg.contains("mov"));
    }
}
