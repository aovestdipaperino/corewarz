//! The battle simulator.

use crate::config::Config;
use crate::core::Core;
use crate::exec::classic_data::StepResult;
use crate::exec::dispatch;
use crate::instruction::Instruction;
use crate::observer::Observer;
use crate::queue::ProcessQueue;
use crate::warrior::{Warrior, WarriorId};

/// How a battle ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Exactly one warrior still had live processes.
    Win(WarriorId),
    /// The cycle limit was reached with more than one warrior alive.
    Tie,
    /// Every warrior died, in the same cycle.
    Draw,
}

/// A warrior placed too close to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeparationError {
    /// Index of the first of the two warriors involved.
    pub first: usize,
    /// Index of the second.
    pub second: usize,
    /// The distance actually between them.
    pub distance: u32,
}

impl std::fmt::Display for SeparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "warriors {} and {} are only {} cells apart",
            self.first, self.second, self.distance
        )
    }
}

impl std::error::Error for SeparationError {}

/// Why a set of warriors could not be admitted to a battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    /// Two warriors were placed too close together.
    Separation(SeparationError),
    /// A warrior contains a CoreWarz extension under the classic dialect.
    ExtendedOpcode {
        /// Index of the offending warrior.
        warrior: usize,
        /// Offset of the offending instruction within that warrior.
        offset: usize,
        /// The offending opcode.
        opcode: crate::instruction::Opcode,
    },
    /// A warrior is longer than `Config::max_length`.
    TooLong {
        warrior: usize,
        length: u32,
        limit: u32,
    },
}

impl std::fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Separation(e) => write!(f, "{e}"),
            Self::ExtendedOpcode {
                warrior,
                offset,
                opcode,
            } => write!(
                f,
                "warrior {warrior} instruction {offset}: {opcode:?} is a CoreWarz extension \
                 and is illegal in classic mode"
            ),
            Self::TooLong {
                warrior,
                length,
                limit,
            } => {
                write!(
                    f,
                    "warrior {warrior} is {length} instructions long, the limit is {limit}"
                )
            }
        }
    }
}

impl std::error::Error for AdmissionError {}

/// A running battle.
#[derive(Debug, Clone)]
pub struct Mars {
    config: Config,
    core: Core,
    warriors: Vec<Warrior>,
    queues: Vec<ProcessQueue>,
    cycle: u32,
}

impl Mars {
    /// Validates `warriors` against `config` and loads them at `positions`.
    ///
    /// This is the second of the two dialect chokepoints: the assembler
    /// rejects extended opcodes in source, and this rejects them in any
    /// warrior that reached the VM by another route. The interpreter's hot
    /// loop therefore contains no dialect branch.
    ///
    /// # Errors
    /// Returns the first [`AdmissionError`] found.
    ///
    /// # Panics
    /// Panics if `positions.len() != warriors.len()`.
    pub fn try_new(
        config: Config,
        warriors: Vec<Warrior>,
        positions: &[u32],
    ) -> Result<Self, AdmissionError> {
        assert_eq!(warriors.len(), positions.len(), "one position per warrior");
        Self::check_positions(&config, positions).map_err(AdmissionError::Separation)?;
        for (index, warrior) in warriors.iter().enumerate() {
            let length = u32::try_from(warrior.code.len()).unwrap_or(u32::MAX);
            if length > config.max_length {
                return Err(AdmissionError::TooLong {
                    warrior: index,
                    length,
                    limit: config.max_length,
                });
            }
            if config.dialect == crate::config::Dialect::Classic {
                for (offset, instr) in warrior.code.iter().enumerate() {
                    if instr.opcode.is_extended() {
                        return Err(AdmissionError::ExtendedOpcode {
                            warrior: index,
                            offset,
                            opcode: instr.opcode,
                        });
                    }
                }
            }
        }
        Ok(Self::new(config, warriors, positions))
    }

