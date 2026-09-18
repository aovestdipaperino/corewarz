//! Line-level tokenisation.

/// A source line split into its syntactic parts, with comments removed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Line {
    /// One-based source line number.
    pub number: usize,
    /// The label, if any. Labels are not column-based: any leading token
    /// that is not a known mnemonic (or, per the grammar fallback, not
    /// followed by an operand-shaped token) is a label regardless of
    /// indentation.
    pub label: Option<String>,
    /// The mnemonic or pseudo-op.
    pub mnemonic: Option<String>,
    /// The modifier suffix, without the dot.
    pub modifier: Option<String>,
    /// The comma-separated operands, trimmed.
    pub operands: Vec<String>,
}

impl Line {
    /// Returns `true` if the line carries no mnemonic and no label.
    #[must_use]
    pub fn is_blank(&self) -> bool {
        self.label.is_none() && self.mnemonic.is_none()
    }
}

/// Returns `true` when `token` looks like the start of an operand rather
/// than a bare identifier: an addressing-mode sigil, a leading digit, or a
/// leading `-` (a negative field, e.g. `$-1`).
fn operand_like(token: &str) -> bool {
    matches!(
        token.chars().next(),
        Some('#' | '$' | '*' | '@' | '{' | '<' | '}' | '>' | '-') | Some('0'..='9')
    )
}

/// Splits `source` into [`Line`]s, stripping comments and blank lines.
///
/// Mnemonics (opcodes and pseudo-ops) are reserved words: the assembler
/// rejects any label named after one (see `assemble::check_not_reserved`),
/// so whenever `head` IS a known mnemonic it is unambiguously the
/// instruction, regardless of indentation or what follows (this also
/// covers a mnemonic standing alone with no operand, e.g. `END`). The
/// remaining ambiguity is a typo'd opcode versus a label: when `head` is
/// NOT a known mnemonic, the grammar decides instead of the dictionary —
/// after a label only a mnemonic can follow, never an operand, so `head` is
/// treated as the (unrecognised) mnemonic itself only when what follows is
/// operand-shaped (`FROB $0, $0`); otherwise `head` is a label (`step EQU
/// 4`, `mylabel FROB $0,$0` blames `FROB`).
#[must_use]
pub fn lex(source: &str, is_mnemonic: &dyn Fn(&str) -> bool) -> Vec<Line> {
    let mut out = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let text = raw.split(';').next().unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }
        let (head, rest) = match text.split_once(char::is_whitespace) {
            Some((h, r)) => (h, r.trim()),
            None => (text, ""),
        };
        let mut line = Line {
            number: index + 1,
            ..Line::default()
        };
        let rest_first = rest.split_whitespace().next().unwrap_or("");
        let head_is_instruction = is_mnemonic(head) || operand_like(rest_first);
        let body = if head_is_instruction {
            text
        } else {
            line.label = Some(head.to_string());
            rest
        };
        if body.is_empty() {
            out.push(line);
            continue;
        }
        let (mnemonic_part, operand_part) = match body.split_once(char::is_whitespace) {
            Some((m, o)) => (m, o.trim()),
            None => (body, ""),
        };
        let (mnemonic, modifier) = match mnemonic_part.split_once('.') {
            Some((m, md)) => (m.to_string(), Some(md.to_string())),
            None => (mnemonic_part.to_string(), None),
        };
        line.mnemonic = Some(mnemonic);
        line.modifier = modifier;
        if !operand_part.is_empty() {
            line.operands = operand_part
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        out.push(line);
    }
    out
}
