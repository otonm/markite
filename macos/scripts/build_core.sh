#!/usr/bin/env bash
# Builds the Rust core for Apple silicon and wraps it for SwiftPM (macOS only; run from anywhere):
#   macos/build/markite_macosFFI.xcframework   static lib + C header + modulemap
#   macos/Sources/MarkiteBridge/*.swift      generated UniFFI bindings
set -euo pipefail
cd "$(dirname "$0")/.."
TARGET=aarch64-apple-darwin
export MACOSX_DEPLOYMENT_TARGET=26.0
FEATURES=()
[ "${TRACE:-0}" = 1 ] && FEATURES=(--features trace)

cargo build --release --target "$TARGET" ${FEATURES[@]+"${FEATURES[@]}"}
LIB="target/$TARGET/release/libmarkite_macos.a"

rm -rf build Sources/MarkiteBridge
mkdir -p build/headers Sources/MarkiteBridge
BINDGEN=(cargo run --release --bin uniffi-bindgen-swift --)
"${BINDGEN[@]}" "$LIB" build/headers --headers
"${BINDGEN[@]}" "$LIB" build/headers --modulemap --modulemap-filename module.modulemap
"${BINDGEN[@]}" "$LIB" Sources/MarkiteBridge --swift-sources
xcodebuild -create-xcframework -library "$LIB" -headers build/headers -output build/markite_macosFFI.xcframework
echo "==> core ready: macos/build/markite_macosFFI.xcframework"
