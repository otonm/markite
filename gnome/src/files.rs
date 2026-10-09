//! Remote file I/O through GIO/GVfs (the GNOME counterpart of the KDE frontend's KIO shim). Local files never come
//! here: core reads and writes them with `std::fs`. Everything is async, so a slow or prompting remote never blocks
//! the UI thread.

use std::future::Future;
use std::io;

use gtk::prelude::*;
use gtk::{gio, glib};
use markite_core::document::{too_large, MAX_FILE_BYTES};
use markite_core::location::redact;

fn io_error(file: &gio::File, e: &glib::Error) -> io::Error {
    io::Error::other(format!("{}: {}", redact(&file.uri()), e.message()))
}

/// Run `attempt`; if the location is not mounted yet and a `parent` window is given, mount it (GVfs asks for
/// credentials or a host-key confirmation through a `MountOperation` dialog) and try once more. Without a parent
/// (background polling) a missing mount is just an error, so a poll never pops up a password prompt.
async fn with_mount<T, Fut: Future<Output = Result<T, glib::Error>>>(
    file: &gio::File,
    parent: Option<&gtk::Window>,
    mut attempt: impl FnMut() -> Fut,
) -> io::Result<T> {
    match attempt().await {
        Err(e) if parent.is_some() && e.matches(gio::IOErrorEnum::NotMounted) => {
            markite_core::trace!("files: {} is not mounted, mounting", redact(&file.uri()));
            let op = gtk::MountOperation::new(parent);
            file.mount_enclosing_volume_future(gio::MountMountFlags::NONE, Some(&op))
                .await
                .map_err(|e| io_error(file, &e))?;
            attempt().await.map_err(|e| io_error(file, &e))
        }
        other => other.map_err(|e| io_error(file, &e)),
    }
}

/// The bytes of a remote file, refusing anything over the size limit before downloading it.
pub async fn read(file: &gio::File, parent: Option<&gtk::Window>) -> io::Result<Vec<u8>> {
    let info = with_mount(file, parent, || {
        file.query_info_future(
            gio::FILE_ATTRIBUTE_STANDARD_SIZE,
            gio::FileQueryInfoFlags::NONE,
            glib::Priority::DEFAULT,
        )
    })
    .await?;
    if u64::try_from(info.size()).unwrap_or(0) > MAX_FILE_BYTES {
        return Err(too_large());
    }
    let (bytes, _) = file
        .load_bytes_future()
        .await
        .map_err(|e| io_error(file, &e))?;
    // The size attribute can be missing or stale: check what actually arrived.
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(too_large());
    }
    Ok(bytes.to_vec())
}

/// Replace the remote file's content with `bytes`.
pub async fn write(file: &gio::File, parent: Option<&gtk::Window>, bytes: &[u8]) -> io::Result<()> {
    with_mount(file, parent, || async {
        file.replace_contents_future(
            bytes.to_vec(),
            None,
            false,
            gio::FileCreateFlags::REPLACE_DESTINATION,
        )
        .await
        .map(|_| ())
        .map_err(|(_, e)| e)
    })
    .await
}
