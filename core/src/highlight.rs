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

// Immutable caches: loading the syntax set is slow, so do it once. two-face = syntect defaults + TOML, TypeScript, ...
static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);
static BUILT: LazyLock<HashMap<&'static str, Theme>> =
    LazyLock::new(|| THEMES.iter().map(|&(name, text, rules)| (name, build(text, rules))).collect());

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
        BUILT.get(theme).map(Self)
    }
}

impl SyntaxHighlighterAdapter for Highlighter {
    fn write_highlighted(&self, out: &mut dyn fmt::Write, lang: Option<&str>, code: &str) -> fmt::Result {
        let token = lang.and_then(|l| l.split([',', ' ']).next()).unwrap_or("");
        let syntax = SYNTAXES
            .find_syntax_by_token(token)
            .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text());
        let mut hl = HighlightLines::new(syntax, self.0);
        let mut html = String::new();
        for line in LinesWithEndings::from(code) {
            let ok = hl.highlight_line(line, &SYNTAXES).ok().map(|regions| {
                append_highlighted_html_for_styled_line(&regions, IncludeBackground::No, &mut html)
            });
            if !matches!(ok, Some(Ok(()))) {
                return comrak::html::escape(out, code); // fall back to plain text
            }
        }
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
