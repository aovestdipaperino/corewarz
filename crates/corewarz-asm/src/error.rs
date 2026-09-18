//! Assembly errors.
//!
//! The assembler is the only CoreWarz component that reports errors to a
//! human. The VM has no runtime errors: a loaded warrior is well-formed by
//! construction.

use corewarz_core::{Modifier, Opcode};

/// What went wrong while assembling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsmErrorKind {
    /// The mnemonic is not a known opcode.
    UnknownOpcode(String),
    /// This opcode does not accept this modifier.
    IllegalModifier { opcode: Opcode, modifier: Modifier },
    /// A CoreWarz extension was used under the classic dialect.
    ExtendedOpcodeInClassic(Opcode),
    /// An operand referred to a label that was never defined.
    UnresolvedLabel(String),
    /// The same label was defined twice.
    DuplicateLabel(String),
    /// The warrior exceeds `Config::max_length`.
    TooLong { length: u32, limit: u32 },
    /// An operand could not be parsed.
    MalformedOperand(String),
    /// A numeric literal could not be parsed.
    BadNumber(String),
    /// The instruction needs at least one operand and none was given.
    MissingOperand,
    /// A label was named after a mnemonic (opcode or pseudo-op), which
    /// pMARS reserves as a keyword and never allows as an identifier.
    ReservedWord(String),
}

impl std::fmt::Display for AsmErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownOpcode(s) => write!(f, "unknown opcode `{s}`"),
            Self::IllegalModifier { opcode, modifier } => {
                write!(f, "{opcode:?} does not accept the modifier {modifier:?}")
            }
            Self::ExtendedOpcodeInClassic(op) => {
                write!(
                    f,
                    "{op:?} is a CoreWarz extension and is illegal in classic mode"
                )
            }
            Self::UnresolvedLabel(s) => write!(f, "undefined label `{s}`"),
            Self::DuplicateLabel(s) => write!(f, "label `{s}` is defined more than once"),
            Self::TooLong { length, limit } => {
                write!(
                    f,
                    "warrior is {length} instructions long, the limit is {limit}"
                )
            }
            Self::MalformedOperand(s) => write!(f, "malformed operand `{s}`"),
            Self::BadNumber(s) => write!(f, "`{s}` is not a number"),
            Self::MissingOperand => write!(f, "missing operand"),
            Self::ReservedWord(s) => {
                write!(f, "`{s}` is a mnemonic and cannot be used as a label")
            }
        }
    }
}

/// An assembly error, with the source line that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsmError {
    /// One-based source line number.
    pub line: usize,
    /// What went wrong.
    pub kind: AsmErrorKind,
}

impl std::fmt::Display for AsmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.kind)
    }
}

impl std::error::Error for AsmError {}
