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
    assert_eq!(
        names,
        vec!["Claude Code", "GitHub Copilot", "Pi Coding Agent"]
    );

    let commands: Vec<&str> = AiCli::ALL
        .into_iter()
        .map(|which| which.provider().command())
        .collect();
    assert_eq!(commands, vec!["claude", "copilot", "pi"]);
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
        update(state, Msg::ScriptPathCheckStarted { origin });
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

    #[test]
    fn a_relative_path_says_it_is_not_checked_then_011s_note() {
        for outcome in every_outcome() {
            let mut expected = vec![note(REL)];
            expected.extend(lines_011(&outcome, Some(P)));
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
            expected.extend(lines_011(&outcome, Some(P)));
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
                    // indication (SC-002).
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
