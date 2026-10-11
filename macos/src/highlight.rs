//! Markdown source highlighting for the macOS editor: which byte ranges of the text are headings, emphasis, code, ...
//!
//! This lives in the macOS crate rather than in `markite-core` on purpose: only this frontend uses it (KDE and GNOME
//! highlight with their toolkits' own engines). It is parsing logic, so it is tested here (`tests/highlight.rs`).
//!
//! Spans come out in pre-order (a parent before its children) and Swift applies them in order, so a child overrides
//! its parent. Offsets are UTF-16 code units, which is what `NSRange` uses.

use comrak::nodes::{AstNode, NodeValue};
use comrak::{parse_document, Arena, Options};

/// Above this size the text is not highlighted (it would re-parse on every keystroke).
/// shortcut: no highlighting for huge files; upgrade to highlighting only the visible range.
const MAX_HIGHLIGHT_BYTES: usize = 4 << 20;

/// Kinds mirror the token names used by the generated palette (`Palette.swift`, key in brackets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SpanKind {
    Header,           // header
    HorizontalRule,   // horizontal-rule
    ListMarker,       // list-marker
    BlockquoteMarker, // blockquote-marker
    QuoteText,        // quote-text
    CodeSpan,         // code-span
    Code,             // code (indented block)
    Fence,            // fence
    Table,            // table
    Emphasis,         // emphasis
    StrongEmphasis,   // strong-emphasis
    BoldItalic,       // bold-italic
    LinkText,         // link-text
    Url,              // url
    Autolink,         // autolink
    Image,            // image
    ImageUrl,         // image-url
    BackslashEscape,  // backslash-escape
    Comment,          // comment
}

/// A half-open `[start, end)` range in UTF-16 code units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub kind: SpanKind,
}

/// Highlight spans for `text`, in application order. Never panics: a node whose position does not map onto the text
/// is skipped.
#[uniffi::export]
pub fn markdown_spans(text: String) -> Vec<Span> {
    if text.len() > MAX_HIGHLIGHT_BYTES {
        markite_core::trace!(
            "markdown_spans: {} bytes, over the limit, skipped",
            text.len()
        );
        return Vec::new();
    }
    let mut opts = Options::default();
    opts.extension.table = true;
    opts.extension.strikethrough = true;
    opts.extension.tasklist = true;
    opts.extension.autolink = true;
    opts.extension.footnotes = true;
    let arena = Arena::new();
    let root = parse_document(&arena, &text, &opts);

    let mut lines = vec![0usize];
    lines.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    let mut byte_spans: Vec<(usize, usize, SpanKind)> = Vec::new();
    for node in root.descendants() {
        collect(node, &text, &lines, &mut byte_spans);
    }
    to_utf16(&text, byte_spans)
}

/// Byte range of a node, from its 1-based (line, byte column) start and inclusive end.
fn node_range(node: &AstNode, text: &str, lines: &[usize]) -> Option<(usize, usize)> {
    let pos = node.data.borrow().sourcepos;
    if pos.start.line == 0 || pos.start.column == 0 || pos.end.line == 0 || pos.end.column == 0 {
        return None;
    }
    let start = lines.get(pos.start.line - 1)? + (pos.start.column - 1);
    let end = lines.get(pos.end.line - 1)? + pos.end.column;
    let end = end.min(text.len());
    (start < end && text.is_char_boundary(start) && text.is_char_boundary(end))
        .then_some((start, end))
}

