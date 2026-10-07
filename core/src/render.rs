use comrak::Options;

/// Markdown options for the preview. The Markdown is untrusted (any file the user opens), so the safe renderer
/// settings are pinned explicitly rather than left to comrak's defaults.
pub(crate) fn options() -> Options<'static> {
    let mut opts = Options::default();
    opts.extension.table = true;
    opts.extension.strikethrough = true;
    opts.extension.tasklist = true;
    opts.extension.autolink = true;
    opts.extension.footnotes = true;
    opts.render.r#unsafe = false; // raw HTML is replaced by a comment; javascript:/vbscript:/file: links lose their target
    opts.render.sourcepos = true; // data-sourcepos attrs for HTML-based frontends
    opts
}
