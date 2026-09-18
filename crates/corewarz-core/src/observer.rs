//! Battle observation.
//!
//! This trait is the only interface through which anything outside the VM
//! learns what a battle is doing. The TUI implements it to drive its display;
//! the differential test harness implements it to produce an execution trace.

use crate::warrior::WarriorId;

/// Receives notifications as a battle runs.
///
/// Every method has a default no-op implementation, so an implementor need
/// only override what it cares about.
pub trait Observer {
    /// A cell was written by `warrior`.
    fn on_write(&mut self, addr: u32, warrior: WarriorId) {
        let _ = (addr, warrior);
    }

    /// `warrior` is about to execute the instruction at `addr`.
    fn on_exec(&mut self, addr: u32, warrior: WarriorId) {
        let _ = (addr, warrior);
    }

    /// A process of `warrior` died executing the instruction at `addr`.
    fn on_death(&mut self, warrior: WarriorId, addr: u32) {
        let _ = (warrior, addr);
    }
}

/// An [`Observer`] that does nothing.
///
/// The compiler erases it entirely, so headless runs pay no observation cost.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullObserver;

impl Observer for NullObserver {}
