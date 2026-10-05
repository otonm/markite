use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("io.github.otonm.markite")
            .qml_file("src/qml/Main.qml")
            .qml_file("src/qml/ViewButton.qml"),
    )
    .qrc("src/icons.qrc")
    .files(["src/document.rs"])
    .build();
}
