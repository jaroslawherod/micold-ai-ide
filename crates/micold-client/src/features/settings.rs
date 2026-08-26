//! The Settings form's in-progress edit (feature 021, T018).
//!
//! Every field is text, even the two that are numbers: the form holds what the user has typed,
//! and parsing happens on save, so a half-typed "12" is representable without being a valid
//! scrollback limit.
//!
//! **This feature is still split.** The validation that turns these strings into settings — the
//! range checks and their error messages — lives in `shell/persist.rs`'s `on_settings_saved`
//! rather than beside the type it validates, which is what FR-001 asks against. Moving it is
//! Tier 3 work (it is reducer code, and the path returns a `Task`), not Tier 1's; recorded here so
//! the split is visible from the module rather than only from the plan.
//!
//! Feature 028 narrowed the split without closing it: the *routing* now has one home,
//! `shell/settings.rs`, which decides which of the ten transitions need an effect. The validation
//! it calls into is still `persist`'s.
//!
//! # The vocabulary this feature declares
//!
//! Ten transitions in [`Msg`]: the theme's three (`ThemePreferenceChanged`, `ThemeModeCycled`,
//! `SystemThemeChanged`), the form's five (`Opened`, `ScrollbackChanged`,
//! `EnvIncludeEnabledToggled`, `EnvIncludePathChanged`, `EnvIncludeTimeoutChanged`), and the two ways
//! out (`Saved`, `Cancelled`).
//!
//! [`update`] routes all ten and is pure (data-model.md §1.1 shape A), but the feature's entry
//! shape is **B**: `main.rs` has one `Message::Settings` arm and it goes to `shell/settings.rs`,
//! because four of the ten additionally need `settings.json` written or read back
//! (`Opened`, `Saved`, `ThemePreferenceChanged`, `ThemeModeCycled`). The other six reach
//! [`update`] from there through the same wrapper variant they arrived under, so the pure path is
//! identical whether or not the shell was in the way. One routing table, one place to look —
//! which is the narrowing described above; the validation it delegates to `shell/persist.rs` is
//! what is left of the split.

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
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::Settings(Msg::Cancelled))
    }
}

impl Registered for SettingsDialog {
    fn open_in(state: &State) -> Option<Self> {
        state.settings_draft.as_ref().map(|_| SettingsDialog)
    }
}

/// Everything the user can do to their settings (feature 028, FR-001).
///
/// # The variants kept their meaning and lost their prefix
///
/// The six that began with `Settings` do not any more — the type says which form (contract M1),
/// so `SettingsScrollbackChanged` is `Msg::ScrollbackChanged`. The four theme variants keep their
/// names: `Theme` is not this feature's name, it is which setting, and dropping it would leave a
/// bare `ModeCycled` that says nothing about what mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The user selected a theme preference (Follow system / Light / Dark) (FR-007, FR-008).
    /// The shell persists the updated preference afterward.
    ThemePreferenceChanged(micold_core::theme::ThemePreference),
    /// Cycle the theme mode to the next one (Auto → Light → Dark → Auto) from the toolbar
    /// menu's mode toggle. The shell persists the updated preference; the menu stays open.
    ThemeModeCycled,
    /// The OS light/dark preference poll observed a (changed) scheme (FR-006). Transient;
    /// never persisted. Carries the raw detection outcome — `Err(())` for a transient failure
    /// (e.g. `dark_light::detect()` timing out under CPU load) — rather than an
    /// already-resolved `SystemScheme`, specifically so the periodic poll's `Subscription::map`
    /// closure (`os_theme_poll`, `src/main.rs`) does not need to capture the previous scheme:
    /// iced panics if a subscription's mapping closure captures state, since that breaks the
    /// stable identity it relies on to avoid restarting the underlying timer every frame.
    /// [`system_theme_changed`] applies the same last-known fallback
    /// (`theme::observe_system_scheme`) that used to be baked in at the call site instead.
    SystemThemeChanged(Result<micold_core::theme::SystemScheme, ()>),
    /// Open the Settings form (from the toolbar menu) (FR-019). The shell seeds the draft with
    /// the current scrollback value.
    Opened,
    /// The Settings scrollback field changed.
    ScrollbackChanged(String),
    /// The Settings environment-include enabled checkbox was toggled (feature 011, FR-001).
    EnvIncludeEnabledToggled(bool),
    /// The Settings environment-include script path field changed (FR-002).
    EnvIncludePathChanged(String),
    /// The Settings environment-include timeout field changed (FR-003).
    EnvIncludeTimeoutChanged(String),
    /// Save the Settings form (validated + persisted by the shell) (FR-020, FR-021).
    Saved,
    /// Dismiss the Settings form without saving (Cancel or Esc).
    Cancelled,
}

/// The pure half of this feature's reducer surface: shape A (contract M2).
///
/// All ten arms are here, and all ten are pure. Four of them additionally need an effect — a
/// write to `settings.json`, or the current values read back into the draft — and that half is
/// `shell/settings.rs`’s `update`, which runs the effect and routes the rest here. Splitting
/// by effect rather than by variant is what M2 asks for: nothing about opening the form is
/// duplicated between the two, the shell simply has something extra to do afterwards.
pub fn update(state: &mut State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::ThemePreferenceChanged(pref) => theme_preference_changed(state, pref),
        Msg::ThemeModeCycled => theme_mode_cycled(state),
        Msg::SystemThemeChanged(detected) => system_theme_changed(state, detected),
        Msg::Opened => opened(state),
        Msg::ScrollbackChanged(text) => scrollback_changed(state, text),
        Msg::EnvIncludeEnabledToggled(enabled) => env_include_enabled_toggled(state, enabled),
        Msg::EnvIncludePathChanged(text) => env_include_path_changed(state, text),
        Msg::EnvIncludeTimeoutChanged(text) => env_include_timeout_changed(state, text),
        Msg::Saved => saved(state),
        Msg::Cancelled => cancelled(state),
    }
    Vec::new()
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
