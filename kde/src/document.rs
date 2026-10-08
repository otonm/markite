//! The cxx-qt bridge between Qt and the core. It mirrors `markite_core::Document`
//! into Q_PROPERTYs and forwards QML calls into it. No editor logic lives here.

use std::io;
use std::path::Path;
use std::pin::Pin;

use crate::kio;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QString, QStringList};
use markite_core::blocks::{self, Block};
use markite_core::document::ExternalChange;
use markite_core::location;

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qlist.h");
        type QList_i32 = cxx_qt_lib::QList<i32>;
        type QList_f64 = cxx_qt_lib::QList<f64>;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, text)]
        /// Rendered preview, one HTML fragment per top-level Markdown block.
        #[qproperty(QStringList, blocks_html)]
        #[qproperty(QString, path)]
        #[qproperty(bool, dirty)]
        /// Encoding and line ending of the file on disk, for the status bar.
        #[qproperty(QString, encoding)]
        #[qproperty(QString, line_ending)]
        #[qproperty(i32, word_count)]
        #[qproperty(i32, char_count)]
        type Document = super::DocumentRust;

        /// Called from QML `onTextChanged`; re-renders and updates `dirty` unless the text is what core already holds.
        #[qinvokable]
        fn update_text(self: Pin<&mut Document>, text: &QString);
        /// Editor theme name (e.g. "Breeze Dark"); colours fenced code in the preview.
        #[qinvokable]
        fn set_syntax_theme(self: Pin<&mut Document>, theme: &QString);
        /// Whether saving converts to UTF-8 / LF (true) or keeps the file's own encoding / line ending.
        #[qinvokable]
        fn set_conversion(self: Pin<&mut Document>, encoding: bool, line_endings: bool);
        #[qinvokable]
        fn open(self: Pin<&mut Document>, path: &QString);
        /// Re-read the file from disk (local or remote). Returns true if the buffer was reloaded (QML must then
        /// refresh the editor); emits `error` when the file changed but unsaved edits were kept.
        #[qinvokable]
        fn check_external(self: Pin<&mut Document>) -> bool;
        #[qinvokable]
        fn save(self: Pin<&mut Document>);
        #[qinvokable]
        fn save_as(self: Pin<&mut Document>, path: &QString);
        /// Flat [indent, len, kind, marker, ...] quadruples per line for the code map (kind: 0 blank, 1 heading, 2 code,
        /// 3 text, 4 list item, 5 quote, 6 table; marker: list marker length).
        #[qinvokable]
        fn minimap_rows(&self) -> QList_i32;
        /// Fractional 1-based source line -> [block index, fraction 0..1 through that block].
        #[qinvokable]
        fn line_to_block(&self, line: f64) -> QList_f64;
        /// Inverse of `line_to_block`.
        #[qinvokable]
        fn block_to_line(&self, index: i32, fraction: f64) -> f64;

        #[qsignal]
        fn error(self: Pin<&mut Document>, message: QString);
    }
}

pub struct DocumentRust {
    text: QString,
    blocks_html: QStringList,
    path: QString,
    dirty: bool,
    encoding: QString,
    line_ending: QString,
    word_count: i32,
    char_count: i32,
    inner: markite_core::Document,
    blocks: Vec<Block>,
    syntax_theme: String,
}

impl Default for DocumentRust {
    fn default() -> Self {
        let inner = markite_core::Document::default();
        Self {
            text: QString::default(),
            blocks_html: QStringList::default(),
            path: QString::default(),
            dirty: false,
            encoding: QString::from(&inner.encoding_label()),
            line_ending: QString::from(inner.line_ending_label()),
            word_count: 0,
            char_count: 0,
            inner,
            blocks: Vec::new(),
            syntax_theme: String::new(),
        }
    }
}

