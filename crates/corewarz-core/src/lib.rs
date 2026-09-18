//! The CoreWarz MARS virtual machine.
//!
//! This crate owns the instruction representation, the core memory, the
//! process scheduler, and the execution step function. It performs no I/O and
//! knows nothing about text, files, or terminals.

pub mod config;
pub mod core;
pub mod exec;
pub mod instruction;
pub mod mars;
pub mod observer;
pub mod queue;
pub mod warrior;

pub use config::{Config, Dialect};
pub use core::Core;
pub use instruction::{AddrMode, Instruction, Modifier, Opcode};
pub use mars::{Mars, Outcome, SeparationError};
pub use observer::{NullObserver, Observer};
pub use warrior::{Warrior, WarriorId};
