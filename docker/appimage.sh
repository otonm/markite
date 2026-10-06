#!/bin/sh
# Release build + AppImage, all inside the Fedora image.
#   ./docker/appimage.sh  -> debug-build/markite-x86_64.AppImage
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
sudo docker run --rm -v "$PWD":/src -w /src -e APPIMAGE_EXTRACT_AND_RUN=1 markite-build sh -eu -c '
  cargo build --release -p markite-kde
  C=.docker-cache; A=$C/appimage/AppDir; rm -rf "$C/appimage"; mkdir -p "$A/usr/bin" "$A/usr/lib"
  Q=/usr/lib64/qt6

  cp $C/target-fedora/release/markite "$A/usr/bin/"
  cp -a $Q/qml "$A/usr/qml"
  mkdir -p "$A/usr/plugins"
  for d in $Q/plugins/*; do case ${d##*/} in designer|qmllint|qmlls|qmltooling|sqldrivers) ;; *) cp -a "$d" "$A/usr/plugins/";; esac; done
  # Keep only the KIO workers we actually use (kio-extras installs many, each dragging deps).
  find "$A/usr/plugins/kf6/kio" -name "*.so" -type f \
    ! -name "kio_file.so" ! -name "kio_trash.so" ! -name "kio_http.so" ! -name "kio_ftp.so" ! -name "sftp.so" -delete

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
  ARCH=x86_64 $T --no-appstream "$A" debug-build/markite-x86_64.AppImage >/dev/null
'
sudo chown -R "$(id -u):$(id -g)" debug-build .docker-cache
ls -l debug-build/*.AppImage
