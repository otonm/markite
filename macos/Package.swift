// swift-tools-version:6.2
import PackageDescription

// `scripts/build_core.sh` builds build/markite_macosFFI.xcframework and writes the generated UniFFI
// Swift file into Sources/MarkiteBridge/ (both are gitignored); run it before `swift build`.
let package = Package(
    name: "Markite",
    platforms: [.macOS(.v26)],
    targets: [
        .binaryTarget(name: "markite_macosFFI", path: "build/markite_macosFFI.xcframework"),
        .target(name: "MarkiteBridge", dependencies: ["markite_macosFFI"]),
        .executableTarget(name: "Markite", dependencies: ["MarkiteBridge"]),
    ],
    swiftLanguageModes: [.v5]
)
