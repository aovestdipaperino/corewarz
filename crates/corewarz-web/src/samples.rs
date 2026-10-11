//! Sample warriors bundled into the page.
//!
//! These are the same files the repository ships under `warriors/`,
//! `tests/corpus/` and `docs/snippets/`, compiled in so the page needs no
//! server-side file access.

/// A bundled warrior.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// The name shown in the page's picker.
    pub name: &'static str,
    /// The Redcode source.
    pub source: &'static str,
    /// Whether it uses CoreWarz extensions.
    pub extended: bool,
}

macro_rules! sample {
    ($name:literal, $path:literal, $extended:literal) => {
        Sample {
            name: $name,
            source: include_str!(concat!("../../../", $path)),
            extended: $extended,
        }
    };
}

/// Every bundled warrior: classic fighters first, then extended demos.
pub const SAMPLES: &[Sample] = &[
    sample!("imp", "warriors/imp.red", false),
    sample!("dwarf", "warriors/dwarf.red", false),
    sample!("creeper", "tests/corpus/creeper.red", false),
    sample!("prober", "tests/corpus/prober.red", false),
    sample!("replicator", "tests/corpus/replicator.red", false),
    sample!("scanner", "tests/corpus/scanner.red", false),
    sample!("vampire", "tests/corpus/vampire.red", false),
    sample!("suicide", "warriors/suicide.red", false),
    sample!("and", "docs/snippets/and.red", true),
    sample!("call_ret", "docs/snippets/call_ret.red", true),
    sample!("jez", "docs/snippets/jez.red", true),
    sample!("jgz", "docs/snippets/jgz.red", true),
    sample!("jlz", "docs/snippets/jlz.red", true),
    sample!("jnz", "docs/snippets/jnz.red", true),
    sample!("not", "docs/snippets/not.red", true),
    sample!("or", "docs/snippets/or.red", true),
    sample!("push_pop", "docs/snippets/push_pop.red", true),
    sample!("shl", "docs/snippets/shl.red", true),
    sample!("shr", "docs/snippets/shr.red", true),
    sample!("tst", "docs/snippets/tst.red", true),
    sample!("xor", "docs/snippets/xor.red", true),
];

/// The sample called `name`.
#[must_use]
pub fn find(name: &str) -> Option<&'static Sample> {
    SAMPLES.iter().find(|sample| sample.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, Settings};

    #[test]
    fn every_sample_assembles_in_its_dialect() {
        for sample in SAMPLES {
            let settings = Settings {
                extended: sample.extended,
                core_size: 8000,
                max_cycles: 1000,
                seed: 0,
            };
            if let Err(e) = Session::new(&[sample.source.to_string()], settings) {
                panic!("{}: {e}", sample.name);
            }
        }
    }
}
