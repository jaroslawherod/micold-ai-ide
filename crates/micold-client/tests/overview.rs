//! Feature 813 (T010): the Overview's client state — open flag, and the cards built from what the
//! window holds.

use micold_client::app::{Message, State};
use micold_client::features::agent_confirm::Prompt;
use micold_client::features::overview::Msg;
use micold_client::features::pr_status::Msg as PrMsg;
use micold_client::ui::overview::cards;
use micold_core::local_time::LocalTime;
use micold_core::overview::{CardReason, CardState, LocationType, PrCounts, PrDisplay};
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::ActivitySignal;
use micold_core::protocol::messages::ConfirmOperation;
use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};
use micold_core::session::{AiCli, Session, SessionLifecycle, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

fn project_path() -> PathBuf {
    PathBuf::from("/repo")
}

fn worktree(dir: &str, branch: Option<&str>, status: WorktreeStatus, included: bool) -> Worktree {
    Worktree {
        dir_name: dir.to_string(),
        path: PathBuf::from(format!("/repo/.claude/worktrees/{dir}")),
        branch: branch.map(str::to_string),
        status,
        included,
    }
}

fn project_with(worktrees: Vec<Worktree>) -> State {
    let mut state = State::default();
    let path = project_path();
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    state.worktree.worktrees = worktrees;
    state
}

fn add_session(state: &mut State, location: SessionLocation) -> Session {
    let mut session = Session::start_new(location, AiCli::ClaudeCode);
    session.lifecycle = SessionLifecycle::Idle;
    state
        .workspace
        .sessions
        .entry(project_path())
        .or_default()
        .push(session.clone());
    session
}

fn running(state: &mut State, location: SessionLocation) -> Session {
    let mut session = Session::start_new(location, AiCli::ClaudeCode);
    session.lifecycle = SessionLifecycle::Running;
    session.activity = ActivitySignal::Working;
    state
        .workspace
        .sessions
        .entry(project_path())
        .or_default()
        .push(session.clone());
    session
}

fn wt(dir: &str) -> SessionLocation {
    SessionLocation::Worktree(dir.to_string())
}

fn status(state: PrState, review: ReviewState) -> PullRequestStatus {
    PullRequestStatus {
        number: 7,
        title: "t".to_string(),
        url: "https://github.com/o/r/pull/7".to_string(),
        state,
        review,
        head: "abc".to_string(),
        conflict: false,
    }
}

fn read(state: &mut State, branch: &str, pr: PullRequestStatus, removable: bool) {
    state.pr_status.enabled = true;
    state.pr_status.read_at = Some(100);
    state.pr_status.statuses.insert(branch.to_string(), pr);
    if removable {
        state.pr_status.removable.insert(branch.to_string());
    }
}

fn card<'a>(
    cards: &'a [micold_core::overview::CardModel],
    location: &SessionLocation,
) -> &'a micold_core::overview::CardModel {
    cards.iter().find(|c| &c.location == location).unwrap()
}

fn prompt(session: &Session) -> Prompt {
    Prompt {
        id: 1,
        project: project_path(),
        caller_label: "planner".to_string(),
        session: session.id,
        operation: ConfirmOperation::DeleteSession,
        target_label: "x".to_string(),
    }
}

#[test]
fn toggled_opens_then_closes_and_closed_closes() {
    let mut state = State::default();
    assert!(!state.overview.open);
    state.update(Message::Overview(Msg::Toggled {
        settings_open: false,
    }));
    assert!(state.overview.open);
    state.update(Message::Overview(Msg::Toggled {
        settings_open: false,
    }));
    assert!(!state.overview.open);
    state.update(Message::Overview(Msg::Toggled {
        settings_open: false,
    }));
    state.update(Message::Overview(Msg::Closed));
    assert!(!state.overview.open);
}

#[test]
fn toggled_while_settings_is_open_changes_nothing() {
    let mut state = State::default();
    state.update(Message::Overview(Msg::Toggled {
        settings_open: true,
    }));
    assert!(!state.overview.open);
    state.overview.open = true;
    state.update(Message::Overview(Msg::Toggled {
        settings_open: true,
    }));
    assert!(state.overview.open);
}

