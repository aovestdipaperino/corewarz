# CoreWarz — Design Spec

Date: 2026-09-17

## 1. Purpose

CoreWarz is an extended version of A. K. Dewdney's Core War, written in Rust.

Classic Core War is a game in which warrior programs written in Redcode compete
inside a shared circular memory array (the core), executed by a simulator called
MARS. A warrior loses when all of its processes have executed an illegal
instruction.

CoreWarz keeps that game intact and adds a second, extended dialect whose goal is
a **richer machine model**: warriors gain real arithmetic, real conditional
branches, and real subroutines, so that a warrior can be written as a structured
program rather than only as clever self-modifying code.

The project is explicitly *not* an attempt to change the topology of the core, the
rules of conflict, or the win conditions. Those are out of scope (see Section 9).

## 2. Relationship to ICWS'94

CoreWarz supports two dialects sharing one virtual machine:

- **Classic** — full ICWS'94 fidelity. A valid ICWS'94 warrior assembles and runs
  with standard semantics. Extended opcodes are rejected.
- **Extended** — classic plus the additional opcodes in Section 4.

Classic mode exists as a correctness oracle. Because both dialects share operand
resolution, addressing modes, scheduling, and memory arithmetic, proving classic
mode correct against an existing reference implementation (pMARS) proves the
shared machinery underneath the extended dialect correct as well.

The dialect is a field on `Config` (`Dialect::Classic | Dialect::Extended`). It is
enforced at exactly two chokepoints:

1. The assembler refuses extended opcodes when the dialect is Classic.
2. The VM validates loaded warriors on admission.

The interpreter's hot loop contains no dialect branch. Consequences: extended mode
costs classic mode no performance, and the two dialects cannot drift apart through
scattered conditionals.

## 3. Architecture

A Cargo workspace of four crates with a strictly acyclic dependency graph pointing
at `corewarz-core`.

```
corewarz/
├── Cargo.toml              # workspace
├── crates/
│   ├── corewarz-core       # instruction representation + MARS VM
│   ├── corewarz-asm        # assembler / disassembler  (depends: core)
│   ├── corewarz-tui        # visualizer & debugger     (depends: core, asm)
│   └── corewarz-cli        # `corewarz` binary         (depends: core, asm, tui)
└── docs/
```

### corewarz-core

Owns `Instruction`, `Core`, the process queues, `Config`, and the step function.
Knows nothing about text, files, or terminals. Public surface:

- `Mars::new(Config, Vec<Warrior>) -> Mars`
- `Mars::step(&mut self, &mut impl Observer)`
- `Mars::run(&mut self, &mut impl Observer) -> Outcome`
- accessors for core contents, ownership map, process counts, cycle count
- the `Observer` trait (Section 5)

Testable, fuzzable, and benchmarkable with zero I/O. It also carries a
dev-dependency on `corewarz-asm`, used only by its differential test harness
to assemble corpus warriors for comparison against pMARS; this never appears
in the production dependency graph.

### corewarz-asm

Redcode source → loadable warrior, and warrior → disassembly. Handles parsing,
`EQU` / `ORG` / `END`, label resolution, addressing-mode syntax, modifier defaults,
the extended opcodes, and rejection of ill-formed opcode/modifier pairs. Depends on
`corewarz-core` for instruction types; never the reverse.

### corewarz-tui

Renders and drives a `Mars` via the `Observer` trait. Built on `ratatui` +
`crossterm`. Depends on `corewarz-core` for the `Mars` it drives and on
`corewarz-asm` to disassemble cells for display. Cannot corrupt a battle: it
holds a `Mars`, calls `step()`, and reads public state.

### corewarz-cli

Thin. Argument parsing, file loading, dialect selection, headless-vs-TUI dispatch,
result reporting, exit codes.

## 4. The extended instruction set

All arithmetic is unsigned mod `CORESIZE`, as in classic Redcode. Where a negative
value is required, the conventional signed reading applies: a value strictly
greater than `CORESIZE/2` counts as negative.

New opcodes accept **only the modifiers that make sense for them** (decision:
restricted modifiers). The assembler rejects all other opcode/modifier pairs. There
is no opcode whose modifier semantics must be invented.

### 4.1 Arithmetic and bitwise

`AND`, `OR`, `XOR`, `SHL`, `SHR`, `NOT`.

Semantics are identical in shape to classic `ADD`: same operand evaluation, same
modifier rules (`.A .B .AB .BA .F .X`), same write-back. They reuse the existing
binary-operation machinery and inherit its tests.

- `SHL` / `SHR` shift the destination field by the source field's value.
  A shift count greater than or equal to the word width produces zero.
