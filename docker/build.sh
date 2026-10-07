#!/bin/sh
# Builds and tests everything inside Docker. Nothing is installed on the host.
#   ./docker/build.sh          # core static tests, KDE build (with trace), QML unit tests, headless smoke test
# Outputs land in builds/debug/: markite (KDE app, dynamic Qt).
set -eu
cd "$(dirname "$0")/.."
D="sudo docker"
OUT=builds/debug
mkdir -p .docker-cache/cargo .docker-cache/target "$OUT"

echo "== core: static musl tests"
$D run --rm -v "$PWD":/src -w /src \
    -v "$PWD/.docker-cache/cargo":/usr/local/cargo/registry \
    -e CARGO_TARGET_DIR=/src/.docker-cache/target rust:alpine \
    sh -c 'apk add -q musl-dev && cargo test -p markite-core --target x86_64-unknown-linux-musl \
            && cargo test -p markite-core --features trace --target x86_64-unknown-linux-musl'

echo "== kde: build image (cached after first run)"
$D build -q -t markite-build -f docker/Dockerfile.kde docker

echo "== kde: build"
$D run --rm -v "$PWD":/src -w /src markite-build cargo test -q -p markite-kde --features trace --bin markite
$D run --rm -v "$PWD":/src -w /src markite-build cargo build -p markite-kde --features trace
cp .docker-cache/target-fedora/debug/markite "$OUT/markite"

echo "== kde: QML unit tests (kde/tests/qml, QtQuickTest)"
$D run --rm -v "$PWD":/src -w /src -e QT_QPA_PLATFORM=offscreen markite-build \
    /usr/lib64/qt6/bin/qmltestrunner -input kde/tests/qml

echo "== kde: headless QML smoke test (8s, exit 124 = ran without crashing)"
$D run --rm -v "$PWD":/src -w /src -e QT_QPA_PLATFORM=offscreen markite-build \
    sh -c 'timeout 8 ./'"$OUT"'/markite >/tmp/log 2>&1; rc=$?; cat /tmp/log; [ $rc = 124 ] && ! grep -qi error /tmp/log && grep -q "\[trace" /tmp/log'
sudo chown -R "$(id -u):$(id -g)" "$OUT" .docker-cache
echo "OK -> $OUT/"; ls -l "$OUT"
