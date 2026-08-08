//! The Settings form's in-progress edit (feature 021, T018).
//!
//! Every field is text, even the two that are numbers: the form holds what the user has typed,
//! and parsing happens on save, so a half-typed "12" is representable without being a valid
//! scrollback limit.
//!
//! **This feature is still split.** The validation that turns these strings into settings — the
//! range checks and their error messages — lives in `main.rs`'s `Message::SettingsSaved` arm
//! rather than beside the type it validates, which is what FR-001 asks against. Moving it is
//! Tier 3 work (it is reducer code, and the arm returns a `Task`), not Tier 1's; recorded here so
//! the split is visible from the module rather than only from the plan.

/// In-progress Settings form state, present only while the Settings overlay is open (feature
/// 006, FR-020). The scrollback field is edited as text and validated/parsed on save.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsDraft {
    /// The editable scrollback-limit value (parsed/validated on save).
    pub scrollback_lines: String,
    /// Whether environment-include is enabled (feature 011, FR-001).
    pub env_include_enabled: bool,
    /// The editable environment-include script path (FR-002).
    pub env_include_script_path: String,
    /// The editable environment-include timeout, in seconds as text (parsed/validated on save,
    /// FR-003).
    pub env_include_timeout: String,
    /// The last validation error shown after a rejected save.
    pub error: Option<String>,
}
