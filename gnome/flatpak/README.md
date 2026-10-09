# Flatpak packaging (GNOME frontend)

Build: `./docker/build_gnome_flatpak.sh` (bundle in `builds/gnome/`), release: `./docker/build_gnome_release.sh <version>`.
`cargo-sources.json` (the vendored crate list, built offline) is generated from `gnome/Cargo.lock` by the build script
and is git-ignored.

## Permissions (`finish-args`) and why

| Argument | Why |
|---|---|
| `--socket=wayland`, `--socket=fallback-x11` | Show the window (X11 only when there is no Wayland). |
| `--share=ipc` | Needed by X11 shared memory; harmless on Wayland. |
| `--device=dri` | GPU rendering for GTK and WebKit (the preview). |
| `--talk-name=org.gtk.vfs.*` | Talk to the host GVfs daemon, so `sftp://` (and other GVfs) locations can be opened, saved and monitored. |
| `--filesystem=xdg-run/gvfsd` | The GVfs FUSE/socket directory in `$XDG_RUNTIME_DIR`; GVfs clients in the sandbox need it. |

Deliberately absent: `--filesystem=home` or any other broad file access. Local files reach the app through the
file chooser and drag and drop (document portal), and through `%U` on the command line.

SFTP: the GVfs daemon runs on the host, so it uses the host's `ssh` and keys; nothing under `~/.ssh` has to be exposed
to the sandbox, and password prompts come back through GIO's mount operation. This follows the permissions other
Flathub apps use for GVfs; it was NOT tested against a real SFTP server (no server or host GVfs in the build container).

Not requested: network (the preview is local and its content security policy blocks remote loads), notifications,
secret service.

## Fonts and licences

The ten font families are downloaded at build time from pinned URLs with sha256 checksums (see the `fonts` module) into
`/app/share/fonts/markite/`; their licence texts go to `/app/share/licenses/io.github.otonm.markite/`. Open Sans and
Roboto are taken from upstream (SIL OFL), so their licence texts are the upstream ones, not the copies in `kde/fonts`.
