use crate::encoding::{self, Format};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One open file. The frontend owns exactly one of these per editor tab/window
/// and forwards edits into it; it never keeps its own copy of the truth.
#[derive(Debug)]
pub struct Document {
    text: String,
    path: Option<PathBuf>,
    saved_text: String,
    /// Disk content we already reported as a conflict, so a poll doesn't repeat the warning.
    conflict_seen: Option<String>,
    /// How the file is stored on disk right now (the buffer itself is always `\n`-separated Unicode).
    format: Format,
    /// On save, write UTF-8 / LF instead of keeping the file's own encoding / line ending.
    convert_encoding: bool,
    convert_eol: bool,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            text: String::new(),
            path: None,
            saved_text: String::new(),
            conflict_seen: None,
            format: Format::default(),
            convert_encoding: true,
            convert_eol: true,
        }
    }
}

/// The encoded bytes of a save in progress (see `Document::prepare_save`).
#[derive(Debug)]
pub struct SavePlan {
    pub bytes: Vec<u8>,
    text: String,
    format: Format,
}

/// What `Document::external_change` did with the content found on disk.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ExternalChange {
    Unchanged,
    /// The buffer was clean, so it now holds the new disk content.
    Reloaded,
    /// The buffer has unsaved edits: it is left alone (the user decides by saving or reopening).
    Conflict,
}

/// Largest Markdown file Markite will load, from disk or over KIO.
pub const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

pub fn too_large() -> io::Error {
    io::Error::new(
        io::ErrorKind::FileTooLarge,
        format!("file is larger than the {} MB limit", MAX_FILE_BYTES >> 20),
    )
}

/// Read a local file's bytes, refusing anything over `MAX_FILE_BYTES`. Reads at most one byte past
/// the limit, so a file that grows after a size check still can't be loaded whole.
pub fn read_limited(path: &Path) -> io::Result<Vec<u8>> {
    use io::Read;
    // Only regular files: opening a FIFO would block the caller (the UI thread) and a device could yield endless data.
    if !fs::metadata(path).map_err(with_path(path))?.is_file() {
        crate::trace!("read_limited: {} is not a regular file", path.display());
        return Err(with_path(path)(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        )));
    }
    let mut buf = Vec::new();
    let n = fs::File::open(path)
        .and_then(|f| f.take(MAX_FILE_BYTES + 1).read_to_end(&mut buf))
        .map_err(with_path(path))?;
    if n as u64 > MAX_FILE_BYTES {
        crate::trace!("read_limited: {} exceeds the limit", path.display());
        return Err(too_large());
    }
    Ok(buf)
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
        let bytes = read_limited(path)?;
        let mut doc = Self::default();
        doc.load(path, &bytes);
        Ok(doc)
    }

    /// Encoding of the file on disk, e.g. "UTF-8" or "windows-1252 BOM".
    pub fn encoding_label(&self) -> String {
        self.format.encoding_label()
    }

    /// "LF", "CRLF" or "CR".
    pub fn line_ending_label(&self) -> &'static str {
        self.format.eol.label()
    }

    pub fn set_convert_encoding(&mut self, on: bool) {
        self.convert_encoding = on;
    }

    pub fn set_convert_line_endings(&mut self, on: bool) {
        self.convert_eol = on;
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

    /// Load bytes the frontend already fetched (e.g. a remote file via KIO), detecting their encoding and line
    /// endings. The path is kept as a token for display and for `save_with`; `load` itself does no I/O. (`save`/`save_as`
    /// would write to it with `std::fs`, so frontends use `save_with`/`save_as_with` for non-local paths.)
    pub fn load(&mut self, path: impl AsRef<Path>, bytes: &[u8]) {
        let path = path.as_ref();
        self.path = Some(path.to_path_buf());
        (self.text, self.format) = encoding::decode(bytes, None);
        self.saved_text = self.text.clone();
        self.conflict_seen = None;
        crate::trace!(
            "load: {} bytes from frontend-provided path {} (clean)",
            self.text.len(),
            crate::location::redact(&path.to_string_lossy())
        );
    }

    /// Encode the buffer for saving (honouring the convert options) without touching any state, for frontends
    /// whose write is asynchronous: write `bytes`, then call `finish_save`. Edits made while the write is in
    /// flight stay dirty, because the plan remembers the text it encoded.
    pub fn prepare_save(&self) -> io::Result<SavePlan> {
        let (bytes, format) = encoding::encode(
            &self.text,
            self.format,
            self.convert_encoding,
            self.convert_eol,
        )?;
        Ok(SavePlan {
            bytes,
            text: self.text.clone(),
            format,
        })
    }

    /// Record that `plan` was written to `path`: the path becomes the document's path and dirty tracking resets
    /// to the text the plan encoded.
    pub fn finish_save(&mut self, path: impl AsRef<Path>, plan: SavePlan) {
        self.path = Some(path.as_ref().to_path_buf());
        self.format = plan.format;
        self.saved_text = plan.text;
        self.conflict_seen = None;
        crate::trace!("finish_save: path updated, saved text recorded");
    }

    /// Write the buffer to `path` through a caller-supplied writer (frontend remote I/O). On success the
    /// path becomes the document's path and dirty tracking resets; on failure nothing changes.
    pub fn save_as_with(
        &mut self,
        path: impl AsRef<Path>,
        write: impl FnOnce(&Path, &[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        let path = path.as_ref();
        crate::trace!(
            "save_as_with: {} bytes to {}",
            self.text.len(),
            crate::location::redact(&path.to_string_lossy())
        );
        let plan = self.prepare_save()?;
        write(path, &plan.bytes)?;
        self.finish_save(path, plan);
        Ok(())
    }

    /// `save_as_with` to the current path through a caller-supplied writer; dirty tracking stays here.
    pub fn save_with(
        &mut self,
        write: impl FnOnce(&Path, &[u8]) -> io::Result<()>,
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

    /// Compare the bytes a frontend just read from the document's path with the last saved/loaded content.
    /// A clean buffer follows the disk; a dirty one is kept and the change reported once as a conflict.
    pub fn external_change(&mut self, bytes: &[u8]) -> ExternalChange {
        let (disk, format) = encoding::decode(bytes, Some(self.format.encoding));
        if disk == self.saved_text || self.conflict_seen.as_deref() == Some(&disk) {
            return ExternalChange::Unchanged;
        }
        if self.is_dirty() {
            crate::trace!("external_change: disk changed but buffer has unsaved edits");
            self.conflict_seen = Some(disk);
            return ExternalChange::Conflict;
        }
        crate::trace!("external_change: reloading {} bytes from disk", disk.len());
        self.text = disk.clone();
        self.saved_text = disk;
        self.format = format;
        ExternalChange::Reloaded
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
