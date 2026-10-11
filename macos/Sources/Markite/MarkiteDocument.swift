import AppKit
import MarkiteBridge
import SwiftUI

enum ViewMode: String, CaseIterable, Identifiable {
    case editor, split, preview
    var id: String { rawValue }
    var title: String {
        switch self {
        case .editor: return "Editor"
        case .split: return "Split"
        case .preview: return "Preview"
        }
    }
    /// With Command+Option: the View menu shortcut.
    var key: KeyEquivalent {
        switch self {
        case .editor: return "1"
        case .split: return "2"
        case .preview: return "3"
        }
    }
}

/// What the status bar shows; refreshed (debounced) after edits.
struct DocumentStatus: Equatable {
    var path: String?
    var encoding = ""
    var lineEnding = ""
    var words: UInt64 = 0
    var chars: UInt64 = 0
}

/// Window/file shell only: the text, encoding, dirty state and save logic live in the core `Doc`.
@objc(MarkiteDocument)
final class MarkiteDocument: NSDocument, ObservableObject {
    private static let pollInterval: TimeInterval = 2
    private static let refreshDelay: TimeInterval = 0.15

    let doc = Doc()

    @Published var viewMode: ViewMode = .split
    @Published private(set) var status = DocumentStatus()
    /// Bumped (debounced) after every change to the text; the preview re-renders when it moves.
    @Published private(set) var revision = 0
    /// Bumped when the text was replaced from outside the editor (open, revert, reload): the editor reloads it.
    @Published private(set) var editorGeneration = 0
    /// Short message for the status bar (external change, ...); clears itself.
    @Published private(set) var notice: String?

    /// Scroll sync hooks, set by the editor and preview while they are on screen.
    var scrollPreview: ((Int, Double) -> Void)?
    var scrollEditor: ((Double) -> Void)?

    private var blocks: [Block] = []
    private var blocksKey: (Int, String)?
    private var refreshTimer: Timer?
    private var pollTimer: Timer?
    private var polling = false
    private var noticeToken = 0

    // MARK: NSDocument

    override class var autosavesInPlace: Bool { false }

    override init() {
        super.init()
        refreshStatus()
    }

    override func makeWindowControllers() {
        let host = NSHostingController(rootView: ContentView(document: self))
        host.sizingOptions = []  // the window keeps the size we give it instead of following the content
        let window = NSWindow(contentViewController: host)
        window.setContentSize(NSSize(width: 1000, height: 700))
        window.tabbingMode = .disallowed
        let controller = NSWindowController(window: window)
        addWindowController(controller)
        startPolling()
    }

    override func close() {
        pollTimer?.invalidate()
        pollTimer = nil
        refreshTimer?.invalidate()
        refreshTimer = nil
        scrollPreview = nil
        scrollEditor = nil
        super.close()
    }

    override func read(from data: Data, ofType typeName: String) throws {
        try load(bytes: data, path: fileURL?.path ?? "")
    }

    /// Reads through core's size-limited reader so a huge file is refused before it is loaded into memory.
    override func read(from url: URL, ofType typeName: String) throws {
        let bytes: Data
        do {
            bytes = try readLimited(path: url.path)
        } catch {
            throw Self.nsError(error, writing: false)
        }
        try load(bytes: bytes, path: url.path)
    }

    override func data(ofType typeName: String) throws -> Data {
        do {
            return try doc.prepareSave()
        } catch {
            throw Self.nsError(error, writing: true)
        }
    }

    /// The final URL is only known here (the actual write may go to a temporary file first), so this is where core is
    /// told the save succeeded. "Save a copy" leaves the document's own path and dirty state alone.
    override func save(
        to url: URL, ofType typeName: String, for saveOperation: NSDocument.SaveOperationType,
        completionHandler: @escaping (Error?) -> Void
    ) {
        super.save(to: url, ofType: typeName, for: saveOperation) { [weak self] error in
            guard error == nil, saveOperation != .saveToOperation, let self = self else {
                completionHandler(error)
                return
            }
            do {
                try self.doc.finishSave(path: url.path)
                self.refreshStatus()
                completionHandler(nil)
            } catch {
                completionHandler(Self.nsError(error, writing: true))
            }
        }
    }

    // MARK: Text

    private func load(bytes: Data, path: String) throws {
        do {
            try doc.load(path: path, bytes: bytes)
        } catch {
            throw Self.nsError(error, writing: false)
        }
        contentReplaced()
    }

