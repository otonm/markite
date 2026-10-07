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

- kio-fuse writes (what a plain `open(O_TRUNC)` save does) fail with EIO/EPERM — remote URLs go through
  the KIO shim instead. `libKF6CoreAddons.so` has no second K (`KF6CoreAddons`, not `KF6KCoreAddons`).
- Remote KIO can't be e2e-tested in a plain container: every DBus-activated KDE daemon (kwalletd6,
  kpasswdserver6) SIGABRTs there — even stock `kioexec` core-dumps on an sftp URL. Proven instead: the KIO
  round-trip over `file://`, worker discovery/spawn for sftp, and unit tests for the path→URL mapping.
- KF6 renamed kioslave→`kioworker` (`/usr/libexec/kf6/kioworker`, package `kf6-kio-core`, not `-libs`):
  without it KIO discovers workers but jobs silently hang. The AppImage uses the host's kioworker plus its
  bundled `kf6/kio/sftp.so` found via `QT_PLUGIN_PATH`.
- Docker: keep the big dnf layer cacheable (new packages get their own `RUN`); a failed `--no-cache`
  build eats disk — `docker builder prune --filter until=…` recovers it from the failed attempts.
- `AGENTS.md` has twice turned up zero-filled (0 bytes, mtime 1970) — likely pool corruption under disk
  pressure; if it's empty, restore from git and check what changed recently.

## QML gotchas already hit

- In `Controls.Action` handlers read the state via the action's id (`wrapAction.checked`): a bare `checked`
  triggers a deprecated-parameter-injection warning, and a formal `checked =>` parameter receives the
  signal's argument instead of the state (the setting then never flips).

- SplitView: bind `SplitView.preferredWidth` (SplitView overwrites it on drag). Setting it in `Component.onCompleted` gives 0 width.
- Content inside a ScrollView needs explicit `width/height: Math.max(scroll.available*, implicit*)` or it collapses.
- Editor line metrics: use the measured `editor.lineH` / `editor.firstLineY` (via `positionToRectangle`) for gutter, minimap and sync.
- Names declared on a nested object (e.g. a property on the `Kirigami.Page`) are NOT resolvable from its children unless the object has an `id`: reference it as `mainPage.prop`.
- `TextEdit.lineCount` counts *visual* lines when wrapping; use source-line data (`editor.lineStarts`) for the gutter. `positionToRectangle()` is not reactive: bind on `contentHeight`/`width` too.
- `icon.color` does not recolour plain SVGs: ship a white and a black variant and pick by `Kirigami.Theme.textColor.hsvValue`.
- Kirigami's page toolbar is right-aligned; the left-hand toolbar lives in `titleDelegate`, and the menu is a plain `Controls.Menu` over shared `Controls.Action`s (they own the shortcuts).
- The preview is one TextArea per top-level block (`doc.blocksHtml`). Scroll sync maps line ↔ block via core
  (`doc.lineToBlock`, `doc.blockToLine`). Only user scrolling of the preview (hover/moving) drives the editor.

## Verifying UI changes

A headless smoke run (`QT_QPA_PLATFORM=offscreen`) only proves QML loads. For layout/behaviour, run the app in a throwaway container with Xvfb + xdotool + ImageMagick, drive it, and screenshot. Paste text via `xclip`, then inspect the PNGs.

## Building

When done with the implementation of a fix or a feature, always build an AppImage file in debug-build.

## Honesty

State what was and wasn't tested. Don't present unsourced claims as fact.
