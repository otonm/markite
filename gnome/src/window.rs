//! The main window: wires one `markite_core::Document` to the editor, the preview, the header bar, the status bar,
//! the Preferences dialog and the files (local through core, remote through GIO). Policy and text handling stay in
//! core; this file only mirrors them into widgets.

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use adw::prelude::*;
use gtk::{gdk, gio, glib};
use markite_core::blocks::{self, Block};
use markite_core::document::{read_limited, ExternalChange};
use markite_core::{location, options, Document};

use crate::editor::Editor;
use crate::fonts::Fonts;
use crate::preview::{Preview, PreviewStyle};
use crate::{files, palette, prefs, settings};

/// Header toggle order (Editor | Both | Preview) -> the `view-mode` value (0 editor, 1 preview, 2 both).
const MODE_OF_TOGGLE: [i32; 3] = [0, 2, 1];
const POLL: Duration = Duration::from_millis(1500);
/// Remote files are re-read every 7th tick (about 10 s): each check downloads the whole file.
const REMOTE_POLL_EVERY: u32 = 7;
const RENDER_DELAY: Duration = Duration::from_millis(120);
/// Scroll events this soon after a programmatic scroll are its echo, not the user.
const ECHO: Duration = Duration::from_millis(150);
const SLIDE: u32 = 180;

thread_local! {
    /// Keeps each window's state alive while the window exists (callbacks only hold weak references).
    static WINDOWS: RefCell<Vec<Rc<Win>>> = const { RefCell::new(Vec::new()) };
}

pub struct Win {
    app: adw::Application,
    window: adw::ApplicationWindow,
    settings: gio::Settings,
    fonts: Fonts,
    doc: RefCell<Document>,
    blocks: RefCell<Vec<Block>>,
    editor: Rc<Editor>,
    preview: Rc<Preview>,
    paned: gtk::Paned,
    toggles: adw::ToggleGroup,
    title: adw::WindowTitle,
    toasts: adw::ToastOverlay,
    status_bar: gtk::Box,
    path_label: gtk::Label,
    counts_label: gtk::Label,
    format_label: gtk::Label,
    render_source: RefCell<Option<glib::SourceId>>,
    /// View mode the panes show (animations lag the setting by up to `SLIDE` ms).
    shown_mode: Cell<i32>,
    animating: Cell<bool>,
    anim: RefCell<Option<adw::TimedAnimation>>,
    /// Editor pane width in "both" mode once the user dragged the divider (None = follow the window, half each).
    both_position: Cell<Option<i32>>,
    programmatic_position: Cell<bool>,
    editor_echo_until: Cell<Instant>,
    polling: Cell<bool>,
    poll_tick: Cell<u32>,
    close_confirmed: Cell<bool>,
}

/// A window action's handler.
type Handler = Box<dyn Fn(&Rc<Win>)>;

/// File name only (decoded for URLs); the status bar shows the full path.
fn display_name(path: Option<&Path>) -> String {
    let Some(path) = path else {
        return "Untitled".into();
    };
    let path = path.to_string_lossy();
    let last = path.rsplit('/').next().unwrap_or(&path);
    if path.contains("://") {
        glib::Uri::unescape_string(last, None).map_or_else(|| last.to_string(), |s| s.to_string())
    } else {
        last.to_string()
    }
}

fn file_for(path: &str) -> gio::File {
    if location::is_remote(path) {
        gio::File::for_uri(path)
    } else {
        gio::File::for_path(path)
    }
}

/// Open `file` in a window nobody has used yet, else in a new window (the GNOME convention).
pub fn open(app: &adw::Application, settings: &gio::Settings, file: gio::File) {
    let unused = WINDOWS.with(|w| w.borrow().iter().find(|w| w.pristine()).cloned());
    let target = unused.unwrap_or_else(|| new(app, settings));
    target.present();
    target.open_file(file);
}

