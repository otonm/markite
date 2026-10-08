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

#[test]
fn classifies_markdown_constructs() {
    let got = lines("1. one\n- two\n> q\n| a | b |\n\n    indented\nplain\n");
    let kinds: Vec<_> = got.iter().map(|l| l.kind).collect();
    assert_eq!(
        kinds,
        [
            LineKind::ListItem,
            LineKind::ListItem,
            LineKind::Quote,
            LineKind::Table,
            LineKind::Blank,
            LineKind::Code,
            LineKind::Text
        ]
    );
    assert_eq!((got[0].marker, got[1].marker, got[6].marker), (2, 1, 0));
    // A dash without a following space is text, and a number without a dot is not a list.
    let kinds: Vec<_> = lines("-x\n12 apples\n").iter().map(|l| l.kind).collect();
    assert_eq!(kinds, [LineKind::Text, LineKind::Text]);
}
