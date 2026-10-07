use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};
use cxx_qt_lib_extras::QApplication;
use std::env;
use std::ffi::c_char;

mod document;
mod kio;

extern "C" {
    fn markite_init_app(version: *const c_char); // app_init.cpp
}

/// NUL-terminated crate version for the C++ side.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");

fn main() {
    markite_core::trace!(
        "main: Markite {} starting (trace build)",
        VERSION.trim_end_matches('\0')
    );
    let mut app = QApplication::new();
    QGuiApplication::set_desktop_file_name(&QString::from("io.github.otonm.markite"));
    // SAFETY: `VERSION` is a NUL-terminated `'static` string (see its definition) that outlives the call.
    unsafe { markite_init_app(VERSION.as_ptr().cast()) };
    if env::var("QT_QUICK_CONTROLS_STYLE").is_err() {
        markite_core::trace!("main: QT_QUICK_CONTROLS_STYLE unset -> forcing org.kde.desktop");
        QQuickStyle::set_style(&QString::from("org.kde.desktop"));
    } else {
        markite_core::trace!("main: QT_QUICK_CONTROLS_STYLE set by the environment, leaving it");
    }

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        markite_core::trace!("main: loading Main.qml from the qrc");
        engine.load(&QUrl::from(
            "qrc:/qt/qml/io/github/otonm/markite/src/qml/Main.qml",
        ));
    }
    if let Some(app) = app.as_mut() {
        markite_core::trace!("main: entering the Qt event loop");
        app.exec();
        markite_core::trace!("main: event loop ended, exiting");
    }
}
