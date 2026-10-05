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
    title: (doc.dirty ? "* " : "") + (doc.path || "Untitled") + " — Markite"
    minimumWidth: Kirigami.Units.gridUnit * 30
    minimumHeight: Kirigami.Units.gridUnit * 20
    width: Kirigami.Units.gridUnit * 60
    height: Kirigami.Units.gridUnit * 35

    Document {
        id: doc
        onError: message => showPassiveNotification(message)
        onBlocksHtmlChanged: Qt.callLater(sync.fromEditor)
    }

    Settings {
        id: settings
        location: StandardPaths.writableLocation(StandardPaths.ConfigLocation) + "/markiterc"
        category: "View"
        property bool syncScroll: true
        property bool wrapText: false
        property int viewMode: 2   // 0 code only, 1 preview only, 2 both; restored on next launch
        onViewModeChanged: Qt.callLater(sync.fromEditor)
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
        onAccepted: { doc.open(selectedFile.toString().replace("file://", "")); editor.text = doc.text }
    }
    FileDialog {
        id: saveDialog
        fileMode: FileDialog.SaveFile
        nameFilters: openDialog.nameFilters
        onAccepted: doc.saveAs(selectedFile.toString().replace("file://", ""))
    }

    pageStack.initialPage: Kirigami.Page {
        id: mainPage
        // Top-left: live statistics. Top-right: one vertical three-dots menu holding every action.
        title: doc.wordCount + (doc.wordCount === 1 ? " word" : " words")
        padding: 0

        // Plain SVGs aren't recoloured by icon.color: use the white glyph on dark themes, black on light.
        readonly property string iconSuffix: Kirigami.Theme.textColor.hsvValue > 0.5 ? "-light" : ""

        // Toolbar: shape-only buttons (custom display component, no label).
        Kirigami.Action {
            id: toolbarCode
            displayComponent: ViewButton { active: settings.viewMode === 0; tip: "Code only (Ctrl+1)"
                                           showCode: true; onClicked: settings.viewMode = 0 }
        }
        Kirigami.Action {
            id: toolbarPreview
            displayComponent: ViewButton { active: settings.viewMode === 1; tip: "Preview only (Ctrl+2)"
                                           showPreview: true; onClicked: settings.viewMode = 1 }
        }
        Kirigami.Action {
            id: toolbarBoth
            displayComponent: ViewButton { active: settings.viewMode === 2; tip: "Code and preview (Ctrl+3)"
                                           showCode: true; showPreview: true; onClicked: settings.viewMode = 2 }
        }

        // View submenu: labelled entries with the same shapes. They own the shortcuts; the
        // Bindings re-assert `checked` from settings (a click would otherwise break the binding).
        Kirigami.Action {
            id: menuCode
            text: "Code Only"; shortcut: "Ctrl+1"; checkable: true
            icon.source: "qrc:/icons/view-code" + mainPage.iconSuffix + ".svg"; icon.color: Kirigami.Theme.textColor
            onTriggered: settings.viewMode = 0
        }
        Kirigami.Action {
            id: menuPreview
            text: "Preview Only"; shortcut: "Ctrl+2"; checkable: true
            icon.source: "qrc:/icons/view-preview" + mainPage.iconSuffix + ".svg"; icon.color: Kirigami.Theme.textColor
            onTriggered: settings.viewMode = 1
        }
        Kirigami.Action {
            id: menuBoth
            text: "Code and Preview"; shortcut: "Ctrl+3"; checkable: true
            icon.source: "qrc:/icons/view-both" + mainPage.iconSuffix + ".svg"; icon.color: Kirigami.Theme.textColor
            onTriggered: settings.viewMode = 2
        }
        Binding { target: menuCode; property: "checked"; value: settings.viewMode === 0 }
        Binding { target: menuPreview; property: "checked"; value: settings.viewMode === 1 }
        Binding { target: menuBoth; property: "checked"; value: settings.viewMode === 2 }

        Kirigami.Action {
            id: menuAction
            text: "Menu"
            icon.name: "overflow-menu"
            Kirigami.Action { text: "Open…"; icon.name: "document-open"; shortcut: StandardKey.Open; onTriggered: openDialog.open() }
            Kirigami.Action { text: "Save"; icon.name: "document-save"; shortcut: StandardKey.Save
                onTriggered: doc.path ? doc.save() : saveDialog.open() }
            Kirigami.Action { text: "Save As…"; icon.name: "document-save-as"; shortcut: StandardKey.SaveAs; onTriggered: saveDialog.open() }
            Kirigami.Action { separator: true }
            Kirigami.Action {
                text: "View"
                icon.name: "view-split-left-right"
                children: [menuCode, menuPreview, menuBoth]
            }
            Kirigami.Action {
                text: "Sync Scrolling"
                tooltip: "Keep editor and preview at the same position"
                icon.name: "link"
                checkable: true
                checked: settings.syncScroll
                shortcut: "Ctrl+Shift+L"
                onToggled: { settings.syncScroll = checked; if (checked) sync.fromEditor() }
            }
            Kirigami.Action {
                text: "Wrap Text"
                tooltip: "Wrap long lines instead of scrolling horizontally"
                icon.name: "text-wrap"
                checkable: true
                checked: settings.wrapText
                onToggled: settings.wrapText = checked
            }
        }

        actions: [toolbarCode, toolbarPreview, toolbarBoth, menuAction]

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

            // ---- editor: gutter | text | minimap ----
            RowLayout {
                visible: settings.viewMode !== 1
                Controls.SplitView.fillWidth: settings.viewMode === 0
                Controls.SplitView.minimumWidth: Kirigami.Units.gridUnit * 10
                // 50/50 until the user drags: SplitView assigns preferredWidth on drag, replacing this binding.
                Controls.SplitView.preferredWidth: split.width / 2
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
                            color: Kirigami.Theme.disabledTextColor
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
                            // Measured line pitch and first-line offset (padding, document margin) for gutter, minimap and sync.
                            // ponytail: lineH is one average pitch; minimap/sync drift under wrap (default off). Measure per-line if it matters.
                            readonly property real firstLineY: length >= 0 ? positionToRectangle(0).y : 0
                            readonly property real lineH: lineCount > 1
                                ? (positionToRectangle(length).y - firstLineY) / (lineCount - 1) : fm.lineSpacing
                            // Char offset of each line start, so the fixed gutter can place numbers per line.
                            // Must be a binding on text: opening a file sets text programmatically without onTextChanged.
                            readonly property var lineStarts: {
                                const o = [0];
                                for (let i = 0; i < text.length; i++) if (text.charCodeAt(i) === 10) o.push(i + 1);
                                return o;
                            }
                            onTextChanged: doc.updateText(text)
                            SyntaxHighlighter { textEdit: editor; definition: "Markdown"; theme: Repository.theme("Breeze Dark") }
                            FontMetrics { id: fm; font: editor.font }
                        }
                    }
                }

                // Code map: paints one row per line from core's classification.
                // ponytail: full repaint on every keystroke; cache rows per line if large files lag.
                Canvas {
                    id: minimap
                    Layout.preferredWidth: 80
                    Layout.fillHeight: true
                    property int rowH: 2
                    Connections { target: editor; function onTextChanged() { minimap.requestPaint() } }
                    Connections { target: scroll.contentItem; function onContentYChanged() { minimap.requestPaint() } }
                    onPaint: {
                        const ctx = getContext("2d");
                        ctx.clearRect(0, 0, width, height);
                        const rows = doc.minimapRows(); // [indent, len, kind] triples from core
                        const colors = [null, Kirigami.Theme.highlightColor, Kirigami.Theme.positiveTextColor, Kirigami.Theme.disabledTextColor];
                        for (let i = 0; i * 3 < rows.length && i * rowH < height; i++) {
                            const indent = rows[i * 3], len = rows[i * 3 + 1], kind = rows[i * 3 + 2];
                            if (kind === 0) continue;
                            ctx.fillStyle = colors[kind];
                            ctx.fillRect(indent, i * rowH, Math.min(len, width), rowH - 1);
                        }
                        // viewport
                        const f = scroll.contentItem;
                        ctx.fillStyle = Qt.rgba(0.5, 0.5, 0.5, 0.25);
                        ctx.fillRect(0, f.contentY / editor.lineH * rowH, width, f.height / editor.lineH * rowH);
                    }
                    MouseArea { anchors.fill: parent; onPressed: mouse => sync.setY(scroll.contentItem, mouse.y / minimap.rowH * editor.lineH)
                                onPositionChanged: mouse => { if (pressed) sync.setY(scroll.contentItem, mouse.y / minimap.rowH * editor.lineH) } }
                }
            }

            // ---- preview ----
            // One item per top-level Markdown block so sync can measure where each landed.
            // ponytail: Repeater rebuilds every block per keystroke; diff by index if long docs lag.
            Controls.ScrollView {
                id: previewScroll
                visible: settings.viewMode !== 0
                Controls.SplitView.fillWidth: true
                Controls.SplitView.minimumWidth: Kirigami.Units.gridUnit * 10
                HoverHandler { id: previewHover }
                Column {
                    id: blocksCol
                    width: previewScroll.availableWidth
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
                            text: modelData
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
