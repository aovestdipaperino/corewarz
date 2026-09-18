//! The CoreWarz MARS virtual machine.
//!
//! This crate owns the instruction representation, the core memory, the
//! process scheduler, and the execution step function. It performs no I/O and
//! knows nothing about text, files, or terminals.

pub mod instruction;

pub use instruction::{AddrMode, Instruction, Modifier, Opcode};