/// Saturating conversion for `Q_PROPERTY` counts: a document never realistically exceeds `i32::MAX` words.
fn to_i32(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// Writer for `Document::save_with`/`save_as_with` when the path is a remote URL.
fn kio_write(path: &Path, data: &[u8]) -> io::Result<()> {
    kio::write(&path.to_string_lossy(), data)
}

impl ffi::Document {
    /// Re-render from core and publish `blocks_html` + `dirty`. Never touches `text`
    /// (echoing it back into the TextArea would reset the cursor).
    fn rerender(mut self: Pin<&mut Self>) {
        markite_core::trace!(
            "bridge rerender: asking core for blocks (theme {:?})",
            self.rust().syntax_theme
        );
        let blocks = {
            let r = self.rust();
            r.inner.render_blocks(&r.syntax_theme)
        };
        let mut html = QList::<QString>::default();
        for b in &blocks {
            html.append(QString::from(&b.html));
        }
        let dirty = self.rust().inner.is_dirty();
        let words = to_i32(self.rust().inner.word_count());
        let chars = to_i32(self.rust().inner.char_count());
        markite_core::trace!(
            "bridge rerender: publishing {} blocks, dirty={dirty}, {words} words, {chars} chars",
            blocks.len()
        );
        self.as_mut().rust_mut().blocks = blocks;
        self.as_mut().set_blocks_html(QStringList::from(&html));
        self.as_mut().set_dirty(dirty);
        self.as_mut().set_word_count(words);
        self.set_char_count(chars);
    }

    fn sync_from_core(mut self: Pin<&mut Self>) {
        markite_core::trace!(
            "bridge sync_from_core: copying text and path from core into the QObject properties"
        );
        let (text, path, encoding, line_ending) = {
            let d = &self.rust().inner;
            (
                QString::from(d.text()),
                QString::from(
                    &d.path()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                ),
                QString::from(&d.encoding_label()),
                QString::from(d.line_ending_label()),
            )
        };
        self.as_mut().set_text(text);
        self.as_mut().set_path(path);
        self.as_mut().set_encoding(encoding);
        self.as_mut().set_line_ending(line_ending);
        self.rerender();
    }

    fn report<T>(mut self: Pin<&mut Self>, r: io::Result<T>) {
        match r {
            Ok(_) => self.sync_from_core(),
            Err(e) => {
                markite_core::trace!(
                    "bridge report: operation failed, emitting error() toast: {e}"
                );
                self.as_mut().error(QString::from(&e.to_string()))
            }
        }
    }

    pub fn update_text(mut self: Pin<&mut Self>, text: &QString) {
        markite_core::trace!(
            "update_text (QML onTextChanged): {} UTF-16 units",
            text.len()
        );
        if self.as_mut().rust_mut().inner.set_text(text.to_string()) {
            self.rerender();
        } else {
            markite_core::trace!(
                "update_text: identical to core's text (echo after open/load), skipping re-render"
            );
        }
    }

    pub fn set_syntax_theme(mut self: Pin<&mut Self>, theme: &QString) {
        markite_core::trace!("set_syntax_theme (QML themeName changed): {theme}");
        self.as_mut().rust_mut().syntax_theme = theme.to_string();
        self.rerender();
    }

    pub fn set_conversion(mut self: Pin<&mut Self>, encoding: bool, line_endings: bool) {
        markite_core::trace!("set_conversion: encoding {encoding}, line endings {line_endings}");
        let inner = &mut self.as_mut().rust_mut().inner;
        inner.set_convert_encoding(encoding);
        inner.set_convert_line_endings(line_endings);
    }

    pub fn open(mut self: Pin<&mut Self>, path: &QString) {
        let p = location::resolve(&path.to_string()); // kio-fuse paths become their sftp:// URL
        markite_core::trace!("open: requested {}", location::redact(&p));
        // Remote URLs (sftp://, ...) go through KIO; local paths use core's std I/O.
        let r = if location::is_remote(&p) {
            kio::read(&p).map(|bytes| self.as_mut().rust_mut().inner.load(&p, &bytes))
        } else {
            markite_core::Document::open(&p).map(|d| self.as_mut().rust_mut().inner = d)
        };
        self.report(r);
    }

    pub fn check_external(mut self: Pin<&mut Self>) -> bool {
        let Some(p) = self
            .rust()
            .inner
            .path()
            .map(|p| p.to_string_lossy().into_owned())
        else {
            return false;
        };
        // A read error (file briefly missing during an atomic save, network blip) is not worth a toast: retry next poll.
        let disk = if location::is_remote(&p) {
            kio::read(&p)
        } else {
            markite_core::document::read_limited(Path::new(&p))
        };
        let Ok(disk) = disk else {
            markite_core::trace!("check_external: could not read {}", location::redact(&p));
            return false;
        };
        match self.as_mut().rust_mut().inner.external_change(&disk) {
            ExternalChange::Unchanged => false,
            ExternalChange::Reloaded => {
                self.sync_from_core();
                true
            }
            ExternalChange::Conflict => {
                self.error(QString::from(
                    "File changed on disk; your unsaved edits were kept. Save to overwrite it.",
                ));
                false
            }
        }
    }

    pub fn save(mut self: Pin<&mut Self>) {
        let remote = self
            .rust()
            .inner
            .path()
            .is_some_and(|p| location::is_remote(&p.to_string_lossy()));
        markite_core::trace!("save requested (remote: {remote})");
        let inner = &mut self.as_mut().rust_mut().inner;
        let r = if remote {
            inner.save_with(kio_write)
        } else {
            inner.save()
        };
        self.report(r);
    }

    pub fn save_as(mut self: Pin<&mut Self>, path: &QString) {
        let p = location::resolve(&path.to_string());
        let remote = location::is_remote(&p);
        markite_core::trace!(
            "save_as: requested {} (remote: {remote})",
            location::redact(&p)
        );
        let inner = &mut self.as_mut().rust_mut().inner;
        let r = if remote {
            inner.save_as_with(&p, kio_write)
        } else {
            inner.save_as(&p)
        };
        self.report(r);
    }

    pub fn minimap_rows(&self) -> QList<i32> {
        use markite_core::minimap::{lines, LineKind};
        markite_core::trace!("minimap_rows: QML Canvas asked for the code map");
        let mut out = QList::<i32>::default();
        for l in lines(self.rust().inner.text()) {
            out.append(i32::from(l.indent));
            out.append(i32::from(l.len));
            out.append(match l.kind {
                LineKind::Blank => 0,
                LineKind::Heading => 1,
                LineKind::Code => 2,
                LineKind::Text => 3,
                LineKind::ListItem => 4,
                LineKind::Quote => 5,
                LineKind::Table => 6,
            });
            out.append(i32::from(l.marker));
        }
        out
    }

    pub fn line_to_block(&self, line: f64) -> QList<f64> {
        markite_core::trace!(
            "line_to_block: editor line {line} -> preview block (scroll sync editor->preview)"
        );
        let (i, f) = blocks::line_to_block(&self.rust().blocks, line);
        let mut out = QList::<f64>::default();
        out.append(i as f64);
        out.append(f);
        out
    }

    pub fn block_to_line(&self, index: i32, fraction: f64) -> f64 {
        markite_core::trace!("block_to_line: preview block {index} @ {fraction:.3} -> editor line (scroll sync preview->editor)");
        blocks::block_to_line(
            &self.rust().blocks,
            usize::try_from(index).unwrap_or(0),
            fraction,
        )
    }
}
