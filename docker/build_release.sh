#!/bin/sh
# Lean release build: ./docker/build_release.sh 1.0.3   (also sets the project version, see set_version.sh)
#   -> builds/release/markite-1.0.3-x86_64.AppImage (+ .sha256)
# Same AppImage as appimage.sh but size-optimised and with everything Markite never loads pruned
# (the prune list lives in appimage.sh under `LEAN`). When a feature adds a plugin/QML module/library,
# check it is not pruned; when it makes something unnecessary, add it to the list. Update ARCHITECTURE.md ("Build") too.
set -eu
cd "$(dirname "$0")/.."
V=${1:?usage: build_release.sh <version>  e.g. 1.0.3}
./docker/set_version.sh "$V"   # Cargo.toml x2, Cargo.lock, CMakeLists.txt, metainfo

OUT=builds/release/markite-$V-x86_64.AppImage
LEAN=1 OUT=$OUT ./docker/appimage.sh

echo "== smoke test: launch the AppImage offscreen (8s, exit 124 = ran without crashing)"
sudo docker run --rm -v "$PWD":/src -w /src -e QT_QPA_PLATFORM=offscreen -e APPIMAGE_EXTRACT_AND_RUN=1 markite-build \
    sh -c 'timeout 8 ./'"$OUT"' >/tmp/log 2>&1; rc=$?; cat /tmp/log; [ $rc = 124 ] && ! grep -qiE "error|module .* is not installed|cannot load|\[trace" /tmp/log'

echo "== release must contain no trace code"
# Distinctive strings from trace messages in core, the bridge and the KIO shim, plus the emit() prefix.
for m in "[trace " "bridge rerender" "to_blocks: parsing" "kio::read" "[trace kio_shim]"; do
  ! grep -aqF "$m" .docker-cache/appimage/AppDir/usr/bin/markite || { echo "trace string '$m' found in the release binary" >&2; exit 1; }
done

(cd builds/release && sha256sum "markite-$V-x86_64.AppImage" > "markite-$V-x86_64.AppImage.sha256")
echo "OK -> $OUT"; ls -l "$OUT"*
