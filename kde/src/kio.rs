//! Remote (`sftp://`, `smb://`, ...) file I/O via the KIO C++ shim (`kio_shim.cpp`).
//! Files with a non-file scheme are routed here instead of kio-fuse, whose write path
//! is unreliable (kio-fuse issue #10: EIO/EPERM on save).

use markite_core::location::redact;
use std::ffi::{c_char, c_longlong, CStr, CString};
use std::{io, ptr, slice};

extern "C" {
    fn markite_kio_read(
        url: *const c_char,
        data: *mut *mut c_char,
        len: *mut c_longlong,
        err: *mut *mut c_char,
    ) -> i32;
    fn markite_kio_write(
        url: *const c_char,
        data: *const c_char,
        len: c_longlong,
        err: *mut *mut c_char,
    ) -> i32;
    fn markite_kio_free(p: *mut c_char);
}

/// A NUL-terminated buffer allocated by the shim; freed exactly once, whatever path returns early.
struct ShimBuf(*mut c_char);

impl ShimBuf {
    fn null() -> Self {
        Self(ptr::null_mut())
    }

    /// Out-pointer for the shim to fill in.
    fn out(&mut self) -> *mut *mut c_char {
        &mut self.0
    }

    fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// The shim's error message, or a generic one if it allocated none.
    fn message(&self) -> String {
        if self.is_null() {
            return "unknown KIO error".into();
        }
        // SAFETY: non-null, and the shim always NUL-terminates what it allocates (`dup_bytes`).
        unsafe { CStr::from_ptr(self.0) }
            .to_string_lossy()
            .into_owned()
    }
}

impl Drop for ShimBuf {
    fn drop(&mut self) {
        // SAFETY: null (a no-op) or a pointer from the shim's malloc that nothing else owns or frees.
        unsafe { markite_kio_free(self.0) };
    }
}

/// KIO wants full URLs. Remote callers already pass one; a bare path (e.g. when exercising the shim
/// against a local file) becomes a file:// URL. A NUL byte can't cross the C boundary: that is an error, not a panic.
fn as_url(path: &str) -> io::Result<CString> {
    let url = if path.contains("://") {
        path.to_string()
    } else {
        format!("file://{path}")
    };
    markite_core::trace!("kio::as_url: {} -> {}", redact(path), redact(&url));
    CString::new(url)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL byte"))
}

fn shim_error(err: &ShimBuf, url: &str) -> io::Error {
    io::Error::other(format!("{}: {}", redact(url), err.message()))
}

/// Known limitation: the content is decoded as UTF-8 lossily, so a non-UTF-8 remote file is altered (and saved back altered).
pub fn read(path: &str) -> io::Result<String> {
    markite_core::trace!(
        "kio::read: {} (blocking KIO storedGet in the C++ shim)",
        redact(path)
    );
    let url = as_url(path)?;
    let (mut data, mut err) = (ShimBuf::null(), ShimBuf::null());
    let mut len: c_longlong = 0;
    // SAFETY: `url` is a valid NUL-terminated string for the whole call and the out-pointers refer to live locals.
    let rc = unsafe { markite_kio_read(url.as_ptr(), data.out(), &mut len, err.out()) };
    if rc != 0 {
        markite_core::trace!("kio::read: shim returned error code {rc}");
        return Err(shim_error(&err, path));
    }
    let len =
        usize::try_from(len).map_err(|_| io::Error::other("KIO returned a negative length"))?;
    if data.is_null() {
        return Err(io::Error::other(format!(
            "{}: KIO returned no data",
            redact(path)
        )));
    }
    // SAFETY: on success the shim sets `data` to a buffer of `len` readable bytes, owned by `data` until it drops.
    let bytes = unsafe { slice::from_raw_parts(data.0.cast::<u8>(), len) };
    markite_core::trace!("kio::read: {len} bytes received, decoded as UTF-8 (lossy)");
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

pub fn write(path: &str, text: &str) -> io::Result<()> {
    markite_core::trace!(
        "kio::write: {} bytes to {} (KIO storedPut, overwrite)",
        text.len(),
        redact(path)
    );
    let url = as_url(path)?;
    let len =
        c_longlong::try_from(text.len()).map_err(|_| io::Error::other("document too large"))?;
    let mut err = ShimBuf::null();
    // SAFETY: `url` and `text` stay valid for the call, and `len` is exactly `text.len()`.
    let rc = unsafe { markite_kio_write(url.as_ptr(), text.as_ptr().cast(), len, err.out()) };
    if rc != 0 {
        markite_core::trace!("kio::write: shim returned error code {rc}");
        return Err(shim_error(&err, path));
    }
    markite_core::trace!("kio::write: ok");
    Ok(())
}
