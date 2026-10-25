//! The API token never leaves its file (feature 533, FR-009, SC-004, US3 AS6): with a sentinel
//! token, a save and a removal run through the shell operations, and the settings document, the
//! wire frames, the `Debug` of the draft, its messages and the validated form, and the error
//! text are scanned for the sentinel. Later milestones extend it with load and each failure.

#[path = "../src/shell/jira.rs"]
#[allow(dead_code)]
mod jira;

use micold_client::features::settings::{update, Msg, SettingsDraft, TokenInput};
use micold_core::jira::ConnectionAction;
use micold_core::protocol::messages::ClientMsg;
use micold_core::settings::Settings;
use micold_core::store::jira_token_path;
use std::collections::BTreeMap;
use std::path::PathBuf;

const SENTINEL: &str = "sentinel-token-value-9f3a";

fn typed_draft() -> micold_client::app::State {
    let mut state = micold_client::app::State::default();
    let mut draft = SettingsDraft::from_settings(&Settings {
        jira_enabled: true,
        ..Settings::default()
    });
    draft.jira.follow(
        Some(PathBuf::from("/work/app")),
        &BTreeMap::new(),
        &BTreeMap::new(),
        false,
    );
    state.settings.settings_draft = Some(draft);
    for msg in [
        Msg::JiraSiteChanged("https://acme.atlassian.net".to_owned()),
        Msg::JiraKeyChanged("ABC".to_owned()),
        Msg::JiraEmailChanged("me@acme.com".to_owned()),
        Msg::JiraTokenChanged(TokenInput::new(SENTINEL.to_owned())),
    ] {
        assert!(
            !format!("{msg:?}").contains(SENTINEL),
            "a message holding the token redacts"
        );
        update(&mut state, msg);
    }
    state
}

#[test]
fn save_and_remove_keep_the_token_in_its_file_alone() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("jira");
    let settings_file = root.path().join("settings.json");

    let state = typed_draft();
    let draft = state.settings.settings_draft.as_ref().unwrap();
    assert!(!format!("{draft:?}").contains(SENTINEL), "draft Debug");
    let valid = draft.validate().unwrap();
    assert!(!format!("{valid:?}").contains(SENTINEL), "validated Debug");
    let action = valid.jira.clone();
    assert!(matches!(action, ConnectionAction::Save { .. }));

    // Save: the token file holds it; nothing else the change touches does.
    let change = jira::apply(Some(&dir), &action).unwrap().unwrap();
    let mut settings = valid.into_settings();
    change.apply(&mut settings.jira_connections);
    let document = serde_json::to_string(&settings).unwrap();
    std::fs::write(&settings_file, &document).unwrap();
    assert!(!std::fs::read_to_string(&settings_file)
        .unwrap()
        .contains(SENTINEL));

    let frame = ClientMsg::SettingsSet {
        req: 1,
        scrollback_lines: None,
        env_include_enabled: None,
        env_include_script_path: None,
        env_include_timeout_secs: None,
        default_ai_cli: None,
        default_prompt_template: None,
        pi_activity_component: None,
        save_terminal_history: None,
        tool_server_enabled: None,
        worktree_guard_enabled: None,
        desktop_notifications: None,
        diff_layout: None,
        cross_session_access: None,
        pr_status_enabled: None,
        jira_enabled: Some(true),
        jira_connection: Some(change),
        github_issues_enabled: None,
        jira_base_url: None,
        notification_kinds: None,
        long_task_threshold_secs: None,
        plan_usage_enabled: None,
        plan_usage_warn_percent: None,
    };
    let bytes = serde_json::to_string(&frame).unwrap();
    assert!(!bytes.contains(SENTINEL), "wire frame");
    assert!(!format!("{frame:?}").contains(SENTINEL), "frame Debug");

    let file = jira_token_path(&dir, std::path::Path::new("/work/app"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), SENTINEL);

    // Remove: the file is gone and the change carries no token either.
    let removal = jira::apply(
        Some(&dir),
        &ConnectionAction::Remove {
            project: PathBuf::from("/work/app"),
        },
    )
    .unwrap()
    .unwrap();
    assert!(!file.exists());
    assert!(!format!("{removal:?}").contains(SENTINEL));
}

