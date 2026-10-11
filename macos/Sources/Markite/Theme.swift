import AppKit
import MarkiteBridge

/// Colours the editor and the preview use. Every theme lookup in the app goes through `Theme.colors`.
struct ThemeColors: Equatable {
    var background: NSColor
    var text: NSColor
    var selection: NSColor
    var lineNumber: NSColor
    var heading: NSColor
    var code: NSColor
    var link: NSColor
    var quote: NSColor
    /// nil = derive it from text and background.
    var codeBackground: NSColor?

    init(from p: EditorPalette) {
        background = .fromHex(p.background)
        text = .fromHex(p.text)
        selection = .fromHex(p.selection, alpha: p.selectionAlpha)
        lineNumber = .fromHex(p.lineNumber)
        heading = .fromHex(p.heading)
        code = .fromHex(p.code)
        link = .fromHex(p.link)
        quote = .fromHex(p.quote)
        codeBackground = .fromHex(p.currentLine, alpha: p.currentLineAlpha)
    }

    init(
        background: NSColor, text: NSColor, selection: NSColor, lineNumber: NSColor, heading: NSColor, code: NSColor,
        link: NSColor, quote: NSColor
    ) {
        self.background = background
        self.text = text
        self.selection = selection
        self.lineNumber = lineNumber
        self.heading = heading
        self.code = code
        self.link = link
        self.quote = quote
    }

    /// System colours, used when a theme name has no palette.
    static let system = ThemeColors(
        background: .textBackgroundColor,
        text: .textColor,
        selection: .selectedTextBackgroundColor,
        lineNumber: .tertiaryLabelColor,
        heading: .labelColor,
        code: .labelColor,
        link: .linkColor,
        quote: .secondaryLabelColor
    )
}

enum Theme {
    /// Colours for a theme family ("Breeze", "Atom One", ...) in its light or dark variant.
    static func colors(family: String, dark: Bool) -> ThemeColors {
        // The only place that reads the generated Palette.swift; unknown names fall back to system colours.
        guard let palette = editorPalette(named: themeName(family: family, dark: dark)) else { return .system }
        return ThemeColors(from: palette)
    }

    /// Whether the given "appearance" setting ("system" / "light" / "dark") currently means dark.
    static func isDark(setting: String) -> Bool {
        switch setting {
        case "dark": return true
        case "light": return false
        default: return NSApp.effectiveAppearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
        }
    }

    /// Makes the whole app follow the setting (nil = follow the system).
    static func applyAppearance(setting: String) {
        switch setting {
        case "dark": NSApp.appearance = NSAppearance(named: .darkAqua)
        case "light": NSApp.appearance = NSAppearance(named: .aqua)
        default: NSApp.appearance = nil
        }
    }
}

extension NSColor {
    /// sRGB colour from 0xRRGGBB.
    static func fromHex(_ rgb: UInt32, alpha: Double = 1) -> NSColor {
        NSColor(
            srgbRed: CGFloat((rgb >> 16) & 0xFF) / 255, green: CGFloat((rgb >> 8) & 0xFF) / 255,
            blue: CGFloat(rgb & 0xFF) / 255, alpha: CGFloat(alpha))
    }

    /// "#RRGGBB" (or "#RRGGBBAA" when translucent) of this colour as it looks in the light or dark appearance (dynamic system colours differ).
    func cssHex(dark: Bool) -> String {
        var result = "#808080"
        let appearance = NSAppearance(named: dark ? .darkAqua : .aqua) ?? NSApp.effectiveAppearance
        appearance.performAsCurrentDrawingAppearance {
            if let c = self.usingColorSpace(.sRGB) {
                let r = Int((c.redComponent * 255).rounded())
                let g = Int((c.greenComponent * 255).rounded())
                let b = Int((c.blueComponent * 255).rounded())
                result = String(format: "#%02X%02X%02X", r, g, b)
                if c.alphaComponent < 0.999 { result += String(format: "%02X", Int((c.alphaComponent * 255).rounded())) }
            }
        }
        return result
    }
}
