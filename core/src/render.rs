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
