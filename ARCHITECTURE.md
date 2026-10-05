# Architecture

```
core/   markite-core   pure Rust, no toolkit deps, `cargo test` without Qt
kde/    markite-kde    cxx-qt bridge (src/document.rs) + QML (src/qml/) + main.rs
```

## The boundary (the only rule that matters)

`core` exposes plain Rust: structs, methods, `std` types, `io::Result`. It never
imports a GUI crate, never spawns threads, never holds globals. A frontend is a
*mirror*: it owns one `core::Document`, forwards user actions into it, and copies
the resulting state out into whatever its toolkit wants (Q_PROPERTY, GObject
property, iced message, ...).

| Core (toolkit-neutral)             | KDE frontend (Qt-specific)                      |
|------------------------------------|-------------------------------------------------|
| `Document` state, dirty tracking   | `Document` QObject with `text/html/path/dirty`  |
| `render::to_html`                  | preview `TextArea` bound to `doc.html`          |
| `minimap::lines` classification    | `Canvas` painting `doc.minimapRows()`           |
| file I/O (`open/save/save_as`)     | `FileDialog`, `Kirigami.Action`, shortcuts      |
| (later) settings, search, outline  | (later) QML settings page, Kirigami sheets      |

Rules of thumb when adding a feature:
1. Write it in `core` as a method returning data. Add its test there.
2. In `kde/src/document.rs` add one `#[qinvokable]` or `#[qproperty]` that converts
   `String`/`Vec`/`bool` to `QString`/`QList`/`bool`. No logic in the bridge.
3. Bind it in QML.
If step 2 contains an `if` that is not type conversion, it belongs in step 1.

## Why this shape (and not traits / plugins / message bus)
- One concrete `Document` struct is enough. A `trait Frontend` with one implementation
  is a second frontend's problem; add it when that frontend exists.
- Full-buffer `set_text` is fine for Markdown-sized files. Range edits can be added
  to `Document` later without changing the bridge's signature.
- Threading is the frontend's job (cxx-qt `qt_thread`, GTK `glib::spawn`). Core stays
  synchronous so it can be driven from any event loop or from tests.

## Adding a second frontend later
Create `gtk/` (gtk4 + libadwaita + sourceview5) or `tui/`; depend on `markite-core`;
reimplement only the right-hand column of the table above. Core is untouched.

## Build
```
cargo test                       # core only; no Qt needed
cargo run -p markite-kde        # needs Qt6, KF6 Kirigami, KSyntaxHighlighting, qqc2-desktop-style
cmake -B build --install-prefix ~/.local && cmake --build build && cmake --install build
```
Without a host toolchain: `./docker/build.sh` runs core tests as a static musl binary
(`rust:alpine`), builds the KDE crate (`docker/Dockerfile.kde`), and smoke-runs the QML headlessly (offscreen QPA).

`./docker/appimage.sh` makes `debug-build/markite-x86_64.AppImage` (release build, bundles Qt 6.11, KF6 QML modules, qqc2-desktop-style and plasma-integration so it follows Plasma colours/icons/dialogs). Needs glibc >= the build image's (Fedora 44). Libraries are copied unmodified; linuxdeploy is not used because its patchelf corrupts Fedora 44's RELR relocations.
