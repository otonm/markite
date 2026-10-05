use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("io.github.otonm.markite").qml_file("src/qml/Main.qml"),
    )
    .files(["src/document.rs"])
    .build();
}
