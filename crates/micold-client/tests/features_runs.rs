//! The Run in parallel dialog and the group list, exercised in isolation (feature 483, T012).
//!
//! Names only `features::runs` and the domain types its API mentions.

use std::path::PathBuf;

use micold_client::features::changes::Load;
use micold_client::features::runs::{
    compare_rows, open_diff, status_text, update, Effect, Invalid, Msg, Opening, RunCounts, State,
    SummaryRead, GROUP_MENU_ITEMS,
};
use micold_client::features::Outcome;
use micold_core::naming::{ConventionalType, NamingError};
use micold_core::protocol::messages::ActivitySignal;
use micold_core::protocol::messages::ClientMsg;
use micold_core::runs::{
    GroupId, NewGroup, Run, RunGroup, RunStatus, DEFAULT_RUNS, MAX_RUNS, MIN_RUNS,
};
use micold_core::runs::{RunStep, RunSummary};
use micold_core::session::AiCli;
use micold_core::session::SessionLocation;

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
fn the_group_rows_menu_holds_compare_and_dismiss_group() {
    assert_eq!(GROUP_MENU_ITEMS, ["Compare", "Dismiss group"]);
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

// ---- Compare (feature 483, T042, contracts/parallel-surfaces.md C1–C5) ----

fn summary(files: u32, added: u32, removed: u32) -> RunSummary {
    RunSummary {
        files,
        added,
        removed,
        uncommitted: false,
    }
}

/// A state whose group 1 has run 1 prompted, run 2 failed creating its worktree and run 3 failed
/// starting its session.
fn compare_ready() -> (State, GroupId) {
    let mut g = group(1, "a");
    g.runs[0].status = RunStatus::Prompted;
    g.runs[1].status = RunStatus::Failed {
        step: RunStep::Worktree,
        reason: "the branch exists".into(),
    };
    let mut third = g.runs[0].clone();
    third.number = 3;
    third.names.dir_name = "feat-a-3".into();
    third.status = RunStatus::Failed {
        step: RunStep::Session,
        reason: "no cli".into(),
    };
    g.runs.push(third);
    let id = g.id;
    let mut state = State::default();
    update(&mut state, Msg::GroupsChanged(vec![g]));
    (state, id)
}

fn open_compare(state: &mut State, id: GroupId) -> Vec<SummaryRead> {
    let effect = update(
        state,
        Msg::CompareOpened {
            group: id,
            project: PathBuf::from("/p"),
        },
    );
    match effect {
        Effect::ReadSummaries(reads) => reads,
        other => panic!("expected reads, got {other:?}"),
    }
}

#[test]
fn opening_compare_asks_for_one_read_per_run_with_a_worktree_and_reads_nothing_itself() {
    let (mut state, id) = compare_ready();
    let reads = open_compare(&mut state, id);
    assert_eq!(
        reads.iter().map(|r| r.run).collect::<Vec<_>>(),
        vec![1, 3],
        "run 2 never got a worktree"
    );
    assert_eq!(reads[0].dir_name, "feat-a-1");
    assert_eq!(reads[0].base_branch, "main");
    assert_ne!(reads[0].seq, reads[1].seq);
    let rows = compare_rows(&state, &[]).expect("open").1;
    assert_eq!(rows[0].counts, RunCounts::Loading, "nothing read yet");
    assert_eq!(rows[1].counts, RunCounts::None);
}

#[test]
fn an_answer_shows_and_a_stale_answer_is_dropped() {
    let (mut state, id) = compare_ready();
    let reads = open_compare(&mut state, id);
    let seq = reads[0].seq;
    let effect = update(
        &mut state,
        Msg::SummaryRead {
            seq: seq + 100,
            run: 1,
            result: Ok(summary(9, 9, 9)),
        },
    );
    assert_eq!(effect, Effect::None);
    assert_eq!(
        compare_rows(&state, &[]).unwrap().1[0].counts,
        RunCounts::Loading
    );
    update(
        &mut state,
        Msg::SummaryRead {
            seq,
            run: 1,
            result: Ok(summary(4, 120, 30)),
        },
    );
    assert_eq!(
        compare_rows(&state, &[]).unwrap().1[0].counts,
        RunCounts::Ready(summary(4, 120, 30))
    );
}

#[test]
fn changes_while_a_read_runs_coalesce_into_one_more_read() {
    let (mut state, id) = compare_ready();
    let seq = open_compare(&mut state, id)[0].seq;
    for _ in 0..5 {
        assert_eq!(update(&mut state, Msg::RunChanged { run: 1 }), Effect::None);
    }
    let Effect::ReadSummaries(again) = update(
        &mut state,
        Msg::SummaryRead {
            seq,
            run: 1,
            result: Ok(summary(1, 1, 1)),
        },
    ) else {
        panic!("one more read follows");
    };
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].run, 1);
    assert_ne!(again[0].seq, seq);
}

