//! Rendered preview as a list of top-level blocks, each tagged with its source lines.
//! This is what makes editor <-> preview position sync toolkit-neutral: a frontend lays
//! the blocks out however it likes, measures where each one landed, and uses
//! `line_to_block` / `block_to_line` to translate scroll positions.

use comrak::{format_html, parse_document, Arena};

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// 1-based, inclusive.
    pub start_line: usize,
    pub end_line: usize,
    pub html: String,
}

pub fn to_blocks(markdown: &str) -> Vec<Block> {
    let arena = Arena::new();
    let opts = crate::render::options();
    let root = parse_document(&arena, markdown, &opts);
    root.children()
        .map(|node| {
            let pos = node.data.borrow().sourcepos;
            let mut html = String::new();
            format_html(node, &opts, &mut html).expect("writing to String cannot fail");
            Block {
                start_line: pos.start.line.max(1),
                end_line: pos.end.line.max(pos.start.line).max(1),
                html,
            }
        })
        .collect()
}

/// Source position (1-based, fractional: 12.5 = halfway through line 12) ->
/// (block index, 0..1 fraction through that block). Lines between blocks map to the
/// start of the next block; lines after the last block map to its end.
pub fn line_to_block(blocks: &[Block], line: f64) -> (usize, f64) {
    if blocks.is_empty() {
        return (0, 0.0);
    }
    for (i, b) in blocks.iter().enumerate() {
        let (start, end) = (b.start_line as f64, b.end_line as f64 + 1.0);
        if line < start {
            return (i, 0.0);
        }
        if line < end {
            return (i, (line - start) / (end - start));
        }
    }
    (blocks.len() - 1, 1.0)
}

/// Inverse of `line_to_block`.
pub fn block_to_line(blocks: &[Block], index: usize, fraction: f64) -> f64 {
    match blocks.get(index) {
        None => 1.0,
        Some(b) => {
            let (start, end) = (b.start_line as f64, b.end_line as f64 + 1.0);
            start + fraction.clamp(0.0, 1.0) * (end - start)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MD: &str = "# Title\n\npara one\nstill one\n\n- a\n- b\n- c\n";

    #[test]
    fn splits_top_level_blocks_with_lines() {
        let b = to_blocks(MD);
        let spans: Vec<_> = b.iter().map(|b| (b.start_line, b.end_line)).collect();
        assert_eq!(spans, [(1, 1), (3, 4), (6, 8)]);
        assert!(b[0].html.contains("<h1"));
        assert!(b[2].html.contains("<ul"));
    }

    #[test]
    fn line_block_mapping_roundtrips() {
        let b = to_blocks(MD);
        assert_eq!(line_to_block(&b, 1.0), (0, 0.0));
        assert_eq!(line_to_block(&b, 2.0), (1, 0.0)); // blank line -> start of next block
        assert_eq!(line_to_block(&b, 4.0), (1, 0.5));
        assert_eq!(line_to_block(&b, 99.0), (2, 1.0));
        for line in [1.0, 3.0, 3.5, 4.0, 6.0, 7.25] {
            let (i, f) = line_to_block(&b, line);
            assert!((block_to_line(&b, i, f) - line).abs() < 1e-9, "line {line}");
        }
        assert_eq!(line_to_block(&[], 5.0), (0, 0.0));
    }
}
