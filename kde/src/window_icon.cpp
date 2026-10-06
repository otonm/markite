// cxx-qt-lib has no QIcon, and Qt does not derive the window icon from the desktop file name
// on X11 or for an unintegrated AppImage, so set it explicitly from the bundled resource.
#include <QGuiApplication>
#include <QIcon>

extern "C" void markite_set_window_icon() {
    QGuiApplication::setWindowIcon(QIcon(QStringLiteral(":/icons/app.svg")));
}
