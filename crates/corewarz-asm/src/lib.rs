//! Redcode assembler and disassembler for CoreWarz.
//!
//! The assembler is the only component that reports errors to a human, and the
//! only place where the classic/extended dialect distinction is enforced at
//! the source level.

pub mod assemble;
pub mod disassemble;
pub mod error;
pub mod lexer;
pub mod modifier;
pub mod parser;

pub use assemble::assemble;
pub use disassemble::{disassemble, disassemble_instruction, mode_sigil};
pub use error::{AsmError, AsmErrorKind};
pub use modifier::{default_modifier, legal_modifiers};
