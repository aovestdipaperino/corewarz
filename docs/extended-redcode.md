# The extended Redcode dialect

CoreWarz runs classic '94 Redcode plus a small set of extended opcodes,
available under the `--extended` dialect flag (`corewarz run --extended ...`,
`corewarz watch --extended ...`). This document is the reference for those
extensions: syntax, legal modifiers, exact semantics, and a runnable example
for each opcode. It is written against the actual implementation in
`crates/corewarz-core/src/exec/{ext_bitwise,ext_compare,ext_branch,ext_stack}.rs`
and the modifier table in `crates/corewarz-asm/src/modifier.rs`; where this
document and the design spec disagree, this document follows the code, and
any such divergence is called out below.

Every example in this file also exists as a standalone, runnable file under
[`docs/snippets/`](snippets/). They are verified as part of this repository's
checks:

```bash
for f in docs/snippets/*.red; do
    cargo run -q -p corewarz-cli -- run "$f" --extended --cycles 100 || echo "FAILED: $f"
done
```

Background on classic Redcode addressing modes, fields, and modifiers is
assumed; see the [design spec](superpowers/specs/2026-09-17-corewarz-design.md)
section 4 for the design rationale behind each extension.

## Conventions used below

All arithmetic is unsigned modulo the core size (`CORESIZE`, default 8000), as
in classic Redcode. Where a signed reading is needed:

- a value `v` is **negative** iff `v > CORESIZE / 2`
- a value `v` is **positive** iff `v != 0 && v <= CORESIZE / 2`
- `-1` is represented as `CORESIZE - 1`

Every extended opcode shares the operand-resolution and modifier machinery
used by classic opcodes: `.A`/`.B` select one field on both sides, `.AB`/`.BA`
cross a single field, `.F` operates fieldwise (A with A, B with B), and `.X`
crosses both fields. The assembler rejects any opcode/modifier pair that is
not listed below.

## 4.1 Arithmetic and bitwise: `AND`, `OR`, `XOR`, `SHL`, `SHR`, `NOT`

**Syntax:** `OPCODE.MOD a-operand, b-operand`

**Legal modifiers:** `.A .B .AB .BA .F .X`

These behave exactly like classic `ADD` in shape: the same operand evaluation,
the same modifier rules, and the same read-modify-write into the B-operand's
cell. Only the arithmetic differs, per selected field pair:

- `AND` — bitwise AND of the source field into the destination field
- `OR` — bitwise OR
- `XOR` — bitwise XOR
- `SHL` — destination field shifted left by the source field's value
- `SHR` — destination field shifted right by the source field's value
- `NOT` — destination field complemented; the source field's *value* is
  ignored (the source operand is still evaluated and its addressing side
  effects, e.g. pre-decrement, still happen — only the bit pattern is unused)

`SHL`/`SHR` operate on a 32-bit word: a shift count of 32 or more produces 0.
The result of any of these opcodes is folded back into the core size exactly
like `ADD`'s result is (e.g. a left shift that overflows the core size wraps).

As with `ADD`, the default modifier (when none is written) is `.AB` if the
A-operand is immediate, `.B` if only the B-operand is immediate, otherwise
`.F`. Writing a modifier explicitly is recommended for clarity, and is
required whenever you want a mode other than the default.

**Example** (`docs/snippets/and.red`): AND 12 (`1100b`) into a cell whose
B-field holds 10 (`1010b`), leaving 8 (`1000b`).

```redcode
;name and-demo
;strategy AND.AB masks the immediate value 12 (1100b) against target's
;strategy B-field, 10 (1010b), leaving 8 (1000b) there.
        AND.AB #12, target
        JMP     $0
target  DAT.F   #0, #10
```

`.AB` reads the immediate's A-field (12) as the source and writes into
`target`'s B-field, since with `#12` there is nothing else the immediate could
mean — it is not an address, only a literal.

`OR`, `XOR`, `SHL`, `SHR`, and `NOT` follow the identical shape; see
`docs/snippets/or.red`, `xor.red`, `shl.red`, `shr.red`, and `not.red`.

## 4.2 Three-way compare: `TST`

**Syntax:** `TST.MOD a-operand, b-operand`

**Legal modifiers:** `.A .B .AB .BA .F .X`

Compares the selected field(s) and writes the result into the corresponding
field(s) of the B-operand's cell:

- `0` if the fields are equal
- `1` if the source field is greater than the destination field
- `CORESIZE - 1` (i.e. `-1`) if the source field is smaller

This is the value-producing comparison the dialect needs, since classic
Redcode can only skip on a comparison (`SEQ`/`SNE`). `TST` followed by one of
the conditional branches below expresses any comparison-branch.

**Example** (`docs/snippets/tst.red`): compare immediate 3 against a cell
whose B-field is 9. Since 3 < 9, the destination's B-field becomes
`CORESIZE - 1`.

```redcode
;name tst-demo
;strategy TST.AB compares the immediate 3 against target's B-field, 9. 3 < 9,
;strategy so target's B-field becomes -1 (core-size minus one).
        TST.AB #3, target
        JMP     $0
target  DAT.F   #0, #9
```

## 4.3 Conditional branches: `JEZ`, `JNZ`, `JLZ`, `JGZ`

**Syntax:** `OPCODE.MOD target, test`

**Legal modifiers:** `.A .B .F .X` (which field(s) of `test` to examine)

Each evaluates `test`'s selected field(s) and jumps to `target` if the
condition holds; otherwise execution falls through to the next instruction.
Under `.F` or `.X`, **both** compared fields must satisfy the condition — this
is a deliberate departure from classic `JMN`, which jumps if *either* field is
non-zero.

