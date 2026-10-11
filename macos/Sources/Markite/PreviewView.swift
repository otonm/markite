import AppKit
import MarkiteBridge
import SwiftUI
import WebKit

/// Theme, font and width of the preview, as the CSS custom properties the page stylesheet reads.
struct PreviewStyle: Equatable {
    var css: String
    var background: NSColor

    /// `fontPt` nil = the page default; `widthPercent` 100 = full width. Only numbers and colours from our own
    /// theme code go into the CSS, never text from the document or from saved settings.
    static func make(colors: ThemeColors, dark: Bool, fontPt: Double?, widthPercent: Int) -> PreviewStyle {
        let bg = colors.background.cssHex(dark: dark)
        let fg = colors.text.cssHex(dark: dark)
        let codeBg = colors.codeBackground?.cssHex(dark: dark) ?? "color-mix(in srgb, \(fg) 8%, \(bg))"
        let size: String
        if let pt = fontPt, pt >= 4, pt <= 200 { size = "\(pt)pt" } else { size = "16px" }
        let maxWidth = (20..<100).contains(widthPercent) ? "\(widthPercent)%" : "none"
        let css =
            ":root{color-scheme:\(dark ? "dark" : "light");--bg:\(bg);--fg:\(fg);"
            + "--heading:\(colors.heading.cssHex(dark: dark));--code:\(colors.code.cssHex(dark: dark));"
            + "--code-bg:\(codeBg);--link:\(colors.link.cssHex(dark: dark));"
            + "--quote:\(colors.quote.cssHex(dark: dark));--sel:\(colors.selection.cssHex(dark: dark));"
            + "--size:\(size);--maxw:\(maxWidth)}"
        return PreviewStyle(css: css, background: colors.background)
    }
}

/// Context menu reduced to the copy entries.
final class PreviewWebView: WKWebView {
    private static let kept = ["WKMenuItemIdentifierCopy", "WKMenuItemIdentifierCopyLink"]

    override func willOpenMenu(_ menu: NSMenu, with event: NSEvent) {
        for item in menu.items where !Self.kept.contains(item.identifier?.rawValue ?? "") {
            menu.removeItem(item)
        }
    }
}

/// The rendered Markdown. The page is loaded once; blocks, style and scroll position are pushed in with a script call
/// afterwards, so the scroll position survives edits. The page is treated as hostile: its CSP forbids script and
/// remote content except https images, the only navigation allowed is the initial load, links open in the browser,
/// and the data store is not persistent.
struct PreviewView: NSViewRepresentable {
    let document: MarkiteDocument
    let themeName: String
    let style: PreviewStyle

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeNSView(context: Context) -> WKWebView {
        let c = context.coordinator
        c.onScrolled = { [weak document] index, fraction in
            document?.previewScrolled(index: index, fraction: fraction)
        }
        document.scrollPreview = { [weak c] index, fraction in c?.scrollTo(block: index, fraction: fraction) }
        c.apply(document: document, themeName: themeName, style: style)
        return c.webView
    }

    func updateNSView(_ view: WKWebView, context: Context) {
        context.coordinator.apply(document: document, themeName: themeName, style: style)
    }

    static func dismantleNSView(_ view: WKWebView, coordinator: Coordinator) {
        coordinator.teardown()
    }

    final class Coordinator: NSObject, WKNavigationDelegate, WKScriptMessageHandler {
        let webView: PreviewWebView
        var onScrolled: ((Int, Double) -> Void)?

        private var loaded = false
        private var initialNavigation = true
        private var blockCount = 0
        private var flushScheduled = false
        private var pendingBlocks: [String]?
        private var pendingCSS: String?
        private var pendingScroll: (block: Int, fraction: Double)?
        private var appliedRevision = -1
        private var appliedTheme = ""
        private var appliedStyle: PreviewStyle?
        private weak var content: WKUserContentController?