#[test]
fn a_failed_token_write_does_not_echo_the_token() {
    let root = tempfile::tempdir().unwrap();
    let blocker = root.path().join("jira");
    std::fs::write(&blocker, b"in the way").unwrap();
    let state = typed_draft();
    let valid = state.settings.settings_draft.unwrap().validate().unwrap();
    let error = jira::apply(Some(&blocker), &valid.jira).unwrap_err();
    assert!(!error.contains(SENTINEL), "{error}");
}

/// Feature 1185 (FR-006): a reporter choice's name and account print in no `Debug` of the form,
/// of the module state or of a filter message, as the own user, a searched person or the pick.
#[test]
fn no_reporter_name_or_account_prints_in_the_form_or_its_messages() {
    use micold_client::app::Message;
    use micold_client::features::worktree_form::jira_filters::FilterMsg;
    use micold_client::features::worktree_form::{BranchSource, Msg as FormMsg};
    use micold_core::github::IssueListing;
    use micold_core::jira::{connection_key, JiraConnection};
    use micold_core::jira_filter::{AccountId, FilterKind, JiraUser};

    const NAME: &str = "Reporter Sentinel Name";
    const ACCOUNT: &str = "account-sentinel-5b10ac";
    let user = |account: &str, is_me| JiraUser {
        account: AccountId::parse(account).unwrap(),
        name: NAME.to_owned(),
        is_me,
    };
    let project = PathBuf::from("/work/app");
    let mut state = micold_client::app::State::default();
    state.workspace.active = Some(project.clone());
    state.settings.jira_enabled = true;
    state.settings.jira_connections.insert(
        connection_key(&project),
        JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap(),
    );
    let assert_clean = |what: &str, debug: String| {
        assert!(
            !debug.contains(NAME) && !debug.contains(ACCOUNT) && !debug.contains("Sentinel"),
            "{what} prints a reporter: {debug}"
        );
    };
    let send = |state: &mut micold_client::app::State, msg: FormMsg| {
        assert_clean("a message", format!("{msg:?}"));
        state.update(Message::WorktreeForm(msg));
    };
    send(&mut state, FormMsg::Opened);
    send(&mut state, FormMsg::RemotesListed(Ok(Vec::new())));
    send(&mut state, FormMsg::SourceChanged(BranchSource::Issue));
    let seq = state.worktree_form.awaited_issue_load().unwrap();
    send(
        &mut state,
        FormMsg::IssuesLoaded {
            seq,
            result: Ok((
                IssueListing {
                    issues: Vec::new(),
                    total_open: 0,
                    complete: true,
                },
                PathBuf::new(),
            )),
            resolved_env: None,
        },
    );
    send(&mut state, FormMsg::JiraFilter(FilterMsg::PanelToggled));
    send(
        &mut state,
        FormMsg::JiraFilter(FilterMsg::Focused(FilterKind::Reporter)),
    );
    let seq = state.worktree_form.awaited_choices().unwrap().1;
    send(
        &mut state,
        FormMsg::JiraFilter(FilterMsg::MeLoaded {
            seq,
            result: Ok(Some(user("me-account-sentinel", true))),
        }),
    );
    send(
        &mut state,
        FormMsg::JiraFilter(FilterMsg::QueryChanged(FilterKind::Reporter, "Rep".into())),
    );
    let seq = state.worktree_form.pending_people_search().unwrap();
    send(
        &mut state,
        FormMsg::JiraFilter(FilterMsg::PeopleSearchDue { seq }),
    );
    send(
        &mut state,
        FormMsg::JiraFilter(FilterMsg::PeopleLoaded {
            seq,
            result: Ok(vec![user(ACCOUNT, false)]),
        }),
    );
    send(
        &mut state,
        FormMsg::JiraFilter(FilterMsg::Picked(FilterKind::Reporter, 2)),
    );
    let form = state.worktree_form.form.as_ref().unwrap();
    assert_eq!(
        form.jira_filters
            .chosen
            .reporter()
            .map(|u| u.account.as_str()),
        Some(ACCOUNT),
        "the searched person is chosen"
    );
    assert_clean("the form", format!("{form:?}"));
    assert_clean("the module state", format!("{:?}", state.worktree_form));
}
