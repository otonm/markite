use markite_core::blocks::{block_to_line, line_to_block, to_blocks};

const MD: &str = "# Title\n\npara one\nstill one\n\n- a\n- b\n- c\n";

#[test]
fn splits_top_level_blocks_with_lines() {
    let b = to_blocks(MD, "");
    let spans: Vec<_> = b.iter().map(|b| (b.start_line, b.end_line)).collect();
    assert_eq!(spans, [(1, 1), (3, 4), (6, 8)]);
    assert!(b[0].html.contains("<h1"));
    assert!(b[2].html.contains("<ul"));
}

#[test]
fn line_block_mapping_roundtrips() {
    let b = to_blocks(MD, "");
    assert_eq!(line_to_block(&b, 1.0), (0, 0.0));
    assert_eq!(line_to_block(&b, 2.0), (1, 0.0)); // blank line -> start of next block
    assert_eq!(line_to_block(&b, 4.0), (1, 0.5));
    assert_eq!(line_to_block(&b, 99.0), (2, 1.0));
    for line in [1.0, 3.0, 3.5, 4.0, 6.0, 7.25] {
        let (i, f) = line_to_block(&b, line);
        assert!((block_to_line(&b, i, f) - line).abs() < 1e-9, "line {line}");
    }
    assert_eq!(line_to_block(&[], 5.0), (0, 0.0));
}

#[test]
fn non_finite_inputs_are_sanitised() {
    let b = to_blocks(MD, "");
    assert_eq!(line_to_block(&b, f64::NAN), (0, 0.0));
    assert_eq!(line_to_block(&b, f64::INFINITY), (2, 1.0));
    assert_eq!(line_to_block(&b, f64::NEG_INFINITY), (0, 0.0));
    assert_eq!(block_to_line(&b, 1, f64::NAN), block_to_line(&b, 1, 0.0));
    assert_eq!(
        block_to_line(&b, 1, f64::INFINITY),
        block_to_line(&b, 1, 1.0)
    );
    assert_eq!(block_to_line(&b, 99, 0.5), 1.0, "out-of-range block");
}
