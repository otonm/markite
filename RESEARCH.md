# Markite: native KDE Markdown editor in Rust — framework research

Date: 2026-10-04. Target: a Rust app that looks and behaves native on Plasma first, GNOME second.
v1 scope: plain code editor (line numbers, code map/minimap) + rendered preview. Theming/tweaks later.

## TL;DR recommendation

**Qt 6 + Kirigami via cxx-qt, with QML UI and Rust logic.** It is the KDE-sanctioned path,
has an official KDE tutorial that is literally a Rust markdown viewer, and on GNOME it
degrades gracefully (qqc2-desktop-style + Breeze colours, portal file dialogs).
The one thing you have to build yourself is the minimap; everything else is off the shelf.

Fallback if you decide GNOME is actually primary: **GTK4 + libadwaita + GtkSourceView 5**
(crates `gtk4`, `libadwaita`, `sourceview5`). It gives you line numbers AND a minimap
(`sourceview5::Map`) with zero custom code, but libadwaita apps ignore the Breeze GTK
theme on Plasma, so they never look native there.

Do not pick a "pure Rust" toolkit (iced, egui, Slint, gpui, Dioxus) for this goal.
None of them renders with the host toolkit's widgets, so none looks native on either desktop.

## Candidate matrix

| Approach | Native on KDE | Native on GNOME | Line numbers | Minimap | Preview | Maturity | Verdict |
|---|---|---|---|---|---|---|---|
| cxx-qt + Kirigami (QML) | Yes (Breeze, qqc2-desktop-style) | OK (Breeze style, needs colour-scheme sync) | DIY, ~40 lines QML | DIY | QML `Text.MarkdownText` or QtWebEngine | cxx-qt 0.8, KDE docs, prod use | **Pick** |
| cxx-qt + QtWidgets + KTextEditor (KatePart) | Yes | OK | Built in | Built in (scrollbar minimap) | QTextBrowser / QtWebEngine | Needs C++ shim, no QML | Strong alt if you want Kate's editor |
| gtk4-rs + libadwaita + sourceview5 | No (ignores Breeze) | Yes | Built in | Built in (`Map`) | WebKitGTK (`webkit6`) or `gtk4cmark-rs` | Very mature, many Rust apps | Pick only if GNOME-first |
| Slint (Qt backend, `native` style) | Partial (Qt widget style, software rendering only) | Partial | DIY | DIY | DIY | Qt backend supports Qt 5.15+ and Qt 6 (since Slint 1.9), software-rendered only | No |
| iced / libcosmic | No | No | DIY | DIY | DIY | Used by System76 COSMIC | No |
| egui / gpui / Dioxus / Tauri | No | No | DIY or webview | DIY | webview | Varies | No |
| Qt Bridges for Rust (`qtbridge` crate, Qt Group) | Yes | OK | DIY | DIY | same as Kirigami | 0.3 beta (Sept 2026), cxx-qt compatible | Watch; not for v1 |

## Option A (recommended): cxx-qt + Kirigami

### What exists already
- KDE tutorial "A full Rust + Kirigami application" builds `simplemdviewer`: Kirigami window,
  QML `TextArea`, a Rust `QObject` with `#[qinvokable] md_format()` using the `markdown` crate,
  plus `.desktop`, AppStream metainfo, icon, and CMake install via Corrosion.
  https://develop.kde.org/docs/getting-started/rust/rust-app/
- `cxx-kde-frameworks` (KDE-maintained) adds Rust bindings for KCrash, KCoreAddons (KAboutData),
  KI18n (translations). Drop-in crate alongside cxx-qt.
  https://develop.kde.org/docs/getting-started/rust/rust-bindings/
- Minimal template: https://github.com/antroids/cxx-qt-kirigami
- Qt Group's `qtbridge` 0.3 beta is explicitly cxx-qt compatible, so you can migrate later.

