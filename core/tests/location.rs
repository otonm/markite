use markite_core::location::{is_remote, resolve};

#[test]
fn kio_fuse_paths_become_urls() {
    assert_eq!(
        resolve("/run/user/1000/kio-fuse-Ab12Cd/sftp/oton@server:2222/docs/my notes.md"),
        "sftp://oton@server:2222/docs/my%20notes.md"
    );
    assert_eq!(
        resolve("/run/user/1000/kio-fuse-x/sftp/oton@server/NOTES.md"),
        "sftp://oton@server/NOTES.md"
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
