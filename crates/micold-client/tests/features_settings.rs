//! The Settings draft, in isolation (feature 021 SC-004; feature 027 T062 — US3 scenarios 2 and 3).
//!
//! This file names exactly one feature module. It builds no `State`, references no other feature's
//! types, and needs no application shell.
//!
//! **It used to be three assertions about a struct's shape**, and said so: the draft had no
//! operations to test because its validation lived in `main.rs`'s `SettingsSaved` arm rather than
//! beside the type it validated. Feature 027 brought the validation across — the sectioned view
//! made the split untenable, since a rejected save now has to name a *section* as well as a
//! message, and the reducer arm had no idea which section a field belonged to. So the cases below
//! are the ones the shell could never have asked for.

use std::collections::BTreeSet;

use micold_client::features::settings::{SettingsDraft, SettingsSection};
use micold_client::features::window::FieldId;
use micold_core::sandbox::placement::PlacementKind;
use micold_core::sandbox::CredentialShare;
use micold_core::theme::ThemePreference;

/// A draft holding values that pass validation, so a test that is about something else does not
/// have to think about the numeric fields.
fn valid() -> SettingsDraft {
    let mut draft = SettingsDraft::default();
    draft.terminal.scrollback_lines = "5000".into();
    draft.environment.timeout_secs = "5".into();
    draft
}

#[test]
fn a_fresh_draft_shows_no_error() {
    let draft = SettingsDraft::default();

    assert!(
        draft.error.is_none(),
        "the form opens clean — an error is the result of a rejected save, not a starting state"
    );
}

#[test]
fn the_numeric_fields_are_held_as_text_so_a_half_typed_value_is_representable() {
    let mut draft = SettingsDraft::default();
    draft.terminal.scrollback_lines = "1".into();
    draft.environment.timeout_secs = String::new();

    assert_eq!(
        draft.terminal.scrollback_lines, "1",
        "typing the first digit of 10000 must not be rejected mid-keystroke, which is why the \
         field is a String and not a usize"
    );
    assert!(
        draft.environment.timeout_secs.is_empty(),
        "an empty field is a state the user passes through, not an invalid parse to report yet"
    );
}

#[test]
fn clearing_the_enable_toggle_leaves_the_script_path_intact() {
    let mut draft = SettingsDraft::default();
    draft.environment.enabled = false;
    draft.environment.script_path = "/p/env.sh".into();

    assert_eq!(
        draft.environment.script_path, "/p/env.sh",
        "turning the feature off must not discard what was configured — re-enabling it should \
         not mean re-typing the path"
    );
}

// ---------------------------------------------------------------------------------------------
// US3 scenario 2 — one form, several pages
// ---------------------------------------------------------------------------------------------

/// The failure a rail invites: each section reseeding itself as it is shown, so an edit made
/// before navigating away is silently reverted on the way back. One draft, several views of it.
#[test]
fn an_unsaved_edit_survives_a_section_change() {
    let mut draft = valid();
    draft.terminal.scrollback_lines = "12345".into();

    draft.show(SettingsSection::Environment);
    draft.show(SettingsSection::Daemon);
    draft.show(SettingsSection::Terminal);

    assert_eq!(
        draft.terminal.scrollback_lines, "12345",
        "an edit was lost by navigating away from its section — the sections are views of one \
         draft, not four forms (US3 scenario 2)"
    );
    assert_eq!(
        draft.section,
        SettingsSection::Terminal,
        "the shown section follows the last navigation"
    );
}

/// Save is a property of the whole draft, not of the page that happens to be on screen — the
/// alternative is a user who edited three sections, pressed Save, and got one of them.
#[test]
fn a_save_applies_every_visited_section_together() {
    let mut draft = valid();
    draft.appearance.theme = ThemePreference::Dark;
    draft.show(SettingsSection::Terminal);
    draft.terminal.scrollback_lines = "9000".into();
    draft.show(SettingsSection::Environment);
    draft.environment.enabled = true;
    draft.environment.script_path = "/p/env.sh".into();
    draft.environment.timeout_secs = "7".into();
    draft.show(SettingsSection::Daemon);
    draft.daemon.placement = PlacementKind::LocalSandbox;

    let valid = draft.validate().expect("every field is in range");

    assert_eq!(valid.theme, ThemePreference::Dark);
    assert_eq!(valid.scrollback_lines, 9000);
    assert!(valid.env_include_enabled);
    assert_eq!(valid.env_include_script_path, "/p/env.sh");
    assert_eq!(valid.env_include_timeout_secs, 7);
    assert_eq!(valid.daemon.placement, PlacementKind::LocalSandbox);
}

/// Editing the sandbox while it is switched off keeps what was set: a user who configures the
/// container and then decides to try the host process first has not asked to lose the
/// configuration (mirrors `DaemonConfig`'s own "kept whether or not it is selected").
#[test]
fn the_sandbox_configuration_survives_switching_back_to_the_host() {
    let mut draft = valid();
    draft.daemon.placement = PlacementKind::LocalSandbox;
    draft
        .daemon
        .profile
        .credentials
        .insert(CredentialShare::GitConfig);
    draft.daemon.placement = PlacementKind::HostProcess;

    let valid = draft.validate().expect("every field is in range");

    assert_eq!(
        valid.daemon.sandbox.credentials,
        BTreeSet::from([CredentialShare::GitConfig]),
        "turning the sandbox off discarded what was shared with it — switching back would mean \
         setting it up again"
    );
}

