import MarkiteBridge
import SwiftUI

/// The content of one document window: editor and/or preview, with the status bar underneath. The view mode picker
/// lives in the window toolbar (and in the View menu).
struct ContentView: View {
    @ObservedObject var document: MarkiteDocument
    @AppStorage(Pref.themeFamily) private var family = Pref.defaultFamily
    @AppStorage(Pref.appearance) private var appearance = Pref.defaultAppearance
    @AppStorage(Pref.wrapText) private var wrapText = true
    @AppStorage(Pref.wrapColumn) private var wrapColumn = Int(defaultWrapColumn())
    @AppStorage(Pref.codeFontSize) private var codeFontSize = 0
    @AppStorage(Pref.previewFontSize) private var previewFontSize = 0
    @AppStorage(Pref.previewWidth) private var previewWidth = Int(defaultPreviewWidth())

    var body: some View {
        let dark = Theme.isDark(setting: appearance)
        let colors = Theme.colors(family: family, dark: dark)
        VStack(spacing: 0) {
            // Both panes always exist so switching mode keeps the cursor and scroll position; a hidden pane is
            // collapsed to zero width. shortcut: a collapsed pane still lays out (and a thin divider shows), upgrade
            // if that costs too much on huge documents.
            let showEditor = document.viewMode != .preview
            let showPreview = document.viewMode != .editor
            HSplitView {
                editor(colors)
                    .frame(minWidth: showEditor ? 200 : 0, maxWidth: showEditor ? .infinity : 0)
                    .allowsHitTesting(showEditor)
                    .accessibilityHidden(!showEditor)
                preview(colors, dark: dark)
                    .frame(minWidth: showPreview ? 200 : 0, maxWidth: showPreview ? .infinity : 0)
                    .allowsHitTesting(showPreview)
                    .accessibilityHidden(!showPreview)
            }
            Divider()
            StatusBar(status: document.status, notice: document.notice)
        }
        .frame(minWidth: 480, minHeight: 320)
        .toolbar {
            ToolbarItem(placement: .principal) {
                Picker("View", selection: $document.viewMode) {
                    ForEach(ViewMode.allCases) { Text($0.title).tag($0) }
                }
                .pickerStyle(.segmented)
                .help("Show the editor, both, or the preview")
            }
        }
        .onAppear { Theme.applyAppearance(setting: appearance) }
        .onChange(of: appearance) { _, new in Theme.applyAppearance(setting: new) }
    }

    private func editor(_ colors: ThemeColors) -> some View {
        EditorView(
            document: document, generation: document.editorGeneration, colors: colors,
            fontSize: sizePoints(codeFontSize), wrap: wrapText, wrapColumn: Int(clampWrapColumn(v: Int32(clamping: wrapColumn))))
    }

    private func preview(_ colors: ThemeColors, dark: Bool) -> some View {
        let width = Int(clampPreviewWidth(v: Int32(clamping: previewWidth)))
        let pt = sizePoints(previewFontSize)
        return PreviewView(
            document: document,
            themeName: themeName(family: family, dark: dark),
            style: PreviewStyle.make(
                colors: colors, dark: dark, fontPt: pt > 0 ? pt : nil, widthPercent: width))
    }

    /// Saved size in points after core's validation; 0 = system default.
    private func sizePoints(_ saved: Int) -> Double {
        Double(fontSize(saved: Int32(clamping: saved)) ?? 0)
    }
}

/// Path, encoding, line ending, counts; plus a transient notice (external change) on the left.
struct StatusBar: View {
    let status: DocumentStatus
    let notice: String?

    var body: some View {
        HStack(spacing: 16) {
            Text(notice ?? status.path ?? "Untitled")
                .lineLimit(1)
                .truncationMode(.middle)
            Spacer()
            Text(status.encoding)
            Text(status.lineEnding)
            Text("\(status.words) words, \(status.chars) characters")
        }
        .font(.caption)
        .foregroundStyle(.secondary)
        .padding(.horizontal, 10)
        .padding(.vertical, 4)
        .accessibilityElement(children: .combine)
    }
}
