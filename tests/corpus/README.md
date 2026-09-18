# Differential test corpus

Classic ICWS'94 warriors used to check CoreWarz against pMARS. Every warrior
here must be legal classic Redcode — no CoreWarz extensions — because the whole
point is that both simulators can run them.

Warriors are published, well-known designs covering the main strategy families:
an imp, a dwarf-style bomber, a scanner, a replicator, a vampire, plus two
warriors added specifically to close opcode-coverage gaps (`creeper.red`,
`prober.red` — see below).

## Opcode / addressing-mode coverage audit

A mutation test proved this matters: inverting `exec_jmz` (making JMZ jump on
non-zero) was **not** caught by the original five-warrior corpus, because none
of imp/dwarf/replicator/scanner/vampire execute JMZ or JMN at all. `creeper.red`
and `prober.red` were added to close that gap and the rest of the classic
opcode set. Audit performed by grepping `tests/corpus/*.red` for each classic
ICWS'94 opcode and addressing mode.

| Opcode | Covered by |
|---|---|
| DAT | dwarf, scanner, vampire, creeper, prober |
| MOV | imp, dwarf, replicator, scanner, vampire, creeper, prober |
| ADD | dwarf, scanner, vampire, creeper, prober |
| SUB | scanner, creeper |
| MUL | prober |
| DIV | prober |
| MOD | prober |
| JMP | dwarf, replicator, scanner, vampire, creeper, prober |
| JMZ | creeper |
| JMN | prober |
| DJN | vampire |
| SPL | replicator, vampire, creeper |
| SEQ | scanner |
| SNE | prober |
| SLT | prober |
| NOP | prober |

| Addressing mode | Covered by |
|---|---|
| `#` immediate | dwarf, scanner, vampire, creeper, prober |
| `$` direct | all eight warriors |
| `*` indirect-A | scanner |
| `@` indirect-B | dwarf, scanner, vampire |
| `{` predec-A | prober |
| `<` predec-B | prober |
| `}` postinc-A | prober |
| `>` postinc-B | creeper |

All sixteen classic opcodes and all eight addressing modes are now exercised
by at least one corpus warrior. `creeper.red` is a JMZ-controlled bomber that
counts a loop counter down with SUB and walks core forward with a
postincrementing (`>`) pointer. `prober.red` is a JMN-based scanner (the
mirror image of `scanner.red`'s SEQ-based test) that also does MUL/DIV/MOD
arithmetic housekeeping, SLT/SNE comparisons, an executed NOP, and bombs core
through three separate pointers exercising `{`, `<`, and `}`. Both were
verified to assemble under `Dialect::Classic`, to be accepted by pMARS, and to
survive long enough to fight real, non-trivial battles against every other
corpus warrior (see the differential test's `compared N battles` output for
the full matrix now run at `cargo test -p corewarz-core --test differential`).

## pMARS build used

`tools/build-pmars.sh`'s original source, `https://github.com/rodrigosetti/pmars`,
no longer exists (`git clone` returns "Repository not found"). It was rebuilt
from `https://github.com/akosela/pmars` instead, which is pMARS **v0.9.3**
(per its own `-h` banner: "pMARS v0.9.3, 8/2/2019, corewar simulator with
ICWS'94 extensions"). This is the ICWS'94-extension pMARS lineage that the
0.9.2 releases also belong to; the flags this harness depends on are
unchanged across that lineage.

Two source bugs in this particular checkout had to be patched locally (in the
gitignored `tools/pmars/src` checkout, not in this repo) to get it to compile
with a modern clang on macOS:

- `src/sim.h` had a broken include guard: `#ifndef SIM_INCLUDED` /
  `#define SIM_INCLUDED` was immediately followed by a stray `#endif`, so the
  guard never actually prevented re-inclusion. Multiple inclusion of
  `sim.h` in `src/disasm.c` then produced a hard "redefinition of `round`"
  error. Fixed by moving the `#endif` to the end of the file, closing the
  guard around the real header body.
- The Makefile's default target and its `curses` target require SVGA/X11 or
  a curses-based core display, neither of which links cleanly against the
  system headers used here (macOS's `curses.h` gives an incomplete `WINDOW`
  type against this old code, and the SVGA libraries do not exist on macOS
  at all). The binary was built with **no display backend defined at all**
  (no `-DGRAPHX`, `-DCURSESGRAPHX`, etc.), which compiles the display-free
  batch simulator only — exactly what headless differential testing needs.
  Build command used:
  `cc -O -DEXT94 -DPERMUTATE -fno-builtin-round -Wno-implicit-function-declaration -Wno-deprecated-non-prototype -c ...`
  followed by linking with no extra libraries.

None of this touches CoreWarz's own source; it is local surgery on the
throwaway, gitignored pMARS checkout under `tools/pmars/src`, needed only to
get an old C codebase through a modern compiler. `tools/build-pmars.sh` was
updated to clone `akosela/pmars` and to build via a plain `cc` invocation
with those flags instead of `make -f makefile.unix` (that makefile does not
exist in this source tree at all).

## Confirmed flag meanings (pMARS v0.9.3, this build)

Run `./tools/pmars/pmars -h` (it prints its usage even though `-h` itself is
reported as "Unknown option") to get:

```
  -r # Rounds to play [1]              -e   Enter debugger
  -s # Size of core [8000]             -b   Brief mode (no source listings)
  -c # Cycles until tie [80000]        -V   Verbose assembly
  -p # Max. processes [8000]           -k   Output in KotH format
  -l # Max. warrior length [100]       -8   Enforce ICWS'88 rules
  -d # Min. warriors distance          -f   Fixed position series
  -F # Fixed position of warrior #2    -o   Sort result output by score
```

So: `-r` rounds, `-s` core size, `-c` cycles, `-p` max processes, `-l` max
warrior length, `-d` minimum separation, `-F` the second warrior's fixed load
position (the first warrior's position is not independently controllable —
it is always placed first, so this harness always treats it as position 0),
`-b` brief/no-source-listing output. These all match what the differential
test harness assumes.

**Deviation from the original plan:** the plan assumed `-b` alone gives one
parseable "wins losses ties" score line per warrior. In this build, `-b`
instead prints one `"<name> by <author> scores <score>"` line per warrior
(a cumulative score across all rounds, not a win count) followed by a single
aggregate line `Results: W L T` (wins/losses/ties for warrior #1, from
warrior #1's perspective, summed over all `-r` rounds). Since the harness
always runs `-r 1`, that `Results:` line unambiguously encodes the single
round's outcome: exactly one of its three counters is 1. The test code in
`crates/corewarz-core/tests/differential.rs` parses that line instead of a
per-warrior score line.

## Provenance

pMARS is built by `tools/build-pmars.sh` from
`https://github.com/akosela/pmars` (v0.9.3). Update this note with the new
source revision if the build script's URL changes again.
