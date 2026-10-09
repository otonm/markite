//! Rendered Markdown preview: a hardened `webkit::WebView` showing the core's HTML blocks.
//!
//! The page is loaded once; afterwards blocks, style and scroll position are pushed in with JavaScript so the scroll
//! position survives edits. The page is treated as hostile: see `build_view` for the layers.

use crate::palette::Palette;
use gtk::{gdk, gio, glib};
use markite_core::blocks::Block;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use webkit::prelude::*;

const PAGE: &str = include_str!("preview.html");
const CSS: &str = include_str!("preview.css");
const SCRIPT: &str = include_str!("preview.js");
/// Name of the script message handler (`window.webkit.messageHandlers.markite`).
const HANDLER: &str = "markite";

pub struct PreviewStyle {
    pub palette: &'static Palette,
    pub font_family: Option<String>,
    pub font_size_pt: Option<f64>,
    pub max_text_width_px: Option<i32>,
}

type ScrolledCallback = Box<dyn Fn(usize, f64)>;

/// Updates not yet sent to the page. Only the newest of each kind matters.
#[derive(Default)]
struct Pending {
    blocks: Option<Vec<String>>,
    css: Option<String>,
    scroll: Option<(usize, f64)>,
    background: Option<gdk::RGBA>,
    flush_scheduled: bool,
}

struct State {
    loaded: Cell<bool>,
    /// The only navigation we allow is the initial `load_html`; this is consumed by it.
    initial_navigation: Cell<bool>,
    block_count: Cell<usize>,
    pending: RefCell<Pending>,
    on_scrolled: RefCell<Option<ScrolledCallback>>,
}

pub struct Preview {
    pub widget: webkit::WebView,
    state: Rc<State>,
}

/// A font family goes into a double-quoted CSS string and comes from saved settings, so only accept names that cannot
/// leave the string or the declaration: letters, digits, spaces and a few punctuation marks.
fn safe_family(name: &str) -> Option<&str> {
    let name = name.trim();
    let ok = !name.is_empty()
        && name.chars().count() <= 100
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '+' | '(' | ')'));
    ok.then_some(name)
}

/// The custom properties the static stylesheet reads. Palette colours are compiled-in constants.
fn style_css(s: &PreviewStyle) -> String {
    let p = s.palette;
    let font = match s.font_family.as_deref().and_then(safe_family) {
        Some(f) => format!("\"{f}\", system-ui, sans-serif"),
        None => "system-ui, sans-serif".to_string(),
    };
    // 4..=200 pt also rejects NaN and infinities.
    let size = match s.font_size_pt {
        Some(pt) if (4.0..=200.0).contains(&pt) => format!("{pt}pt"),
        _ => "16px".to_string(),
    };
    // The 48px is the column's side padding; the setting means the text width itself.
    let maxw = match s.max_text_width_px {
        Some(w) if w > 0 => format!("{}px", i64::from(w) + 48),
        _ => "none".to_string(),
    };
    format!(
        ":root{{--bg:{};--fg:{};--heading:{};--code:{};--code-bg:{};--link:{};--quote:{};--sel:{};--font:{font};--size:{size};--maxw:{maxw}}}",
        p.background, p.text, p.heading, p.code, p.code_background, p.link, p.quote, p.selection
    )
}

impl Preview {
    pub fn new() -> Rc<Preview> {
        let state = Rc::new(State {
            loaded: Cell::new(false),
            initial_navigation: Cell::new(true),
            block_count: Cell::new(0),
            pending: RefCell::default(),
            on_scrolled: RefCell::default(),
        });
        let widget = build_view(&state);
        widget.load_html(&PAGE.replace("@@CSS@@", CSS), None);
        Rc::new(Preview { widget, state })
    }

    /// Replace the page content. Cheap to call on every keystroke: bursts collapse into one update.
    pub fn set_blocks(&self, blocks: &[Block]) {
        self.state.block_count.set(blocks.len());
        self.state.pending.borrow_mut().blocks =
            Some(blocks.iter().map(|b| b.html.clone()).collect());
        self.schedule();
    }

    pub fn set_style(&self, style: &PreviewStyle) {
        // Same colour the page will have, so there is no white flash while it loads or resizes.
        let bg = gdk::RGBA::parse(style.palette.background).ok();
        let mut p = self.state.pending.borrow_mut();
        p.css = Some(style_css(style));
        p.background = bg;
        drop(p);
        self.schedule();
    }

