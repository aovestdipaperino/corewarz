# CoreWarz

Core War is a programming game from 1984: two or more assembly-language
programs ("warriors"), written in a small language called Redcode, run
concurrently inside a shared, circular memory ("the core") and try to kill
each other's processes or overwrite each other into oblivion.

CoreWarz is an extended Core War implementation in Rust. It runs classic
ICWS'94 Redcode unmodified, and adds an opt-in extended dialect — bitwise
operations, a three-way compare, zero-testing conditional branches, and
core-resident subroutine calls (`CALL`/`RET`/`PUSH`/`POP`) — on top of the same
virtual machine, sharing its operand-resolution and scheduling code. The
subroutine stack lives in ordinary core memory and is therefore attackable
like everything else, which is the whole design point: see
[`docs/extended-redcode.md`](docs/extended-redcode.md) for the full reference,
and the [design spec](docs/superpowers/specs/2026-09-17-corewarz-design.md)
for the rationale behind each decision.

Tournaments, hills, scoring, and warrior evolution are explicitly out of
scope for this project (see spec section 9); CoreWarz is the VM, assembler,
and visualizer, not a competition platform.

## Quickstart

```bash
cargo run -p corewarz-cli -- watch warriors/imp.red warriors/dwarf.red
```

This opens the live terminal visualizer with the classic sample warriors
`imp` and `dwarf` fighting in an 8000-cell core. Controls: `space`
pause/resume, `s` single-step, `+`/`-` speed, `q` or `Esc` quit.

To run a battle headlessly and just see the result:

```bash
cargo run -p corewarz-cli -- run warriors/imp.red warriors/dwarf.red
```

Add `--extended` to either subcommand to assemble and run warriors written in
the CoreWarz extended dialect (see `docs/snippets/*.red` for small,
individually-runnable examples of every extended opcode):

```bash
cargo run -p corewarz-cli -- run docs/snippets/call_ret.red --extended --cycles 100
```

Run `corewarz run --help` or `corewarz watch --help` for the full set of
flags (core size, cycle limit, process limit, warrior length, minimum
separation, rounds, placement seed).

## Crate layout

| Crate | Purpose |
|---|---|
| `corewarz-core` | The virtual machine: instruction representation, core memory, process scheduler, the single-step execution function, both dialects. No I/O. |
| `corewarz-asm` | Assembler and disassembler for classic and extended Redcode, including the modifier-legality table and the round-trip guarantees. |
| `corewarz-tui` | The live terminal visualizer (`ratatui` + `crossterm`), implemented purely as an `Observer` over a `Mars` it drives. |
| `corewarz-cli` | The `corewarz` binary: `run` (headless battles and reports) and `watch` (the visualizer). |

## Running the tests

```bash
cargo test --workspace
```

This runs unit tests per opcode, assembler round-trip tests (including
rejection tests for illegal opcode/modifier pairs and for extended opcodes
used under the classic dialect), and property tests. It does **not** require
pMARS.

### Differential testing against pMARS

The correctness oracle for the classic dialect is
[pMARS](https://github.com/akosela/pmars) (v0.9.3): a harness assembles a
published classic warrior with both `corewarz-asm` and pMARS, runs both
simulators, and compares results cycle by cycle over the corpus in
`tests/corpus/`.

pMARS is not available via Homebrew, and the mirror originally planned for
this project (`rodrigosetti/pmars`) no longer exists. Build it from the
working source with:

```bash
tools/build-pmars.sh
```

This clones `github.com/akosela/pmars`, applies two small local patches
needed to get pMARS's old C codebase through a modern clang on macOS (a
broken include guard in `sim.h`, and building with no display backend at all
rather than the curses/SVGA targets, neither of which link on macOS), and
builds `tools/pmars/pmars`. The build output is gitignored; run the script
once before running the differential suite. See `tests/corpus/README.md` for
the confirmed pMARS flags and the opcode/addressing-mode coverage audit for
the corpus.

## Documentation

- [`docs/extended-redcode.md`](docs/extended-redcode.md) — reference for
  every CoreWarz extended opcode: syntax, legal modifiers, exact semantics,
  and a runnable example, plus the full subroutine calling convention.
- [`docs/superpowers/specs/2026-09-17-corewarz-design.md`](docs/superpowers/specs/2026-09-17-corewarz-design.md)
  — the design spec: the full instruction set, execution model, observation
  interface, TUI design, error handling, testing strategy, scope, and the
  rationale behind each decision.
- `cargo doc --workspace --no-deps --open` — generated API documentation for
  all four crates.
