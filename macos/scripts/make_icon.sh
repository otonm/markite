#!/usr/bin/env bash
# Renders the app icon (Markite.icns) from the project's SVG. Needs rsvg-convert (brew install librsvg)
# and iconutil (macOS). Usage: make_icon.sh <out.icns>
set -euo pipefail
OUT="$(cd "$(dirname "${1:?usage: make_icon.sh <out.icns>}")" && pwd)/$(basename "$1")"
cd "$(dirname "$0")/../.."
SET="$(mktemp -d)/Markite.iconset"
mkdir -p "$SET"
for s in 16 32 128 256 512; do
  rsvg-convert -w "$s" -h "$s" io.github.otonm.markite.svg -o "$SET/icon_${s}x${s}.png"
  rsvg-convert -w "$((s * 2))" -h "$((s * 2))" io.github.otonm.markite.svg -o "$SET/icon_${s}x${s}@2x.png"
done
iconutil -c icns "$SET" -o "$OUT"
