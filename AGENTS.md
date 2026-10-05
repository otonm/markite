# AGENTS.md

**Markite**: native Linux Markdown editor in Rust.

## Architecture (read ARCHITECTURE.md for detail)

- `core/` (`markite-core`): pure Rust, no GUI deps. All logic lives here.
- `kde/` (`markite-kde`): cxx-qt bridge (`kde/src/document.rs`) + Kirigami QML (`kde/src/qml/Main.qml`).
- Rule: new feature = core method + test → one conversion-only `#[qinvokable]`/`#[qproperty]` in the bridge → bind in QML.
  If the bridge needs an `if` that isn't type conversion, it belongs in core.
- The frontend mirrors one `core::Document`; it never owns the source of truth.

## Build

- No cargo/Qt on the host. Use `./docker/build.sh`: core tests as a static musl binary (`rust:alpine`),
  KDE debug build + headless QML smoke test in the Fedora image (`docker/Dockerfile.kde`).
- `./docker/appimage.sh` → `debug-build/markite-x86_64.AppImage`.
- Outputs go to `debug-build/`. Caches live in `.docker-cache/` (both gitignored).

## Hard-won constraints

- cxx-qt links Qt's **private API**: the binary only runs on the exact Qt minor it was built against.
- Do **not** use linuxdeploy: its patchelf/strip corrupt Fedora 44's RELR relocations (segfaults).
  `appimage.sh` copies libs unmodified and sets `LD_LIBRARY_PATH` in `AppRun`.
- AppImage needs glibc ≥ the build image's (Fedora 44 = 2.43). It won't run on Ubuntu 24.04.
- Qt 6.11's Wayland platform plugin is a single `libqwayland.so`.

## QML gotchas already hit

- SplitView: bind `SplitView.preferredWidth` (SplitView overwrites it on drag). Setting it in `Component.onCompleted` gives 0 width.
- Content inside a ScrollView needs explicit `width/height: Math.max(scroll.available*, implicit*)` or it collapses.
- Editor line metrics: use the measured `editor.lineH` / `editor.firstLineY` (via `positionToRectangle`) for gutter, minimap and sync.
- The preview is one TextArea per top-level block (`doc.blocksHtml`). Scroll sync maps line ↔ block via core
  (`doc.lineToBlock`, `doc.blockToLine`). Only user scrolling of the preview (hover/moving) drives the editor.

## Verifying UI changes

A headless smoke run (`QT_QPA_PLATFORM=offscreen`) only proves QML loads. For layout/behaviour, run the app in a throwaway container with Xvfb + xdotool + ImageMagick, drive it, and screenshot. Paste text via `xclip`, then inspect the PNGs.

## Building

When done with the implementation of a fix or a feature, always build an AppImage file in debug-build.

## Honesty

State what was and wasn't tested. Don't present unsourced claims as fact.
