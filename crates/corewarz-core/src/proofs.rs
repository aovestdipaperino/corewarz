//! Kani proof harnesses for the classic ICWS'94 VM.
//!
//! Compiled only under `cargo kani`. Each harness checks the VM against a
//! small, deliberately independent reference model of the ICWS'94 draft
//! (`emi94.c`), written below without reusing `field_pairs`, `resolve`, or
//! any `exec_*` function. The core is shrunk to [`N`] cells so that every
//! cell, field, and mode can be left fully symbolic and the proofs still
//! finish in seconds; nothing in the semantics depends on the core size
//! beyond `fold`, which `core.rs` proves separately for every size.
//!
//! Run with `cargo kani -p corewarz-core`.

use crate::config::Config;
use crate::core::Core;
use crate::exec::classic_data::StepResult;
use crate::exec::dispatch;
use crate::exec::operand::resolve;
use crate::instruction::{AddrMode, Instruction, Modifier, Opcode};
use crate::queue::ProcessQueue;
use crate::warrior::WarriorId;

/// Core size used by the step harnesses.
const N: u32 = 8;
const W: WarriorId = WarriorId(0);

// ---------------------------------------------------------------------------
// Symbolic values
// ---------------------------------------------------------------------------

fn any_mode() -> AddrMode {
    match kani::any::<u8>() % 8 {
        0 => AddrMode::Immediate,
        1 => AddrMode::Direct,
        2 => AddrMode::IndirectA,
        3 => AddrMode::IndirectB,
        4 => AddrMode::PredecA,
        5 => AddrMode::PredecB,
        6 => AddrMode::PostincA,
        _ => AddrMode::PostincB,
    }
}

fn any_modifier() -> Modifier {
    match kani::any::<u8>() % 7 {
        0 => Modifier::A,
        1 => Modifier::B,
        2 => Modifier::AB,
        3 => Modifier::BA,
        4 => Modifier::F,
        5 => Modifier::X,
        _ => Modifier::I,
    }
}

fn any_classic_opcode() -> Opcode {
    match kani::any::<u8>() % 16 {
        0 => Opcode::Dat,
        1 => Opcode::Mov,
        2 => Opcode::Add,
        3 => Opcode::Sub,
        4 => Opcode::Mul,
        5 => Opcode::Div,
        6 => Opcode::Mod,
        7 => Opcode::Jmp,
        8 => Opcode::Jmz,
        9 => Opcode::Jmn,
        10 => Opcode::Djn,
        11 => Opcode::Spl,
        12 => Opcode::Seq,
        13 => Opcode::Sne,
        14 => Opcode::Slt,
        _ => Opcode::Nop,
    }
}

/// A field value already folded into `0..N`, as every field in core is.
fn any_field() -> u32 {
    let v: u32 = kani::any();
    kani::assume(v < N);
    v
}

fn any_instruction(opcode: Opcode) -> Instruction {
    Instruction {
        opcode,
        modifier: any_modifier(),
        a_mode: any_mode(),
        a: any_field(),
        b_mode: any_mode(),
        b: any_field(),
    }
}

/// A fully symbolic core: every cell holds an arbitrary classic instruction.
fn any_memory() -> [Instruction; N as usize] {
    [(); N as usize].map(|()| any_instruction(any_classic_opcode()))
}

fn core_from(mem: &[Instruction; N as usize]) -> Core {
    let mut core = Core::new(&Config {
        core_size: N,
        ..Config::default()
    });
    for (addr, cell) in mem.iter().enumerate() {
        core.set(addr as u32, *cell, W);
    }
    let _ = core.take_writes();
    core
}

fn assert_core_matches(core: &Core, mem: &[Instruction; N as usize]) {
    for (addr, cell) in mem.iter().enumerate() {
        assert_eq!(core.get(addr as u32), *cell);
    }
}

// ---------------------------------------------------------------------------
// Reference model: the ICWS'94 draft, transcribed from emi94.c
// ---------------------------------------------------------------------------

fn rfold(v: i64) -> u32 {
    v.rem_euclid(i64::from(N)) as u32
}

/// Result of operand evaluation in the reference model.
struct RefOperands {
    a_ptr: u32,
    b_ptr: u32,
    ira: Instruction,
    irb: Instruction,
}

