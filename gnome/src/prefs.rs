//! The Preferences dialog: every row is bound to GSettings, so changes made elsewhere show up live.

use crate::fonts::Fonts;
use adw::prelude::*;
use gtk::{gio, glib, pango};
use markite_core::options::{
    CODE_FONTS, FONT_SIZE, PREVIEW_WIDTH, SANS_FONTS, SERIF_FONTS, THEME_FAMILIES, WRAP_COLUMN,
};
use std::cell::RefCell;
use std::rc::Rc;

const APPEARANCES: [&str; 3] = ["system", "light", "dark"];
/// Used when the system font size cannot be read (e.g. it is given in pixels).
const FALLBACK_SIZE: i32 = 11;

pub fn show(parent: &impl IsA<gtk::Widget>, settings: &gio::Settings, fonts: &Fonts) {
    markite_core::trace!("prefs: open");
    let sys_size = system_font_size(parent);
    let dialog = adw::PreferencesDialog::new();
    dialog.set_title("Preferences");

    let code_fonts = [(None, fonts.available(&CODE_FONTS))];
    let preview_fonts = [
        (Some("Sans-Serif Fonts"), fonts.available(&SANS_FONTS)),
        (Some("Serif Fonts"), fonts.available(&SERIF_FONTS)),
    ];
    dialog.add(&page(
        "Appearance",
        "applications-graphics-symbolic",
        &[
            group(
                "Theme",
                &[
                    choice_row(
                        settings,
                        "editor-theme",
                        "Theme family",
                        "Colours of the editor and preview",
                        &THEME_FAMILIES,
                        &THEME_FAMILIES,
                    )
                    .upcast(),
                    choice_row(
                        settings,
                        "appearance",
                        "Variant",
                        "Light or dark for the whole application",
                        &APPEARANCES,
                        &["Follow System", "Always Light", "Always Dark"],
                    )
                    .upcast(),
                ],
            ),
            group(
                "Code",
                &[
                    font_row(settings, "code-font", "Code font", &code_fonts).upcast(),
                    size_row(settings, "code-font-size", "Code font size", sys_size).upcast(),
                ],
            ),
            group(
                "Preview",
                &[
                    font_row(settings, "preview-font", "Preview font", &preview_fonts).upcast(),
                    size_row(settings, "preview-font-size", "Preview font size", sys_size).upcast(),
                ],
            ),
        ],
    ));

    let wrap = switch_row(
        settings,
        "wrap-text",
        "Wrap text",
        "Wrap long lines in the editor",
    );
    let column = spin_row(
        settings,
        "wrap-column",
        "Wrap column",
        "Wrap after this many characters, or at the window edge if that comes first",
        &WRAP_COLUMN,
    );
    settings
        .bind("wrap-text", &column, "sensitive")
        .flags(gio::SettingsBindFlags::GET)
        .build();
    dialog.add(&page(
        "Editor",
        "document-edit-symbolic",
        &[group(
            "Editing",
            &[
                wrap.upcast(),
                column.upcast(),
                switch_row(
                    settings,
                    "sync-scroll",
                    "Sync scrolling",
                    "Editor and preview scroll together (Ctrl+Shift+L)",
                )
                .upcast(),
            ],
        )],
    ));

    dialog.add(&page(
        "Preview",
        "view-reveal-symbolic",
        &[group(
            "Layout",
            &[spin_row(
                settings,
                "preview-width",
                "Text width (%)",
                "Maximum width of the preview text as a share of the window. A narrower pane always fits, with no horizontal scrolling.",
                &PREVIEW_WIDTH,
            )
            .upcast()],
        )],
    ));

    dialog.add(&page(
        "Files",
        "folder-documents-symbolic",
        &[group(
            "Saving and reloading",
            &[
                switch_row(
                    settings,
                    "watch-files",
                    "Monitor file changes",
                    "Reload when the file changes on disk (local and remote)",
                )
                .upcast(),
                switch_row(
                    settings,
                    "convert-encoding",
                    "Convert encoding",
                    "Save as UTF-8 instead of the file's own encoding",
                )
                .upcast(),
                switch_row(
                    settings,
                    "convert-line-endings",
                    "Convert line endings",
                    "Save with LF instead of the file's own line endings",
                )
                .upcast(),
            ],
        )],
    ));

    dialog.add(&page(
        "Interface",
        "preferences-system-symbolic",
        &[group(
            "Window",
            &[
                switch_row(
                    settings,
                    "show-minimap",
                    "Show minimap",
                    "Code map beside the editor",
                )
                .upcast(),
                switch_row(
                    settings,
                    "show-status-bar",
                    "Show status bar",
                    "Path, counts, encoding and line ending",
                )
                .upcast(),
            ],
        )],
    ));
    dialog.present(Some(parent));
}