#[test]
fn a_project_switch_keeps_open_and_the_cards_follow_the_project() {
    let mut state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    state.overview.open = true;
    assert_eq!(cards(&state).len(), 2);
    let other = PathBuf::from("/other");
    state.workspace.projects.push(Project {
        path: other.clone(),
        display_name: "other".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(other);
    state.worktree.worktrees = Vec::new();
    assert!(state.overview.open);
    assert_eq!(
        cards(&state).len(),
        1,
        "an empty project is the Default card alone"
    );
}

#[test]
fn no_project_no_cards() {
    assert!(cards(&State::default()).is_empty());
}

#[test]
fn one_card_per_sidebar_entry_in_sidebar_order_with_every_kind() {
    let state = project_with(vec![
        worktree("feat-a", Some("feat/a"), WorktreeStatus::Valid, false),
        worktree("ext", Some("fix/ext"), WorktreeStatus::Valid, true),
        worktree("gone", None, WorktreeStatus::Missing, false),
        worktree("fix-b", Some("fix/b"), WorktreeStatus::Valid, false),
    ]);
    let entries = state.sidebar_entries().len();
    let all = cards(&state);
    assert_eq!(all.len(), entries);
    assert_eq!(all.len(), 5);
    assert_eq!(all[0].location, SessionLocation::Default);
    assert_eq!(all[0].kind, LocationType::ProjectRoot);
    assert_eq!(card(&all, &wt("ext")).kind, LocationType::Included);
    assert_eq!(card(&all, &wt("gone")).kind, LocationType::OrphanInvalid);
    assert_eq!(card(&all, &wt("feat-a")).kind, LocationType::Feature);
    assert_eq!(card(&all, &wt("fix-b")).kind, LocationType::Bugfix);
}

#[test]
fn cards_are_in_state_order_with_sidebar_order_inside_a_state() {
    let mut state = project_with(vec![
        worktree("feat-a", Some("feat/a"), WorktreeStatus::Valid, false),
        worktree("feat-b", Some("feat/b"), WorktreeStatus::Valid, false),
    ]);
    running(&mut state, wt("feat-b"));
    let all = cards(&state);
    // Working sorts before Idle; Default and feat-a stay in sidebar order.
    let order: Vec<_> = all.iter().map(|c| c.location.clone()).collect();
    assert_eq!(
        order,
        vec![wt("feat-b"), SessionLocation::Default, wt("feat-a")]
    );
}

#[test]
fn counts_are_total_and_running_unarchived_sessions() {
    let mut state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    running(&mut state, wt("feat-a"));
    add_session(&mut state, wt("feat-a"));
    let mut archived = Session::start_new(wt("feat-a"), AiCli::ClaudeCode);
    archived.archived = true;
    state
        .workspace
        .sessions
        .get_mut(&project_path())
        .unwrap()
        .push(archived);
    let all = cards(&state);
    assert_eq!(card(&all, &wt("feat-a")).sessions, (2, 1));
    assert_eq!(card(&all, &SessionLocation::Default).sessions, (0, 0));
}

#[test]
fn a_queued_agent_prompt_needs_action_on_its_sessions_location() {
    let mut state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    let session = add_session(&mut state, wt("feat-a"));
    state.agent_confirm.pending.push(prompt(&session));
    let all = cards(&state);
    let c = card(&all, &wt("feat-a"));
    assert_eq!(
        (c.state, c.reason),
        (CardState::NeedsAction, CardReason::AgentRequest)
    );
    assert_eq!(card(&all, &SessionLocation::Default).state, CardState::Idle);
}

#[test]
fn a_failing_pull_request_needs_attention_and_a_merged_removable_one_is_done() {
    let mut state = project_with(vec![
        worktree("feat-a", Some("feat/a"), WorktreeStatus::Valid, false),
        worktree("feat-b", Some("feat/b"), WorktreeStatus::Valid, false),
    ]);
    read(
        &mut state,
        "feat/a",
        status(
            PrState::Open {
                checks: CheckStatus::Failing,
            },
            ReviewState::None,
        ),
        false,
    );
    read(
        &mut state,
        "feat/b",
        status(PrState::Merged, ReviewState::None),
        true,
    );
    let all = cards(&state);
    let a = card(&all, &wt("feat-a"));
    assert_eq!(
        (a.state, a.reason),
        (CardState::NeedsAttention, CardReason::ChecksFailing)
    );
    let b = card(&all, &wt("feat-b"));
    assert_eq!((b.state, b.reason), (CardState::Done, CardReason::Merged));
}

#[test]
fn the_pull_request_signal_is_unknown_when_off_or_not_read() {
    let mut state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    read(
        &mut state,
        "feat/a",
        status(
            PrState::Open {
                checks: CheckStatus::Failing,
            },
            ReviewState::None,
        ),
        false,
    );
    state.pr_status.enabled = false;
    assert_eq!(card(&cards(&state), &wt("feat-a")).state, CardState::Idle);
    state.pr_status.enabled = true;
    state.pr_status.read_at = None;
    assert_eq!(card(&cards(&state), &wt("feat-a")).state, CardState::Idle);
}

#[test]
fn the_default_card_is_never_pull_request_derived() {
    let mut state = project_with(Vec::new());
    read(
        &mut state,
        "main",
        status(
            PrState::Open {
                checks: CheckStatus::Failing,
            },
            ReviewState::None,
        ),
        false,
    );
    let all = cards(&state);
    assert_eq!(all[0].state, CardState::Idle);
    assert_eq!(all[0].branch, None);
}

#[test]
fn the_summary_is_the_branch_else_the_display_name_and_default_is_project_root() {
    let state = project_with(vec![
        worktree("feat-a", Some("feat/a"), WorktreeStatus::Valid, false),
        worktree("gone", None, WorktreeStatus::Missing, false),
    ]);
    let all = cards(&state);
    assert_eq!(
        card(&all, &SessionLocation::Default).summary,
        "Project root"
    );
    assert_eq!(card(&all, &wt("feat-a")).summary, "feat/a");
    let gone = card(&all, &wt("gone"));
    assert_eq!(gone.summary, gone.display_name);
}

#[test]
fn after_a_disconnect_session_states_stay_and_pull_request_ones_are_not_claimed() {
    let mut state = project_with(vec![
        worktree("feat-a", Some("feat/a"), WorktreeStatus::Valid, false),
        worktree("feat-b", Some("feat/b"), WorktreeStatus::Valid, false),
    ]);
    let session = add_session(&mut state, wt("feat-a"));
    state.agent_confirm.pending.push(prompt(&session));
    read(
        &mut state,
        "feat/b",
        status(
            PrState::Open {
                checks: CheckStatus::Failing,
            },
            ReviewState::None,
        ),
        false,
    );
    assert_eq!(
        card(&cards(&state), &wt("feat-b")).state,
        CardState::NeedsAttention
    );
    // What `shell::daemon_sync::on_disconnected` does to this state: the reading is released.
    let _ = state.update_pr_status(PrMsg::Released);
    let all = cards(&state);
    assert_eq!(card(&all, &wt("feat-a")).state, CardState::NeedsAction);
    assert_eq!(card(&all, &wt("feat-b")).state, CardState::Idle);
}

#[test]
fn an_open_overview_takes_the_keyboard_and_the_main_area() {
    let mut state = project_with(Vec::new());
    let session = add_session(&mut state, SessionLocation::Default);
    state.session.active = Some(session.id);
    assert!(state.terminal_focused());
    state.overview.open = true;
    assert!(!state.terminal_focused());
    let facts = state.view_facts(micold_client::app::WindowFacts {
        focused: true,
        holds_project: true,
    });
    assert!(facts.main_area_taken);
}

#[test]
fn building_thirty_locations_with_three_sessions_each_is_fast() {
    let worktrees: Vec<_> = (0..30)
        .map(|i| {
            worktree(
                &format!("feat-{i}"),
                Some(&format!("feat/{i}")),
                WorktreeStatus::Valid,
                false,
            )
        })
        .collect();
    let mut state = project_with(worktrees);
    for i in 0..30 {
        for _ in 0..3 {
            add_session(&mut state, wt(&format!("feat-{i}")));
        }
    }
    let started = std::time::Instant::now();
    assert_eq!(cards(&state).len(), 31);
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
}

/// FR-023: starting a session hands the main area to its terminal, so the Overview closes.
#[test]
fn starting_a_session_closes_the_overview() {
    let mut state = project_with(Vec::new());
    state.overview.open = true;
    let session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    state.update(Message::Session(
        micold_client::features::session::Msg::Started(session),
    ));
    assert!(!state.overview.open);
}

/// FR-023: opening the Changes view hands the main area to it, so the Overview closes.
#[test]
fn opening_changes_closes_the_overview() {
    let mut state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    state.overview.open = true;
    state.update(Message::Sidebar(
        micold_client::features::sidebar::Msg::ReviewChangesRequested(wt("feat-a")),
    ));
    assert!(!state.overview.open);
}

// ---- Feature 813 M2 (T026): pull request counts and the unknown mark ----

fn counts(open: u8, merged: u8) -> PrCounts {
    PrCounts {
        open,
        merged,
        open_capped: false,
        merged_capped: false,
    }
}

fn read_counts(state: &mut State, branch: &str, counts: PrCounts) {
    state.pr_status.enabled = true;
    state.pr_status.read_at = Some(100);
    state.pr_status.counts.insert(branch.to_string(), counts);
}

fn feat_a() -> State {
    project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )])
}