// ---------------------------------------------------------------------------------------------
// US3 scenario 3 — a rejected save says where to look
// ---------------------------------------------------------------------------------------------

/// A message with no location is unactionable once there is more than one page: "Enter a number
/// between 100 and 100000" tells a user nothing about which of four sections is hiding the field.
#[test]
fn a_rejected_save_names_the_field_and_its_section() {
    let mut draft = valid();
    draft.show(SettingsSection::Appearance);
    draft.terminal.scrollback_lines = "1".into();

    let error = draft.validate().expect_err("1 is below the minimum");

    assert_eq!(error.field, FieldId::SettingsScrollback);
    assert_eq!(
        error.section,
        SettingsSection::Terminal,
        "the error must name the section holding the field, not the one the user is looking at \
         (US3 scenario 3)"
    );
    assert!(
        error
            .message
            .contains(&micold_core::settings::MIN_SCROLLBACK_LINES.to_string())
            && error
                .message
                .contains(&micold_core::settings::MAX_SCROLLBACK_LINES.to_string()),
        "the message must name the accepted range, got {:?}",
        error.message
    );
}

/// Reporting the failure includes *showing* it: a section the user cannot see is not a report.
#[test]
fn reporting_a_failure_shows_the_offending_section() {
    let mut draft = valid();
    draft.show(SettingsSection::Daemon);
    draft.environment.timeout_secs = "not a number".into();

    let error = draft.validate().expect_err("the timeout does not parse");
    draft.report(error);

    assert_eq!(
        draft.section,
        SettingsSection::Environment,
        "the draft stayed on the section the user was reading, so the field the message is about \
         is off screen (US3 scenario 3)"
    );
    assert_eq!(
        draft.error.as_ref().map(|e| e.field),
        Some(FieldId::SettingsEnvIncludeTimeout)
    );
}

/// The next keystroke clears it. An error that outlives the value it was about is a form that
/// looks broken after it has been fixed.
#[test]
fn editing_any_field_clears_a_previous_rejection() {
    let mut draft = valid();
    draft.terminal.scrollback_lines = "1".into();
    let error = draft.validate().expect_err("1 is below the minimum");
    draft.report(error);

    draft.edited();

    assert!(
        draft.error.is_none(),
        "the rejection outlived the value it was about"
    );
}

/// Validation is total: it never panics, whatever the user has typed.
#[test]
fn validation_rejects_rather_than_panics_on_anything_typed() {
    for text in ["", " ", "-1", "1e9", "٤٢", "99999999999999999999999", "12 "] {
        let mut draft = valid();
        draft.terminal.scrollback_lines = text.into();
        let _ = draft.validate();

        let mut draft = valid();
        draft.environment.timeout_secs = text.into();
        let _ = draft.validate();
    }
}

// ---------------------------------------------------------------------------------------
// The Default AI CLI preference (feature 026, T023/T058e — FR-003, FR-006, FR-010)
// ---------------------------------------------------------------------------------------

use micold_core::session::AiCli;

#[test]
fn the_default_ai_cli_is_a_closed_choice_not_typed_text() {
    // Every other field in the Environment section is a `String`, because the form holds what the
    // user *typed* and a half-typed "12" has to be representable. This one is not, and the
    // difference is structural rather than stylistic: it is picked from a list of installed CLIs,
    // so there is no intermediate state to hold and nothing to validate on save. `SettingsSaved`
    // has three validation branches and none of them is about this field.
    let draft = SettingsDraft::default();
    assert_eq!(draft.environment.default_ai_cli, AiCli::ClaudeCode);

    let mut chosen = SettingsDraft::default();
    chosen.environment.default_ai_cli = AiCli::Copilot;
    assert_eq!(chosen.environment.default_ai_cli, AiCli::Copilot);
    assert!(
        chosen.error.is_none(),
        "choosing from a list cannot produce a validation error"
    );
}

#[test]
fn the_settings_select_names_clis_the_human_readable_way() {
    // T058e — the two naming registers, and the drift this feature is most likely to produce,
    // since both strings hang off the same provider.
    //
    // Menus and sentences get `display_name()`; a sidebar row and the terminal bar get
    // `command()`, which is a label inside a width budget rather than a menu entry. The Settings
    // select is a menu, so a row of "claude" / "copilot" here would be the leak.
    let names: Vec<&str> = AiCli::ALL
        .into_iter()
        .map(|which| which.provider().display_name())
        .collect();
    assert_eq!(names, vec!["Claude Code", "GitHub Copilot"]);

    let commands: Vec<&str> = AiCli::ALL
        .into_iter()
        .map(|which| which.provider().command())
        .collect();
    assert_eq!(commands, vec!["claude", "copilot"]);
    assert!(
        names.iter().zip(&commands).all(|(name, cmd)| name != cmd),
        "the two registers are distinct strings for every provider, so a leak in either direction \
         is observable rather than a coincidence"
    );
}

#[test]
fn a_failure_message_names_the_cli_the_human_readable_way() {
    // FR-010: "GitHub Copilot isn't installed" is a sentence; "copilot isn't installed" reads as a
    // shell error. Same register as the menus, and the same reason.
    let missing = AiCli::Copilot.provider();
    let message = format!("{} isn't installed.", missing.display_name());
    assert_eq!(message, "GitHub Copilot isn't installed.");
    assert!(!message.contains("copilot "), "not the command name");
}
