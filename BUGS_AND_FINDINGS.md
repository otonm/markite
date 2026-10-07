# Bugs and findings

Pitfalls already hit and facts established while building Markite. Read before touching the area; add new entries
when you learn something that cost time. (Rules for working on the code live in `AGENTS.md`; the design is in
`ARCHITECTURE.md`.)

## Build and packaging

- cxx-qt links Qt's **private API**: the binary only runs on the exact Qt minor it was built against.
- linuxdeploy corrupts the AppImage: its patchelf/strip break Fedora 44's RELR relocations (segfaults).
  `docker/appimage.sh` copies libs unmodified and sets `LD_LIBRARY_PATH` in `AppRun`.
- The AppImage needs glibc >= the build image's (Fedora 44 = 2.43). It won't run on Ubuntu 24.04.
- Qt 6.11's Wayland platform plugin is a single `libqwayland.so`.
- `libKF6CoreAddons.so` has no second K (link `KF6CoreAddons`, not `KF6KCoreAddons`).
- Docker: keep the big dnf layer cacheable (new packages get their own `RUN`). A failed `--no-cache` build eats disk;
  `docker builder prune --filter until=...` recovers it from the failed attempts.
- `AGENTS.md` has twice turned up zero-filled (0 bytes, mtime 1970), likely pool corruption under disk pressure. If it
  is empty, restore it from git and check what changed recently.
- Release pruning (`LEAN` block in `docker/appimage.sh`) was verified by launching the pruned AppImage under Xvfb and
  opening the Open dialog. Remote (sftp) open/save and a real Wayland session were not checked.

## Remote files (KIO)

- kio-fuse writes (what a plain `open(O_TRUNC)` save does) fail with EIO/EPERM, so remote URLs go through the KIO shim
  (`kde/src/kio_shim.cpp`) instead.
- KF6 renamed kioslave to `kioworker` (`/usr/libexec/kf6/kioworker`, package `kf6-kio-core`, not `-libs`). Without it
  KIO discovers workers but jobs silently hang. The AppImage uses the host's kioworker plus its bundled
  `kf6/kio/sftp.so`, found via `QT_PLUGIN_PATH`.
- Remote KIO can't be tested end to end in a plain container: every DBus-activated KDE daemon (kwalletd6,
  kpasswdserver6) aborts there, and even stock `kioexec` core-dumps on an sftp URL. What was proven instead: the KIO
  round-trip over `file://`, worker discovery/spawn for sftp, and the path-to-URL mapping tests in `core/tests/location.rs`.

## QML gotchas

- In `Controls.Action` handlers read the state via the action's id (`wrapAction.checked`). A bare `checked` triggers a
  deprecated-parameter-injection warning, and a formal `checked =>` parameter receives the signal's argument instead of
  the state (the setting then never flips).
- SplitView: bind `SplitView.preferredWidth` (SplitView overwrites it on drag). Setting it in `Component.onCompleted`
  gives 0 width.
- Content inside a ScrollView needs explicit `width/height: Math.max(scroll.available*, implicit*)` or it collapses.
- Editor line metrics: use the measured `editor.lineH` / `editor.firstLineY` (via `positionToRectangle`) for the gutter,
  minimap and sync.
- Names declared on a nested object (e.g. a property on the `Kirigami.Page`) are not resolvable from its children
  unless the object has an `id`: reference it as `mainPage.prop`.
- `TextEdit.lineCount` counts *visual* lines when wrapping; use source-line data (`editor.lineStarts`) for the gutter.
  `positionToRectangle()` is not reactive: bind on `contentHeight`/`width` too.
- `icon.color` does not recolour plain SVGs: ship a white and a black variant and pick by
  `Kirigami.Theme.textColor.hsvValue`.
- Kirigami's page toolbar is right-aligned; the left-hand toolbar lives in `titleDelegate`, and the menu is a plain
  `Controls.Menu` over shared `Controls.Action`s (they own the shortcuts).
- The preview is one TextArea per top-level block (`doc.blocksHtml`). Scroll sync maps line <-> block via core
  (`doc.lineToBlock`, `doc.blockToLine`). Only user scrolling of the preview (hover/moving) drives the editor.
- Assigning `editor.text` programmatically (e.g. in `openFile`) still fires `onTextChanged`, so `update_text` runs with
  text core already holds. `Document::set_text` reports "unchanged" and the bridge skips the re-render.
- Qt rich text ignores CSS classes, so preview code colours must be inline `style=` spans.
- syntect picks the most specific matching scope selector; on an exact tie the earlier rule wins (it compares with `>`).

## Verifying UI changes

A headless smoke run (`QT_QPA_PLATFORM=offscreen`) only proves the QML loads. For layout and behaviour, run the app in a
throwaway container with Xvfb + xdotool + ImageMagick, drive it, and screenshot. Paste text via `xclip`, then inspect the
PNGs. Without a window manager, `xdotool key` shortcuts did not open dialogs; clicking the menu with `xdotool mousemove`
+ `click` did.

## Known limitations

- With Wrap Text on, the minimap box and scroll sync drift on wrapped lines.
- Closing the app does not warn about unsaved changes.
- `render.options()` sets `sourcepos = true`, so `data-sourcepos` attributes are emitted into the preview HTML, but the
  KDE frontend does not use them (block line ranges come from the parse tree).
- The five theme family names in the QML Theme menu are duplicated by hand from `EditorTheme.families`.