/// Create a window for `app`; it is shown by the caller.
pub fn new(app: &adw::Application, settings: &gio::Settings) -> Rc<Win> {
    let editor = Editor::new();
    let preview = Preview::new();
    let paned = gtk::Paned::builder()
        .start_child(&editor.widget)
        .end_child(&preview.widget)
        .resize_start_child(true)
        .resize_end_child(true)
        .shrink_start_child(true)
        .shrink_end_child(true)
        .build();

    let title = adw::WindowTitle::new("Markite", "");
    let toggles = adw::ToggleGroup::new();
    for label in ["Editor", "Both", "Preview"] {
        toggles.add(adw::Toggle::builder().label(label).build());
    }
    let header = adw::HeaderBar::builder().title_widget(&title).build();
    let open_button = gtk::Button::builder()
        .icon_name("document-open-symbolic")
        .action_name("win.open")
        .tooltip_text("Open (Ctrl+O)")
        .build();
    let save_button = gtk::Button::builder()
        .icon_name("document-save-symbolic")
        .action_name("win.save")
        .tooltip_text("Save (Ctrl+S)")
        .build();
    header.pack_start(&open_button);
    header.pack_start(&save_button);
    header.pack_start(&toggles);
    let menu = gio::Menu::new();
    let file_section = gio::Menu::new();
    file_section.append(Some("_Save As…"), Some("win.save-as"));
    menu.append_section(None, &file_section);
    let app_section = gio::Menu::new();
    app_section.append(Some("_Preferences"), Some("win.preferences"));
    app_section.append(Some("_About Markite"), Some("win.about"));
    app_section.append(Some("_Quit"), Some("app.quit"));
    menu.append_section(None, &app_section);
    header.pack_end(
        &gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu)
            .primary(true)
            .tooltip_text("Main menu")
            .build(),
    );

    let path_label = gtk::Label::builder()
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(gtk::pango::EllipsizeMode::Middle)
        .build();
    let counts_label = gtk::Label::new(None);
    let format_label = gtk::Label::new(None);
    let status_bar = gtk::Box::builder()
        .spacing(18)
        .margin_start(12)
        .margin_end(12)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    for label in [&path_label, &counts_label, &format_label] {
        label.add_css_class("caption");
        label.add_css_class("dim-label");
        status_bar.append(label);
    }

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&paned));
    toolbar.add_bottom_bar(&status_bar);
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&toolbar));
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .content(&toasts)
        .title("Markite")
        .default_width(settings.int("window-width"))
        .default_height(settings.int("window-height"))
        .maximized(settings.boolean("window-maximized"))
        .build();
    let fonts = Fonts::detect(&window);

    let win = Rc::new(Win {
        app: app.clone(),
        window,
        settings: settings.clone(),
        fonts,
        doc: RefCell::new(Document::default()),
        blocks: RefCell::new(Vec::new()),
        editor,
        preview,
        paned,
        toggles,
        title,
        toasts,
        status_bar,
        path_label,
        counts_label,
        format_label,
        render_source: RefCell::new(None),
        shown_mode: Cell::new(2),
        animating: Cell::new(false),
        anim: RefCell::new(None),
        both_position: Cell::new(None),
        programmatic_position: Cell::new(false),
        editor_echo_until: Cell::new(Instant::now()),
        polling: Cell::new(false),
        poll_tick: Cell::new(0),
        close_confirmed: Cell::new(false),
    });
    win.connect();
    WINDOWS.with(|w| w.borrow_mut().push(win.clone()));
    win
}

impl Win {
    pub fn present(&self) {
        self.window.present();
    }

    /// True for a window nobody has used: opening a file reuses it instead of creating another.
    fn pristine(&self) -> bool {
        let d = self.doc.borrow();
        d.path().is_none() && !d.is_dirty() && d.text().is_empty()
    }

    fn toast(&self, message: &str) {
        self.toasts.add_toast(
            adw::Toast::builder()
                .title(message)
                .use_markup(false)
                .build(),
        );
    }

    fn dark(&self) -> bool {
        adw::StyleManager::default().is_dark()
    }

    fn family(&self) -> String {
        self.settings.string("editor-theme").to_string()
    }

    // ---- wiring ----

    fn connect(self: &Rc<Self>) {
        self.add_actions();
        self.connect_signals();
        self.apply_all();
        self.set_view_mode(self.settings.int("view-mode"), false);
        self.refresh_status();
        self.render_now();
        self.editor.grab_focus();
        self.start_monitor();
    }

