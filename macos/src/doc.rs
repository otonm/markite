use crate::convert::{u32_sat, Block};
use crate::errors::{check_path, invalid, MarkiteError};
use markite_core::document::{ExternalChange, SavePlan};
use std::sync::{Mutex, MutexGuard};

#[derive(uniffi::Enum)]
pub enum ExternalChangeKind {
    Unchanged,
    Reloaded,
    Conflict,
}

#[derive(Default)]
struct Inner {
    doc: markite_core::Document,
    plan: Option<SavePlan>,
}

#[derive(uniffi::Object, Default)]
pub struct Doc(Mutex<Inner>);

impl Doc {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|p| {
            markite_core::trace!("Doc lock was poisoned; recovering");
            p.into_inner()
        })
    }
}

#[uniffi::export]
impl Doc {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&self, path: String, bytes: Vec<u8>) -> Result<(), MarkiteError> {
        let path = check_path(&path)?;
        let mut g = self.lock();
        g.doc.load(path, &bytes);
        g.plan = None;
        Ok(())
    }

    pub fn text(&self) -> String {
        self.lock().doc.text().to_owned()
    }

    pub fn set_text(&self, text: String) -> bool {
        self.lock().doc.set_text(text)
    }

    pub fn path(&self) -> Option<String> {
        self.lock()
            .doc
            .path()
            .map(|p| p.to_string_lossy().into_owned())
    }

    pub fn is_dirty(&self) -> bool {
        self.lock().doc.is_dirty()
    }

    pub fn encoding_label(&self) -> String {
        self.lock().doc.encoding_label()
    }

    pub fn line_ending_label(&self) -> String {
        self.lock().doc.line_ending_label().to_owned()
    }

    pub fn set_convert_encoding(&self, on: bool) {
        self.lock().doc.set_convert_encoding(on);
    }

    pub fn set_convert_line_endings(&self, on: bool) {
        self.lock().doc.set_convert_line_endings(on);
    }

    /// Encodes the buffer and keeps the plan until `finish_save`.
    pub fn prepare_save(&self) -> Result<Vec<u8>, MarkiteError> {
        let mut g = self.lock();
        let plan = g.doc.prepare_save()?;
        let bytes = plan.bytes.clone();
        g.plan = Some(plan);
        Ok(bytes)
    }

    pub fn finish_save(&self, path: String) -> Result<(), MarkiteError> {
        let path = check_path(&path)?;
        let mut g = self.lock();
        let plan = g
            .plan
            .take()
            .ok_or_else(|| invalid("finish_save without prepare_save"))?;
        g.doc.finish_save(path, plan);
        Ok(())
    }

    pub fn external_change(&self, bytes: Vec<u8>) -> ExternalChangeKind {
        match self.lock().doc.external_change(&bytes) {
            ExternalChange::Unchanged => ExternalChangeKind::Unchanged,
            ExternalChange::Reloaded => ExternalChangeKind::Reloaded,
            ExternalChange::Conflict => ExternalChangeKind::Conflict,
        }
    }

    pub fn word_count(&self) -> u64 {
        u64_of(self.lock().doc.word_count())
    }

    pub fn char_count(&self) -> u64 {
        u64_of(self.lock().doc.char_count())
    }

    pub fn render_blocks(&self, theme: String) -> Vec<Block> {
        self.lock()
            .doc
            .render_blocks(&theme)
            .into_iter()
            .map(|b| Block {
                start_line: u32_sat(b.start_line),
                end_line: u32_sat(b.end_line),
                html: b.html,
            })
            .collect()
    }
}

fn u64_of(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}
