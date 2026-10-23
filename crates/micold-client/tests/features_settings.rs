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

use micold_client::features::settings::{Refusal, SettingsDraft, SettingsSection};
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
    draft.environment.long_task_threshold_secs = "60".into();
    draft.environment.plan_usage_warn_percent = "80".into();
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
    draft.show(SettingsSection::Daemons);
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
    draft.show(SettingsSection::Daemons);
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

    let error = draft
        .validate()
        .expect_err("1 is below the minimum")
        .into_field()
        .expect("a field error");

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
    draft.show(SettingsSection::Daemons);
    draft.environment.timeout_secs = "not a number".into();

    let error = draft
        .validate()
        .expect_err("the timeout does not parse")
        .into_field()
        .expect("a field error");
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
    let error = draft
        .validate()
        .expect_err("1 is below the minimum")
        .into_field()
        .expect("a field error");
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
    assert_eq!(
        names,
        vec![
            "Claude Code",
            "GitHub Copilot",
            "Pi Coding Agent",
            "Codex",
            "OpenCode"
        ]
    );

    let commands: Vec<&str> = AiCli::ALL
        .into_iter()
        .map(|which| which.provider().command())
        .collect();
    assert_eq!(
        commands,
        vec!["claude", "copilot", "pi", "codex", "opencode"]
    );
    assert!(
        names.iter().zip(&commands).all(|(name, cmd)| name != cmd),
        "the two registers are distinct strings for every provider, so a leak in either direction \
         is observable rather than a coincidence"
    );
}

#[test]
fn a_failure_message_names_the_cli_the_human_readable_way() {
    // FR-010: "GitHub Copilot" in a sentence; "copilot" reads as a shell error. Same register as
    // the menus, and the same reason. Since feature 037 the sentence is `explain`'s.
    use micold_core::cli_reason::{explain, AttemptDir, Place, SpawnEnv};
    let said = explain(
        &[AiCli::Copilot],
        SpawnEnv::Applied,
        Place::ThisComputer,
        AttemptDir::Home,
    )
    .expect("one CLI is missing");
    assert!(
        said.reason
            .starts_with(AiCli::Copilot.provider().display_name()),
        "{}",
        said.reason
    );
    assert!(!said.reason.contains("copilot "), "not the command name");
}

// ---------------------------------------------------------------------------------------
// The Pi activity component switch (feature 029, T039 — FR-012e, FR-012f, SC-005a)
// ---------------------------------------------------------------------------------------

use micold_core::settings::Settings;

#[test]
fn the_pi_activity_switch_is_one_application_wide_setting_on_by_default() {
    // One row, in one section, seeded from the one stored value. There is no per-project or
    // per-session form to fall back to, so what the stored settings say is what the form shows.
    let draft = SettingsDraft::from_settings(&Settings::default());
    assert!(
        draft.environment.pi_activity_component,
        "the component is loaded unless the user declines it"
    );

    // And the stored form has exactly one key for it, at the top level: a copy nested under a
    // project, a session or the service profile would be a second switch disagreeing with this one.
    let stored = serde_json::to_value(Settings::default()).unwrap();
    fn keys_naming_pi(value: &serde_json::Value, path: &str, found: &mut Vec<String>) {
        if let serde_json::Value::Object(map) = value {
            for (key, child) in map {
                let here = format!("{path}/{key}");
                if key.contains("pi_activity") {
                    found.push(here.clone());
                }
                keys_naming_pi(child, &here, found);
            }
        }
    }
    let mut found = Vec::new();
    keys_naming_pi(&stored, "", &mut found);
    assert_eq!(found, vec!["/pi_activity_component".to_string()]);
}

#[test]
fn turning_the_pi_activity_switch_off_is_a_choice_not_a_fault() {
    // FR-012f: declining the component leaves the badge unknown, and that is the expected result.
    // So the form treats it like any other preference — no rejection, no error, and the value
    // reaches what Save writes.
    let mut draft = valid();
    draft.show(SettingsSection::Environment);
    draft.environment.pi_activity_component = false;

    assert!(draft.error.is_none());
    let saved = draft
        .validate()
        .expect("an off switch is a valid setting")
        .into_settings();
    assert!(!saved.pi_activity_component);
}

// ---------------------------------------------------------------------------------------
// Feature 041 — Save terminal history (T041; U101–U105; FR-026, FR-031)
// ---------------------------------------------------------------------------------------

#[test]
fn the_draft_holds_the_save_terminal_history_setting_of_the_service() {
    assert!(
        SettingsDraft::from_settings(&Settings::default())
            .terminal
            .save_terminal_history
    );
    let off = Settings {
        save_terminal_history: false,
        ..Settings::default()
    };
    assert!(
        !SettingsDraft::from_settings(&off)
            .terminal
            .save_terminal_history
    );
}

#[test]
fn turning_save_terminal_history_off_reaches_what_save_writes() {
    let mut draft = valid();
    draft.show(SettingsSection::Terminal);
    draft.terminal.save_terminal_history = false;

    assert!(draft.error.is_none());
    let saved = draft
        .validate()
        .expect("an off switch is a valid setting")
        .into_settings();
    assert!(!saved.save_terminal_history);
}

// ---------------------------------------------------------------------------------------
// The tool server's binding toggle (feature 034, T025 — FR-004)
// ---------------------------------------------------------------------------------------

/// U214. The Environment page shows the stored value: on unless the user turned it off.
#[test]
fn the_binding_toggle_is_seeded_from_the_stored_setting() {
    assert!(
        SettingsDraft::from_settings(&Settings::default())
            .environment
            .tool_server_enabled,
        "the binding is on until the user turns it off (FR-004)"
    );
    let off = Settings {
        tool_server_enabled: false,
        ..Settings::default()
    };
    assert!(
        !SettingsDraft::from_settings(&off)
            .environment
            .tool_server_enabled,
        "a user who turned the binding off must see it off when the page opens"
    );
}

/// U215 (the draft half). Turning the row off is a valid preference, and it is what Save writes.
#[test]
fn turning_the_binding_toggle_off_reaches_what_save_writes() {
    let mut draft = valid();
    draft.show(SettingsSection::Environment);
    draft.environment.tool_server_enabled = false;

    let saved = draft
        .validate()
        .expect("an off toggle is a valid setting")
        .into_settings();
    assert!(
        !saved.tool_server_enabled,
        "the toggle the user turned off must be what Save writes"
    );
}

/// Feature 1210 (FR-008, US5): the draft holds the guard switch, on by default; a user who turned
/// it off sees it off when the page opens, and the off toggle is what Save writes.
#[test]
fn the_worktree_guard_row_reflects_and_edits_the_setting() {
    assert!(
        SettingsDraft::from_settings(&Settings::default())
            .environment
            .worktree_guard_enabled,
        "the guard is on until the user turns it off"
    );
    let off = Settings {
        worktree_guard_enabled: false,
        ..Settings::default()
    };
    assert!(
        !SettingsDraft::from_settings(&off)
            .environment
            .worktree_guard_enabled
    );

    let mut draft = valid();
    draft.show(SettingsSection::Environment);
    draft.environment.worktree_guard_enabled = false;
    let saved = draft.validate().expect("valid").into_settings();
    assert!(!saved.worktree_guard_enabled);
}

// ---------------------------------------------------------------------------------------
// The Desktop notifications switch (feature 039, T105 — FR-026)
// ---------------------------------------------------------------------------------------

/// U142, A43 (US4 scenario 1). On default settings the draft holds the switch and it is on; a
/// user who turned it off sees it off when the page opens.
#[test]
fn the_desktop_notifications_switch_is_seeded_from_the_stored_setting() {
    assert!(
        SettingsDraft::from_settings(&Settings::default())
            .environment
            .desktop_notifications,
        "desktop notifications are on until the user turns them off (FR-026)"
    );
    let off = Settings {
        desktop_notifications: false,
        ..Settings::default()
    };
    assert!(
        !SettingsDraft::from_settings(&off)
            .environment
            .desktop_notifications,
        "a user who turned desktop notifications off must see the switch off"
    );
}

/// U143 (the draft half). Off is a valid choice, and it is what Save writes; on is too.
#[test]
fn the_desktop_notifications_switch_reaches_what_save_writes() {
    for chosen in [false, true] {
        let mut draft = valid();
        draft.show(SettingsSection::Environment);
        draft.environment.desktop_notifications = chosen;

        let saved = draft
            .validate()
            .expect("either position of the switch is a valid setting")
            .into_settings();
        assert_eq!(
            saved.desktop_notifications, chosen,
            "the position the user left the switch in must be what Save writes"
        );
    }
}

// ---------------------------------------------------------------------------------------
// Notification kinds and the long-task threshold (feature 613, T035, T060)
// ---------------------------------------------------------------------------------------

use micold_core::attention::{NotificationKind, NotificationKinds};

/// US2.10, S3: the draft opened from the stored settings holds the kinds and the threshold as text.
#[test]
fn the_draft_is_seeded_with_the_stored_kinds_and_threshold() {
    let default = SettingsDraft::from_settings(&Settings::default());
    assert_eq!(
        default.environment.notification_kinds,
        NotificationKinds::default()
    );
    assert_eq!(default.environment.long_task_threshold_secs, "60");

    let kinds = NotificationKinds {
        turn_finished: true,
        needs_permission: false,
        ..NotificationKinds::default()
    };
    let stored = Settings {
        notification_kinds: kinds,
        long_task_threshold_secs: 25,
        ..Settings::default()
    };
    let draft = SettingsDraft::from_settings(&stored);
    assert_eq!(draft.environment.notification_kinds, kinds);
    assert_eq!(draft.environment.long_task_threshold_secs, "25");
}

/// S3: what Save writes carries the whole kinds value, and the threshold as a number.
#[test]
fn save_writes_the_whole_kinds_value_and_the_parsed_threshold() {
    let mut draft = valid();
    draft
        .environment
        .notification_kinds
        .set(NotificationKind::TurnFinished, true);
    draft
        .environment
        .notification_kinds
        .set(NotificationKind::SessionError, false);
    draft.environment.long_task_threshold_secs = "20".into();
    let expected = draft.environment.notification_kinds;

    let valid = draft.validate().expect("valid");
    assert_eq!(valid.notification_kinds, expected);
    assert_eq!(valid.long_task_threshold_secs, 20);
    let saved = valid.into_settings();
    assert_eq!(saved.notification_kinds, expected);
    assert_eq!(saved.long_task_threshold_secs, 20);
}

/// US2.5, FR-012: with the master switch off the kind values and the threshold text are kept.
/// US2.6: turning it back on leaves them as they were.
#[test]
fn the_master_switch_never_changes_the_kinds_or_the_threshold() {
    let mut draft = valid();
    draft
        .environment
        .notification_kinds
        .set(NotificationKind::TurnFinished, true);
    draft.environment.long_task_threshold_secs = "30".into();
    let kinds = draft.environment.notification_kinds;

    draft.environment.desktop_notifications = false;
    assert_eq!(draft.environment.notification_kinds, kinds);
    assert_eq!(draft.environment.long_task_threshold_secs, "30");
    let off = draft.validate().expect("valid while off").into_settings();
    assert_eq!(off.notification_kinds, kinds);
    assert_eq!(off.long_task_threshold_secs, 30);

    draft.environment.desktop_notifications = true;
    assert_eq!(draft.environment.notification_kinds, kinds);
    assert_eq!(draft.environment.long_task_threshold_secs, "30");
}

/// FR-025: the threshold stays valid and editable with Long task finished off.
#[test]
fn the_threshold_is_still_saved_with_long_task_finished_off() {
    let mut draft = valid();
    draft
        .environment
        .notification_kinds
        .set(NotificationKind::LongTaskFinished, false);
    draft.environment.long_task_threshold_secs = "45".into();
    assert_eq!(
        draft.validate().expect("valid").long_task_threshold_secs,
        45
    );
}

/// S6, US2.12, FR-026: out-of-range and non-numeric text refuses the save on the threshold field.
#[test]
fn a_bad_threshold_refuses_the_save_with_the_s6_message() {
    for (text, message) in [
        ("9", "Enter a threshold between 10 and 3600 seconds."),
        ("3601", "Enter a threshold between 10 and 3600 seconds."),
        ("abc", "Enter a whole number of seconds."),
    ] {
        let mut draft = valid();
        draft.environment.long_task_threshold_secs = text.into();
        let error = draft
            .validate()
            .expect_err(text)
            .into_field()
            .expect("a field error");
        assert_eq!(error.field, FieldId::SettingsLongTaskThreshold, "{text:?}");
        assert_eq!(error.section, SettingsSection::Environment);
        assert_eq!(error.message, message, "{text:?}");
    }
    for text in ["10", "3600", "60"] {
        let mut draft = valid();
        draft.environment.long_task_threshold_secs = text.into();
        assert!(draft.validate().is_ok(), "{text:?} is in range");
    }
}

// ---------------------------------------------------------------------------------------
// "Let agents read and type into other sessions" (feature 034, T065 — FR-016)
// ---------------------------------------------------------------------------------------

use micold_core::mcp::policy::CrossSessionAccess;