    fn add_actions(self: &Rc<Self>) {
        let w = &self.window;
        let simple = |name: &str, f: Handler| {
            let win = Rc::downgrade(self);
            let action = gio::SimpleAction::new(name, None);
            action.connect_activate(move |_, _| {
                if let Some(win) = win.upgrade() {
                    f(&win);
                }
            });
            w.add_action(&action);
        };
        simple("open", Box::new(|win| win.choose_file_to_open()));
        simple(
            "save",
            Box::new(|win| {
                let win = win.clone();
                glib::spawn_future_local(async move {
                    win.save(None).await;
                });
            }),
        );
        simple(
            "save-as",
            Box::new(|win| {
                let win = win.clone();
                glib::spawn_future_local(async move {
                    win.save_as().await;
                });
            }),
        );
        simple(
            "preferences",
            Box::new(|win| prefs::show(&win.window, &win.settings, &win.fonts)),
        );
        simple("about", Box::new(|win| win.show_about()));
        for (name, mode) in [("view-editor", 0), ("view-preview", 1), ("view-both", 2)] {
            simple(
                name,
                Box::new(move |win| {
                    let _ = win.settings.set_int("view-mode", mode);
                }),
            );
        }
        w.add_action(&self.settings.create_action("sync-scroll"));
        let win = Rc::downgrade(self);
        let close = gio::SimpleAction::new("close", None);
        close.connect_activate(move |_, _| {
            if let Some(win) = win.upgrade() {
                win.window.close();
            }
        });
        w.add_action(&close);
    }

    fn connect_signals(self: &Rc<Self>) {
        let win = Rc::downgrade(self);
        macro_rules! with_win {
            (|$w:ident $(, $arg:ident)*| $body:expr) => {{
                let win = win.clone();
                move |$($arg),*| {
                    if let Some($w) = win.upgrade() {
                        $body
                    }
                }
            }};
        }

        self.editor.connect_changed(with_win!(|w| w.on_edit()));
        self.editor.connect_scrolled(with_win!(|w| {
            if Instant::now() >= w.editor_echo_until.get() {
                w.sync_from_editor();
            }
        }));
        self.preview.connect_scrolled(with_win!(
            |w, index, fraction| w.sync_from_preview(index, fraction)
        ));
        self.settings
            .connect_changed(None, with_win!(|w, _s, key| w.on_setting(key)));
        let manager = adw::StyleManager::default();
        manager.connect_dark_notify(with_win!(|w, _m| w.apply_theme()));
        self.toggles.connect_active_notify(with_win!(|w, g| {
            let toggle = usize::try_from(g.active()).unwrap_or(0);
            let _ = w
                .settings
                .set_int("view-mode", MODE_OF_TOGGLE[toggle.min(2)]);
        }));

        // The divider: remember where the user put it; keep half and half while they have not.
        self.paned.connect_position_notify(with_win!(|w, p| {
            if !w.programmatic_position.get() && !w.animating.get() && w.shown_mode.get() == 2 {
                w.both_position.set(Some(p.position()));
            }
        }));
        // Window width changes: keep the preview text column limit (a share of the window) and half-and-half current.
        let last_width = Cell::new(0);
        self.window.add_tick_callback({
            let win = win.clone();
            move |window, _| {
                let Some(w) = win.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                let width = window.width();
                if width != last_width.get() {
                    last_width.set(width);
                    w.apply_preview_style();
                    if w.shown_mode.get() == 2
                        && !w.animating.get()
                        && w.both_position.get().is_none()
                    {
                        w.set_position(w.paned.width() / 2);
                    }
                }
                glib::ControlFlow::Continue
            }
        });

        self.window.connect_close_request({
            let win = win.clone();
            move |_| {
                let Some(w) = win.upgrade() else {
                    return glib::Propagation::Proceed;
                };
                w.save_geometry();
                if !w.doc.borrow().is_dirty() || w.close_confirmed.get() {
                    return glib::Propagation::Proceed;
                }
                glib::spawn_future_local(async move { w.confirm_close().await });
                glib::Propagation::Stop
            }
        });
        self.window.connect_destroy(move |_| {
            if let Some(w) = win.upgrade() {
                WINDOWS.with(|ws| ws.borrow_mut().retain(|x| !Rc::ptr_eq(x, &w)));
            }
        });

        // Dropping files onto the window opens them.
        let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        let weak = Rc::downgrade(self);
        drop.connect_drop(move |_, value, _, _| {
            let (Some(w), Ok(list)) = (weak.upgrade(), value.get::<gdk::FileList>()) else {
                return false;
            };
            for file in list.files() {
                w.open_in_best_window(file);
            }
            true
        });
        self.window.add_controller(drop);
    }

