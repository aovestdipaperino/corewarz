//! Display state derived from a battle.
//!
//! [`VisualState`] implements [`Observer`] and holds everything the renderer
//! needs that the VM does not itself track. It contains no terminal code and
//! is fully testable without one.

use std::collections::HashMap;

use corewarz_core::{Observer, WarriorId};

/// How much heat a cell loses per rendered frame.
const DECAY_STEP: u8 = 16;

/// Per-cell activity and per-warrior positions, for rendering.
#[derive(Debug, Clone)]
pub struct VisualState {
    heat: Vec<u8>,
    exec: HashMap<WarriorId, u32>,
    deaths: HashMap<WarriorId, u32>,
}

impl VisualState {
    /// Creates state for a core of `core_size` cells.
    #[must_use]
    pub fn new(core_size: u32) -> Self {
        Self {
            heat: vec![0; core_size as usize],
            exec: HashMap::new(),
            deaths: HashMap::new(),
        }
    }

    /// How recently `addr` was written, from 0 (cold) to 255 (just written).
    #[must_use]
    pub fn heat(&self, addr: u32) -> u8 {
        self.heat[addr as usize]
    }

    /// Cools every cell by one step. Call once per rendered frame.
    pub fn decay(&mut self) {
        for h in &mut self.heat {
            *h = h.saturating_sub(DECAY_STEP);
        }
    }

    /// The address `warrior` most recently executed.
    #[must_use]
    pub fn exec_position(&self, warrior: WarriorId) -> Option<u32> {
        self.exec.get(&warrior).copied()
    }

    /// How many of `warrior`'s processes have died.
    #[must_use]
    pub fn deaths(&self, warrior: WarriorId) -> u32 {
        self.deaths.get(&warrior).copied().unwrap_or(0)
    }

    /// How many core cells each rendered character must represent so that a
    /// core of `core_size` fits into `available` characters.
    ///
    /// Rounds up, so no cell is ever left off the display.
    #[must_use]
    pub const fn cells_per_cell(core_size: u32, available: u32) -> u32 {
        if available == 0 {
            return 1;
        }
        core_size.div_ceil(available)
    }
}

impl Observer for VisualState {
    fn on_write(&mut self, addr: u32, _warrior: WarriorId) {
        self.heat[addr as usize] = u8::MAX;
    }

    fn on_exec(&mut self, addr: u32, warrior: WarriorId) {
        self.exec.insert(warrior, addr);
    }

    fn on_death(&mut self, warrior: WarriorId, _addr: u32) {
        *self.deaths.entry(warrior).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use corewarz_core::{Config, Instruction, Mars, Observer, Warrior, WarriorId};

    fn state() -> VisualState {
        VisualState::new(8000)
    }

    #[test]
    fn a_write_makes_a_cell_maximally_hot() {
        let mut s = state();
        assert_eq!(s.heat(42), 0);
        s.on_write(42, WarriorId(0));
        assert_eq!(s.heat(42), 255);
    }

    #[test]
    fn heat_decays_towards_zero_and_stops_there() {
        let mut s = state();
        s.on_write(42, WarriorId(0));
        for _ in 0..100 {
            s.decay();
        }
        assert_eq!(s.heat(42), 0, "heat must not underflow");
    }

    #[test]
    fn execution_positions_are_tracked_per_warrior() {
        let mut s = state();
        s.on_exec(100, WarriorId(0));
        s.on_exec(200, WarriorId(1));
        assert_eq!(s.exec_position(WarriorId(0)), Some(100));
        assert_eq!(s.exec_position(WarriorId(1)), Some(200));
        assert_eq!(s.exec_position(WarriorId(2)), None);
    }

    #[test]
    fn deaths_are_counted_per_warrior() {
        let mut s = state();
        s.on_death(WarriorId(1), 500);
        s.on_death(WarriorId(1), 501);
        assert_eq!(s.deaths(WarriorId(1)), 2);
        assert_eq!(s.deaths(WarriorId(0)), 0);
    }

    #[test]
    fn the_state_observes_a_real_battle_without_a_terminal() {
        let imp = Warrior {
            name: "imp".into(),
            code: vec![Instruction {
                opcode: corewarz_core::Opcode::Mov,
                modifier: corewarz_core::Modifier::I,
                a_mode: corewarz_core::AddrMode::Direct,
                a: 0,
                b_mode: corewarz_core::AddrMode::Direct,
                b: 1,
            }],
            start: 0,
        };
        let mut mars = Mars::new(Config::default(), vec![imp], &[0]);
        let mut s = state();
        mars.step(&mut s);
        assert_eq!(s.exec_position(WarriorId(0)), Some(0));
        assert_eq!(s.heat(1), 255, "the imp's write is hot");
    }

    #[test]
    fn cells_per_column_aggregates_a_core_larger_than_the_view() {
        assert_eq!(VisualState::cells_per_cell(8000, 8000), 1);
        assert_eq!(VisualState::cells_per_cell(16_000, 8000), 2);
        assert_eq!(
            VisualState::cells_per_cell(8000, 3000),
            3,
            "rounds up so all cells fit"
        );
    }
}
