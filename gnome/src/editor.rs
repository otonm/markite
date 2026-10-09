//! The editor pane: a sourceview `View` with line numbers, an optional native minimap and a wrap column.

use gtk::{gdk, glib, pango};
use markite_core::options::FONT_SIZE;
use sourceview::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Gap kept right of the text when the wrap column is not the limiting factor.
const MIN_RIGHT_MARGIN: i32 = 12;
const LEFT_MARGIN: i32 = 12;
/// Measured: GtkTextView wraps about one character (plus a pixel) earlier than the margins say (cause not
/// identified), so the column gets that much extra room, plus half a character of tolerance for font rounding.
const CURSOR_SLACK: f64 = 1.5;
const MAP_WIDTH: i32 = 110;

/// Wrap settings shared with the resize handler.
struct Wrap {
    on: Cell<bool>,
    column: Cell<i32>,
}

pub struct Editor {
    pub widget: gtk::Widget,
    pub view: sourceview::View,
    pub map: sourceview::Map,
    map_overlay: gtk::Overlay,
    buffer: sourceview::Buffer,
    scroll: gtk::ScrolledWindow,
    css: gtk::CssProvider,
    css_class: String,
    wrap: Rc<Wrap>,
    /// Set while the text is replaced from code, so `connect_changed` only reports user edits.
    quiet: Rc<Cell<bool>>,
}

impl Drop for Editor {
    fn drop(&mut self) {
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_remove_provider_for_display(&display, &self.css);
        }
    }
}

/// Width of one character in the view's current font. Needs the style (font) of the view to be up to date.
fn char_width(view: &sourceview::View) -> f64 {
    let ctx = view.pango_context();
    let metrics = ctx.metrics(ctx.font_description().as_ref(), None);
    f64::from(metrics.approximate_char_width()) / f64::from(pango::SCALE)
}

/// Wraps at `column` characters (left aligned) by making the right margin the part of the pane the column does
/// not need; the margin never goes below `MIN_RIGHT_MARGIN`, so a narrow pane wraps at its edge instead.
fn apply_wrap(view: &sourceview::View, wrap: &Wrap) {
    let margin = if wrap.on.get() {
        let text = view.visible_rect().width() - view.left_margin();
        let needed =
            ((f64::from(wrap.column.get()) + CURSOR_SLACK) * char_width(view)).round() as i32 + 1;
        (text - needed).max(MIN_RIGHT_MARGIN)
    } else {
        MIN_RIGHT_MARGIN
    };
    if view.right_margin() != margin {
        view.set_right_margin(margin);
    }
}

/// A family name goes into a CSS string and comes from saved settings, so only plain name characters pass.
fn safe_family(family: Option<&str>) -> Option<&str> {
    let f = family?.trim();
    let ok = !f.is_empty()
        && f.chars().count() <= 100
        && f.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '+'));
    if !ok && !f.is_empty() {
        markite_core::trace!("editor: rejected unsafe font family name");
    }
    ok.then_some(f)
}

impl Editor {
    pub fn new() -> Rc<Editor> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let css_class = format!("markite-editor-{}", NEXT.fetch_add(1, Ordering::Relaxed));

        let buffer = sourceview::Buffer::new(None);
        let langs = sourceview::LanguageManager::default();
        buffer.set_language(
            langs
                .language("markite-markdown")
                .or_else(|| langs.language("markdown"))
                .as_ref(),
        );
        buffer.set_highlight_syntax(true);

