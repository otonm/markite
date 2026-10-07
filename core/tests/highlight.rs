//! The highlighter is crate-private; exercised through the public block renderer.
use markite_core::blocks::to_blocks;

fn html(lang: &str, code: &str, theme: &str) -> String {
    to_blocks(&format!("```{lang}\n{code}```\n"), theme)[0]
        .html
        .clone()
}

#[test]
fn colours_follow_theme_and_escape() {
    let code = "fn main() { let x = \"<hi>\"; }\n";
    let light = html("rust", code, "Breeze Light");
    assert!(light.contains("style=\"color:#"));
    assert!(light.contains("&lt;hi&gt;"));
    assert_ne!(light, html("rust", code, "Breeze Dark"));
}

#[test]
fn extra_languages_are_highlighted() {
    assert!(
        html("toml", "[a]\nb = \"c\"\n", "Breeze Dark")
            .matches("<span")
            .count()
            > 1
    );
}

#[test]
fn unknown_theme_is_plain() {
    assert!(!html("rust", "fn main() {}\n", "nope").contains("<span"));
}

/// Names the QML side (`EditorTheme.qml`) can pass to `setSyntaxTheme`; a typo on either side would
/// silently disable highlighting.
#[test]
fn every_qml_theme_name_is_known() {
    let names = [
        "Breeze Light",
        "Breeze Dark",
        "Atom One Light",
        "Atom One Dark",
        "Catppuccin Latte",
        "Catppuccin Mocha",
        "GitHub Light",
        "GitHub Dark",
        "Solarized Light",
        "Solarized Dark",
    ];
    for n in names {
        assert!(
            html("rust", "fn main() {}\n", n).contains("style=\"color:#"),
            "{n}"
        );
    }
}