/// Evaluates one operand: pre-decrement, snapshot, post-increment, in that
/// order, all before the next operand is touched.
fn ref_operand(
    mem: &mut [Instruction; N as usize],
    pc: u32,
    mode: AddrMode,
    value: u32,
) -> (u32, Instruction) {
    if mode == AddrMode::Immediate {
        return (pc, mem[pc as usize]);
    }
    let p = rfold(i64::from(pc) + i64::from(value));
    let pi = p as usize;
    let off = match mode {
        AddrMode::Direct => 0,
        AddrMode::IndirectA | AddrMode::PostincA => mem[pi].a,
        AddrMode::IndirectB | AddrMode::PostincB => mem[pi].b,
        AddrMode::PredecA => {
            mem[pi].a = rfold(i64::from(mem[pi].a) - 1);
            mem[pi].a
        }
        AddrMode::PredecB => {
            mem[pi].b = rfold(i64::from(mem[pi].b) - 1);
            mem[pi].b
        }
        AddrMode::Immediate => unreachable!(),
    };
    let target = rfold(i64::from(p) + i64::from(off));
    let snapshot = mem[target as usize];
    match mode {
        AddrMode::PostincA => mem[pi].a = rfold(i64::from(mem[pi].a) + 1),
        AddrMode::PostincB => mem[pi].b = rfold(i64::from(mem[pi].b) + 1),
        _ => {}
    }
    (target, snapshot)
}

fn ref_resolve(mem: &mut [Instruction; N as usize], pc: u32) -> RefOperands {
    let ir = mem[pc as usize];
    let (a_ptr, ira) = ref_operand(mem, pc, ir.a_mode, ir.a);
    let (b_ptr, irb) = ref_operand(mem, pc, ir.b_mode, ir.b);
    RefOperands {
        a_ptr,
        b_ptr,
        ira,
        irb,
    }
}

/// The program counters a step queues, in order. Empty means the process
/// died.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Queued {
    len: u8,
    pcs: [u32; 2],
}

impl Queued {
    const DIE: Self = Self {
        len: 0,
        pcs: [0, 0],
    };
    const fn one(pc: u32) -> Self {
        Self {
            len: 1,
            pcs: [pc, 0],
        }
    }
}

fn queued_from(pc: u32, r: StepResult) -> Queued {
    match r {
        StepResult::Continue => Queued::one(rfold(i64::from(pc) + 1)),
        StepResult::Skip => Queued::one(rfold(i64::from(pc) + 2)),
        StepResult::Jump(t) => Queued::one(t),
        StepResult::Split(t) => Queued {
            len: 2,
            pcs: [rfold(i64::from(pc) + 1), t],
        },
        StepResult::Die => Queued::DIE,
    }
}

#[derive(Clone, Copy)]
enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

/// One arithmetic field update. `None` means a zero divisor: the field is
/// left alone and the process will die.
fn arith(op: Arith, dst: u32, src: u32) -> Option<u32> {
    let (d, s) = (i64::from(dst), i64::from(src));
    match op {
        Arith::Add => Some(rfold(d + s)),
        Arith::Sub => Some(rfold(d - s)),
        Arith::Mul => Some(rfold(d * s)),
        Arith::Div => (s != 0).then(|| rfold(d / s)),
        Arith::Mod => (s != 0).then(|| rfold(d % s)),
    }
}

