//! Canonical Redcode text for an assembled instruction.

use corewarz_core::{AddrMode, Instruction, Modifier, Opcode, Warrior};

/// The source sigil for an addressing mode.
#[must_use]
pub const fn mode_sigil(mode: AddrMode) -> char {
    match mode {
        AddrMode::Immediate => '#',
        AddrMode::Direct => '$',
        AddrMode::IndirectA => '*',
        AddrMode::IndirectB => '@',
        AddrMode::PredecA => '{',
        AddrMode::PredecB => '<',
        AddrMode::PostincA => '}',
        AddrMode::PostincB => '>',
    }
}

fn opcode_name(opcode: Opcode) -> &'static str {
    match opcode {
        Opcode::Dat => "DAT",
        Opcode::Mov => "MOV",
        Opcode::Add => "ADD",
        Opcode::Sub => "SUB",
        Opcode::Mul => "MUL",
        Opcode::Div => "DIV",
        Opcode::Mod => "MOD",
        Opcode::Jmp => "JMP",
        Opcode::Jmz => "JMZ",
        Opcode::Jmn => "JMN",
        Opcode::Djn => "DJN",
        Opcode::Spl => "SPL",
        Opcode::Seq => "SEQ",
        Opcode::Sne => "SNE",
        Opcode::Slt => "SLT",
        Opcode::Nop => "NOP",
        Opcode::And => "AND",
        Opcode::Or => "OR",
        Opcode::Xor => "XOR",
        Opcode::Shl => "SHL",
        Opcode::Shr => "SHR",
        Opcode::Not => "NOT",
        Opcode::Tst => "TST",
        Opcode::Jez => "JEZ",
        Opcode::Jnz => "JNZ",
        Opcode::Jlz => "JLZ",
        Opcode::Jgz => "JGZ",
        Opcode::Call => "CALL",
        Opcode::Ret => "RET",
        Opcode::Push => "PUSH",
        Opcode::Pop => "POP",
    }
}

fn modifier_name(modifier: Modifier) -> &'static str {
    match modifier {
        Modifier::A => "A",
        Modifier::B => "B",
        Modifier::AB => "AB",
        Modifier::BA => "BA",
        Modifier::F => "F",
        Modifier::X => "X",
        Modifier::I => "I",
    }
}

/// Renders one instruction as canonical Redcode.
///
/// Opcodes that accept no modifier are printed without one, so that the text
/// reassembles cleanly.
#[must_use]
pub fn disassemble_instruction(instr: Instruction) -> String {
    let suffix = if crate::modifier::legal_modifiers(instr.opcode).is_empty() {
        String::new()
    } else {
        format!(".{}", modifier_name(instr.modifier))
    };
    format!(
        "{}{} {}{}, {}{}",
        opcode_name(instr.opcode),
        suffix,
        mode_sigil(instr.a_mode),
        instr.a,
        mode_sigil(instr.b_mode),
        instr.b
    )
}

/// Renders a whole warrior, one instruction per line.
#[must_use]
pub fn disassemble(warrior: &Warrior) -> String {
    let mut out = String::new();
    if warrior.start != 0 {
        out.push_str(&format!("ORG {}\n", warrior.start));
    }
    for instr in &warrior.code {
        out.push_str(&disassemble_instruction(*instr));
        out.push('\n');
    }
    out
}