- `NOT` is unary in effect; it takes the standard two operands with the source
  ignored for value purposes, and supports `.A .B .AB .BA .F .X`.

### 4.2 Three-way compare

`TST a, b` — compare the fields selected by the modifier and write into `b`'s
destination:

- `0`   if equal
- `1`   if a > b
- `-1`  (i.e. `CORESIZE-1`) if a < b

Modifiers: `.A .B .AB .BA .F .X`. This is the value-producing comparison the
extended dialect needs; classic Redcode can only skip.

### 4.3 Conditional branches

`JEZ target, test`, `JNZ target, test`, `JLZ target, test`, `JGZ target, test`.

Evaluate `test`'s field(s) under the modifier and jump to `target` if the condition
against zero holds; otherwise fall through to the next instruction.

- `JEZ` — jump if zero
- `JNZ` — jump if non-zero
- `JLZ` — jump if negative (value > `CORESIZE/2`)
- `JGZ` — jump if positive (value non-zero and <= `CORESIZE/2`)

Modifiers: `.A .B .F .X` (which field(s) of `test` to examine). With `.F` / `.X`,
the condition must hold for both compared fields.

Testing against zero rather than comparing two operands keeps these instructions to
two operands. `TST` makes this sufficient: any comparison-branch is expressible as
`TST` followed by a conditional branch.

Computed and indirect jumps require no new opcode — classic `JMP` with indirect
addressing modes already provides them.

### 4.4 Subroutines

The stack lives **in the core** and is therefore fully attackable. No process has
private, un-attackable state; this preserves Core War's central invariant that all
memory is contested.

A *stack pointer cell* is an ordinary core cell whose B-field holds the current
stack pointer (an address).

- `CALL target, sp` — push the return address (the address of the instruction
  following the `CALL`) at the stack pointer, advance the pointer, jump to `target`.
- `RET sp` — retreat the pointer, read the return address there, jump to it.
- `PUSH val, sp` — push `val` at the stack pointer and advance the pointer.
- `POP dst, sp` — retreat the pointer, read the value there, write it to `dst`.

`sp` is resolved through normal addressing modes to locate the stack-pointer cell.
The stack grows upward (increasing addresses), wrapping like all core arithmetic.
Pushed values occupy the B-field of the target cell. These four opcodes take no
modifier; the assembler rejects any modifier on them.

Strategic consequences, which are intended:

- Bombing a stack-pointer cell redirects the victim's next `RET` into garbage.
- The stack is core memory: it can be read, corrupted, or overwritten with a
  chosen return address.
- Subroutines are therefore both powerful and a liability.

### 4.5 Death

Unchanged from classic. Executing `DAT` kills the process. A warrior with no
remaining processes has lost. There is no new failure mode: a malformed `RET`
simply jumps somewhere, usually fatally.

## 5. Execution model

### Core representation

`Vec<Instruction>` of fixed size (default 8000). `Instruction` is a small `Copy`
struct: opcode, modifier, two addressing modes, two field values. Packed for cache
friendliness.

All modular address arithmetic goes through a single `fold` helper. There is
exactly one place in the codebase where wrapping happens; scattered `%` operators
are a classic source of MARS bugs.

### Scheduling

Standard MARS model. Each warrior owns a FIFO process queue; warriors take turns.

One cycle: each live warrior dequeues its next process, executes one instruction,
and enqueues the resulting process(es) at the back of its queue.

- `SPL` enqueues two processes
- `DAT` enqueues none (the process dies)
- everything else enqueues one

Queues are ring buffers of fixed capacity (default 8000). When the process limit is
reached, `SPL` behaves as a no-op, per the standard.

A battle ends when only one warrior has live processes (that warrior wins), when no
warrior does (a draw), or when the cycle limit is reached (a tie).

### A single step

1. Fetch the instruction at the process's program counter.
2. Decode both operands under their addressing modes, applying pre-decrement and
   post-increment side effects on core in the standard order.
3. Dispatch on opcode.

Operand resolution is shared by every opcode, classic and extended. This is why the
extended opcodes are cheap to add: `AND` differs from `ADD` by one line.

### Default configuration

| Parameter        | Default |
|------------------|---------|
| core size        | 8000    |
| cycles before tie| 80000   |
| max processes    | 8000    |
| max warrior length | 100   |
| minimum separation | 100   |

All are settable via `Config` and CLI flags.

## 6. Observation

```rust
pub trait Observer {
    fn on_write(&mut self, addr: usize, warrior: WarriorId) {}
    fn on_exec(&mut self, addr: usize, warrior: WarriorId) {}
    fn on_death(&mut self, warrior: WarriorId, addr: usize) {}
}
```