/// Executes one classic instruction at `pc` against `mem`, ICWS'94 style.
fn ref_step(mem: &mut [Instruction; N as usize], pc: u32) -> Queued {
    let ir = mem[pc as usize];
    let o = ref_resolve(mem, pc);
    let next = rfold(i64::from(pc) + 1);
    let skip = rfold(i64::from(pc) + 2);
    let b = o.b_ptr as usize;
    let (ia, ib) = (o.ira, o.irb);
    let jump_if = |c: bool| {
        if c {
            Queued::one(o.a_ptr)
        } else {
            Queued::one(next)
        }
    };
    let skip_if = |c: bool| {
        if c {
            Queued::one(skip)
        } else {
            Queued::one(next)
        }
    };

    match ir.opcode {
        Opcode::Dat => Queued::DIE,
        Opcode::Nop => Queued::one(next),
        Opcode::Mov => {
            match ir.modifier {
                Modifier::A => mem[b].a = ia.a,
                Modifier::B => mem[b].b = ia.b,
                Modifier::AB => mem[b].b = ia.a,
                Modifier::BA => mem[b].a = ia.b,
                Modifier::F => {
                    mem[b].a = ia.a;
                    mem[b].b = ia.b;
                }
                Modifier::X => {
                    mem[b].b = ia.a;
                    mem[b].a = ia.b;
                }
                Modifier::I => mem[b] = ia,
            }
            Queued::one(next)
        }
        Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Mod => {
            let op = match ir.opcode {
                Opcode::Add => Arith::Add,
                Opcode::Sub => Arith::Sub,
                Opcode::Mul => Arith::Mul,
                Opcode::Div => Arith::Div,
                _ => Arith::Mod,
            };
            // (destination field is A, source value, destination value)
            let mut alive = true;
            let mut write =
                |to_a: bool, src: u32, dst: u32, mem: &mut [Instruction; N as usize]| match arith(
                    op, dst, src,
                ) {
                    Some(v) if to_a => mem[b].a = v,
                    Some(v) => mem[b].b = v,
                    None => alive = false,
                };
            match ir.modifier {
                Modifier::A => write(true, ia.a, ib.a, mem),
                Modifier::B => write(false, ia.b, ib.b, mem),
                Modifier::AB => write(false, ia.a, ib.b, mem),
                Modifier::BA => write(true, ia.b, ib.a, mem),
                Modifier::F | Modifier::I => {
                    write(true, ia.a, ib.a, mem);
                    write(false, ia.b, ib.b, mem);
                }
                Modifier::X => {
                    write(false, ia.a, ib.b, mem);
                    write(true, ia.b, ib.a, mem);
                }
            }
            if alive {
                Queued::one(next)
            } else {
                Queued::DIE
            }
        }
        Opcode::Jmp => Queued::one(o.a_ptr),
        Opcode::Jmz => jump_if(match ir.modifier {
            Modifier::A | Modifier::BA => ib.a == 0,
            Modifier::B | Modifier::AB => ib.b == 0,
            _ => ib.a == 0 && ib.b == 0,
        }),
        Opcode::Jmn => jump_if(match ir.modifier {
            Modifier::A | Modifier::BA => ib.a != 0,
            Modifier::B | Modifier::AB => ib.b != 0,
            _ => ib.a != 0 || ib.b != 0,
        }),
        Opcode::Djn => {
            // Decrement in core, then test the decremented IRB copy.
            let dec = |v: u32| rfold(i64::from(v) - 1);
            let (touch_a, touch_b) = match ir.modifier {
                Modifier::A | Modifier::BA => (true, false),
                Modifier::B | Modifier::AB => (false, true),
                _ => (true, true),
            };
            let mut non_zero = false;
            if touch_a {
                mem[b].a = dec(mem[b].a);
                non_zero |= dec(ib.a) != 0;
            }
            if touch_b {
                mem[b].b = dec(mem[b].b);
                non_zero |= dec(ib.b) != 0;
            }
            jump_if(non_zero)
        }
        Opcode::Spl => Queued {
            len: 2,
            pcs: [next, o.a_ptr],
        },
        Opcode::Seq | Opcode::Sne => {
            let equal = match ir.modifier {
                Modifier::A => ia.a == ib.a,
                Modifier::B => ia.b == ib.b,
                Modifier::AB => ia.a == ib.b,
                Modifier::BA => ia.b == ib.a,
                Modifier::F => ia.a == ib.a && ia.b == ib.b,
                Modifier::X => ia.a == ib.b && ia.b == ib.a,
                Modifier::I => ia == ib,
            };
            skip_if(if ir.opcode == Opcode::Seq {
                equal
            } else {
                !equal
            })
        }
        Opcode::Slt => skip_if(match ir.modifier {
            Modifier::A => ia.a < ib.a,
            Modifier::B => ib.b > ia.b,
            Modifier::AB => ia.a < ib.b,
            Modifier::BA => ia.b < ib.a,
            Modifier::F | Modifier::I => ia.a < ib.a && ia.b < ib.b,
            Modifier::X => ia.a < ib.b && ia.b < ib.a,
        }),
        _ => unreachable!("extended opcodes are not generated"),
    }
}

// ---------------------------------------------------------------------------
// Addressing modes
// ---------------------------------------------------------------------------

