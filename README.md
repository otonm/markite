<p align="center"><img src="io.github.otonm.markite.svg" width="96" alt="Markite icon"></p>

# Markite

A small, native Markdown viewer and editor for Linux, written in Rust with Qt 6 and KDE's Kirigami.

## Why another Markdown editor?

There are plenty of Markdown editors. In my experience each one did some things well and lacked
others, and most were far more complex than I needed. I mostly *read* Markdown: READMEs, notes,
documentation. I only occasionally edit it. Markite is built for that: open a file, see it rendered,
fix a typo when needed, and nothing more. There are no projects, plugins or workspaces, just a file
and its preview.

## Features

- **Three view modes**: code only, preview only, or both side by side. Switch with the toolbar buttons or
  `Ctrl+1` / `Ctrl+2` / `Ctrl+3`. Panes slide when you switch, and the last mode is remembered, so a
  reader can simply stay in preview-only.
- **Live preview** of the rendered Markdown (tables, task lists, strikethrough, footnotes, autolinks).
- **Scroll sync** between editor and preview (toggle in the menu, `Ctrl+Shift+L`).
- **Code editor** with Markdown syntax highlighting, line numbers that stay put while you scroll
  horizontally, optional word wrap, and an optional minimap.
- **Editor themes** (Kate themes): Breeze, Atom One, Catppuccin (Latte / Mocha), GitHub and Solarized, each
  with a light and a dark variant. The variant follows your system theme by default; the Theme menu can
  force always light or always dark. The theme applies to the editor, line numbers, minimap and
  the preview.
- **Syntax highlighting in the preview**: fenced code blocks are coloured with the selected theme's token colours
  (the same colours as the editor), for the languages known to syntect and `two-face` (Rust and TOML are
  covered by tests). Unknown languages stay plain.
- **Status bar** (can be switched off in the menu): full path of the open file on the left, word and character
  count (characters without whitespace) on the right. The window title shows just the file name.
- **Opens files from the command line**: `markite notes.md` (a plain path or a `file://` URL).
- **Simple UI**: three-dots menu with Open, Save, Save As, View, Sync Scrolling, Wrap Text, Show Minimap,
  Show Status Bar, About and Exit (`Ctrl+W`).
- Remembers window size and position, plus your view settings (stored in `~/.config/markiterc`).

## Installation

### AppImage (recommended)

Download `markite-<version>-x86_64.AppImage` from the
[latest release](https://github.com/otonm/markite/releases/latest), then:

```sh
chmod +x markite-*-x86_64.AppImage
./markite-*-x86_64.AppImage notes.md
```

Requirements: x86_64 Linux with **glibc 2.43 or newer** (it is built on Fedora 44, so it will not run on
older distributions such as Ubuntu 24.04). Qt and the KDE pieces it needs are bundled. A SHA-256 checksum
is attached to each release.

**Opening `.md` files with Markite:** the desktop entry declares the `text/markdown` type, but an AppImage
cannot register itself. Use a desktop-integration tool such as
[AppImageLauncher](https://github.com/TheAssassin/AppImageLauncher) or appimaged, and "Open with Markite"
will show up for Markdown files. **Re-integrate after upgrading**, so the newest desktop entry (which also
declares the `sftp://` scheme) gets registered.

**Remote files (sftp):** opening and saving over `sftp://` works through KDE's KIO (the sftp worker is
bundled). Paths handed over via kio-fuse (`/run/user/…/kio-fuse…/sftp/…`) are translated back to their
`sftp://` URL automatically. Saving needs the session's KDE services (wallet/credential cache), so run it
inside your normal Plasma session.

### Build from source

You need Rust, Qt 6, and the KDE Frameworks 6 pieces Kirigami, KSyntaxHighlighting and
QQC2DesktopStyle.

```sh
cargo run -p markite-kde                  # run from the source tree
cmake -B build --install-prefix ~/.local \
  && cmake --build build && cmake --install build   # install with desktop entry and icon (needs Corrosion)
```

Without installing anything on the host, the Docker scripts build everything in a Fedora image:

```sh
./docker/build.sh                # fmt + clippy, core tests, KDE build, QML unit tests, smoke test -> builds/debug/
./docker/appimage.sh             # full AppImage -> builds/debug/markite-x86_64.AppImage
./docker/build_release.sh 1.0.3  # sets the version, builds the lean release AppImage -> builds/release/
```

The binary links Qt's private API, so it only runs against the Qt minor version it was built with. The
AppImage bundles that Qt; for a native build this means building on the machine (or distro release) that
will run it.

## Project layout

- `core/`: the Markdown logic in plain Rust with no GUI dependency (document, block splitting and code
  highlighting for the preview, minimap data, counts, remote-path mapping), with integration tests in `core/tests/`.
- `kde/`: the Qt/Kirigami frontend: a thin cxx-qt bridge and the QML UI, with QML unit tests in `kde/tests/qml/`.
- `docker/`: build, test and release scripts; `builds/` (gitignored) holds their output.

## Status

The core has integration tests and the theme logic has QML unit tests; the rest of the UI has been tested by
hand on KDE Plasma (Wayland) and under Xvfb. Known limits are listed in
[BUGS_AND_FINDINGS.md](BUGS_AND_FINDINGS.md).

## License

[MIT](LICENSE). The editor theme colours are taken from KDE's syntax-highlighting themes (also MIT; copyright notices are in `kde/src/qml/EditorThemes.qml`). The AppImage bundles Qt and KDE Frameworks libraries, which keep their own (LGPL) licenses.
