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

/// Errors carry the path so the toast says which file and operation failed, not just "os error 5".
fn with_path(path: &Path) -> impl FnOnce(io::Error) -> io::Error + '_ {
    move |e| {
        crate::trace!(
            "I/O failed on {} ({:?}): {e}",
            crate::location::redact(&path.to_string_lossy()),
            e.kind()
        );
        io::Error::new(e.kind(), format!("{}: {e}", path.display()))
    }
}

fn no_path() -> io::Error {
    crate::trace!("document has no path yet");
    io::Error::new(io::ErrorKind::NotFound, "document has no path")
}

impl Document {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        crate::trace!("open: reading {}", path.display());
        let text = fs::read_to_string(path).map_err(with_path(path))?;
        crate::trace!("open: read {} bytes; saved_text = text (clean)", text.len());
        Ok(Self {
            saved_text: text.clone(),
            text,
            path: Some(path.to_path_buf()),
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
        crate::trace!(
            "set_text: {} bytes, dirty={}",
            self.text.len(),
            self.is_dirty()
        );
        true
    }

    /// Load content the frontend already fetched (e.g. a remote file via KIO). The path is kept
    /// as a token for display and for `save_with`; `load` itself does no I/O. (`save`/`save_as`
    /// would write to it with `std::fs`, so frontends use `save_with`/`save_as_with` for non-local paths.)
    pub fn load(&mut self, path: impl AsRef<Path>, text: impl Into<String>) {
        let path = path.as_ref();
        self.path = Some(path.to_path_buf());
        self.text = text.into();
        self.saved_text = self.text.clone();
        crate::trace!(
            "load: {} bytes from frontend-provided path {} (clean)",
            self.text.len(),
            crate::location::redact(&path.to_string_lossy())
        );
    }

    /// Write the buffer to `path` through a caller-supplied writer (frontend remote I/O). On success the
    /// path becomes the document's path and dirty tracking resets; on failure nothing changes.
    pub fn save_as_with(
        &mut self,
        path: impl AsRef<Path>,
        write: impl FnOnce(&Path, &str) -> io::Result<()>,
    ) -> io::Result<()> {
        let path = path.as_ref();
        crate::trace!(
            "save_as_with: {} bytes to {}",
            self.text.len(),
            crate::location::redact(&path.to_string_lossy())
        );
        write(path, &self.text)?;
        self.path = Some(path.to_path_buf());
        self.saved_text = self.text.clone();
        crate::trace!("save_as_with: ok, path updated (clean)");
        Ok(())
    }

    /// `save_as_with` to the current path through a caller-supplied writer; dirty tracking stays here.
    pub fn save_with(
        &mut self,
        write: impl FnOnce(&Path, &str) -> io::Result<()>,
    ) -> io::Result<()> {
        let path = self.path.clone().ok_or_else(no_path)?;
        self.save_as_with(path, write)
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.save_with(|p, t| fs::write(p, t).map_err(with_path(p)))
    }

    pub fn save_as(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        self.save_as_with(path, |p, t| fs::write(p, t).map_err(with_path(p)))
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
