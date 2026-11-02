//! A test file nobody wired into `tests/main.rs` or `Cargo.toml` would never run (#1631).

use crate::test_wiring;

#[test]
fn every_test_file_is_wired() {
    test_wiring::assert_wired(env!("CARGO_MANIFEST_DIR"));
}