    /// The editor changed its text. `setText` reports whether anything changed, so redundant calls are free.
    func textEdited(_ text: String) {
        guard doc.setText(text: text) else { return }
        refreshTimer?.invalidate()
        refreshTimer = Timer.scheduledTimer(withTimeInterval: Self.refreshDelay, repeats: false) { [weak self] _ in
            self?.refreshStatus()
        }
    }

    private func contentReplaced() {
        undoManager?.removeAllActions()
        editorGeneration += 1
        refreshStatus()
    }

    private func refreshStatus() {
        status = DocumentStatus(
            path: doc.path(), encoding: doc.encodingLabel(), lineEnding: doc.lineEndingLabel(),
            words: doc.wordCount(), chars: doc.charCount())
        revision += 1
    }

    /// Rendered blocks for the current text and theme; cached until either changes.
    func renderedBlocks(theme: String) -> [Block] {
        if let key = blocksKey, key.0 == revision, key.1 == theme { return blocks }
        blocks = doc.renderBlocks(theme: theme)
        blocksKey = (revision, theme)
        return blocks
    }

    // MARK: Scroll sync (1-based fractional source lines <-> block index and fraction)

    func editorScrolled(line: Double) {
        guard !blocks.isEmpty, line.isFinite else { return }
        let p = lineToBlock(blocks: blocks, line: line)
        scrollPreview?(Int(p.index), p.fraction)
    }

    func previewScrolled(index: Int, fraction: Double) {
        guard !blocks.isEmpty, index >= 0 else { return }
        let line = blockToLine(blocks: blocks, index: UInt32(clamping: index), fraction: fraction)
        if line.isFinite { scrollEditor?(line) }
    }

    // MARK: Watching the file on disk

    private func startPolling() {
        guard pollTimer == nil else { return }
        pollTimer = Timer.scheduledTimer(withTimeInterval: Self.pollInterval, repeats: true) { [weak self] _ in
            self?.poll()
        }
    }

    /// The read happens off the main thread (it may be a large file); the comparison and reload happen on it.
    private func poll() {
        guard !polling, let path = doc.path() else { return }
        polling = true
        DispatchQueue.global(qos: .utility).async { [weak self] in
            // A read error (file briefly missing during another program's atomic save) is retried next tick.
            let bytes = try? readLimited(path: path)
            DispatchQueue.main.async {
                guard let self = self else { return }
                self.polling = false
                if let bytes = bytes, self.doc.path() == path { self.applyExternal(bytes) }
            }
        }
    }

    private func applyExternal(_ bytes: Data) {
        switch doc.externalChange(bytes: bytes) {
        case .unchanged:
            break
        case .reloaded:
            contentReplaced()
            updateChangeCount(.changeCleared)
            show(notice: "File changed on disk: reloaded")
        case .conflict:
            show(notice: "File changed on disk; your unsaved edits were kept. Save to overwrite it.")
        }
    }

    private func show(notice text: String) {
        notice = text
        noticeToken += 1
        let token = noticeToken
        DispatchQueue.main.asyncAfter(deadline: .now() + 8) { [weak self] in
            if let self = self, self.noticeToken == token { self.notice = nil }
        }
    }

    // MARK: Errors

    /// Core errors as `NSError`s with readable messages, so NSDocument's standard error alert shows something useful.
    static func nsError(_ error: Error, writing: Bool) -> NSError {
        let code: CocoaError.Code
        let message: String
        if let e = error as? MarkiteError {
            switch e {
            case .NotFound:
                code = .fileNoSuchFile
                message = "The file could not be found."
            case .PermissionDenied:
                code = writing ? .fileWriteNoPermission : .fileReadNoPermission
                message = writing
                    ? "You do not have permission to save this file." : "You do not have permission to open this file."
            case .TooLarge:
                code = .fileReadTooLarge
                message = "The file is too large to open."
            case .InvalidInput(let detail):
                code = writing ? .fileWriteUnknown : .fileReadUnknown
                message = detail
            case .Other(let detail):
                code = writing ? .fileWriteUnknown : .fileReadUnknown
                message = detail
            }
        } else {
            return error as NSError
        }
        return NSError(
            domain: NSCocoaErrorDomain, code: code.rawValue, userInfo: [NSLocalizedDescriptionKey: message])
    }
}
