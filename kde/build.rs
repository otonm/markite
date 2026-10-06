use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    println!("cargo::rerun-if-changed=../io.github.otonm.markite.svg"); // bundled via icons.qrc
    CxxQtBuilder::new_qml_module(
        QmlModule::new("io.github.otonm.markite")
            .qml_file("src/qml/Main.qml")
            .qml_file("src/qml/ViewButton.qml")
            .qml_file("src/qml/EditorThemes.qml"),
    )
    .qrc("src/icons.qrc")
    .cpp_file("src/window_icon.cpp")
    .files(["src/document.rs"])
    .build();
}
