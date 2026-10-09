//! Markite for GNOME: a GTK 4 / libadwaita frontend over `markite-core`.

mod data;
mod editor;
mod files;
mod fonts;
mod palette;
mod prefs;
mod preview;
mod settings;
mod window;

use adw::prelude::*;
use gtk::{gio, glib};

fn main() -> glib::ExitCode {
    markite_core::trace!(
        "main: Markite {} starting (trace build)",
        env!("CARGO_PKG_VERSION")
    );
    let app = adw::Application::builder()
        .application_id(settings::APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    app.connect_startup(|app| {
        let data_dir = data::dir();
        data::register_sourceview(&data_dir);
        app.add_action_entries([gio::ActionEntry::builder("quit")
            .activate(|app: &adw::Application, _, _| app.quit())
            .build()]);
        for (action, accels) in [
            ("app.quit", &["<Control>q"][..]),
            ("win.open", &["<Control>o"]),
            ("win.save", &["<Control>s"]),
            ("win.save-as", &["<Control><Shift>s"]),
            ("win.preferences", &["<Control>comma"]),
            ("win.close", &["<Control>w"]),
            ("win.sync-scroll", &["<Control><Shift>l"]),
            ("win.view-editor", &["<Control>1"]),
            ("win.view-preview", &["<Control>2"]),
            ("win.view-both", &["<Control>3"]),
        ] {
            app.set_accels_for_action(action, accels);
        }
    });
    app.connect_activate(|app| match app.active_window() {
        Some(window) => window.present(),
        None => window::new(app, &settings()).present(),
    });
    app.connect_open(|app, files, _| {
        let settings = settings();
        for file in files {
            window::open(app, &settings, file.clone());
        }
    });
    app.run()
}

/// The GSettings object, loaded once.
fn settings() -> gio::Settings {
    thread_local!(static SETTINGS: gio::Settings = settings::load(&data::dir()));
    SETTINGS.with(Clone::clone)
}
