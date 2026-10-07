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
fn load_and_save_with_roundtrip() {
    let mut doc = Document::default();
    doc.load("sftp://h/p.md", "remote");
    assert!(!doc.is_dirty());
    doc.set_text("edited");
    assert!(doc.is_dirty());

    let (mut path, mut written) = (String::new(), String::new());
    doc.save_with(|p, t| {
        path = p.display().to_string();
        written = t.to_string();
        Ok(())
    })
    .unwrap();
    assert_eq!((path.as_str(), written.as_str()), ("sftp://h/p.md", "edited"));
    assert!(!doc.is_dirty());
}
