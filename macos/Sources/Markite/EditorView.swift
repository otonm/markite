import AppKit
import SwiftUI

/// Hook for syntax highlighting (not implemented yet): called with the text storage after the text is loaded, edited
/// or the theme/font changes. Implementations must only change attributes, never the characters.
protocol SourceHighlighter {
    func highlight(_ storage: NSTextStorage, colors: ThemeColors, font: NSFont)
}

struct NoOpHighlighter: SourceHighlighter {
    func highlight(_ storage: NSTextStorage, colors: ThemeColors, font: NSFont) {}
}

/// Start offsets (UTF-16) of every source line, to convert between text offsets and 1-based line numbers.
struct LineIndex {
    private(set) var starts: [Int] = [0]

    init(_ text: NSString) {
        var i = 0
        let n = text.length
        while i < n {
            if text.character(at: i) == 10 { starts.append(i + 1) }
            i += 1
        }
    }

    /// 1-based line containing `offset`.
    func line(atOffset offset: Int) -> Int {
        var lo = 0
        var hi = starts.count - 1
        while lo < hi {
            let mid = (lo + hi + 1) / 2
            if starts[mid] <= offset { lo = mid } else { hi = mid - 1 }
        }
        return lo + 1
    }
}

/// The Markdown source editor: an `NSTextView` (TextKit 2) with the system find bar. The core document is the source
/// of truth: edits are pushed to it, and it tells the view (`editorGeneration`) when its text was replaced.
struct EditorView: NSViewRepresentable {
    let document: MarkiteDocument
    let generation: Int
    let colors: ThemeColors
    /// Points; 0 = default size.
    let fontSize: Double
    let wrap: Bool
    let wrapColumn: Int
    var highlighter: SourceHighlighter = NoOpHighlighter()

    func makeCoordinator() -> Coordinator { Coordinator(document: document) }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSTextView.scrollableTextView()
        guard let tv = scroll.documentView as? NSTextView else { return scroll }
        let c = context.coordinator
        c.scroll = scroll
        c.textView = tv
        tv.delegate = c
        tv.isRichText = false
        tv.allowsUndo = true
        tv.usesFindBar = true
        tv.isIncrementalSearchingEnabled = true
        tv.isAutomaticQuoteSubstitutionEnabled = false
        tv.isAutomaticDashSubstitutionEnabled = false
        tv.isAutomaticTextReplacementEnabled = false
        tv.isAutomaticSpellingCorrectionEnabled = false
        tv.isAutomaticLinkDetectionEnabled = false
        tv.textContainerInset = NSSize(width: 8, height: 8)
        tv.setAccessibilityLabel("Markdown source")
        scroll.hasVerticalScroller = true

