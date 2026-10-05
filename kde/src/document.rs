//! The only file that knows both Qt and the core. It mirrors `markite_core::Document`
//! into Q_PROPERTYs and forwards QML calls into it. No editor logic lives here.

use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QString, QStringList};
use markite_core::blocks::{self, Block};

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
        type Document = super::DocumentRust;

        /// Called from QML `onTextChanged`; re-renders and updates `dirty`.
        #[qinvokable]
        fn update_text(self: Pin<&mut Document>, text: &QString);
        #[qinvokable]
        fn open(self: Pin<&mut Document>, path: &QString);
        #[qinvokable]
        fn save(self: Pin<&mut Document>);
        #[qinvokable]
        fn save_as(self: Pin<&mut Document>, path: &QString);
        /// Flat [indent, len, kind, ...] triples per line for the code map (kind: 0 blank, 1 heading, 2 code, 3 text).
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

#[derive(Default)]
pub struct DocumentRust {
    text: QString,
    blocks_html: QStringList,
    path: QString,
    dirty: bool,
    inner: markite_core::Document,
    blocks: Vec<Block>,
}

impl ffi::Document {
    /// Re-render from core and publish `blocks_html` + `dirty`. Never touches `text`
    /// (echoing it back into the TextArea would reset the cursor).
    fn rerender(mut self: Pin<&mut Self>) {
        let blocks = self.rust().inner.render_blocks();
        let mut html = QList::<QString>::default();
        for b in &blocks {
            html.append(QString::from(&b.html));
        }
        let dirty = self.rust().inner.is_dirty();
        self.as_mut().rust_mut().blocks = blocks;
        self.as_mut().set_blocks_html(QStringList::from(&html));
        self.set_dirty(dirty);
    }

    fn sync_from_core(mut self: Pin<&mut Self>) {
        let (text, path) = {
            let d = &self.rust().inner;
            (
                QString::from(d.text()),
                QString::from(&d.path().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()),
            )
        };
        self.as_mut().set_text(text);
        self.as_mut().set_path(path);
        self.rerender();
    }

    fn report<T>(mut self: Pin<&mut Self>, r: std::io::Result<T>) {
        match r {
            Ok(_) => self.sync_from_core(),
            Err(e) => self.as_mut().error(QString::from(&e.to_string())),
        }
    }

    pub fn update_text(mut self: Pin<&mut Self>, text: &QString) {
        self.as_mut().rust_mut().inner.set_text(text.to_string());
        self.rerender();
    }

    pub fn open(mut self: Pin<&mut Self>, path: &QString) {
        let r = markite_core::Document::open(path.to_string()).map(|d| {
            self.as_mut().rust_mut().inner = d;
        });
        self.report(r);
    }

    pub fn save(mut self: Pin<&mut Self>) {
        let r = self.as_mut().rust_mut().inner.save();
        self.report(r);
    }

    pub fn save_as(mut self: Pin<&mut Self>, path: &QString) {
        let r = self.as_mut().rust_mut().inner.save_as(path.to_string());
        self.report(r);
    }

    pub fn minimap_rows(&self) -> QList<i32> {
        use markite_core::minimap::{lines, LineKind};
        let mut out = QList::<i32>::default();
        for l in lines(self.rust().inner.text()) {
            out.append(l.indent as i32);
            out.append(l.len as i32);
            out.append(match l.kind {
                LineKind::Blank => 0,
                LineKind::Heading => 1,
                LineKind::Code => 2,
                LineKind::Text => 3,
            });
        }
        out
    }

    pub fn line_to_block(&self, line: f64) -> QList<f64> {
        let (i, f) = blocks::line_to_block(&self.rust().blocks, line);
        let mut out = QList::<f64>::default();
        out.append(i as f64);
        out.append(f);
        out
    }

    pub fn block_to_line(&self, index: i32, fraction: f64) -> f64 {
        blocks::block_to_line(&self.rust().blocks, index.max(0) as usize, fraction)
    }
}
