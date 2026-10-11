import AppKit
import SwiftUI

@main
struct MarkiteApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate

    // Documents are NSDocuments with their own windows (see MarkiteDocument), so the only SwiftUI scene is Settings.
    var body: some Scene {
        Settings { SettingsView() }
            .commands { MarkiteCommands() }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationWillFinishLaunching(_ notification: Notification) {
        // One window per document, never merged into tabs.
        NSWindow.allowsAutomaticWindowTabbing = false
        // Instantiates the document controller from the Info.plist document types.
        _ = NSDocumentController.shared
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        // SwiftUI builds the menu bar after launch, so wait a turn before adding to it.
        DispatchQueue.main.async { Self.addOpenRecentMenu() }
    }

    /// Clicking the Dock icon with no window open starts a new document.
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if !flag { NSDocumentController.shared.newDocument(nil) }
        return false
    }

    /// AppKit fills a submenu that contains an item with the `clearRecentDocuments:` action with the recent files.
    private static func addOpenRecentMenu() {
        guard let file = NSApp.mainMenu?.items.first(where: { $0.title == "File" })?.submenu,
            !file.items.contains(where: { $0.title == "Open Recent" })
        else { return }
        let recent = NSMenu(title: "Open Recent")
        recent.addItem(
            NSMenuItem(title: "Clear Menu", action: #selector(NSDocumentController.clearRecentDocuments(_:)), keyEquivalent: ""))
        let item = NSMenuItem(title: "Open Recent", action: nil, keyEquivalent: "")
        item.submenu = recent
        let afterOpen = file.items.firstIndex(where: { $0.title.hasPrefix("Open") }).map { $0 + 1 } ?? file.items.count
        file.insertItem(item, at: afterOpen)
    }
}

/// File, View and Find entries. Edit (undo, copy, paste, ...) and Window use the standard menus.
struct MarkiteCommands: Commands {
    @AppStorage(Pref.wrapText) private var wrapText = true

    var body: some Commands {
        CommandGroup(replacing: .newItem) {
            Button("New") { NSDocumentController.shared.newDocument(nil) }
                .keyboardShortcut("n")
            Button("Open…") { NSDocumentController.shared.openDocument(nil) }
                .keyboardShortcut("o")
        }
        CommandGroup(replacing: .saveItem) {
            Button("Close") { NSApp.keyWindow?.performClose(nil) }
                .keyboardShortcut("w")
            Button("Save") { NSDocumentController.shared.currentDocument?.save(nil) }
                .keyboardShortcut("s")
            Button("Save As…") { NSDocumentController.shared.currentDocument?.saveAs(nil) }
                .keyboardShortcut("s", modifiers: [.command, .shift])
            Button("Revert to Saved") { NSDocumentController.shared.currentDocument?.revertToSaved(nil) }
        }
        CommandGroup(after: .pasteboard) {
            Divider()
            Menu("Find") {
                Button("Find…") { findAction(.showFindInterface) }
                    .keyboardShortcut("f")
                Button("Find Next") { findAction(.nextMatch) }
                    .keyboardShortcut("g")
                Button("Find Previous") { findAction(.previousMatch) }
                    .keyboardShortcut("g", modifiers: [.command, .shift])
                Button("Use Selection for Find") { findAction(.setSearchString) }
                    .keyboardShortcut("e")
            }
        }
        CommandGroup(before: .toolbar) {
            ForEach(ViewMode.allCases) { mode in
                Button(mode.title) { MarkiteCommands.setViewMode(mode) }
                    .keyboardShortcut(mode.key, modifiers: [.command, .option])
            }
            Toggle("Wrap Text", isOn: $wrapText)
            Divider()
        }
    }

    private static func setViewMode(_ mode: ViewMode) {
        (NSDocumentController.shared.currentDocument as? MarkiteDocument)?.viewMode = mode
    }

    /// Sends an `NSTextFinder` action to the first responder (the editor's text view) the way a menu item would.
    private func findAction(_ action: NSTextFinder.Action) {
        let sender = NSMenuItem()
        sender.tag = action.rawValue
        NSApp.sendAction(Selector(("performTextFinderAction:")), to: nil, from: sender)
    }
}
