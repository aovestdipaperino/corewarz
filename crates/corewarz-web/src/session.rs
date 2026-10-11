//! A battle as the browser sees it.
//!
//! [`Session`] is plain Rust with no `wasm-bindgen` types, so everything the
//! page relies on is testable natively; `lib.rs` only adapts it to
//! JavaScript.

use corewarz_asm::{assemble, disassemble_instruction};
use corewarz_core::placement::random_positions;
use corewarz_core::{Config, Dialect, Mars, Observer, Outcome, WarriorId};

/// How much heat a cell loses per rendered frame.
const DECAY_STEP: u8 = 16;

/// The most warriors one battle admits; the page has a colour for each.
pub const MAX_WARRIORS: usize = 6;

/// The smallest core the page accepts.
pub const MIN_CORE_SIZE: u32 = 100;

/// The battle parameters the page exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// Accept the CoreWarz extended dialect.
    pub extended: bool,
    /// Number of cells in the core.
    pub core_size: u32,
    /// Cycles before an undecided battle is a tie.
    pub max_cycles: u32,
    /// Seed for warrior placement.
    pub seed: u32,
}

impl Settings {
    fn config(self) -> Config {
        let defaults = Config::default();
        Config {
            core_size: self.core_size,
            max_cycles: self.max_cycles,
            // The defaults assume an 8000-cell core; a smaller one must
            // still fit the warriors and the gaps between them.
            max_processes: self.core_size,
            max_length: defaults.max_length.min(self.core_size / 4),
            min_separation: defaults.min_separation.min(self.core_size / 4),
            dialect: if self.extended {
                Dialect::Extended
            } else {
                Dialect::Classic
            },
        }
    }
}

/// The warrior's `;name` comment, or a numbered fallback.
#[must_use]
pub fn warrior_name(source: &str, index: usize) -> String {
    source
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix(";name"))
        .map(str::trim)
        .find(|name| !name.is_empty())
        .map_or_else(|| format!("warrior {}", index + 1), str::to_string)
}

/// Per-cell activity and per-warrior positions the VM does not track.
struct Activity {
    heat: Vec<u8>,
    exec: Vec<Option<u32>>,
    deaths: Vec<u32>,
}

impl Observer for Activity {
    fn on_write(&mut self, addr: u32, _warrior: WarriorId) {
        self.heat[addr as usize] = u8::MAX;
    }

    fn on_exec(&mut self, addr: u32, warrior: WarriorId) {
        self.exec[warrior.0 as usize] = Some(addr);
    }

    fn on_death(&mut self, warrior: WarriorId, _addr: u32) {
        self.deaths[warrior.0 as usize] += 1;
    }
}

/// A loaded battle and the display state derived from it.
pub struct Session {
    mars: Mars,
    activity: Activity,
}

impl Session {
    /// Assembles `sources`, one per warrior, and places them from the seed.
    ///
    /// # Errors
    /// Returns a message naming the offending warrior if the settings are
    /// out of range or a warrior fails to assemble or be admitted.
    pub fn new(sources: &[String], settings: Settings) -> Result<Self, String> {
        if sources.is_empty() || sources.len() > MAX_WARRIORS {
            return Err(format!(
                "a battle needs between 1 and {MAX_WARRIORS} warriors, got {}",
                sources.len()
            ));
        }
        if settings.core_size < MIN_CORE_SIZE {
            return Err(format!("the core needs at least {MIN_CORE_SIZE} cells"));
        }
        let config = settings.config();
        let mut warriors = Vec::with_capacity(sources.len());
        for (index, source) in sources.iter().enumerate() {
            let name = warrior_name(source, index);
            let mut warrior = assemble(source, &config).map_err(|e| format!("{name}: {e}"))?;
            warrior.name = name;
            warriors.push(warrior);
        }
        let positions = random_positions(&config, warriors.len(), u64::from(settings.seed));
        let mars = Mars::try_new(config, warriors, &positions).map_err(|e| e.to_string())?;
        let count = sources.len();
        Ok(Self {
            activity: Activity {
                heat: vec![0; settings.core_size as usize],
                exec: vec![None; count],
                deaths: vec![0; count],
            },
            mars,
        })
    }

    /// Runs up to `cycles` cycles, stopping early when the battle ends.
    /// Returns whether the battle has ended.
    pub fn run(&mut self, cycles: u32) -> bool {
        for _ in 0..cycles {
            if self.mars.outcome().is_some() {
                break;
            }
            self.mars.step(&mut self.activity);
        }
        self.mars.outcome().is_some()
    }

    /// Cools every cell by one step. Call once per rendered frame.
    pub fn decay(&mut self) {
        for h in &mut self.activity.heat {
            *h = h.saturating_sub(DECAY_STEP);
        }
    }

