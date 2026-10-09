//! GSettings access: locating the schema and the few typed reads that validate values feeding the layout.

use gtk::{gio, prelude::*};
use markite_core::options::{self, Appearance};
use std::path::Path;

pub const APP_ID: &str = "io.github.otonm.markite";

/// Opens the application settings. The schema comes from the default source (installed system-wide or in a
/// Flatpak), else from `<data_dir>/schemas`, a directory holding the compiled `gschemas.compiled`. `Settings::new`
/// aborts the process on a missing schema, so it is only reached once a lookup succeeded.
pub fn load(data_dir: &Path) -> gio::Settings {
    let default = gio::SettingsSchemaSource::default();
    let schema = default
        .as_ref()
        .and_then(|s| s.lookup(APP_ID, true))
        .or_else(|| {
            let dir = data_dir.join("schemas");
            markite_core::trace!("settings: schema not installed, trying {}", dir.display());
            gio::SettingsSchemaSource::from_directory(&dir, default.as_ref(), false)
                .ok()?
                .lookup(APP_ID, true)
        })
        .unwrap_or_else(|| {
            panic!(
                "GSettings schema {APP_ID} not found: install it or compile it into {}/schemas (glib-compile-schemas)",
                data_dir.display()
            )
        });
    gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, None)
}

pub fn appearance(s: &gio::Settings) -> Appearance {
    Appearance::parse(&s.string("appearance"))
}

pub fn wrap_column(s: &gio::Settings) -> i32 {
    options::clamp_wrap_column(s.int("wrap-column"))
}

pub fn preview_width(s: &gio::Settings) -> i32 {
    options::clamp_preview_width(s.int("preview-width"))
}

/// The saved size of a font-size key (`code-font-size`, `preview-font-size`), `None` = system default.
pub fn font_size(s: &gio::Settings, key: &str) -> Option<i32> {
    options::font_size(s.int(key))
}
