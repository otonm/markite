use markite_macos::{markdown_spans, Span, SpanKind};

/// Text covered by the spans of one kind (UTF-16 offsets, so the tests stay ASCII unless they say otherwise).
fn of(text: &str, kind: SpanKind) -> Vec<String> {
    let units: Vec<u16> = text.encode_utf16().collect();
    markdown_spans(text.to_string())
        .into_iter()
        .filter(|s| s.kind == kind)
        .map(|Span { start, end, .. }| {
            String::from_utf16(&units[start as usize..end as usize]).unwrap()
        })
        .collect()
}

#[test]
fn headings_and_rules() {
    assert_eq!(of("# Title\n\ntext\n", SpanKind::Header), ["# Title"]);
    assert_eq!(of("a\n\n---\n", SpanKind::HorizontalRule), ["---"]);
}

#[test]
fn inline_styles() {
    assert_eq!(of("an *em* here", SpanKind::Emphasis), ["*em*"]);
    assert_eq!(of("a **bold** one", SpanKind::StrongEmphasis), ["**bold**"]);
    assert_eq!(of("use `code` now", SpanKind::CodeSpan), ["`code`"]);
    assert_eq!(
        of("**bold _and italic_**", SpanKind::BoldItalic),
        ["_and italic_"]
    );
}

#[test]
fn lists_and_quotes() {
    assert_eq!(of("- one\n- two\n", SpanKind::ListMarker), ["-", "-"]);
    assert_eq!(of("1. one\n", SpanKind::ListMarker), ["1."]);
    assert_eq!(
        of("> quoted\n> more\n", SpanKind::BlockquoteMarker),
        ["> ", "> "]
    );
    assert!(!of("> quoted\n", SpanKind::QuoteText).is_empty());
}

#[test]
fn links_and_images() {
    assert_eq!(
        of("see [a link](http://x.y) ok", SpanKind::LinkText),
        ["[a link](http://x.y)"]
    );
    assert_eq!(
        of("see [a link](http://x.y) ok", SpanKind::Url),
        ["](http://x.y)"]
    );
    assert_eq!(of("![alt](pic.png)", SpanKind::ImageUrl), ["](pic.png)"]);
    assert_eq!(
        of("go <https://example.com> now", SpanKind::Autolink),
        ["<https://example.com>"]
    );
}

#[test]
fn blocks() {
    assert_eq!(of("```rust\nfn main() {}\n```\n", SpanKind::Fence).len(), 1);
    assert_eq!(of("    indented\n", SpanKind::Code).len(), 1);
    assert_eq!(of("|a|b|\n|-|-|\n|1|2|\n", SpanKind::Table).len(), 1);
    assert_eq!(
        of("<!-- note -->\n\ntext\n", SpanKind::Comment),
        ["<!-- note -->"]
    );
}

#[test]
fn markdown_inside_a_fence_is_not_styled() {
    assert!(of("```\n# not a heading\n*x*\n```\n", SpanKind::Header).is_empty());
    assert!(of("```\n*x*\n```\n", SpanKind::Emphasis).is_empty());
}

#[test]
fn unterminated_fence_and_backslash_escape() {
    assert_eq!(of("```\ncode\n", SpanKind::Fence).len(), 1);
    assert_eq!(of(r"a \* b", SpanKind::BackslashEscape), [r"\*"]);
}

#[test]
fn offsets_are_utf16_not_bytes() {
    // The emoji is 2 UTF-16 units (4 bytes) and "é" is 1 unit (2 bytes) before the emphasis.
    let text = "é😀 *em*";
    let spans = markdown_spans(text.to_string());
    let em = spans.iter().find(|s| s.kind == SpanKind::Emphasis).unwrap();
    assert_eq!((em.start, em.end), (4, 8));
    assert_eq!(of(text, SpanKind::Emphasis), ["*em*"]);
}

#[test]
fn degenerate_input_does_not_panic() {
    for text in [
        "",
        "\n",
        "\u{0}",
        "[",
        "![",
        "**",
        "> ",
        "- ",
        "\r\n\r\n# x\r\n",
        "`",
        "é\n> é\n",
    ] {
        let _ = markdown_spans(text.to_string());
    }
}
