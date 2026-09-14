//! Read-only application identity shown in the About dialog.
//!
//! Version, license, and description are fixed at **compile time**, so the version can never
//! drift from the packaged release (FR-007, SC-003) and nothing is read from disk or network at
//! runtime (Constitution Principle IV). The display name is a constant because Cargo package
//! names cannot contain spaces (FR-006).
//!
//! **This module does not read them.** `env!` expands in the crate where it is *written*, so an
//! `env!("CARGO_PKG_DESCRIPTION")` here reads this library's manifest whichever crate calls it —
//! which is how the About dialog came to describe `micold-core` to its users (001 BUG-001). The
//! application resolves its own identity, from its own manifest, through [`AppMetadata::resolve`].

/// The application's display name. Exactly this string per FR-006.
pub const APP_NAME: &str = "Micold AI IDE";

/// The project changelog, embedded at compile time so the app can show a "What's new" view with
/// no filesystem/network access (Constitution Principle IV). Maintained by release-please from
/// Conventional Commits; see `CHANGELOG.md` at the repository root.
pub const CHANGELOG: &str = include_str!("../../../CHANGELOG.md");

/// Shown in place of any metadata field whose source value is empty (FR-016).
const FALLBACK: &str = "unknown";

/// The identity of the running application, as displayed in the About dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppMetadata {
    /// Application name — always [`APP_NAME`].
    pub name: &'static str,
    /// Version string (the application's `CARGO_PKG_VERSION`).
    pub version: String,
    /// OSI-approved license name (the application's `CARGO_PKG_LICENSE`).
    pub license: String,
    /// One-line description of the application, written for its users (FR-009).
    pub description: String,
}

impl AppMetadata {
    /// Resolve identity from raw metadata strings, applying the empty → fallback rule.
    ///
    /// The only constructor. The caller passes its own compile-time values, so the `env!`s expand
    /// against the application's manifest rather than this library's (BUG-001), and the fallback
    /// behaviour stays a pure function that can be unit-tested with arbitrary inputs (FR-016).
    pub fn resolve(version: &str, license: &str, description: &str) -> Self {
        Self {
            name: APP_NAME,
            version: or_fallback(version),
            license: or_fallback(license),
            description: or_fallback(description),
        }
    }
}

/// Return the trimmed value, or the [`FALLBACK`] label if it is empty/whitespace.
fn or_fallback(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        FALLBACK.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_values_pass_through_untruncated() {
        // The data layer never truncates; the dialog is responsible for wrapping long
        // license/description text without hiding the Close control (edge case).
        let long = "x".repeat(500);
        let m = AppMetadata::resolve(&long, &long, &long);
        assert_eq!(m.description.len(), 500);
        assert_eq!(m.license.len(), 500);
    }

    #[test]
    fn whitespace_only_is_treated_as_empty() {
        assert_eq!(or_fallback("\t  \n"), FALLBACK);
    }
}
