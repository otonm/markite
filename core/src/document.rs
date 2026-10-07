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

    pub fn render_blocks(&self, theme: &str) -> Vec<crate::blocks::Block> {
        crate::blocks::to_blocks(&self.text, theme)
    }
}