    /// Programmatic scroll: `fraction` (0..1) of the way through block `block`. Out-of-range values are clamped,
    /// NaN counts as 0. The page does not report it back through `connect_scrolled`.
    pub fn scroll_to(&self, block: usize, fraction: f64) {
        let f = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.state.pending.borrow_mut().scroll = Some((block, f));
        self.schedule();
    }

    /// Called with (block index, fraction 0..1) when the USER scrolls.
    pub fn connect_scrolled(&self, f: impl Fn(usize, f64) + 'static) {
        *self.state.on_scrolled.borrow_mut() = Some(Box::new(f));
    }

    fn schedule(&self) {
        let state = self.state.clone();
        if state.pending.borrow().flush_scheduled {
            return;
        }
        state.pending.borrow_mut().flush_scheduled = true;
        let weak = self.widget.downgrade();
        // One flush per main-loop iteration: a burst of calls sends only the last state.
        glib::idle_add_local_once(move || {
            state.pending.borrow_mut().flush_scheduled = false;
            if let Some(web) = weak.upgrade() {
                flush(&web, &state);
            }
        });
    }
}

/// Send everything pending to the page in a single script call. Does nothing before the first load completes;
/// `load_changed` calls it again then.
fn flush(web: &webkit::WebView, state: &State) {
    if !state.loaded.get() {
        return;
    }
    let mut p = state.pending.borrow_mut();
    if let Some(bg) = p.background.take() {
        web.set_background_color(&bg);
    }
    let mut msg = serde_json::Map::new();
    if let Some(css) = p.css.take() {
        msg.insert("css".into(), css.into());
    }
    if let Some(blocks) = p.blocks.take() {
        msg.insert("blocks".into(), blocks.into());
    }
    if let Some((i, f)) = p.scroll.take() {
        msg.insert("scroll".into(), serde_json::json!([i, f]));
    }
    drop(p);
    if msg.is_empty() {
        return;
    }
    // A JSON document is a valid JS expression, so nothing in the text can end the call early.
    let js = format!("window.markite.apply({})", serde_json::Value::Object(msg));
    markite_core::trace!("preview: flush {} bytes of script", js.len());
    web.evaluate_javascript(&js, None, None, gio::Cancellable::NONE, |r| {
        if let Err(_e) = r {
            markite_core::trace!("preview: script failed: {_e}");
        }
    });
}

/// Page -> Rust message `{"t":"scroll","i":N,"f":F}`. Anything else, or a malformed one, is dropped; the index and
/// fraction are clamped because the page is not trusted.
fn parse_scroll(msg: &str, block_count: usize) -> Option<(usize, f64)> {
    let v: serde_json::Value = serde_json::from_str(msg).ok()?;
    if v.get("t")?.as_str()? != "scroll" || block_count == 0 {
        return None;
    }
    let i = usize::try_from(v.get("i")?.as_u64()?)
        .ok()?
        .min(block_count - 1);
    let f = v
        .get("f")?
        .as_f64()
        .filter(|f| f.is_finite())?
        .clamp(0.0, 1.0);
    Some((i, f))
}

/// Open a link in the user's browser / mail client, never in this view.
fn launch(web: &webkit::WebView, uri: &str) {
    markite_core::trace!("preview: opening external link");
    let parent = web.root().and_then(|r| r.downcast::<gtk::Window>().ok());
    gtk::UriLauncher::new(uri).launch(parent.as_ref(), gio::Cancellable::NONE, |r| {
        if let Err(_e) = r {
            markite_core::trace!("preview: launcher failed: {_e}");
        }
    });
}

fn is_external(uri: &str) -> bool {
    let l = uri.to_ascii_lowercase();
    l.starts_with("http://") || l.starts_with("https://") || l.starts_with("mailto:")
}