/// U216. The Environment draft carries the option, seeded from the stored value: Auto unless the
/// user chose otherwise.
#[test]
fn the_cross_session_option_is_seeded_from_the_stored_setting() {
    assert_eq!(
        SettingsDraft::from_settings(&Settings::default())
            .environment
            .cross_session_access,
        CrossSessionAccess::Auto,
        "Auto until the user chooses otherwise (FR-016)"
    );
    for chosen in [CrossSessionAccess::ConfirmEachSend, CrossSessionAccess::Off] {
        let stored = Settings {
            cross_session_access: chosen,
            ..Settings::default()
        };
        assert_eq!(
            SettingsDraft::from_settings(&stored)
                .environment
                .cross_session_access,
            chosen,
            "a user who tightened the option must see their choice when the page opens"
        );
    }
}

/// U216 (the save half). A closed choice: every value is valid, and it is what Save writes.
#[test]
fn every_value_of_the_cross_session_option_reaches_what_save_writes() {
    for chosen in CrossSessionAccess::ALL {
        let mut draft = valid();
        draft.show(SettingsSection::Environment);
        draft.environment.cross_session_access = chosen;

        assert!(draft.error.is_none());
        let saved = draft
            .validate()
            .expect("choosing from a list cannot produce a validation error")
            .into_settings();
        assert_eq!(saved.cross_session_access, chosen);
    }
}

/// The select draws each value by its `Display`, so these are the words the user reads (FR-016).
#[test]
fn the_cross_session_select_names_its_three_values_as_the_requirement_does() {
    let names: Vec<String> = CrossSessionAccess::ALL
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(names, ["Auto", "Confirm each send", "Off"]);
}

// --- Feature 034: the label-to-type mapping rides along with every save (U86) ------------------

#[test]
fn the_mapping_survives_other_saves() {
    use micold_core::issue_types::LabelTypeEntry;
    use micold_core::naming::ConventionalType;

    // FR-016, FR-020: the client's save writes its whole half of the document, so the mapping must
    // be in what it writes or a theme change would reset it to nothing.
    let stored = Settings {
        issue_label_types: vec![LabelTypeEntry {
            label: "perf".to_string(),
            type_: ConventionalType::Perf,
        }],
        ..Settings::default()
    };
    let mut draft = SettingsDraft::from_settings(&stored);
    draft.show(SettingsSection::Appearance);
    draft.appearance.theme = ThemePreference::Dark;

    let saved = draft
        .validate()
        .expect("a theme change is valid")
        .into_settings();
    assert_eq!(saved.theme, ThemePreference::Dark);
    assert_eq!(
        saved.issue_label_types, stored.issue_label_types,
        "a save that changes only the theme keeps the stored mapping"
    );
}

// --- Feature 034 US3: the GitHub issues section edits the mapping (U87–U92) --------------------

mod issue_mapping {
    use micold_client::app::State;
    use micold_client::features::settings::{update, Msg, SettingsDraft, SettingsSection};
    use micold_client::features::window::FieldId;
    use micold_core::issue_types::{default_mapping, LabelTypeEntry};
    use micold_core::naming::ConventionalType;
    use micold_core::settings::Settings;
    use micold_core::typeahead::Direction;

    fn entry(label: &str, type_: ConventionalType) -> LabelTypeEntry {
        LabelTypeEntry {
            label: label.to_string(),
            type_,
        }
    }

    /// Settings open on a valid draft holding `entries`.
    fn open_with(entries: Vec<LabelTypeEntry>) -> State {
        let mut draft = SettingsDraft::from_settings(&Settings::default());
        draft.github.entries = entries;
        let mut state = State::default();
        state.settings.settings_draft = Some(draft);
        state
    }

    fn entries(state: &State) -> Vec<(String, ConventionalType)> {
        state
            .settings
            .settings_draft
            .as_ref()
            .expect("Settings is open")
            .github
            .entries
            .iter()
            .map(|e| (e.label.clone(), e.type_))
            .collect()
    }

    fn pairs(list: &[(&str, ConventionalType)]) -> Vec<(String, ConventionalType)> {
        list.iter().map(|(l, t)| (l.to_string(), *t)).collect()
    }

    fn abc() -> Vec<LabelTypeEntry> {
        vec![
            entry("a", ConventionalType::Fix),
            entry("b", ConventionalType::Feat),
            entry("c", ConventionalType::Docs),
        ]
    }

    #[test]
    fn the_section_is_fifth_and_named() {
        assert_eq!(
            SettingsSection::ALL.get(4),
            Some(&SettingsSection::GithubIssues),
            "GitHub is fifth in the rail, after Daemons (AS1); Jira follows it (feature 533)"
        );
        assert_eq!(SettingsSection::ALL.len(), 6);
        assert_eq!(SettingsSection::ALL.get(5), Some(&SettingsSection::Jira));
        // Feature 040 (FR-029): the page also holds the pull request switch, so it is "GitHub".
        assert_eq!(SettingsSection::GithubIssues.label(), "GitHub");
        assert_eq!(
            SettingsSection::GithubIssues.icon(),
            micold_client::icons::Icon::IssueMapping
        );
    }

    /// FR-014: the list also serves Jira issues, and the page says so; the defaults still map
    /// `Bug` (a Jira issue type name) to `fix`.
    #[test]
    fn the_intro_says_the_list_also_applies_to_jira_issues() {
        assert!(
            micold_client::ui::GITHUB_ISSUES_INTRO.contains(
                "Also applies to Jira issues: an issue's type name and labels are matched against this list."
            ),
            "{}",
            micold_client::ui::GITHUB_ISSUES_INTRO
        );
        let bug =
            micold_core::issue_types::type_for_labels(&default_mapping(), &["Bug".to_string()]);
        assert_eq!(bug, Some(ConventionalType::Fix));
    }

    #[test]
    fn the_draft_loads_the_mapping() {
        let stored = Settings {
            issue_label_types: vec![entry("defect", ConventionalType::Fix)],
            ..Settings::default()
        };
        assert_eq!(
            SettingsDraft::from_settings(&stored).github.entries,
            stored.issue_label_types,
            "the section shows the stored mapping, in order (AS1)"
        );
        let never_edited: Settings =
            serde_json::from_str("{}").expect("an empty document reads as the defaults");
        assert_eq!(
            SettingsDraft::from_settings(&never_edited).github.entries,
            default_mapping(),
            "a mapping never edited shows the default table (AS6)"
        );
    }

    #[test]
    fn entries_are_edited() {
        let mut state = open_with(abc());
        update(&mut state, Msg::IssueMappingAdded);
        assert_eq!(
            entries(&state).last(),
            Some(&(String::new(), ConventionalType::Feat)),
            "Add entry appends a blank label typed `feat` (FR-018)"
        );
        update(
            &mut state,
            Msg::IssueMappingLabelChanged(3, "defect".into()),
        );
        update(
            &mut state,
            Msg::IssueMappingTypeChanged(3, ConventionalType::Fix),
        );
        update(
            &mut state,
            Msg::IssueMappingTypeChanged(0, ConventionalType::Chore),
        );
        update(&mut state, Msg::IssueMappingRemoved(1));
        assert_eq!(
            entries(&state),
            pairs(&[
                ("a", ConventionalType::Chore),
                ("c", ConventionalType::Docs),
                ("defect", ConventionalType::Fix),
            ]),
            "label and type change in place; remove deletes only that entry (AS2, AS3)"
        );
    }

    #[test]
    fn entries_are_reordered() {
        let mut state = open_with(abc());
        update(&mut state, Msg::IssueMappingMoved(0, Direction::Prev));
        update(&mut state, Msg::IssueMappingMoved(2, Direction::Next));
        assert_eq!(
            entries(&state),
            pairs(&[
                ("a", ConventionalType::Fix),
                ("b", ConventionalType::Feat),
                ("c", ConventionalType::Docs),
            ]),
            "moving the first entry up or the last entry down changes nothing"
        );
        update(&mut state, Msg::IssueMappingMoved(2, Direction::Prev));
        assert_eq!(
            entries(&state),
            pairs(&[
                ("a", ConventionalType::Fix),
                ("c", ConventionalType::Docs),
                ("b", ConventionalType::Feat),
            ]),
            "moving up swaps an entry with the one above it (FR-017)"
        );
        update(&mut state, Msg::IssueMappingMoved(0, Direction::Next));
        assert_eq!(
            entries(&state),
            pairs(&[
                ("c", ConventionalType::Docs),
                ("a", ConventionalType::Fix),
                ("b", ConventionalType::Feat),
            ]),
            "moving down swaps an entry with the one below it (FR-017)"
        );
    }

    #[test]
    fn restore_defaults() {
        let mut state = open_with(vec![entry("defect", ConventionalType::Fix)]);
        update(&mut state, Msg::IssueMappingDefaultsRestored);
        assert_eq!(
            state
                .settings
                .settings_draft
                .as_ref()
                .unwrap()
                .github
                .entries,
            default_mapping(),
            "Restore defaults returns the draft to the default table (AS6)"
        );
    }

    #[test]
    fn an_invalid_mapping_refuses_the_save() {
        let mut state = open_with(vec![
            entry("bug", ConventionalType::Fix),
            entry("  ", ConventionalType::Feat),
        ]);
        let draft = state.settings.settings_draft.as_ref().unwrap();
        assert_eq!(
            draft.validate(),
            Err(micold_client::features::settings::Refusal::Missing),
            "a blank label is the required error on its field (feature 937)"
        );

        update(&mut state, Msg::IssueMappingLabelChanged(1, "Bug".into()));
        let draft = state.settings.settings_draft.as_ref().unwrap();
        let error = draft
            .validate()
            .expect_err("a label repeating another ignoring case cannot be saved")
            .into_field()
            .expect("a field error");
        assert_eq!(
            (error.field, error.section),
            (FieldId::IssueMappingLabel(1), SettingsSection::GithubIssues),
            "the duplicate is reported on the later entry (AS5, FR-019)"
        );
        assert!(
            error.message.contains("bug"),
            "the message names the label it repeats, since the rows carry no numbers: {:?}",
            error.message
        );

        update(
            &mut state,
            Msg::IssueMappingLabelChanged(1, " defect ".into()),
        );
        let saved = state
            .settings
            .settings_draft
            .as_ref()
            .unwrap()
            .validate()
            .expect("a valid mapping saves")
            .into_settings();
        assert_eq!(
            saved.issue_label_types,
            vec![
                entry("bug", ConventionalType::Fix),
                entry("defect", ConventionalType::Feat)
            ],
            "a valid save writes the mapping in order, labels trimmed (AS4, FR-020)"
        );
    }
}

// --- Spec 035: the script path check, in the reducer (contracts/settings-indication.md §1) -------

mod script_path_check {
    use micold_client::app::State;
    use micold_client::features::settings::{update, CheckOrigin, Msg, ScriptCheck, SettingsDraft};
    use micold_core::script_path_check::{CheckedScriptPath, ScriptPathState};

    const MISSING: &str = "/tmp/does-not-exist.sh";

    fn checked(state: ScriptPathState) -> CheckedScriptPath {
        CheckedScriptPath {
            path: MISSING.to_string(),
            enabled: false,
            state,
        }
    }

    fn start(state: &mut State, origin: CheckOrigin) -> u64 {
        update(
            state,
            Msg::ScriptPathCheckStarted {
                origin,
                path: MISSING.to_string(),
                enabled: false,
            },
        );
        state.settings.script_check_seq
    }

    fn land(state: &mut State, seq: u64, origin: CheckOrigin, result: Option<CheckedScriptPath>) {
        update(
            state,
            Msg::ScriptPathChecked {
                seq,
                origin,
                result,
            },
        );
    }

    #[test]
    fn starting_a_check_raises_the_sequence_and_keeps_the_previous_answer_showing() {
        let mut state = State::default();
        let previous = checked(ScriptPathState::NotFound { tilde: false });
        state.settings.script_check = ScriptCheck::Done(previous.clone());
        state.settings.script_check_seq = 4;

        update(
            &mut state,
            Msg::ScriptPathCheckStarted {
                origin: CheckOrigin::Opened,
                path: MISSING.to_string(),
                enabled: false,
            },
        );

        assert_eq!(
            state.settings.script_check_seq, 5,
            "each start takes a new number"
        );
        assert_eq!(
            state.settings.script_check,
            ScriptCheck::Pending {
                seq: 5,
                last: Some(previous)
            },
            "a re-check must not blank the notice while it runs"
        );
    }

    #[test]
    fn a_check_of_another_path_or_state_does_not_keep_showing_the_previous_answer() {
        let previous = checked(ScriptPathState::Present);
        for (path, enabled) in [("/tmp/saved-elsewhere.sh", false), (MISSING, true)] {
            let mut state = State::default();
            state.settings.script_check = ScriptCheck::Done(previous.clone());

            update(
                &mut state,
                Msg::ScriptPathCheckStarted {
                    origin: CheckOrigin::Opened,
                    path: path.to_string(),
                    enabled,
                },
            );

            assert_eq!(
                state.settings.script_check,
                ScriptCheck::Pending { seq: 1, last: None },
                "an answer about {MISSING} (off) says nothing true about {path} (enabled = \
                 {enabled}): another window's save changed what is stored (review A)"
            );
        }
    }