fn prs(state: &State) -> PrDisplay {
    card(&cards(state), &wt("feat-a")).prs
}

/// US1 scenario 3: one open and two merged pull requests read 1 open, 2 merged.
#[test]
fn a_branch_with_one_open_and_two_merged_reads_counts() {
    let mut state = feat_a();
    read_counts(&mut state, "feat/a", counts(1, 2));
    assert_eq!(prs(&state), PrDisplay::Counts(counts(1, 2)));
}

/// FR-004: a branch the reading covers with no pull request is 0 / 0, not unknown.
#[test]
fn a_branch_without_pull_requests_reads_zero_and_zero() {
    let mut state = feat_a();
    read_counts(&mut state, "feat/a", counts(0, 0));
    assert_eq!(prs(&state), PrDisplay::Counts(counts(0, 0)));
}

/// FR-004, edge case: each reason the counts cannot be shown, in words.
#[test]
fn unknown_counts_name_their_reason() {
    let reason = |state: &State| match prs(state) {
        PrDisplay::Unknown(reason) => reason.label(),
        other => panic!("expected an unknown mark, got {other:?}"),
    };

    let mut state = feat_a();
    state.pr_status.enabled = true;
    assert_eq!(reason(&state), "not read yet");

    state.pr_status.enabled = false;
    assert_eq!(reason(&state), "pull request status is off");

    state.pr_status.enabled = true;
    state.pr_status.unreachable = true;
    assert_eq!(reason(&state), "GitHub is not reachable");

    state.pr_status.unreachable = false;
    let until = 4_000_000_000;
    state.pr_status.pause_until = Some(until);
    assert_eq!(
        reason(&state),
        format!(
            "rate-limited until {}",
            LocalTime::local(until as i64).hh_mm()
        )
    );

    // A reading that does not cover this branch is just not read yet.
    state.pr_status.pause_until = None;
    state.pr_status.read_at = Some(100);
    assert_eq!(reason(&state), "not read yet");
}

