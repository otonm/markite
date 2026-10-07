use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    println!("cargo::rerun-if-changed=../io.github.otonm.markite.svg"); // bundled via icons.qrc
    let builder = CxxQtBuilder::new_qml_module(
        QmlModule::new("io.github.otonm.markite")
            .qml_file("src/qml/Main.qml")
            .qml_file("src/qml/ViewButton.qml")
            .qml_file("src/qml/EditorThemes.qml")
            .qml_file("src/qml/EditorTheme.qml"),
    )
    .qrc("src/icons.qrc")
    .cpp_file("src/app_init.cpp")
    .cpp_file("src/kio_shim.cpp");
    let trace = std::env::var_os("CARGO_FEATURE_TRACE").is_some(); // `--features trace`
    // KIO shim includes (Fedora layout; the build image installs kf6-kio-devel).
    let builder = unsafe {
        builder.cc_builder(move |cc| {
            if trace {
                cc.define("MARKITE_TRACE", None);
            }
            cc.include("/usr/include/KF6/KIOCore").include("/usr/include/KF6/KIO").include("/usr/include/KF6/KCoreAddons");
        })
    };
    builder.files(["src/document.rs"]).build();
    // kio_shim.cpp links against KIO (Fedora system path).
    println!("cargo::rustc-link-lib=dylib=KF6KIOCore");
    println!("cargo::rustc-link-lib=dylib=KF6CoreAddons"); // note: KCoreAddons = libKF6CoreAddons, no extra K
}
