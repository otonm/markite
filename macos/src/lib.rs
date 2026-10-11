//! Conversion-only bridge from `markite-core` to Swift. Logic belongs in core, not here;
//! the one exception is `highlight`, which only this frontend uses (see its module docs).

mod convert;
mod doc;
mod errors;
mod highlight;

pub use convert::*;
pub use doc::{Doc, ExternalChangeKind};
pub use errors::MarkiteError;
pub use highlight::{markdown_spans, Span, SpanKind};

uniffi::setup_scaffolding!();

/// Whitespace-separated word count of a string, from core.
#[uniffi::export]
pub fn word_count(text: String) -> u64 {
    let mut doc = markite_core::Document::default();
    doc.set_text(text);
    doc.word_count() as u64
}
