//! Differential testing against pMARS, the reference implementation.
//!
//! Any divergence in classic mode is a bug in CoreWarz. These tests are
//! skipped when the pMARS binary is absent; build it with
//! `tools/build-pmars.sh`.

use std::path::{Path, PathBuf};
use std::process::Command;

use corewarz_asm::assemble;
use corewarz_core::{Config, Mars, NullObserver, Outcome, Warrior, WarriorId};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn pmars_binary() -> Option<PathBuf> {
    let path = repo_root().join("tools/pmars/pmars");
    path.is_file().then_some(path)
}

fn corpus() -> Vec<PathBuf> {
    let dir = repo_root().join("tests/corpus");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("corpus directory exists")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "red"))
        .collect();
    files.sort();
    files
}

fn load(path: &Path, config: &Config) -> Warrior {
    let source = std::fs::read_to_string(path).expect("warrior file readable");
    let mut w = assemble(&source, config)
        .unwrap_or_else(|e| panic!("{} must assemble as classic Redcode: {e}", path.display()));
    w.name = path.file_stem().unwrap().to_string_lossy().into_owned();
    w
}

/// Runs pMARS on two warriors at fixed positions and returns, for each
/// warrior, whether it survived.
///
/// Confirmed against pMARS 0.9.3, built from
/// <https://github.com/akosela/pmars> (see `tests/corpus/README.md`):
/// `-r <n>` rounds, `-s <n>` core size, `-c <n>` cycles, `-p <n>` max
/// processes, `-l <n>` max warrior length, `-d <n>` minimum separation,
/// `-F <n>` fixes warrior #2's load position relative to warrior #1 (which
/// this harness always fixes at 0), and `-b` selects brief mode (no source
/// listing).
///
/// With `-r 1`, pMARS prints a single summary line `Results: W L T` where
/// exactly one of the three counters is 1: `W` means warrior #1 won, `L`
/// means warrior #2 won, `T` means the round was a tie (both still had live
/// processes when the cycle limit was reached). That is what this harness
/// parses; the earlier plan's assumption of one parseable "wins" score line
/// per warrior did not match this build's actual output and was replaced by
/// this simpler, unambiguous per-round result.
fn pmars_survivors(
    binary: &Path,
    config: &Config,
    a: &Path,
    b: &Path,
    positions: (u32, u32),
) -> Vec<bool> {
    assert_eq!(
        positions.0, 0,
        "harness always fixes the first warrior at 0"
    );
    let output = Command::new(binary)
        .args([
            "-r",
            "1",
            "-s",
            &config.core_size.to_string(),
            "-c",
            &config.max_cycles.to_string(),
            "-p",
            &config.max_processes.to_string(),
            "-l",
            &config.max_length.to_string(),
            "-d",
            &config.min_separation.to_string(),
            "-F",
            &positions.1.to_string(),
            "-b",
        ])
        .arg(a)
        .arg(b)
        .current_dir(repo_root())
        .output()
        .expect("pmars runs");
    assert!(
        output.status.success(),
        "pmars failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let text = String::from_utf8_lossy(&output.stdout);
    let results_line = text
        .lines()
        .find(|line| line.trim_start().starts_with("Results:"))
        .unwrap_or_else(|| panic!("no Results line in pmars output: {text}"));
    let counters: Vec<u32> = results_line
        .split_whitespace()
        .filter_map(|token| token.parse::<u32>().ok())
        .collect();
    assert_eq!(
        counters.len(),
        3,
        "expected 'Results: W L T', got: {results_line}"
    );
    let (wins, losses, ties) = (counters[0], counters[1], counters[2]);
    assert_eq!(
        wins + losses + ties,
        1,
        "expected exactly one round, got: {results_line}"
    );
    match (wins, losses, ties) {
        (1, 0, 0) => vec![true, false],
        (0, 1, 0) => vec![false, true],
        (0, 0, 1) => vec![true, true],
        _ => panic!("unexpected Results line: {results_line}"),
    }
}

fn corewarz_survivors(config: &Config, warriors: Vec<Warrior>, positions: &[u32]) -> Vec<bool> {
    let count = warriors.len();
    let mut mars = Mars::try_new(*config, warriors, positions).expect("warriors are admissible");
    let outcome = mars.run(&mut NullObserver);
    match outcome {
        Outcome::Win(WarriorId(id)) => (0..count).map(|i| i == id as usize).collect(),
        Outcome::Tie | Outcome::Draw => (0..count)
            .map(|i| mars.process_count(WarriorId(i as u8)) > 0)
            .collect(),
    }
}

#[test]
fn classic_battles_match_pmars() {
    let Some(binary) = pmars_binary() else {
        eprintln!("skipping: tools/pmars/pmars not built — run tools/build-pmars.sh");
        return;
    };

    let configs = [
        Config::default(),
        Config {
            core_size: 800,
            max_cycles: 8000,
            max_length: 20,
            min_separation: 20,
            ..Config::default()
        },
        Config {
            core_size: 55_440,
            max_cycles: 500_000,
            max_length: 200,
            min_separation: 200,
            ..Config::default()
        },
    ];

    let files = corpus();
    let mut compared = 0;
    for config in configs {
        for (i, first) in files.iter().enumerate() {
            for second in files.iter().skip(i + 1) {
                // Run each unordered pair in both load orders: which warrior
                // is "#1" affects nothing in principle, but exercising both
                // orders doubles coverage of the operand-resolution and
                // scheduling code for free.
                for (a, b) in [(first, second), (second, first)] {
                    for offset in [
                        100u32,
                        300,
                        1234,
                        config.core_size / 3,
                        config.core_size / 2,
                        config.core_size.saturating_sub(150),
                    ] {
                        if offset == 0
                            || offset >= config.core_size
                            || offset < config.min_separation
                            || config.core_size - offset < config.min_separation
                        {
                            continue;
                        }
                        let positions = [0, offset];
                        let ours = corewarz_survivors(
                            &config,
                            vec![load(a, &config), load(b, &config)],
                            &positions,
                        );
                        let theirs = pmars_survivors(&binary, &config, a, b, (0, offset));
                        assert_eq!(
                            ours,
                            theirs,
                            "divergence: {} vs {} at offset {offset}, core {}",
                            a.display(),
                            b.display(),
                            config.core_size
                        );
                        compared += 1;
                    }
                }
            }
        }
    }
    assert!(compared > 0, "the matrix must actually run");
    eprintln!("compared {compared} battles against pMARS");
}

#[test]
fn every_corpus_warrior_is_legal_classic_redcode() {
    let config = Config::default();
    for path in corpus() {
        let source = std::fs::read_to_string(&path).expect("readable");
        assert!(
            assemble(&source, &config).is_ok(),
            "{} must be legal classic Redcode",
            path.display()
        );
    }
}
