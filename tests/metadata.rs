//! US2 tests: application identity resolution and empty-value fallback
//! (FR-006, FR-007, FR-008, FR-009, FR-016).

use micold_ai_ide::metadata::{AppMetadata, APP_NAME};

#[test]
fn name_is_exactly_micold_ai_ide() {
    assert_eq!(APP_NAME, "Micold AI IDE");
    assert_eq!(AppMetadata::from_env().name, "Micold AI IDE");
}

#[test]
fn version_comes_from_cargo_metadata_not_hardcoded() {
    let m = AppMetadata::from_env();
    assert_eq!(m.version, env!("CARGO_PKG_VERSION"));
    assert!(!m.version.is_empty());
}

#[test]
fn license_and_description_resolve_from_cargo() {
    let m = AppMetadata::from_env();
    assert_eq!(m.license, "Apache-2.0");
    assert!(!m.description.is_empty());
}

#[test]
fn empty_metadata_falls_back_to_unknown() {
    let m = AppMetadata::resolve("", "   ", "");
    assert_eq!(m.version, "unknown");
    assert_eq!(m.license, "unknown");
    assert_eq!(m.description, "unknown");
    // The name is a constant and is never subject to the fallback rule.
    assert_eq!(m.name, "Micold AI IDE");
}

#[test]
fn present_metadata_is_passed_through_trimmed() {
    let m = AppMetadata::resolve("1.2.3", "MIT", "An IDE.");
    assert_eq!(m.version, "1.2.3");
    assert_eq!(m.license, "MIT");
    assert_eq!(m.description, "An IDE.");
}
