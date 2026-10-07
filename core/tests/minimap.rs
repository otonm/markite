use markite_core::minimap::{lines, LineKind};

#[test]
fn classifies_lines() {
    let got = lines("# T\n\n  body\n```\nx = 1\n```\n");
    let kinds: Vec<_> = got.iter().map(|l| l.kind).collect();
    assert_eq!(
        kinds,
        [
            LineKind::Heading,
            LineKind::Blank,
            LineKind::Text,
            LineKind::Code,
            LineKind::Code,
            LineKind::Code
        ]
    );
    assert_eq!(got[2].indent, 2);
    assert_eq!(got[2].len, 4);
}