fn page(title: &str, icon: &str, groups: &[adw::PreferencesGroup]) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(title)
        .icon_name(icon)
        .build();
    for g in groups {
        page.add(g);
    }
    page
}

fn group(title: &str, rows: &[gtk::Widget]) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title(title).build();
    for r in rows {
        group.add(r);
    }
    group
}

fn switch_row(s: &gio::Settings, key: &str, title: &str, subtitle: &str) -> adw::SwitchRow {
    let row = adw::SwitchRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build();
    s.bind(key, &row, "active").build();
    row
}

/// A spin row whose limits equal the schema range, so the UI can never ask for an out-of-range value.
fn spin_row(
    s: &gio::Settings,
    key: &str,
    title: &str,
    subtitle: &str,
    range: &std::ops::RangeInclusive<i32>,
) -> adw::SpinRow {
    let row = adw::SpinRow::with_range(*range.start() as f64, *range.end() as f64, 1.0);
    row.set_title(title);
    row.set_subtitle(subtitle);
    s.bind(key, &row, "value").build();
    row
}

/// A combo row for a string key restricted to `ids`; `labels` are what the user sees.
fn choice_row(
    s: &gio::Settings,
    key: &str,
    title: &str,
    subtitle: &str,
    ids: &'static [&'static str],
    labels: &[&str],
) -> adw::ComboRow {
    let row = adw::ComboRow::builder()
        .title(title)
        .subtitle(subtitle)
        .model(&gtk::StringList::new(labels))
        .build();
    s.bind(key, &row, "selected")
        .mapping(move |v, _| {
            let id = v.get::<String>()?;
            ids.iter()
                .position(|i| *i == id)
                .map(|i| (i as u32).to_value())
        })
        .set_mapping(move |val, _| {
            ids.get(val.get::<u32>().ok()? as usize)
                .map(|i| i.to_variant())
        })
        .build();
    row
}

/// The size of the default UI font in points.
fn system_font_size(widget: &impl IsA<gtk::Widget>) -> i32 {
    let size = widget
        .pango_context()
        .font_description()
        .map_or(0, |d| d.size());
    match (size as f64 / pango::SCALE as f64).round() as i32 {
        s if s > 0 => s.clamp(*FONT_SIZE.start(), *FONT_SIZE.end()),
        _ => FALLBACK_SIZE,
    }
}

/// Runs `f(widget)` whenever `key` changes, for as long as the widget lives. Done by hand (not `bind`) for rows that
/// show a derived value; the handler is dropped with the widget so reopening the dialog never piles them up.
fn watch<W: IsA<gtk::Widget>>(s: &gio::Settings, key: &str, widget: &W, f: impl Fn(&W) + 'static) {
    let weak = widget.downgrade();
    let id = s.connect_changed(Some(key), move |_, _| {
        if let Some(w) = weak.upgrade() {
            f(&w);
        }
    });
    let id = RefCell::new(Some(id));
    let s = s.clone();
    widget.connect_destroy(move |_| {
        if let Some(id) = id.take() {
            s.disconnect(id);
        }
    });
}

/// Writes cannot fail for values inside the schema range, which the rows guarantee; a failure is only traced.
#[allow(unused_variables)]
fn log_err(key: &str, result: Result<(), glib::BoolError>) {
    if let Err(e) = result {
        markite_core::trace!("prefs: {key} not saved: {e}");
    }
}

