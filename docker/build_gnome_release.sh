#!/bin/sh
# GNOME Flatpak release: ./docker/build_gnome_release.sh 1.0.3   (also sets the project version, see set_version.sh)
#   -> builds/gnome/markite-1.0.3-x86_64.flatpak (+ .sha256), and nothing else in builds/gnome.
set -eu
. "$(dirname "$0")/common.sh"
V=${1:?usage: build_gnome_release.sh <version>  e.g. 1.0.3}
./docker/set_version.sh "$V"

echo "== release must contain only the app: no download or build leftovers in the data directory"
! find gnome/data \( -name "*.zip" -o -name "*.part" -o -name "*.rs" -o -name "*.o" -o -name "*.orig" \) | grep -q . \
  || { echo "stray files in gnome/data" >&2; exit 1; }

# builds/gnome holds only the version being released.
find builds/gnome -mindepth 1 -maxdepth 1 -exec rm -rf {} + 2>/dev/null || true
./docker/build_gnome_flatpak.sh

echo "== release must contain no trace code (the Flatpak builds without the trace feature)"
sudo docker run --rm -v markite-flatpak:/root/.local markite-flatpak-build sh -eu -c '
  F=/root/.local/markite/build/files
  ! grep -aqF "[trace " $F/bin/markite-gnome || { echo "trace strings in the release binary" >&2; exit 1; }
  ! find $F \( -name "*.zip" -o -name "*.part" -o -name "*.rs" -o -name "*.o" \) | grep -q . || { echo "stray files in the app" >&2; exit 1; }'

find builds/gnome -mindepth 1 ! -name "markite-$V-x86_64.flatpak*" -exec rm -rf {} + 2>/dev/null || true
echo "OK"; ls -l builds/gnome
