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
/// that ambiguity by consulting `parse_opcode`.
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
        // `head` is ambiguous when it matches a pseudo-op keyword (`END`,
        // `ORG`) that also happens to be used as a label: e.g. a label named
        // `end` followed by a real instruction. Resolve the ambiguity by
        // checking whether the next token is itself a mnemonic; if it is,
        // `head` must be a label, since pseudo-ops and opcodes never nest.
        let rest_first = rest.split_whitespace().next().unwrap_or("");
        let head_is_instruction =
            is_mnemonic(head) && !(!rest_first.is_empty() && is_mnemonic(rest_first));
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
