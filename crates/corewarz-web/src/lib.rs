//! WebAssembly bindings that run CoreWarz battles in a browser.
//!
//! The page in `web/` at the workspace root loads this crate (built with
//! `web/build.sh`) and drives a [`Battle`] from its animation loop: it calls
//! [`Battle::run`] with a cycle budget, reads the per-cell owner and heat
//! buffers to paint the core, then calls [`Battle::decay`] once per frame.
//!
//! All the logic lives in [`session`], which has no `wasm-bindgen` types and
//! is tested natively; this module only adapts it to JavaScript.

pub mod samples;
pub mod session;

use wasm_bindgen::prelude::*;

use corewarz_core::WarriorId;
use session::{Session, Settings};

/// A battle running in the page.
#[wasm_bindgen]
pub struct Battle {
    session: Session,
}

#[wasm_bindgen]
impl Battle {
    /// Assembles `sources`, one per warrior, and places them from `seed`.
    ///
    /// # Errors
    /// Throws a message naming the offending warrior if one fails to
    /// assemble or the settings cannot admit the battle.
    #[wasm_bindgen(constructor)]
    pub fn new(
        sources: Vec<String>,
        extended: bool,
        core_size: u32,
        max_cycles: u32,
        seed: u32,
    ) -> Result<Battle, JsError> {
        let settings = Settings {
            extended,
            core_size,
            max_cycles,
            seed,
        };
        Session::new(&sources, settings)
            .map(|session| Self { session })
            .map_err(|message| JsError::new(&message))
    }

    /// Runs up to `cycles` cycles. Returns whether the battle has ended.
    pub fn run(&mut self, cycles: u32) -> bool {
        self.session.run(cycles)
    }

    /// Cools every cell by one step. Call once per rendered frame.
    pub fn decay(&mut self) {
        self.session.decay();
    }

    /// Cycles executed so far.
    #[must_use]
    pub fn cycle(&self) -> u32 {
        self.session.mars().cycle()
    }

    /// The cycle limit.
    #[must_use]
    #[wasm_bindgen(js_name = maxCycles)]
    pub fn max_cycles(&self) -> u32 {
        self.session.mars().config().max_cycles
    }

    /// Number of cells in the core.
    #[must_use]
    #[wasm_bindgen(js_name = coreSize)]
    pub fn core_size(&self) -> u32 {
        self.session.mars().core().size()
    }

    /// Each cell's owner as warrior index + 1, or 0 for an unwritten cell.
    #[must_use]
    pub fn owners(&self) -> Vec<u8> {
        self.session.owners()
    }

    /// How recently each cell was written, from 0 (cold) to 255.
    #[must_use]
    pub fn heat(&self) -> Vec<u8> {
        self.session.heat().to_vec()
    }

    /// Number of warriors in the battle.
    #[must_use]
    #[wasm_bindgen(js_name = warriorCount)]
    pub fn warrior_count(&self) -> usize {
        self.session.mars().warriors().len()
    }

    /// The name of warrior `index`.
    #[must_use]
    #[wasm_bindgen(js_name = warriorName)]
    pub fn warrior_name(&self, index: usize) -> String {
        self.session.mars().warriors()[index].name.clone()
    }

    /// Live processes of warrior `index`.
    #[must_use]
    #[wasm_bindgen(js_name = processCount)]
    pub fn process_count(&self, index: usize) -> usize {
        self.session.mars().process_count(warrior_id(index))
    }

    /// Processes of warrior `index` that have died.
    #[must_use]
    pub fn deaths(&self, index: usize) -> u32 {
        self.session.deaths(index)
    }

    /// The address warrior `index` executes next, if it is alive.
    #[must_use]
    #[wasm_bindgen(js_name = nextPc)]
    pub fn next_pc(&self, index: usize) -> Option<u32> {
        self.session.mars().process_pc(warrior_id(index))
    }

    /// The address warrior `index` most recently executed.
    #[must_use]
    #[wasm_bindgen(js_name = execPosition)]
    pub fn exec_position(&self, index: usize) -> Option<u32> {
        self.session.exec_position(index)
    }

    /// The instruction at `addr`, disassembled.
    #[must_use]
    #[wasm_bindgen(js_name = instructionAt)]
    pub fn instruction_at(&self, addr: u32) -> String {
        self.session.instruction_at(addr)
    }

    /// How the battle ended, or `undefined` while it is still running.
    #[must_use]
    pub fn outcome(&self) -> Option<String> {
        self.session.outcome_text()
    }
}

/// Names of the bundled sample warriors, in display order.
#[must_use]
#[wasm_bindgen(js_name = sampleNames)]
pub fn sample_names() -> Vec<String> {
    samples::SAMPLES
        .iter()
        .map(|sample| sample.name.to_string())
        .collect()
}

/// The source of the sample called `name`.
#[must_use]
#[wasm_bindgen(js_name = sampleSource)]
pub fn sample_source(name: &str) -> Option<String> {
    samples::find(name).map(|sample| sample.source.to_string())
}

/// Whether the sample called `name` needs the extended dialect.
#[must_use]
#[wasm_bindgen(js_name = sampleIsExtended)]
pub fn sample_is_extended(name: &str) -> bool {
    samples::find(name).is_some_and(|sample| sample.extended)
}

fn warrior_id(index: usize) -> WarriorId {
    WarriorId(u8::try_from(index).expect("at most six warriors"))
}
