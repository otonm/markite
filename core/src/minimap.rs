//! Toolkit-neutral data for a code map. The frontend only paints:
//! one row per `Line`, x = indent, width = len, colour by `kind`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Blank,
    Heading,
    Code,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    pub indent: u16,
    pub len: u16,
    pub kind: LineKind,
}

fn clamp_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

pub fn lines(text: &str) -> Vec<Line> {
    crate::trace!("minimap::lines: classifying {} bytes", text.len());
    let mut in_fence = false;
    text.lines()
        .map(|raw| {
            let trimmed = raw.trim_start();
            // Known limitation: `indent` counts bytes but `len` counts chars, so a non-ASCII leading space overstates the indent.
            let indent = clamp_u16(raw.len() - trimmed.len());
            let len = clamp_u16(trimmed.trim_end().chars().count());
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
            } else if trimmed.starts_with('#') {
                LineKind::Heading
            } else {
                LineKind::Text
            };
            Line { indent, len, kind }
        })
        .collect()
}
