//! Syntax highlighting for fenced code in the preview. Qt rich text ignores CSS classes, so colours are
//! emitted inline. Token colours come from the same KDE themes the editor uses (`syntax_themes.rs`,
//! generated), mapped onto TextMate scopes, so preview and editor match.

use crate::syntax_themes::THEMES;
use comrak::adapters::SyntaxHighlighterAdapter;
use std::{borrow::Cow, collections::HashMap, fmt, str::FromStr, sync::LazyLock};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem},
    html::{append_highlighted_html_for_styled_line, IncludeBackground},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

// Immutable caches, built once on first use and shared. two-face = syntect's default syntaxes plus extra languages (e.g. TOML).
static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(|| {
    crate::trace!("highlight: loading the two-face syntax set (first use only)");
    let set = two_face::syntax::extra_newlines();
    crate::trace!("highlight: {} syntaxes loaded", set.syntaxes().len());
    set
});
static BUILT: LazyLock<HashMap<&'static str, Theme>> = LazyLock::new(|| {
    crate::trace!("highlight: building {} syntect themes from the generated KDE colour tables (first use only)", THEMES.len());
    THEMES.iter().map(|&(name, text, rules)| (name, build(text, rules))).collect()
});

fn color(hex: &str) -> Color {
    let n = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
    Color { r: (n >> 16) as u8, g: (n >> 8) as u8, b: n as u8, a: 255 }
}

fn build(text: &str, rules: crate::syntax_themes::Rules) -> Theme {
    let mut theme = Theme::default();
    theme.settings.foreground = Some(color(text));
    theme.scopes = rules
        .iter()
        .map(|&(scope, fg, flags)| ThemeItem {
            scope: ScopeSelectors::from_str(scope).expect("generated scope selector"),
            style: StyleModifier {
                foreground: Some(color(fg)),
                background: None,
                font_style: Some(flags.chars().fold(FontStyle::empty(), |f, c| {
                    f | match c {
                        'b' => FontStyle::BOLD,
                        'i' => FontStyle::ITALIC,
                        _ => FontStyle::UNDERLINE,
                    }
                })),
            },
        })
        .collect();
    theme
}

/// Highlights with the named KDE theme (e.g. "Breeze Dark"); `None` for an unknown name.
pub(crate) struct Highlighter(&'static Theme);

impl Highlighter {
    pub(crate) fn new(theme: &str) -> Option<Self> {
        let found = BUILT.get(theme).map(Self);
        crate::trace!("Highlighter::new({theme:?}): {}", if found.is_some() { "known theme" } else { "unknown theme -> None, code stays plain" });
        found
    }
}

impl SyntaxHighlighterAdapter for Highlighter {
    fn write_highlighted(&self, out: &mut dyn fmt::Write, lang: Option<&str>, code: &str) -> fmt::Result {
        let token = lang.and_then(|l| l.split([',', ' ']).next()).unwrap_or("");
        let syntax = SYNTAXES.find_syntax_by_token(token).unwrap_or_else(|| {
            crate::trace!("write_highlighted: no syntax for {token:?}, using plain text");
            SYNTAXES.find_syntax_plain_text()
        });
        crate::trace!("write_highlighted: info {lang:?} -> token {token:?} -> syntax {:?}, {} bytes of code", syntax.name, code.len());
        let mut hl = HighlightLines::new(syntax, self.0);
        let mut html = String::new();
        for line in LinesWithEndings::from(code) {
            let ok = hl.highlight_line(line, &SYNTAXES).ok().map(|regions| {
                append_highlighted_html_for_styled_line(&regions, IncludeBackground::No, &mut html)
            });
            if !matches!(ok, Some(Ok(()))) {
                crate::trace!("write_highlighted: syntect failed on a line, falling back to escaped plain text");
                return comrak::html::escape(out, code); // fall back to plain text
            }
        }
        crate::trace!("write_highlighted: produced {} bytes of coloured HTML", html.len());
        out.write_str(&html)
    }

    // Qt rich text needs neither attributes nor classes; the block background comes from the preview CSS.
    fn write_pre_tag(&self, out: &mut dyn fmt::Write, _: HashMap<&'static str, Cow<'_, str>>) -> fmt::Result {
        out.write_str("<pre>")
    }

    fn write_code_tag(&self, out: &mut dyn fmt::Write, _: HashMap<&'static str, Cow<'_, str>>) -> fmt::Result {
        out.write_str("<code>")
    }
}
