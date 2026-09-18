//! Instruction representation.
//!
//! An [`Instruction`] is a small `Copy` struct so that the core can be a flat,
//! cache-friendly `Vec`. Field values are always folded into `0..core_size`.

/// A Redcode operation.
///
/// Variants up to and including [`Opcode::Nop`] are ICWS'94 classic opcodes.
/// The remainder are CoreWarz extensions, available only in the extended
/// dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Opcode {
    // --- classic ---
    Dat,
    Mov,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Jmp,
    Jmz,
    Jmn,
    Djn,
    Spl,
    Seq,
    Sne,
    Slt,
    Nop,
    // --- extended ---
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Not,
    Tst,
    Jez,
    Jnz,
    Jlz,
    Jgz,
    Call,
    Ret,
    Push,
    Pop,
}

impl Opcode {
    /// Returns `true` if this opcode is a CoreWarz extension and therefore
    /// illegal in the classic dialect.
    #[must_use]
    pub const fn is_extended(self) -> bool {
        !matches!(
            self,
            Opcode::Dat
                | Opcode::Mov
                | Opcode::Add
                | Opcode::Sub
                | Opcode::Mul
                | Opcode::Div
                | Opcode::Mod
                | Opcode::Jmp
                | Opcode::Jmz
                | Opcode::Jmn
                | Opcode::Djn
                | Opcode::Spl
                | Opcode::Seq
                | Opcode::Sne
                | Opcode::Slt
                | Opcode::Nop
        )
    }
}

/// An instruction modifier, selecting which fields an operation acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// A-field of source to A-field of destination.
    A,
    /// B-field of source to B-field of destination.
    B,
    /// A-field of source to B-field of destination.
    AB,
    /// B-field of source to A-field of destination.
    BA,
    /// Both fields, straight through.
    F,
    /// Both fields, crossed.
    X,
    /// The whole instruction.
    I,
}

/// An addressing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddrMode {
    /// `#` — the field value is the operand itself.
    Immediate,
    /// `$` — the field value is an offset from the current instruction.
    Direct,
    /// `*` — indirect through the A-field of the pointed-at instruction.
    IndirectA,
    /// `@` — indirect through the B-field of the pointed-at instruction.
    IndirectB,
    /// `{` — decrement the pointed-at A-field, then indirect through it.
    PredecA,
    /// `<` — decrement the pointed-at B-field, then indirect through it.
    PredecB,
    /// `}` — indirect through the pointed-at A-field, then increment it.
    PostincA,
    /// `>` — indirect through the pointed-at B-field, then increment it.
    PostincB,
}

/// A single core cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub modifier: Modifier,
    pub a_mode: AddrMode,
    pub a: u32,
    pub b_mode: AddrMode,
    pub b: u32,
}

impl Instruction {
    /// The instruction every core cell holds before warriors are loaded:
    /// `DAT.F $0, $0`.
    pub const DAT_ZERO: Instruction = Instruction {
        opcode: Opcode::Dat,
        modifier: Modifier::F,
        a_mode: AddrMode::Direct,
        a: 0,
        b_mode: AddrMode::Direct,
        b: 0,
    };
}

impl Default for Instruction {
    fn default() -> Self {
        Self::DAT_ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_opcodes_are_not_extended() {
        for op in [
            Opcode::Dat,
            Opcode::Mov,
            Opcode::Add,
            Opcode::Jmp,
            Opcode::Spl,
            Opcode::Nop,
        ] {
            assert!(!op.is_extended(), "{op:?} should be classic");
        }
    }

    #[test]
    fn extended_opcodes_are_extended() {
        for op in [
            Opcode::And,
            Opcode::Tst,
            Opcode::Jez,
            Opcode::Call,
            Opcode::Ret,
        ] {
            assert!(op.is_extended(), "{op:?} should be extended");
        }
    }

    #[test]
    fn dat_zero_is_the_initial_core_cell() {
        let d = Instruction::DAT_ZERO;
        assert_eq!(d.opcode, Opcode::Dat);
        assert_eq!(d.modifier, Modifier::F);
        assert_eq!(d.a_mode, AddrMode::Direct);
        assert_eq!(d.b_mode, AddrMode::Direct);
        assert_eq!(d.a, 0);
        assert_eq!(d.b, 0);
    }
}