#[test]
fn a_change_re_reads_only_that_run_and_keeps_the_old_counts_meanwhile() {
    let (mut state, id) = compare_ready();
    let reads = open_compare(&mut state, id);
    for read in &reads {
        update(
            &mut state,
            Msg::SummaryRead {
                seq: read.seq,
                run: read.run,
                result: Ok(summary(2, 3, 4)),
            },
        );
    }
    let Effect::ReadSummaries(reread) = update(&mut state, Msg::RunChanged { run: 1 }) else {
        panic!("a read");
    };
    assert_eq!(reread.iter().map(|r| r.run).collect::<Vec<_>>(), vec![1]);
    assert_eq!(
        compare_rows(&state, &[]).unwrap().1[0].counts,
        RunCounts::Ready(summary(2, 3, 4)),
        "the old counts stay on screen"
    );
    assert_eq!(
        update(&mut state, Msg::RunChanged { run: 2 }),
        Effect::None,
        "run 2 has no worktree"
    );
}

#[test]
fn status_text_follows_the_run_and_the_sessions_activity() {
    let prompted = RunStatus::Prompted;
    assert_eq!(status_text(&RunStatus::Creating, None), "Creating");
    assert_eq!(status_text(&RunStatus::Starting, None), "Starting");
    assert_eq!(
        status_text(&prompted, Some(&ActivitySignal::Working)),
        "Working"
    );
    assert_eq!(
        status_text(&prompted, Some(&ActivitySignal::AwaitingInput)),
        "Waiting for input"
    );
    assert_ne!(
        status_text(&prompted, Some(&ActivitySignal::Unknown)),
        "Waiting for input",
        "unknown is never shown as waiting (A1)"
    );
    let undelivered = RunStatus::PromptNotDelivered { reason: "x".into() };
    assert_eq!(status_text(&undelivered, None), "Prompt not delivered");
    let failed = RunStatus::Failed {
        step: RunStep::Session,
        reason: "x".into(),
    };
    assert_eq!(status_text(&failed, None), "Failed");
    assert_eq!(status_text(&RunStatus::Picked, None), "Picked");
}

#[test]
fn a_failed_run_shows_its_reason_and_has_no_diff_without_a_worktree() {
    let (mut state, id) = compare_ready();
    open_compare(&mut state, id);
    let rows = compare_rows(&state, &[]).unwrap().1;
    assert_eq!(rows[1].reason.as_deref(), Some("the branch exists"));
    assert_eq!(rows[1].counts, RunCounts::None);
    assert!(!rows[1].can_open_diff, "no worktree, no Open diff");
    assert_eq!(rows[2].reason.as_deref(), Some("no cli"));
    assert!(rows[2].can_open_diff, "its worktree was created");
    assert!(open_diff(&state, 2).is_empty());
}

#[test]
fn open_diff_asks_for_that_runs_changes_view() {
    let (mut state, id) = compare_ready();
    open_compare(&mut state, id);
    assert_eq!(
        open_diff(&state, 1),
        vec![Outcome::ChangesRequested(SessionLocation::Worktree(
            "feat-a-1".into()
        ))]
    );
}