    #[test]
    fn a_save_marks_its_check_as_one_to_report_and_an_open_does_not() {
        let mut state = State::default();

        let opened = start(&mut state, CheckOrigin::Opened);
        assert_eq!(
            state.settings.script_check_save_seq, None,
            "opening Settings never asks for a report (FR-007), check {opened}"
        );

        let saved = start(&mut state, CheckOrigin::Saved);
        assert_eq!(
            state.settings.script_check_save_seq,
            Some(saved),
            "a save's check is the one whose result may be reported (FR-004)"
        );
    }

    #[test]
    fn the_current_checks_answer_is_shown() {
        let mut state = State::default();
        let seq = start(&mut state, CheckOrigin::Opened);
        let answer = checked(ScriptPathState::NotFound { tilde: false });

        land(&mut state, seq, CheckOrigin::Opened, Some(answer.clone()));

        assert_eq!(state.settings.script_check, ScriptCheck::Done(answer));
    }

    #[test]
    fn a_blank_paths_answer_leaves_nothing_to_show() {
        let mut state = State::default();
        state.settings.script_check =
            ScriptCheck::Done(checked(ScriptPathState::NotFound { tilde: false }));
        let seq = start(&mut state, CheckOrigin::Opened);

        land(&mut state, seq, CheckOrigin::Opened, None);

        assert_eq!(
            state.settings.script_check,
            ScriptCheck::Idle,
            "a blank path has no check, so the old answer must go (FR-011)"
        );
    }

    #[test]
    fn an_older_checks_answer_is_dropped() {
        let mut state = State::default();
        let older = start(&mut state, CheckOrigin::Opened);
        let newer = start(&mut state, CheckOrigin::Opened);
        let before = state.settings.script_check.clone();

        land(
            &mut state,
            older,
            CheckOrigin::Opened,
            Some(checked(ScriptPathState::NotFound { tilde: false })),
        );

        assert_eq!(
            state.settings.script_check, before,
            "check {older} was superseded by {newer}: a slow answer about an older path must not \
             overwrite the newer one"
        );
    }

    #[test]
    fn neither_check_message_touches_the_draft_or_a_setting() {
        let mut state = State::default();
        let mut draft = SettingsDraft::default();
        draft.environment.enabled = true;
        draft.environment.script_path = MISSING.to_string();
        draft.environment.timeout_secs = "5".to_string();
        state.settings.settings_draft = Some(draft);
        let before = state.clone();

        let seq = start(&mut state, CheckOrigin::Saved);
        land(
            &mut state,
            seq,
            CheckOrigin::Saved,
            Some(checked(ScriptPathState::NotFound { tilde: false })),
        );

        let mut after = state.clone();
        after.settings.script_check = before.settings.script_check.clone();
        after.settings.script_check_seq = before.settings.script_check_seq;
        after.settings.script_check_save_seq = before.settings.script_check_save_seq;
        assert_eq!(
            after, before,
            "a check reports on the path and changes nothing else: no recovery, no rewrite \
             (FR-008, FR-010)"
        );
    }

    // --- S5–S7: the save-time notification (FR-004, FR-007) ---------------------------------

    use micold_client::features::Outcome;
    use micold_core::notify::{Level, Notification};

    const NOT_FOUND: &str = "The environment-include script was not found: /tmp/does-not-exist.sh";

    fn info(message: &str) -> Vec<Outcome> {
        vec![Outcome::NotificationRaised(Notification::new(
            Level::Info,
            message,
        ))]
    }

    /// What `update` returns when a check with `seq` lands.
    fn landed(
        state: &mut State,
        seq: u64,
        origin: CheckOrigin,
        result: Option<CheckedScriptPath>,
    ) -> Vec<Outcome> {
        update(
            state,
            Msg::ScriptPathChecked {
                seq,
                origin,
                result,
            },
        )
    }

    /// A save's check, started and landed with `state`, and what `update` returned.
    fn saved_with(state: ScriptPathState) -> Vec<Outcome> {
        let mut app = State::default();
        let seq = start(&mut app, CheckOrigin::Saved);
        landed(&mut app, seq, CheckOrigin::Saved, Some(checked(state)))
    }

    #[test]
    fn a_save_leaving_a_missing_path_posts_one_notice_naming_it() {
        assert_eq!(
            saved_with(ScriptPathState::NotFound { tilde: false }),
            info(NOT_FOUND),
            "US1 scenario 5: the save goes through and one Info notice names the path (FR-004)"
        );
    }

    #[test]
    fn a_missing_path_starting_with_a_tilde_says_the_tilde_is_not_expanded() {
        assert_eq!(
            saved_with(ScriptPathState::NotFound { tilde: true }),
            info(&format!("{NOT_FOUND} (~ is not expanded; use a full path)")),
        );
    }

    #[test]
    fn a_path_that_is_not_a_readable_file_says_so() {
        assert_eq!(
            saved_with(ScriptPathState::NotReadable),
            info("The environment-include script is not a readable file: /tmp/does-not-exist.sh"),
        );
    }

    #[test]
    fn a_save_with_nothing_wrong_to_report_posts_nothing() {
        for state in [
            ScriptPathState::Present,
            ScriptPathState::Relative,
            ScriptPathState::Unchecked,
        ] {
            assert_eq!(
                saved_with(state.clone()),
                Vec::<Outcome>::new(),
                "{state:?} has nothing to report at save time (S6)"
            );
        }
        let mut app = State::default();
        let seq = start(&mut app, CheckOrigin::Saved);
        assert_eq!(
            landed(&mut app, seq, CheckOrigin::Saved, None),
            Vec::<Outcome>::new(),
            "a blank path is not checked, so there is nothing to report (S6)"
        );
    }

    #[test]
    fn opening_settings_never_posts_a_notice() {
        let mut app = State::default();
        let seq = start(&mut app, CheckOrigin::Opened);
        assert_eq!(
            landed(
                &mut app,
                seq,
                CheckOrigin::Opened,
                Some(checked(ScriptPathState::NotFound { tilde: false })),
            ),
            Vec::<Outcome>::new(),
            "the report outside a save stays on the Settings page (FR-007, S7)"
        );
    }

    #[test]
    fn a_saves_notice_is_posted_even_when_a_newer_open_took_over_the_page() {
        let mut app = State::default();
        let save = start(&mut app, CheckOrigin::Saved);
        let _open = start(&mut app, CheckOrigin::Opened);

        assert_eq!(
            landed(
                &mut app,
                save,
                CheckOrigin::Saved,
                Some(checked(ScriptPathState::NotFound { tilde: false })),
            ),
            info(NOT_FOUND),
            "Save closed Settings, so the notice is the only report this save gets (S5 runs \
             whether or not S4 applied)"
        );
    }

    #[test]
    fn an_older_saves_answer_is_not_reported_once_a_newer_save_started() {
        let mut app = State::default();
        let older = start(&mut app, CheckOrigin::Saved);
        let _newer = start(&mut app, CheckOrigin::Saved);

        assert_eq!(
            landed(
                &mut app,
                older,
                CheckOrigin::Saved,
                Some(checked(ScriptPathState::NotFound { tilde: false })),
            ),
            Vec::<Outcome>::new(),
            "the newer save's own check reports on the path as it is now"
        );
    }

    #[test]
    fn a_saves_answer_delivered_twice_is_reported_once() {
        let mut app = State::default();
        let seq = start(&mut app, CheckOrigin::Saved);
        let missing = Some(checked(ScriptPathState::NotFound { tilde: false }));

        let first = landed(&mut app, seq, CheckOrigin::Saved, missing.clone());
        let second = landed(&mut app, seq, CheckOrigin::Saved, missing);

        assert_eq!(first, info(NOT_FOUND));
        assert_eq!(second, Vec::<Outcome>::new(), "one save, one notice");
    }
}

// --- Spec 035: what the Environment page says, feature off (contracts/settings-indication.md §2) --

mod script_path_notice_off {
    use micold_client::features::settings::{script_path_notice, NoticeLine, ScriptCheck};
    use micold_core::env_include::EnvIncludeOutcome;
    use micold_core::script_path_check::{CheckedScriptPath, ScriptPathState};

    pub(super) const P: &str = "/tmp/does-not-exist.sh";
    pub(super) const OFF: &str =
        "Environment include is off, so no script is sourced. Turning it on will \
                       not source one until this path names a readable file.";
    pub(super) const TILDE: &str = "~ is not expanded. Use a full path.";
    pub(super) const REL: &str =
        "Relative path: whether the script is found depends on each session's \
                       directory.";
    const HUNG: &str =
        "No answer within 2 seconds. The file may be on a drive that is not responding.";
    pub(super) const DIAGNOSTIC: &str = "env.sh: line 3: nvm: command not found";

    pub(super) fn done(state: ScriptPathState, enabled: bool) -> ScriptCheck {
        ScriptCheck::Done(CheckedScriptPath {
            path: P.to_string(),
            enabled,
            state,
        })
    }

    pub(super) fn caution(text: &str) -> NoticeLine {
        NoticeLine::Caution(text.to_string())
    }

    pub(super) fn note(text: &str) -> NoticeLine {
        NoticeLine::Note(text.to_string())
    }

    pub(super) fn non_zero_exit() -> EnvIncludeOutcome {
        EnvIncludeOutcome::NonZeroExit {
            code: 1,
            diagnostic: DIAGNOSTIC.to_string(),
        }
    }

    pub(super) fn timed_out() -> EnvIncludeOutcome {
        EnvIncludeOutcome::TimedOut {
            diagnostic: DIAGNOSTIC.to_string(),
        }
    }

    /// 011's lines for `outcome` (FR-013 of 011). From M3 its `MissingScript` line names the
    /// path whenever a check has named one (spec 035 FR-005, contracts §2 "011(o)").
    pub(super) fn lines_011(outcome: &EnvIncludeOutcome, path: Option<&str>) -> Vec<NoticeLine> {
        match outcome {
            EnvIncludeOutcome::Disabled | EnvIncludeOutcome::Success => vec![],
            EnvIncludeOutcome::MissingScript => match path {
                Some(path) => vec![caution(&format!("Script not found: {path}"))],
                None => vec![caution("Script not found")],
            },
            EnvIncludeOutcome::NonZeroExit { diagnostic, .. } => {
                vec![caution("Exited with an error"), note(diagnostic)]
            }
            EnvIncludeOutcome::TimedOut { diagnostic } => {
                vec![caution("Timed out"), note(diagnostic)]
            }
        }
    }

    pub(super) fn every_outcome() -> Vec<EnvIncludeOutcome> {
        vec![
            EnvIncludeOutcome::Disabled,
            EnvIncludeOutcome::Success,
            EnvIncludeOutcome::MissingScript,
            non_zero_exit(),
            timed_out(),
        ]
    }

    pub(super) fn every_state() -> Vec<ScriptPathState> {
        vec![
            ScriptPathState::Present,
            ScriptPathState::NotFound { tilde: false },
            ScriptPathState::NotFound { tilde: true },
            ScriptPathState::NotReadable,
            ScriptPathState::Relative,
            ScriptPathState::Unchecked,
        ]
    }

    #[test]
    fn with_no_check_yet_the_page_shows_011s_note_unchanged() {
        assert_eq!(
            script_path_notice(&ScriptCheck::Idle, &non_zero_exit()),
            vec![caution("Exited with an error"), note(DIAGNOSTIC)]
        );
        assert_eq!(
            script_path_notice(&ScriptCheck::Idle, &timed_out()),
            vec![caution("Timed out"), note(DIAGNOSTIC)]
        );
        assert_eq!(
            script_path_notice(&ScriptCheck::Idle, &EnvIncludeOutcome::MissingScript),
            vec![caution("Script not found")]
        );
        for quiet in [EnvIncludeOutcome::Success, EnvIncludeOutcome::Disabled] {
            assert_eq!(
                script_path_notice(&ScriptCheck::Idle, &quiet),
                vec![],
                "{quiet:?} has nothing to report (N1)"
            );
        }
    }

    #[test]
    fn a_check_in_flight_shows_the_previous_answer_or_011s_note_when_there_is_none() {
        let previous = CheckedScriptPath {
            path: P.to_string(),
            enabled: false,
            state: ScriptPathState::NotFound { tilde: false },
        };
        let outcome = EnvIncludeOutcome::Disabled;

        assert_eq!(
            script_path_notice(
                &ScriptCheck::Pending {
                    seq: 2,
                    last: Some(previous.clone())
                },
                &outcome
            ),
            script_path_notice(&ScriptCheck::Done(previous), &outcome),
            "a re-check must not blank the notice"
        );
        assert_eq!(
            script_path_notice(
                &ScriptCheck::Pending { seq: 1, last: None },
                &non_zero_exit()
            ),
            lines_011(&non_zero_exit(), None),
            "with no previous answer, the page is 011's (N1)"
        );
    }

    #[test]
    fn off_and_not_found_says_so_by_path_and_that_the_feature_is_off() {
        assert_eq!(
            script_path_notice(
                &done(ScriptPathState::NotFound { tilde: false }, false),
                &EnvIncludeOutcome::Disabled
            ),
            vec![caution(&format!("Script not found: {P}")), note(OFF)],
            "issue #435 (N2)"
        );
    }

