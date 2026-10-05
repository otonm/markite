#!/bin/sh
# Builds and tests everything inside Docker. Nothing is installed on the host.
#   ./docker/build.sh          # core static tests + KDE build + headless QML smoke test
# Outputs land in debug-build/: markite (KDE app, dynamic Qt) and markite_core-tests (static musl).
set -eu
cd "$(dirname "$0")/.."
D="sudo docker"
OUT=debug-build
mkdir -p .docker-cache/cargo .docker-cache/target "$OUT"

echo "== core: static musl tests"
$D run --rm -v "$PWD":/src -w /src \
    -v "$PWD/.docker-cache/cargo":/usr/local/cargo/registry \
    -e CARGO_TARGET_DIR=/src/.docker-cache/target rust:alpine \
    sh -c 'apk add -q musl-dev && cargo test -p markite-core --target x86_64-unknown-linux-musl \
            && cp "$(ls -t .docker-cache/target/x86_64-unknown-linux-musl/debug/deps/markite_core-* | grep -v "\.d$" | head -1)" '"$OUT"'/markite_core-tests'

echo "== kde: build image (cached after first run)"
$D build -q -t markite-build -f docker/Dockerfile.kde docker

echo "== kde: build"
$D run --rm -v "$PWD":/src -w /src markite-build cargo build -p markite-kde
cp .docker-cache/target-fedora/debug/markite "$OUT/markite"

echo "== kde: headless QML smoke test (8s, exit 124 = ran without crashing)"
$D run --rm -v "$PWD":/src -w /src -e QT_QPA_PLATFORM=offscreen markite-build \
    sh -c 'timeout 8 ./'"$OUT"'/markite >/tmp/log 2>&1; rc=$?; cat /tmp/log; [ $rc = 124 ] && ! grep -qi error /tmp/log'
sudo chown -R "$(id -u):$(id -g)" "$OUT" .docker-cache
echo "OK -> $OUT/"; ls -l "$OUT"
