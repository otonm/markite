#!/bin/sh
# Sets the project version everywhere it lives: ./docker/set_version.sh 1.0.3
# (Cargo.toml x2, Cargo.lock, CMakeLists.txt, metainfo release entry). Idempotent.
# Add new places that carry the version here.
set -eu
cd "$(dirname "$0")/.."
V=${1:?usage: set_version.sh <version>  e.g. 1.0.3}
echo "$V" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || { echo "version must look like 1.2.3" >&2; exit 1; }

for f in core/Cargo.toml kde/Cargo.toml; do
  sed -i '0,/^version = ".*"/s//version = "'"$V"'"/' "$f" # first match = [package] version
done
for p in markite-core markite-kde; do
  sed -i '/^name = "'"$p"'"$/{n;s/^version = ".*"/version = "'"$V"'"/}' Cargo.lock
done
sed -i 's/^project(markite VERSION [0-9.]*)/project(markite VERSION '"$V"')/' CMakeLists.txt
M=io.github.otonm.markite.metainfo.xml
grep -q "<release version=\"$V\"" "$M" || sed -i 's#<releases>#<releases><release version="'"$V"'" date="'"$(date +%F)"'"/>#' "$M"
echo "version set to $V"