/// For every pair of addressing modes, every field value, and every core
/// content, `resolve` designates the same cells, takes the same snapshots,
/// and leaves the core in the same state as ICWS'94 operand evaluation, and
/// both pointers stay in bounds.
#[kani::proof]
#[kani::unwind(10)]
fn resolve_matches_icws94() {
    let mut mem = any_memory();
    let pc = any_field();
    let mut core = core_from(&mem);

    let ops = resolve(&mut core, pc, mem[pc as usize], W);
    let expected = ref_resolve(&mut mem, pc);

    assert!(ops.a_ptr < N && ops.b_ptr < N);
    assert_eq!(ops.a_ptr, expected.a_ptr);
    assert_eq!(ops.b_ptr, expected.b_ptr);
    assert_eq!(ops.ira, expected.ira);
    assert_eq!(ops.irb, expected.irb);
    assert_core_matches(&core, &mem);
}

// ---------------------------------------------------------------------------
// One step of each classic opcode
// ---------------------------------------------------------------------------

/// Executes the instruction at a symbolic `pc` (with the given opcode and a
/// symbolic modifier, modes, and fields) on a fully symbolic core, through
/// `dispatch`, and checks it against `ref_step`: same queued program
/// counters, same core afterwards, and every field still folded.
fn check_step(opcode: Opcode) {
    let mut mem = any_memory();
    let pc = any_field();
    mem[pc as usize] = any_instruction(opcode);
    let mut core = core_from(&mem);

    let got = queued_from(pc, dispatch(&mut core, pc, W));
    let expected = ref_step(&mut mem, pc);

    assert_eq!(got, expected);
    for i in 0..usize::from(got.len) {
        assert!(got.pcs[i] < N);
    }
    for addr in 0..N {
        let cell = core.get(addr);
        assert!(cell.a < N && cell.b < N, "fields stay folded");
    }
    assert_core_matches(&core, &mem);
}

macro_rules! step_proofs {
    ($($name:ident => $op:expr),* $(,)?) => {$(
        #[kani::proof]
        #[kani::unwind(10)]
        fn $name() {
            check_step($op);
        }
    )*};
}

step_proofs! {
    step_dat => Opcode::Dat,
    step_nop => Opcode::Nop,
    step_mov => Opcode::Mov,
    step_add => Opcode::Add,
    step_sub => Opcode::Sub,
    step_mul => Opcode::Mul,
    step_div => Opcode::Div,
    step_mod => Opcode::Mod,
    step_jmp => Opcode::Jmp,
    step_jmz => Opcode::Jmz,
    step_jmn => Opcode::Jmn,
    step_djn => Opcode::Djn,
    step_spl => Opcode::Spl,
    step_seq => Opcode::Seq,
    step_sne => Opcode::Sne,
    step_slt => Opcode::Slt,
}

// ---------------------------------------------------------------------------
// Process queues
// ---------------------------------------------------------------------------

/// A process queue is a bounded FIFO: it never exceeds its capacity, a push
/// to a full queue is refused without effect, and pops return pushes in
/// order.
fn check_queue(capacity: u32) {
    let mut q = ProcessQueue::new(capacity);
    // Shadow FIFO of at most 3 entries.
    let mut shadow = [0u32; 3];
    let (mut head, mut tail) = (0usize, 0usize);
    for _ in 0..3 {
        if kani::any() {
            let pc = any_field();
            let before = q.len();
            let pushed = q.push(pc);
            assert_eq!(pushed, before < capacity as usize);
            if pushed {
                shadow[tail] = pc;
                tail += 1;
            } else {
                assert_eq!(q.len(), before);
            }
        } else {
            let popped = q.pop();
            if head < tail {
                assert_eq!(popped, Some(shadow[head]));
                head += 1;
            } else {
                assert_eq!(popped, None);
            }
        }
        assert!(q.len() <= capacity as usize);
        assert_eq!(q.len(), tail - head);
        assert_eq!(q.is_empty(), head == tail);
    }
}

// CBMC ran for over 45 minutes without finishing on a symbolically sized
// `VecDeque`, and on a five-operation sequence against a fixed one, so the
// capacities are fixed and the sequences three operations long. Capacity 0 refuses every push; capacity 2 exercises
// both an accepted push and a refused one. The scheduler built on top of the
// queue (`Mars::step`) proved intractable for CBMC even over two cycles; its
// process limit is covered by the unit tests in `mars.rs` and the property
// tests in `tests/properties.rs`.

#[kani::proof]
#[kani::unwind(7)]
fn process_queue_is_a_bounded_fifo_cap_0() {
    check_queue(0);
}

#[kani::proof]
#[kani::unwind(7)]
fn process_queue_is_a_bounded_fifo_cap_2() {
    check_queue(2);
}