    #[test]
    fn off_and_a_tilde_path_explains_that_tilde_is_not_expanded() {
        assert_eq!(
            script_path_notice(
                &done(ScriptPathState::NotFound { tilde: true }, false),
                &EnvIncludeOutcome::Disabled
            ),
            vec![
                caution(&format!("Script not found: {P}")),
                note(TILDE),
                note(OFF)
            ],
            "N5, feature off"
        );
    }

    #[test]
    fn off_and_not_readable_says_so_by_path_and_that_the_feature_is_off() {
        assert_eq!(
            script_path_notice(
                &done(ScriptPathState::NotReadable, false),
                &EnvIncludeOutcome::Disabled
            ),
            vec![caution(&format!("Not a readable file: {P}")), note(OFF)],
            "N6"
        );
    }

    /// 011's lines after a check that named `P`, with the `MissingScript` line spelled out as a
    /// literal: it is the one line spec 035 changed (FR-005), so it must not be derived from a
    /// helper that mirrors the production mapping. The other outcomes are 011's own, unchanged.
    fn literal_011(outcome: &EnvIncludeOutcome) -> Vec<NoticeLine> {
        match outcome {
            EnvIncludeOutcome::MissingScript => {
                vec![caution("Script not found: /tmp/does-not-exist.sh")]
            }
            other => lines_011(other, Some(P)),
        }
    }

    #[test]
    fn a_relative_path_says_it_is_not_checked_then_011s_note() {
        for outcome in every_outcome() {
            let mut expected = vec![note(REL)];
            expected.extend(literal_011(&outcome));
            assert_eq!(
                script_path_notice(&done(ScriptPathState::Relative, false), &outcome),
                expected,
                "N8 with {outcome:?}: 011's lines, naming the path the check knows"
            );
        }
    }

    #[test]
    fn a_check_with_no_answer_says_it_could_not_check_then_011s_note() {
        for outcome in every_outcome() {
            let mut expected = vec![
                caution(&format!("Couldn't check the script path: {P}")),
                note(HUNG),
            ];
            expected.extend(literal_011(&outcome));
            assert_eq!(
                script_path_notice(&done(ScriptPathState::Unchecked, false), &outcome),
                expected,
                "N9 with {outcome:?}"
            );
        }
    }

    #[test]
    fn off_and_present_says_nothing() {
        assert_eq!(
            script_path_notice(
                &done(ScriptPathState::Present, false),
                &EnvIncludeOutcome::Disabled
            ),
            vec![],
            "a readable file is not a problem (N11, SC-002)"
        );
    }

    #[test]
    fn every_off_state_problem_names_the_path_and_says_the_feature_is_off_once() {
        let problems = [
            ScriptPathState::NotFound { tilde: false },
            ScriptPathState::NotFound { tilde: true },
            ScriptPathState::NotReadable,
        ];
        for state in problems {
            for outcome in every_outcome() {
                let lines = script_path_notice(&done(state.clone(), false), &outcome);
                assert!(
                    matches!(lines.first(), Some(NoticeLine::Caution(c)) if c.ends_with(P)),
                    "{state:?}: the first line names the path (FR-002), got {lines:?}"
                );
                assert_eq!(
                    lines.iter().filter(|l| **l == note(OFF)).count(),
                    1,
                    "{state:?} with {outcome:?}: the OFF note appears exactly once"
                );
            }
        }
    }
}

// --- Spec 035: what the Environment page says, feature on (contracts/settings-indication.md §2) --

mod script_path_notice_on {
    use super::script_path_notice_off::{
        caution, done, every_outcome, every_state, lines_011, non_zero_exit, note, DIAGNOSTIC, OFF,
        P, REL, TILDE,
    };
    use micold_client::features::settings::{script_path_notice, NoticeLine, ScriptCheck};
    use micold_core::env_include::EnvIncludeOutcome;
    use micold_core::script_path_check::ScriptPathState;

    const ON: &str = "Environment include is on, but the script cannot be sourced until this path \
                      names a readable file.";
    const NOT_FOUND: ScriptPathState = ScriptPathState::NotFound { tilde: false };

    #[test]
    fn on_and_not_found_after_a_missing_script_attempt_says_it_once_by_path() {
        assert_eq!(
            script_path_notice(&done(NOT_FOUND, true), &EnvIncludeOutcome::MissingScript),
            vec![caution(&format!("Script not found: {P}")), note(ON)],
            "N3: 011's line is merged into the path's, not repeated (FR-005)"
        );
    }

    #[test]
    fn on_and_not_found_after_another_failure_keeps_011s_note_after_the_path() {
        assert_eq!(
            script_path_notice(&done(NOT_FOUND, true), &non_zero_exit()),
            vec![
                caution(&format!("Script not found: {P}")),
                note(ON),
                caution("Exited with an error"),
                note(DIAGNOSTIC),
            ],
            "N4"
        );
    }

    #[test]
    fn on_and_a_tilde_path_explains_that_tilde_is_not_expanded() {
        assert_eq!(
            script_path_notice(
                &done(ScriptPathState::NotFound { tilde: true }, true),
                &EnvIncludeOutcome::MissingScript
            ),
            vec![
                caution(&format!("Script not found: {P}")),
                note(TILDE),
                note(ON)
            ],
            "N5, feature on"
        );
    }

    #[test]
    fn on_and_not_readable_shows_the_path_caution_then_011s_note() {
        assert_eq!(
            script_path_notice(&done(ScriptPathState::NotReadable, true), &non_zero_exit()),
            vec![
                caution(&format!("Not a readable file: {P}")),
                note(ON),
                caution("Exited with an error"),
                note(DIAGNOSTIC),
            ],
            "N7: a directory gets both notes (Edge Cases)"
        );
    }

    #[test]
    fn on_and_not_readable_after_a_missing_script_attempt_says_it_once_by_path() {
        assert_eq!(
            script_path_notice(
                &done(ScriptPathState::NotReadable, true),
                &EnvIncludeOutcome::MissingScript
            ),
            vec![caution(&format!("Not a readable file: {P}")), note(ON)],
            "N7: a path the probe cannot read that the resolver did not find is one problem, not \
             two contradicting cautions (FR-005, review A)"
        );
    }

    #[test]
    fn on_and_present_after_a_missing_script_attempt_says_the_file_exists_now() {
        assert_eq!(
            script_path_notice(
                &done(ScriptPathState::Present, true),
                &EnvIncludeOutcome::MissingScript
            ),
            vec![
                caution("The last attempt could not find the script"),
                note(&format!(
                    "{P} exists now. Save Settings or restart a session to source it."
                )),
            ],
            "N10 (FR-014)"
        );
    }

    #[test]
    fn on_and_present_after_a_successful_attempt_says_nothing() {
        assert_eq!(
            script_path_notice(
                &done(ScriptPathState::Present, true),
                &EnvIncludeOutcome::Success
            ),
            vec![],
            "N11, the other side of N10 (SC-002)"
        );
    }

    #[test]
    fn a_relative_path_after_a_missing_script_attempt_names_the_path_in_011s_line() {
        for enabled in [false, true] {
            assert_eq!(
                script_path_notice(
                    &done(ScriptPathState::Relative, enabled),
                    &EnvIncludeOutcome::MissingScript
                ),
                vec![note(REL), caution(&format!("Script not found: {P}"))],
                "N8, enabled = {enabled}"
            );
        }
    }

    #[test]
    fn without_a_checked_path_011s_missing_script_line_reads_as_before() {
        assert_eq!(
            script_path_notice(&ScriptCheck::Idle, &EnvIncludeOutcome::MissingScript),
            lines_011(&EnvIncludeOutcome::MissingScript, None),
            "no check has named the path, so 011's line cannot either"
        );
    }

    #[test]
    fn switching_the_feature_changes_only_what_is_said_about_its_effect() {
        for state in every_state() {
            for outcome in every_outcome() {
                if state == ScriptPathState::Present && outcome == EnvIncludeOutcome::MissingScript
                {
                    // N10: a stale attempt, which only the feature-on page has; not a not-found
                    // indication (SC-002). Off, the same pair is N11: 011's line, path named.
                    assert_eq!(
                        script_path_notice(&done(state.clone(), false), &outcome),
                        vec![caution(&format!("Script not found: {P}"))],
                        "N11: FR-014's note is the feature-on page's only"
                    );
                    continue;
                }
                let first_caution = |enabled| {
                    script_path_notice(&done(state.clone(), enabled), &outcome)
                        .into_iter()
                        .find(|l| matches!(l, NoticeLine::Caution(_)))
                };
                assert_eq!(
                    first_caution(true),
                    first_caution(false),
                    "SC-003: {state:?} with {outcome:?} reads the same on and off"
                );
            }
        }
    }

    #[test]
    fn no_page_says_script_not_found_twice() {
        let mut checks = vec![ScriptCheck::Idle];
        for state in every_state() {
            for enabled in [false, true] {
                checks.push(done(state.clone(), enabled));
            }
        }
        for check in &checks {
            for outcome in every_outcome() {
                let lines = script_path_notice(check, &outcome);
                assert!(
                    lines
                        .iter()
                        .filter(|l| matches!(l, NoticeLine::Caution(c) if c.starts_with("Script not found")))
                        .count()
                        <= 1,
                    "FR-005: {check:?} with {outcome:?} gave {lines:?}"
                );
            }
        }
    }

    #[test]
    fn every_on_state_problem_names_the_path_and_says_the_feature_is_on_once() {
        let problems = [
            NOT_FOUND,
            ScriptPathState::NotFound { tilde: true },
            ScriptPathState::NotReadable,
        ];
        for state in problems {
            for outcome in every_outcome() {
                let lines = script_path_notice(&done(state.clone(), true), &outcome);
                assert!(
                    matches!(lines.first(), Some(NoticeLine::Caution(c)) if c.ends_with(P)),
                    "{state:?}: the first line names the path (FR-002), got {lines:?}"
                );
                assert_eq!(
                    (
                        lines.iter().filter(|l| **l == note(ON)).count(),
                        lines.iter().filter(|l| **l == note(OFF)).count()
                    ),
                    (1, 0),
                    "{state:?} with {outcome:?}: exactly one ON note and no OFF note"
                );
            }
        }
    }
}

// --- 011 BUG-454: each session directory whose own resolution failed (FR-022) ---

mod directory_failure_lines {
    use super::script_path_notice_off::{caution, non_zero_exit, note, timed_out, DIAGNOSTIC};
    use micold_client::features::settings::directory_failure_lines;
    use micold_core::env_include::EnvIncludeOutcome;
    use micold_core::protocol::messages::EnvIncludeFailure;
    use std::path::PathBuf;

    const DIR_A: &str = "/work/app/.claude/worktrees/feat-a";
    const DIR_B: &str = "/work/other";

    fn failure(dir: &str, outcome: EnvIncludeOutcome) -> EnvIncludeFailure {
        EnvIncludeFailure {
            dir: PathBuf::from(dir),
            outcome,
        }
    }

    /// Each failing directory gets a caution naming it and FR-013's category label, then a note
    /// holding what the script printed there.
    #[test]
    fn each_failing_directory_is_named_with_its_category_and_its_output() {
        let failures = [failure(DIR_A, non_zero_exit()), failure(DIR_B, timed_out())];
        assert_eq!(
            directory_failure_lines(&failures, &EnvIncludeOutcome::Success),
            vec![
                caution(&format!("Exited with an error in {DIR_A}")),
                note(DIAGNOSTIC),
                caution(&format!("Timed out in {DIR_B}")),
                note(DIAGNOSTIC),
            ]
        );
    }

    /// A missing script has no output, and an attempt that printed nothing adds no empty note.
    #[test]
    fn a_failure_with_no_output_has_no_note() {
        let failures = [
            failure(DIR_A, EnvIncludeOutcome::MissingScript),
            failure(
                DIR_B,
                EnvIncludeOutcome::NonZeroExit {
                    code: 2,
                    diagnostic: String::new(),
                },
            ),
        ];
        assert_eq!(
            directory_failure_lines(&failures, &EnvIncludeOutcome::Success),
            vec![
                caution(&format!("Script not found in {DIR_A}")),
                caution(&format!("Exited with an error in {DIR_B}")),
            ]
        );
    }

    /// A directory whose failure is the one the representative note above already shows is folded
    /// into it (FR-022 MAY): an absolute path missing everywhere still reads as one note.
    #[test]
    fn a_failure_equal_to_the_representative_note_is_folded_into_it() {
        let failures = [
            failure(DIR_A, EnvIncludeOutcome::MissingScript),
            failure(DIR_B, non_zero_exit()),
        ];
        assert_eq!(
            directory_failure_lines(&failures, &EnvIncludeOutcome::MissingScript),
            vec![
                caution(&format!("Exited with an error in {DIR_B}")),
                note(DIAGNOSTIC),
            ]
        );
    }