fn collect<'a>(
    node: &'a AstNode<'a>,
    text: &str,
    lines: &[usize],
    out: &mut Vec<(usize, usize, SpanKind)>,
) {
    let Some((start, end)) = node_range(node, text, lines) else {
        return;
    };
    let src = &text[start..end];
    let value = node.data.borrow().value.clone();
    match value {
        NodeValue::Heading(_) => out.push((start, end, SpanKind::Header)),
        NodeValue::ThematicBreak => out.push((start, end, SpanKind::HorizontalRule)),
        NodeValue::BlockQuote => {
            out.push((start, end, SpanKind::QuoteText));
            // The `>` markers: each line's leading run of `>` and spaces.
            let mut at = start;
            for line in src.split_inclusive('\n') {
                let lead = line.len() - line.trim_start_matches([' ', '>']).len();
                if line[..lead].contains('>') {
                    out.push((at, at + lead, SpanKind::BlockquoteMarker));
                }
                at += line.len();
            }
        }
        NodeValue::Item(_) => {
            let marker = src.find(char::is_whitespace).unwrap_or(src.len());
            out.push((start, start + marker, SpanKind::ListMarker));
        }
        NodeValue::Code(_) => out.push((start, end, SpanKind::CodeSpan)),
        NodeValue::CodeBlock(cb) => out.push((
            start,
            end,
            if cb.fenced {
                SpanKind::Fence
            } else {
                SpanKind::Code
            },
        )),
        NodeValue::HtmlBlock(_) | NodeValue::HtmlInline(_) if src.starts_with("<!--") => {
            out.push((start, end, SpanKind::Comment))
        }
        NodeValue::Table(_) => out.push((start, end, SpanKind::Table)),
        NodeValue::Emph | NodeValue::Strong => {
            let strong = matches!(value, NodeValue::Strong);
            out.push((
                start,
                end,
                if strong {
                    SpanKind::StrongEmphasis
                } else {
                    SpanKind::Emphasis
                },
            ));
            // Nested in the other kind: bold and italic at once.
            if node.ancestors().skip(1).any(|a| {
                let v = &a.data.borrow().value;
                if strong {
                    matches!(v, NodeValue::Emph)
                } else {
                    matches!(v, NodeValue::Strong)
                }
            }) {
                out.push((start, end, SpanKind::BoldItalic));
            }
        }
        NodeValue::Link(_) | NodeValue::Image(_) => {
            let image = matches!(value, NodeValue::Image(_));
            match src.rfind("](") {
                Some(split) => {
                    out.push((
                        start,
                        end,
                        if image {
                            SpanKind::Image
                        } else {
                            SpanKind::LinkText
                        },
                    ));
                    out.push((
                        start + split,
                        end,
                        if image {
                            SpanKind::ImageUrl
                        } else {
                            SpanKind::Url
                        },
                    ));
                }
                // `<https://...>` or a bare URL found by the autolink extension.
                None if !image && !src.starts_with('[') => {
                    out.push((start, end, SpanKind::Autolink))
                }
                None => out.push((
                    start,
                    end,
                    if image {
                        SpanKind::Image
                    } else {
                        SpanKind::LinkText
                    },
                )),
            }
        }
        NodeValue::Text(_) => {
            for (i, _) in src.match_indices('\\') {
                let next = src[i + 1..].chars().next();
                if next.is_some_and(|c| c.is_ascii_punctuation()) {
                    out.push((start + i, start + i + 2, SpanKind::BackslashEscape));
                }
            }
        }
        _ => {}
    }
}

/// Convert byte offsets to UTF-16 offsets in one pass over the text (no per-offset rescans).
fn to_utf16(text: &str, spans: Vec<(usize, usize, SpanKind)>) -> Vec<Span> {
    let mut offsets: Vec<usize> = spans.iter().flat_map(|&(s, e, _)| [s, e]).collect();
    offsets.sort_unstable();
    offsets.dedup();
    let mut units = Vec::with_capacity(offsets.len());
    let (mut next, mut count) = (0, 0u32);
    for (byte, ch) in text
        .char_indices()
        .chain(std::iter::once((text.len(), '\0')))
    {
        while next < offsets.len() && offsets[next] == byte {
            units.push(count);
            next += 1;
        }
        count = count.saturating_add(ch.len_utf16() as u32);
    }
    let unit = |byte: usize| {
        offsets
            .binary_search(&byte)
            .ok()
            .and_then(|i| units.get(i).copied())
    };
    spans
        .into_iter()
        .filter_map(|(s, e, kind)| {
            Some(Span {
                start: unit(s)?,
                end: unit(e)?,
                kind,
            })
        })
        .filter(|s| s.start < s.end)
        .collect()
}
