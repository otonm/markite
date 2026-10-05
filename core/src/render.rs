use comrak::{markdown_to_html, Options};

/// Markdown -> HTML fragment (no <html>/<body>; the frontend wraps and styles it).
pub fn to_html(markdown: &str) -> String {
    markdown_to_html(markdown, &options())
}

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
    use super::*;

    #[test]
    fn gfm_table_and_sourcepos() {
        let html = to_html("# T\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(html.contains("<h1"));
        assert!(html.contains("<table"));
        assert!(html.contains("data-sourcepos"));
    }
}
