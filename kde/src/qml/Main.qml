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
    property bool ready: false          // false while settings load: no transition animation at startup
    property real bothWidth: -1         // editor width in "both" mode (remembered across mode switches)
    Component.onCompleted: {
        Qt.callLater(() => ready = true);
        // `markite file.md` / file-manager "open with": first non-option argument.
        const file = Qt.application.arguments.slice(1).find(a => !a.startsWith("-"));
        if (file) openFile(file);
    }

    // Accepts a plain path or a (percent-encoded) file:// URL.
    function openFile(location) {
        const path = String(location).startsWith("file://") ? decodeURIComponent(String(location).slice(7)) : String(location);
        doc.open(path);
        editor.text = doc.text;
    }
    title: (doc.dirty ? "* " : "") + displayName(doc.path) + " — Markite"

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

    // Radio-style menu entry driven by `selected`; a click can never leave it out of step with the setting.
    component ChoiceItem: Controls.MenuItem {
        id: item
        property bool selected: false
        checkable: true
        checked: selected
        onCheckedChanged: if (checked !== selected) checked = Qt.binding(() => item.selected)
    }


    Document {
        id: doc
        onError: message => showPassiveNotification(message)
        onBlocksHtmlChanged: Qt.callLater(sync.fromEditor)
        // Core colours fenced code with the editor theme's token colours; re-render on every theme change.
        Component.onCompleted: setSyntaxTheme(mainPage.themeName)
    }
    Connections { target: mainPage; function onThemeNameChanged() { doc.setSyntaxTheme(mainPage.themeName) } }

    Settings {
        id: settings
        location: StandardPaths.writableLocation(StandardPaths.ConfigLocation) + "/markiterc"
        category: "View"
        property bool syncScroll: true
        property bool wrapText: false
        property bool showMinimap: true
        property bool showStatusBar: true
        property string editorTheme: "Breeze"   // theme family, see EditorTheme.qml
        property string appearance: "system"    // "system" | "light" | "dark": which variant of the family
        property int viewMode: 2   // 0 code only, 1 preview only, 2 both; restored on next launch
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

    FileDialog {
        id: openDialog
        nameFilters: ["Markdown (*.md *.markdown)", "All files (*)"]
        onAccepted: openFile(selectedFile)
    }
    FileDialog {
        id: saveDialog
        fileMode: FileDialog.SaveFile
        nameFilters: openDialog.nameFilters
        onAccepted: doc.saveAs(decodeURIComponent(selectedFile.toString().replace("file://", "")))
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
        }
    }

    Kirigami.Dialog {
        id: aboutDialog
        title: "About Markite"
        standardButtons: Kirigami.Dialog.Close
        padding: Kirigami.Units.largeSpacing
        ColumnLayout {
            spacing: Kirigami.Units.smallSpacing
            implicitWidth: Kirigami.Units.gridUnit * 22
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
        // Top bar: the three-dots menu and view buttons. Path and statistics live in the status bar.
        padding: 0

        // Theme logic lives in EditorTheme.qml (unit-tested in kde/tests/qml); these forward to it.
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
        readonly property string iconSuffix: Kirigami.Theme.textColor.hsvValue > 0.5 ? "-light" : ""

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
            Controls.Menu {
                title: "Theme"
                icon.name: "preferences-desktop-color"
                ChoiceItem { text: "Breeze"; selected: settings.editorTheme === "Breeze"; onTriggered: settings.editorTheme = "Breeze" }
                ChoiceItem { text: "Atom One"; selected: settings.editorTheme === "Atom One"; onTriggered: settings.editorTheme = "Atom One" }
                ChoiceItem { text: "Catppuccin"; selected: settings.editorTheme === "Catppuccin"; onTriggered: settings.editorTheme = "Catppuccin" }
                ChoiceItem { text: "GitHub"; selected: settings.editorTheme === "GitHub"; onTriggered: settings.editorTheme = "GitHub" }
                ChoiceItem { text: "Solarized"; selected: settings.editorTheme === "Solarized"; onTriggered: settings.editorTheme = "Solarized" }
                Controls.MenuSeparator {}
                ChoiceItem { text: "Follow System"; selected: settings.appearance === "system"; onTriggered: settings.appearance = "system" }
                ChoiceItem { text: "Always Light"; selected: settings.appearance === "light"; onTriggered: settings.appearance = "light" }
                ChoiceItem { text: "Always Dark"; selected: settings.appearance === "dark"; onTriggered: settings.appearance = "dark" }
            }
            Controls.MenuItem { action: syncAction }
            Controls.MenuItem { action: wrapAction }
            Controls.MenuItem { action: minimapAction }
            Controls.MenuItem { action: statusBarAction }
            Controls.MenuSeparator {}
            Controls.MenuItem { action: aboutAction }
            Controls.MenuItem { action: quitAction }
        }

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
            readonly property var ed: scroll.contentItem
            readonly property var pv: previewScroll.contentItem

            function atEnd(f) { return f.contentHeight > f.height && f.contentY >= f.contentHeight - f.height - 1 }
            function setY(f, y) { f.contentY = Math.max(0, Math.min(y, f.contentHeight - f.height)) }

            function fromEditor() {
                if (!settings.syncScroll || busy || blockRep.count === 0) return;
                busy = true;
                blocksCol.forceLayout();
                if (atEnd(ed)) {
                    setY(pv, pv.contentHeight);
                } else {
                    const line = Math.max(0, ed.contentY - editor.y - editor.firstLineY) / editor.lineH + 1;
                    const m = doc.lineToBlock(line);
                    const item = blockRep.itemAt(m[0]);
                    if (item) setY(pv, blocksCol.y + item.y + m[1] * item.height);
                }
                busy = false;
            }

            function fromPreview() {
                if (!settings.syncScroll || busy || blockRep.count === 0) return;
                busy = true;
                blocksCol.forceLayout();
                if (atEnd(pv)) {
                    setY(ed, ed.contentHeight);
                } else {
                    const y = pv.contentY - blocksCol.y;
                    let i = 0;
                    while (i < blockRep.count - 1 && blockRep.itemAt(i).y + blockRep.itemAt(i).height <= y) i++;
                    const item = blockRep.itemAt(i);
                    const frac = item.height > 0 ? Math.max(0, Math.min(1, (y - item.y) / item.height)) : 0;
                    setY(ed, editor.y + editor.firstLineY + (doc.blockToLine(i, frac) - 1) * editor.lineH);
                }
                busy = false;
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
                            font: Kirigami.Theme.fixedWidthFont
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
                        const rows = doc.minimapRows(); // [indent, len, kind] triples from core
                        const t = mainPage.themeColors;
                        const colors = [null, t.heading, t.code, Qt.alpha(t.text, 0.5)];
                        for (let i = 0; i * 3 < rows.length && i * rowH < height; i++) {
                            const indent = rows[i * 3], len = rows[i * 3 + 1], kind = rows[i * 3 + 2];
                            if (kind === 0) continue;
                            ctx.fillStyle = colors[kind];
                            ctx.fillRect(indent, i * rowH, Math.min(len, width), rowH - 1);
                        }
                        // viewport
                        const f = scroll.contentItem;
                        ctx.fillStyle = Qt.alpha(mainPage.themeColors.text, 0.2);
                        ctx.fillRect(0, f.contentY / editor.lineH * rowH, width, f.height / editor.lineH * rowH);
                    }
                    MouseArea { anchors.fill: parent; onPressed: mouse => sync.setY(scroll.contentItem, mouse.y / minimap.rowH * editor.lineH)
                                onPositionChanged: mouse => { if (pressed) sync.setY(scroll.contentItem, mouse.y / minimap.rowH * editor.lineH) } }
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
                    padding: Kirigami.Units.largeSpacing
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