    #[test]
    fn no_failures_say_nothing() {
        assert_eq!(
            directory_failure_lines(&[], &non_zero_exit()),
            Vec::<micold_client::features::settings::NoticeLine>::new()
        );
    }
}

// ---- The diff layout (feature 482, T042, R12, contracts/changes-view.md D1) ----

use micold_client::features::changes::{self, Effect as ChangesEffect, Msg as ChangesMsg};
use micold_client::features::settings::diff_layout_set;
use micold_core::protocol::messages::ClientMsg;
use micold_core::session::SessionLocation;
use micold_core::settings::DiffLayout;

#[test]
fn choosing_a_layout_tells_the_service_that_layout_and_nothing_else() {
    let mut state = changes::State::default();
    let effect = changes::update(&mut state, ChangesMsg::LayoutChosen(DiffLayout::SideBySide));
    assert_eq!(effect, ChangesEffect::SetLayout(DiffLayout::SideBySide));
    assert_eq!(
        state.layout,
        DiffLayout::SideBySide,
        "the choice shows at once, before the echo"
    );
    match diff_layout_set(7, DiffLayout::SideBySide) {
        ClientMsg::SettingsSet {
            req,
            scrollback_lines,
            env_include_enabled,
            env_include_script_path,
            env_include_timeout_secs,
            default_ai_cli,
            default_prompt_template,
            pi_activity_component,
            save_terminal_history,
            tool_server_enabled,
            worktree_guard_enabled,
            cross_session_access,
            pr_status_enabled,
            jira_enabled,
            jira_connection,
            github_issues_enabled,
            jira_base_url,
            desktop_notifications,
            notification_kinds,
            long_task_threshold_secs,
            diff_layout,
            plan_usage_enabled,
            plan_usage_warn_percent,
        } => {
            assert_eq!((req, diff_layout), (7, Some(DiffLayout::SideBySide)));
            assert!(
                scrollback_lines.is_none()
                    && env_include_enabled.is_none()
                    && env_include_script_path.is_none()
                    && env_include_timeout_secs.is_none()
                    && default_ai_cli.is_none()
                    && default_prompt_template.is_none()
                    && pi_activity_component.is_none()
                    && save_terminal_history.is_none()
                    && tool_server_enabled.is_none()
                    && worktree_guard_enabled.is_none()
                    && cross_session_access.is_none()
                    && pr_status_enabled.is_none()
                    && jira_enabled.is_none()
                    && jira_connection.is_none()
                    && github_issues_enabled.is_none()
                    && jira_base_url.is_none()
                    && desktop_notifications.is_none()
                    && notification_kinds.is_none()
                    && long_task_threshold_secs.is_none()
                    && plan_usage_enabled.is_none()
                    && plan_usage_warn_percent.is_none(),
                "every other setting is left as it is"
            );
        }
        other => panic!("expected SettingsSet, got {other:?}"),
    }
}

#[test]
fn the_layout_in_force_comes_from_the_service_and_outlives_the_view() {
    let mut state = changes::State::default();
    assert_eq!(state.layout, DiffLayout::Unified, "unified until told");
    let effect = changes::update(
        &mut state,
        ChangesMsg::LayoutInForce(DiffLayout::SideBySide),
    );
    assert_eq!(effect, ChangesEffect::None, "an echo sends nothing back");
    assert_eq!(state.layout, DiffLayout::SideBySide);
    let entry = SessionLocation::Worktree("wt".into());
    let _ = changes::update(
        &mut state,
        ChangesMsg::Opened {
            project: std::path::PathBuf::from("/p"),
            entry,
        },
    );
    let _ = changes::update(&mut state, ChangesMsg::Closed);
    assert_eq!(
        state.layout,
        DiffLayout::SideBySide,
        "closing and reopening the view keeps the layout"
    );
    let _ = changes::update(&mut state, ChangesMsg::LayoutInForce(DiffLayout::Unified));
    assert_eq!(
        state.layout,
        DiffLayout::Unified,
        "another window's choice applies"
    );
}

// --- Feature 040 US1: the pull request switch (U104–U107) ---------------------------------------

mod pr_status_switch {
    use micold_client::app::State;
    use micold_client::features::settings::{
        update, Msg, SettingsDraft, SettingsSection, ValidSettings,
    };
    use micold_core::settings::Settings;

    fn open(stored: bool) -> State {
        let settings = Settings {
            pr_status_enabled: stored,
            ..Settings::default()
        };
        let mut state = State::default();
        state.settings.settings_draft = Some(SettingsDraft::from_settings(&settings));
        state
    }

    fn shown(state: &State) -> bool {
        state
            .settings
            .settings_draft
            .as_ref()
            .expect("Settings is open")
            .github
            .pr_status_enabled
    }

    /// FR-030: off until the user turns it on; a draft seeded from the stored value shows it.
    #[test]
    fn the_draft_is_seeded_from_the_stored_value_and_off_by_default() {
        assert!(!shown(&open(false)));
        assert!(shown(&open(true)));
        assert!(
            !SettingsDraft::default().github.pr_status_enabled,
            "a never-seeded draft shows the switch off"
        );
    }

    /// U104.
    #[test]
    fn toggling_sets_the_draft_to_the_value_given() {
        let mut state = open(false);
        update(&mut state, Msg::PrStatusToggled(true));
        assert!(shown(&state));
        update(&mut state, Msg::PrStatusToggled(false));
        assert!(!shown(&state));
    }

    /// U104: it is an edit like any other, so a rejection shown earlier no longer describes the form.
    #[test]
    fn toggling_clears_a_shown_error() {
        let mut state = open(false);
        let draft = state.settings.settings_draft.as_mut().unwrap();
        draft.terminal.scrollback_lines = "x".into();
        let error = draft
            .validate()
            .expect_err("not a number")
            .into_field()
            .expect("a field error");
        draft.report(error);
        update(&mut state, Msg::PrStatusToggled(true));
        assert!(state.settings.settings_draft.unwrap().error.is_none());
    }

    /// U105 (the draft half): what Save writes carries the switch.
    #[test]
    fn save_writes_the_switch() {
        for chosen in [false, true] {
            let mut state = open(!chosen);
            update(&mut state, Msg::PrStatusToggled(chosen));
            let valid: ValidSettings = state.settings.settings_draft.unwrap().validate().unwrap();
            assert_eq!(valid.pr_status_enabled, chosen);
            assert_eq!(valid.into_settings().pr_status_enabled, chosen);
        }
    }

    /// U107.
    #[test]
    fn the_section_is_titled_github() {
        assert_eq!(SettingsSection::GithubIssues.label(), "GitHub");
    }
}

// --- Feature 1102: the GitHub issues switch (T008) ------------------------------------------------

mod github_issues_switch {
    use micold_client::app::State;
    use micold_client::features::settings::{update, Msg, SettingsDraft, ValidSettings};
    use micold_core::settings::Settings;

    fn open(stored: bool) -> State {
        let settings = Settings {
            github_issues_enabled: stored,
            ..Settings::default()
        };
        let mut state = State::default();
        state.settings.settings_draft = Some(SettingsDraft::from_settings(&settings));
        state
    }

    fn shown(state: &State) -> bool {
        state
            .settings
            .settings_draft
            .as_ref()
            .unwrap()
            .github
            .issues_enabled
    }

    /// FR-001, FR-007: on until the user turns it off; the app state and a fresh draft say so.
    #[test]
    fn it_is_on_by_default_and_the_draft_is_seeded_from_the_stored_value() {
        assert!(State::default().settings.github_issues_enabled);
        assert!(SettingsDraft::default().github.issues_enabled);
        assert!(shown(&open(true)));
        assert!(!shown(&open(false)));
    }

    #[test]
    fn toggling_edits_the_draft_and_save_carries_it() {
        for chosen in [false, true] {
            let mut state = open(!chosen);
            update(&mut state, Msg::GithubIssuesToggled(chosen));
            assert_eq!(shown(&state), chosen);
            let valid: ValidSettings = state.settings.settings_draft.unwrap().validate().unwrap();
            assert_eq!(valid.github_issues_enabled, chosen);
            assert_eq!(valid.into_settings().github_issues_enabled, chosen);
        }
    }

    /// The baseline a save is compared against holds the value the draft was seeded from.
    #[test]
    fn the_baseline_is_seeded_with_the_stored_value() {
        let draft = open(false).settings.settings_draft.unwrap();
        assert_eq!(
            draft.baseline.as_ref().map(|b| b.github_issues_enabled),
            Some(false)
        );
    }
}

// ---- Feature 490, T043: Claude plan usage (US4 s1–s4; FR-001, FR-005) --------------------------

#[test]
fn the_plan_usage_drafts_start_from_the_stored_settings() {
    let draft = SettingsDraft::from_settings(&micold_core::settings::Settings::default());
    assert!(
        draft.environment.plan_usage_enabled,
        "on by default (FR-001)"
    );
    assert_eq!(draft.environment.plan_usage_warn_percent, "80");
}

#[test]
fn a_threshold_outside_50_to_100_refuses_the_save() {
    for text in ["49", "101", "abc"] {
        let mut draft = valid();
        draft.environment.plan_usage_warn_percent = text.into();
        let error = draft
            .validate()
            .expect_err(text)
            .into_field()
            .expect("a field error");
        assert_eq!(error.field, FieldId::SettingsPlanUsageWarn, "{text:?}");
        assert_eq!(error.section, SettingsSection::Environment);
        assert_eq!(
            error.message, "Enter a whole number from 50 to 100",
            "{text:?}"
        );
    }
}

#[test]
fn a_valid_plan_usage_save_carries_both_fields() {
    let mut draft = valid();
    draft.environment.plan_usage_enabled = false;
    draft.environment.plan_usage_warn_percent = " 65 ".into();
    let settings = draft.validate().expect("valid").into_settings();
    assert!(!settings.plan_usage_enabled);
    assert_eq!(settings.plan_usage_warn_percent, 65);
}

// --- Feature 533 US2: the Jira switch (T015) ---------------------------------------------------

mod jira_switch {
    use micold_client::app::State;
    use micold_client::features::settings::{update, Msg, SettingsDraft, SettingsSection};
    use micold_core::jira::JiraConnection;
    use micold_core::settings::Settings;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn open(stored: bool) -> State {
        let settings = Settings {
            jira_enabled: stored,
            ..Settings::default()
        };
        let mut state = State::default();
        state.settings.settings_draft = Some(SettingsDraft::from_settings(&settings));
        state
    }

    fn draft(state: &State) -> &SettingsDraft {
        state
            .settings
            .settings_draft
            .as_ref()
            .expect("Settings is open")
    }

    /// FR-002: the draft follows the persisted switch, off by default.
    #[test]
    fn the_draft_follows_the_persisted_switch() {
        assert!(!draft(&open(false)).jira.enabled);
        assert!(draft(&open(true)).jira.enabled);
        assert!(!SettingsDraft::default().jira.enabled);
    }

    /// US2 AS1: off, the page shows no status, only the switch.
    #[test]
    fn off_there_is_no_status_line() {
        assert_eq!(draft(&open(false)).jira.status(), None);
    }

    /// US2 AS3, FR-004: on, the status names the project state.
    #[test]
    fn on_the_status_says_what_the_open_project_has() {
        let mut state = open(false);
        update(&mut state, Msg::JiraEnabledToggled(true));
        assert_eq!(
            draft(&state).jira.status().as_deref(),
            Some("Open a project to connect it to Jira.")
        );

        let project = PathBuf::from("/work/app");
        let mut connections = BTreeMap::new();
        state.settings.settings_draft.as_mut().unwrap().jira.follow(
            Some(project.clone()),
            &connections,
            &BTreeMap::new(),
            false,
        );
        assert_eq!(
            draft(&state).jira.status().as_deref(),
            Some("Enabled. This project has no Jira connection yet.")
        );

        connections.insert(
            "/work/app".to_owned(),
            JiraConnection::parse("https://acme.atlassian.net", "abc", "me@acme.com").unwrap(),
        );
        state.settings.settings_draft.as_mut().unwrap().jira.follow(
            Some(project),
            &connections,
            &BTreeMap::new(),
            false,
        );
        let status = draft(&state).jira.status().unwrap();
        assert!(status.contains("https://acme.atlassian.net"), "{status}");
        assert!(status.contains("ABC"), "{status}");
    }

    /// Save writes the switch, and nothing else about Jira.
    #[test]
    fn save_writes_the_switch() {
        for chosen in [false, true] {
            let mut state = open(!chosen);
            update(&mut state, Msg::JiraEnabledToggled(chosen));
            let valid = state.settings.settings_draft.unwrap().validate().unwrap();
            assert_eq!(valid.jira_enabled, chosen);
            assert_eq!(valid.into_settings().jira_enabled, chosen);
        }
    }

    #[test]
    fn the_section_is_titled_jira() {
        assert_eq!(SettingsSection::Jira.label(), "Jira");
    }
}

// --- Feature 533 US3: connect, change and remove (T023) ----------------------------------------

mod jira_connection_form {
    use micold_client::app::State;
    use micold_client::features::settings::{
        update, KeepOrRemove, Msg, SettingsDraft, SettingsSection, TokenInput,
    };
    use micold_client::features::window::FieldId;
    use micold_core::jira::{ApiToken, ConnectionAction, JiraConnection};
    use micold_core::settings::Settings;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    const SENTINEL: &str = "sentinel-token-value-9f3a";

