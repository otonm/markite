// App-level Qt setup that cxx-qt-lib has no bindings for, called once from main.rs.
// - window icon: Qt does not derive it from the desktop file name on X11 or for an unintegrated
//   AppImage, and cxx-qt-lib has no QIcon, so set it from the bundled resource.
// - version: shown in the About dialog via Qt.application.version.
#include <QGuiApplication>
#include <QIcon>

extern "C" void markite_init_app(const char *version) {
    QGuiApplication::setApplicationVersion(QString::fromUtf8(version));
    QGuiApplication::setWindowIcon(QIcon(QStringLiteral(":/icons/app.svg")));
}