/// A font size row showing the effective size: while the saved size is 0 it shows the system size and the key stays
/// 0 until the user changes the value. Only a differing value is written, which also prevents an update loop.
fn size_row(s: &gio::Settings, key: &'static str, title: &str, system: i32) -> adw::SpinRow {
    let row = adw::SpinRow::with_range(*FONT_SIZE.start() as f64, *FONT_SIZE.end() as f64, 1.0);
    row.set_title(title);
    row.set_subtitle("In points; the system size until changed");
    let effective = {
        let s = s.clone();
        move || markite_core::options::font_size(s.int(key)).unwrap_or(system)
    };
    row.set_value(effective() as f64);
    let eff = effective.clone();
    let s2 = s.clone();
    row.connect_value_notify(move |r| {
        let v = r.value().round() as i32;
        if v != eff() && FONT_SIZE.contains(&v) {
            log_err(key, s2.set_int(key, v));
        }
    });
    watch(s, key, &row, move |r| r.set_value(effective() as f64));
    row
}

const SYSTEM_DEFAULT: &str = "System Default";

/// A row that picks a font family from a popover list: "System Default" first, then the sections with dimmed
/// headings (an `adw::ComboRow` cannot show headings). The subtitle shows the current choice; a saved family that
/// is not in the list counts as the system default.
fn font_row(
    s: &gio::Settings,
    key: &'static str,
    title: &str,
    sections: &[(Option<&str>, Vec<String>)],
) -> adw::ActionRow {
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["navigation-sidebar"])
        .build();
    // (row, family id ("" = system default), check mark)
    let mut choices: Vec<(gtk::ListBoxRow, String, gtk::Image)> = Vec::new();
    let mut add_choice = |id: &str, label: &str| {
        let check = gtk::Image::from_icon_name("object-select-symbolic");
        let inner = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let name = gtk::Label::builder()
            .label(label)
            .xalign(0.0)
            .hexpand(true)
            .build();
        inner.append(&name);
        inner.append(&check);
        let row = gtk::ListBoxRow::builder().child(&inner).build();
        row.update_property(&[gtk::accessible::Property::Label(label)]);
        list.append(&row);
        choices.push((row, id.to_string(), check));
    };
    add_choice("", SYSTEM_DEFAULT);
    for (heading, families) in sections.iter().filter(|(_, f)| !f.is_empty()) {
        if let Some(h) = heading {
            let label = gtk::Label::builder()
                .label(*h)
                .xalign(0.0)
                .css_classes(["caption-heading", "dim-label"])
                .margin_start(12)
                .margin_top(6)
                .build();
            let head = gtk::Box::new(gtk::Orientation::Vertical, 6);
            head.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            head.append(&label);
            list.append(
                &gtk::ListBoxRow::builder()
                    .child(&head)
                    .activatable(false)
                    .selectable(false)
                    .focusable(false)
                    .build(),
            );
        }
        for f in families {
            add_choice(f, f);
        }
    }
    let choices = Rc::new(choices);

    let scroll = gtk::ScrolledWindow::builder()
        .child(&list)
        .propagate_natural_height(true)
        .max_content_height(360)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    let popover = gtk::Popover::builder().child(&scroll).build();
    let button = gtk::MenuButton::builder()
        .popover(&popover)
        .icon_name("pan-down-symbolic")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    button.update_property(&[gtk::accessible::Property::Label(&format!(
        "Choose {}",
        title.to_lowercase()
    ))]);
    let row = adw::ActionRow::builder().title(title).build();
    row.add_suffix(&button);
    row.set_activatable_widget(Some(&button));

    // Marks the saved family (or the system default when it is unknown) and mirrors it in the subtitle.
    let refresh = {
        let choices = choices.clone();
        let s = s.clone();
        move |row: &adw::ActionRow| {
            let saved = s.string(key);
            let known = choices.iter().any(|(_, id, _)| *id == saved);
            let current = if known { saved.as_str() } else { "" };
            for (r, id, check) in choices.iter() {
                let on = id == current;
                check.set_opacity(if on { 1.0 } else { 0.0 });
                r.update_state(&[gtk::accessible::State::Selected(Some(on))]);
                if on {
                    row.set_subtitle(if id.is_empty() { SYSTEM_DEFAULT } else { id });
                }
            }
        }
    };
    refresh(&row);
    watch(s, key, &row, refresh);

    let s = s.clone();
    list.connect_row_activated(move |_, activated| {
        if let Some((_, id, _)) = choices.iter().find(|(r, _, _)| r == activated) {
            log_err(key, s.set_string(key, id));
        }
        popover.popdown();
    });
    row
}