#[test]
fn a_run_with_uncommitted_changes_carries_the_tag() {
    let (mut state, id) = compare_ready();
    let reads = open_compare(&mut state, id);
    update(
        &mut state,
        Msg::SummaryRead {
            seq: reads[0].seq,
            run: 1,
            result: Ok(RunSummary {
                uncommitted: true,
                ..summary(1, 1, 0)
            }),
        },
    );
    let RunCounts::Ready(got) = compare_rows(&state, &[]).unwrap().1[0].counts.clone() else {
        panic!("ready");
    };
    assert!(got.uncommitted);
}

#[test]
fn a_run_that_gains_a_worktree_is_read_and_a_vanished_group_closes_compare() {
    let mut g = group(1, "a");
    g.runs[0].status = RunStatus::Creating;
    g.runs[1].status = RunStatus::Creating;
    let id = g.id;
    let mut state = State::default();
    update(&mut state, Msg::GroupsChanged(vec![g.clone()]));
    let effect = update(
        &mut state,
        Msg::CompareOpened {
            group: id,
            project: PathBuf::from("/p"),
        },
    );
    assert_eq!(effect, Effect::None, "nothing to read while creating");
    g.runs[0].status = RunStatus::Starting;
    let Effect::ReadSummaries(reads) = update(&mut state, Msg::GroupsChanged(vec![g])) else {
        panic!("the run now has a worktree");
    };
    assert_eq!(reads.iter().map(|r| r.run).collect::<Vec<_>>(), vec![1]);
    update(&mut state, Msg::GroupsChanged(Vec::new()));
    assert!(state.compare.is_none(), "its group is gone");
}

#[test]
fn closing_compare_ends_its_reads() {
    let (mut state, id) = compare_ready();
    let seq = open_compare(&mut state, id)[0].seq;
    update(&mut state, Msg::CompareClosed);
    assert!(state.compare.is_none());
    assert_eq!(
        update(
            &mut state,
            Msg::SummaryRead {
                seq,
                run: 1,
                result: Ok(summary(1, 1, 1))
            }
        ),
        Effect::None
    );
    assert_eq!(update(&mut state, Msg::RunChanged { run: 1 }), Effect::None);
    assert!(compare_rows(&state, &[]).is_none());
    let _ = Load::<RunSummary>::Idle;
}

#[test]
fn compare_opens_from_the_menu_and_closes_it() {
    let (mut state, id) = compare_ready();
    update(&mut state, Msg::MenuToggled(id, (1, 2)));
    open_compare(&mut state, id);
    assert!(state.menu.is_none());
    assert_eq!(state.compare.as_ref().map(|c| c.group), Some(id));
}

// ---- Pick this one (T054, C6-C8) ----

use micold_client::features::runs::{
    picked_text, PickAvailability, PICK_BUSY_REASON, PICK_WORKING_CONFIRMATION,
};
use micold_core::runs::Integration;

/// Compare open on a group whose runs have the given statuses.
fn pick_ready(statuses: &[RunStatus]) -> (State, GroupId) {
    let mut g = group(1, "a");
    g.runs = statuses
        .iter()
        .enumerate()
        .map(|(i, status)| {
            let mut run = g.runs[0].clone();
            run.number = i as u8 + 1;
            run.names.dir_name = format!("feat-a-{}", i + 1);
            run.status = status.clone();
            run
        })
        .collect();
    let id = g.id;
    let mut state = State::default();
    update(&mut state, Msg::GroupsChanged(vec![g]));
    open_compare(&mut state, id);
    (state, id)
}

fn pick_of(state: &State, index: usize) -> PickAvailability {
    compare_rows(state, &[]).unwrap().1[index].pick.clone()
}

fn failed() -> RunStatus {
    RunStatus::Failed {
        step: RunStep::Session,
        reason: "x".into(),
    }
}

fn pick_msg(id: GroupId, run: u8) -> ClientMsg {
    ClientMsg::RunGroupPick {
        req: 0,
        project: PathBuf::from("/p"),
        group: id,
        run,
    }
}

#[test]
fn pick_is_absent_for_a_failed_run_and_offered_for_the_others() {
    let (state, _) = pick_ready(&[RunStatus::Prompted, failed()]);
    assert_eq!(
        pick_of(&state, 0),
        PickAvailability::Enabled { confirm: false }
    );
    assert_eq!(pick_of(&state, 1), PickAvailability::Hidden);
}

