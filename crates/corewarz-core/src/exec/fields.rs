//! Modifier-driven field selection.
//!
//! [`field_pairs`] is the single source of truth for what every binary
//! operation does under each modifier. Arithmetic, bitwise, and comparison
//! opcodes all plug into it, which is why they share one set of semantics.

use crate::instruction::{Instruction, Modifier};

/// One of an instruction's two value fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    A,
    B,
}

impl Field {
    /// Reads this field from `instr`.
    #[must_use]
    pub const fn get(self, instr: &Instruction) -> u32 {
        match self {
            Field::A => instr.a,
            Field::B => instr.b,
        }
    }

    /// Writes `value` into this field of `instr`.
    pub fn set(self, instr: &mut Instruction, value: u32) {
        match self {
            Field::A => instr.a = value,
            Field::B => instr.b = value,
        }
    }
}

/// The (source, destination) field pairs a binary operation acts on under
/// `modifier`.
///
/// `.I` is treated as `.F` here; opcodes for which `.I` means "the whole
/// instruction" (only `MOV`, `SEQ`, and `SNE`) handle it before consulting
/// this table.
#[must_use]
pub const fn field_pairs(modifier: Modifier) -> &'static [(Field, Field)] {
    match modifier {
        Modifier::A => &[(Field::A, Field::A)],
        Modifier::B => &[(Field::B, Field::B)],
        Modifier::AB => &[(Field::A, Field::B)],
        Modifier::BA => &[(Field::B, Field::A)],
        Modifier::F | Modifier::I => &[(Field::A, Field::A), (Field::B, Field::B)],
        Modifier::X => &[(Field::A, Field::B), (Field::B, Field::A)],
    }
}