/// Layers: ephemeral session, locked-down settings, CSP in the page (no script, no remote content except https
/// images), page logic only through a user script, navigation policy (only the initial load), no new windows, and a
/// copy-only context menu.
fn build_view(state: &Rc<State>) -> webkit::WebView {
    let ucm = webkit::UserContentManager::new();
    ucm.add_script(&webkit::UserScript::new(
        SCRIPT,
        webkit::UserContentInjectedFrames::TopFrame,
        webkit::UserScriptInjectionTime::Start,
        &[],
        &[],
    ));
    if !ucm.register_script_message_handler(HANDLER, None) {
        markite_core::trace!("preview: script message handler registration failed");
    }
    let st = state.clone();
    ucm.connect_script_message_received(Some(HANDLER), move |_, v| {
        if let Some((i, f)) = parse_scroll(&v.to_str(), st.block_count.get()) {
            if let Some(cb) = st.on_scrolled.borrow().as_ref() {
                cb(i, f);
            }
        }
    });

    // JavaScript stays on: the user script needs it (the CSP is what keeps page content from running any).
    let settings = webkit::Settings::builder()
        .enable_javascript(true)
        .allow_file_access_from_file_urls(false)
        .allow_universal_access_from_file_urls(false)
        .enable_developer_extras(false)
        .enable_html5_database(false)
        .enable_html5_local_storage(false)
        .enable_webgl(false)
        .enable_media_stream(false)
        .enable_page_cache(false)
        .enable_back_forward_navigation_gestures(false)
        .javascript_can_access_clipboard(false)
        .javascript_can_open_windows_automatically(false)
        .build();
    let web = webkit::WebView::builder()
        .user_content_manager(&ucm)
        .network_session(&webkit::NetworkSession::new_ephemeral())
        .settings(&settings)
        .hexpand(true)
        .vexpand(true)
        .build();

    let st = state.clone();
    web.connect_load_changed(move |web, ev| {
        if ev == webkit::LoadEvent::Finished && !st.loaded.replace(true) {
            markite_core::trace!("preview: page loaded");
            flush(web, &st);
        }
    });
    web.connect_create(|_, _| None::<gtk::Widget>);
    let st = state.clone();
    web.connect_decide_policy(move |web, decision, ty| {
        use webkit::PolicyDecisionType as T;
        if !matches!(ty, T::NavigationAction | T::NewWindowAction) {
            return false;
        }
        let Some(nd) = decision.downcast_ref::<webkit::NavigationPolicyDecision>() else {
            decision.ignore();
            return true;
        };
        let action = nd.navigation_action();
        let uri = action
            .as_ref()
            .and_then(|a| a.request())
            .and_then(|r| r.uri())
            .map(|u| u.to_string())
            .unwrap_or_default();
        let initial = ty == T::NavigationAction
            && uri == "about:blank"
            && action
                .as_ref()
                .is_some_and(|a| a.navigation_type() == webkit::NavigationType::Other)
            && st.initial_navigation.replace(false);
        if initial {
            decision.use_();
            return true;
        }
        decision.ignore();
        if is_external(&uri)
            && action.is_some_and(|a| a.navigation_type() == webkit::NavigationType::LinkClicked)
        {
            launch(web, &uri);
        } else {
            markite_core::trace!("preview: navigation ignored");
        }
        true
    });
    // Keep only the copy entries; with nothing left, no menu at all.
    web.connect_context_menu(|_, menu, _| {
        use webkit::ContextMenuAction as A;
        for item in menu.items() {
            if !matches!(item.stock_action(), A::Copy | A::CopyLinkToClipboard) {
                menu.remove(&item);
            }
        }
        menu.n_items() == 0
    });
    web
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_rejects_breakouts() {
        assert_eq!(safe_family("Open Sans"), Some("Open Sans"));
        for bad in [
            "a\"b", "a\\b", "a;b", "a}b", "a\nb", "a<b", "", "  ", "a/*b",
        ] {
            assert_eq!(safe_family(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn scroll_messages_are_clamped() {
        assert_eq!(
            parse_scroll(r#"{"t":"scroll","i":99,"f":3}"#, 5),
            Some((4, 1.0))
        );
        assert_eq!(
            parse_scroll(r#"{"t":"scroll","i":1,"f":-2}"#, 5),
            Some((1, 0.0))
        );
        assert_eq!(parse_scroll(r#"{"t":"scroll","i":-1,"f":0}"#, 5), None);
        assert_eq!(parse_scroll(r#"{"t":"scroll","i":1,"f":null}"#, 5), None);
        assert_eq!(parse_scroll(r#"{"t":"scroll","i":0,"f":0}"#, 0), None);
        assert_eq!(parse_scroll("garbage", 3), None);
    }
}
