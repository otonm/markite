// swift-tools-version:6.2
import PackageDescription

// `scripts/build_core.sh` must run first: it writes the generated UniFFI header into
// Sources/markite_macosFFI/include/, the generated Swift into Sources/MarkiteBridge/, and the Rust static
// library into build/lib/ (all gitignored). The header target gives Swift the C types; the library is linked
// into the app. shortcut: unsafeFlags (fine for a root package, not for a dependency), no XCFramework.
let package = Package(
    name: "Markite",
    platforms: [.macOS(.v26)],
    targets: [
        .target(name: "markite_macosFFI", path: "Sources/markite_macosFFI"),
        .target(name: "MarkiteBridge", dependencies: ["markite_macosFFI"]),
        .executableTarget(
            name: "Markite", dependencies: ["MarkiteBridge"],
            linkerSettings: [.unsafeFlags(["-Lbuild/lib"]), .linkedLibrary("markite_macos")]),
    ],
    swiftLanguageModes: [.v5]
)
