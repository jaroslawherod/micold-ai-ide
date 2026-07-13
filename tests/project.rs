//! US1 tests: the pure project-name derivation (FR-004).
//! US4 tests: rename validation (FR-020).

use micold_ai_ide::project::{default_display_name, validate_rename, RenameError};
use std::path::Path;

#[test]
fn default_display_name_is_final_component() {
    assert_eq!(
        default_display_name(Path::new("/home/alice/my-repo")),
        "my-repo"
    );
}

#[test]
fn default_display_name_ignores_trailing_slash() {
    assert_eq!(
        default_display_name(Path::new("/home/alice/notes/")),
        "notes"
    );
}

#[test]
fn default_display_name_falls_back_for_root() {
    // A filesystem root has no final "normal" component; the fallback must be non-empty
    // so a project never gets a blank display name.
    let name = default_display_name(Path::new("/"));
    assert!(!name.trim().is_empty());
}

#[test]
fn validate_rename_rejects_empty() {
    assert_eq!(validate_rename(""), Err(RenameError::Empty));
}

#[test]
fn validate_rename_rejects_whitespace_only() {
    assert_eq!(validate_rename("   \t "), Err(RenameError::Whitespace));
}

#[test]
fn validate_rename_accepts_and_trims_valid_name() {
    assert_eq!(
        validate_rename("  My Project  "),
        Ok("My Project".to_string())
    );
}
