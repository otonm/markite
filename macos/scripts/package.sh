#!/usr/bin/env bash
# Builds Markite.app (Apple silicon, ad-hoc signed) and a drag-to-Applications DMG in macos/dist/.
# Run macos/scripts/build_core.sh first. Not notarized: on first launch use right-click > Open.
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="$(sed -n 's/^version = "\(.*\)"   # set with.*/\1/p' Cargo.toml)"
[ -n "$VERSION" ] || { echo "cannot read version from Cargo.toml" >&2; exit 1; }

swift build -c release --arch arm64
BIN="$(swift build -c release --arch arm64 --show-bin-path)/Markite"

APP=dist/Markite.app
rm -rf dist && mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" dist/dmg
cp "$BIN" "$APP/Contents/MacOS/Markite"
macos/scripts/make_icon.sh "$APP/Contents/Resources/Markite.icns"
sed "s/@VERSION@/$VERSION/g" Resources/Info.plist.in >"$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist"
# Apple silicon refuses unsigned code; ad-hoc is enough to run locally and on machines you trust.
codesign --force --sign - "$APP"
codesign --verify --strict "$APP"

ditto "$APP" dist/dmg/Markite.app
ln -s /Applications dist/dmg/Applications
# makehybrid + convert avoids `hdiutil create -srcfolder`, which is flaky on CI runners.
hdiutil makehybrid -hfs -hfs-volume-name Markite -hfs-openfolder dist/dmg -o dist/raw.dmg dist/dmg
hdiutil convert dist/raw.dmg -format UDZO -o "dist/markite-$VERSION-macos-arm64.dmg"
rm -rf dist/raw.dmg dist/dmg
ls -lh dist