    // ---- settings -> widgets ----

    fn apply_all(self: &Rc<Self>) {
        self.apply_theme();
        self.apply_editor_font();
        self.apply_wrap();
        self.apply_convert();
        self.editor
            .set_show_map(self.settings.boolean("show-minimap"));
        self.status_bar
            .set_visible(self.settings.boolean("show-status-bar"));
    }

    fn on_setting(self: &Rc<Self>, key: &str) {
        match key {
            "appearance" | "editor-theme" => self.apply_theme(),
            "code-font" | "code-font-size" => self.apply_editor_font(),
            "preview-font" | "preview-font-size" | "preview-width" => self.apply_preview_style(),
            "wrap-text" | "wrap-column" => self.apply_wrap(),
            "convert-encoding" | "convert-line-endings" => self.apply_convert(),
            "show-minimap" => self
                .editor
                .set_show_map(self.settings.boolean("show-minimap")),
            "show-status-bar" => self
                .status_bar
                .set_visible(self.settings.boolean("show-status-bar")),
            "view-mode" => self.set_view_mode(self.settings.int("view-mode"), true),
            _ => {}
        }
    }

    fn apply_theme(self: &Rc<Self>) {
        let scheme = match settings::appearance(&self.settings) {
            options::Appearance::System => adw::ColorScheme::Default,
            options::Appearance::Light => adw::ColorScheme::ForceLight,
            options::Appearance::Dark => adw::ColorScheme::ForceDark,
        };
        adw::StyleManager::default().set_color_scheme(scheme);
        self.editor
            .set_scheme(&palette::scheme_id(&self.family(), self.dark()));
        self.apply_preview_style();
        // The preview's fenced-code colours depend on the theme name core is asked for.
        self.render_now();
    }

    /// The chosen family if installed (else None = system font) and the saved size if set.
    fn font(&self, family_key: &str, size_key: &str) -> (Option<String>, Option<f64>) {
        let family = self.settings.string(family_key);
        let family = (!family.is_empty() && self.fonts.has(&family)).then(|| family.to_string());
        let size = settings::font_size(&self.settings, size_key).map(f64::from);
        (family, size)
    }

    fn apply_editor_font(&self) {
        let (family, size) = self.font("code-font", "code-font-size");
        self.editor.set_font(family.as_deref(), size);
    }

    fn apply_preview_style(&self) {
        let (font_family, font_size_pt) = self.font("preview-font", "preview-font-size");
        let window_width = match self.window.width() {
            0 => self.settings.int("window-width"),
            w => w,
        };
        let percent = settings::preview_width(&self.settings);
        self.preview.set_style(&PreviewStyle {
            palette: palette::palette(&self.family(), self.dark()),
            font_family,
            font_size_pt,
            max_text_width_px: Some(window_width * percent / 100),
        });
    }

    fn apply_wrap(&self) {
        let column = settings::wrap_column(&self.settings);
        self.editor
            .set_wrap(self.settings.boolean("wrap-text"), column);
    }

    fn apply_convert(&self) {
        let mut d = self.doc.borrow_mut();
        d.set_convert_encoding(self.settings.boolean("convert-encoding"));
        d.set_convert_line_endings(self.settings.boolean("convert-line-endings"));
    }

    // ---- document -> widgets ----

    fn on_edit(self: &Rc<Self>) {
        let text = self.editor.text();
        if self.doc.borrow_mut().set_text(text) {
            self.refresh_status();
            self.queue_render();
        }
    }

