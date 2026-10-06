use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};
use cxx_qt_lib_extras::QApplication;
use std::env;

mod document;
mod kio;

extern "C" {
    fn markite_set_window_icon(); // window_icon.cpp
}

fn main() {
    let mut app = QApplication::new();
    QGuiApplication::set_desktop_file_name(&QString::from("io.github.otonm.markite"));
    unsafe { markite_set_window_icon() };
    if env::var("QT_QUICK_CONTROLS_STYLE").is_err() {
        QQuickStyle::set_style(&QString::from("org.kde.desktop"));
    }

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/io/github/otonm/markite/src/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
