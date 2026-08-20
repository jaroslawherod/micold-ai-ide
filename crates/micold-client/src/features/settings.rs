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
use micold_core::session::AiCli;

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
    /// The chosen default AI CLI (feature 026, FR-003).
    ///
    /// Not a `String`, unlike every field above it, and the difference is the point: those hold
    /// what the user *typed*, which may not yet be a valid setting. This is a closed enum picked
    /// from a list, so there is no half-typed state to represent and nothing to validate on save.
    pub default_ai_cli: AiCli,
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

/// The theme mode was advanced one step (feature 003, FR-005).
///
/// The menu stays open, so repeated clicks cycle.
pub fn theme_mode_cycled(state: &mut State) {
    state.theme_pref = state.theme_pref.next();
}

/// A theme preference was chosen outright.
///
/// Pure state change; the shell persists it at the I/O boundary (FR-009).
pub fn theme_preference_changed(state: &mut State, pref: micold_core::theme::ThemePreference) {
    state.theme_pref = pref;
}

/// The OS reported its light/dark preference (feature 003).
///
/// `observe_system_scheme` is what decides whether a detection is believed: an OS that answers
/// "unknown" must not overwrite a scheme already observed, or a single unanswered probe would
/// flip the whole UI. The rule lives in core; this arm only records its answer.
pub fn system_theme_changed(
    state: &mut State,
    detected: Result<micold_core::theme::SystemScheme, ()>,
) {
    state.system_scheme = micold_core::theme::observe_system_scheme(detected, state.system_scheme);
}

/// The Settings dialog was opened (feature 006, FR-020).
///
/// The shell seeds the current values; a draft is ensured here so the reducer path alone is
/// enough to open the form in a test.
pub fn opened(state: &mut State) {
    state.clear_for_dialog();
    if state.settings_draft.is_none() {
        state.settings_draft = Some(SettingsDraft::default());
    }
}

/// The scrollback field was edited.
pub fn scrollback_changed(state: &mut State, text: String) {
    edit(state, |draft| draft.scrollback_lines = text);
}

/// The environment-include toggle was flipped (feature 011).
pub fn env_include_enabled_toggled(state: &mut State, enabled: bool) {
    edit(state, |draft| draft.env_include_enabled = enabled);
}

/// The environment-include script path was edited (feature 011).
pub fn env_include_path_changed(state: &mut State, text: String) {
    edit(state, |draft| draft.env_include_script_path = text);
}

/// The environment-include timeout was edited (feature 011).
pub fn env_include_timeout_changed(state: &mut State, text: String) {
    edit(state, |draft| draft.env_include_timeout = text);
}

/// The **Default AI CLI** select changed (feature 026, FR-003).
pub fn default_ai_cli_changed(state: &mut State, which: AiCli) {
    edit(state, |draft| draft.default_ai_cli = which);
}

/// Apply an edit to the open draft, if there is one, and clear the pending error.
///
/// Every field edit did these two things and the second was easy to forget: a stale validation
/// error left beside a field the user has since corrected is the form telling them they are wrong
/// after they have fixed it. One place, so a new field cannot omit it.
fn edit(state: &mut State, change: impl FnOnce(&mut SettingsDraft)) {
    if let Some(draft) = &mut state.settings_draft {
        change(draft);
        draft.error = None;
    }
}

/// The form was saved (feature 006).
///
/// Validation and persistence happen in the shell; the reducer closes the form.
pub fn saved(state: &mut State) {
    state.settings_draft = None;
}

/// The form was dismissed without saving.
pub fn cancelled(state: &mut State) {
    state.settings_draft = None;
}
