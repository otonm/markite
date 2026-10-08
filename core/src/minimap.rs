//! Toolkit-neutral data for a code map. The frontend only paints:
//! one row per `Line`, x = indent, width = len, colour by `kind`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Blank,
    Heading,
    Code,
    Text,
    /// A list item: `marker` chars of bullet or number, then text.
    ListItem,
    Quote,
    Table,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    pub indent: u16,
    pub len: u16,
    pub kind: LineKind,
    /// Length of the list marker (`-`, `1.`) for `ListItem`, else 0; the frontend colours it apart from the text.
    pub marker: u16,
}

fn clamp_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// Length of a leading list marker (`-`, `*`, `+`, `1.`, `1)`) followed by a space, or 0.
fn list_marker(trimmed: &str) -> usize {
    let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
    let len = if digits > 0 { digits + 1 } else { 1 };
    let marker_ok = match trimmed.as_bytes().get(digits) {
        Some(b'.' | b')') if digits > 0 => true,
        Some(b'-' | b'*' | b'+') if digits == 0 => true,
        _ => false,
    };
    if marker_ok && trimmed.as_bytes().get(len) == Some(&b' ') {
        len
    } else {
        0
    }
}

/// Classify each line the way the editor's Markdown highlighting colours it, so the map matches the text.
/// Known limitation: a rough line-by-line approximation (no inline styles; a 4-space-indented line after a blank
/// line counts as code even when it continues a list item).
pub fn lines(text: &str) -> Vec<Line> {
    crate::trace!("minimap::lines: classifying {} bytes", text.len());
    let mut in_fence = false;
    let mut prev = LineKind::Blank;
    text.lines()
        .map(|raw| {
            let trimmed = raw.trim_start();
            // Known limitation: `indent` counts bytes but `len` counts chars, so a non-ASCII leading space overstates the indent.
            let indent = clamp_u16(raw.len() - trimmed.len());
            let len = clamp_u16(trimmed.trim_end().chars().count());
            let marker = list_marker(trimmed);
            let kind = if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_fence = !in_fence;
                crate::trace!(
                    "minimap::lines: a code fence {}",
                    if in_fence { "opens" } else { "closes" }
                );
                LineKind::Code
            } else if in_fence {
                LineKind::Code
            } else if len == 0 {
                LineKind::Blank
            } else if indent >= 4 && matches!(prev, LineKind::Blank | LineKind::Code) {
                LineKind::Code
            } else if trimmed.starts_with('#') {
                LineKind::Heading
            } else if trimmed.starts_with('>') {
                LineKind::Quote
            } else if marker > 0 {
                LineKind::ListItem
            } else if trimmed.starts_with('|') {
                LineKind::Table
            } else {
                LineKind::Text
            };
            prev = kind;
            Line {
                indent,
                len,
                kind,
                marker: if kind == LineKind::ListItem {
                    clamp_u16(marker)
                } else {
                    0
                },
            }
        })
        .collect()
}
