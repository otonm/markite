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
        crate::trace!("open: reading {}", path.as_ref().display());
        let text = fs::read_to_string(&path).map_err(|e| {
            crate::trace!("open: failed ({:?}): {e}", e.kind());
            io::Error::new(e.kind(), format!("{}: {e}", path.as_ref().display()))
        })?;
        crate::trace!("open: read {} bytes; saved_text = text (clean)", text.len());
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
    /// changing callers. Returns whether the text actually changed, so a frontend can skip
    /// re-rendering when it only echoes back what core already holds (e.g. right after `open`).
    pub fn set_text(&mut self, text: impl Into<String>) -> bool {
        let text = text.into();
        if text == self.text {
            crate::trace!("set_text: {} bytes, unchanged", text.len());
            return false;
        }
        self.text = text;
        crate::trace!("set_text: {} bytes, dirty={}", self.text.len(), self.is_dirty());
        true
    }

    /// Load content the frontend already fetched (e.g. a remote file via KIO). The path is kept
    /// as a token for display and for `save_with`; `load` itself does no I/O. (`save`/`save_as`
    /// would write to it with `std::fs`, so frontends use `save_with` for non-local paths.)
    pub fn load(&mut self, path: impl AsRef<Path>, text: impl Into<String>) {
        self.path = Some(path.as_ref().to_path_buf());
        self.text = text.into();
        self.saved_text = self.text.clone();
        crate::trace!("load: {} bytes from frontend-provided path {} (clean)", self.text.len(), path.as_ref().display());
    }

    /// Save through a caller-supplied writer (frontend remote I/O); dirty tracking stays here.
    pub fn save_with(&mut self, write: impl FnOnce(&Path, &str) -> io::Result<()>) -> io::Result<()> {
        let path = self.path.clone().ok_or_else(|| {
            crate::trace!("save_with: no path, refusing");
            io::Error::new(io::ErrorKind::NotFound, "document has no path")
        })?;
        crate::trace!("save_with: handing {} bytes for {} to the frontend writer", self.text.len(), path.display());
        write(&path, &self.text).inspect_err(|_e| crate::trace!("save_with: writer failed: {_e}"))?;
        self.saved_text = self.text.clone();
        crate::trace!("save_with: ok (clean)");
        Ok(())
    }

    pub fn save(&mut self) -> io::Result<()> {
        let path = self.path.clone().ok_or_else(|| {
            crate::trace!("save: no path yet, caller must use save_as");
            io::Error::new(io::ErrorKind::NotFound, "document has no path")
        })?;
        crate::trace!("save: writing to the existing path");
        self.save_as(path)
    }

    pub fn save_as(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        crate::trace!("save_as: writing {} bytes to {}", self.text.len(), path.as_ref().display());
        fs::write(&path, &self.text).map_err(|e| {
            crate::trace!("save_as: failed ({:?}): {e}", e.kind());
            io::Error::new(e.kind(), format!("{}: {e}", path.as_ref().display()))
        })?;
        self.path = Some(path.as_ref().to_path_buf());
        self.saved_text = self.text.clone();
        crate::trace!("save_as: ok, path updated (clean)");
        Ok(())
    }

    /// Whitespace-separated token count, shown as the live word count.
    /// Known limitation: Markdown syntax tokens (e.g. `#`, `-`) count as words; a real prose
    /// count would need to parse the Markdown.
    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }

    /// Characters excluding all whitespace (spaces, tabs, newlines).
    pub fn char_count(&self) -> usize {
        self.text.chars().filter(|c| !c.is_whitespace()).count()
    }

    pub fn render_blocks(&self, theme: &str) -> Vec<crate::blocks::Block> {
        crate::trace!("render_blocks: {} bytes, theme {theme:?}", self.text.len());
        crate::blocks::to_blocks(&self.text, theme)
    }
}
