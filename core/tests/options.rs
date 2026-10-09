use markite_core::options::*;

#[test]
fn clamps_and_font_size() {
    assert_eq!(clamp_wrap_column(10), 60);
    assert_eq!(clamp_wrap_column(999), 200);
    assert_eq!(clamp_preview_width(0), 20);
    assert_eq!(clamp_preview_width(50), 50);
    assert_eq!(font_size(0), None);
    assert_eq!(font_size(-3), None);
    assert_eq!(font_size(3), Some(6));
    assert_eq!(font_size(12), Some(12));
    assert_eq!(font_size(99), Some(40));
}

#[test]
fn appearance_and_theme_names() {
    assert_eq!(Appearance::parse("dark"), Appearance::Dark);
    assert_eq!(Appearance::parse("light").as_str(), "light");
    assert_eq!(Appearance::parse("bogus"), Appearance::System);
    assert_eq!(theme_name("Catppuccin", true), "Catppuccin Mocha");
    assert_eq!(theme_name("Solarized", false), "Solarized Light");
    assert_eq!(theme_name("nope", true), "Breeze Dark");
    for f in THEME_FAMILIES {
        assert_ne!(theme_name(f, false), theme_name(f, true));
    }
}