/// A reading held while a pause runs still shows its counts.
#[test]
fn counts_of_the_last_reading_show_during_a_pause() {
    let mut state = feat_a();
    read_counts(&mut state, "feat/a", counts(2, 0));
    state.pr_status.pause_until = Some(4_000_000_000);
    assert_eq!(prs(&state), PrDisplay::Counts(counts(2, 0)));
}

/// No branch, no counts: Default, a detached HEAD and an orphan show none, even when a reading
/// holds counts for a branch of that name.
#[test]
fn locations_without_a_branch_hide_the_counts() {
    let mut state = project_with(vec![
        worktree("detached", None, WorktreeStatus::Valid, false),
        worktree("gone", None, WorktreeStatus::Missing, false),
    ]);
    read_counts(&mut state, "main", counts(3, 3));
    let all = cards(&state);
    assert_eq!(card(&all, &SessionLocation::Default).prs, PrDisplay::Hidden);
    assert_eq!(card(&all, &wt("detached")).prs, PrDisplay::Hidden);
    assert_eq!(card(&all, &wt("gone")).prs, PrDisplay::Hidden);
}

/// FR-004: a count of 100 or more is flagged so the card reads *100+*.
#[test]
fn a_full_page_is_flagged_as_capped() {
    let mut state = feat_a();
    let capped = PrCounts {
        open: 0,
        merged: 100,
        open_capped: false,
        merged_capped: true,
    };
    read_counts(&mut state, "feat/a", capped);
    assert_eq!(prs(&state), PrDisplay::Counts(capped));
}

/// R6: a current open pull request that conflicts needs attention, with the reason named.
#[test]
fn an_open_pull_request_with_a_conflict_needs_attention_for_a_merge_conflict() {
    let mut state = feat_a();
    let mut conflicting = status(
        PrState::Open {
            checks: CheckStatus::Passing,
        },
        ReviewState::None,
    );
    conflicting.conflict = true;
    read(&mut state, "feat/a", conflicting, false);
    let all = cards(&state);
    let a = card(&all, &wt("feat-a"));
    assert_eq!(a.state, CardState::NeedsAttention);
    assert_eq!(a.reason, CardReason::MergeConflict);
}
