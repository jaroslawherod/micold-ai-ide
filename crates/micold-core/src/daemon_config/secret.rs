//! A path to a file holding one secret. The content never reaches `Debug` or `Display`.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// An absolute path to a file that holds exactly one secret. `Debug` and `Display` print the
/// path only (SC-005).
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct SecretPath(PathBuf);

impl SecretPath {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Reads the secret, opened read-only, with the trailing newline stripped.
    pub fn read(&self) -> std::io::Result<SecretValue> {
        let mut text = std::fs::read_to_string(&self.0)?;
        while text.ends_with('\n') || text.ends_with('\r') {
            text.pop();
        }
        Ok(SecretValue(text))
    }
}

impl std::fmt::Debug for SecretPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretPath({:?})", self.0)
    }
}

impl std::fmt::Display for SecretPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

/// A secret's content. Deliberately without `Display`; `Debug` is redacted.
pub struct SecretValue(String);

impl SecretValue {
    /// The content, for the one place that hands it to a process or a request.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretValue(<redacted>)")
    }
}
