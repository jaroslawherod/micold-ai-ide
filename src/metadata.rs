//! Read-only application identity shown in the About dialog.
//!
//! Version, license, and description come from Cargo package metadata at **compile
//! time** (`env!`), so the version can never drift from the packaged release
//! (FR-007, SC-003) and nothing is read from disk or network at runtime
//! (Constitution Principle IV). The display name is a constant because Cargo package
//! names cannot contain spaces (FR-006).

/// The application's display name. Exactly this string per FR-006.
pub const APP_NAME: &str = "Micold AI IDE";

/// Shown in place of any metadata field whose source value is empty (FR-016).
const FALLBACK: &str = "unknown";

/// The identity of the running application, as displayed in the About dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppMetadata {
    /// Application name — always [`APP_NAME`].
    pub name: &'static str,
    /// Version string (from `CARGO_PKG_VERSION`).
    pub version: String,
    /// OSI-approved license name (from `CARGO_PKG_LICENSE`).
    pub license: String,
    /// One-line description (from `CARGO_PKG_DESCRIPTION`).
    pub description: String,
}

impl AppMetadata {
    /// Resolve identity from Cargo package metadata baked in at compile time.
    pub fn from_env() -> Self {
        Self::resolve(
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_LICENSE"),
            env!("CARGO_PKG_DESCRIPTION"),
        )
    }

    /// Resolve identity from raw metadata strings, applying the empty → fallback rule.
    ///
    /// Kept separate from [`from_env`](Self::from_env) so the fallback behavior is a pure
    /// function that can be unit-tested with arbitrary inputs (FR-016).
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