    fn refresh_status(&self) {
        let d = self.doc.borrow();
        let name = display_name(d.path());
        let path = d
            .path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let marker = if d.is_dirty() { "• " } else { "" };
        self.title.set_title(&format!("{marker}{name}"));
        self.title.set_subtitle("");
        self.window
            .set_title(Some(&format!("{marker}{name} — Markite")));
        self.path_label
            .set_text(if path.is_empty() { "Untitled" } else { &path });
        let (words, chars) = (d.word_count(), d.char_count());
        self.counts_label.set_text(&format!(
            "{words} {} / {chars} {}",
            if words == 1 { "word" } else { "words" },
            if chars == 1 {
                "character"
            } else {
                "characters"
            }
        ));
        self.format_label.set_text(&format!(
            "{} / {}",
            d.encoding_label(),
            d.line_ending_label()
        ));
    }

    fn queue_render(self: &Rc<Self>) {
        if self.render_source.borrow().is_some() {
            return;
        }
        let win = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(RENDER_DELAY, move || {
            if let Some(w) = win.upgrade() {
                *w.render_source.borrow_mut() = None;
                w.render_now();
                w.sync_from_editor();
            }
        });
        *self.render_source.borrow_mut() = Some(id);
    }

    fn render_now(&self) {
        if let Some(id) = self.render_source.borrow_mut().take() {
            id.remove();
        }
        let theme = options::theme_name(&self.family(), self.dark());
        let blocks = self.doc.borrow().render_blocks(theme);
        self.preview.set_blocks(&blocks);
        *self.blocks.borrow_mut() = blocks;
    }

    // ---- scroll sync (core maps lines <-> blocks) ----

    fn sync_from_editor(&self) {
        if !self.settings.boolean("sync-scroll") {
            return;
        }
        let blocks = self.blocks.borrow();
        if blocks.is_empty() {
            return;
        }
        let (index, fraction) = blocks::line_to_block(&blocks, self.editor.top_line());
        self.preview.scroll_to(index, fraction);
    }

    fn sync_from_preview(&self, index: usize, fraction: f64) {
        if !self.settings.boolean("sync-scroll") {
            return;
        }
        let line = blocks::block_to_line(&self.blocks.borrow(), index, fraction);
        self.editor_echo_until.set(Instant::now() + ECHO);
        self.editor.scroll_to_line(line);
    }

    // ---- view mode ----

    fn set_position(&self, position: i32) {
        self.programmatic_position.set(true);
        self.paned.set_position(position);
        self.programmatic_position.set(false);
    }

    fn pane_visibility(&self, mode: i32) {
        self.editor.widget.set_visible(mode != 1);
        self.preview.widget.set_visible(mode != 0);
    }

    /// Show the editor, the preview or both; the divider slides between them when `animate`.
    fn set_view_mode(self: &Rc<Self>, mode: i32, animate: bool) {
        let mode = mode.clamp(0, 2);
        if let Some(i) = MODE_OF_TOGGLE.iter().position(|m| *m == mode) {
            self.toggles.set_active(u32::try_from(i).unwrap_or(0));
        }
        let previous = self.shown_mode.replace(mode);
        let width = self.paned.width();
        if !animate || width <= 0 || !self.window.is_mapped() {
            self.pane_visibility(mode);
            if mode == 2 {
                self.set_position(self.both_position.get().unwrap_or(width / 2));
            }
            return;
        }
        if previous == 2 && !self.animating.get() {
            self.both_position.set(Some(self.paned.position()));
        }
        let from = if self.editor.widget.is_visible() {
            f64::from(self.paned.position())
        } else {
            0.0
        };
        let to = match mode {
            0 => width,
            1 => 0,
            _ => self.both_position.get().unwrap_or(width / 2),
        };
        self.pane_visibility(2);
        self.animating.set(true);
        let win = Rc::downgrade(self);
        let target = adw::CallbackAnimationTarget::new({
            let win = win.clone();
            move |v| {
                if let Some(w) = win.upgrade() {
                    w.set_position(v.round() as i32);
                }
            }
        });
        let animation = adw::TimedAnimation::builder()
            .widget(&self.paned)
            .value_from(from)
            .value_to(f64::from(to))
            .duration(SLIDE)
            .easing(adw::Easing::EaseOutCubic)
            .target(&target)
            .build();
        animation.connect_done(move |_| {
            if let Some(w) = win.upgrade() {
                w.animating.set(false);
                w.pane_visibility(w.shown_mode.get());
            }
        });
        animation.play();
        *self.anim.borrow_mut() = Some(animation);
    }

