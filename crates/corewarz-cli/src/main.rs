//! The `corewarz` command-line interface.

mod battle;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use corewarz_asm::assemble;
use corewarz_core::{Config, Dialect, Outcome, Warrior, WarriorId};

#[derive(Parser)]
#[command(name = "corewarz", about = "An extended Core War", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run one or more battles headlessly and report the results.
    Run(RunArgs),
}

#[derive(Args, Clone)]
struct RunArgs {
    /// Redcode source files, one per warrior.
    #[arg(required = true)]
    warriors: Vec<PathBuf>,

    /// Number of cells in the core.
    #[arg(long, default_value_t = 8000)]
    core_size: u32,

    /// Cycles before an undecided battle is a tie.
    #[arg(long = "cycles", default_value_t = 80_000)]
    max_cycles: u32,

    /// Maximum simultaneous processes per warrior.
    #[arg(long, default_value_t = 8000)]
    max_processes: u32,

    /// Maximum instructions in a warrior.
    #[arg(long, default_value_t = 100)]
    max_length: u32,

    /// Minimum distance between two warriors.
    #[arg(long, default_value_t = 100)]
    min_separation: u32,

    /// Accept the CoreWarz extended dialect.
    #[arg(long)]
    extended: bool,

    /// Number of rounds to fight.
    #[arg(long, default_value_t = 1)]
    rounds: u32,

    /// Seed for warrior placement. The same seed reproduces the same battles.
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Explicit comma-separated load addresses, one per warrior.
    #[arg(long, value_delimiter = ',')]
    positions: Option<Vec<u32>>,
}

impl RunArgs {
    fn config(&self) -> Config {
        Config {
            core_size: self.core_size,
            max_cycles: self.max_cycles,
            max_processes: self.max_processes,
            max_length: self.max_length,
            min_separation: self.min_separation,
            dialect: if self.extended {
                Dialect::Extended
            } else {
                Dialect::Classic
            },
        }
    }
}

fn warrior_name(path: &Path) -> String {
    path.file_stem().map_or_else(
        || "warrior".to_string(),
        |s| s.to_string_lossy().into_owned(),
    )
}

fn load(path: &Path, config: &Config) -> Result<Warrior, String> {
    let source = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut warrior = assemble(&source, config).map_err(|e| format!("{}: {e}", path.display()))?;
    warrior.name = warrior_name(path);
    Ok(warrior)
}

fn run(args: &RunArgs) -> Result<(), String> {
    let config = args.config();
    let mut warriors = Vec::with_capacity(args.warriors.len());
    for path in &args.warriors {
        warriors.push(load(path, &config)?);
    }

    let mut wins = vec![0u32; warriors.len()];
    let mut ties = 0u32;
    let mut draws = 0u32;

    for round in 0..args.rounds {
        let positions = match &args.positions {
            Some(explicit) => {
                if explicit.len() != warriors.len() {
                    return Err(format!(
                        "got {} positions for {} warriors",
                        explicit.len(),
                        warriors.len()
                    ));
                }
                explicit.clone()
            }
            None => battle::random_positions(
                &config,
                warriors.len(),
                args.seed.wrapping_add(u64::from(round)),
            ),
        };
        match battle::run_battle(&warriors, &config, &positions).map_err(|e| e.to_string())? {
            Outcome::Win(WarriorId(id)) => wins[id as usize] += 1,
            Outcome::Tie => ties += 1,
            Outcome::Draw => draws += 1,
        }
    }

    for (index, warrior) in warriors.iter().enumerate() {
        println!("{:<20} wins {}", warrior.name, wins[index]);
    }
    println!("{:<20} {ties}", "ties");
    println!("{:<20} {draws}", "draws");
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match &cli.command {
        Command::Run(args) => run(args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}
