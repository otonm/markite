use comrak::Options;

/// Markdown options shared by every renderer, so block splitting and any future export agree.
pub(crate) fn options() -> Options<'static> {
    let mut opts = Options::default();
    opts.extension.table = true;
    opts.extension.strikethrough = true;
    opts.extension.tasklist = true;
    opts.extension.autolink = true;
    opts.extension.footnotes = true;
    opts.render.sourcepos = true; // data-sourcepos attrs for HTML-based frontends
    opts
}

#[cfg(test)]
mod tests {
    use crate::blocks::to_blocks;

    #[test]
    fn gfm_table_and_sourcepos() {
        let blocks = to_blocks("# T\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
        let html: String = blocks.iter().map(|b| b.html.as_str()).collect();
        assert!(html.contains("<h1"));
        assert!(html.contains("<table"));
        assert!(html.contains("data-sourcepos"));
    }
}
