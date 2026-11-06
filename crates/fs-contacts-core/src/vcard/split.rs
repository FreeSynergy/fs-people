//! Splitting a `.vcf` stream into single vCards.
//!
//! Pattern: Iterator-style scan — one pass over the lines, yielding the exact source text of
//! each `BEGIN:VCARD` .. `END:VCARD` block (needed to keep the original byte for byte, ADR 0030).

/// Split a `.vcf` stream into the exact text of each `BEGIN:VCARD` .. `END:VCARD` block.
pub fn split_vcards(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if t.eq_ignore_ascii_case("BEGIN:VCARD") {
            start = Some(pos);
        } else if t.eq_ignore_ascii_case("END:VCARD") {
            if let Some(s) = start.take() {
                out.push(&text[s..pos + line.len()]);
            }
        }
        pos += line.len();
    }
    out
}
