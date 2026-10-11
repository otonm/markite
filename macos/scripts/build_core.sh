#!/usr/bin/env bash
# Builds the Rust core for Apple silicon and wraps it for SwiftPM (macOS only; run from anywhere):
#   macos/build/lib/libmarkite_macos.a                 static library
#   macos/Sources/markite_macosFFI/include/*.h         generated C header
#   macos/Sources/MarkiteBridge/*.swift                generated UniFFI bindings
set -euo pipefail
cd "$(dirname "$0")/.."
TARGET=aarch64-apple-darwin
export MACOSX_DEPLOYMENT_TARGET=26.0
FEATURES=()
[ "${TRACE:-0}" = 1 ] && FEATURES=(--features trace)

cargo build --release --target "$TARGET" ${FEATURES[@]+"${FEATURES[@]}"}
LIB="target/$TARGET/release/libmarkite_macos.a"

rm -rf build Sources/MarkiteBridge Sources/markite_macosFFI/include
mkdir -p build/lib Sources/MarkiteBridge Sources/markite_macosFFI/include
BINDGEN=(cargo run --release --bin uniffi-bindgen-swift --)
# Header only: SwiftPM synthesises the module map for the C target.
"${BINDGEN[@]}" "$LIB" Sources/markite_macosFFI/include --headers
"${BINDGEN[@]}" "$LIB" Sources/MarkiteBridge --swift-sources
cp "$LIB" build/lib/
echo "==> core ready: macos/build/lib, generated header and Swift sources"
