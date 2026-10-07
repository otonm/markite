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

pub fn lines(text: &str) -> Vec<Line> {
    let mut in_fence = false;
    text.lines()
        .map(|raw| {
            let trimmed = raw.trim_start();
            let indent = (raw.len() - trimmed.len()).min(u16::MAX as usize) as u16;
            let len = trimmed.trim_end().chars().count().min(u16::MAX as usize) as u16;
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_fence = !in_fence;
                return Line { indent, len, kind: LineKind::Code };
            }
            let kind = if in_fence {
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
