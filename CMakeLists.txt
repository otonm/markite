cmake_minimum_required(VERSION 3.28)
project(markite VERSION 1.0.7)

find_package(ECM 6.0 REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH})
include(KDEInstallDirs)
include(ECMUninstallTarget)
include(ECMFindQmlModule)
ecm_find_qmlmodule(org.kde.kirigami REQUIRED)
ecm_find_qmlmodule(org.kde.syntaxhighlighting REQUIRED)
find_package(KF6 REQUIRED COMPONENTS QQC2DesktopStyle)
find_package(Corrosion REQUIRED)

corrosion_import_crate(MANIFEST_PATH Cargo.toml CRATES markite-kde)

install(PROGRAMS ${CMAKE_BINARY_DIR}/markite DESTINATION ${KDE_INSTALL_BINDIR})
install(FILES io.github.otonm.markite.desktop DESTINATION ${KDE_INSTALL_APPDIR})
install(FILES io.github.otonm.markite.metainfo.xml DESTINATION ${KDE_INSTALL_METAINFODIR})
install(FILES io.github.otonm.markite.svg DESTINATION ${KDE_INSTALL_FULL_ICONDIR}/hicolor/scalable/apps)
