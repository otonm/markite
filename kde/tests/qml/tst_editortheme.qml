import QtQuick
import QtTest
import "../../src/qml"

TestCase {
    name: "EditorTheme"
    when: windowShown

    Component { id: make; EditorTheme {} }

    function theme(props) { return createTemporaryObject(make, null, props); }

    function test_variant_follows_appearance() {
        compare(theme({ family: "Breeze", appearance: "light", systemDark: true }).themeName, "Breeze Light");
        compare(theme({ family: "Breeze", appearance: "dark", systemDark: false }).themeName, "Breeze Dark");
    }

    function test_system_appearance_uses_palette() {
        compare(theme({ family: "Catppuccin", appearance: "system", systemDark: false }).themeName, "Catppuccin Latte");
        compare(theme({ family: "Catppuccin", appearance: "system", systemDark: true }).themeName, "Catppuccin Mocha");
    }

    function test_unknown_appearance_follows_the_system() {
        compare(theme({ family: "Breeze", appearance: "bogus", systemDark: true }).themeName, "Breeze Dark");
        compare(theme({ family: "Breeze", appearance: "bogus", systemDark: false }).themeName, "Breeze Light");
    }

    function test_unknown_family_falls_back_to_first() {
        compare(theme({ family: "nope", appearance: "light" }).themeName, "Breeze Light");
    }

    function test_theme_change_updates_bindings() {
        const t = theme({ family: "GitHub", appearance: "light" });
        compare(t.themeName, "GitHub Light");
        t.appearance = "dark";
        compare(t.themeName, "GitHub Dark");
        verify(t.previewStyle.indexOf(t.colors.link) >= 0);
    }

    // Every name a family can resolve to must have colours, or the UI breaks (and core finds no theme).
    function test_every_family_variant_has_colours() {
        const t = theme({});
        const keys = ["background", "text", "selectedText", "selection", "lineNumber", "heading", "code", "link", "quote", "listMarker", "listText", "table", "codeBackground"];
        t.families.forEach(f => [f.light, f.dark].forEach(n => {
            verify(t.data.themes[n] !== undefined, n + " missing from EditorThemes");
            keys.forEach(k => verify(/^#([0-9a-fA-F]{2})?[0-9a-fA-F]{6}$/ /* #RRGGBB or Qt #AARRGGBB */.test(t.data.themes[n][k]), n + "." + k));
        }));
    }

    function test_preview_style_uses_theme_colours() {
        const t = theme({ family: "Solarized", appearance: "dark" });
        const c = t.colors;
        verify(t.previewStyle.indexOf("a { color: " + c.link + "; }") >= 0);
        verify(t.previewStyle.indexOf("background-color: " + c.codeBackground) >= 0);
    }
}