#[test]
fn pick_is_disabled_on_every_row_with_the_reason_while_a_run_is_creating_or_starting() {
    for busy in [RunStatus::Creating, RunStatus::Starting] {
        let (state, _) = pick_ready(&[RunStatus::Prompted, busy]);
        assert_eq!(
            pick_of(&state, 0),
            PickAvailability::Disabled(PICK_BUSY_REASON)
        );
        assert_eq!(
            pick_of(&state, 1),
            PickAvailability::Disabled(PICK_BUSY_REASON)
        );
    }
    assert!(!PICK_BUSY_REASON.is_empty());
}

#[test]
fn after_a_pick_no_row_offers_it_and_the_winner_reads_picked() {
    let (mut state, id) = pick_ready(&[RunStatus::Prompted, RunStatus::Prompted]);
    let mut g = state.groups[0].clone();
    g.winner = Some(1);
    g.runs[0].status = RunStatus::Picked;
    update(&mut state, Msg::GroupsChanged(vec![g]));
    assert_eq!(state.groups[0].id, id);
    let rows = compare_rows(&state, &[]).unwrap().1;
    assert_eq!(rows[0].status, "Picked");
    assert!(rows.iter().all(|r| r.pick == PickAvailability::Hidden));
}

#[test]
fn a_working_session_makes_the_row_ask_for_confirmation() {
    let (mut state, _) = pick_ready(&[RunStatus::Prompted]);
    let sid = micold_core::session::SessionId(uuid::Uuid::from_u128(7));
    state.groups[0].runs[0].session = Some(sid);
    let mut s = micold_core::session::Session::start_new(
        SessionLocation::Worktree("feat-a-1".into()),
        AiCli::ClaudeCode,
    );
    s.id = sid;
    s.activity = ActivitySignal::Working;
    let rows = compare_rows(&state, &[s]).unwrap().1;
    assert_eq!(rows[0].pick, PickAvailability::Enabled { confirm: true });
}

#[test]
fn picking_a_run_that_is_not_working_sends_at_once() {
    let (mut state, id) = pick_ready(&[RunStatus::Prompted, RunStatus::Prompted]);
    let effect = update(
        &mut state,
        Msg::PickPressed {
            run: 2,
            working: false,
        },
    );
    assert_eq!(effect, Effect::Send(pick_msg(id, 2)));
    assert!(state.pick_target.is_none());
}

#[test]
fn picking_a_working_run_asks_first_and_cancelling_sends_nothing() {
    let (mut state, _) = pick_ready(&[RunStatus::Prompted]);
    let effect = update(
        &mut state,
        Msg::PickPressed {
            run: 1,
            working: true,
        },
    );
    assert_eq!(effect, Effect::None);
    assert_eq!(state.pick_target.as_ref().map(|t| t.run), Some(1));
    assert!(PICK_WORKING_CONFIRMATION.contains("still working"));
    assert_eq!(update(&mut state, Msg::PickCancelled), Effect::None);
    assert!(state.pick_target.is_none());
    assert_eq!(update(&mut state, Msg::PickConfirmed), Effect::None);
}

#[test]
fn confirming_sends_exactly_one_pick() {
    let (mut state, id) = pick_ready(&[RunStatus::Prompted]);
    update(
        &mut state,
        Msg::PickPressed {
            run: 1,
            working: true,
        },
    );
    assert_eq!(
        update(&mut state, Msg::PickConfirmed),
        Effect::Send(pick_msg(id, 1))
    );
    assert_eq!(update(&mut state, Msg::PickConfirmed), Effect::None);
}

#[test]
fn a_pick_that_is_not_offered_sends_nothing() {
    let (mut state, _) = pick_ready(&[failed(), RunStatus::Creating]);
    for run in [1, 2] {
        assert_eq!(
            update(
                &mut state,
                Msg::PickPressed {
                    run,
                    working: false
                }
            ),
            Effect::None
        );
    }
}

