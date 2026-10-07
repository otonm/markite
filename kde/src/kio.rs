//! Remote (`sftp://`, `smb://`, ...) file I/O via the KIO C++ shim (`kio_shim.cpp`).
//! Files with a non-file scheme are routed here instead of kio-fuse, whose write path
//! is unreliable (kio-fuse issue #10: EIO/EPERM on save).

use std::ffi::{c_char, c_longlong, CStr, CString};
use std::io;
use std::ptr;

extern "C" {
    fn markite_kio_read(url: *const c_char, data: *mut *mut c_char, len: *mut c_longlong, err: *mut *mut c_char) -> i32;
    fn markite_kio_write(url: *const c_char, data: *const c_char, len: c_longlong, err: *mut *mut c_char) -> i32;
    fn markite_kio_free(p: *mut c_char);
}

/// KIO wants full URLs. Remote callers already pass one; a bare path (e.g. when exercising the shim
/// against a local file) becomes a file:// URL.
fn as_url(path: &str) -> CString {
    let url = if path.contains("://") { path.to_string() } else { format!("file://{path}") };
    markite_core::trace!("kio::as_url: {path} -> {url}");
    CString::new(url).expect("path contains NUL")
}

fn take_err(err: *mut c_char, url: &str) -> io::Error {
    let msg = if err.is_null() {
        "unknown KIO error".into()
    } else {
        unsafe { CStr::from_ptr(err) }.to_string_lossy().into_owned()
    };
    unsafe { markite_kio_free(err) };
    io::Error::other(format!("{url}: {msg}"))
}

pub fn read(path: &str) -> io::Result<String> {
    markite_core::trace!("kio::read: {path} (blocking KIO storedGet in the C++ shim)");
    let mut data: *mut c_char = ptr::null_mut();
    let mut len: c_longlong = 0;
    let mut err: *mut c_char = ptr::null_mut();
    let rc = unsafe { markite_kio_read(as_url(path).as_ptr(), &mut data, &mut len, &mut err) };
    if rc != 0 {
        markite_core::trace!("kio::read: shim returned error code {rc}");
        return Err(take_err(err, path));
    }
    let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), len.max(0) as usize) };
    let text = String::from_utf8_lossy(bytes).into_owned();
    markite_core::trace!("kio::read: {} bytes received, decoded as UTF-8 (lossy)", bytes.len());
    unsafe { markite_kio_free(data) };
    Ok(text)
}

pub fn write(path: &str, text: &str) -> io::Result<()> {
    markite_core::trace!("kio::write: {} bytes to {path} (KIO storedPut, overwrite)", text.len());
    let mut err: *mut c_char = ptr::null_mut();
    let rc = unsafe {
        markite_kio_write(as_url(path).as_ptr(), text.as_ptr().cast(), text.len() as c_longlong, &mut err)
    };
    if rc != 0 {
        markite_core::trace!("kio::write: shim returned error code {rc}");
        return Err(take_err(err, path));
    }
    markite_core::trace!("kio::write: ok");
    Ok(())
}