    fn connection() -> JiraConnection {
        JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap()
    }

    /// Settings open on `/work/app` with the switch `enabled`; a connection and token when asked.
    fn open(enabled: bool, stored: bool) -> State {
        let settings = Settings {
            jira_enabled: enabled,
            ..Settings::default()
        };
        let mut draft = SettingsDraft::from_settings(&settings);
        let mut connections = BTreeMap::new();
        if stored {
            connections.insert("/work/app".to_owned(), connection());
        }
        draft.jira.follow(
            Some(PathBuf::from("/work/app")),
            &connections,
            &BTreeMap::new(),
            stored,
        );
        let mut state = State::default();
        state.settings.settings_draft = Some(draft);
        state
    }

    fn draft(state: &State) -> &SettingsDraft {
        state.settings.settings_draft.as_ref().unwrap()
    }

    fn type_all(state: &mut State, site: &str, key: &str, email: &str, token: &str) {
        update(state, Msg::JiraSiteChanged(site.to_owned()));
        update(state, Msg::JiraKeyChanged(key.to_owned()));
        update(state, Msg::JiraEmailChanged(email.to_owned()));
        update(
            state,
            Msg::JiraTokenChanged(TokenInput::new(token.to_owned())),
        );
    }

    #[test]
    fn a_complete_form_saves_the_connection_and_the_new_token() {
        let mut state = open(true, false);
        type_all(
            &mut state,
            "https://acme.atlassian.net/",
            "abc",
            "me@acme.com",
            SENTINEL,
        );
        let valid = draft(&state).validate().unwrap();
        assert_eq!(
            valid.jira,
            ConnectionAction::Save {
                project: PathBuf::from("/work/app"),
                connection: connection(),
                token: Some(ApiToken::parse(SENTINEL).unwrap()),
            }
        );
    }

    /// US3 AS5: each refused field is marked, in the Jira section, and nothing is stored.
    #[test]
    fn a_refused_field_is_marked_and_nothing_is_saved() {
        let cases = [
            (
                "http://acme.atlassian.net",
                "ABC",
                "me@acme.com",
                SENTINEL,
                FieldId::SettingsJiraSite,
            ),
            (
                "https://acme.atlassian.net",
                "1",
                "me@acme.com",
                SENTINEL,
                FieldId::SettingsJiraKey,
            ),
            (
                "https://acme.atlassian.net",
                "ABC",
                "me",
                SENTINEL,
                FieldId::SettingsJiraEmail,
            ),
        ];
        for (site, key, email, token, field) in cases {
            let mut state = open(true, false);
            type_all(&mut state, site, key, email, token);
            let error = draft(&state)
                .validate()
                .unwrap_err()
                .into_field()
                .expect("a field error");
            assert_eq!(error.field, field, "{site} {key} {email}");
            assert_eq!(error.section, SettingsSection::Jira);
        }
    }

    /// A blank token keeps the stored one; the connection is unchanged, so nothing is saved.
    #[test]
    fn a_blank_token_keeps_the_stored_one() {
        let mut state = open(true, true);
        update(&mut state, Msg::JiraKeyChanged("xyz".to_owned()));
        let valid = draft(&state).validate().unwrap();
        let ConnectionAction::Save {
            token, connection, ..
        } = valid.jira
        else {
            panic!("a changed key is saved");
        };
        assert_eq!(token, None);
        assert_eq!(connection.project_key.as_str(), "XYZ");

        let untouched = open(true, true);
        assert_eq!(
            draft(&untouched).validate().unwrap().jira,
            ConnectionAction::Keep
        );
    }

    #[test]
    fn a_stored_connection_prefills_the_form_and_shows_a_token_is_stored() {
        let state = open(true, true);
        let jira = &draft(&state).jira;
        assert_eq!(jira.site, "https://acme.atlassian.net");
        assert_eq!(jira.project_key, "ABC");
        assert_eq!(jira.email, "me@acme.com");
        assert!(jira.token_stored);
        assert_eq!(jira.token.as_str(), "", "the token itself is never held");
    }

    /// US3 AS3, FR-010: Remove connection deletes on save.
    #[test]
    fn remove_connection_removes_on_save() {
        let mut state = open(true, true);
        update(&mut state, Msg::JiraRemoveRequested);
        assert!(draft(&state).jira.remove_requested);
        assert_eq!(
            draft(&state).validate().unwrap().jira,
            ConnectionAction::Remove {
                project: PathBuf::from("/work/app")
            }
        );
    }

    /// US2 AS5, US3 AS4: switching off over a stored connection asks, and applies nothing yet.
    #[test]
    fn switching_off_over_a_connection_raises_the_question() {
        let mut state = open(true, true);
        update(&mut state, Msg::JiraEnabledToggled(false));
        assert!(draft(&state).needs_keep_or_remove());
        let draft_mut = state.settings.settings_draft.as_mut().unwrap();
        draft_mut.raise_keep_or_remove();
        assert!(draft(&state).jira.prompt.is_some());
        assert_eq!(draft(&state).section, SettingsSection::Jira);
    }

    #[test]
    fn keep_leaves_it_stored_and_does_not_ask_again() {
        let mut state = open(true, true);
        update(&mut state, Msg::JiraEnabledToggled(false));
        state
            .settings
            .settings_draft
            .as_mut()
            .unwrap()
            .raise_keep_or_remove();
        update(&mut state, Msg::JiraKeepOrRemoveChosen(KeepOrRemove::Keep));
        assert!(!draft(&state).needs_keep_or_remove());
        assert_eq!(draft(&state).jira.prompt, None);
        assert_eq!(
            draft(&state).validate().unwrap().jira,
            ConnectionAction::Keep
        );
    }

    #[test]
    fn remove_deletes_it_with_the_switch_off() {
        let mut state = open(true, true);
        update(&mut state, Msg::JiraEnabledToggled(false));
        update(
            &mut state,
            Msg::JiraKeepOrRemoveChosen(KeepOrRemove::Remove),
        );
        assert!(!draft(&state).needs_keep_or_remove());
        let valid = draft(&state).validate().unwrap();
        assert!(!valid.jira_enabled);
        assert_eq!(
            valid.jira,
            ConnectionAction::Remove {
                project: PathBuf::from("/work/app")
            }
        );
    }

    #[test]
    fn with_no_connection_switching_off_asks_nothing() {
        let mut state = open(true, false);
        update(&mut state, Msg::JiraEnabledToggled(false));
        assert!(!draft(&state).needs_keep_or_remove());
    }

    /// FR-009: the typed token never shows in `Debug`.
    #[test]
    fn the_token_message_and_draft_redact_in_debug() {
        let mut state = open(true, false);
        type_all(
            &mut state,
            "https://acme.atlassian.net",
            "ABC",
            "me@acme.com",
            SENTINEL,
        );
        let message = Msg::JiraTokenChanged(TokenInput::new(SENTINEL.to_owned()));
        assert!(!format!("{message:?}").contains(SENTINEL));
        assert!(!format!("{:?}", draft(&state)).contains(SENTINEL));
        let valid = draft(&state).validate().unwrap();
        assert!(!format!("{valid:?}").contains(SENTINEL));
    }

    #[test]
    fn no_connection_is_required_while_the_form_is_empty() {
        let state = open(true, false);
        assert_eq!(
            draft(&state).validate().unwrap().jira,
            ConnectionAction::Keep
        );
    }
}

// --- Feature 1122 M5 (D4, R4): each project's Jira base URL (T042) ------------------------------

mod jira_base_url_form {
    use micold_client::app::State;
    use micold_client::features::settings::{
        update, KeepOrRemove, Msg, SettingsDraft, SettingsSection,
    };
    use micold_client::features::window::FieldId;
    use micold_core::jira::{
        connection_key, ConnectionAction, JiraBaseUrl, JiraConnection, BASE_URL_REFUSED,
    };
    use micold_core::settings::{JiraBaseUrlChange, Settings};
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    fn base(url: &str) -> JiraBaseUrl {
        JiraBaseUrl::parse(url).unwrap()
    }

    fn bases(entries: &[(&str, &str)]) -> BTreeMap<String, JiraBaseUrl> {
        entries
            .iter()
            .map(|(project, url)| (connection_key(Path::new(project)), base(url)))
            .collect()
    }

    /// Settings open on `project` with the switch on, the given base URLs and connections stored.
    fn open_with(
        project: &str,
        base_urls: &BTreeMap<String, JiraBaseUrl>,
        connections: &BTreeMap<String, JiraConnection>,
    ) -> State {
        let settings = Settings {
            jira_enabled: true,
            ..Settings::default()
        };
        let mut draft = SettingsDraft::from_settings(&settings);
        draft
            .jira
            .follow(Some(PathBuf::from(project)), connections, base_urls, false);
        let mut state = State::default();
        state.settings.settings_draft = Some(draft);
        state
    }

    fn open(project: &str, base_urls: &BTreeMap<String, JiraBaseUrl>) -> State {
        open_with(project, base_urls, &BTreeMap::new())
    }

    fn draft(state: &State) -> &SettingsDraft {
        state.settings.settings_draft.as_ref().unwrap()
    }

    fn follow(state: &mut State, project: &str, base_urls: &BTreeMap<String, JiraBaseUrl>) {
        state.settings.settings_draft.as_mut().unwrap().jira.follow(
            Some(PathBuf::from(project)),
            &BTreeMap::new(),
            base_urls,
            false,
        );
    }

    fn type_base(state: &mut State, text: &str) {
        update(state, Msg::JiraBaseUrlChanged(text.to_owned()));
    }

    /// R4: the field is seeded from the open project's entry and re-seeded on a project switch.
    #[test]
    fn follow_seeds_the_open_projects_value_and_reseeds_on_a_switch() {
        let stored = bases(&[
            ("/work/a", "https://a.atlassian.net"),
            ("/work/b", "https://b.atlassian.net"),
        ]);
        let mut state = open("/work/a", &stored);
        assert_eq!(draft(&state).jira.base_url, "https://a.atlassian.net");
        type_base(&mut state, "https://typed.example.com");
        follow(&mut state, "/work/b", &stored);
        assert_eq!(draft(&state).jira.base_url, "https://b.atlassian.net");
        follow(&mut state, "/work/c", &stored);
        assert_eq!(draft(&state).jira.base_url, "", "unset by default");
    }

    /// R4: a reported map refreshes an untouched field and keeps a typed one.
    #[test]
    fn a_changed_map_refreshes_an_untouched_field_and_keeps_a_typed_one() {
        let mut state = open("/work/a", &BTreeMap::new());
        follow(
            &mut state,
            "/work/a",
            &bases(&[("/work/a", "https://a.atlassian.net")]),
        );
        assert_eq!(draft(&state).jira.base_url, "https://a.atlassian.net");
        type_base(&mut state, "https://typed.example.com");
        follow(
            &mut state,
            "/work/a",
            &bases(&[("/work/a", "https://other.atlassian.net")]),
        );
        assert_eq!(draft(&state).jira.base_url, "https://typed.example.com");
    }

    /// R4 (review A M5 F2): typed text that parses to the stored value counts as untouched, so a
    /// saved `…/` keeps following other windows' changes.
    #[test]
    fn a_typed_trailing_slash_that_was_saved_still_follows_the_map() {
        let mut state = open("/work/a", &BTreeMap::new());
        type_base(&mut state, " https://a.atlassian.net/ ");
        let saved = bases(&[("/work/a", "https://a.atlassian.net")]);
        follow(&mut state, "/work/a", &saved);
        follow(
            &mut state,
            "/work/a",
            &bases(&[("/work/a", "https://other.atlassian.net")]),
        );
        assert_eq!(draft(&state).jira.base_url, "https://other.atlassian.net");
    }

    /// FR-010, FR-011: a valid value saves as that project's change, its trailing `/` removed,
    /// with the connection's fields blank and none of them marked; the connection is kept.
    #[test]
    fn a_valid_value_saves_with_no_connection() {
        let mut state = open("/work/a", &BTreeMap::new());
        type_base(&mut state, " https://jira.example.com/jira/ ");
        let valid = state
            .settings
            .settings_draft
            .as_mut()
            .unwrap()
            .validated_for_save()
            .expect("saves");
        assert_eq!(
            valid.jira_base_url,
            Some(JiraBaseUrlChange {
                project: connection_key(Path::new("/work/a")),
                url: Some(base("https://jira.example.com/jira")),
            })
        );
        assert_eq!(valid.jira, ConnectionAction::Keep);
        assert!(draft(&state)
            .required_fields(SettingsSection::Jira)
            .is_empty());
        assert_eq!(draft(&state).error, None);
    }

