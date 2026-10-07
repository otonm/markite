#!/bin/sh
# Builds and tests everything inside Docker. Nothing is installed on the host.
#   ./docker/build.sh   # format + lint + core tests, KDE build (with trace) + lint, QML unit tests, headless smoke test
# Outputs land in builds/debug/: markite (KDE app, dynamic Qt).
set -eu
. "$(dirname "$0")/common.sh"
OUT=builds/debug
mkdir -p .docker-cache/cargo .docker-cache/target "$OUT"

echo "== core: format, lint, static musl tests"
run -v "$PWD/.docker-cache/cargo":/usr/local/cargo/registry -e CARGO_TARGET_DIR=/src/.docker-cache/target rust:alpine \
    sh -eu -c '
  apk add -q musl-dev && rustup component add rustfmt clippy >/dev/null
  T="--target x86_64-unknown-linux-musl"
  cargo fmt --check -p markite-core
  for f in "" "--features trace"; do   # with and without tracing: both must stay warning-free and green
    cargo clippy -p markite-core --all-targets $f $T -- -D warnings
    cargo test -p markite-core $f $T
  done'

echo "== kde: build image (cached after first run)"
sudo docker build -q -t markite-build -f docker/Dockerfile.kde docker

echo "== kde: format, lint, build"
run markite-build sh -eu -c '
  cargo fmt --check -p markite-kde
  cargo clippy -p markite-kde --features trace -- -D warnings
  cargo build -p markite-kde --features trace'
cp .docker-cache/target-fedora/debug/markite "$OUT/markite"

echo "== kde: QML unit tests (kde/tests/qml, QtQuickTest)"
run -e QT_QPA_PLATFORM=offscreen markite-build /usr/lib64/qt6/bin/qmltestrunner -input kde/tests/qml

echo "== kde: headless QML smoke test (8s, exit 124 = ran without crashing)"
run -e QT_QPA_PLATFORM=offscreen -e OUT="$OUT" markite-build \
    sh -c 'timeout 8 "./$OUT/markite" >/tmp/log 2>&1; rc=$?; cat /tmp/log; [ $rc = 124 ] && ! grep -qi error /tmp/log && grep -q "\[trace" /tmp/log'
echo "OK -> $OUT/"; ls -l "$OUT"
