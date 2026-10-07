#!/bin/sh
# Release build + AppImage, all inside the Fedora image.
#   ./docker/appimage.sh  -> builds/debug/markite-x86_64.AppImage
#   LEAN=1 OUT=path ./docker/appimage.sh   -> pruned build (use docker/build_release.sh)
#
# Why not linuxdeploy: its bundled patchelf/strip predate RELR relocations, which Fedora 44
# uses everywhere. Rewriting RUNPATH leaves .relr.dyn stale and every patched .so segfaults
# in its constructors. We copy libraries unmodified and set LD_LIBRARY_PATH in AppRun instead.
#
# glibc: the AppImage needs the build distro's glibc or newer (Fedora 44 = 2.43). Fine for
# Bazzite on F44; older distros need an image built on an older base.
set -eu
cd "$(dirname "$0")/.."
sudo docker build -q -t markite-build -f docker/Dockerfile.kde docker >/dev/null
LEAN=${LEAN:-}; OUT=${OUT:-builds/debug/markite-x86_64.AppImage}
mkdir -p "$(dirname "$OUT")"
# Lean: size-optimised, stripped Rust binary in its own target dir (so normal builds keep their cache).
if [ -n "$LEAN" ]; then
  TD=/src/.docker-cache/target-fedora-lean
  LEAN_ENV="-e CARGO_TARGET_DIR=$TD -e CARGO_PROFILE_RELEASE_STRIP=true -e CARGO_PROFILE_RELEASE_LTO=thin -e CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 -e CARGO_PROFILE_RELEASE_OPT_LEVEL=s"
else
  TD=/src/.docker-cache/target-fedora; LEAN_ENV=""
fi
sudo docker run --rm -v "$PWD":/src -w /src -e APPIMAGE_EXTRACT_AND_RUN=1 -e LEAN="$LEAN" -e OUT="$OUT" -e TD="$TD" $LEAN_ENV markite-build sh -eu -c '
  cargo build --release -p markite-kde
  C=.docker-cache; A=$C/appimage/AppDir; rm -rf "$C/appimage"; mkdir -p "$A/usr/bin" "$A/usr/lib"
  Q=/usr/lib64/qt6

  cp "$TD/release/markite" "$A/usr/bin/"
  cp -a $Q/qml "$A/usr/qml"
  mkdir -p "$A/usr/plugins"
  for d in $Q/plugins/*; do case ${d##*/} in designer|qmllint|qmlls|qmltooling|sqldrivers) ;; *) cp -a "$d" "$A/usr/plugins/";; esac; done
  # Keep only the KIO workers we actually use (kio-extras installs many, each dragging deps).
  find "$A/usr/plugins/kf6/kio" -name "*.so" -type f \
    ! -name "kio_file.so" ! -name "kio_trash.so" ! -name "kio_http.so" ! -name "kio_ftp.so" ! -name "sftp.so" -delete

  if [ -n "$LEAN" ]; then
    # LEAN: drop what Markite never loads. Must run BEFORE the ldd scan below, so the libraries these
    # pull in are not bundled either. Keep this list in step with new features (see AGENTS.md); after
    # changing it, launch the AppImage and check the preview, dialogs, themes and remote open still work.
    P=$A/usr/plugins; Y=$A/usr/qml
    rm -f  "$P/platformthemes/libqgtk3.so"                                  # GTK3 + pango/cairo/pixbuf/glycin
    rm -rf "$P/kf6/thumbcreator" "$P/kf6/kfileitemaction" "$P/kf6/kauth"    # OpenEXR, exiv2, ...
    for st in FluentWinUI3 Material Universal Imagine Fusion; do          # QQC2 styles we never select (org.kde.desktop + Basic stay)
      rm -rf "$Y/QtQuick/Controls/$st"; find "$Y" -type d -name "+$st" -prune -exec rm -rf {} +
    done
    rm -rf "$Y/QtQuick/Controls/designer"
    rm -rf "$Y/Qt5Compat" "$Y/QtWayland"                                    # GraphicalEffects (ShaderTools, SPIRV) / Wayland compositor API
    rm -rf "$Y/QtTest" "$Y/QtQuick/tooling"; find "$Y" -name "*.qmltypes" -delete
  fi

  # Bundle every shared-lib dependency except the ones that must come from the host
  # (glibc, libstdc++/libgcc, GL/EGL/Vulkan drivers, X11/xcb, wayland, fonts): AppImage excludelist policy.
  skip="^(ld-linux|libc|libm|libdl|libpthread|librt|libresolv|libutil|libstdc\+\+|libgcc_s|libGL|libGLX|libGLdispatch|libOpenGL|libEGL|libgbm|libdrm|libvulkan|libX|libxcb|libxkbcommon-x11|libwayland-|libfontconfig|libfreetype|libharfbuzz|libz|libexpat|libuuid|libSM|libICE|libasound|libpipewire)\."
  find "$A" -type f \( -name "*.so*" -o -path "*/usr/bin/*" \) -exec ldd {} + 2>/dev/null \
    | awk "/=> \\//{print \$3}" | sort -u | while read -r lib; do
      n=${lib##*/}; echo "$n" | grep -qE "$skip" || cp -Ln "$lib" "$A/usr/lib/" 2>/dev/null || true
    done

  cat > "$A/AppRun" <<"EOF"
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
export LD_LIBRARY_PATH="$HERE/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export QT_PLUGIN_PATH="$HERE/usr/plugins"
export QML_IMPORT_PATH="$HERE/usr/qml" QML2_IMPORT_PATH="$HERE/usr/qml"
exec "$HERE/usr/bin/markite" "$@"
EOF
  chmod +x "$A/AppRun"
  cp io.github.otonm.markite.desktop io.github.otonm.markite.svg "$A/"
  ln -s io.github.otonm.markite.svg "$A/.DirIcon"
  mkdir -p "$A/usr/share/icons/hicolor/scalable/apps"; cp io.github.otonm.markite.svg "$A/usr/share/icons/hicolor/scalable/apps/"
  mkdir -p "$A/usr/share/metainfo"; cp io.github.otonm.markite.metainfo.xml "$A/usr/share/metainfo/"

  T=$C/appimagetool-x86_64.AppImage
  [ -x $T ] || { curl -sSL -o $T https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage; chmod +x $T; }
  ARCH=x86_64 $T --no-appstream "$A" "$OUT" >/dev/null
'
sudo chown -R "$(id -u):$(id -g)" builds .docker-cache
ls -l "$OUT"