- `JEZ` — jump if the field(s) are zero
- `JNZ` — jump if the field(s) are non-zero
- `JLZ` — jump if the field(s) are negative (`> CORESIZE / 2`)
- `JGZ` — jump if the field(s) are positive (non-zero and `<= CORESIZE / 2`)

Since `TST` can produce any three-way result into a cell, `TST` followed by
one of these four branches is sufficient to express any comparison-branch —
which is why these opcodes take only a target and a value to test, not two
values to compare directly. Computed and indirect jumps need no new opcode:
classic `JMP` with indirect addressing modes already covers them.

**Examples** (`docs/snippets/jez.red`, `jnz.red`, `jlz.red`, `jgz.red`):

```redcode
;name jez-demo
;strategy test's B-field is zero, so JEZ jumps to target instead of falling
;strategy through to the DAT that would otherwise kill the process.
        JEZ.B  target, test
        DAT.F  #0, #0
test    DAT.F  #0, #0
target  JMP    $0
```

```redcode
;name jlz-demo
;strategy test's B-field, 7999, reads as negative (greater than core-size/2),
;strategy so JLZ jumps to target.
        JLZ.B  target, test
        DAT.F  #0, #0
test    DAT.F  #0, #7999
target  JMP    $0
```

## 4.4 Subroutines: `CALL`, `RET`, `PUSH`, `POP`

**Syntax:**

```
CALL target, sp
RET  sp
PUSH val, sp
POP  dst, sp
```

**Legal modifiers:** none. The assembler rejects any modifier on these four
opcodes (write them bare, or with no suffix at all).

### The calling convention

- A **stack-pointer cell** is an ordinary core cell whose **B-field** holds
  the current stack pointer — an address into the core. There is nothing
  special about this cell other than convention: any warrior can designate
  any cell as its stack pointer, and the opcodes below only ever read and
  write its B-field.
- `CALL`, `PUSH`, and `POP` take the stack-pointer cell from their **second**
  operand (`sp`, the B-operand). `RET` takes it from its **first and only**
  operand (`sp`, the A-operand).
- The stack **grows upward**: pushing advances the pointer to a higher
  address, popping retreats it to a lower one. All arithmetic on the pointer
  wraps modulo the core size like everything else in the core.
- Pushed values occupy the **B-field** of the cell at the stack pointer.
  `PUSH val, sp` pushes the **value** found in the B-field of the cell the
  `val` operand resolves to — it pushes a value, not an address. (An earlier
  draft of the design spec described `PUSH` as pushing an address; the
  implementation and its tests push a value, and this document follows the
  implementation.)
- `CALL target, sp` pushes the address of the instruction immediately
  following the `CALL` (the return address), advances the pointer, then jumps
  to `target`.
- `RET sp` retreats the pointer first, then jumps to whatever address is
  stored at the cell the (now retreated) pointer designates.
- `POP dst, sp` retreats the pointer, reads the value stored there, and
  writes it into the B-field of the cell `dst` resolves to.

### The stack lives in the core

This is the point of the design, not an incidental detail: the stack is
ordinary, attackable core memory. No process has private state. Consequences
that are intended, not bugs:

- Bombing a stack-pointer cell corrupts a live pointer, redirecting the
  victim's next `RET` into whatever garbage now occupies the intended
  address.
- The stack region itself can be read, overwritten, or seeded with a chosen
  return address by an opponent, turning a victim's own `RET` into a jump
  anywhere in the core.
- A malformed `RET` is not a runtime error — it is simply a jump, usually
  fatal to the process that takes it, exactly like every other misdirected
  jump in Core War.

**Example** (`docs/snippets/call_ret.red`):

```redcode
;name call-ret-demo
;strategy sp is the stack-pointer cell: its B-field holds the current stack
;strategy pointer, an address into the core. CALL pushes the return address
;strategy (the instruction after the CALL) at that pointer, advances the
;strategy pointer, and jumps to sub. RET there retreats the pointer and jumps
;strategy back, landing on the JMP just after the CALL.
        CALL   sub, sp
        JMP    $0
sub     RET    sp
sp      DAT.F  #0, #stack
stack   DAT.F  #0, #0
```

**Example** (`docs/snippets/push_pop.red`):

```redcode
;name push-pop-demo
;strategy PUSH stores the B-field value found through its first operand (here
;strategy 1234, not an address) at the stack pointer and advances it. POP
;strategy retreats the pointer and writes the popped value into dst's B-field.
        PUSH   val, sp
        POP    dst, sp
        JMP    $0
val     DAT.F  #0, #1234
dst     DAT.F  #0, #0
sp      DAT.F  #0, #stack
stack   DAT.F  #0, #0
```

## 4.5 Death

Unchanged from classic Redcode. Executing `DAT` kills the process that
executed it. A warrior with no remaining live processes has lost. The
extended opcodes introduce no new failure mode: a corrupted `RET`, for
instance, simply jumps somewhere — usually fatally — rather than raising an
error. The virtual machine has no runtime errors at all; every in-battle
mishap is a game outcome, not a failure. (Errors are strictly a compile-time,
assembler-level concern — see the design spec section 8.)

## Not in scope

To avoid any ambiguity: tournaments, hills, scoring, and warrior evolution
are **not** part of CoreWarz and are not documented here because they do not
exist. Nor do private per-process registers/stacks, alternate core
topologies, or environmental/sensing opcodes. See the design spec section 9
for the full, deliberate scope boundary.
