use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One open file. The frontend owns exactly one of these per editor tab/window
/// and forwards edits into it; it never keeps its own copy of the truth.
#[derive(Debug, Default)]
pub struct Document {
    text: String,
    path: Option<PathBuf>,
    saved_text: String,
}

impl Document {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let text = fs::read_to_string(&path)?;
        Ok(Self {
            saved_text: text.clone(),
            text,
            path: Some(path.as_ref().to_path_buf()),
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved_text
    }

    /// Replace the whole buffer. Fine for Markdown-sized files; a frontend with
    /// an incremental text model can switch to range edits later without
    /// changing callers.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn save(&mut self) -> io::Result<()> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "document has no path"))?;
        self.save_as(path)
    }

    pub fn save_as(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        fs::write(&path, &self.text)?;
        self.path = Some(path.as_ref().to_path_buf());
        self.saved_text = self.text.clone();
        Ok(())
    }

    pub fn render_html(&self) -> String {
        crate::render::to_html(&self.text)
    }

    pub fn render_blocks(&self) -> Vec<crate::blocks::Block> {
        crate::blocks::to_blocks(&self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_tracking_and_save_roundtrip() {
        let dir = std::env::temp_dir().join(format!("markite-core-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.md");

        let mut doc = Document::default();
        assert!(!doc.is_dirty());
        doc.set_text("# hi");
        assert!(doc.is_dirty());
        assert!(doc.save().is_err(), "no path yet");

        doc.save_as(&file).unwrap();
        assert!(!doc.is_dirty());

        let reopened = Document::open(&file).unwrap();
        assert_eq!(reopened.text(), "# hi");
        assert_eq!(reopened.path(), Some(file.as_path()));

        fs::remove_dir_all(dir).unwrap();
    }
}
