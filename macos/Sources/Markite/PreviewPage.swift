// The preview page: static HTML shell, stylesheet and the user script that applies updates and reports scrolling.
// Adapted from the GNOME frontend's preview.html / preview.css / preview.js. The strings are raw literals so the
// backslashes and quotes inside them need no escaping.

enum PreviewPage {
    /// Name of the script message handler (`window.webkit.messageHandlers.markite`).
    static let handlerName = "markite"

    /// Complete page. The CSP forbids all script in the page itself (so Markdown can never run code); `script` is
    /// injected by WebKit as a user script, which the CSP does not apply to. Images may be data: or https: only.
    static let html: String = """
<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data: https:; base-uri 'none'; form-action 'none'">
<meta name="referrer" content="no-referrer">
<style>
\(PreviewPage.css)
</style>
<style id="dyn"></style>
</head>
<body><main id="c"></main></body>
</html>
"""

    static let css: String = #"""
:root {
  --bg: #fff; --fg: #000; --heading: #000; --code: #000; --code-bg: #eee; --link: #06c; --quote: #555; --sel: #9bf;
  --font: -apple-system, system-ui, sans-serif; --size: 16px; --maxw: none;
}
html { overflow-x: hidden; background: var(--bg); }
body {
  margin: 0; background: var(--bg); color: var(--fg);
  font-family: var(--font); font-size: var(--size); line-height: 1.5;
  font-kerning: normal; font-feature-settings: "liga" 1, "calt" 1, "kern" 1; text-rendering: optimizeLegibility;
  overflow-wrap: anywhere;
}
::selection { background: var(--sel); }
/* border-box: --maxw already includes the side padding, so the text column itself is the requested width. */
main { box-sizing: border-box; width: 100%; max-width: var(--maxw); margin: 0 auto; padding: 16px 24px; }
/* The block's own padding (not a margin) carries the gap, so a block's box tiles the page and
   position <-> (block, fraction) mapping has no dead zones. */
section { display: flow-root; padding-bottom: 0.75em; overflow-anchor: auto; }
section > :first-child { margin-top: 0; }
section > :last-child { margin-bottom: 0; }
h1, h2, h3, h4, h5, h6 { color: var(--heading); line-height: 1.25; margin: 1.2em 0 0.5em; }
h1 { font-size: 2em; } h2 { font-size: 1.5em; } h3 { font-size: 1.25em; } h4, h5, h6 { font-size: 1em; }
a { color: var(--link); }
p, ul, ol, pre, blockquote, table { margin: 0 0 0.75em; }
ul, ol { padding-left: 1.6em; }
li.task-list-item { list-style: none; margin-left: -1.4em; }
blockquote { color: var(--quote); margin-left: 0; padding-left: 1em; border-left: 3px solid color-mix(in srgb, var(--quote) 50%, transparent); }
code, pre { font-family: ui-monospace, "SF Mono", Menlo, monospace; color: var(--code); background: var(--code-bg); }
code { font-size: 0.9em; padding: 0.1em 0.3em; border-radius: 4px; }
/* Code scrolls inside itself so a long line never widens the page. */
pre { padding: 0.75em 1em; border-radius: 6px; overflow-x: auto; overflow-wrap: normal; line-height: 1.4; }
pre code { padding: 0; background: none; font-size: 0.9em; }
table { display: block; max-width: 100%; overflow-x: auto; border-collapse: collapse; }
th, td { border: 1px solid color-mix(in srgb, var(--fg) 25%, transparent); padding: 0.3em 0.7em; }
th { background: color-mix(in srgb, var(--fg) 8%, transparent); }
img { max-width: 100%; height: auto; }
hr { border: 0; border-top: 1px solid color-mix(in srgb, var(--fg) 25%, transparent); }
"""#

    static let script: String = #"""
// Page logic, injected as a user script at document start (the page itself has no script: its CSP forbids it).
// The app drives it through window.markite.apply(); it reports back with
// window.webkit.messageHandlers.markite.postMessage(<JSON string>).
(function () {
  'use strict';
  var htmls = [];       // block HTML last applied, to touch only the sections that changed
  var expected = null;  // scrollY we set ourselves: the scroll event it causes is not a user scroll
  var queued = false;
  var anchor = null;    // [block, fraction] at the top of the viewport, to hold the same text there when the layout reflows

  function post(msg) {
    try { window.webkit.messageHandlers.markite.postMessage(JSON.stringify(msg)); } catch (e) { /* no handler */ }
  }
  function sections() { return document.getElementById('c').children; }
  function docTop(el) { return el.getBoundingClientRect().top + window.scrollY; }

  // Runs fn and, if it moved the scroll position (clamping after the content shrank, a programmatic scroll),
  // remembers where, so that scroll event is not mistaken for the user.
  function quiet(fn) {
    var before = window.scrollY;
    fn();
    var after = window.scrollY; // reading it forces layout
    if (Math.abs(after - before) >= 0.5) {
      expected = after;
      requestAnimationFrame(function () { requestAnimationFrame(function () { expected = null; }); });
    }
  }

  function setBlocks(list) {
    var c = document.getElementById('c'), s = sections();
    for (var i = 0; i < list.length; i++) {
      if (i >= s.length) { var n = document.createElement('section'); n.dataset.i = i; c.appendChild(n); }
      if (htmls[i] !== list[i]) s[i].innerHTML = list[i];
    }
    while (s.length > list.length) c.removeChild(c.lastChild);
    htmls = list.slice();
  }

  function position() {
    var s = sections(), n = s.length;
    if (!n) return null;
    var y = window.scrollY;
    if (window.innerHeight + y >= document.documentElement.scrollHeight - 1 && y > 0) return [n - 1, 1];
    var lo = 0, hi = n - 1; // last section whose top is at or above the viewport top
    while (lo < hi) { var mid = (lo + hi + 1) >> 1; if (docTop(s[mid]) <= y + 0.5) lo = mid; else hi = mid - 1; }
    var h = s[lo].getBoundingClientRect().height;
    return [lo, h > 0 ? Math.min(1, Math.max(0, (y - docTop(s[lo])) / h)) : 0];
  }

  function scrollTo(i, f) {
    var s = sections();
    if (!s.length) return;
    i = Math.min(Math.max(0, Math.floor(i) || 0), s.length - 1);
    f = isFinite(f) ? Math.min(1, Math.max(0, f)) : 0;
    var r = s[i].getBoundingClientRect();
    anchor = [i, f];
    quiet(function () { window.scrollTo(0, docTop(s[i]) + f * r.height); });
  }

  window.markite = {
    apply: function (m) {
      // A new font or width reflows everything: keep the same text at the top instead of the same pixel offset.
      var keep = m.css !== undefined && !m.scroll ? position() : null;
      quiet(function () {
        if (m.css !== undefined) document.getElementById('dyn').textContent = m.css;
        if (m.blocks) setBlocks(m.blocks);
        if (keep) scrollTo(keep[0], keep[1]);
      });
      if (m.scroll) scrollTo(m.scroll[0], m.scroll[1]);
    },
    position: position,
  };

  window.addEventListener('scroll', function () {
    var ours = expected !== null && Math.abs(window.scrollY - expected) < 1;
    if (queued) return;
    queued = true;
    requestAnimationFrame(function () {
      queued = false;
      anchor = position();
      if (anchor && !ours) post({ t: 'scroll', i: anchor[0], f: anchor[1] });
    });
  }, { passive: true });
  // Pane resized (window, view-mode animation): the text rewraps, so put the last known position back.
  window.addEventListener('resize', function () { if (anchor) scrollTo(anchor[0], anchor[1]); });
})();
"""#
}
