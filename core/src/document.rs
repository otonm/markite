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
    /// Errors carry the path so the toast says which file and operation failed,
    /// not just "os error 5".
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let text = fs::read_to_string(&path)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.as_ref().display())))?;
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

    /// Load content the frontend already fetched (e.g. a remote file via KIO). The path is
    /// kept as an opaque token for display and later saves; this crate never touches it itself.
    pub fn load(&mut self, path: impl AsRef<Path>, text: impl Into<String>) {
        self.path = Some(path.as_ref().to_path_buf());
        self.text = text.into();
        self.saved_text = self.text.clone();
    }

    /// Save through a caller-supplied writer (frontend remote I/O); dirty tracking stays here.
    pub fn save_with(&mut self, write: impl FnOnce(&Path, &str) -> io::Result<()>) -> io::Result<()> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "document has no path"))?;
        write(&path, &self.text)?;
        self.saved_text = self.text.clone();
        Ok(())
    }

    pub fn save(&mut self) -> io::Result<()> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "document has no path"))?;
        self.save_as(path)
    }

    pub fn save_as(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        fs::write(&path, &self.text)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.as_ref().display())))?;
        self.path = Some(path.as_ref().to_path_buf());
        self.saved_text = self.text.clone();
        Ok(())
    }

    /// Whitespace-separated token count, shown as the live word count.
    /// ponytail: counts Markdown syntax tokens (e.g. `#`, `-`) as words; refine if a
    /// real prose count is wanted.
    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }

    /// Characters excluding all whitespace (spaces, tabs, newlines).
    pub fn char_count(&self) -> usize {
        self.text.chars().filter(|c| !c.is_whitespace()).count()
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
        assert_eq!(reopened.word_count(), 2);
        assert_eq!(reopened.char_count(), 3); // "# hi" without the space
        assert_eq!(reopened.path(), Some(file.as_path()));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn load_and_save_with_roundtrip() {
        let mut doc = Document::default();
        doc.load("sftp://h/p.md", "remote");
        assert!(!doc.is_dirty());
        doc.set_text("edited");
        assert!(doc.is_dirty());

        let (mut path, mut written) = (String::new(), String::new());
        doc.save_with(|p, t| {
            path = p.display().to_string();
            written = t.to_string();
            Ok(())
        })
        .unwrap();
        assert_eq!((path.as_str(), written.as_str()), ("sftp://h/p.md", "edited"));
        assert!(!doc.is_dirty());
    }
}
