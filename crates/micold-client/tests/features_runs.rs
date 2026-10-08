//! The Run in parallel dialog and the group list, exercised in isolation (feature 483, T012).
//!
//! Names only `features::runs` and the domain types its API mentions.

use std::path::PathBuf;

use micold_client::features::runs::{
    update, Effect, Invalid, Msg, Opening, State, GROUP_MENU_ITEMS,
};
use micold_core::naming::{ConventionalType, NamingError};
use micold_core::protocol::messages::ClientMsg;
use micold_core::runs::{
    GroupId, NewGroup, Run, RunGroup, RunStatus, DEFAULT_RUNS, MAX_RUNS, MIN_RUNS,
};
use micold_core::session::AiCli;

fn opening() -> Opening {
    Opening {
        project: PathBuf::from("/p"),
        default_cli: AiCli::Copilot,
        offered: vec![AiCli::ClaudeCode, AiCli::Copilot],
        base_branch: Some("trunk".into()),
    }
}

fn opened() -> State {
    let mut state = State::default();
    let effect = update(&mut state, Msg::Opened(opening()));
    assert!(matches!(effect, Effect::Send(ClientMsg::BranchList { .. })));
    state
}

/// A dialog with every required field filled.
fn filled() -> State {
    let mut state = opened();
    update(&mut state, Msg::PromptChanged("add a login page".into()));
    update(&mut state, Msg::TypeChanged(ConventionalType::Feat));
    update(&mut state, Msg::NameChanged("login page".into()));
    state
}

fn dialog(state: &State) -> &micold_client::features::runs::ParallelDialog {
    state.dialog.as_ref().expect("the dialog is open")
}

#[test]
fn opening_gives_the_default_run_count_on_the_default_cli_and_the_roots_branch() {
    let state = opened();
    let d = dialog(&state);
    assert_eq!(d.runs, vec![AiCli::Copilot; DEFAULT_RUNS]);
    assert_eq!(d.base_branch, "trunk");
    assert_eq!(d.project, PathBuf::from("/p"));
    assert!(d.error.is_none(), "a fresh dialog does not scold");
}

#[test]
fn each_field_reducer_writes_its_field() {
    let mut state = filled();
    update(&mut state, Msg::TicketChanged("ABC-1".into()));
    update(&mut state, Msg::BaseBranchChanged("dev".into()));
    update(&mut state, Msg::ProviderChanged(1, AiCli::ClaudeCode));
    let d = dialog(&state);
    assert_eq!(d.prompt, "add a login page");
    assert_eq!(d.naming.type_, Some(ConventionalType::Feat));
    assert_eq!(d.naming.name, "login page");
    assert_eq!(d.naming.ticket.as_deref(), Some("ABC-1"));
    assert_eq!(d.base_branch, "dev");
    assert_eq!(d.runs, vec![AiCli::Copilot, AiCli::ClaudeCode]);
}

#[test]
fn runs_can_be_added_and_removed_between_the_minimum_and_the_maximum() {
    let mut state = opened();
    for _ in 0..20 {
        update(&mut state, Msg::RunAdded);
    }
    assert_eq!(dialog(&state).runs.len(), MAX_RUNS);
    assert_eq!(
        dialog(&state).runs[MAX_RUNS - 1],
        AiCli::Copilot,
        "a new run takes the default CLI"
    );
    for _ in 0..20 {
        update(&mut state, Msg::RunRemoved(0));
    }
    assert_eq!(dialog(&state).runs.len(), MIN_RUNS);
}

#[test]
fn the_derived_names_follow_the_name_and_the_count() {
    let mut state = filled();
    update(&mut state, Msg::RunAdded);
    let names = dialog(&state).derived_names().expect("valid");
    assert_eq!(names.len(), 3);
    assert_eq!(names[0].branch, "feat/login-page-1");
    assert_eq!(names[2].dir_name, "feat-login-page-3");
    update(&mut state, Msg::NameChanged("signup".into()));
    update(&mut state, Msg::RunRemoved(2));
    let names = dialog(&state).derived_names().expect("valid");
    assert_eq!(names.len(), 2);
    assert_eq!(names[1].branch, "feat/signup-2");
}