#[test]
fn a_winner_arriving_while_the_confirmation_is_open_closes_it() {
    let (mut state, _) = pick_ready(&[RunStatus::Prompted, RunStatus::Prompted]);
    update(
        &mut state,
        Msg::PickPressed {
            run: 1,
            working: true,
        },
    );
    let mut g = state.groups[0].clone();
    g.winner = Some(2);
    update(&mut state, Msg::GroupsChanged(vec![g]));
    assert!(state.pick_target.is_none());
}

#[test]
fn success_names_the_base_branch_and_a_refusal_changes_nothing_in_the_view() {
    let ff = Integration::FastForward {
        base_tip: "t".into(),
    };
    let mc = Integration::MergeCommit { commit: "c".into() };
    assert_eq!(picked_text(2, "main", &ff), "Run 2 fast-forwarded main");
    assert_eq!(picked_text(2, "main", &mc), "Run 2 was merged into main");
    // A refusal arrives outside the reducer (the shell reports its text); the rows stay as they were.
    let (state, _) = pick_ready(&[RunStatus::Prompted]);
    let before = compare_rows(&state, &[]).unwrap().1;
    assert_eq!(compare_rows(&state, &[]).unwrap().1, before);
}

// ---- Clean up the losers (T062, K1-K6) ----

use micold_client::features::runs::{CleanupOffer, SummaryRead as Read, Uncommitted};
use std::collections::BTreeMap;

fn dirty() -> RunSummary {
    RunSummary {
        uncommitted: true,
        ..summary(1, 1, 0)
    }
}

/// A group of three prompted runs, run 2 picked: the offer is open with its three reads pending.
fn offer_ready() -> (State, GroupId, Vec<Read>) {
    let (mut state, id) = pick_ready(&vec![RunStatus::Prompted; 3]);
    let sessions = BTreeMap::from([(1, 2), (3, 1)]);
    let effect = update(
        &mut state,
        Msg::Picked {
            group: id,
            project: PathBuf::from("/p"),
            run: 2,
            integration: Integration::MergeCommit { commit: "c".into() },
            sessions,
        },
    );
    let Effect::ReadCleanup(reads) = effect else {
        panic!("expected one read per loser, got {effect:?}");
    };
    (state, id, reads)
}

fn offer(state: &State) -> &CleanupOffer {
    state.cleanup.as_ref().expect("the offer is open")
}

fn answer(state: &mut State, read: &Read, result: Result<RunSummary, String>) -> Effect {
    update(
        state,
        Msg::CleanupRead {
            seq: read.seq,
            run: read.run,
            result,
        },
    )
}

fn deletes(effect: Effect) -> Vec<(String, bool)> {
    let Effect::SendEach(msgs) = effect else {
        panic!("expected deletes, got {effect:?}");
    };
    msgs.into_iter()
        .map(|m| match m {
            ClientMsg::WorktreeDelete {
                dir_name,
                stop_sessions: true,
                delete_branch,
                ..
            } => (dir_name, delete_branch),
            other => panic!("expected only worktree deletes, got {other:?}"),
        })
        .collect()
}

#[test]
fn a_pick_opens_the_offer_once_with_a_row_and_a_fresh_read_per_loser() {
    let (mut state, id, reads) = offer_ready();
    let o = offer(&state);
    assert_eq!(o.heading, "Run 2 was merged into main");
    let numbers: Vec<u8> = o.losers.iter().map(|l| l.number).collect();
    assert_eq!(numbers, [1, 3]);
    assert_eq!(o.losers[0].sessions, 2);
    assert_eq!(o.losers[1].sessions, 1);
    assert!(o.losers.iter().all(|l| l.delete_branch));
    assert_eq!(reads.iter().map(|r| r.run).collect::<Vec<_>>(), [1, 3]);
    assert!(reads.iter().all(|r| r.base_branch == "main"));
    // Dismissing it emits nothing and it does not reopen.
    assert_eq!(update(&mut state, Msg::CleanupDismissed), Effect::None);
    assert!(state.cleanup.is_none());
    let again = update(
        &mut state,
        Msg::Picked {
            group: id,
            project: PathBuf::from("/p"),
            run: 2,
            integration: Integration::MergeCommit { commit: "c".into() },
            sessions: BTreeMap::new(),
        },
    );
    assert_eq!(again, Effect::None);
    assert!(state.cleanup.is_none());
}

