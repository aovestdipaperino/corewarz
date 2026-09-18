//! Loadable warrior programs.

use crate::instruction::Instruction;

/// Identifies a warrior within a battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WarriorId(pub u8);

/// An assembled, loadable warrior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warrior {
    /// The warrior's name, for reporting.
    pub name: String,
    /// The instructions, in load order.
    pub code: Vec<Instruction>,
    /// Offset within `code` at which execution begins (the `ORG` / `END`
    /// address).
    pub start: u32,
}