    /// FR-011: an empty field over a stored value clears it; an unchanged value sends nothing.
    #[test]
    fn empty_clears_and_unchanged_sends_nothing() {
        let stored = bases(&[("/work/a", "https://a.atlassian.net")]);
        let unchanged = open("/work/a", &stored);
        assert_eq!(draft(&unchanged).validate().unwrap().jira_base_url, None);
        let mut same = open("/work/a", &stored);
        type_base(&mut same, "https://a.atlassian.net/");
        assert_eq!(draft(&same).validate().unwrap().jira_base_url, None);
        let mut cleared = open("/work/a", &stored);
        type_base(&mut cleared, "  ");
        assert_eq!(
            draft(&cleared).validate().unwrap().jira_base_url,
            Some(JiraBaseUrlChange {
                project: connection_key(Path::new("/work/a")),
                url: None,
            })
        );
        let empty = open("/work/a", &BTreeMap::new());
        assert_eq!(draft(&empty).validate().unwrap().jira_base_url, None);
    }

    /// FR-011, SC-006: each refused value marks the field with the reason and saves nothing.
    #[test]
    fn each_refused_value_marks_the_field_and_saves_nothing() {
        for value in [
            "http://x",
            "https://u@x",
            "https://x?q",
            "https://x#f",
            "https://",
        ] {
            let mut state = open("/work/a", &BTreeMap::new());
            type_base(&mut state, value);
            let draft_mut = state.settings.settings_draft.as_mut().unwrap();
            assert!(draft_mut.validated_for_save().is_none(), "{value}");
            let error = draft(&state).error.clone().expect("a field error");
            assert_eq!(error.field, FieldId::SettingsJiraBaseUrl, "{value}");
            assert_eq!(error.section, SettingsSection::Jira, "{value}");
            assert_eq!(error.message, BASE_URL_REFUSED, "{value}");
            assert_eq!(draft(&state).section, SettingsSection::Jira);
        }
    }

    /// Spec edge case: the Keep-or-Remove answer `Remove` sends no base-URL change.
    #[test]
    fn the_remove_answer_sends_no_base_url_change() {
        let mut connections = BTreeMap::new();
        connections.insert(
            "/work/a".to_owned(),
            JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap(),
        );
        let mut state = open_with(
            "/work/a",
            &bases(&[("/work/a", "https://a.atlassian.net")]),
            &connections,
        );
        type_base(&mut state, "https://other.atlassian.net");
        update(&mut state, Msg::JiraEnabledToggled(false));
        update(
            &mut state,
            Msg::JiraKeepOrRemoveChosen(KeepOrRemove::Remove),
        );
        let valid = draft(&state).validate().unwrap();
        assert!(matches!(valid.jira, ConnectionAction::Remove { .. }));
        assert_eq!(valid.jira_base_url, None);
    }
}

/// Feature 937 (T016): the fields each section refuses empty, from `SettingsDraft::required_fields`.
mod required_fields {
    use super::*;
    use micold_client::features::settings::TokenInput;
    use micold_core::issue_types::LabelTypeEntry;
    use micold_core::naming::ConventionalType;
    use std::path::PathBuf;

    fn ids(draft: &SettingsDraft, section: SettingsSection) -> Vec<FieldId> {
        draft
            .required_fields(section)
            .into_iter()
            .filter(|(_, r)| r.required)
            .map(|(id, _)| id)
            .collect()
    }

    fn empty_ids(draft: &SettingsDraft, section: SettingsSection) -> Vec<FieldId> {
        draft
            .required_fields(section)
            .into_iter()
            .filter(|(_, r)| r.required && r.empty)
            .map(|(id, _)| id)
            .collect()
    }

    fn fresh() -> SettingsDraft {
        SettingsDraft::from_settings(&Settings::default())
    }

    #[test]
    fn appearance_and_daemons_list_nothing() {
        let draft = fresh();
        assert!(draft
            .required_fields(SettingsSection::Appearance)
            .is_empty());
        assert!(draft.required_fields(SettingsSection::Daemons).is_empty());
    }

    #[test]
    fn terminal_requires_the_scrollback_and_it_is_empty_exactly_when_validate_refuses_it() {
        let mut draft = fresh();
        assert_eq!(
            ids(&draft, SettingsSection::Terminal),
            vec![FieldId::SettingsScrollback]
        );
        assert!(empty_ids(&draft, SettingsSection::Terminal).is_empty());
        for blank in ["", "   "] {
            draft.terminal.scrollback_lines = blank.into();
            assert_eq!(
                empty_ids(&draft, SettingsSection::Terminal),
                vec![FieldId::SettingsScrollback]
            );
            assert_eq!(draft.validate(), Err(Refusal::Missing));
        }
    }

    #[test]
    fn environment_requires_timeout_threshold_and_warn_percent() {
        let mut draft = fresh();
        let mut listed = ids(&draft, SettingsSection::Environment);
        listed.sort_by_key(|id| format!("{id:?}"));
        let mut want = vec![
            FieldId::SettingsEnvIncludeTimeout,
            FieldId::SettingsLongTaskThreshold,
            FieldId::SettingsPlanUsageWarn,
        ];
        want.sort_by_key(|id| format!("{id:?}"));
        assert_eq!(listed, want);
        assert!(empty_ids(&draft, SettingsSection::Environment).is_empty());

        draft.environment.timeout_secs.clear();
        assert_eq!(
            empty_ids(&draft, SettingsSection::Environment),
            vec![FieldId::SettingsEnvIncludeTimeout]
        );
        assert_eq!(draft.validate(), Err(Refusal::Missing));
        draft.environment.timeout_secs = "5".into();

        draft.environment.long_task_threshold_secs = " ".into();
        assert_eq!(
            empty_ids(&draft, SettingsSection::Environment),
            vec![FieldId::SettingsLongTaskThreshold]
        );
        assert_eq!(draft.validate(), Err(Refusal::Missing));
        draft.environment.long_task_threshold_secs = "60".into();

        draft.environment.plan_usage_warn_percent.clear();
        assert_eq!(
            empty_ids(&draft, SettingsSection::Environment),
            vec![FieldId::SettingsPlanUsageWarn]
        );
        assert_eq!(draft.validate(), Err(Refusal::Missing));
    }

    #[test]
    fn two_empty_environment_fields_both_read_empty_at_once() {
        let mut draft = fresh();
        draft.environment.timeout_secs.clear();
        draft.environment.plan_usage_warn_percent.clear();
        assert_eq!(empty_ids(&draft, SettingsSection::Environment).len(), 2);
    }

    #[test]
    fn github_requires_one_label_per_entry_and_blank_is_what_validate_refuses() {
        let mut draft = fresh();
        let n = draft.github.entries.len();
        assert_eq!(
            ids(&draft, SettingsSection::GithubIssues),
            (0..n).map(FieldId::IssueMappingLabel).collect::<Vec<_>>()
        );
        assert!(empty_ids(&draft, SettingsSection::GithubIssues).is_empty());
        draft.github.entries.push(LabelTypeEntry {
            label: "  ".into(),
            type_: ConventionalType::Feat,
        });
        assert_eq!(
            empty_ids(&draft, SettingsSection::GithubIssues),
            vec![FieldId::IssueMappingLabel(n)]
        );
        assert_eq!(draft.validate(), Err(Refusal::Missing));
    }

    fn jira_draft() -> SettingsDraft {
        let mut draft = fresh();
        draft.jira.enabled = true;
        draft.jira.project = Some(PathBuf::from("/work/app"));
        draft
    }

    #[test]
    fn jira_lists_nothing_until_the_form_has_input_or_a_connection() {
        let mut draft = jira_draft();
        assert!(draft.required_fields(SettingsSection::Jira).is_empty());
        draft.jira.site = "https://acme.atlassian.net".into();
        assert_eq!(
            ids(&draft, SettingsSection::Jira),
            vec![
                FieldId::SettingsJiraSite,
                FieldId::SettingsJiraKey,
                FieldId::SettingsJiraEmail,
                FieldId::SettingsJiraToken,
            ]
        );
        draft.jira.enabled = false;
        assert!(draft.required_fields(SettingsSection::Jira).is_empty());
    }

    #[test]
    fn jira_token_is_required_only_while_none_is_stored() {
        let mut draft = jira_draft();
        draft.jira.site = "https://acme.atlassian.net".into();
        draft.jira.token_stored = true;
        assert!(!ids(&draft, SettingsSection::Jira).contains(&FieldId::SettingsJiraToken));
        draft.jira.token_stored = false;
        assert!(ids(&draft, SettingsSection::Jira).contains(&FieldId::SettingsJiraToken));
    }

    #[test]
    fn jira_fields_are_empty_exactly_when_validate_refuses_them() {
        let mut draft = jira_draft();
        draft.jira.site = "https://acme.atlassian.net".into();
        // Three fields empty at once: every one reads empty, validate names the first.
        assert_eq!(
            empty_ids(&draft, SettingsSection::Jira),
            vec![
                FieldId::SettingsJiraKey,
                FieldId::SettingsJiraEmail,
                FieldId::SettingsJiraToken,
            ]
        );
        assert_eq!(draft.validate(), Err(Refusal::Missing));
        draft.jira.project_key = "ABC".into();
        draft.jira.email = "a@b.co".into();
        assert_eq!(
            empty_ids(&draft, SettingsSection::Jira),
            vec![FieldId::SettingsJiraToken]
        );
        assert_eq!(draft.validate(), Err(Refusal::Missing));
        draft.jira.token = TokenInput::new("secret".into());
        assert!(empty_ids(&draft, SettingsSection::Jira).is_empty());
        assert!(draft.validate().is_ok());
    }

    #[test]
    fn removing_or_moving_a_mapping_entry_forgets_every_left_label() {
        use micold_client::app::State;
        use micold_client::features::settings::{update, Msg};
        use micold_core::required::FieldRequirement;
        for msg in [
            Msg::IssueMappingRemoved(0),
            Msg::IssueMappingMoved(1, micold_core::typeahead::Direction::Prev),
            Msg::IssueMappingDefaultsRestored,
        ] {
            let mut state = State::default();
            let mut draft = fresh();
            draft.github.entries[1].label.clear();
            draft.required.leave(FieldId::IssueMappingLabel(1));
            draft.required.leave(FieldId::SettingsScrollback);
            state.settings.settings_draft = Some(draft);
            let _ = update(&mut state, msg);
            let draft = state.settings.settings_draft.as_ref().unwrap();
            let r = micold_core::required::Requirement::text(true, "");
            assert_eq!(
                draft.required.state(FieldId::IssueMappingLabel(1), r),
                FieldRequirement::Required
            );
            assert_eq!(
                draft.required.state(FieldId::SettingsScrollback, r),
                FieldRequirement::Missing
            );
        }
    }
}

/// Save with a required field empty (feature 937, M5: FR-005, FR-006, FR-010, SC-004).
mod save_required {
    use super::{valid, Refusal, SettingsDraft, SettingsSection};
    use micold_client::features::window::FieldId;
    use micold_core::required::FieldRequirement;

    fn reads(draft: &SettingsDraft, section: SettingsSection, id: FieldId) -> FieldRequirement {
        draft.required.state_in(&draft.required_fields(section), id)
    }

    #[test]
    fn an_empty_scrollback_reads_missing_after_save_and_nothing_is_saved() {
        let mut draft = valid();
        draft.terminal.scrollback_lines.clear();
        let id = FieldId::SettingsScrollback;
        assert_eq!(
            reads(&draft, SettingsSection::Terminal, id),
            FieldRequirement::Required
        );
        assert!(draft.validated_for_save().is_none());
        assert_eq!(
            reads(&draft, SettingsSection::Terminal, id),
            FieldRequirement::Missing
        );
        assert!(draft.error.is_none(), "no sentence beside the field");
        assert_eq!(draft.section, SettingsSection::Terminal);
    }

    #[test]
    fn two_empty_environment_fields_both_read_missing_after_one_save() {
        let mut draft = valid();
        draft.environment.timeout_secs.clear();
        draft.environment.plan_usage_warn_percent.clear();
        assert_eq!(draft.validate(), Err(Refusal::Missing));
        assert!(draft.validated_for_save().is_none());
        for id in [
            FieldId::SettingsEnvIncludeTimeout,
            FieldId::SettingsPlanUsageWarn,
        ] {
            assert_eq!(
                reads(&draft, SettingsSection::Environment, id),
                FieldRequirement::Missing
            );
        }
        // The one that is filled in stays quiet.
        assert_eq!(
            reads(
                &draft,
                SettingsSection::Environment,
                FieldId::SettingsLongTaskThreshold
            ),
            FieldRequirement::Required
        );
    }

    #[test]
    fn empty_fields_in_two_sections_all_read_missing_after_one_save() {
        let mut draft = valid();
        draft.terminal.scrollback_lines.clear();
        draft.environment.long_task_threshold_secs.clear();
        assert!(draft.validated_for_save().is_none());
        assert_eq!(
            reads(
                &draft,
                SettingsSection::Terminal,
                FieldId::SettingsScrollback
            ),
            FieldRequirement::Missing
        );
        assert_eq!(
            reads(
                &draft,
                SettingsSection::Environment,
                FieldId::SettingsLongTaskThreshold
            ),
            FieldRequirement::Missing
        );
        assert_eq!(
            draft.section,
            SettingsSection::Terminal,
            "the first section in the rail"
        );
    }