#[test]
fn a_fast_forward_pick_says_so() {
    let (mut state, id) = pick_ready(&vec![RunStatus::Prompted; 2]);
    update(
        &mut state,
        Msg::Picked {
            group: id,
            project: PathBuf::from("/p"),
            run: 1,
            integration: Integration::FastForward {
                base_tip: "t".into(),
            },
            sessions: BTreeMap::new(),
        },
    );
    assert_eq!(offer(&state).heading, "Run 1 fast-forwarded main");
}

#[test]
fn a_loser_whose_read_is_pending_failed_or_uncommitted_starts_unselected_and_is_not_removable() {
    let (mut state, _, reads) = offer_ready();
    // Pending: unselected, unknown, not removable (even when the user ticks it).
    assert!(offer(&state).losers.iter().all(|l| !l.selected));
    assert!(offer(&state)
        .losers
        .iter()
        .all(|l| l.uncommitted == Uncommitted::Unknown));
    update(&mut state, Msg::CleanupToggled(1));
    assert!(offer(&state).removable().is_empty());
    // Clean: selected and removable.
    answer(&mut state, &reads[0], Ok(summary(1, 1, 0)));
    assert_eq!(offer(&state).removable(), [1]);
    // Uncommitted: unselected, tagged, not removable even when ticked.
    answer(&mut state, &reads[1], Ok(dirty()));
    assert_eq!(
        offer(&state).losers[1].uncommitted,
        Uncommitted::Uncommitted
    );
    assert!(!offer(&state).losers[1].selected);
    update(&mut state, Msg::CleanupToggled(3));
    assert_eq!(offer(&state).removable(), [1]);
}

#[test]
fn a_failed_read_counts_as_uncommitted() {
    let (mut state, _, reads) = offer_ready();
    answer(&mut state, &reads[1], Err("no git".into()));
    assert_eq!(offer(&state).losers[1].uncommitted, Uncommitted::Unknown);
    assert!(!offer(&state).losers[1].selected);
    update(&mut state, Msg::CleanupToggled(3));
    assert!(!offer(&state).removable().contains(&3));
}

#[test]
fn confirming_reads_the_selected_again_and_sends_nothing_until_they_answer() {
    let (mut state, _, reads) = offer_ready();
    answer(&mut state, &reads[0], Ok(summary(1, 1, 0)));
    answer(&mut state, &reads[1], Ok(summary(1, 1, 0)));
    let Effect::ReadCleanup(fresh) = update(&mut state, Msg::CleanupConfirmed) else {
        panic!("confirm must read first");
    };
    assert_eq!(fresh.iter().map(|r| r.run).collect::<Vec<_>>(), [1, 3]);
    assert!(fresh.iter().all(|f| reads.iter().all(|r| r.seq != f.seq)));
    assert_eq!(
        answer(&mut state, &fresh[0], Ok(summary(1, 1, 0))),
        Effect::None
    );
    // A stale answer changes nothing.
    assert_eq!(answer(&mut state, &reads[1], Ok(dirty())), Effect::None);
    let sent = deletes(answer(&mut state, &fresh[1], Ok(summary(1, 1, 0))));
    assert_eq!(
        sent,
        [
            ("feat-a-1".to_string(), true),
            ("feat-a-3".to_string(), true)
        ]
    );
    assert!(state.cleanup.is_none());
}

#[test]
fn keeping_the_branch_is_passed_through_and_an_unselected_loser_is_left_alone() {
    let (mut state, _, reads) = offer_ready();
    answer(&mut state, &reads[0], Ok(summary(1, 1, 0)));
    answer(&mut state, &reads[1], Ok(summary(1, 1, 0)));
    update(&mut state, Msg::CleanupToggled(3));
    update(&mut state, Msg::CleanupBranchToggled(1));
    let Effect::ReadCleanup(fresh) = update(&mut state, Msg::CleanupConfirmed) else {
        panic!("confirm must read first");
    };
    assert_eq!(fresh.len(), 1);
    let sent = deletes(answer(&mut state, &fresh[0], Ok(summary(1, 1, 0))));
    assert_eq!(sent, [("feat-a-1".to_string(), false)]);
}

