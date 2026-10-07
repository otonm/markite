//! Rendered preview as a list of top-level blocks, each tagged with its source lines.
//! This is what makes editor <-> preview position sync toolkit-neutral: a frontend lays
//! the blocks out however it likes, measures where each one landed, and uses
//! `line_to_block` / `block_to_line` to translate scroll positions.

use crate::highlight::Highlighter;
use comrak::{format_html_with_plugins, options::Plugins, parse_document, Arena};

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// 1-based, inclusive.
    pub start_line: usize,
    pub end_line: usize,
    pub html: String,
}

/// `theme` names a KDE editor theme ("Breeze Dark", ...) used to colour fenced code; unknown/empty = plain.
pub fn to_blocks(markdown: &str, theme: &str) -> Vec<Block> {
    let arena = Arena::new();
    let opts = crate::render::options();
    let root = parse_document(&arena, markdown, &opts);
    let highlighter = Highlighter::new(theme);
    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = highlighter.as_ref().map(|h| h as _);
    root.children()
        .map(|node| {
            let pos = node.data.borrow().sourcepos;
            let mut html = String::new();
            format_html_with_plugins(node, &opts, &mut html, &plugins).expect("writing to String cannot fail");
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
