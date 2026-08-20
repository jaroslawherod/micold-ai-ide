//! The Settings form's draft, in isolation (feature 021, SC-004).
//!
//! This file names exactly one feature module. It builds no `State`, references no other feature's
//! types, and needs no application shell.
//!
//! It is a short file, and that is itself the finding: `SettingsDraft` has no operations to test
//! because its validation lives in `main.rs`'s `SettingsSaved` arm rather than beside the type.
//! What can be asserted in isolation today is the draft's shape and its opening state. When Tier 3
//! brings the validation across, this file is where its tests belong.

use micold_client::features::settings::SettingsDraft;

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
    let draft = SettingsDraft {
        scrollback_lines: "1".into(),
        env_include_timeout: "".into(),
        ..SettingsDraft::default()
    };

    assert_eq!(
        draft.scrollback_lines, "1",
        "typing the first digit of 10000 must not be rejected mid-keystroke, which is why the \
         field is a String and not a usize"
    );
    assert!(
        draft.env_include_timeout.is_empty(),
        "an empty field is a state the user passes through, not an invalid parse to report yet"
    );
}

#[test]
fn clearing_the_enable_toggle_leaves_the_script_path_intact() {
    let draft = SettingsDraft {
        env_include_enabled: false,
        env_include_script_path: "/p/env.sh".into(),
        ..SettingsDraft::default()
    };

    assert_eq!(
        draft.env_include_script_path, "/p/env.sh",
        "turning the feature off must not discard what was configured — re-enabling it should \
         not mean re-typing the path"
    );
}

// ---------------------------------------------------------------------------------------
// The Default AI CLI preference (feature 026, T023/T058e — FR-003, FR-006, FR-010)
// ---------------------------------------------------------------------------------------

use micold_core::session::AiCli;

#[test]
fn the_default_ai_cli_is_a_closed_choice_not_typed_text() {
    // Every other field on this draft is a `String`, because the form holds what the user *typed*
    // and a half-typed "12" has to be representable. This one is not, and the difference is
    // structural rather than stylistic: it is picked from a list of installed CLIs, so there is no
    // intermediate state to hold and nothing to validate on save. `SettingsSaved` has three
    // validation branches and none of them is about this field.
    let draft = SettingsDraft::default();
    assert_eq!(draft.default_ai_cli, AiCli::ClaudeCode);

    let chosen = SettingsDraft {
        default_ai_cli: AiCli::Copilot,
        ..SettingsDraft::default()
    };
    assert_eq!(chosen.default_ai_cli, AiCli::Copilot);
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
