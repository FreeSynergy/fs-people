//! Splitting a `.vcf` stream into single vCards.
//!
//! Pattern: Iterator-style scan — one pass over the lines, yielding the exact source text of
//! each `BEGIN:VCARD` .. `END:VCARD` block (needed to keep the original byte for byte, ADR 0030),
//! and counting the lines no complete block holds, so an import can report them.

/// Split a `.vcf` stream into the exact text of each `BEGIN:VCARD` .. `END:VCARD` block.
pub fn split_vcards(text: &str) -> Vec<&str> {
    scan(text).0
}

/// The blocks of `text` and how many non-empty lines are outside any complete one.
pub(super) fn scan(text: &str) -> (Vec<&str>, usize) {
    let mut out = Vec::new();
    let mut lost = 0;
    let mut open: Option<(usize, usize)> = None; // where it starts, how many lines it has
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if t.eq_ignore_ascii_case("BEGIN:VCARD") {
            lost += open.map_or(0, |(_, lines)| lines);
            open = Some((pos, 1));
        } else if t.eq_ignore_ascii_case("END:VCARD") {
            match open.take() {
                Some((start, _)) => out.push(&text[start..pos + line.len()]),
                None => lost += 1,
            }
        } else if let Some((_, lines)) = &mut open {
            *lines += 1;
        } else if !t.is_empty() {
            lost += 1;
        }
        pos += line.len();
    }
    (out, lost + open.map_or(0, |(_, lines)| lines))
}
