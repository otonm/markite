use markite_core::Document;
use std::fs;

#[test]
fn dirty_tracking_and_save_roundtrip() {
    let dir = std::env::temp_dir().join(format!("markite-core-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("a.md");

    let mut doc = Document::default();
    assert!(!doc.is_dirty());
    doc.set_text("# hi");
    assert!(doc.is_dirty());
    assert!(doc.save().is_err(), "no path yet");

    doc.save_as(&file).unwrap();
    assert!(!doc.is_dirty());

    let reopened = Document::open(&file).unwrap();
    assert_eq!(reopened.text(), "# hi");
    assert_eq!(reopened.word_count(), 2);
    assert_eq!(reopened.char_count(), 3); // "# hi" without the space
    assert_eq!(reopened.path(), Some(file.as_path()));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn set_text_reports_whether_it_changed() {
    let mut doc = Document::default();
    assert!(!doc.set_text(""), "empty over empty is no change");
    assert!(doc.set_text("a"));
    assert!(!doc.set_text("a"));
    assert!(doc.is_dirty());
    doc.load("x.md", b"b");
    assert!(!doc.set_text("b"), "echo of loaded text");
    assert!(!doc.is_dirty());
    assert!(doc.set_text("c") && doc.is_dirty());
}

#[test]
fn load_and_save_with_roundtrip() {
    let mut doc = Document::default();
    doc.load("sftp://h/p.md", b"remote");
    assert!(!doc.is_dirty());
    doc.set_text("edited");
    assert!(doc.is_dirty());

    let (mut path, mut written) = (String::new(), String::new());
    doc.save_with(|p, t| {
        path = p.display().to_string();
        written = String::from_utf8(t.to_vec()).unwrap();
        Ok(())
    })
    .unwrap();
    assert_eq!(
        (path.as_str(), written.as_str()),
        ("sftp://h/p.md", "edited")
    );
    assert!(!doc.is_dirty());
}

#[test]
fn save_as_with_changes_nothing_when_the_writer_fails() {
    let mut doc = Document::default();
    doc.load("sftp://h/a.md", b"x");
    doc.set_text("y");
    assert!(doc
        .save_as_with("sftp://h/b.md", |_, _| Err(std::io::Error::other("boom")))
        .is_err());
    assert_eq!(doc.path().unwrap().to_str(), Some("sftp://h/a.md"));
    assert!(doc.is_dirty());
    doc.save_as_with("sftp://h/b.md", |_, _| Ok(())).unwrap();
    assert_eq!(doc.path().unwrap().to_str(), Some("sftp://h/b.md"));
    assert!(!doc.is_dirty());
}

#[test]
fn external_change_reloads_clean_and_keeps_dirty() {
    use markite_core::document::ExternalChange::*;
    let mut d = markite_core::Document::default();
    d.load("/x.md", b"a");
    assert_eq!(d.external_change(b"a"), Unchanged);
    assert_eq!(d.external_change(b"b"), Reloaded);
    assert_eq!((d.text(), d.is_dirty()), ("b", false));
    d.set_text("mine");
    assert_eq!(d.external_change(b"c"), Conflict);
    assert_eq!(d.external_change(b"c"), Unchanged); // reported once
    assert_eq!(d.text(), "mine");
}

#[test]
fn read_limited_reads_small_files() {
    let p = std::env::temp_dir().join("markite_read_limited.md");
    std::fs::write(&p, "hi").unwrap();
    assert_eq!(markite_core::document::read_limited(&p).unwrap(), b"hi");
    assert!(markite_core::document::read_limited(&p.with_extension("none")).is_err());
}

#[test]
fn detects_and_converts_encoding_and_line_endings() {
    // Latin-1 text with CRLF line endings.
    let bytes = b"caf\xe9\r\nbar\r\n";
    let mut d = Document::default();
    d.load("/x.md", bytes);
    assert_eq!(d.text(), "café\nbar\n");
    assert_eq!(d.encoding_label(), "windows-1252");
    assert_eq!(d.line_ending_label(), "CRLF");

    // Keeping the format round-trips the exact bytes.
    d.set_convert_encoding(false);
    d.set_convert_line_endings(false);
    let mut out = Vec::new();
    d.save_as_with("/x.md", |_, b| {
        out = b.to_vec();
        Ok(())
    })
    .unwrap();
    assert_eq!(out, bytes);

    // Converting (the default) writes UTF-8 / LF and updates the reported format.
    d.set_convert_encoding(true);
    d.set_convert_line_endings(true);
    d.save_as_with("/x.md", |_, b| {
        out = b.to_vec();
        Ok(())
    })
    .unwrap();
    assert_eq!(out, "café\nbar\n".as_bytes());
    assert_eq!(
        (d.encoding_label().as_str(), d.line_ending_label()),
        ("UTF-8", "LF")
    );
}

#[test]
fn utf16_bom_roundtrip_and_unmappable_text() {
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend("hi\n".encode_utf16().flat_map(u16::to_le_bytes));
    let mut d = Document::default();
    d.load("/x.md", &bytes);
    assert_eq!(
        (d.text(), d.encoding_label().as_str()),
        ("hi\n", "UTF-16LE BOM")
    );
    d.set_convert_encoding(false);
    let mut out = Vec::new();
    d.save_as_with("/x.md", |_, b| {
        out = b.to_vec();
        Ok(())
    })
    .unwrap();
    assert_eq!(out, bytes);

    d.load("/y.md", b"caf\xe9");
    d.set_convert_encoding(false);
    d.set_text("emoji \u{1F600}");
    assert!(d.save_as_with("/y.md", |_, _| Ok(())).is_err());
    assert!(d.is_dirty());
}
