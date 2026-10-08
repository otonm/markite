// App-level Qt setup that cxx-qt-lib has no bindings for, called once from main.rs.
// - window icon: Qt does not derive it from the desktop file name on X11 or for an unintegrated
//   AppImage, and cxx-qt-lib has no QIcon, so set it from the bundled resource.
// - bundled fonts: the AppImage ships the code fonts in <prefix>/share/markite/fonts; register them so QML can use
//   them by family name without installing anything.
// - version: shown in the About dialog via Qt.application.version.
#include <QCoreApplication>
#include <QDir>
#include <QFontDatabase>
#include <QGuiApplication>
#include <QIcon>
#include "trace.h"

static void load_bundled_fonts()
{
    const QDir dir(QCoreApplication::applicationDirPath() + QStringLiteral("/../share/markite/fonts"));
    const auto files = dir.entryList({QStringLiteral("*.ttf"), QStringLiteral("*.otf")}, QDir::Files);
    int loaded = 0;
    for (const QString &file : files) {
        if (QFontDatabase::addApplicationFont(dir.filePath(file)) >= 0) {
            ++loaded;
        }
    }
    TRACE("fonts: %d of %lld files loaded from %s", loaded, static_cast<long long>(files.size()),
          dir.path().toUtf8().constData());
}

extern "C" void markite_init_app(const char *version) {
    QGuiApplication::setApplicationVersion(QString::fromUtf8(version));
    load_bundled_fonts();
    QGuiApplication::setWindowIcon(QIcon(QStringLiteral(":/icons/app.svg")));
}