    #[test]
    fn two_blank_mapping_labels_both_read_missing_after_one_save() {
        let mut draft = valid();
        draft.github.entries[0].label.clear();
        draft.github.entries[1].label = "  ".into();
        assert!(draft.validated_for_save().is_none());
        for i in 0..2 {
            assert_eq!(
                reads(
                    &draft,
                    SettingsSection::GithubIssues,
                    FieldId::IssueMappingLabel(i)
                ),
                FieldRequirement::Missing
            );
        }
    }

    #[test]
    fn the_empty_jira_fields_all_read_missing_after_one_save() {
        let mut draft = valid();
        draft.jira.enabled = true;
        draft.jira.project = Some(std::path::PathBuf::from("/work/app"));
        draft.jira.site = "https://acme.atlassian.net".into();
        assert!(draft.validated_for_save().is_none());
        for id in [
            FieldId::SettingsJiraKey,
            FieldId::SettingsJiraEmail,
            FieldId::SettingsJiraToken,
        ] {
            assert_eq!(
                reads(&draft, SettingsSection::Jira, id),
                FieldRequirement::Missing
            );
        }
        assert_eq!(
            reads(&draft, SettingsSection::Jira, FieldId::SettingsJiraSite),
            FieldRequirement::Required
        );
    }

    #[test]
    fn a_filled_in_invalid_value_keeps_its_own_error_beside_an_empty_one() {
        let mut draft = valid();
        draft.terminal.scrollback_lines = "abc".into();
        draft.environment.timeout_secs.clear();
        assert!(draft.validated_for_save().is_none());
        let error = draft.error.clone().expect("the scrollback keeps its error");
        assert_eq!(error.field, FieldId::SettingsScrollback);
        assert_eq!(error.message, "Enter a whole number of lines.");
        assert_eq!(
            reads(
                &draft,
                SettingsSection::Environment,
                FieldId::SettingsEnvIncludeTimeout
            ),
            FieldRequirement::Missing
        );
    }

    #[test]
    fn a_complete_draft_validates_and_marks_nothing() {
        let mut draft = valid();
        assert!(draft.validated_for_save().is_some());
        assert_eq!(draft.required, Default::default());
    }
}

/// Feature 938, FR-023: the note under **Save terminal history** says it covers structured
/// sessions' events too, and that turning it off deletes them.
#[test]
fn the_save_terminal_history_note_names_structured_sessions_events() {
    let note = micold_client::features::settings::SAVE_TERMINAL_HISTORY_NOTE;
    assert!(note.contains("structured sessions"), "{note}");
    assert!(note.contains("events"), "{note}");
    assert!(note.contains("deletes"), "{note}");
}

/// The *GitHub access tokens* section (#1002, contracts/pat-source.md §3). Every token is fake.
mod github_access_tokens {
    use micold_client::app::State;
    use micold_client::features::settings::github_tokens::{help, parse_repo};
    use micold_client::features::settings::{update, Msg, TokenInput};
    use micold_core::github::GithubRepo;

    const FAKE: &str = "github_pat_FAKE_0123456789";
    const T0: u64 = 1_800_000_000;
    const DAY: u64 = 86_400;

    fn repo(text: &str) -> GithubRepo {
        parse_repo(text).expect("a repository")
    }

    fn typed(state: &mut State, repository: &str, token: &str) {
        update(state, Msg::GithubPatRepoChanged(repository.into()));
        update(
            state,
            Msg::GithubPatTokenChanged(TokenInput::new(token.into())),
        );
    }

    #[test]
    fn a_row_per_stored_repository_with_its_expiry_date() {
        let mut state = State::default();
        update(
            &mut state,
            Msg::GithubPatsLoaded(vec![
                (repo("o/b"), T0 + 90 * DAY),
                (repo("o/a"), T0 + 60 * DAY),
            ]),
        );
        let rows = &state.settings.github_tokens.rows;
        assert_eq!(rows.len(), 2);
        assert!(
            rows[0].label().starts_with("o/a - expires on 2027-"),
            "{}",
            rows[0].label()
        );
        assert!(!format!("{rows:?}").contains(FAKE));
    }

    #[test]
    fn from_a_day_before_the_expiry_the_row_warns_and_before_that_it_does_not() {
        let mut state = State::default();
        update(
            &mut state,
            Msg::GithubPatsLoaded(vec![(repo("o/a"), T0 + 2 * DAY)]),
        );
        let row = &state.settings.github_tokens.rows[0];
        assert_eq!(row.warning(T0), None);
        let warning = row.warning(T0 + DAY + 1).expect("a warning");
        assert_eq!(
            warning,
            format!(
                "Git credential for o/a expires on {}: paste a new token in Settings",
                row.expiry_date()
            )
        );
        assert!(row
            .warning(T0 + 3 * DAY)
            .expect("expired")
            .contains("expired on"));
    }

    #[test]
    fn add_accepts_owner_name_or_a_github_url_and_refuses_anything_else_at_the_field() {
        for ok in [
            "o/r",
            "https://github.com/o/r",
            "https://github.com/o/r.git",
        ] {
            let mut state = State::default();
            typed(&mut state, ok, FAKE);
            update(&mut state, Msg::GithubPatAddRequested);
            let tokens = &mut state.settings.github_tokens;
            assert_eq!(tokens.error, None, "{ok}");
            assert_eq!(tokens.take_request().expect("raised").repo, repo("o/r"));
        }
        for bad in [
            "",
            "o",
            "https://gitlab.com/o/r",
            "o/r/x",
            "git@github.com:o/r",
        ] {
            let mut state = State::default();
            typed(&mut state, bad, FAKE);
            update(&mut state, Msg::GithubPatAddRequested);
            let tokens = &mut state.settings.github_tokens;
            assert!(tokens.error.is_some(), "{bad:?}");
            assert!(tokens.take_request().is_none(), "{bad:?}");
        }
    }

    #[test]
    fn a_token_that_is_not_fine_grained_is_refused_before_any_check() {
        let mut state = State::default();
        typed(&mut state, "o/r", "ghp_classic");
        update(&mut state, Msg::GithubPatAddRequested);
        assert!(state.settings.github_tokens.error.is_some());
        assert!(state.settings.github_tokens.take_request().is_none());
    }

    #[test]
    fn a_refused_check_is_shown_at_the_form_and_stores_nothing() {
        let mut state = State::default();
        typed(&mut state, "o/r", FAKE);
        update(&mut state, Msg::GithubPatAddRequested);
        update(
            &mut state,
            Msg::GithubPatChecked {
                repo: repo("o/r"),
                result: Err("GitHub refused the token: the token cannot push to o/r.".into()),
            },
        );
        let tokens = &mut state.settings.github_tokens;
        assert!(tokens.error.as_deref().unwrap().contains("cannot push"));
        assert!(tokens.rows.is_empty() && !tokens.checking);
        assert!(tokens.take_refresh().is_empty());
    }

    #[test]
    fn a_save_clears_the_token_field_and_asks_the_wiring_to_refresh() {
        let mut state = State::default();
        update(
            &mut state,
            Msg::GithubPatsLoaded(vec![(repo("o/r"), T0 + DAY)]),
        );
        update(&mut state, Msg::GithubPatReplaceRequested(repo("o/r")));
        assert_eq!(state.settings.github_tokens.repo, "o/r");
        typed(&mut state, "o/r", FAKE);
        update(&mut state, Msg::GithubPatAddRequested);
        update(
            &mut state,
            Msg::GithubPatChecked {
                repo: repo("o/r"),
                result: Ok(T0 + 30 * DAY),
            },
        );
        let tokens = &mut state.settings.github_tokens;
        assert_eq!(tokens.token.as_str(), "");
        assert_eq!(tokens.rows.len(), 1);
        assert_eq!(tokens.rows[0].expires_at, T0 + 30 * DAY);
        assert_eq!(tokens.take_refresh(), vec![repo("o/r")]);
        assert!(tokens.take_refresh().is_empty());
    }

    #[test]
    fn remove_drops_the_row_and_asks_the_wiring_to_refresh() {
        let mut state = State::default();
        update(
            &mut state,
            Msg::GithubPatsLoaded(vec![(repo("o/r"), T0 + DAY)]),
        );
        update(&mut state, Msg::GithubPatRemoveRequested(repo("o/r")));
        let tokens = &mut state.settings.github_tokens;
        assert!(tokens.rows.is_empty());
        assert_eq!(tokens.take_refresh(), vec![repo("o/r")]);
    }

    #[test]
    fn the_token_is_never_in_the_state_debug() {
        let mut state = State::default();
        typed(&mut state, "o/r", FAKE);
        update(&mut state, Msg::GithubPatAddRequested);
        assert!(!format!("{:?}", state.settings.github_tokens).contains(FAKE));
        assert!(!format!(
            "{:?}",
            Msg::GithubPatTokenChanged(TokenInput::new(FAKE.into()))
        )
        .contains(FAKE));
    }

    #[test]
    fn the_help_names_the_permissions_and_the_risks() {
        let text = help(true).join(" ");
        for needle in [
            "Contents: Read and write",
            "Metadata: Read",
            "short expiry",
            "everything the token can",
            "cannot narrow",
            "until it expires or you delete it on GitHub",
            "does not delete it on GitHub",
        ] {
            assert!(text.contains(needle), "{needle}");
        }
        assert!(!text.contains("other repositories"));
        assert!(help(false)
            .join(" ")
            .contains("cannot verify which other repositories"));
    }
}

/// Feature 1161, T011: the default prompt template is edited in a multi-line editor; the draft
/// opens with the stored text and a save carries the text the editor holds.
#[test]
fn the_default_prompt_template_opens_with_the_stored_text_and_saves_the_edited_one() {
    let stored = micold_core::settings::Settings {
        default_prompt_template: "/speckit-autopilot #{issue}\nthen {branch}".into(),
        ..micold_core::settings::Settings::default()
    };
    let mut draft = SettingsDraft::from_settings(&stored);
    assert_eq!(
        draft.environment.default_prompt_editor.content().text(),
        "/speckit-autopilot #{issue}\nthen {branch}"
    );

    draft
        .environment
        .set_default_prompt_template("Fix {issue_title}\n\nline".into());

    let saved = draft
        .validate()
        .expect("the stored numbers are in range")
        .into_settings();
    assert_eq!(saved.default_prompt_template, "Fix {issue_title}\n\nline");
}

/// Feature 1161, FR-001: no template is the empty string.
#[test]
fn an_empty_template_saves_as_empty() {
    let draft = SettingsDraft::from_settings(&micold_core::settings::Settings::default());
    let saved = draft.validate().unwrap().into_settings();
    assert_eq!(saved.default_prompt_template, "");
}

/// Feature 1161, US3: the default prompt template is checked on save, and the refusal names it.
mod default_prompt_template {
    use super::*;
    use micold_core::settings::Settings;

    const UNKNOWN: &str = "Unknown placeholder {issu}; valid: {issue}, {issue_title}, {issue_body}, {branch}, {type}, {name}";

    fn refusal(template: &str) -> micold_client::features::settings::FieldError {
        let mut draft = valid();
        draft.environment.default_prompt_template = template.into();
        draft
            .validate()
            .expect_err("the template is invalid")
            .into_field()
            .expect("a field error")
    }

    #[test]
    fn an_unknown_placeholder_is_refused_with_its_name_and_the_valid_list() {
        let error = refusal("Do {issu}");
        assert_eq!(error.message, UNKNOWN);
        assert_eq!(error.field, FieldId::SettingsDefaultPromptTemplate);
        assert_eq!(error.section, SettingsSection::Environment);
    }

    #[test]
    fn an_unbalanced_brace_is_refused() {
        let error = refusal("Do {issue");
        assert!(
            error.message.contains("Unbalanced brace"),
            "{}",
            error.message
        );
    }

    #[test]
    fn a_refused_save_stores_nothing() {
        let mut draft = valid();
        draft.environment.default_prompt_template = "Do {issu}".into();
        assert!(draft.validated_for_save().is_none());
        assert_eq!(
            draft.error.as_ref().map(|e| e.message.as_str()),
            Some(UNKNOWN)
        );
    }

    #[test]
    fn a_valid_template_passes_and_is_kept() {
        let mut draft = valid();
        draft.environment.default_prompt_template = "Fix #{issue} {{ok}}".into();
        let saved = draft.validate().expect("valid");
        assert_eq!(saved.default_prompt_template, "Fix #{issue} {{ok}}");
    }

    #[test]
    fn opening_with_an_invalid_stored_template_shows_the_error() {
        let settings = Settings {
            default_prompt_template: "Do {issu}".into(),
            ..Settings::default()
        };
        let draft = SettingsDraft::from_settings(&settings);
        let error = draft.error.expect("the error is shown on open");
        assert_eq!(error.message, UNKNOWN);
        assert_eq!(draft.section, SettingsSection::Environment);
        assert_eq!(
            draft.environment.default_prompt_template, "Do {issu}",
            "the text is kept"
        );
    }

    #[test]
    fn opening_with_a_valid_stored_template_shows_no_error() {
        let settings = Settings {
            default_prompt_template: "Do {issue}".into(),
            ..Settings::default()
        };
        assert!(SettingsDraft::from_settings(&settings).error.is_none());
    }
}
