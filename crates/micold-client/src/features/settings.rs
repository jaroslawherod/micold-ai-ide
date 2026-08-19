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
use crate::app::{Message, State};
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::overlay::Layer;

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
    /// Whether the daemon runs in a container (feature 027, FR-001).
    ///
    /// A boolean here rather than the `PlacementKind` it maps to, because this dialog offers the
    /// two placements as an on/off choice. When Settings becomes a sectioned view (US3) the daemon
    /// section renders the placement properly, and this field goes with the dialog.
    pub sandboxed: bool,
    /// The last validation error shown after a rejected save.
    pub error: Option<String>,
}

/// The Settings form, as a floating surface (feature 021, T032).
///
/// Dismissible, like every other dialog here. `DismissalRules::protecting_input` exists for a
/// form whose loss would cost the user real work; whether this is one is a behaviour question,
/// and FR-027 puts those outside this feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsDialog;

impl FloatingSurface for SettingsDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("settings")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::SettingsCancelled)
    }
}

impl Registered for SettingsDialog {
    fn open_in(state: &State) -> Option<Self> {
        state.settings_draft.as_ref().map(|_| SettingsDialog)
    }
}