### Dependencies (Cargo.toml, from the KDE tutorial)
```toml
[dependencies]
cxx = "1"
cxx-qt = "0.8"
cxx-qt-lib = { version = "0.8", features = ["qt_full"] }
cxx-qt-lib-extras = "0.8"
cxx-kde-frameworks = "*"      # KCrash, KAboutData, KI18n
comrak = "0.54"               # or pulldown-cmark; see Markdown section

[build-dependencies]
cxx-qt-build = { version = "0.8", features = ["link_qt_object_files"] }
```
System: Qt 6 (QtQuick, QuickControls2), KF6 Kirigami, qqc2-desktop-style, KSyntaxHighlighting,
extra-cmake-modules, CMake >= 3.28, Corrosion.

### Editor widget (line numbers + highlighting)
- Editor = `QtQuick.Controls.TextArea` inside a `Flickable` (or Kirigami's `ScrollablePage`).
- Syntax highlighting: `org.kde.syntaxhighlighting` QML module, part of KF6.
  ```qml
  import org.kde.syntaxhighlighting
  SyntaxHighlighter { textEdit: editor; definition: "Markdown"; theme: Repository.theme("Breeze Dark") }
  ```
  300+ languages, same engine as Kate, theme follows Breeze light/dark via `Repository`.
  https://api.kde.org/qml-org-kde-syntaxhighlighting-syntaxhighlighter.html
- Line numbers: no built-in gutter in QML. Standard approach is a `Column`/`ListView` beside the
  `TextArea` driven by `editor.lineCount` and `FontMetrics.height` (monospace, no wrap) or by
  `TextEdit.positionToRectangle(...)` when wrapping is on. ~40 lines of QML.
- Monospace font: `Kirigami.Theme.fixedWidthFont` (follows system "Fixed width" setting).

### Code map / minimap
- Not available in QML. Build it as a Rust `QQuickPaintedItem`-style element or a plain QML
  `Canvas` that paints one 1-2px line per text line, coloured by the highlighter's token type,
  with a draggable viewport rectangle bound to `flickable.contentY`. Start with a `Canvas` and a
  `Repeater` over lines; optimise only if large files feel slow.
- Alternative with zero custom code: Option B (KatePart) ships a scrollbar minimap.

### Preview
Two tiers:
1. v1: `Text { textFormat: Text.MarkdownText }` or `TextEdit` read-only. Qt >= 5.14 renders
   CommonMark + GitHub dialect natively, including task-list checkboxes. No web engine, zero
   extra deps, follows Kirigami colours automatically. Limits: Qt's own Markdown parser,
   limited HTML subset, no CSS theming beyond fonts/colours.
2. Later: `QtWebEngine` `WebEngineView` fed HTML from `comrak` + your CSS. Full fidelity,
   themeable with CSS, but ~100 MB dependency and heavier startup. Kate's markdown preview and
   KDE's Marknote use this route.
Run the Rust parser in a `qinvokable` so the UI stays declarative; debounce on `textChanged`.

### Desktop integration you get for free on Plasma
- Breeze widgets via `QQuickStyle::set_style("org.kde.desktop")` (already in the tutorial).
- Window icon on Wayland via `QGuiApplication::set_desktop_file_name`.
- Native KDE file dialogs (`FileDialog` from `QtQuick.Dialogs` uses the platform theme).
- Global shortcuts, menus: `Kirigami.Action` + `Kirigami.GlobalDrawer` / `Kirigami.ActionToolBar`.
- Settings: `KConfig` is not in cxx-kde-frameworks yet; use `QSettings` via cxx-qt-lib or
  Rust-side TOML in `$XDG_CONFIG_HOME` (dirs crate). Keep it simple.
- Crash dialog (DrKonqi): `KCrash::initialize()` from cxx-kde-frameworks.

### What it looks like on GNOME
- Kirigami uses qqc2-desktop-style + Breeze there too. Readable and consistent, but visibly a
  Qt app (like Kdenlive or Krita on GNOME).
- Known issue: dark/light does not follow GNOME automatically. The fix used by Kirigami apps
  (e.g. Bazzite Updater) is to read `org.freedesktop.appearance color-scheme` from the settings
  portal (`ashpd::desktop::settings::Settings`) when `XDG_CURRENT_DESKTOP != KDE` and apply the
  Breeze Light/Dark colour scheme. Flatpak users get `qgnomeplatform`/`xdg-desktop-portal-gnome`
  dialogs automatically.

### Build and packaging
- Dev loop: `cargo run`. Install: CMake wrapper with `corrosion_import_crate` installing the
  binary, `.desktop`, metainfo, icon (tutorial has the full `CMakeLists.txt`).
- Flatpak: KDE runtime `org.kde.Platform//6.x` + `org.kde.Sdk` + `org.freedesktop.Sdk.Extension.rust-stable`.
  Flathub requires offline cargo sources (`flatpak-cargo-generator.py`).

## Option B: QtWidgets + KTextEditor (KatePart) via a C++ shim
- KTextEditor is the editor used by Kate/KDevelop/Kile. Gives line numbers, folding,
  scrollbar minimap (`scrollbar-minimap` config key), multi-cursor, 300+ highlighters, vi mode.
- Not QML-embeddable (it is a QWidget). You would use the KDE "Using Rust together with C++"
  pattern: C++ `main.cpp` + `KParts::MainWindow` hosting `KTextEditor::View`, with Rust as a
  static library doing markdown conversion and file logic.
  https://develop.kde.org/docs/getting-started/rust/rust-mixed/
- Preview: `QTextBrowser::setMarkdown()` (Qt native) or `QWebEngineView`.
- Trade: most editor features for the least Rust in the UI layer. A C++ reference app with
  exactly this shape is Tilde (Qt 6 + KatePart markdown/LaTeX editor): https://github.com/funnym0th/Tilde
- Looks native on KDE, less so on GNOME than Kirigami (QWidgets vs GNOME HIG spacing).

## Option C: GTK4 + libadwaita + GtkSourceView 5 (GNOME-first)
- Crates: `gtk4`, `libadwaita` (as `adw`), `sourceview5`, `webkit6` (optional), `ashpd`.
- `sourceview5::View` with `set_show_line_numbers(true)`; `sourceview5::Map` with
  `set_view(&view)` is a ready-made minimap. `LanguageManager` has a markdown language,
  `StyleSchemeManager` has Adwaita/Adwaita-dark schemes that follow `adw::StyleManager`.
- Preview: `webkit6::WebView` (full HTML/CSS) or the small `gtk4cmark-rs` widget for native rendering.
- Existing Rust examples: `gnacho/scribe` (GTK4 + libadwaita + WebKitGTK, pre-alpha), `Ranrar/Marco`.
- The disqualifier for your stated priority: libadwaita hard-codes its stylesheet. Breeze GTK
  does not theme libadwaita apps, and KDE devs say it will not. You get accent/dark-mode
  via portal, but not Breeze widgets, so it never looks like a Plasma app.
- relm4 is an Elm-style layer over gtk4-rs if you want less imperative code; optional.

## Markdown parsing crates (Rust side, all options)
| Crate | Model | GFM | Notes |
|---|---|---|---|
| `comrak` | AST, CommonMark + GFM, many extensions (footnotes, math, wikilinks, front matter) | Yes (opt-in) | Safest default; AST lets you do source-line mapping for scroll sync |
| `pulldown-cmark` | Streaming events, fast, small | Yes | Great if you only emit HTML; events carry byte offsets |
| `markdown` (markdown-rs) | AST + events | Yes | Used by the KDE tutorial; slower than the others |
| `rushdown` | AST, fastest in its own benchmarks | Yes | Young; verify maintenance before adopting |
Recommendation: `comrak` with `extension.table/strikethrough/tasklist/autolink` on, and
`render.sourcepos` on for future scroll-sync between editor and preview.

## Theming & tweakability later (design for it now, do not build it)
- Keep UI in QML files loaded as a Qt resource; theming becomes "ship extra QML/CSS", not a rewrite.
- Highlighter themes come from KSyntaxHighlighting `Repository.themes` for free (user-droppable
  `.theme` JSON files in `~/.local/share/org.kde.syntax-highlighting/themes/`).
- Preview themes: once on QtWebEngine, themes are CSS files in a config dir.
- Expose settings as `#[qproperty]` on one Rust `Settings` QObject; QML binds to it.

## Risks
- cxx-qt is pre-1.0 (0.8); API churn between minor versions. Pin it.
- Build complexity: Qt + KF6 dev packages + CMake + Corrosion; CI needs a KDE image
  (`invent.kde.org` CI templates or `kdeorg/ci-suse-qt6*` containers).
- QML `TextArea` with very large files (tens of thousands of lines) is slow; KTextEditor or
  GtkSourceView handle that better. Fine for Markdown documents.
- GNOME users will still see a Qt app. If that becomes unacceptable, the only real fix is a
  second GTK front-end sharing the Rust core, which is why the core should be a UI-free crate
  from day one.

## Suggested v1 layout
```
markite/
├── Cargo.toml            # workspace
├── core/                 # pure Rust: markdown -> html, file io, settings (no Qt)
├── app/                  # cxx-qt bridge + QML
│   ├── build.rs
│   ├── src/main.rs
│   ├── src/document.rs   # QObject: text, path, dirty, render_html()
│   └── src/qml/Main.qml  # Kirigami.ApplicationWindow, SplitView: editor | preview
├── CMakeLists.txt        # install .desktop, metainfo, icon
└── io.github.otonm.markite.{desktop,metainfo.xml,svg}
```

## Sources
- KDE Rust + Kirigami tutorial: https://develop.kde.org/docs/getting-started/rust/rust-app/
- KDE Rust + C++ mixed: https://develop.kde.org/docs/getting-started/rust/rust-mixed/
- KDE Frameworks Rust bindings: https://develop.kde.org/docs/getting-started/rust/rust-bindings/
- cxx-qt book: https://kdab.github.io/cxx-qt/book/
- cxx-kde-frameworks: https://github.com/KDE/cxx-kde-frameworks
- KSyntaxHighlighting QML: https://api.kde.org/qml-org-kde-syntaxhighlighting-syntaxhighlighter.html
- QML Markdown rendering: https://doc.qt.io/qt-6/qml-qtquick-text.html (textFormat MarkdownText)
- Qt Bridges for Rust 0.3 beta: https://www.qt.io/blog/qt-bridge-for-rust-0.3-cxx-qt-compatibility-and-soundness
- KTextEditor API / minimap: https://api.kde.org/ktexteditor-view.html
- sourceview5 Map: https://world.pages.gitlab.gnome.org/Rust/sourceview5-rs/stable/latest/docs/sourceview5/struct.Map.html
- gtk4-rs libadwaita chapter: https://gtk-rs.org/gtk4-rs/git/book/libadwaita.html
- Breeze GTK vs libadwaita: https://discuss.kde.org/t/gtk4-apps-don-t-follow-breeze-theme/2969
- Kirigami cross-desktop theming: https://discuss.kde.org/t/making-kde-apps-that-theme-cross-desktop/50230
- Qt/KDE platform integration explained: https://nicolasfella.de/posts/how-platform-integration-works/
- ashpd (portals): https://docs.rs/ashpd
- Slint Qt backend: https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backend_qt
- Rust GUI survey 2026: https://lobste.rs/s/83yugk/2026_survey_rust_gui_libraries
- comrak: https://github.com/kivikakk/comrak