    // ---- files ----

    fn choose_file_to_open(self: &Rc<Self>) {
        let win = self.clone();
        glib::spawn_future_local(async move {
            let dialog = gtk::FileDialog::builder()
                .title("Open")
                .filters(&markdown_filters())
                .modal(true)
                .build();
            match dialog.open_future(Some(&win.window)).await {
                Ok(file) => win.open_in_best_window(file),
                Err(e) if e.matches(gtk::DialogError::Dismissed) => {}
                Err(e) => win.toast(e.message()),
            }
        });
    }

    fn open_in_best_window(&self, file: gio::File) {
        open(&self.app, &self.settings, file);
    }

    pub fn open_file(self: &Rc<Self>, file: gio::File) {
        let win = self.clone();
        glib::spawn_future_local(async move {
            let loaded: io::Result<Document> = match file.path() {
                Some(path) => Document::open(path),
                None => {
                    let uri = file.uri();
                    markite_core::trace!("open: remote {}", location::redact(&uri));
                    match files::read(&file, Some(win.window.upcast_ref())).await {
                        Ok(bytes) => {
                            let mut d = Document::default();
                            d.load(uri.as_str(), &bytes);
                            Ok(d)
                        }
                        Err(e) => Err(e),
                    }
                }
            };
            match loaded {
                Ok(doc) => {
                    let text = doc.text().to_string();
                    *win.doc.borrow_mut() = doc;
                    win.apply_convert();
                    win.editor.set_text(&text);
                    win.editor.grab_focus();
                    win.refresh_status();
                    win.render_now();
                    // A fresh document starts at the top of both panes (the editor has no layout yet to sync from).
                    win.preview.scroll_to(0, 0.0);
                }
                Err(e) => win.toast(&e.to_string()),
            }
        });
    }

    /// Save to `target` (Save As) or to the document's path, asking for one if it has none. Returns whether
    /// the document is now saved.
    async fn save(self: &Rc<Self>, target: Option<gio::File>) -> bool {
        let explicit = target.is_some();
        let existing = self
            .doc
            .borrow()
            .path()
            .map(|p| p.to_string_lossy().into_owned());
        let Some(file) = target.or_else(|| existing.as_deref().map(file_for)) else {
            return self.save_as().await;
        };
        let result: io::Result<()> = match file.path() {
            Some(path) if explicit => self.doc.borrow_mut().save_as(path),
            Some(_) => self.doc.borrow_mut().save(),
            None => self.save_remote(&file).await,
        };
        match result {
            Ok(()) => {
                self.refresh_status();
                true
            }
            Err(e) => {
                self.toast(&e.to_string());
                false
            }
        }
    }

    async fn save_remote(&self, file: &gio::File) -> io::Result<()> {
        let plan = self.doc.borrow().prepare_save()?;
        files::write(file, Some(self.window.upcast_ref()), &plan.bytes).await?;
        self.doc.borrow_mut().finish_save(file.uri().as_str(), plan);
        Ok(())
    }

    async fn save_as(self: &Rc<Self>) -> bool {
        let dialog = gtk::FileDialog::builder()
            .title("Save As")
            .initial_name(display_name(self.doc.borrow().path()))
            .filters(&markdown_filters())
            .modal(true)
            .build();
        match dialog.save_future(Some(&self.window)).await {
            Ok(file) => Box::pin(self.save(Some(file))).await,
            Err(e) => {
                if !e.matches(gtk::DialogError::Dismissed) {
                    self.toast(e.message());
                }
                false
            }
        }
    }

    // ---- watching the file on disk ----