        c.generation = generation
        c.setText(document.doc.text())
        c.observe(scroll: scroll)
        document.scrollEditor = { [weak c] line in c?.scrollTo(line: line) }
        apply(to: c)
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        let c = context.coordinator
        if c.generation != generation {
            c.generation = generation
            c.setText(document.doc.text())
        }
        apply(to: c)
    }

    static func dismantleNSView(_ scroll: NSScrollView, coordinator: Coordinator) {
        coordinator.document.scrollEditor = nil
        coordinator.stopObserving()
    }

    private func apply(to c: Coordinator) {
        c.highlighter = highlighter
        c.style(colors: colors, fontSize: fontSize, wrap: wrap, wrapColumn: wrapColumn)
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        let document: MarkiteDocument
        weak var scroll: NSScrollView?
        weak var textView: NSTextView?
        var generation = 0
        var highlighter: SourceHighlighter = NoOpHighlighter()

        private struct StyleKey: Equatable {
            let colors: ThemeColors
            let fontSize: Double
            let wrap: Bool
            let wrapColumn: Int
        }

        private var appliedStyle: StyleKey?
        private var colors = ThemeColors.system
        private var font = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        private var wrap = true
        private var wrapColumn = 100
        private var applying = false
        private var ignoreScroll = false
        private var lineIndex: LineIndex?
        private var observers: [NSObjectProtocol] = []

        init(document: MarkiteDocument) {
            self.document = document
        }

        deinit { stopObserving() }

        // MARK: Text

        /// Replaces the whole text (open, revert, reload) without telling the document back.
        func setText(_ text: String) {
            guard let tv = textView else { return }
            applying = true
            tv.string = text
            applying = false
            lineIndex = nil
            rehighlight()
        }

        func textDidChange(_ notification: Notification) {
            guard !applying, let tv = textView else { return }
            lineIndex = nil
            document.textEdited(tv.string)
            rehighlight()
        }

        private func rehighlight() {
            guard let storage = textView?.textStorage else { return }
            highlighter.highlight(storage, colors: colors, font: font)
        }

        // MARK: Style and wrapping

        func style(colors: ThemeColors, fontSize: Double, wrap: Bool, wrapColumn: Int) {
            guard let tv = textView, let scroll = scroll else { return }
            // Re-styling touches every character, so only do it when something actually changed.
            let key = StyleKey(colors: colors, fontSize: fontSize, wrap: wrap, wrapColumn: wrapColumn)
            if key == appliedStyle { return }
            appliedStyle = key
            self.colors = colors
            self.wrap = wrap
            self.wrapColumn = wrapColumn
            font = NSFont.monospacedSystemFont(ofSize: fontSize > 0 ? CGFloat(fontSize) : 13, weight: .regular)
            tv.font = font
            tv.textColor = colors.text
            tv.insertionPointColor = colors.text
            tv.backgroundColor = colors.background
            tv.drawsBackground = true
            tv.selectedTextAttributes = [.backgroundColor: colors.selection]
            tv.typingAttributes = [.font: font, .foregroundColor: colors.text]
            scroll.drawsBackground = true
            scroll.backgroundColor = colors.background
            applyWrap()
            rehighlight()
        }

        /// Wraps at `wrapColumn` characters, or at the pane edge when that is narrower.
        func applyWrap() {
            guard let tv = textView, let scroll = scroll, let container = tv.textContainer else { return }
            tv.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
            container.widthTracksTextView = false
            if wrap {
                scroll.hasHorizontalScroller = false
                tv.isHorizontallyResizable = false
                let charWidth = ("0" as NSString).size(withAttributes: [.font: font]).width
                let columnWidth = CGFloat(wrapColumn) * charWidth + 2 * container.lineFragmentPadding
                let available = scroll.contentSize.width - 2 * tv.textContainerInset.width
                container.containerSize = NSSize(
                    width: max(50, min(available, columnWidth)), height: CGFloat.greatestFiniteMagnitude)
            } else {
                scroll.hasHorizontalScroller = true
                tv.isHorizontallyResizable = true
                container.containerSize = NSSize(
                    width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
            }
        }

        // MARK: Scrolling

        func observe(scroll: NSScrollView) {
            scroll.postsFrameChangedNotifications = true
            scroll.contentView.postsBoundsChangedNotifications = true
            let center = NotificationCenter.default
            observers.append(
                center.addObserver(forName: NSView.frameDidChangeNotification, object: scroll, queue: .main) {
                    [weak self] _ in self?.applyWrap()
                })
            observers.append(
                center.addObserver(
                    forName: NSView.boundsDidChangeNotification, object: scroll.contentView, queue: .main
                ) { [weak self] _ in self?.scrolled() })
        }

        func stopObserving() {
            for o in observers { NotificationCenter.default.removeObserver(o) }
            observers.removeAll()
        }

        private func lines() -> LineIndex? {
            if lineIndex == nil, let tv = textView { lineIndex = LineIndex(tv.string as NSString) }
            return lineIndex
        }

        /// The user scrolled (or typing moved the view): tell the document the 1-based fractional top line.
        private func scrolled() {
            guard !ignoreScroll, let line = topLine() else { return }
            document.editorScrolled(line: line)
        }

        private func topLine() -> Double? {
            guard let tv = textView, let scroll = scroll, let index = lines(),
                let layout = tv.textLayoutManager, let content = layout.textContentManager
            else { return nil }
            let y = max(0, scroll.contentView.bounds.minY - tv.textContainerOrigin.y)
            guard let fragment = layout.textLayoutFragment(for: CGPoint(x: 1, y: y)) else { return nil }
            let offset = content.offset(
                from: content.documentRange.location, to: fragment.rangeInElement.location)
            let frame = fragment.layoutFragmentFrame
            let within = frame.height > 0 ? min(1, max(0, (y - frame.minY) / frame.height)) : 0
            return Double(index.line(atOffset: offset)) + Double(within)
        }

        /// Scrolls so that 1-based fractional source `line` is at the top, without echoing it back as a user scroll.
        func scrollTo(line: Double) {
            guard let tv = textView, let scroll = scroll, let index = lines(),
                let layout = tv.textLayoutManager, line.isFinite
            else { return }
            let whole = floor(line)
            let fraction = CGFloat(line - whole)
            let row = min(max(Int(whole) - 1, 0), index.starts.count - 1)
            guard let location = layout.location(layout.documentRange.location, offsetBy: index.starts[row])
            else { return }
            var top: CGFloat?
            _ = layout.enumerateTextLayoutFragments(from: location, options: [.ensuresLayout]) { fragment in
                let frame = fragment.layoutFragmentFrame
                top = frame.minY + fraction * frame.height
                return false
            }
            guard let y = top else { return }
            let clip = scroll.contentView
            let maxY = max(0, tv.frame.height - clip.bounds.height)
            let target = min(max(0, y + tv.textContainerOrigin.y), maxY)
            ignoreScroll = true
            clip.scroll(to: NSPoint(x: clip.bounds.minX, y: target))
            scroll.reflectScrolledClipView(clip)
            ignoreScroll = false
        }
    }
}
