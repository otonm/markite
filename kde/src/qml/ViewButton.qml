import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

// Toolbar button drawn as a plain frame: a half is filled when that pane is shown.
// Colours come from the theme's text colour, so it inverts with light/dark themes.
Controls.ToolButton {
    id: btn
    property bool active: false
    property string tip
    property bool showCode: false
    property bool showPreview: false
    property string iconName   // when set, shows this theme icon instead of the pane shapes

    padding: 4
    Controls.ToolTip.text: tip
    Controls.ToolTip.visible: hovered && !(iconName !== "" && active)
    Controls.ToolTip.delay: Kirigami.Units.toolTipDelay

    // Tint the active mode; the desktop style doesn't mark it clearly.
    background: Rectangle {
        radius: 3
        color: btn.active ? Qt.alpha(Kirigami.Theme.textColor, 0.2)
             : btn.hovered ? Qt.alpha(Kirigami.Theme.textColor, 0.08) : "transparent"
    }

    contentItem: Item {
        // Same 10:7 proportions as the menu icons; height ~ the neighbouring menu button.
        implicitWidth: 30
        implicitHeight: 21
        Kirigami.Icon {
            anchors.centerIn: parent
            width: Kirigami.Units.iconSizes.smallMedium; height: width
            visible: btn.iconName !== ""
            source: btn.iconName
        }
        Rectangle {
            visible: btn.iconName === ""
            anchors.fill: parent
            color: "transparent"
            radius: 2
            border.width: 1
            border.color: Kirigami.Theme.textColor
        }
        Rectangle {  // left half = code
            x: 4; y: 4; width: parent.width / 2 - 4.5; height: parent.height - 8
            visible: btn.iconName === "" && btn.showCode
            color: Kirigami.Theme.textColor
        }
        Rectangle {  // right half = preview
            x: parent.width / 2 + 0.5; y: 4; width: parent.width / 2 - 4.5; height: parent.height - 8
            visible: btn.iconName === "" && btn.showPreview
            color: Kirigami.Theme.textColor
        }
    }
}
