//! Where the app's data files live (language definition, style schemes, compiled schemas) and registering them
//! with GtkSourceView.

use std::path::{Path, PathBuf};

/// `MARKITE_DATA_DIR` if set, else `<exe dir>/../share/markite` (an installed layout), else the source tree's
/// `gnome/data` (running from `cargo run`).
pub fn dir() -> PathBuf {
    if let Some(d) = std::env::var_os("MARKITE_DATA_DIR") {
        return PathBuf::from(d);
    }
    let installed = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join("../share/markite")))
        .filter(|d| d.is_dir());
    installed.unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("data"))
}

/// Add our language definition and style schemes to GtkSourceView's search paths (once per process).
pub fn register_sourceview(dir: &Path) {
    let languages = sourceview::LanguageManager::default();
    let languages_dir = dir.join("language-specs").to_string_lossy().into_owned();
    let mut paths: Vec<String> = languages
        .search_path()
        .iter()
        .map(|p| p.to_string())
        .collect();
    if !paths.contains(&languages_dir) {
        paths.push(languages_dir);
        let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
        languages.set_search_path(&refs);
    }
    let schemes = sourceview::StyleSchemeManager::default();
    let styles_dir = dir.join("styles").to_string_lossy().into_owned();
    if !schemes
        .search_path()
        .iter()
        .any(|p| p.as_str() == styles_dir)
    {
        schemes.append_search_path(&styles_dir);
    }
}