A no-op implementation (`NullObserver`) is provided; the compiler erases it, so
headless runs pay nothing. The TUI implements it to build its display. The
differential test harness implements it to produce an execution trace for
comparison against pMARS.

This is the single interface that keeps the VM ignorant of everything above it.

### Ownership map

A parallel `Vec<Option<WarriorId>>`, updated on every write, recording which
warrior last wrote each cell. It has **no effect on the rules** — it exists solely
so the visualizer can color the battlefield.

## 7. TUI visualizer

Built on `ratatui` + `crossterm`. Runs as an `Observer` over a `Mars` it drives.

**Core view** (centerpiece): the core rendered as a dense grid of colored blocks,
one character per cell, colored by owning warrior, with brightness indicating
recent activity so writes flash and fade. Execution positions are marked distinctly
from mere ownership. At typical terminal sizes 8000 cells fit; for larger cores each
character aggregates a run of cells.

**Source pane**: disassembly around each warrior's current instruction pointer,
executing line highlighted. This is what makes the extended opcodes debuggable —
watching a `CALL`/`RET` pair land while the stack cells change in the core view is
the fastest check on the Section 4 semantics.

**Status pane**: cycle count, per-warrior process counts, last instruction executed
per warrior.

**Controls**: `space` pause/resume, `s` single-step, `+`/`-` speed, `q` or
`Esc` quit.

## 8. Error handling and testing

### Error handling

Errors are a **compile-time** concern only.

The assembler is the only component that reports errors to a human. `AsmError`
carries a source position and covers: unknown opcode, illegal opcode/modifier pair,
extended opcode used under the Classic dialect, unresolved label, warrior exceeding
the instruction limit, malformed operand.

The VM has **no runtime errors**. A loaded warrior is well-formed by construction,
and every in-battle mishap is a game outcome (a dead process), not a failure.

The CLI turns assembler errors into diagnostics and battle outcomes into reports
and exit codes.

### Testing, in build order

1. **Unit tests per opcode.** For each instruction, a table of cases: initial core,
   one step, expected core and queue state. Written before each opcode's
   implementation. This is where extended semantics are pinned down precisely.

2. **Assembler round-trip tests.** Source → warrior → disassembly → source, plus
   explicit rejection tests for every illegal opcode/modifier combination and for
   every extended opcode under the Classic dialect.

3. **Differential testing against pMARS.** The correctness oracle. A harness
   assembles a classic warrior with both `corewarz-asm` and pMARS, runs both
   simulators, and compares results cycle by cycle over a corpus of published
   warriors from the standard hills. Any divergence in classic mode is a bug in
   CoreWarz.

4. **Property tests.** Randomized warriors run in both engines, plus invariants
   that must always hold: address arithmetic never escapes the core, process counts
   never exceed the limit, every battle terminates by the cycle limit.

The extended opcodes have no external oracle. They rely on the unit tables and on
the TUI for visual verification — which is why the visualizer is in v1 scope.

## 9. Scope

### In scope for v1

- `corewarz-core`: VM with both dialects
- `corewarz-asm`: assembler and disassembler for both dialects
- `corewarz-tui`: live visualizer with pause/step debugging
- `corewarz-cli`: `corewarz run` and `corewarz watch`
- Differential test harness against pMARS

### Explicitly out of scope

- Tournament runner, scoring tables, persistent hills — a layer over a finished
  VM; a better second project once the extended ISA has settled.
- Changes to core topology (2-D / segmented cores).
- Changes to conflict rules (cell ownership as a game mechanic, capture, process
  economy, fog of war).
- Environmental agency (resource nodes, hazards, sensing instructions).
- Private per-process state (registers or stacks outside the core). Rejected by
  design in Section 4.4.
- Process-management opcodes beyond `SPL` (self-kill, process-count query,
  priorities). These are conflict-rule changes in machine-model clothing.
- Warrior evolution / genetic programming harnesses.

## 10. Key decisions and rationale

| Decision | Choice | Why |
|---|---|---|
| Relationship to ICWS'94 | Superset with a classic-mode switch | Classic mode is a ruthless correctness oracle; without it an extended VM is hard to trust |
| Extension dimension | Richer machine model | Warriors become structured programs rather than only self-modifying loops |
| Private state | None; everything in the core | Preserves the invariant that all memory is contested; keeps classic mode a genuine subset |
| Capabilities | Subroutines, bitwise/compare arithmetic, conditional branches | The coherent core of "warriors as real programs"; each independently testable |
| Modifiers on new opcodes | Restricted subset, assembler-enforced | Avoids a combinatorial test burden over pairs no warrior would write, without creating two dialects in one language |
| v1 scope | Library + CLI + TUI | The visualizer is the primary verification tool for semantics that have no external oracle |
