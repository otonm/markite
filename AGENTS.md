# AGENTS.md

**Markite**: native KDE Plasma Markdown editor in Rust: code editor (line numbers, minimap) + live preview.
Target user system: **Bazzite (Fedora 44, Qt 6.11.2, KDE Plasma)**.

## Architecture (read ARCHITECTURE.md for detail)
- `core/` (`markite-core`): pure Rust, no GUI deps. All logic lives here, with tests.
- `kde/` (`markite-kde`): cxx-qt 0.8 bridge (`kde/src/document.rs`) + Kirigami QML (`kde/src/qml/Main.qml`).
- Rule: new feature = core method + test → one conversion-only `#[qinvokable]`/`#[qproperty]` in the bridge → bind in QML.
  If the bridge needs an `if` that isn't type conversion, it belongs in core.
- The frontend mirrors one `core::Document`; it never owns the source of truth.

## Build (never install packages on the host; everything runs in Docker via passwordless sudo)
- No cargo/Qt on the host. Use `./docker/build.sh`: core tests as a static musl binary (`rust:alpine`),
  KDE debug build + headless QML smoke test in the Fedora image (`docker/Dockerfile.kde`).
- `./docker/appimage.sh` → `debug-build/markite-x86_64.AppImage`.
- Outputs go to `debug-build/`. Caches live in `.docker-cache/` (both gitignored).

## Hard-won constraints
- cxx-qt links Qt's **private API**: the binary only runs on the exact Qt minor it was built against.
  The build image must match the user's Fedora release (`ARG FEDORA` in `docker/Dockerfile.kde`). Bump it when Bazzite moves.
- Do **not** use linuxdeploy: its patchelf/strip corrupt Fedora 44's RELR relocations (segfaults).
  `appimage.sh` copies libs unmodified and sets `LD_LIBRARY_PATH` in `AppRun`.
- AppImage needs glibc ≥ the build image's (Fedora 44 = 2.43). It won't run on Ubuntu 24.04.
- Only the core is static. Fully static Qt/KF6 isn't feasible.
- Qt 6.11's Wayland platform plugin is a single `libqwayland.so`.
- Disk is tight (~1.4 GB free). Don't delete others' Docker images/volumes (`mrr-*`, gradle, etc.); clean only our own artefacts.

## QML gotchas already hit
- SplitView: bind `SplitView.preferredWidth` (SplitView overwrites it on drag). Setting it in `Component.onCompleted` gives 0 width.
- Content inside a ScrollView needs explicit `width/height: Math.max(scroll.available*, implicit*)` or it collapses.
- Editor line metrics: use the measured `editor.lineH` / `editor.firstLineY` (via `positionToRectangle`) for gutter, minimap and sync.
- The preview is one TextArea per top-level block (`doc.blocksHtml`). Scroll sync maps line ↔ block via core
  (`doc.lineToBlock`, `doc.blockToLine`). Only user scrolling of the preview (hover/moving) drives the editor.

## Verifying UI changes
A headless smoke run (`QT_QPA_PLATFORM=offscreen`) only proves QML loads. For layout/behaviour, run the app in a throwaway
container with Xvfb + xdotool + ImageMagick, drive it, and screenshot. Paste text via `xclip`, then inspect the PNGs.
This caught real layout bugs a smoke test missed.

## Honesty
State what was and wasn't tested. Don't present unsourced claims as fact (a wrong "Slint is Qt 5 only" claim slipped into RESEARCH.md once).
