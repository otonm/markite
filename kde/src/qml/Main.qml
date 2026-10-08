import QtCore
import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import org.kde.kirigami as Kirigami
import org.kde.syntaxhighlighting
import io.github.otonm.markite

Kirigami.ApplicationWindow {
    id: root
    property bool ready: false          // false until startup has finished: no transition animation while the window is set up
    property real bothWidth: -1         // editor width in "both" mode (remembered across mode switches)
    Component.onCompleted: {
        Qt.callLater(() => ready = true);
        // `markite file.md` / file-manager "open with": first non-option argument.
        const file = Qt.application.arguments.slice(1).find(a => !a.startsWith("-"));
        if (file) openFile(file);
    }

    // A (percent-encoded) file:// URL becomes a plain path; anything else is returned unchanged.
    function toPath(location) {
        const s = String(location);
        if (!s.startsWith("file://")) return s;
        try { return decodeURIComponent(s.slice(7)); } catch (e) { return s.slice(7); } // malformed %-escape: keep it literal
    }

    // Accepts a plain path or a file:// URL.
    function openFile(location) {
        doc.open(toPath(location));
        editor.text = doc.text;
    }
    title: (doc.dirty ? "* " : "") + displayName(doc.path) + " — Markite"

    readonly property var codeFonts: ["Fira Code", "JetBrains Mono", "Cascadia Code", "Monaspace Neon"]
    readonly property var sansFonts: ["Inter", "Open Sans", "Roboto"]
    readonly property var serifFonts: ["Merriweather", "Lora", "Source Serif 4"]
    function fontAvailable(name) { return name !== "" && Qt.fontFamilies().indexOf(name) >= 0 }
    // The chosen family if it is installed or bundled, else `fallback` (the system font). Kerning and shaping
    // (ligatures, contextual alternates) are requested explicitly rather than left to defaults.
    // `size` is the chosen point size; 0 (or an unusable saved value) keeps the fallback's size.
    readonly property int minFontSize: 6
    readonly property int maxFontSize: 40
    function fontSize(size, fallback) {
        return size > 0 ? Math.max(minFontSize, Math.min(maxFontSize, Math.round(size))) : Math.round(fallback.pointSize)
    }
    function fontFor(family, fallback, size) {
        if (!fontAvailable(family) && !(size > 0)) return fallback;
        return Qt.font({ family: fontAvailable(family) ? family : fallback.family, pointSize: fontSize(size, fallback),
                         kerning: true, preferShaping: true })
    }

    // Ranges of the Options sliders. Saved values are clamped on use, so a hand-edited rc file cannot break layout.
    readonly property int minWrapColumn: 60
    readonly property int maxWrapColumn: 200
    readonly property int minPreviewWidth: 20
    readonly property int maxPreviewWidth: 100
    function clamp(value, lo, hi) { return Math.max(lo, Math.min(hi, value)) }
    readonly property int wrapColumn: clamp(settings.wrapColumn, minWrapColumn, maxWrapColumn)
    readonly property int previewWidth: clamp(settings.previewWidth, minPreviewWidth, maxPreviewWidth)

    // Combo-box entries for font groups ({title, families}): "System Default", then per group an optional
    // non-selectable heading and the families that are installed or bundled (a missing family would fall back to a
    // proportional font, so it is not offered).
    function fontChoices(groups) {
        const out = [{ label: "System Default", family: "" }];
        for (const g of groups) {
            const found = g.families.filter(fontAvailable);
            if (found.length === 0) continue;
            if (g.title) out.push({ label: g.title, header: true });
            for (const f of found) out.push({ label: f, family: f });
        }
        return out;
    }
    readonly property var codeFontChoices: fontChoices([{ families: codeFonts }])
    readonly property var previewFontChoices: fontChoices([
        { title: "Sans-Serif Fonts", families: sansFonts }, { title: "Serif Fonts", families: serifFonts }])

    // File name only (decoded for URLs); the status bar shows the full path.
    function displayName(path) {
        if (!path) return "Untitled";
        const last = path.split("/").pop();
        try { return path.indexOf("://") >= 0 ? decodeURIComponent(last) : last; } catch (e) { return last; }
    }
    minimumWidth: Kirigami.Units.gridUnit * 30
    minimumHeight: Kirigami.Units.gridUnit * 20
    width: Kirigami.Units.gridUnit * 60
    height: Kirigami.Units.gridUnit * 35

    Document {
        id: doc
        onError: message => showPassiveNotification(message)
        onBlocksHtmlChanged: Qt.callLater(sync.fromEditor)
        // Core colours fenced code with the editor theme's token colours; re-render on every theme change.
        Component.onCompleted: { setSyntaxTheme(mainPage.themeName); applyConversion() }
        function applyConversion() { setConversion(settings.convertEncoding, settings.convertLineEndings) }
    }
    Connections {
        target: settings
        function onConvertEncodingChanged() { doc.applyConversion() }
        function onConvertLineEndingsChanged() { doc.applyConversion() }
    }
    Connections { target: mainPage; function onThemeNameChanged() { doc.setSyntaxTheme(mainPage.themeName) } }

    Settings {
        id: settings
        location: StandardPaths.writableLocation(StandardPaths.ConfigLocation) + "/markiterc"
        category: "View"
        property bool syncScroll: true
        property bool wrapText: false
        property bool showMinimap: true
        property bool convertEncoding: true     // on save: write UTF-8 instead of the file's own encoding
        property bool convertLineEndings: true  // on save: write \n instead of the file's own line endings
        property string codeFont: ""          // "" = system default monospace font, else a family name
        property string previewFont: ""       // same, for the rendered preview
        property int codeFontSize: 0          // points; 0 = the system default size
        property int previewFontSize: 0
        property int wrapColumn: 100          // characters; the editor wraps here when Wrap Text is on
        property int previewWidth: 100        // max preview text width, % of the window
        property bool watchFiles: true          // reload when the file changes on disk
        property bool showStatusBar: true
        property string editorTheme: "Breeze"   // theme family, see EditorTheme.qml
        property string appearance: "system"    // "system" | "light" | "dark": which variant of the family
        property int viewMode: 2   // 0 code only, 1 preview only, 2 both; restored on next launch
        Component.onCompleted: if (viewMode < 0 || viewMode > 2) viewMode = 2 // out-of-range value from a hand-edited rc file
        onViewModeChanged: { Qt.callLater(sync.fromEditor); split.switchTo(viewMode) }
    }

    // Two-way aliases: Settings restores these on launch and saves them on change.
    Settings {
        location: settings.location
        category: "Window"
        property alias x: root.x
        property alias y: root.y
        property alias width: root.width
        property alias height: root.height
    }

    // Remote files are re-read over KIO, so poll them less often.
    Timer {
        id: watchTimer
        property bool busy: false   // a remote read runs a nested event loop that could fire this timer again
        interval: doc.path.indexOf("://") >= 0 ? 10000 : 1500
        repeat: true
        running: settings.watchFiles && doc.path !== ""
        onTriggered: {
            if (busy) return;
            busy = true;
            if (doc.checkExternal()) { editor.text = doc.text; showPassiveNotification("File changed on disk: reloaded"); }
            busy = false;
        }
    }

    FileDialog {
        id: openDialog
        nameFilters: ["Markdown (*.md *.markdown)", "All files (*)"]
        onAccepted: openFile(selectedFile)
    }
    FileDialog {
        id: saveDialog
        fileMode: FileDialog.SaveFile
        nameFilters: openDialog.nameFilters
        onAccepted: doc.saveAs(toPath(selectedFile))
    }

    // Status bar: window-wide and independent of the view mode; just a little taller than its text.
    footer: Controls.ToolBar {
        id: statusBar
        readonly property real textSize: Kirigami.Theme.defaultFont.pointSize * 0.704
        visible: settings.showStatusBar
        position: Controls.ToolBar.Footer
        padding: 0
        implicitHeight: pathLabel.implicitHeight + Kirigami.Units.smallSpacing * 2
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Kirigami.Units.largeSpacing
            anchors.rightMargin: Kirigami.Units.largeSpacing
            Controls.Label {
                id: pathLabel
                text: doc.path || "Untitled"
                font.pointSize: statusBar.textSize
                elide: Text.ElideMiddle
                Layout.fillWidth: true
            }
            Controls.Label {
                text: doc.wordCount + (doc.wordCount === 1 ? " word" : " words")
                      + " / " + doc.charCount + (doc.charCount === 1 ? " character" : " characters")
                font.pointSize: statusBar.textSize
            }
            Controls.Label {
                text: doc.encoding + " / " + doc.lineEnding
                font.pointSize: statusBar.textSize
            }
        }
    }

    // A real, application-modal dialog window: the window manager moves and resizes it, unlike an in-window overlay.
    // Esc and the Close button dismiss it.
    component DialogWindow: Controls.ApplicationWindow {
        flags: Qt.Dialog
        modality: Qt.ApplicationModal
        transientParent: root
        function open() {
            x = root.x + (root.width - width) / 2;   // centre over the main window (compositors may ignore this)
            y = root.y + (root.height - height) / 2;
            show(); raise(); requestActivate();
        }
        footer: Controls.DialogButtonBox {
            standardButtons: Controls.DialogButtonBox.Close
            onRejected: close()
        }
        Shortcut { sequence: "Esc"; onActivated: close() }
    }

    // A small, dimmed explanation line under an option.
    component Hint: Controls.Label {
        font.pointSize: Kirigami.Theme.defaultFont.pointSize * 0.85
        opacity: 0.7
    }
    readonly property real hintIndent: Kirigami.Units.gridUnit * 2.2   // aligns controls under a CheckOption's label

    // A checkbox bound to a shared Action (which owns the setting and shortcut), laid out as checkbox, icon, a gap,
    // text, and a one-line explanation below. The stock CheckBox packs icon and text tightly and ignores `spacing`.
    component CheckOption: ColumnLayout {
        id: opt
        required property Controls.Action action
        property string hint
        spacing: 0
        RowLayout {
            spacing: Kirigami.Units.gridUnit * 0.75
            Controls.CheckBox {
                id: box
                padding: 0
                // Binding (not an assignment) so a shortcut that flips the action also updates the box.
                Binding { target: box; property: "checked"; value: opt.action.checked }
                onToggled: opt.action.toggle()
            }
            Kirigami.Icon {
                source: opt.action.icon.name
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: Kirigami.Units.iconSizes.small
            }
            Controls.Label {
                text: opt.action.text
                TapHandler { onTapped: box.toggle() }
            }
        }
        Hint { text: opt.hint; leftPadding: box.width + Kirigami.Units.gridUnit * 0.75 }
    }

    // An integer slider with a value label ("12 pt"); `value` is shown, `picked` reports a user change.
    component ValueSlider: RowLayout {
        id: vs
        required property int value
        required property int from
        required property int to
        property string unit
        property alias sliderEnabled: slider.enabled
        signal picked(int value)
        Controls.Slider {
            id: slider
            from: vs.from; to: vs.to; stepSize: 1; snapMode: Controls.Slider.SnapAlways
            Layout.preferredWidth: Kirigami.Units.gridUnit * 12
            value: vs.value
            onMoved: vs.picked(value)
        }
        Controls.Label {
            text: Math.round(slider.value) + vs.unit
            enabled: slider.enabled
            Layout.minimumWidth: Kirigami.Units.gridUnit * 3
        }
    }

    // A font family combo box over `choices` ({label, family}), reading and writing one setting.
    component FontCombo: Controls.ComboBox {
        id: combo
        required property var choices
        required property string current
        signal picked(string family)
        model: choices
        textRole: "label"
        currentIndex: Math.max(0, choices.findIndex(c => !c.header && c.family === current))
        onActivated: index => { if (!choices[index].header) picked(choices[index].family) }
        // Headings are small, dimmed and fenced by lines above and below; they cannot be chosen.
        delegate: Controls.ItemDelegate {
            required property var modelData
            required property int index
            width: ListView.view.width
            enabled: !modelData.header
            highlighted: combo.highlightedIndex === index
            contentItem: ColumnLayout {
                spacing: Kirigami.Units.smallSpacing
                Kirigami.Separator { visible: modelData.header === true; Layout.fillWidth: true }
                Controls.Label {
                    text: modelData.label
                    font.pointSize: Kirigami.Theme.defaultFont.pointSize * (modelData.header ? 0.85 : 1)
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
                Kirigami.Separator { visible: modelData.header === true; Layout.fillWidth: true }
            }
        }
    }

    DialogWindow {
        id: optionsDialog
        title: "Options"
        width: Kirigami.Units.gridUnit * 36; height: Kirigami.Units.gridUnit * 18
        minimumWidth: Kirigami.Units.gridUnit * 28; minimumHeight: Kirigami.Units.gridUnit * 14
        RowLayout {
            anchors.fill: parent
            spacing: 0
            ColumnLayout {
                Layout.fillHeight: true
                // Fixed width: exactly as wide as the longest page name, whatever the page content or window size.
                Layout.preferredWidth: implicitWidth
                Layout.minimumWidth: implicitWidth
                Layout.maximumWidth: implicitWidth
                spacing: 0
                Repeater {
                    model: ["Appearance", "Editor", "Preview", "Files", "Interface"]
                    Controls.ItemDelegate {
                        required property string modelData
                        required property int index
                        Layout.fillWidth: true
                        text: modelData
                        highlighted: pages.currentIndex === index
                        onClicked: pages.currentIndex = index
                    }
                }
                Item { Layout.fillHeight: true }
            }
            Kirigami.Separator { Layout.fillHeight: true }
            StackLayout {
                id: pages
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.margins: Kirigami.Units.largeSpacing
                Kirigami.FormLayout {
                    Controls.ComboBox {
                        Kirigami.FormData.label: "Theme:"
                        model: editorTheme.families
                        textRole: "name"
                        currentIndex: Math.max(0, editorTheme.families.findIndex(f => f.name === settings.editorTheme))
                        onActivated: index => settings.editorTheme = editorTheme.families[index].name
                    }
                    Controls.ComboBox {
                        Kirigami.FormData.label: "Variant:"
                        readonly property var values: ["system", "light", "dark"]
                        model: ["Follow System", "Always Light", "Always Dark"]
                        currentIndex: Math.max(0, values.indexOf(settings.appearance))
                        onActivated: index => settings.appearance = values[index]
                    }
                    FontCombo {
                        Kirigami.FormData.label: "Code font:"
                        choices: root.codeFontChoices; current: settings.codeFont
                        onPicked: family => settings.codeFont = family
                    }
                    ValueSlider {
                        Kirigami.FormData.label: "Code font size:"
                        from: root.minFontSize; to: root.maxFontSize; unit: " pt"
                        value: root.fontSize(settings.codeFontSize, Kirigami.Theme.fixedWidthFont)
                        onPicked: v => settings.codeFontSize = v
                    }
                    FontCombo {
                        Kirigami.FormData.label: "Preview font:"
                        choices: root.previewFontChoices; current: settings.previewFont
                        onPicked: family => settings.previewFont = family
                    }
                    ValueSlider {
                        Kirigami.FormData.label: "Preview font size:"
                        from: root.minFontSize; to: root.maxFontSize; unit: " pt"
                        value: root.fontSize(settings.previewFontSize, Kirigami.Theme.defaultFont)
                        onPicked: v => settings.previewFontSize = v
                    }
                }
                ColumnLayout {
                    CheckOption { action: wrapAction; hint: "Wrap long lines in the editor" }
                    ValueSlider {
                        Layout.leftMargin: root.hintIndent
                        from: root.minWrapColumn; to: root.maxWrapColumn; unit: " chars"
                        sliderEnabled: settings.wrapText
                        value: root.wrapColumn
                        onPicked: v => settings.wrapColumn = v
                    }
                    Hint {
                        text: "Wrap after this many characters, or at the window edge if that comes first"
                        Layout.leftMargin: root.hintIndent
                        Layout.bottomMargin: Kirigami.Units.largeSpacing
                    }
                    CheckOption { action: syncAction; hint: "Editor and preview scroll together (Ctrl+Shift+L)" }
                    Item { Layout.fillHeight: true }
                }
                ColumnLayout {
                    Controls.Label { text: "Text width"; font.bold: true }
                    ValueSlider {
                        from: root.minPreviewWidth; to: root.maxPreviewWidth; unit: " %"
                        value: root.previewWidth
                        onPicked: v => settings.previewWidth = v
                    }
                    Hint {
                        text: "Maximum width of the preview text as a share of the window. A narrower pane always fits, with no horizontal scrolling."
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }
                    Item { Layout.fillHeight: true }
                }
                ColumnLayout {
                    CheckOption { action: watchAction; hint: "Reload when the file changes on disk (local and remote)" }
                    CheckOption { action: convertEncodingAction; hint: "Save as UTF-8 instead of the file's own encoding" }
                    CheckOption { action: convertEolAction; hint: "Save with LF instead of the file's own line endings" }
                    Item { Layout.fillHeight: true }
                }
                ColumnLayout {
                    CheckOption { action: minimapAction; hint: "Code map beside the editor" }
                    CheckOption { action: statusBarAction; hint: "Path, counts, encoding and line ending" }
                    Item { Layout.fillHeight: true }
                }
            }
        }
    }

    DialogWindow {
        id: aboutDialog
        title: "About Markite"
        width: Kirigami.Units.gridUnit * 24; height: Kirigami.Units.gridUnit * 22
        minimumWidth: Kirigami.Units.gridUnit * 18; minimumHeight: Kirigami.Units.gridUnit * 18
        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.smallSpacing
            Image {
                source: "qrc:/icons/app.svg"
                sourceSize.width: Kirigami.Units.gridUnit * 5
                sourceSize.height: Kirigami.Units.gridUnit * 5
                Layout.alignment: Qt.AlignHCenter
            }
            Kirigami.Heading { text: "Markite"; Layout.alignment: Qt.AlignHCenter }
            Controls.Label { text: "Version " + Qt.application.version; Layout.alignment: Qt.AlignHCenter }
            Controls.Label {
                text: "A small, native Markdown viewer and editor for Linux."
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
                Layout.fillWidth: true
            }
            Controls.Label {
                text: "© 2026 Oton Mahnic · MIT License"
                color: Kirigami.Theme.disabledTextColor
                Layout.alignment: Qt.AlignHCenter
            }
            Controls.Label {
                // Rich-text links ignore the palette; colour them via CSS so they read on dark themes.
                text: "<style>a { color: " + Kirigami.Theme.linkColor + "; }</style>"
                      + "<a href=\"https://github.com/otonm/markite\">github.com/otonm/markite</a>"
                textFormat: Text.RichText
                onLinkActivated: link => Qt.openUrlExternally(link)
                Layout.alignment: Qt.AlignHCenter
            }
            Controls.Label {
                text: "Built with Qt and KDE Frameworks"
                color: Kirigami.Theme.disabledTextColor
                Layout.alignment: Qt.AlignHCenter
            }
        }
    }

    pageStack.initialPage: Kirigami.Page {
        id: mainPage
        padding: 0

        // Theme logic lives in EditorTheme.qml (unit-tested in kde/tests/qml); these forward to it.
        // The code font: the system monospace font, or the chosen family if it is installed or bundled.
        readonly property font codeFont: root.fontFor(settings.codeFont, Kirigami.Theme.fixedWidthFont, settings.codeFontSize)
        readonly property font previewFont: root.fontFor(settings.previewFont, Kirigami.Theme.defaultFont, settings.previewFontSize)
        readonly property string themeName: editorTheme.themeName
        readonly property var themeColors: editorTheme.colors
        readonly property string previewStyle: editorTheme.previewStyle
        EditorTheme {
            id: editorTheme
            family: settings.editorTheme
            appearance: settings.appearance
            systemDark: Kirigami.Theme.textColor.hsvValue > 0.5
        }

        // Plain SVGs aren't recoloured by icon.color: use the white glyph on dark themes, black on light.
        readonly property string iconSuffix: editorTheme.systemDark ? "-light" : ""

        // Actions own the shortcuts and are shown in the menu below.
        Controls.Action { id: openAction; text: "Open…"; icon.name: "document-open"; shortcut: StandardKey.Open
            onTriggered: openDialog.open() }
        Controls.Action { id: saveAction; text: "Save"; icon.name: "document-save"; shortcut: StandardKey.Save
            onTriggered: doc.path ? doc.save() : saveDialog.open() }
        Controls.Action { id: saveAsAction; text: "Save As…"; icon.name: "document-save-as"; shortcut: StandardKey.SaveAs
            onTriggered: saveDialog.open() }
        Controls.Action { id: quitAction; text: "Exit"; icon.name: "application-exit"; shortcut: "Ctrl+W"
            onTriggered: Qt.quit() }

        // View modes. The Bindings re-assert `checked` from settings (a click would otherwise break the binding).
        Controls.Action {
            id: viewCodeAction
            text: "Code Only"; shortcut: "Ctrl+1"; checkable: true
            icon.source: "qrc:/icons/view-code" + mainPage.iconSuffix + ".svg"; icon.color: Kirigami.Theme.textColor
            onTriggered: settings.viewMode = 0
            // Triggering the active mode again toggles it off; re-check it once Qt has finished toggling.
            onCheckedChanged: if (!viewCodeAction.checked && settings.viewMode === 0) Qt.callLater(() => viewCodeAction.checked = true)
        }
        Controls.Action {
            id: viewPreviewAction
            text: "Preview Only"; shortcut: "Ctrl+2"; checkable: true
            icon.source: "qrc:/icons/view-preview" + mainPage.iconSuffix + ".svg"; icon.color: Kirigami.Theme.textColor
            onTriggered: settings.viewMode = 1
            // Triggering the active mode again toggles it off; re-check it once Qt has finished toggling.
            onCheckedChanged: if (!viewPreviewAction.checked && settings.viewMode === 1) Qt.callLater(() => viewPreviewAction.checked = true)
        }
        Controls.Action {
            id: viewBothAction
            text: "Code and Preview"; shortcut: "Ctrl+3"; checkable: true
            icon.source: "qrc:/icons/view-both" + mainPage.iconSuffix + ".svg"; icon.color: Kirigami.Theme.textColor
            onTriggered: settings.viewMode = 2
            // Triggering the active mode again toggles it off; re-check it once Qt has finished toggling.
            onCheckedChanged: if (!viewBothAction.checked && settings.viewMode === 2) Qt.callLater(() => viewBothAction.checked = true)
        }
        Binding { target: viewCodeAction; property: "checked"; value: settings.viewMode === 0 }
        Binding { target: viewPreviewAction; property: "checked"; value: settings.viewMode === 1 }
        Binding { target: viewBothAction; property: "checked"; value: settings.viewMode === 2 }

        Controls.Action {
            id: syncAction
            text: "Sync Scrolling"; icon.name: "link"; shortcut: "Ctrl+Shift+L"
            checkable: true; checked: settings.syncScroll
            onToggled: { settings.syncScroll = syncAction.checked; if (syncAction.checked) sync.fromEditor() }
        }
        Controls.Action {
            id: wrapAction
            text: "Wrap Text"; icon.name: "text-wrap"
            checkable: true; checked: settings.wrapText
            onToggled: settings.wrapText = wrapAction.checked
        }
        Controls.Action {
            id: optionsAction
            text: "Options…"; icon.name: "configure"; shortcut: StandardKey.Preferences
            onTriggered: optionsDialog.open()
        }
        Controls.Action {
            id: aboutAction
            text: "About Markite"; icon.name: "help-about"
            onTriggered: aboutDialog.open()
        }
        Controls.Action {
            id: statusBarAction
            text: "Show Status Bar"; icon.name: "view-statistics"
            checkable: true; checked: settings.showStatusBar
            onToggled: settings.showStatusBar = statusBarAction.checked
        }
        Controls.Action {
            id: watchAction
            text: "Monitor File Changes"; icon.name: "view-refresh"
            checkable: true; checked: settings.watchFiles
            onToggled: settings.watchFiles = watchAction.checked
        }
        Controls.Action {
            id: convertEncodingAction
            text: "Convert Encoding"; icon.name: "format-text-code"
            checkable: true; checked: settings.convertEncoding
            onToggled: settings.convertEncoding = convertEncodingAction.checked
        }
        Controls.Action {
            id: convertEolAction
            text: "Convert Line Endings"; icon.name: "format-justify-left"
            checkable: true; checked: settings.convertLineEndings
            onToggled: settings.convertLineEndings = convertEolAction.checked
        }
        Controls.Action {
            id: minimapAction
            text: "Show Minimap"; icon.name: "view-list-details"
            checkable: true; checked: settings.showMinimap
            onToggled: settings.showMinimap = minimapAction.checked
        }

        Controls.Menu {
            id: mainMenu
            Controls.MenuItem { action: openAction }
            Controls.MenuItem { action: saveAction }
            Controls.MenuItem { action: saveAsAction }
            Controls.MenuSeparator {}
            Controls.Menu {
                title: "View"
                icon.name: "view-split-left-right"
                Controls.MenuItem { action: viewCodeAction }
                Controls.MenuItem { action: viewPreviewAction }
                Controls.MenuItem { action: viewBothAction }
            }
            Controls.MenuItem { action: optionsAction }
            Controls.MenuSeparator {}
            Controls.MenuItem { action: aboutAction }
            Controls.MenuItem { action: quitAction }
        }

        // Top bar (left-aligned): the three-dots menu and view buttons. Path and statistics live in the status bar.
        titleDelegate: RowLayout {
            spacing: Kirigami.Units.smallSpacing
            ViewButton { id: menuButton; iconName: "overflow-menu"; tip: "Menu"
                         active: mainMenu.visible; onClicked: mainMenu.popup(menuButton, 0, menuButton.height) }
            ViewButton { active: settings.viewMode === 0; tip: "Code only (Ctrl+1)"
                         showCode: true; onClicked: settings.viewMode = 0 }
            ViewButton { active: settings.viewMode === 1; tip: "Preview only (Ctrl+2)"
                         showPreview: true; onClicked: settings.viewMode = 1 }
            ViewButton { active: settings.viewMode === 2; tip: "Code and preview (Ctrl+3)"
                         showCode: true; showPreview: true; onClicked: settings.viewMode = 2 }
        }

        // Editor <-> preview position sync. Core maps source lines to preview blocks
        // (doc.lineToBlock / doc.blockToLine); this only measures where blocks landed.
        QtObject {
            id: sync
            property bool busy: false
            readonly property Flickable ed: scroll.contentItem
            readonly property Flickable pv: previewScroll.contentItem

            function atEnd(f) { return f.contentHeight > f.height && f.contentY >= f.contentHeight - f.height - 1 }
            function setY(f, y) { f.contentY = Math.max(0, Math.min(y, f.contentHeight - f.height)) }

            // One guarded sync step: skipped when sync is off, there is nothing to map, the line height is unknown
            // or another step is running; `busy` is cleared even if `body` throws.
            function guarded(body) {
                if (!settings.syncScroll || busy || blockRep.count === 0 || !(editor.lineH > 0)) return;
                busy = true;
                try { blocksCol.forceLayout(); body(); } finally { busy = false; }
            }

            function fromEditor() {
                guarded(() => {
                    if (atEnd(ed)) {
                        setY(pv, pv.contentHeight);
                        return;
                    }
                    const line = Math.max(0, ed.contentY - editor.y - editor.firstLineY) / editor.lineH + 1;
                    const m = doc.lineToBlock(line);
                    const item = blockRep.itemAt(m[0]);
                    if (item) setY(pv, blocksCol.y + item.y + m[1] * item.height);
                });
            }

            function fromPreview() {
                guarded(() => {
                    if (atEnd(pv)) {
                        setY(ed, ed.contentHeight);
                        return;
                    }
                    const y = pv.contentY - blocksCol.y;
                    let i = 0;
                    while (i < blockRep.count - 1 && blockRep.itemAt(i).y + blockRep.itemAt(i).height <= y) i++;
                    const item = blockRep.itemAt(i);
                    const frac = item.height > 0 ? Math.max(0, Math.min(1, (y - item.y) / item.height)) : 0;
                    setY(ed, editor.y + editor.firstLineY + (doc.blockToLine(i, frac) - 1) * editor.lineH);
                });
            }
        }
        Connections { target: sync.ed; function onContentYChanged() { sync.fromEditor() } }
        // Only user scrolling of the preview drives the editor; re-layout while typing must not.
        // Dragging the scrollbar grabs the pointer (no hover, not "moving"), so check pressed too.
        Connections { target: sync.pv; function onContentYChanged() {
            const bar = previewScroll.Controls.ScrollBar.vertical;
            if (previewHover.hovered || sync.pv.moving || (bar && bar.pressed)) sync.fromPreview();
        } }
        Connections { target: sync.pv; function onContentHeightChanged() { if (!previewHover.hovered) Qt.callLater(sync.fromEditor) } }

        // Don't let NoWrap editor content push the page/window minimum size around.
        implicitWidth: 0
        implicitHeight: 0
        Controls.SplitView {
            id: split
            anchors.fill: parent

            // View-mode transition: animate the editor pane's width; the preview (fillWidth) takes
            // the rest. Both panes stay visible with min widths lifted until the animation ends.
            property int shown: 2
            property bool animating: false
            function switchTo(mode) {
                const prev = shown;
                shown = mode;
                if (!root.ready || width <= 0) return;
                if (prev === 2 && !animating) root.bothWidth = editorPane.width;
                const from = editorPane.visible ? editorPane.width : 0;
                const target = mode === 0 ? width : mode === 1 ? 0 : (root.bothWidth > 0 ? root.bothWidth : width / 2);
                animating = true;
                editorPane.Controls.SplitView.preferredWidth = from;
                slide.from = from;
                slide.to = target;
                slide.restart();
            }
            NumberAnimation {
                id: slide
                target: editorPane.Controls.SplitView
                property: "preferredWidth"
                duration: 180
                easing.type: Easing.OutCubic
                onFinished: {
                    split.animating = false;
                    // Back to 50/50 (or the remembered width) following window resizes until the next drag.
                    if (split.shown === 2)
                        editorPane.Controls.SplitView.preferredWidth = Qt.binding(() => root.bothWidth > 0 ? root.bothWidth : split.width / 2);
                }
            }

            // ---- editor: gutter | text | minimap ----
            Item {
                id: editorPane
                clip: true
                Rectangle { anchors.fill: parent; color: mainPage.themeColors.background }
                visible: settings.viewMode !== 1 || split.animating
                Controls.SplitView.fillWidth: settings.viewMode === 0 && !split.animating
                Controls.SplitView.minimumWidth: split.animating ? 0 : Kirigami.Units.gridUnit * 10
                // 50/50 until the user drags: SplitView assigns preferredWidth on drag, replacing this binding.
                Controls.SplitView.preferredWidth: split.width / 2
                implicitWidth: 0

              // Content keeps at least its "both" width and is right-anchored, so while the pane grows
              // from zero the editor slides in from the left instead of reflowing.
              RowLayout {
                id: editorRow
                readonly property real steadyWidth: root.bothWidth > 0 ? root.bothWidth : split.width / 2
                width: Math.max(editorPane.width, steadyWidth)
                height: editorPane.height
                x: editorPane.width - width
                spacing: 0
                implicitWidth: 0

                // Fixed gutter: stays put during horizontal scroll, follows vertical scroll.
                // Each number is placed at its line's measured y (positionToRectangle), so it
                // stays aligned under wrapping too.
                Item {
                    id: gutter
                    Layout.fillHeight: true
                    Layout.preferredWidth: editor.lineStarts.length.toString().length * fm.averageCharacterWidth + Kirigami.Units.largeSpacing
                    // Stop the numbers above the editor's horizontal scrollbar instead of behind it.
                    Layout.bottomMargin: scroll.Controls.ScrollBar.horizontal.visible ? scroll.Controls.ScrollBar.horizontal.height : 0
                    clip: true
                    Repeater {
                        model: editor.lineStarts.length
                        Controls.Label {
                            text: index + 1
                            font: editor.font
                            color: mainPage.themeColors.lineNumber
                            horizontalAlignment: Text.AlignRight
                            width: gutter.width
                            rightPadding: Kirigami.Units.smallSpacing
                            height: editor.lineH
                            // positionToRectangle() isn't reactive: the leading terms make the binding re-run
                            // whenever wrapping re-lays out the text (wrap toggled, width settled, text loaded).
                            y: (settings.wrapText, editor.width, editor.contentHeight,
                                editor.positionToRectangle(editor.lineStarts[index] || 0).y - scroll.contentItem.contentY)
                        }
                    }
                }

                Controls.ScrollView {
                    id: scroll
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.minimumWidth: 0
                    Layout.preferredWidth: 0

                    RowLayout {
                        spacing: 0
                        // Fill the visible area; grow (and scroll horizontally) for long lines.
                        // When wrapping, pin to the viewport so the editor is constrained and actually wraps.
                        width: settings.wrapText ? scroll.availableWidth : Math.max(scroll.availableWidth, implicitWidth)
                        height: Math.max(scroll.availableHeight, implicitHeight)  // click anywhere below the text to focus it
                        Controls.TextArea {
                            id: editor
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            // Wrap after `wrapColumn` characters (exact for a monospace font) when that is narrower than the pane.
                            Layout.maximumWidth: settings.wrapText
                                ? root.wrapColumn * fm.averageCharacterWidth + leftPadding + rightPadding
                                : Number.POSITIVE_INFINITY
                            font: mainPage.codeFont
                            wrapMode: settings.wrapText ? TextEdit.Wrap : TextEdit.NoWrap
                            background: null
                            color: mainPage.themeColors.text
                            selectionColor: mainPage.themeColors.selection
                            selectedTextColor: mainPage.themeColors.selectedText
                            // Measured line pitch and first-line offset (padding, document margin) for gutter, minimap and sync.
                            // Known limitation: lineH is one average pitch, so minimap and scroll sync drift when wrapping (off by default).
                            // `length >= 0` is always true; it only makes the binding re-run when the text changes.
                            readonly property real firstLineY: length >= 0 ? positionToRectangle(0).y : 0
                            readonly property real lineH: lineCount > 1
                                ? (positionToRectangle(length).y - firstLineY) / (lineCount - 1) : fm.lineSpacing
                            // Char offset of each line start, so the fixed gutter can place numbers per line.
                            // Derived from `text`, so it updates for typing and for programmatic loads (openFile) alike.
                            readonly property var lineStarts: {
                                const o = [0];
                                for (let i = 0; i < text.length; i++) if (text.charCodeAt(i) === 10) o.push(i + 1);
                                return o;
                            }
                            onTextChanged: doc.updateText(text)
                            SyntaxHighlighter { textEdit: editor; definition: "Markdown"; theme: Repository.theme(mainPage.themeName) }
                            FontMetrics { id: fm; font: editor.font }
                        }
                    }
                }

                // Code map: paints one row per line from core's classification.
                // Known limitation: repaints fully on every keystroke and scroll; large files may lag.
                Canvas {
                    id: minimap
                    visible: settings.showMinimap
                    Layout.preferredWidth: 80
                    Layout.fillHeight: true
                    property int rowH: 2
                    Connections { target: editor; function onTextChanged() { minimap.requestPaint() } }
                    Connections { target: scroll.contentItem; function onContentYChanged() { minimap.requestPaint() } }
                    Connections { target: mainPage; function onThemeColorsChanged() { minimap.requestPaint() } }
                    onPaint: {
                        const ctx = getContext("2d");
                        ctx.clearRect(0, 0, width, height);
                        const rows = doc.minimapRows(); // [indent, len, kind, marker] quadruples from core
                        const t = mainPage.themeColors;
                        // Same colours the editor's Markdown highlighting uses, by line kind (see EditorTheme.qml).
                        const colors = [null, t.heading, t.code, t.text, t.listText, t.quote, t.table];
                        for (let i = 0; i * 4 < rows.length && i * rowH < height; i++) {
                            const indent = rows[i * 4], len = rows[i * 4 + 1], kind = rows[i * 4 + 2], marker = rows[i * 4 + 3];
                            if (kind === 0) continue;
                            const y = i * rowH, h = rowH - 1;
                            if (kind === 4 && marker > 0) {   // list marker, then the item text after a space
                                ctx.fillStyle = t.listMarker;
                                ctx.fillRect(indent, y, Math.min(marker, width - indent), h);
                                const textX = indent + marker + 1;
                                ctx.fillStyle = t.listText;
                                if (textX < width) ctx.fillRect(textX, y, Math.max(0, Math.min(len - marker - 1, width - textX)), h);
                                continue;
                            }
                            ctx.fillStyle = colors[kind];
                            ctx.fillRect(indent, y, Math.min(len, width), h);
                        }
                        // viewport
                        const f = scroll.contentItem;
                        ctx.fillStyle = Qt.alpha(mainPage.themeColors.text, 0.2);
                        ctx.fillRect(0, f.contentY / editor.lineH * rowH, width, f.height / editor.lineH * rowH);
                    }
                    // Scroll the editor so the clicked/dragged minimap row is at the top.
                    function seek(y) { sync.setY(scroll.contentItem, y / rowH * editor.lineH) }
                    MouseArea { anchors.fill: parent; onPressed: mouse => minimap.seek(mouse.y)
                                onPositionChanged: mouse => { if (pressed) minimap.seek(mouse.y) } }
                }
              }
            }

            // ---- preview ----
            // One item per top-level Markdown block so sync can measure where each landed.
            // Known limitation: the Repeater rebuilds every block whenever the preview changes; long documents may lag.
            Controls.ScrollView {
                id: previewScroll
                visible: settings.viewMode !== 0 || split.animating
                background: Rectangle { color: mainPage.themeColors.background }
                Controls.ScrollBar.horizontal.policy: split.animating ? Controls.ScrollBar.AlwaysOff : Controls.ScrollBar.AsNeeded
                Controls.SplitView.fillWidth: true
                Controls.SplitView.minimumWidth: split.animating ? 0 : Kirigami.Units.gridUnit * 10
                HoverHandler { id: previewHover }
                Column {
                    id: blocksCol
                    // While the pane is animating, keep the "both" width so the text is covered/uncovered
                    // instead of reflowing into a squeezed column.
                    width: split.animating ? Math.max(previewScroll.availableWidth, split.width - editorRow.steadyWidth)
                                           : previewScroll.availableWidth
                    // Text column: at most `previewWidth` % of the whole view, never wider than this pane (no horizontal
                    // scrolling), centred.
                    readonly property real textWidth: Math.max(0, Math.min(width - 2 * Kirigami.Units.largeSpacing,
                        split.width * root.previewWidth / 100))
                    leftPadding: (width - textWidth) / 2
                    rightPadding: leftPadding
                    topPadding: Kirigami.Units.largeSpacing
                    bottomPadding: Kirigami.Units.largeSpacing
                    spacing: Kirigami.Units.smallSpacing
                    Repeater {
                        id: blockRep
                        model: doc.blocksHtml
                        Controls.TextArea {
                            required property string modelData
                            width: blocksCol.width - blocksCol.leftPadding - blocksCol.rightPadding
                            readOnly: true
                            textFormat: TextEdit.RichText
                            text: mainPage.previewStyle + modelData + "</body></html>"
                            color: mainPage.themeColors.text
                            selectionColor: mainPage.themeColors.selection
                            selectedTextColor: mainPage.themeColors.selectedText
                            font: mainPage.previewFont
                            wrapMode: TextEdit.Wrap
                            background: null
                            padding: 0
                        }
                    }
                }
            }
        }
    }
}
