import MarkiteBridge
import SwiftUI

/// UserDefaults keys (shared by `@AppStorage` in the views) and their defaults.
enum Pref {
    static let themeFamily = "themeFamily"
    static let appearance = "appearance"
    static let wrapText = "wrapText"
    static let wrapColumn = "wrapColumn"
    static let codeFontSize = "codeFontSize"
    static let previewFontSize = "previewFontSize"
    static let previewWidth = "previewWidth"

    static let defaultFamily = "Breeze"
    static let defaultAppearance = "system"
}

/// Settings window (stub: the options that already have an effect).
struct SettingsView: View {
    @AppStorage(Pref.themeFamily) private var family = Pref.defaultFamily
    @AppStorage(Pref.appearance) private var appearance = Pref.defaultAppearance
    @AppStorage(Pref.wrapText) private var wrapText = true
    @AppStorage(Pref.wrapColumn) private var wrapColumn = Int(defaultWrapColumn())
    @AppStorage(Pref.codeFontSize) private var codeFontSize = 0
    @AppStorage(Pref.previewFontSize) private var previewFontSize = 0
    @AppStorage(Pref.previewWidth) private var previewWidth = Int(defaultPreviewWidth())

    var body: some View {
        Form {
            Picker("Theme", selection: $family) {
                ForEach(themeFamilies(), id: \.self) { Text($0).tag($0) }
            }
            Picker("Appearance", selection: $appearance) {
                Text("System").tag("system")
                Text("Light").tag("light")
                Text("Dark").tag("dark")
            }
            Toggle("Wrap text in the editor", isOn: $wrapText)
            Stepper("Wrap column: \(wrapColumn)", value: $wrapColumn, in: 60...200)
            Stepper(sizeLabel("Editor font size", codeFontSize), value: $codeFontSize, in: 0...40)
            Stepper(sizeLabel("Preview font size", previewFontSize), value: $previewFontSize, in: 0...40)
            Stepper("Preview width: \(previewWidth)%", value: $previewWidth, in: 20...100, step: 5)
        }
        .padding(20)
        .frame(width: 420)
    }

    /// 0 means "system default size" (see `fontSize(saved:)` in core).
    private func sizeLabel(_ name: String, _ size: Int) -> String {
        size > 0 ? "\(name): \(size) pt" : "\(name): default"
    }
}
