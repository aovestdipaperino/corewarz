//! Modifier legality and ICWS'94 default modifiers.

use corewarz_core::{AddrMode, Modifier, Opcode};

use Modifier::{A, AB, B, BA, F, I, X};

/// The modifiers `opcode` accepts.
///
/// This is where the restricted-modifier rule for CoreWarz extensions lives:
/// `CALL`, `RET`, `PUSH`, and `POP` accept none, and the conditional branches
/// accept only the modifiers that select which field of the tested operand to
/// examine.
#[must_use]
pub const fn legal_modifiers(opcode: Opcode) -> &'static [Modifier] {
    match opcode {
        Opcode::Dat => &[F],
        Opcode::Mov | Opcode::Seq | Opcode::Sne => &[A, B, AB, BA, F, X, I],
        Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Mod | Opcode::Slt => {
            &[A, B, AB, BA, F, X]
        }
        Opcode::Jmp | Opcode::Spl | Opcode::Nop => &[A, B, AB, BA, F, X, I],
        Opcode::Jmz | Opcode::Jmn | Opcode::Djn => &[A, B, F, X, I],
        // --- extended ---
        Opcode::And
        | Opcode::Or
        | Opcode::Xor
        | Opcode::Shl
        | Opcode::Shr
        | Opcode::Not
        | Opcode::Tst => &[A, B, AB, BA, F, X],
        Opcode::Jez | Opcode::Jnz | Opcode::Jlz | Opcode::Jgz => &[A, B, F, X],
        Opcode::Call | Opcode::Ret | Opcode::Push | Opcode::Pop => &[],
    }
}

/// The ICWS'94 default modifier for `opcode` with the given addressing modes.
///
/// Opcodes that accept no modifier are given [`Modifier::F`] as an inert
/// placeholder; it is never consulted during execution.
#[must_use]
pub fn default_modifier(opcode: Opcode, a_mode: AddrMode, b_mode: AddrMode) -> Modifier {
    let a_immediate = a_mode == AddrMode::Immediate;
    let b_immediate = b_mode == AddrMode::Immediate;
    match opcode {
        Opcode::Dat => F,
        Opcode::Mov | Opcode::Seq | Opcode::Sne | Opcode::Tst => {
            if a_immediate {
                AB
            } else if b_immediate {
                B
            } else {
                I
            }
        }
        Opcode::Add
        | Opcode::Sub
        | Opcode::Mul
        | Opcode::Div
        | Opcode::Mod
        | Opcode::And
        | Opcode::Or
        | Opcode::Xor
        | Opcode::Shl
        | Opcode::Shr
        | Opcode::Not => {
            if a_immediate {
                AB
            } else if b_immediate {
                B
            } else {
                F
            }
        }
        Opcode::Slt => {
            if a_immediate {
                AB
            } else {
                B
            }
        }
        Opcode::Jmp
        | Opcode::Jmz
        | Opcode::Jmn
        | Opcode::Djn
        | Opcode::Spl
        | Opcode::Nop
        | Opcode::Jez
        | Opcode::Jnz
        | Opcode::Jlz
        | Opcode::Jgz => B,
        Opcode::Call | Opcode::Ret | Opcode::Push | Opcode::Pop => F,
    }
}

/// Parses a mnemonic, case-insensitively. `CMP` is accepted as an alias of
/// `SEQ`.
#[must_use]
pub fn parse_opcode(text: &str) -> Option<Opcode> {
    let op = match text.to_ascii_uppercase().as_str() {
        "DAT" => Opcode::Dat,
        "MOV" => Opcode::Mov,
        "ADD" => Opcode::Add,
        "SUB" => Opcode::Sub,
        "MUL" => Opcode::Mul,
        "DIV" => Opcode::Div,
        "MOD" => Opcode::Mod,
        "JMP" => Opcode::Jmp,
        "JMZ" => Opcode::Jmz,
        "JMN" => Opcode::Jmn,
        "DJN" => Opcode::Djn,
        "SPL" => Opcode::Spl,
        "SEQ" | "CMP" => Opcode::Seq,
        "SNE" => Opcode::Sne,
        "SLT" => Opcode::Slt,
        "NOP" => Opcode::Nop,
        "AND" => Opcode::And,
        "OR" => Opcode::Or,
        "XOR" => Opcode::Xor,
        "SHL" => Opcode::Shl,
        "SHR" => Opcode::Shr,
        "NOT" => Opcode::Not,
        "TST" => Opcode::Tst,
        "JEZ" => Opcode::Jez,
        "JNZ" => Opcode::Jnz,
        "JLZ" => Opcode::Jlz,
        "JGZ" => Opcode::Jgz,
        "CALL" => Opcode::Call,
        "RET" => Opcode::Ret,
        "PUSH" => Opcode::Push,
        "POP" => Opcode::Pop,
        _ => return None,
    };
    Some(op)
}

/// Parses a modifier suffix, case-insensitively.
#[must_use]
pub fn parse_modifier(text: &str) -> Option<Modifier> {
    let m = match text.to_ascii_uppercase().as_str() {
        "A" => A,
        "B" => B,
        "AB" => AB,
        "BA" => BA,
        "F" => F,
        "X" => X,
        "I" => I,
        _ => return None,
    };
    Some(m)
}

/// Parses an addressing-mode sigil. Returns [`AddrMode::Direct`] for `None`.
#[must_use]
pub const fn parse_mode(sigil: Option<char>) -> Option<AddrMode> {
    let mode = match sigil {
        None | Some('$') => AddrMode::Direct,
        Some('#') => AddrMode::Immediate,
        Some('*') => AddrMode::IndirectA,
        Some('@') => AddrMode::IndirectB,
        Some('{') => AddrMode::PredecA,
        Some('<') => AddrMode::PredecB,
        Some('}') => AddrMode::PostincA,
        Some('>') => AddrMode::PostincB,
        Some(_) => return None,
    };
    Some(mode)
}