#[test]
fn a_loser_that_turned_uncommitted_needs_the_second_confirmation_and_declining_removes_the_rest() {
    let (mut state, _, reads) = offer_ready();
    answer(&mut state, &reads[0], Ok(summary(1, 1, 0)));
    answer(&mut state, &reads[1], Ok(summary(1, 1, 0)));
    let Effect::ReadCleanup(fresh) = update(&mut state, Msg::CleanupConfirmed) else {
        panic!("confirm must read first");
    };
    answer(&mut state, &fresh[0], Ok(summary(1, 1, 0)));
    assert_eq!(answer(&mut state, &fresh[1], Ok(dirty())), Effect::None);
    assert_eq!(offer(&state).confirming, Some(vec![3]));
    assert!(!offer(&state).removable().contains(&3));
    let sent = deletes(update(&mut state, Msg::CleanupSecondDeclined));
    assert_eq!(sent, [("feat-a-1".to_string(), true)]);
    assert!(state.cleanup.is_none());
}

#[test]
fn a_selected_uncommitted_loser_is_removed_only_after_the_second_confirmation() {
    let (mut state, _, reads) = offer_ready();
    answer(&mut state, &reads[0], Ok(summary(1, 1, 0)));
    answer(&mut state, &reads[1], Ok(dirty()));
    update(&mut state, Msg::CleanupToggled(3));
    let Effect::ReadCleanup(fresh) = update(&mut state, Msg::CleanupConfirmed) else {
        panic!("confirm must read first");
    };
    answer(&mut state, &fresh[0], Ok(summary(1, 1, 0)));
    assert_eq!(answer(&mut state, &fresh[1], Ok(dirty())), Effect::None);
    assert_eq!(offer(&state).confirming, Some(vec![3]));
    // Escape (dismissing) at the second question removes nothing at all.
    let mut backed = state.clone();
    assert_eq!(update(&mut backed, Msg::CleanupDismissed), Effect::None);
    assert!(backed.cleanup.is_none());
    let sent = deletes(update(&mut state, Msg::CleanupSecondConfirmed));
    assert_eq!(
        sent,
        [
            ("feat-a-1".to_string(), true),
            ("feat-a-3".to_string(), true)
        ]
    );
}

#[test]
fn confirming_with_nothing_selected_closes_without_sending() {
    let (mut state, _, reads) = offer_ready();
    answer(&mut state, &reads[0], Ok(dirty()));
    answer(&mut state, &reads[1], Ok(dirty()));
    assert_eq!(update(&mut state, Msg::CleanupConfirmed), Effect::None);
    assert!(state.cleanup.is_none());
}

#[test]
fn the_choice_is_frozen_while_confirm_re_reads_and_a_late_read_keeps_the_users_untick() {
    let (mut state, _, reads) = offer_ready();
    // Untick run 1 before its read arrives: the clean answer does not tick it again.
    update(&mut state, Msg::CleanupToggled(1));
    update(&mut state, Msg::CleanupToggled(1));
    answer(&mut state, &reads[0], Ok(summary(1, 1, 0)));
    answer(&mut state, &reads[1], Ok(summary(1, 1, 0)));
    update(&mut state, Msg::CleanupToggled(1));
    update(&mut state, Msg::CleanupToggled(1));
    assert_eq!(offer(&state).removable(), [3]);
    let Effect::ReadCleanup(fresh) = update(&mut state, Msg::CleanupConfirmed) else {
        panic!("confirm must read first");
    };
    // Ticking the unselected run 1 now would delete it on a stale read: it is ignored.
    update(&mut state, Msg::CleanupToggled(1));
    update(&mut state, Msg::CleanupBranchToggled(3));
    assert!(!offer(&state).losers[0].selected);
    assert!(offer(&state).losers[1].delete_branch);
    let sent = deletes(answer(&mut state, &fresh[0], Ok(summary(1, 1, 0))));
    assert_eq!(sent, [("feat-a-3".to_string(), true)]);
}
