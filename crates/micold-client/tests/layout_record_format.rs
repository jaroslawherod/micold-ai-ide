//! Normalisation and emission order for the layout fixture (feature 019, T006/T007).
//!
//! Both properties exist to stop the fixture lying in one of two directions. Under-normalising
//! makes it flap on floating-point noise, so it gets regenerated reflexively and stops meaning
//! anything. Sorting makes it stable in a way that *hides* structural reordering, which is one of
//! the changes it exists to report.

mod support;

use support::layout::{self as lay, Layer, LayoutRecord};

fn record(path: &[usize], x: f32, y: f32, w: f32, h: f32) -> LayoutRecord {
    LayoutRecord {
        path: path.to_vec(),
        layer: Layer::Base,
        x,
        y,
        width: w,
        height: h,
    }
}

// --- T006 — numeric normalisation (contract §2, FR-012) ---------------------------------------

#[test]
fn values_round_to_one_decimal_place() {
    assert_eq!(lay::normalise(244.528_02), 244.5);
    assert_eq!(lay::normalise(87.449_9), 87.4);
    assert_eq!(lay::normalise(87.45), 87.5);
    assert_eq!(lay::normalise(20.8), 20.8);
}

/// One decimal is far below what a person can see, and far above floating-point noise. The
/// motivating overlap defect was tens of pixels; a tenth of one is not a regression.
#[test]
fn sub_tenth_differences_normalise_away() {
    assert_eq!(lay::normalise(100.001), lay::normalise(100.004));
}

#[test]
fn negative_zero_is_written_as_zero() {
    assert_eq!(lay::format_value(-0.0).trim(), "0.0");
    assert_eq!(lay::format_value(0.0).trim(), "0.0");
    assert_eq!(
        lay::format_value(-0.0),
        lay::format_value(0.0),
        "a value that is not different must not print differently"
    );
}

#[test]
fn every_value_carries_exactly_one_fractional_digit() {
    for v in [0.0, 1.0, 20.8, 1280.0, 244.528_02, -0.0] {
        let s = lay::format_value(v);
        let t = s.trim();
        let frac = t.split_once('.').map(|(_, f)| f).unwrap_or("");
        assert_eq!(
            frac.len(),
            1,
            "{t:?} must carry exactly one fractional digit — `0` and `0.0` are the same number but \
             different bytes, and the fixture is compared as bytes"
        );
    }
}

#[test]
fn values_are_right_aligned_to_a_stable_width() {
    let widths: Vec<usize> = [0.0_f32, 8.0, 87.4, 1280.0]
        .iter()
        .map(|v| lay::format_value(*v).len())
        .collect();

    assert!(
        widths.windows(2).all(|w| w[0] == w[1]),
        "geometry columns must be a fixed width so they line up down the file; got {widths:?}"
    );
}

/// `{:?}` on an `f32` is not a stable text form — it prints the shortest round-tripping
/// representation, which changes with the value rather than with the format.
#[test]
fn formatting_does_not_fall_back_to_debug() {
    assert_ne!(
        lay::format_value(244.528_02).trim(),
        "244.52802",
        "full precision would make the fixture flap on shaping noise"
    );
    assert_eq!(lay::format_value(244.528_02).trim(), "244.5");
    assert!(
        lay::format_value(1280.0).len() > "1280.0".len(),
        "values must be padded to the fixed column width, not printed bare"
    );
    assert!(lay::format_value(1280.0).ends_with("1280.0"));
}

// --- T007 — emission order (contract §3, FR-002) ----------------------------------------------

/// Records must be emitted in the tree's own depth-first order, never sorted.
#[test]
fn records_are_emitted_depth_first_and_never_sorted() {
    let records = vec![
        record(&[], 0.0, 0.0, 1280.0, 800.0),
        record(&[1], 0.0, 0.0, 640.0, 800.0),
        record(&[1, 0], 0.0, 0.0, 320.0, 800.0),
        record(&[0], 640.0, 0.0, 640.0, 800.0),
    ];

    let rendered: Vec<String> = records.iter().map(lay::format_record).collect();
    let paths: Vec<&str> = rendered
        .iter()
        .map(|line| line.split_whitespace().nth(1).unwrap())
        .collect();

    assert_eq!(
        paths,
        vec!["0", "0/1", "0/1/0", "0/0"],
        "child 1 was declared before child 0 and must be emitted in that order — sorting here \
         would conceal exactly the structural reordering this gate reports"
    );
}

#[test]
fn the_layer_token_distinguishes_base_from_overlay() {
    let mut base = record(&[0], 0.0, 0.0, 10.0, 10.0);
    base.layer = Layer::Base;
    let mut over = record(&[0], 0.0, 0.0, 10.0, 10.0);
    over.layer = Layer::Overlay;

    assert!(lay::format_record(&base).starts_with("base"));
    assert!(lay::format_record(&over).starts_with("over"));
}

/// Depth is conveyed by indenting the path column, so the file reads as a tree while the numeric
/// columns stay aligned.
#[test]
fn depth_is_visible_as_indentation_of_the_path_column() {
    let shallow = lay::format_record(&record(&[0], 0.0, 0.0, 1.0, 1.0));
    let deep = lay::format_record(&record(&[0, 0, 0], 0.0, 0.0, 1.0, 1.0));

    let indent = |s: &str| s.len() - s.trim_start_matches(|c| c == ' ').len();
    let after_layer = |s: &str| s.strip_prefix("base").unwrap().to_string();

    assert!(
        indent(&after_layer(&deep)) > indent(&after_layer(&shallow)),
        "a deeper element must be indented further"
    );
}
