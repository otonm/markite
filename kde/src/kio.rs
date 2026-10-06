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

/// kio-fuse mirrors a KIO URL as `/run/user/<uid>/kio-fuse-<id>/<scheme>/<authority>/<path>`
/// (password stripped, decoded names — kiofusevfs.cpp `mapUrlToVfs`). Dolphin passes such a path
/// whenever the app isn't registered for the scheme, so rebuild the URL and still use KIO.
fn from_kio_fuse(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/run/user/")?;
    let (uid, rest) = rest.split_once('/')?;
    if uid.is_empty() || !uid.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let rest = rest.strip_prefix("kio-fuse-")?.split_once('/')?.1; // drop the mount id
    let (scheme, rest) = rest.split_once('/')?;
    if scheme != "sftp" {
        return None; // only the sftp worker is bundled
    }
    let (authority, tail) = rest.split_once('/')?;
    let mut url = format!("sftp://{authority}");
    for seg in tail.split('/') {
        url.push('/');
        url.push_str(&percent_encode(seg));
    }
    Some(url)
}

fn percent_encode(seg: &str) -> String {
    // Keep what QUrl allows in a path (RFC 3986 unreserved + sub-delims + ":@"); encode the rest.
    let mut out = String::with_capacity(seg.len());
    for b in seg.bytes() {
        let allowed = b.is_ascii_alphanumeric() || (b < 128 && "-._~!$&'()*+,;=:@".contains(b as char));
        if allowed {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Resolve any path the app receives: kio-fuse paths become their remote URL, everything else
/// (local paths, already-schemed URLs) stays as-is.
pub fn resolve(path: &str) -> String {
    from_kio_fuse(path).unwrap_or_else(|| path.to_string())
}

/// True for any URL with a non-`file` scheme; plain paths are local and use `core`'s std I/O.
pub fn is_remote(path: &str) -> bool {
    match path.find("://") {
        Some(i) => !path[..i].eq_ignore_ascii_case("file"),
        None => false,
    }
}

/// KIO wants full URLs; a bare path only reaches here in tests, so make it a file:// URL.
fn as_url(path: &str) -> CString {
    let url = if path.contains("://") { path.to_string() } else { format!("file://{path}") };
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
    let mut data: *mut c_char = ptr::null_mut();
    let mut len: c_longlong = 0;
    let mut err: *mut c_char = ptr::null_mut();
    let rc = unsafe { markite_kio_read(as_url(path).as_ptr(), &mut data, &mut len, &mut err) };
    if rc != 0 {
        return Err(take_err(err, path));
    }
    let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), len.max(0) as usize) };
    let text = String::from_utf8_lossy(bytes).into_owned();
    unsafe { markite_kio_free(data) };
    Ok(text)
}

pub fn write(path: &str, text: &str) -> io::Result<()> {
    let mut err: *mut c_char = ptr::null_mut();
    let rc = unsafe {
        markite_kio_write(as_url(path).as_ptr(), text.as_ptr().cast(), text.len() as c_longlong, &mut err)
    };
    if rc != 0 {
        return Err(take_err(err, path));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kio_fuse_paths_become_urls() {
        assert_eq!(
            resolve("/run/user/1000/kio-fuse-Ab12Cd/sftp/oton@server:2222/docs/my notes.md"),
            "sftp://oton@server:2222/docs/my%20notes.md"
        );
        assert_eq!(
            resolve("/run/user/1000/kio-fuse-x/sftp/oton@server/AGENTS.md"),
            "sftp://oton@server/AGENTS.md"
        );
    }

    #[test]
    fn everything_else_is_untouched() {
        assert_eq!(resolve("/tmp/plain.md"), "/tmp/plain.md");
        assert_eq!(resolve("sftp://h/p.md"), "sftp://h/p.md");
        assert_eq!(resolve("/run/user/1000/doc.md"), "/run/user/1000/doc.md");
        assert_eq!(resolve("/run/user/1000/kio-fuse-x/smb/h/p.md"), "/run/user/1000/kio-fuse-x/smb/h/p.md");
    }

    #[test]
    fn remote_detection() {
        assert!(is_remote("sftp://host/p.md"));
        assert!(!is_remote("file:///p.md"));
        assert!(!is_remote("/p.md"));
    }
}
