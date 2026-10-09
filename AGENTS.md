# AGENTS.md

**Markite**: native Linux Markdown editor in Rust (`core/` pure logic, `kde/` Qt/Kirigami frontend, `gnome/` GTK 4/libadwaita frontend). The two frontends are
separate products: changing one must not touch the other (`gnome/` has its own cargo workspace).
Design: `ARCHITECTURE.md`. Pitfalls and facts already established: `BUGS_AND_FINDINGS.md` (read it before touching
build, KIO or QML code; add to it when you learn something that cost time).

## Behaviour

- State what was and wasn't tested. Don't present unsourced claims as fact; verify third-party behaviour
  (read the source, run it) or leave it out.
- When a fix or feature is done, build the debug AppImage (`./docker/appimage.sh`); for `gnome/` run `./docker/build_gnome.sh`.

## Coding standards

- All logic lives in `core` (pure Rust, no GUI deps). A feature is: core method + test, then one conversion-only
  `#[qinvokable]`/`#[qproperty]` in `kde/src/document.rs`, then bind it in QML (GNOME: call core from `gnome/src/window.rs`). If the bridge needs an `if` that isn't
  type conversion, it belongs in core. The frontend mirrors one `core::Document`; it never owns the truth.
- Validate input where it enters, and never panic on it: paths (NUL bytes, malformed `%` escapes), numbers from QML
  (NaN, negatives, out of `i32`/`u16` range), theme names, and document text (untrusted Markdown: raw HTML and
  `javascript:` links stay neutralised, see `core/src/render.rs`). Return `io::Result`/defaults instead of
  `unwrap`/`expect`/indexing on anything an input can influence.
- FFI/`unsafe`: keep it in `kde/src/kio.rs` and the C++ shim; every `unsafe` block gets a `// SAFETY:` comment; foreign
  buffers are owned by an RAII wrapper (no manual free on some paths); check lengths and null before use; prefer
  `static_cast` and `qsizetype` in C++.
- No duplicated blocks: factor repeated logic into a helper (Rust, QML, shell). Shell scripts share `docker/common.sh`.
- A core mutator a frontend may call redundantly reports whether it changed anything (`Document::set_text` returns
  `bool`) so the bridge can skip work.
- Format and lint must pass (`cargo fmt`, `cargo clippy -D warnings`, with and without the `trace` feature):
  `./docker/build.sh` enforces it. Generated files (`core/src/syntax_themes.rs`, `kde/src/qml/EditorThemes.qml`):
  re-run `kde/tools/gen_editor_themes.py`, never hand-edit.
- Tests: core in `core/tests/*.rs` (one binary per file, public API only); QML logic in `kde/tests/qml/` (QtQuickTest).
  kde has no Rust tests (binary crate): put testable logic in core, or extract it into a Kirigami-free QML component.
- Comments explain *why* and must stay true: when you change code, re-read nearby comments and fix or delete stale ones.
- Comments, docs and commit messages must make sense to someone who has never seen the AI tooling used here: never
  mention assistants, skills or prompts, or use their markers. Mark a deliberate shortcut with a plain
  `Known limitation: <what, and when it matters>` and list it in `BUGS_AND_FINDINGS.md`. Don't reference `AGENTS.md`
  from code, scripts or user docs.
- Tracing: add `markite_core::trace!(...)` lines for new code paths (branches, errors, FFI calls; C++ uses `TRACE`).
  Never log document text or URL passwords (`location::redact`). It prints only with the `trace` feature, which debug
  builds enable and release builds don't.
- UI changes: the headless smoke test only proves the QML loads. Drive the app under Xvfb and look at screenshots
  (recipe in `BUGS_AND_FINDINGS.md`).

## Building

No cargo/Qt on the host; everything runs in Docker. Outputs go to `builds/debug/` and `builds/release/`, caches to
`.docker-cache/` (all gitignored). Never use linuxdeploy (it breaks the AppImage, see `BUGS_AND_FINDINGS.md`).

- `./docker/build.sh`: fmt + clippy + core tests (static musl), KDE build with tracing + clippy, QML unit tests, smoke test.
- `./docker/appimage.sh`: full, unpruned `builds/debug/markite-x86_64.AppImage` with all libraries and `[trace]` output.
- `./docker/build_release.sh <version>`: sets the version everywhere (`docker/set_version.sh`; add any new place that
  carries the version there), builds the lean, trace-free `builds/release/markite-<version>-x86_64.AppImage` + `.sha256`,
  and fails if trace code remains.
- GNOME (`gnome/`, Flatpak only): `./docker/build_gnome.sh` (fmt, clippy with and without `trace`, tests, schema/desktop/
  metainfo validation, headless smoke test), `./docker/build_gnome_flatpak.sh` (bundle in `builds/gnome/`; privileged
  container, runtimes cached in the `markite-flatpak` volume), `./docker/build_gnome_release.sh <version>`. Settings are
  GSettings: a new option means a key in `gnome/data/*.gschema.xml` plus its row in `gnome/src/prefs.rs`. Colours and the
  Markdown language come from `python3 gnome/tools/gen_gnome_data.py`; never hand-edit `gnome/src/palette.rs` or
  `gnome/data/styles/`. Verify UI under Xvfb with `WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1` (containers only).
- What the lean build prunes is the `LEAN` block in `docker/appimage.sh`. Keep it updated: a feature that needs a
  pruned plugin, QML module or library must remove it from the list; one that makes something unnecessary adds to it.
  After editing it, launch the AppImage under Xvfb and check the preview, Open dialog and themes.
