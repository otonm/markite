#!/bin/sh
# Builds the GNOME frontend as a Flatpak bundle inside a privileged Docker container (CI-friendly).
#   ./docker/build_gnome_flatpak.sh   -> builds/gnome/markite-<version>-x86_64.flatpak (+ .sha256)
# The version comes from gnome/Cargo.toml (docker/set_version.sh sets it).
#
# Disk: the named volume `markite-flatpak` holds the GNOME runtime/SDK (~3 GB), flatpak-builder's download and
# build cache and the OSTree repo, so repeat builds are fast. Remove it with `docker volume rm markite-flatpak`.
# The vendored crate list gnome/flatpak/cargo-sources.json is generated here from gnome/Cargo.lock (not committed).
set -eu
. "$(dirname "$0")/common.sh"
V=$(sed -n 's/^version = "\(.*\)".*/\1/p' gnome/Cargo.toml | head -1)
[ -n "$V" ] || { echo "no version in gnome/Cargo.toml" >&2; exit 1; }
APP=io.github.otonm.markite
OUT=builds/gnome/markite-$V-x86_64.flatpak
C=.docker-cache; mkdir -p "$C" builds/gnome

echo "== flatpak: build image (cached after first run)"
sudo docker build -q -t markite-flatpak-build -f docker/Dockerfile.flatpak docker

# Pinned generator (it is executed), verified by checksum like appimagetool in appimage.sh.
GEN_COMMIT=74697c75b630d7330e77250fc13cb5ea688d9479
GEN=$C/flatpak-cargo-generator-$GEN_COMMIT.py
if [ ! -f "$GEN" ]; then
  curl -fsSL -o "$GEN.part" "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/$GEN_COMMIT/cargo/flatpak-cargo-generator.py"
  echo "0a2db6be87d75910facef28ab46d4d6460802e8419ab850d0caa6a364d26b380  $GEN.part" | sha256sum -c - || { rm -f "$GEN.part"; exit 1; }
  mv "$GEN.part" "$GEN"
fi

fp() { run --privileged -v markite-flatpak:/root/.local -e HOME=/root -e V="$V" -e APP="$APP" -e OUT="$OUT" -e GEN="$GEN" markite-flatpak-build "$@"; }

echo "== flatpak: vendored crates from gnome/Cargo.lock, validate data"
fp sh -eu -c '
  python3 "$GEN" gnome/Cargo.lock -o gnome/flatpak/cargo-sources.json
  desktop-file-validate gnome/data/$APP.desktop
  appstreamcli validate --pedantic gnome/data/$APP.metainfo.xml'

sudo chown "$(id -u):$(id -g)" gnome/flatpak/cargo-sources.json

echo "== flatpak: build (first run downloads ~3 GB of runtimes into the markite-flatpak volume)"
fp sh -eu -c '
  flatpak --user remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
  flatpak --user install -y --noninteractive flathub org.gnome.Platform//51 org.gnome.Sdk//51 \
      org.freedesktop.Sdk.Extension.rust-stable//26.08
  L=/root/.local/markite
  flatpak-builder --user --disable-rofiles-fuse --force-clean --state-dir=$L/state --repo=$L/repo $L/build \
      gnome/flatpak/$APP.json
  rm -f "$OUT"
  flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo $L/repo "$OUT" $APP'

echo "== flatpak: smoke test (install the bundle, inspect the installed layout)"
fp sh -eu -c '
  flatpak --user uninstall -y --noninteractive $APP >/dev/null 2>&1 || true   # the volume outlives runs
  flatpak --user install -y --noninteractive --bundle "$OUT"
  flatpak run --command=sh $APP -c "
    set -eu
    test -x /app/bin/markite-gnome
    ls /app/share/markite; ls /app/share/licenses/'$APP'
    n=\$(fc-list : file | grep -c /app/share/fonts/markite/); echo \"bundled font files seen by fontconfig: \$n (expect 38)\"; [ \$n = 38 ]
    for f in \"Fira Code\" \"JetBrains Mono\" \"Cascadia Code\" \"Monaspace Neon\" Inter \"Open Sans\" Roboto Merriweather Lora \"Source Serif 4\"; do
      fc-list : family | grep -qx \"\$f\" || fc-list : family | grep -q \"^\$f\" || { echo \"missing font family: \$f\"; exit 1; }
    done
    ls /app/share/glib-2.0/schemas/gschemas.compiled /app/share/applications/$APP.desktop \
       /app/share/metainfo/$APP.metainfo.xml /app/share/icons/hicolor/scalable/apps/$APP.svg"
  grep -H "^Exec" /root/.local/share/flatpak/exports/share/applications/$APP.desktop
  # Start it for 8 s under Xvfb. The WebKit own sandbox needs the flatpak-spawn portal, which cannot work in a
  # container-in-container (the app aborts with "failed to receive credentials"), so it is disabled here only.
  Xvfb :99 -screen 0 1280x800x24 >/dev/null 2>&1 & sleep 1
  export DISPLAY=:99
  dbus-run-session -- timeout 8 flatpak run --env=WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 --env=GSETTINGS_BACKEND=memory \
      --env=GTK_A11Y=none $APP >/tmp/log 2>&1 && rc=0 || rc=$?
  cat /tmp/log; [ $rc = 124 ] && ! grep -v "^MESA-EGL\|p11-kit" /tmp/log | grep -qiE "critical|error"'

sudo chown "$(id -u):$(id -g)" "$OUT"
(cd builds/gnome && sha256sum "markite-$V-x86_64.flatpak" > "markite-$V-x86_64.flatpak.sha256")
echo "OK -> $OUT"; ls -l builds/gnome
