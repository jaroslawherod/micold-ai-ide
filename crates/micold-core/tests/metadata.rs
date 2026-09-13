//! US2 tests: application identity resolution and empty-value fallback
//! (FR-006, FR-007, FR-008, FR-009, FR-016).

use micold_core::metadata::{AppMetadata, APP_NAME};

#[test]
fn name_is_exactly_micold_ai_ide() {
    assert_eq!(APP_NAME, "Micold AI IDE");
    assert_eq!(
        AppMetadata::resolve("1.0.0", "MIT", "An IDE.").name,
        "Micold AI IDE"
    );
}

// Where the version, license and description come *from* is the application's business, not this
// library's: `env!` expands where it is written, and written here it read `micold-core`'s manifest
// (001 BUG-001). `micold-client/tests/about_description.rs` checks what the dialog actually shows.

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
