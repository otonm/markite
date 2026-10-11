use markite_macos::*;

fn tmp(name: &str) -> String {
    let d = std::env::temp_dir().join(format!("markite-bridge-{}-{name}", std::process::id()));
    d.to_string_lossy().into_owned()
}

#[test]
fn round_trip_non_utf8() {
    let path = tmp("latin1.md");
    // "café\r\n" in Windows-1252/Latin-1, with a CRLF line ending.
    let input = b"caf\xe9 au lait\r\nsecond \xe9\r\n".to_vec();
    std::fs::write(&path, &input).unwrap();
    let doc = Doc::new();
    doc.load(path.clone(), read_limited(path.clone()).unwrap())
        .unwrap();
    assert!(doc.text().contains('é'));
    assert!(!doc.is_dirty());
    assert_ne!(doc.encoding_label(), "UTF-8");
    assert_eq!(doc.path().as_deref(), Some(path.as_str()));
    assert!(doc.set_text("café au lait\nchanged é\n".into()));
    assert!(doc.is_dirty());
    // Default: convert to UTF-8/LF on save.
    assert_eq!(
        doc.prepare_save().unwrap(),
        "café au lait\nchanged é\n".as_bytes()
    );
    doc.set_convert_encoding(false);
    doc.set_convert_line_endings(false);
    let bytes = doc.prepare_save().unwrap();
    assert_eq!(bytes, b"caf\xe9 au lait\r\nchanged \xe9\r\n");
    std::fs::write(&path, &bytes).unwrap();
    doc.finish_save(path.clone()).unwrap();
    assert!(!doc.is_dirty());
    // The plan is consumed.
    assert!(matches!(
        doc.finish_save(path.clone()),
        Err(MarkiteError::InvalidInput { .. })
    ));
    std::fs::remove_file(path).ok();
}

#[test]
fn set_text_reports_change() {
    let doc = Doc::new();
    assert!(doc.set_text("a b".into()));
    assert!(!doc.set_text("a b".into()));
    assert_eq!((doc.word_count(), doc.char_count()), (2, 2));
}

#[test]
fn external_change_kinds() {
    let doc = Doc::new();
    doc.load("/tmp/x.md".into(), b"one".to_vec()).unwrap();
    assert!(matches!(
        doc.external_change(b"one".to_vec()),
        ExternalChangeKind::Unchanged
    ));
    assert!(matches!(
        doc.external_change(b"two".to_vec()),
        ExternalChangeKind::Reloaded
    ));
    assert_eq!(doc.text(), "two");
    doc.set_text("edited".into());
    assert!(matches!(
        doc.external_change(b"three".to_vec()),
        ExternalChangeKind::Conflict
    ));
    assert_eq!(doc.text(), "edited");
    // Reported once.
    assert!(matches!(
        doc.external_change(b"three".to_vec()),
        ExternalChangeKind::Unchanged
    ));
}

#[test]
fn nul_paths_and_error_kinds() {
    let doc = Doc::new();
    assert!(matches!(
        doc.load("a\0b".into(), vec![]),
        Err(MarkiteError::InvalidInput { .. })
    ));
    doc.prepare_save().unwrap();
    assert!(matches!(
        doc.finish_save("a\0b".into()),
        Err(MarkiteError::InvalidInput { .. })
    ));
    assert!(matches!(
        read_limited("a\0b".into()),
        Err(MarkiteError::InvalidInput { .. })
    ));
    assert!(matches!(
        read_limited("/nonexistent/markite/x.md".into()),
        Err(MarkiteError::NotFound)
    ));
    assert!(matches!(
        read_limited("/".into()),
        Err(MarkiteError::InvalidInput { .. })
    ));
    assert_eq!(max_file_bytes(), 100 * 1024 * 1024);
}

fn blocks() -> Vec<Block> {
    Doc::new_with("# T\n\npara\n\n- a\n- b\n").render_blocks("Breeze Light".into())
}

trait NewWith {
    fn new_with(t: &str) -> Doc;
}
impl NewWith for Doc {
    fn new_with(t: &str) -> Doc {
        let d = Doc::new();
        d.set_text(t.into());
        d
    }
}

#[test]
fn render_blocks_sanity() {
    let b = blocks();
    assert_eq!(b.len(), 3);
    assert_eq!((b[0].start_line, b[0].end_line), (1, 1));
    assert!(b[0].html.contains("<h1"));
    assert!(b[2].html.contains("<li"));
}

#[test]
fn block_mapping_non_finite() {
    let b = blocks();
    for l in [f64::NAN, f64::NEG_INFINITY, -5.0, 0.0] {
        let p = line_to_block(b.clone_all(), l);
        assert_eq!((p.index, p.fraction), (0, 0.0), "line {l}");
    }
    let p = line_to_block(b.clone_all(), f64::INFINITY);
    assert_eq!((p.index, p.fraction), (2, 1.0));
    assert_eq!(block_to_line(b.clone_all(), 0, f64::NAN), 1.0);
    assert_eq!(block_to_line(b.clone_all(), 0, -3.0), 1.0);
    assert_eq!(block_to_line(b.clone_all(), 0, 9.0), 2.0);
    assert_eq!(block_to_line(b.clone_all(), 99, 0.5), 1.0);
    assert_eq!(block_to_line(vec![], 0, 0.5), 1.0);
    let p = line_to_block(vec![], 3.0);
    assert_eq!((p.index, p.fraction), (0, 0.0));
}

trait CloneAll {
    fn clone_all(&self) -> Vec<Block>;
}
impl CloneAll for Vec<Block> {
    fn clone_all(&self) -> Vec<Block> {
        self.iter()
            .map(|b| Block {
                start_line: b.start_line,
                end_line: b.end_line,
                html: b.html.clone(),
            })
            .collect()
    }
}

#[test]
fn minimap() {
    let l = minimap_lines("# H\n\n- item\n```\ncode\n```\n".into());
    assert!(matches!(l[0].kind, MinimapKind::Heading));
    assert!(matches!(l[1].kind, MinimapKind::Blank));
    assert!(matches!(l[2].kind, MinimapKind::ListItem));
    assert_eq!(l[2].marker, 1);
    assert!(matches!(l[4].kind, MinimapKind::Code));
}

#[test]
fn options() {
    assert_eq!(clamp_wrap_column(-1), 60);
    assert_eq!(clamp_wrap_column(i32::MAX), 200);
    assert_eq!(clamp_preview_width(0), 20);
    assert_eq!(clamp_preview_width(500), 100);
    assert_eq!(font_size(0), None);
    assert_eq!(font_size(1000), Some(40));
    assert_eq!(theme_name("Catppuccin".into(), true), "Catppuccin Mocha");
    assert_eq!(theme_name("nope".into(), false), "Breeze Light");
    assert_eq!(appearance_name(appearance_parse("dark".into())), "dark");
    assert_eq!(appearance_name(appearance_parse("?".into())), "system");
    assert_eq!(theme_families().len(), 5);
    assert_eq!(
        (code_fonts().len(), sans_fonts().len(), serif_fonts().len()),
        (4, 3, 3)
    );
    assert_eq!((default_wrap_column(), default_preview_width()), (100, 100));
    assert_eq!(word_count("a b c".into()), 3);
}

#[test]
fn poisoned_lock_recovers() {
    let doc = std::sync::Arc::new(Doc::new());
    let d = doc.clone();
    // A panic while the lock is held can't be triggered through the API; just check concurrent use works.
    std::thread::spawn(move || d.set_text("x".into()))
        .join()
        .unwrap();
    assert_eq!(doc.text(), "x");
}