        let view = sourceview::View::with_buffer(&buffer);
        view.add_css_class(&css_class);
        view.set_show_line_numbers(true);
        view.set_highlight_current_line(false);
        view.set_monospace(true);
        view.set_tab_width(4);
        view.set_auto_indent(true);
        view.set_left_margin(LEFT_MARGIN);
        view.set_right_margin(MIN_RIGHT_MARGIN);
        view.set_top_margin(8);
        view.set_bottom_margin(8);
        view.set_wrap_mode(gtk::WrapMode::None);
        view.upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Label("Markdown editor")]);

        // The scrollbar must stay available: with wrapping off, long lines scroll horizontally.
        let scroll = gtk::ScrolledWindow::builder()
            .child(&view)
            .hexpand(true)
            .vexpand(true)
            .build();

        // The minimap draws the text scaled down from its own font; at the view's font size it would be
        // full-size and ask for a huge minimum width, so it gets a 1pt font and a fixed width.
        let map = sourceview::Map::new();
        map.set_view(&view);
        map.set_hexpand(false);
        map.set_width_request(MAP_WIDTH);
        map.set_font_desc(Some(&map_font(None)));

        let map_overlay = gtk::Overlay::new();
        map_overlay.set_child(Some(&map));
        let indicator = gtk::Box::new(gtk::Orientation::Vertical, 0);
        indicator.add_css_class("markite-map-viewport");
        indicator.set_can_target(false);
        map_overlay.add_overlay(&indicator);
        // The map's own viewport slider is allocated but not drawn in this setup (checked: right size and position,
        // no pixels), so the visible region is marked with a translucent box laid over the map instead.
        map_overlay.connect_get_child_position({
            let (view, map) = (view.downgrade(), map.downgrade());
            move |_, _| Some(viewport_rect(&view.upgrade()?, &map.upgrade()?))
        });
        for adjustment in [scroll.vadjustment(), map.vadjustment().unwrap_or_default()] {
            let overlay = map_overlay.downgrade();
            let requeue = move |_: &gtk::Adjustment| {
                if let Some(o) = overlay.upgrade() {
                    o.queue_allocate();
                }
            };
            adjustment.connect_value_changed(requeue.clone());
            adjustment.connect_upper_notify(move |a| requeue(a));
        }

        let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        root.set_hexpand(true);
        root.set_vexpand(true);
        root.append(&scroll);
        root.append(&map_overlay);

        let viewport_css = gtk::CssProvider::new();
        viewport_css.load_from_string(
            ".markite-map-viewport { background-color: alpha(currentColor, 0.18); border-radius: 2px; }",
        );
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &viewport_css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let css = gtk::CssProvider::new();
        css.connect_parsing_error(|_, _, _err| markite_core::trace!("editor: css error: {_err}"));
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let wrap = Rc::new(Wrap {
            on: Cell::new(false),
            column: Cell::new(markite_core::options::DEFAULT_WRAP_COLUMN),
        });
        // The pane size changes the text width the column is measured against.
        let weak = view.downgrade();
        let w = wrap.clone();
        scroll.hadjustment().connect_page_size_notify(move |_| {
            if let Some(view) = weak.upgrade() {
                apply_wrap(&view, &w);
            }
        });

        let editor = Rc::new(Editor {
            widget: root.upcast(),
            view,
            map,
            map_overlay,
            buffer,
            scroll,
            css,
            css_class,
            wrap,
            quiet: Rc::new(Cell::new(false)),
        });
        editor.set_font(None, None);
        editor
    }

    pub fn text(&self) -> String {
        self.buffer
            .text(&self.buffer.start_iter(), &self.buffer.end_iter(), false)
            .to_string()
    }

    /// Replaces the text without making the replacement an undo step or reporting a user edit.
    fn replace(&self, text: &str) {
        self.quiet.set(true);
        self.buffer.begin_irreversible_action();
        self.buffer.set_text(text);
        self.buffer.end_irreversible_action();
        self.quiet.set(false);
    }

    pub fn set_text(&self, text: &str) {
        self.replace(text);
        self.buffer.place_cursor(&self.buffer.start_iter());
        self.scroll.vadjustment().set_value(0.0);
    }

    pub fn replace_text_keep_view(&self, text: &str) {
        let line = self.top_line();
        let cursor = self.buffer.cursor_position();
        self.replace(text);
        // iter_at_offset clamps an offset past the new end.
        self.buffer
            .place_cursor(&self.buffer.iter_at_offset(cursor));
        self.scroll_to_line(line);
    }

    pub fn connect_changed(&self, f: impl Fn() + 'static) {
        let quiet = self.quiet.clone();
        self.buffer.connect_changed(move |_| {
            if !quiet.get() {
                f();
            }
        });
    }

    pub fn set_scheme(&self, scheme_id: &str) {
        let scheme = sourceview::StyleSchemeManager::default().scheme(scheme_id);
        if scheme.is_none() {
            markite_core::trace!("editor: unknown style scheme {scheme_id}");
        }
        self.buffer.set_style_scheme(scheme.as_ref());
    }

    pub fn set_font(&self, family: Option<&str>, size_pt: Option<f64>) {
        let family = safe_family(family);
        let (lo, hi) = (f64::from(*FONT_SIZE.start()), f64::from(*FONT_SIZE.end()));
        let mut rule = String::from(
            "font-feature-settings: \"liga\" 1, \"calt\" 1, \"kern\" 1; font-kerning: normal;",
        );
        if let Some(f) = family {
            rule += &format!(" font-family: \"{f}\", monospace;");
        }
        if let Some(s) = size_pt.filter(|s| s.is_finite()) {
            rule += &format!(" font-size: {}pt;", s.clamp(lo, hi));
        }
        self.css
            .load_from_string(&format!("textview.{} {{ {rule} }}", self.css_class));
        self.map.set_font_desc(Some(&map_font(family)));
        // The view's pango font follows the CSS only once its style is resolved, which can be after this call.
        apply_wrap(&self.view, &self.wrap);
        let (view, wrap) = (self.view.downgrade(), self.wrap.clone());
        glib::idle_add_local_once(move || {
            if let Some(view) = view.upgrade() {
                apply_wrap(&view, &wrap);
            }
        });
    }

    pub fn set_wrap(&self, on: bool, column: i32) {
        self.wrap.on.set(on);
        self.wrap
            .column
            .set(markite_core::options::clamp_wrap_column(column));
        self.view.set_wrap_mode(if on {
            gtk::WrapMode::WordChar
        } else {
            gtk::WrapMode::None
        });
        apply_wrap(&self.view, &self.wrap);
    }

    pub fn set_show_map(&self, on: bool) {
        self.map_overlay.set_visible(on);
    }

    /// 1-based line at the top of the view; the fraction is how far the top edge is into that logical line's
    /// vertical extent (so with wrapping, 0.5 is half way down its display lines, not half its text).
    pub fn top_line(&self) -> f64 {
        let y = self.view.visible_rect().y();
        let (iter, line_top) = self.view.line_at_y(y);
        let (_, height) = self.view.line_yrange(&iter);
        let fraction = if height > 0 {
            (f64::from(y - line_top) / f64::from(height)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        f64::from(iter.line()) + 1.0 + fraction
    }

    /// Inverse of `top_line`. A line past the end scrolls to the last one; NaN counts as the first.
    pub fn scroll_to_line(&self, line: f64) {
        scroll_view_to_line(&self.view, line);
        // Right after a text change the line heights, so the scroll range, are not known yet and the move
        // falls short: repeat it once they are.
        if (self.top_line() - line).abs() > 0.01 {
            let view = self.view.downgrade();
            glib::idle_add_local_once(move || {
                if let Some(view) = view.upgrade() {
                    scroll_view_to_line(&view, line);
                }
            });
        }
    }

    pub fn connect_scrolled(&self, f: impl Fn() + 'static) {
        self.scroll
            .vadjustment()
            .connect_value_changed(move |_| f());
    }

    pub fn grab_focus(&self) {
        self.view.grab_focus();
    }
}

/// Where the editor's visible part sits on the map: the same proportions GtkSourceMap uses for its own slider,
/// at least `MIN_VIEWPORT` tall so it stays visible on very long documents.
fn viewport_rect(view: &sourceview::View, map: &sourceview::Map) -> gdk::Rectangle {
    const MIN_VIEWPORT: f64 = 10.0;
    let end = view.buffer().end_iter();
    let bottom = |v: &gtk::TextView| {
        let r = v.iter_location(&end);
        f64::from(r.y() + r.height()).max(1.0)
    };
    let (view_total, map_total) = (bottom(view.upcast_ref()), bottom(map.upcast_ref()));
    let visible = view.visible_rect();
    let top = f64::from(visible.y()) / view_total * map_total;
    let bottom_edge = f64::from(visible.y() + visible.height()) / view_total * map_total;
    let y = top - f64::from(map.visible_rect().y());
    gdk::Rectangle::new(
        0,
        y.round() as i32,
        map.width(),
        (bottom_edge - top).max(MIN_VIEWPORT).round() as i32,
    )
}

fn scroll_view_to_line(view: &sourceview::View, line: f64) {
    let buffer = view.buffer();
    let line = if line.is_finite() { line.max(1.0) } else { 1.0 };
    let whole = line.floor();
    let index = (whole as i64 - 1).min(i64::from(buffer.line_count() - 1)) as i32;
    let Some(iter) = buffer.iter_at_line(index) else {
        return;
    };
    // Set the adjustment directly: scroll_to_iter animates, which is wrong for scroll sync and for reloads. The move
    // is the difference to what is visible now, which keeps the margins in these coordinates out of the sum.
    let (y, height) = view.line_yrange(&iter);
    let target = f64::from(y) + (line - whole) * f64::from(height);
    if let Some(adj) = view.vadjustment() {
        adj.set_value(adj.value() + target - f64::from(view.visible_rect().y()));
    }
}

fn map_font(family: Option<&str>) -> pango::FontDescription {
    let mut fd = pango::FontDescription::new();
    fd.set_family(family.unwrap_or("Monospace"));
    fd.set_size(pango::SCALE); // 1pt
    fd
}
