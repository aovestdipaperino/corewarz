//! The CoreWarz MARS virtual machine.
//!
//! This crate owns the instruction representation, the core memory, the
//! process scheduler, and the execution step function. It performs no I/O and
//! knows nothing about text, files, or terminals.
//!
//! # Orientation
//!
//! - [`Instruction`] and [`Opcode`]/[`Modifier`]/[`AddrMode`] are the fixed,
//!   `Copy` instruction representation shared by classic ICWS'94 Redcode and
//!   the CoreWarz extensions ([`exec`] module docs and
//!   `docs/extended-redcode.md` at the workspace root describe the
//!   extensions).
//! - [`Core`] is the circular memory every warrior fights over; all modular
//!   address arithmetic goes through its `fold` method, the one place
//!   wrapping happens.
//! - [`Mars`] drives the scheduler: it owns each warrior's process queue and
//!   advances the whole battle one cycle at a time via [`Mars::step`].
//! - [`Observer`] is the sole interface the VM exposes to anything watching a
//!   battle (a visualizer, a differential-test harness); [`NullObserver`] is
//!   the zero-cost default a headless run uses.
//! - [`Config`] and [`Dialect`] select core size, cycle/process limits, and
//!   whether the extended opcodes are legal.
//!
//! This crate assembles nothing and prints nothing — see `corewarz-asm` for
//! the assembler/disassembler and `corewarz-cli`/`corewarz-tui` for anything
//! that touches a terminal.

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
pub use mars::{AdmissionError, Mars, Outcome, SeparationError};
pub use observer::{NullObserver, Observer};
pub use warrior::{Warrior, WarriorId};
