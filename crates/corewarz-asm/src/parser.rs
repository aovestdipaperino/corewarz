//! Operand parsing: an addressing-mode sigil followed by a label-or-number
//! expression of the form `term [+|- integer]`.

use crate::error::AsmErrorKind;
use crate::modifier::parse_mode;
use corewarz_core::AddrMode;

/// An operand before label resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawOperand {
    /// The addressing mode.
    pub mode: AddrMode,
    /// A label the value is relative to, if the expression names one.
    pub label: Option<String>,
    /// The literal offset added to the label, or the whole value when there is
    /// no label.
    pub offset: i64,
}

impl RawOperand {
    /// The operand an omitted second operand assumes: `$0`.
    #[must_use]
    pub const fn direct_zero() -> Self {
        Self {
            mode: AddrMode::Direct,
            label: None,
            offset: 0,
        }
    }
}

/// Parses one operand.
///
/// # Errors
/// Returns [`AsmErrorKind::MalformedOperand`] for an unrecognised sigil or an
/// empty expression, and [`AsmErrorKind::BadNumber`] for an unparseable
/// integer.
pub fn parse_operand(text: &str) -> Result<RawOperand, AsmErrorKind> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AsmErrorKind::MalformedOperand(text.to_string()));
    }
    let first = text.chars().next().expect("non-empty");
    let (mode, expr) = if first.is_alphanumeric() || first == '-' || first == '_' {
        (AddrMode::Direct, text)
    } else {
        let mode = parse_mode(Some(first))
            .ok_or_else(|| AsmErrorKind::MalformedOperand(text.to_string()))?;
        (mode, text[first.len_utf8()..].trim())
    };
    if expr.is_empty() {
        return Err(AsmErrorKind::MalformedOperand(text.to_string()));
    }

    // Split off a trailing `+ n` or `- n`, ignoring a leading sign.
    let chars: Vec<char> = expr.chars().collect();
    let mut split = None;
    for i in (1..chars.len()).rev() {
        if chars[i] == '+' || chars[i] == '-' {
            split = Some(i);
            break;
        }
    }

    let (term, adjust) = match split {
        Some(i) => {
            let head: String = chars[..i].iter().collect();
            let tail: String = chars[i..].iter().collect();
            let n: i64 = tail
                .replace(' ', "")
                .parse()
                .map_err(|_| AsmErrorKind::BadNumber(tail.clone()))?;
            (head.trim().to_string(), n)
        }
        None => (expr.to_string(), 0),
    };

    if let Ok(n) = term.parse::<i64>() {
        return Ok(RawOperand {
            mode,
            label: None,
            offset: n + adjust,
        });
    }
    if term.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return Err(AsmErrorKind::BadNumber(term));
    }
    Ok(RawOperand {
        mode,
        label: Some(term),
        offset: adjust,
    })
}