    /// Loads `warriors` at `positions` and prepares them to run.
    ///
    /// `positions` must have the same length as `warriors`. Use
    /// [`Mars::check_positions`] first if the caller has not already validated
    /// separation.
    ///
    /// # Panics
    /// Panics if `positions.len() != warriors.len()`.
    #[must_use]
    pub fn new(config: Config, warriors: Vec<Warrior>, positions: &[u32]) -> Self {
        assert_eq!(warriors.len(), positions.len(), "one position per warrior");
        let mut core = Core::new(&config);
        let mut queues = Vec::with_capacity(warriors.len());
        for (index, warrior) in warriors.iter().enumerate() {
            let id = WarriorId(u8::try_from(index).expect("at most 256 warriors"));
            let base = positions[index];
            for (offset, instr) in warrior.code.iter().enumerate() {
                let addr = core.fold(i64::from(base) + offset as i64);
                core.set(addr, *instr, id);
            }
            let mut queue = ProcessQueue::new(config.max_processes);
            queue.push(core.fold(i64::from(base) + i64::from(warrior.start)));
            queues.push(queue);
        }
        Self {
            config,
            core,
            warriors,
            queues,
            cycle: 0,
        }
    }

    /// Verifies that no two positions are closer than `config.min_separation`,
    /// measuring distance circularly.
    ///
    /// # Errors
    /// Returns the first violating pair.
    pub fn check_positions(config: &Config, positions: &[u32]) -> Result<(), SeparationError> {
        let size = i64::from(config.core_size);
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let raw = (i64::from(positions[j]) - i64::from(positions[i])).rem_euclid(size);
                let distance = raw.min(size - raw) as u32;
                if distance < config.min_separation {
                    return Err(SeparationError {
                        first: i,
                        second: j,
                        distance,
                    });
                }
            }
        }
        Ok(())
    }

    /// The battle configuration.
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// The core, for display and inspection.
    #[must_use]
    pub const fn core(&self) -> &Core {
        &self.core
    }

    /// Mutable access to the core.
    ///
    /// Intended for tests and tooling that need to plant or corrupt cells; the
    /// VM itself never needs it.
    pub fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// The loaded warriors.
    #[must_use]
    pub fn warriors(&self) -> &[Warrior] {
        &self.warriors
    }

    /// Cycles executed so far.
    #[must_use]
    pub const fn cycle(&self) -> u32 {
        self.cycle
    }

    /// The number of live processes belonging to `warrior`.
    #[must_use]
    pub fn process_count(&self, warrior: WarriorId) -> usize {
        self.queues[warrior.0 as usize].len()
    }

    /// The program counter `warrior` will execute next, if it is alive.
    #[must_use]
    pub fn process_pc(&self, warrior: WarriorId) -> Option<u32> {
        self.queues[warrior.0 as usize].front()
    }

    /// Alias of [`Mars::process_pc`], for readability at call sites.
    #[must_use]
    pub fn current_pc(&self, warrior: WarriorId) -> Option<u32> {
        self.process_pc(warrior)
    }

    /// The instruction `warrior` will execute next, if it is alive.
    #[must_use]
    pub fn current_instruction(&self, warrior: WarriorId) -> Option<Instruction> {
        self.process_pc(warrior).map(|pc| self.core.get(pc))
    }

    /// Runs one cycle: every live warrior executes one instruction, in warrior
    /// order.
    pub fn step(&mut self, observer: &mut impl Observer) {
        for index in 0..self.queues.len() {
            let id = WarriorId(index as u8);
            let Some(pc) = self.queues[index].pop() else {
                continue;
            };
            observer.on_exec(pc, id);
            let result = dispatch(&mut self.core, pc, id);
            for (addr, writer) in self.core.take_writes() {
                observer.on_write(addr, writer);
            }
            match result {
                StepResult::Continue => {
                    self.queues[index].push(self.core.fold(i64::from(pc) + 1));
                }
                StepResult::Skip => {
                    self.queues[index].push(self.core.fold(i64::from(pc) + 2));
                }
                StepResult::Jump(target) => {
                    self.queues[index].push(target);
                }
                StepResult::Split(target) => {
                    self.queues[index].push(self.core.fold(i64::from(pc) + 1));
                    self.queues[index].push(target);
                }
                StepResult::Die => {
                    observer.on_death(id, pc);
                }
            }
        }
        self.cycle += 1;
    }

    /// Returns the outcome if the battle has ended, otherwise `None`.
    #[must_use]
    pub fn outcome(&self) -> Option<Outcome> {
        let alive: Vec<WarriorId> = (0..self.queues.len())
            .filter(|i| !self.queues[*i].is_empty())
            .map(|i| WarriorId(i as u8))
            .collect();
        match alive.len() {
            0 => Some(Outcome::Draw),
            1 if self.queues.len() > 1 => Some(Outcome::Win(alive[0])),
            _ if self.cycle >= self.config.max_cycles => Some(Outcome::Tie),
            _ => None,
        }
    }

    /// Runs until the battle ends or the cycle limit is reached.
    pub fn run(&mut self, observer: &mut impl Observer) -> Outcome {
        loop {
            if let Some(outcome) = self.outcome() {
                return outcome;
            }
            self.step(observer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{AddrMode, Modifier, Opcode};
    use crate::observer::NullObserver;

    fn warrior(name: &str, code: Vec<Instruction>) -> Warrior {
        Warrior {
            name: name.to_string(),
            code,
            start: 0,
        }
    }

    fn i(opcode: Opcode, a: u32, b: u32) -> Instruction {
        Instruction {
            opcode,
            modifier: Modifier::B,
            a_mode: AddrMode::Direct,
            a,
            b_mode: AddrMode::Direct,
            b,
        }
    }

    /// `imp`: MOV.I $0, $1 — the simplest warrior that survives forever.
    fn imp() -> Warrior {
        warrior(
            "imp",
            vec![Instruction {
                modifier: Modifier::I,
                ..i(Opcode::Mov, 0, 1)
            }],
        )
    }

    /// A warrior that immediately executes DAT and dies.
    fn suicide() -> Warrior {
        warrior("suicide", vec![i(Opcode::Dat, 0, 0)])
    }

    #[test]
    fn warriors_load_at_the_requested_addresses() {
        let mars = Mars::new(Config::default(), vec![imp(), suicide()], &[0, 4000]);
        assert_eq!(mars.core().get(0).opcode, Opcode::Mov);
        assert_eq!(mars.core().get(4000).opcode, Opcode::Dat);
        assert_eq!(mars.core().owner(0), Some(WarriorId(0)));
        assert_eq!(mars.core().owner(4000), Some(WarriorId(1)));
    }

    #[test]
    fn a_warrior_that_executes_dat_loses() {
        let mut mars = Mars::new(Config::default(), vec![imp(), suicide()], &[0, 4000]);
        let outcome = mars.run(&mut NullObserver);
        assert_eq!(outcome, Outcome::Win(WarriorId(0)));
        assert_eq!(mars.process_count(WarriorId(1)), 0);
    }

    #[test]
    fn two_imps_tie_at_the_cycle_limit() {
        let config = Config {
            max_cycles: 500,
            ..Config::default()
        };
        let mut mars = Mars::new(config, vec![imp(), imp()], &[0, 4000]);
        assert_eq!(mars.run(&mut NullObserver), Outcome::Tie);
        assert_eq!(mars.cycle(), 500);
    }

    #[test]
    fn an_imp_advances_one_cell_per_cycle() {
        let mut mars = Mars::new(Config::default(), vec![imp()], &[0]);
        assert_eq!(mars.current_pc(WarriorId(0)), Some(0));
        mars.step(&mut NullObserver);
        assert_eq!(mars.core().get(1).opcode, Opcode::Mov);
        assert_eq!(mars.current_pc(WarriorId(0)), Some(1));
    }

    #[test]
    fn spl_increases_the_process_count() {
        let bomb = warrior("splitter", vec![i(Opcode::Spl, 0, 0), i(Opcode::Jmp, 0, 0)]);
        let mut mars = Mars::new(Config::default(), vec![bomb], &[0]);
        mars.step(&mut NullObserver);
        assert_eq!(mars.process_count(WarriorId(0)), 2);
    }

    #[test]
    fn spl_is_a_no_op_at_the_process_limit() {
        let config = Config {
            max_processes: 1,
            ..Config::default()
        };
        let splitter = warrior("splitter", vec![i(Opcode::Spl, 0, 0)]);
        let mut mars = Mars::new(config, vec![splitter], &[0]);
        mars.step(&mut NullObserver);
        assert_eq!(mars.process_count(WarriorId(0)), 1);
    }

    #[test]
    fn skip_advances_two_instructions() {
        let code = vec![
            Instruction {
                modifier: Modifier::I,
                ..i(Opcode::Seq, 0, 0)
            },
            i(Opcode::Dat, 0, 0),
            i(Opcode::Jmp, 0, 0),
        ];
        let mut mars = Mars::new(Config::default(), vec![warrior("skipper", code)], &[0]);
        mars.step(&mut NullObserver);
        assert_eq!(
            mars.current_pc(WarriorId(0)),
            Some(2),
            "SEQ of a cell with itself must skip"
        );
    }

    #[test]
    fn the_observer_sees_executions_writes_and_deaths() {
        #[derive(Default)]
        struct Recorder {
            execs: Vec<u32>,
            writes: Vec<u32>,
            deaths: Vec<WarriorId>,
        }
        impl Observer for Recorder {
            fn on_exec(&mut self, addr: u32, _w: WarriorId) {
                self.execs.push(addr);
            }
            fn on_write(&mut self, addr: u32, _w: WarriorId) {
                self.writes.push(addr);
            }
            fn on_death(&mut self, w: WarriorId, _addr: u32) {
                self.deaths.push(w);
            }
        }

        let mut rec = Recorder::default();
        let mut mars = Mars::new(Config::default(), vec![imp(), suicide()], &[0, 4000]);
        mars.step(&mut rec);
        assert_eq!(rec.execs, vec![0, 4000]);
        assert!(rec.writes.contains(&1), "the imp writes to cell 1");
        assert_eq!(rec.deaths, vec![WarriorId(1)]);
    }

    #[test]
    fn check_positions_rejects_warriors_that_are_too_close() {
        let config = Config {
            min_separation: 100,
            ..Config::default()
        };
        assert!(Mars::check_positions(&config, &[0, 4000]).is_ok());
        assert!(Mars::check_positions(&config, &[0, 50]).is_err());
        assert!(
            Mars::check_positions(&config, &[0, 7960]).is_err(),
            "separation wraps"
        );
    }

    #[test]
    fn spl_enqueues_the_continuation_before_the_split_target() {
        let code = vec![
            i(Opcode::Spl, 5, 0), // pc=0: SPL with A=5, targets pc=5
            i(Opcode::Dat, 0, 0), // pc=1: DAT (continuation dies)
            i(Opcode::Mov, 0, 0), // pc=2-4: filler
            i(Opcode::Mov, 0, 0),
            i(Opcode::Mov, 0, 0),
            i(Opcode::Jmp, 0, 0), // pc=5: JMP (split target)
        ];
        let mut mars = Mars::new(Config::default(), vec![warrior("splitter", code)], &[0]);

        // Execute one cycle to split
        mars.step(&mut NullObserver);
        assert_eq!(
            mars.process_count(WarriorId(0)),
            2,
            "SPL must create two processes"
        );

        // SPL must enqueue the continuation first, before the split target.
        // The front of the queue should be pc+1 (the continuation at pc=1), not pc=5 (the target).
        // If the implementation enqueued in the wrong order, this would fail loudly.
        assert_eq!(
            mars.current_pc(WarriorId(0)),
            Some(1),
            "SPL must enqueue the continuation first, before the split target"
        );

        // Execute a second cycle to verify both processes are present and in the right order
        mars.step(&mut NullObserver);
        assert_eq!(
            mars.process_count(WarriorId(0)),
            1,
            "continuation at pc=1 (DAT) should die, leaving only the split target"
        );
        assert_eq!(
            mars.current_pc(WarriorId(0)),
            Some(5),
            "remaining process should be at the split target (pc=5)"
        );
    }
}
