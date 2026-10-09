// Page logic, injected as a user script at document start (the page itself has no script: its CSP forbids it).
// Rust drives it through window.markite.apply(); it reports back with
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
