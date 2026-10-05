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

    globalDrawer: Kirigami.GlobalDrawer {
        actions: [
            Kirigami.Action { text: "Open…"; icon.name: "document-open"; shortcut: StandardKey.Open; onTriggered: openDialog.open() },
            Kirigami.Action { text: "Save"; icon.name: "document-save"; shortcut: StandardKey.Save
                onTriggered: doc.path ? doc.save() : saveDialog.open() },
            Kirigami.Action { text: "Save As…"; icon.name: "document-save-as"; shortcut: StandardKey.SaveAs; onTriggered: saveDialog.open() }
        ]
    }

    pageStack.initialPage: Kirigami.Page {
        title: "Markdown"
        padding: 0
        actions: [
            Kirigami.Action {
                text: "Sync Scrolling"
                tooltip: "Keep editor and preview at the same position"
                icon.name: "link"
                checkable: true
                checked: settings.syncScroll
                shortcut: "Ctrl+Shift+L"
                onToggled: { settings.syncScroll = checked; if (checked) sync.fromEditor() }
            }
        ]

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
        Connections { target: sync.pv; function onContentYChanged() { if (previewHover.hovered || sync.pv.moving) sync.fromPreview() } }
        Connections { target: sync.pv; function onContentHeightChanged() { if (!previewHover.hovered) Qt.callLater(sync.fromEditor) } }

        // Don't let NoWrap editor content push the page/window minimum size around.
        implicitWidth: 0
        implicitHeight: 0
        Controls.SplitView {
            id: split
            anchors.fill: parent

            // ---- editor: gutter | text | minimap ----
            RowLayout {
                Controls.SplitView.minimumWidth: Kirigami.Units.gridUnit * 10
                // 50/50 until the user drags: SplitView assigns preferredWidth on drag, replacing this binding.
                Controls.SplitView.preferredWidth: split.width / 2
                spacing: 0
                implicitWidth: 0

                Controls.ScrollView {
                    id: scroll
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.minimumWidth: 0
                    Layout.preferredWidth: 0

                    RowLayout {
                        spacing: 0
                        // Fill the visible area; grow (and scroll horizontally) for long lines.
                        width: Math.max(scroll.availableWidth, implicitWidth)
                        height: Math.max(scroll.availableHeight, implicitHeight)  // click anywhere below the text to focus it
                        // Line numbers: one Text per line, aligned by monospace line height.
                        Column {
                            Layout.alignment: Qt.AlignTop
                            topPadding: editor.firstLineY
                            Repeater {
                                model: editor.lineCount
                                Controls.Label {
                                    text: index + 1
                                    font: editor.font
                                    color: Kirigami.Theme.disabledTextColor
                                    horizontalAlignment: Text.AlignRight
                                    width: editor.lineCount.toString().length * fm.averageCharacterWidth + Kirigami.Units.largeSpacing
                                    rightPadding: Kirigami.Units.smallSpacing
                                    height: editor.lineH
                                }
                            }
                        }
                        Controls.TextArea {
                            id: editor
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            font: Kirigami.Theme.fixedWidthFont
                            wrapMode: TextEdit.NoWrap
                            background: null
                            // Measured line pitch and first-line offset (padding, document margin) for gutter, minimap and sync.
                            readonly property real firstLineY: length >= 0 ? positionToRectangle(0).y : 0
                            readonly property real lineH: lineCount > 1
                                ? (positionToRectangle(length).y - firstLineY) / (lineCount - 1) : fm.lineSpacing
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
