use markite_core::blocks::to_blocks;

#[test]
fn gfm_table_and_sourcepos() {
    let blocks = to_blocks("# T\n\n| a | b |\n|---|---|\n| 1 | 2 |\n", "");
    let html: String = blocks.iter().map(|b| b.html.as_str()).collect();
    assert!(html.contains("<h1"));
    assert!(html.contains("<table"));
    assert!(html.contains("data-sourcepos"));
}

#[test]
fn fenced_code_is_coloured_only_with_a_theme() {
    let md = "```rust\nfn main() {}\n```\n";
    assert!(to_blocks(md, "Breeze Dark")[0]
        .html
        .contains("style=\"color:#"));
    assert!(!to_blocks(md, "")[0].html.contains("style=\"color:#"));
}

#[test]
fn untrusted_markdown_is_neutralised() {
    let html = to_blocks(
        "<script>alert(1)</script>\n\n[x](javascript:alert(1))\n",
        "",
    )
    .iter()
    .map(|b| b.html.as_str())
    .collect::<String>();
    assert!(
        !html.contains("<script"),
        "raw HTML must not pass through: {html}"
    );
    assert!(
        !html.contains("javascript:"),
        "javascript: links must lose their target: {html}"
    );
}
