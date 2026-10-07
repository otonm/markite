import QtQuick

// Editor theme resolution: a family has a light and a dark Kate theme; the variant follows the system
// palette unless `appearance` forces one. Themed: editor, gutter, minimap and preview.
QtObject {
    property string family: "Breeze"
    property string appearance: "system"   // "system" | "light" | "dark"
    property bool systemDark: false        // the frontend's palette says dark

    readonly property EditorThemes data: EditorThemes {}
    readonly property var families: [
        { name: "Breeze", light: "Breeze Light", dark: "Breeze Dark" },
        { name: "Atom One", light: "Atom One Light", dark: "Atom One Dark" },
        { name: "Catppuccin", light: "Catppuccin Latte", dark: "Catppuccin Mocha" },
        { name: "GitHub", light: "GitHub Light", dark: "GitHub Dark" },
        { name: "Solarized", light: "Solarized Light", dark: "Solarized Dark" }
    ]
    // Anything but "light"/"dark" (e.g. a hand-edited rc file) follows the system, like "system".
    readonly property bool dark: appearance === "dark" || (appearance !== "light" && systemDark)
    // Also the name core uses to colour fenced code (Document.setSyntaxTheme).
    readonly property string themeName: {
        const f = families.find(f => f.name === family) || families[0];
        return dark ? f.dark : f.light;
    }
    readonly property var colors: data.themes[themeName]
    // Qt rich text supports a CSS subset; this styles the HTML blocks of the preview.
    readonly property string previewStyle: "<html><head><style>"
        + "a { color: " + colors.link + "; }"
        + "h1, h2, h3, h4, h5, h6 { color: " + colors.heading + "; }"
        + "code, pre { color: " + colors.code + "; background-color: " + colors.codeBackground + "; }"
        + "blockquote { color: " + colors.quote + "; }"
        + "</style></head><body>"
}
