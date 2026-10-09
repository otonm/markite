#!/bin/sh
# Builds and tests the GNOME (GTK 4 + libadwaita) frontend inside Docker. Nothing is installed on the host.
#   ./docker/build_gnome.sh   # format + lint (with/without trace), build, tests, data validation, headless smoke test
# Output: builds/gnome/markite-gnome (debug build, with trace). The Flatpak is built by build_gnome_flatpak.sh.
set -eu
. "$(dirname "$0")/common.sh"
OUT=builds/gnome
D=gnome/data
mkdir -p .docker-cache/cargo-gnome .docker-cache/target-gnome "$OUT"

echo "== gnome: build image (cached after first run)"
sudo docker build -q -t markite-gnome -f docker/Dockerfile.gnome docker

echo "== gnome: format, lint, build, tests"
run markite-gnome sh -eu -c '
  cd gnome
  cargo fmt --check
  for f in "" "--features trace"; do   # with and without tracing: both must stay warning-free
    cargo clippy --locked --all-targets $f -- -D warnings
  done
  cargo build --locked --features trace
  cargo test --locked'
cp .docker-cache/target-gnome/debug/markite-gnome "$OUT/markite-gnome"

echo "== gnome: schema, desktop file, metainfo"
# The compiled schema goes to a git-ignored directory: the app looks for it in <data dir>/schemas when the schema
# is not installed system-wide (settings.rs). Run as the calling user so the file is not root-owned.
run -u "$(id -u):$(id -g)" markite-gnome sh -eu -c '
  mkdir -p '$D'/schemas
  glib-compile-schemas --strict '$D' --targetdir='$D'/schemas
  desktop-file-validate '$D'/io.github.otonm.markite.desktop
  appstreamcli validate --pedantic '$D'/io.github.otonm.markite.metainfo.xml'

echo "== gnome: headless smoke test (8s, exit 124 = ran without crashing)"
# Software GL in Xvfb prints harmless MESA-EGL DRI3 warnings (filtered). No sandbox/accessibility bus in a container; the in-memory GSettings backend keeps the run from writing anywhere.
run -e OUT="$OUT" markite-gnome sh -c '
  Xvfb :99 -screen 0 1280x800x24 >/dev/null 2>&1 & sleep 1
  export DISPLAY=:99 GDK_BACKEND=x11 GTK_A11Y=none GSETTINGS_BACKEND=memory WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 MARKITE_DATA_DIR=/src/'$D'
  dbus-run-session -- timeout 8 "./$OUT/markite-gnome" >/tmp/log 2>&1; rc=$?
  cat /tmp/log; [ $rc = 124 ] && ! grep -v "^MESA-EGL" /tmp/log | grep -qiE "critical|error"'
echo "OK -> $OUT/"; ls -l "$OUT"