    /// The battle.
    #[must_use]
    pub const fn mars(&self) -> &Mars {
        &self.mars
    }

    /// Each cell's owner as warrior index + 1, or 0 for a cell nobody wrote.
    #[must_use]
    pub fn owners(&self) -> Vec<u8> {
        let core = self.mars.core();
        (0..core.size())
            .map(|addr| core.owner(addr).map_or(0, |w| w.0 + 1))
            .collect()
    }

    /// How recently each cell was written, from 0 (cold) to 255.
    #[must_use]
    pub fn heat(&self) -> &[u8] {
        &self.activity.heat
    }

    /// The address `warrior` most recently executed.
    #[must_use]
    pub fn exec_position(&self, warrior: usize) -> Option<u32> {
        self.activity.exec.get(warrior).copied().flatten()
    }

    /// How many of `warrior`'s processes have died.
    #[must_use]
    pub fn deaths(&self, warrior: usize) -> u32 {
        self.activity.deaths.get(warrior).copied().unwrap_or(0)
    }

    /// The instruction at `addr`, disassembled.
    #[must_use]
    pub fn instruction_at(&self, addr: u32) -> String {
        let core = self.mars.core();
        disassemble_instruction(core.get(core.fold(i64::from(addr))))
    }

    /// A sentence describing how the battle ended, if it has.
    #[must_use]
    pub fn outcome_text(&self) -> Option<String> {
        self.mars.outcome().map(|outcome| match outcome {
            Outcome::Win(id) => format!("{} wins", self.mars.warriors()[id.0 as usize].name),
            Outcome::Tie => "tie: the cycle limit was reached".to_string(),
            Outcome::Draw => "draw: every warrior died".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMP: &str = ";name imp\nMOV.I $0, $1\n";
    const SUICIDE: &str = ";name suicide\nDAT.F $0, $0\n";

    fn settings() -> Settings {
        Settings {
            extended: false,
            core_size: 8000,
            max_cycles: 80_000,
            seed: 0,
        }
    }

    fn sources(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn names_come_from_the_name_comment() {
        assert_eq!(warrior_name(IMP, 0), "imp");
        assert_eq!(warrior_name("MOV 0, 1", 2), "warrior 3");
        assert_eq!(warrior_name(";name   \nMOV 0, 1", 0), "warrior 1");
    }

    #[test]
    fn a_suicidal_warrior_loses_on_the_first_cycle() {
        let mut session = Session::new(&sources(&[IMP, SUICIDE]), settings()).unwrap();
        assert!(session.run(10));
        assert_eq!(session.mars().cycle(), 1);
        assert_eq!(session.outcome_text().as_deref(), Some("imp wins"));
        assert_eq!(session.deaths(1), 1);
    }

    #[test]
    fn running_tracks_writes_and_execution() {
        let mut session = Session::new(&sources(&[IMP]), settings()).unwrap();
        assert!(!session.run(3));
        assert_eq!(session.exec_position(0), Some(2));
        let owners = session.owners();
        assert_eq!(&owners[0..4], &[1, 1, 1, 1]);
        assert_eq!(owners[4], 0);
        assert_eq!(session.heat()[3], 255);
        session.decay();
        assert_eq!(session.heat()[3], 255 - DECAY_STEP);
        assert_eq!(session.instruction_at(8003), session.instruction_at(3));
    }

    #[test]
    fn assembly_errors_name_the_warrior() {
        let err = Session::new(&sources(&[IMP, ";name broken\nFOO 1, 2"]), settings())
            .err()
            .unwrap();
        assert!(err.starts_with("broken: line 2"), "{err}");
    }

    #[test]
    fn extended_opcodes_need_the_extended_dialect() {
        let shl = ";name shifty\nSHL #1, $1\n";
        assert!(Session::new(&sources(&[shl]), settings()).is_err());
        let extended = Settings {
            extended: true,
            ..settings()
        };
        assert!(Session::new(&sources(&[shl]), extended).is_ok());
    }

    #[test]
    fn out_of_range_settings_are_rejected() {
        assert!(Session::new(&[], settings()).is_err());
        assert!(Session::new(&sources(&[IMP; 7]), settings()).is_err());
        let tiny = Settings {
            core_size: 10,
            ..settings()
        };
        assert!(Session::new(&sources(&[IMP]), tiny).is_err());
    }

    #[test]
    fn a_small_core_still_fits_several_warriors() {
        let small = Settings {
            core_size: 400,
            ..settings()
        };
        assert!(Session::new(&sources(&[IMP, IMP, IMP]), small).is_ok());
    }
}
