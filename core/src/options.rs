//! User-option vocabulary shared by frontends: value ranges, the curated font families and the colour-theme
//! families. Frontends store the values their own way (QSettings, GSettings) but validate with these, so a
//! hand-edited settings file cannot produce an unusable layout.

use std::ops::RangeInclusive;

/// Characters after which the editor wraps (when wrapping is on).
pub const WRAP_COLUMN: RangeInclusive<i32> = 60..=200;
pub const DEFAULT_WRAP_COLUMN: i32 = 100;
/// Maximum width of the preview text, percent of the window.
pub const PREVIEW_WIDTH: RangeInclusive<i32> = 20..=100;
pub const DEFAULT_PREVIEW_WIDTH: i32 = 100;
/// Font size in points; a saved 0 (or anything below the range) means "the system default size".
pub const FONT_SIZE: RangeInclusive<i32> = 6..=40;

pub const CODE_FONTS: [&str; 4] = [
    "Fira Code",
    "JetBrains Mono",
    "Cascadia Code",
    "Monaspace Neon",
];
pub const SANS_FONTS: [&str; 3] = ["Inter", "Open Sans", "Roboto"];
pub const SERIF_FONTS: [&str; 3] = ["Merriweather", "Lora", "Source Serif 4"];

/// Colour-theme families; each has a light and a dark variant (see `theme_name`).
pub const THEME_FAMILIES: [&str; 5] = ["Breeze", "Atom One", "Catppuccin", "GitHub", "Solarized"];

pub fn clamp_wrap_column(v: i32) -> i32 {
    v.clamp(*WRAP_COLUMN.start(), *WRAP_COLUMN.end())
}

pub fn clamp_preview_width(v: i32) -> i32 {
    v.clamp(*PREVIEW_WIDTH.start(), *PREVIEW_WIDTH.end())
}

/// The font size to use: the saved size clamped into range, or `None` for "keep the system default".
pub fn font_size(saved: i32) -> Option<i32> {
    (saved > 0).then(|| saved.clamp(*FONT_SIZE.start(), *FONT_SIZE.end()))
}

/// Which variant of a theme family to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    /// Anything unknown (e.g. a hand-edited value) follows the system.
    pub fn parse(s: &str) -> Self {
        match s {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

/// The KDE-style theme name for a family and variant, e.g. ("Catppuccin", dark) -> "Catppuccin Mocha". An unknown
/// family falls back to the first one.
pub fn theme_name(family: &str, dark: bool) -> &'static str {
    const NAMES: [(&str, &str, &str); 5] = [
        ("Breeze", "Breeze Light", "Breeze Dark"),
        ("Atom One", "Atom One Light", "Atom One Dark"),
        ("Catppuccin", "Catppuccin Latte", "Catppuccin Mocha"),
        ("GitHub", "GitHub Light", "GitHub Dark"),
        ("Solarized", "Solarized Light", "Solarized Dark"),
    ];
    let (_, light, dark_name) = NAMES.iter().find(|n| n.0 == family).unwrap_or(&NAMES[0]);
    if dark {
        dark_name
    } else {
        light
    }
}
