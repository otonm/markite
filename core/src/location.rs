//! Where a document lives: local path or remote URL. Pure string logic, so it is testable without
//! Qt/KIO; the frontend only does the actual remote I/O.

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