        override init() {
            let config = WKWebViewConfiguration()
            config.websiteDataStore = WKWebsiteDataStore.nonPersistent()
            config.preferences.javaScriptCanOpenWindowsAutomatically = false
            let controller = WKUserContentController()
            controller.addUserScript(
                WKUserScript(source: PreviewPage.script, injectionTime: .atDocumentStart, forMainFrameOnly: true))
            config.userContentController = controller
            webView = PreviewWebView(frame: .zero, configuration: config)
            content = controller
            super.init()
            // The controller keeps a strong reference to the handler until `teardown` removes it.
            controller.add(self, name: PreviewPage.handlerName)
            webView.navigationDelegate = self
            webView.setAccessibilityLabel("Markdown preview")
            webView.loadHTMLString(PreviewPage.html, baseURL: nil)
        }

        func teardown() {
            content?.removeScriptMessageHandler(forName: PreviewPage.handlerName)
            onScrolled = nil
        }

        /// Pushes what changed since the last call (text revision, theme, style) to the page.
        func apply(document: MarkiteDocument, themeName: String, style: PreviewStyle) {
            if style != appliedStyle {
                appliedStyle = style
                webView.underPageBackgroundColor = style.background
                pendingCSS = style.css
                schedule()
            }
            if document.revision != appliedRevision || themeName != appliedTheme {
                appliedRevision = document.revision
                appliedTheme = themeName
                let blocks = document.renderedBlocks(theme: themeName)
                blockCount = blocks.count
                pendingBlocks = blocks.map { $0.html }
                schedule()
            }
        }

        /// Programmatic scroll; the page does not report it back. Out-of-range values are clamped in the page.
        func scrollTo(block: Int, fraction: Double) {
            pendingScroll = (max(0, block), fraction.isFinite ? min(1, max(0, fraction)) : 0)
            schedule()
        }

        /// One flush per main-loop pass: a burst of calls sends only the last state.
        private func schedule() {
            if flushScheduled { return }
            flushScheduled = true
            DispatchQueue.main.async { [weak self] in self?.flush() }
        }

        private func flush() {
            flushScheduled = false
            guard loaded else { return }  // didFinish flushes again once the page is there
            var message: [String: Any] = [:]
            if let css = pendingCSS { message["css"] = css }
            if let blocks = pendingBlocks { message["blocks"] = blocks }
            if let s = pendingScroll { message["scroll"] = [s.block, s.fraction] as [Any] }
            pendingCSS = nil
            pendingBlocks = nil
            pendingScroll = nil
            guard !message.isEmpty,
                let data = try? JSONSerialization.data(withJSONObject: message),
                let json = String(data: data, encoding: .utf8)
            else { return }
            // A JSON document is a valid JS expression, so nothing in the text can end the call early.
            webView.evaluateJavaScript("window.markite.apply(\(json))", completionHandler: nil)
        }

        // MARK: WKNavigationDelegate

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            loaded = true
            flush()
        }

        func webView(
            _ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
            decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
        ) {
            if initialNavigation && navigationAction.navigationType == .other {
                initialNavigation = false
                decisionHandler(.allow)
                return
            }
            decisionHandler(.cancel)
            if navigationAction.navigationType == .linkActivated, let url = navigationAction.request.url,
                Self.isExternal(url)
            {
                NSWorkspace.shared.open(url)
            }
        }

        private static func isExternal(_ url: URL) -> Bool {
            guard let scheme = url.scheme?.lowercased() else { return false }
            return scheme == "http" || scheme == "https" || scheme == "mailto"
        }

        // MARK: WKScriptMessageHandler

        /// Page -> app message `{"t":"scroll","i":N,"f":F}`. Anything else is dropped; values are clamped because the
        /// page is not trusted.
        func userContentController(
            _ userContentController: WKUserContentController, didReceive message: WKScriptMessage
        ) {
            guard blockCount > 0, let text = message.body as? String,
                let data = text.data(using: .utf8),
                let object = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any],
                object["t"] as? String == "scroll",
                let i = (object["i"] as? NSNumber)?.intValue,
                let f = (object["f"] as? NSNumber)?.doubleValue, f.isFinite
            else { return }
            onScrolled?(min(max(i, 0), blockCount - 1), min(max(f, 0), 1))
        }
    }
}