#[test]
fn validate_reports_the_first_failing_rule_in_the_documented_order() {
    let mut state = opened();
    assert_eq!(dialog(&state).validate(), Err(Invalid::EmptyPrompt));
    update(&mut state, Msg::PromptChanged("   ".into()));
    assert_eq!(
        dialog(&state).validate(),
        Err(Invalid::EmptyPrompt),
        "blank is empty"
    );
    update(&mut state, Msg::PromptChanged("do it".into()));
    assert_eq!(dialog(&state).validate(), Err(Invalid::NoType));
    update(&mut state, Msg::TypeChanged(ConventionalType::Fix));
    assert_eq!(dialog(&state).validate(), Err(Invalid::EmptyName));
    update(&mut state, Msg::NameChanged("x".into()));
    assert_eq!(dialog(&state).validate(), Ok(()));
    update(&mut state, Msg::NameChanged("!!!".into()));
    assert_eq!(
        dialog(&state).validate(),
        Err(Invalid::BadName(NamingError::EmptyNameAfterSlug))
    );
    update(&mut state, Msg::NameChanged("x".into()));
    update(&mut state, Msg::ProviderChanged(0, AiCli::Pi));
    assert_eq!(
        dialog(&state).validate(),
        Err(Invalid::ProviderNotOffered(AiCli::Pi))
    );
}

#[test]
fn a_refused_confirm_keeps_the_dialog_and_says_why() {
    let mut state = opened();
    let effect = update(&mut state, Msg::Confirmed);
    assert_eq!(effect, Effect::None);
    assert!(dialog(&state).error.is_some());
    assert_eq!(
        dialog(&state).error.as_deref(),
        Some(Invalid::EmptyPrompt.to_string().as_str())
    );
}

#[test]
fn a_valid_confirm_closes_the_dialog_and_sends_one_create_in_run_order() {
    let mut state = filled();
    update(&mut state, Msg::TicketChanged("  ".into()));
    update(&mut state, Msg::RunAdded);
    update(&mut state, Msg::ProviderChanged(0, AiCli::ClaudeCode));
    let effect = update(&mut state, Msg::Confirmed);
    assert!(state.dialog.is_none());
    match effect {
        Effect::Send(ClientMsg::RunGroupCreate {
            project,
            naming,
            prompt,
            base_branch,
            providers,
            ..
        }) => {
            assert_eq!(project, PathBuf::from("/p"));
            assert_eq!(naming.ticket, None, "a blank ticket is no ticket");
            assert_eq!(naming.name, "login page");
            assert_eq!(prompt, "add a login page");
            assert_eq!(base_branch, "trunk");
            assert_eq!(
                providers,
                vec![AiCli::ClaudeCode, AiCli::Copilot, AiCli::Copilot]
            );
        }
        other => panic!("expected RunGroupCreate, got {other:?}"),
    }
}

#[test]
fn dismissing_emits_nothing() {
    let mut state = filled();
    assert_eq!(update(&mut state, Msg::Dismissed), Effect::None);
    assert!(state.dialog.is_none());
}

#[test]
fn the_branch_listing_fills_the_select_and_defaults_the_base_when_none_is_set() {
    use micold_core::worktree::{BlockReason, BranchCandidate, BranchOrigin};
    let candidate = |name: &str, origin, blocked_by| BranchCandidate {
        name: name.into(),
        origin,
        blocked_by,
    };
    let mut state = State::default();
    update(
        &mut state,
        Msg::Opened(Opening {
            base_branch: None,
            ..opening()
        }),
    );
    update(
        &mut state,
        Msg::BranchesListed(vec![
            candidate("a", BranchOrigin::Local, None),
            candidate(
                "origin-only",
                BranchOrigin::Remote {
                    remote: "origin".into(),
                },
                None,
            ),
            candidate(
                "main",
                BranchOrigin::Local,
                Some(BlockReason::CheckedOutInProjectRoot),
            ),
        ]),
    );
    let d = dialog(&state);
    assert_eq!(
        d.branches,
        vec!["a".to_string(), "main".to_string()],
        "local branches only"
    );
    assert_eq!(d.base_branch, "main", "the project root's current branch");
}

fn group(id: u128, name: &str) -> RunGroup {
    RunGroup::new(NewGroup {
        id: GroupId(uuid::Uuid::from_u128(id)),
        name: name.into(),
        naming: micold_core::naming::WorktreeNaming {
            type_: Some(ConventionalType::Feat),
            ticket: None,
            name: name.into(),
        },
        prompt: "p".into(),
        base_branch: "main".into(),
        base_commit: "abc".into(),
        created: std::time::SystemTime::UNIX_EPOCH,
        runs: (1..=2u8)
            .map(|number| Run {
                number,
                provider: AiCli::ClaudeCode,
                names: micold_core::naming::DerivedNames {
                    dir_name: format!("feat-{name}-{number}"),
                    branch: format!("feat/{name}-{number}"),
                },
                session: None,
                status: RunStatus::Creating,
            })
            .collect(),
    })
    .expect("valid group")
}

