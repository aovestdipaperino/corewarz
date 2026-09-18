//! Line-level tokenisation.

/// A source line split into its syntactic parts, with comments removed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Line {
    /// One-based source line number.
    pub number: usize,
    /// The label in column one, if any.
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

/// Splits `source` into [`Line`]s, stripping comments and blank lines.
///
/// A leading token is treated as a label when the line has more than one
/// token and the first token is not a known mnemonic; the caller resolves
/// that ambiguity by consulting `parse_opcode`. `allows_empty_operand`
/// reports whether a mnemonic is legal with no operand at all (`END`, and
/// the opcodes `DAT`/`NOP`/`RET`, whose emitter defaults a missing operand
/// to direct zero); it is consulted only for a bare single-token line.
#[must_use]
pub fn lex(
    source: &str,
    is_mnemonic: &dyn Fn(&str) -> bool,
    allows_empty_operand: &dyn Fn(&str) -> bool,
) -> Vec<Line> {
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
        // `head` is ambiguous when it matches a mnemonic (opcode or
        // pseudo-op) that also happens to be used as a label, e.g. a label
        // named `end` or `mov` on its own line, followed by a real
        // instruction. Two independent checks resolve this; DO NOT collapse
        // them into one, they cover different shapes of ambiguity:
        //
        // 1. When there IS a remainder, and that remainder's first token is
        //    itself a mnemonic, `head` must be a label: pseudo-ops and
        //    opcodes never nest, so two mnemonics on one line can only be
        //    "label mnemonic ...".
        // 2. When there is NO remainder (a single bare token on the line),
        //    `head` can only be a standalone instruction if that mnemonic is
        //    legal with zero operands. Every opcode this assembler supports
        //    requires at least one operand except `DAT`/`NOP`/`RET` (whose
        //    emitter defaults the missing operand to direct zero), and the
        //    only operand-less pseudo-op is `END`. Anything else standing
        //    alone on a line — e.g. `mov`, which requires an operand and
        //    cannot appear bare as an instruction — must be a label naming
        //    the following line, not an ill-formed instruction.
        let rest_first = rest.split_whitespace().next().unwrap_or("");
        let head_is_instruction = if rest_first.is_empty() {
            is_mnemonic(head) && allows_empty_operand(head)
        } else {
            // `head` is a label only when the next token is itself a
            // recognised mnemonic ("label MOV.I ..."). Otherwise `head` is
            // the intended mnemonic, even if it turns out to be unknown
            // (e.g. a typo like "FROB $0, $0"): an unrecognised opcode
            // should be reported by name, not misfiled as a label with the
            // error pointing at its first operand.
            !is_mnemonic(rest_first)
        };
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