    fn start_monitor(self: &Rc<Self>) {
        let win = Rc::downgrade(self);
        glib::timeout_add_local(POLL, move || match win.upgrade() {
            Some(w) => {
                w.poll();
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        });
    }

    fn poll(self: &Rc<Self>) {
        if !self.settings.boolean("watch-files") || self.polling.get() {
            return;
        }
        let Some(path) = self
            .doc
            .borrow()
            .path()
            .map(|p| p.to_string_lossy().into_owned())
        else {
            return;
        };
        if !location::is_remote(&path) {
            // A read error (file briefly missing during another program's atomic save) is retried next tick.
            if let Ok(bytes) = read_limited(Path::new(&path)) {
                self.apply_external(&bytes);
            }
            return;
        }
        let tick = self.poll_tick.get() + 1;
        self.poll_tick.set(tick);
        if !tick.is_multiple_of(REMOTE_POLL_EVERY) {
            return;
        }
        self.polling.set(true);
        let win = self.clone();
        glib::spawn_future_local(async move {
            // No parent window: a background check must never open a password prompt.
            let read = files::read(&gio::File::for_uri(&path), None).await;
            win.polling.set(false);
            if let Ok(bytes) = read {
                win.apply_external(&bytes);
            }
        });
    }

    fn apply_external(self: &Rc<Self>, bytes: &[u8]) {
        let change = self.doc.borrow_mut().external_change(bytes);
        match change {
            ExternalChange::Unchanged => {}
            ExternalChange::Reloaded => {
                let text = self.doc.borrow().text().to_string();
                self.editor.replace_text_keep_view(&text);
                self.refresh_status();
                self.render_now();
                self.toast("File changed on disk: reloaded");
            }
            ExternalChange::Conflict => self
                .toast("File changed on disk; your unsaved edits were kept. Save to overwrite it."),
        }
    }

    // ---- closing, geometry, about ----

    async fn confirm_close(self: &Rc<Self>) {
        let name = display_name(self.doc.borrow().path());
        let dialog = adw::AlertDialog::new(
            Some("Save changes?"),
            Some(&format!(
                "“{name}” has unsaved changes. Changes that are not saved will be lost."
            )),
        );
        dialog.add_responses(&[
            ("cancel", "_Cancel"),
            ("discard", "_Discard"),
            ("save", "_Save"),
        ]);
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");
        let close = match dialog.choose_future(Some(&self.window)).await.as_str() {
            "save" => self.save(None).await,
            "discard" => true,
            _ => false,
        };
        if close {
            self.close_confirmed.set(true);
            self.window.close();
        }
    }

    fn save_geometry(&self) {
        let maximized = self.window.is_maximized();
        let _ = self.settings.set_boolean("window-maximized", maximized);
        if !maximized {
            let (w, h) = self.window.default_size();
            if w > 0 && h > 0 {
                let _ = self.settings.set_int("window-width", w.clamp(300, 16000));
                let _ = self.settings.set_int("window-height", h.clamp(200, 16000));
            }
        }
    }

    fn show_about(&self) {
        adw::AboutDialog::builder()
            .application_name("Markite")
            .application_icon(crate::settings::APP_ID)
            .developer_name("Oton Mahnic")
            .version(env!("CARGO_PKG_VERSION"))
            .comments("A small, native Markdown viewer and editor for Linux.")
            .website("https://github.com/otonm/markite")
            .issue_url("https://github.com/otonm/markite/issues")
            .copyright("© 2026 Oton Mahnic")
            .license_type(gtk::License::MitX11)
            .build()
            .present(Some(&self.window));
    }
}

fn markdown_filters() -> gio::ListStore {
    let markdown = gtk::FileFilter::new();
    markdown.set_name(Some("Markdown"));
    markdown.add_suffix("md");
    markdown.add_suffix("markdown");
    markdown.add_mime_type("text/markdown");
    let all = gtk::FileFilter::new();
    all.set_name(Some("All files"));
    all.add_pattern("*");
    let store = gio::ListStore::new::<gtk::FileFilter>();
    store.append(&markdown);
    store.append(&all);
    store
}