#[test]
fn groups_changed_replaces_the_list() {
    let mut state = State::default();
    update(
        &mut state,
        Msg::GroupsChanged(vec![group(1, "a"), group(2, "b")]),
    );
    assert_eq!(state.groups.len(), 2);
    update(&mut state, Msg::GroupsChanged(vec![group(2, "b")]));
    assert_eq!(state.groups.len(), 1);
    assert_eq!(state.groups[0].name, "b");
}

#[test]
fn a_group_can_be_collapsed_and_expanded_again() {
    let mut state = State::default();
    update(&mut state, Msg::GroupsChanged(vec![group(1, "a")]));
    let id = state.groups[0].id;
    assert!(state.is_expanded(id), "groups start expanded");
    update(&mut state, Msg::GroupToggled(id));
    assert!(!state.is_expanded(id));
    update(&mut state, Msg::GroupToggled(id));
    assert!(state.is_expanded(id));
}

fn with_group() -> (State, GroupId) {
    let mut state = State::default();
    update(&mut state, Msg::GroupsChanged(vec![group(1, "a")]));
    let id = state.groups[0].id;
    (state, id)
}

#[test]
fn the_group_rows_menu_is_dismiss_group_only() {
    assert_eq!(GROUP_MENU_ITEMS, ["Dismiss group"]);
}

#[test]
fn the_group_menu_opens_at_the_press_point_and_toggles_shut() {
    let (mut state, id) = with_group();
    update(&mut state, Msg::MenuToggled(id, (10, 20)));
    let menu = state.menu.as_ref().expect("open");
    assert_eq!((menu.group, menu.anchor), (id, (10, 20)));
    update(&mut state, Msg::MenuToggled(id, (10, 20)));
    assert!(state.menu.is_none());
    update(&mut state, Msg::MenuToggled(id, (1, 2)));
    update(&mut state, Msg::MenuDismissed);
    assert!(state.menu.is_none());
}

#[test]
fn dismiss_group_asks_first_and_sends_nothing_yet() {
    let (mut state, id) = with_group();
    update(&mut state, Msg::MenuToggled(id, (1, 2)));
    let effect = update(
        &mut state,
        Msg::DismissAsked {
            group: id,
            project: PathBuf::from("/p"),
        },
    );
    assert_eq!(effect, Effect::None);
    assert!(state.menu.is_none(), "the pick closes the menu");
    assert_eq!(state.dismiss_target.as_ref().map(|t| t.group), Some(id));
}

#[test]
fn the_confirmation_names_that_worktrees_branches_and_sessions_stay() {
    let text = micold_client::features::runs::DISMISS_CONFIRMATION;
    for word in ["worktrees", "branches", "sessions"] {
        assert!(text.contains(word), "{text}");
    }
    assert!(text.contains("stay"), "{text}");
}

#[test]
fn confirming_sends_exactly_one_dismiss_and_closes_the_dialog() {
    let (mut state, id) = with_group();
    update(
        &mut state,
        Msg::DismissAsked {
            group: id,
            project: PathBuf::from("/p"),
        },
    );
    let effect = update(&mut state, Msg::DismissConfirmed);
    assert_eq!(
        effect,
        Effect::Send(ClientMsg::RunGroupDismiss {
            req: 0,
            project: PathBuf::from("/p"),
            group: id
        })
    );
    assert!(state.dismiss_target.is_none());
    assert_eq!(
        update(&mut state, Msg::DismissConfirmed),
        Effect::None,
        "a second confirm has nothing to send"
    );
}

#[test]
fn cancelling_emits_nothing_and_keeps_the_group() {
    let (mut state, id) = with_group();
    update(
        &mut state,
        Msg::DismissAsked {
            group: id,
            project: PathBuf::from("/p"),
        },
    );
    assert_eq!(update(&mut state, Msg::DismissCancelled), Effect::None);
    assert!(state.dismiss_target.is_none());
    assert_eq!(state.groups.len(), 1);
}

#[test]
fn a_group_that_goes_away_takes_its_menu_and_confirmation_with_it() {
    let (mut state, id) = with_group();
    update(&mut state, Msg::MenuToggled(id, (1, 2)));
    update(
        &mut state,
        Msg::DismissAsked {
            group: id,
            project: PathBuf::from("/p"),
        },
    );
    update(&mut state, Msg::GroupsChanged(vec![]));
    assert!(state.menu.is_none() && state.dismiss_target.is_none());
}
