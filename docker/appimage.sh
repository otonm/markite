#!/bin/sh
# Builds the AppImage inside the Fedora image.
#   ./docker/appimage.sh  -> builds/debug/markite-x86_64.AppImage (full, with [trace] output)
#   LEAN=1 OUT=path ./docker/appimage.sh   -> pruned build (use docker/build_release.sh)
#
# Why not linuxdeploy: its bundled patchelf/strip predate RELR relocations, which Fedora 44
# uses everywhere. Rewriting RUNPATH leaves .relr.dyn stale and every patched .so segfaults
# in its constructors. We copy libraries unmodified and set LD_LIBRARY_PATH in AppRun instead.
#
# glibc: the AppImage needs the build distro's glibc or newer (Fedora 44 = 2.43). Fine for
# Bazzite on F44; older distros need an image built on an older base.
set -eu
. "$(dirname "$0")/common.sh"
sudo docker build -q -t markite-build -f docker/Dockerfile.kde docker >/dev/null
LEAN=${LEAN:-}; OUT=${OUT:-builds/debug/markite-x86_64.AppImage}
mkdir -p "$(dirname "$OUT")"
run -e APPIMAGE_EXTRACT_AND_RUN=1 -e LEAN="$LEAN" -e OUT="$OUT" markite-build sh -eu -c '
  if [ -n "$LEAN" ]; then
    # Lean: size-optimised, stripped Rust binary in its own target dir (so normal builds keep their cache); no trace code.
    export CARGO_TARGET_DIR=/src/.docker-cache/target-fedora-lean CARGO_PROFILE_RELEASE_STRIP=true \
           CARGO_PROFILE_RELEASE_LTO=thin CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_OPT_LEVEL=s
    cargo build --release -p markite-kde
  else
    cargo build --release -p markite-kde --features trace   # prints [trace ...] lines to stderr
  fi
  C=.docker-cache; A=$C/appimage/AppDir; rm -rf "$C/appimage"; mkdir -p "$A/usr/bin" "$A/usr/lib"
  Q=/usr/lib64/qt6

  cp "$CARGO_TARGET_DIR/release/markite" "$A/usr/bin/"
  cp -a $Q/qml "$A/usr/qml"
  mkdir -p "$A/usr/plugins"
  for d in $Q/plugins/*; do case ${d##*/} in designer|qmllint|qmlls|qmltooling|sqldrivers) ;; *) cp -a "$d" "$A/usr/plugins/";; esac; done
  # Keep only the KIO workers we actually use (kio-extras installs many, each dragging deps).
  find "$A/usr/plugins/kf6/kio" -name "*.so" -type f \
    ! -name "kio_file.so" ! -name "kio_trash.so" ! -name "kio_http.so" ! -name "kio_ftp.so" ! -name "sftp.so" -delete

  if [ -n "$LEAN" ]; then
    # LEAN: drop what Markite never loads. Must run BEFORE the ldd scan below, so the libraries these
    # pull in are not bundled either. Keep this list in step with new features (see ARCHITECTURE.md, "Build"); after
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

  # Fonts for the Code Font and Preview Font menus (kde/src/app_init.cpp registers this directory at startup): regular, bold and
  # italics only. Fira Code, JetBrains Mono and Cascadia Code come from packages in the build image; Monaspace Neon
  # is a pinned download, verified by checksum. Licences (SIL OFL) travel with the fonts.
  FD=$A/usr/share/markite/fonts; mkdir -p "$FD" "$A/usr/share/licenses/markite-fonts"
  find /usr/share/fonts/fira-code /usr/share/fonts/jetbrains-mono-fonts /usr/share/fonts/cascadia-code-fonts \
    \( -name "*-Regular.*" -o -name "*-Bold.*" -o -name "*-Italic.*" -o -name "*-BoldItalic.*" \) -exec cp {} "$FD/" \;
  # Download once into the cache, verifying a pinned checksum: fetch <file> <url> <sha256>.
  fetch() {
    if [ ! -f "$C/$1" ]; then
      curl -fsSL -o "$C/$1.part" "$2"
      echo "$3  $C/$1.part" | sha256sum -c - || { rm -f "$C/$1.part"; exit 1; }
      mv "$C/$1.part" "$C/$1"
    fi
  }
  fetch monaspace-static-v1.400.zip https://github.com/githubnext/monaspace/releases/download/v1.400/monaspace-static-v1.400.zip \
    ab66d71be751495f679727332a3345597943bd4d7beebca03f5cde04bf994de7
  for w in Regular Bold Italic BoldItalic; do unzip -qjo "$C/monaspace-static-v1.400.zip" "Static Fonts/Monaspace Neon/MonaspaceNeon-$w.otf" -d "$FD"; done

  # Preview fonts (sans: Inter, Open Sans, Roboto; serif: Merriweather, Lora, Source Serif 4): the same four styles.
  # The first three and Merriweather are packages in the build image; Lora and Source Serif 4 are pinned downloads.
  pick() { for w in Regular Bold Italic BoldItalic; do cp "$1/$2-$w."* "$FD/"; done; }
  pick /usr/share/fonts/rsms-inter-fonts Inter
  pick /usr/share/fonts/open-sans OpenSans
  pick /usr/share/fonts/google-roboto Roboto
  pick /usr/share/fonts/sorkintype-merriweather-fonts Merriweather
  fetch lora-v3.021.zip https://github.com/cyrealtype/Lora-Cyrillic/releases/download/v3.021/Lora.zip \
    19061972d1124d258dffd41f3ad12ce2db513f9b31fb98ece7afe7b538e8647f
  for w in Regular Bold Italic BoldItalic; do unzip -qjo "$C/lora-v3.021.zip" "ttf/Lora-$w.ttf" -d "$FD"; done
  fetch source-serif-4.005.zip https://github.com/adobe-fonts/source-serif/releases/download/4.005R/source-serif-4.005_Desktop.zip \
    549fdb8f9a682bd06944298621404969f6de77c2e422ff3b8244a1dcd6a0c425
  for w in Regular Bold It BoldIt; do unzip -qjo "$C/source-serif-4.005.zip" "source-serif-4.005_Desktop/TTF/SourceSerif4-$w.ttf" -d "$FD"; done
  cp kde/fonts/LICENSE-*.txt "$A/usr/share/licenses/markite-fonts/"

  # Pinned release, verified by checksum (the tool is executed).
  T=$C/appimagetool-1.9.1-x86_64.AppImage
  if [ ! -x "$T" ]; then
    curl -fsSL -o "$T.part" https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage
    echo "ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0  $T.part" | sha256sum -c - || { rm -f "$T.part"; exit 1; }
    chmod +x "$T.part"; mv "$T.part" "$T"
  fi
  ARCH=x86_64 $T --no-appstream "$A" "$OUT" >/dev/null
'
ls -l "$OUT"
