//! Feature 813 (T010): the Overview's client state — open flag, and the cards built from what the
//! window holds.

mod support;

use micold_client::app::{Message, State};
use micold_client::features::agent_confirm::Prompt;
use micold_client::features::overview::{badge, Badge, Msg, Tone};
use micold_client::features::pr_status::Msg as PrMsg;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::ui::overview::{arrow_focus, cards, follow_up};
use micold_core::local_time::LocalTime;
use micold_core::overview::Activation;
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

// The Toggled/Closed reducer cases are pinned once, in `tests/features_overview.rs` (the feature's
// isolation test); the root only routes them.

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

/// Feature 1121 R8 (Edge Cases): each card carries its directory — a worktree's path, the project
/// root for *Default*, the recorded path for an orphan.
#[test]
fn each_card_carries_its_directory() {
    let state = project_with(vec![
        worktree("feat-a", Some("feat/a"), WorktreeStatus::Valid, false),
        worktree("gone", None, WorktreeStatus::Missing, false),
    ]);
    let all = cards(&state);
    assert_eq!(
        card(&all, &SessionLocation::Default).dir,
        project_path(),
        "Default's directory is the project root"
    );
    assert_eq!(
        card(&all, &wt("feat-a")).dir,
        PathBuf::from("/repo/.claude/worktrees/feat-a"),
        "a worktree's directory is its path"
    );
    assert_eq!(
        card(&all, &wt("gone")).dir,
        PathBuf::from("/repo/.claude/worktrees/gone"),
        "an orphan's directory is its recorded path"
    );
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
fn the_title_is_the_branch_else_the_display_name() {
    let state = project_with(vec![
        worktree("feat-a", Some("feat/a"), WorktreeStatus::Valid, false),
        worktree("gone", None, WorktreeStatus::Missing, false),
    ]);
    let all = cards(&state);
    let root = card(&all, &SessionLocation::Default);
    assert_eq!(
        root.title, root.display_name,
        "the root has no branch on its card"
    );
    assert_eq!(card(&all, &wt("feat-a")).title, "feat/a");
    let gone = card(&all, &wt("gone"));
    assert_eq!(gone.title, gone.display_name);
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

/// US1.6 / FR-008 / SC-004: a change driven through `update` (a session starts, a task reading
/// arrives) shows on the next `cards`, with no refresh step in between.
#[test]
fn a_change_through_update_shows_on_the_next_cards() {
    let mut state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    state.overview.open = true;
    let before = card(&cards(&state), &wt("feat-a")).clone();
    assert_eq!(before.sessions.0, 0);
    assert_eq!(before.progress.source, Source::None);

    state.update(Message::Session(SessionMsg::Started(Session::start_new(
        wt("feat-a"),
        AiCli::ClaudeCode,
    ))));
    assert_eq!(card(&cards(&state), &wt("feat-a")).sessions.0, 1);

    state.update(Message::Overview(Msg::ProgressRead {
        project: project_path(),
        dir: "feat-a".to_string(),
        progress: micold_core::task_progress::FileProgress {
            tasks: Some(Ratio { done: 1, total: 4 }),
            ..Default::default()
        },
    }));
    let after = card(&cards(&state), &wt("feat-a")).clone();
    assert_eq!(after.progress.source, Source::TaskList);
    assert_eq!(after.progress.ratio, Some(Ratio { done: 1, total: 4 }));
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
    // SC-007's bound is on the work, not on a wall clock: one card per location, 30 worktrees
    // plus the project root, and every session counted into its card without a second pass.
    let all = cards(&state);
    assert_eq!(all.len(), 31);
    assert!(all
        .iter()
        .filter(|c| c.location != SessionLocation::Default)
        .all(|c| c.sessions.0 == 3));
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

/// Feature 970 (FR-010): the app's own exit, the Changes view taking the main area, leaves full
/// screen through the real message, not by setting a flag.
#[test]
fn opening_changes_leaves_full_screen() {
    let mut state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    open_overview(&mut state);
    toggle_full_screen(&mut state);
    assert!(state.overview.full_screen());
    state.update(Message::Sidebar(
        micold_client::features::sidebar::Msg::ReviewChangesRequested(wt("feat-a")),
    ));
    assert!(!state.overview.full_screen());
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

// --- The Overview button's badge (US2.4, FR-012) ---

fn failing_checks(state: &mut State, dir: &str, branch: &str) {
    state
        .worktree
        .worktrees
        .push(worktree(dir, Some(branch), WorktreeStatus::Valid, false));
    read(
        state,
        branch,
        status(
            PrState::Open {
                checks: CheckStatus::Failing,
            },
            ReviewState::None,
        ),
        false,
    );
}

fn badge_of(state: &State) -> Option<Badge> {
    badge(&cards(state))
}

#[test]
fn any_card_needing_action_gives_an_error_badge_counting_those_cards() {
    for open in [false, true] {
        let mut state = project_with(vec![
            worktree("feat-a", Some("feat/a"), WorktreeStatus::Valid, false),
            worktree("feat-b", Some("feat/b"), WorktreeStatus::Valid, false),
        ]);
        let a = add_session(&mut state, wt("feat-a"));
        state.agent_confirm.pending.push(prompt(&a));
        failing_checks(&mut state, "feat-c", "feat/c");
        state.overview.open = open;

        assert_eq!(
            badge_of(&state),
            Some(Badge {
                tone: Tone::Error,
                count: 1,
                label: "1 need action".to_string(),
            }),
            "overview open: {open}; attention cards do not count beside action ones"
        );
    }
}

#[test]
fn only_attention_cards_give_a_warning_badge() {
    for open in [false, true] {
        let mut state = project_with(Vec::new());
        failing_checks(&mut state, "feat-a", "feat/a");
        failing_checks(&mut state, "feat-b", "feat/b");
        state.overview.open = open;

        assert_eq!(
            badge_of(&state),
            Some(Badge {
                tone: Tone::Warning,
                count: 2,
                label: "2 need attention".to_string(),
            }),
            "overview open: {open}"
        );
    }
}

#[test]
fn nothing_to_do_gives_no_badge() {
    let state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    assert_eq!(badge_of(&state), None);
    assert_eq!(badge(&[]), None);
}

fn open_overview(state: &mut State) {
    state.update(Message::Overview(Msg::Toggled {
        settings_open: false,
    }));
    assert!(state.overview.open);
}

#[test]
fn activating_a_card_with_a_session_closes_and_selects_it() {
    let mut state = project_with(vec![]);
    let session = add_session(&mut state, SessionLocation::Default);
    let activation = Activation::OpenSession(session.id);
    open_overview(&mut state);
    state.update(Message::Overview(Msg::CardActivated(activation.clone())));
    assert!(!state.overview.open);
    assert_eq!(
        follow_up(&activation),
        Some(Message::Session(SessionMsg::Selected(session.id)))
    );
}

#[test]
fn activating_a_card_without_a_session_reveals_the_row_and_offers_the_start_menu_only() {
    let mut state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    let activation = Activation::OfferStart(wt("feat-a"));
    open_overview(&mut state);
    state.update(Message::Overview(Msg::CardActivated(activation.clone())));
    assert!(!state.overview.open);
    assert_eq!(state.sidebar.revealed, Some(wt("feat-a")));
    assert_eq!(
        follow_up(&activation),
        Some(Message::Session(SessionMsg::StartMenuOpened {
            location: wt("feat-a"),
            unavailable_default: None,
        }))
    );
    assert!(
        state
            .workspace
            .sessions
            .get(&project_path())
            .is_none_or(Vec::is_empty),
        "nothing starts without the user's confirmation (FR-013)"
    );
}

#[test]
fn activating_an_orphan_card_only_reveals_its_row() {
    let mut state = project_with(vec![]);
    let activation = Activation::RevealOnly(wt("gone"));
    open_overview(&mut state);
    state.update(Message::Overview(Msg::CardActivated(activation.clone())));
    assert!(!state.overview.open);
    assert_eq!(state.sidebar.revealed, Some(wt("gone")));
    assert_eq!(follow_up(&activation), None);
}

#[test]
fn the_revealed_row_clears_on_the_next_sidebar_selection() {
    let mut state = project_with(vec![]);
    let session = add_session(&mut state, SessionLocation::Default);
    state.update(Message::Overview(Msg::CardActivated(
        Activation::RevealOnly(wt("gone")),
    )));
    assert!(state.sidebar.revealed.is_some());
    state.update(Message::Session(SessionMsg::Selected(session.id)));
    assert_eq!(state.sidebar.revealed, None);
}

/// The pressed key, not only the mapping: Escape with nothing floating closes the Overview.
#[test]
fn pressing_escape_closes_the_overview() {
    let mut state = project_with(vec![]);
    open_overview(&mut state);
    state.update(Message::EscapePressed);
    assert!(!state.overview.open);
}

#[test]
fn escape_closes_the_overview_after_the_overlay_registry_and_the_settings_draft() {
    use micold_client::app::on_escape;
    let mut state = project_with(vec![]);
    assert_eq!(on_escape(&state), None);
    open_overview(&mut state);
    assert_eq!(on_escape(&state), Some(Message::Overview(Msg::Closed)));
    // A Settings draft is answered first.
    state.settings.settings_draft = Some(Default::default());
    assert_eq!(
        on_escape(&state),
        Some(Message::Settings(
            micold_client::features::settings::Msg::Cancelled
        ))
    );
}

#[test]
fn arrow_keys_move_the_focus_and_other_keys_do_not() {
    use iced::keyboard::key::Named;
    use iced::keyboard::Key;
    let named = |n| Key::Named(n);
    assert_eq!(arrow_focus(&named(Named::ArrowRight)), Some(true));
    assert_eq!(arrow_focus(&named(Named::ArrowDown)), Some(true));
    assert_eq!(arrow_focus(&named(Named::ArrowLeft)), Some(false));
    assert_eq!(arrow_focus(&named(Named::ArrowUp)), Some(false));
    assert_eq!(arrow_focus(&named(Named::Enter)), None);
    assert_eq!(arrow_focus(&Key::Character("a".into())), None);
}

#[test]
fn a_card_whose_worktree_sits_in_a_collapsed_group_has_its_row_drawn_once_activated() {
    use micold_client::features::runs::Msg as RunsMsg;
    use micold_client::features::sidebar::location_row;
    use micold_core::naming::{ConventionalType, DerivedNames, WorktreeNaming};
    use micold_core::runs::{GroupId, NewGroup, Run, RunGroup, RunStatus};
    let mut state = project_with(vec![worktree(
        "feat-a-1",
        Some("feat/a-1"),
        WorktreeStatus::Valid,
        false,
    )]);
    let id = GroupId(uuid::Uuid::from_u128(7));
    let group = RunGroup::new(NewGroup {
        id,
        name: "a".into(),
        naming: WorktreeNaming {
            type_: Some(ConventionalType::Feat),
            ticket: None,
            name: "a".into(),
        },
        prompt: "p".into(),
        base_branch: "main".into(),
        base_commit: "abc".into(),
        created: std::time::SystemTime::UNIX_EPOCH,
        runs: (1..=2u8)
            .map(|number| Run {
                number,
                provider: AiCli::ClaudeCode,
                names: DerivedNames {
                    dir_name: format!("feat-a-{number}"),
                    branch: format!("feat/a-{number}"),
                },
                session: None,
                status: RunStatus::Prompted,
            })
            .collect(),
    })
    .expect("valid group");
    state.update(Message::Runs(RunsMsg::GroupsChanged(vec![group])));
    state.update(Message::Runs(RunsMsg::GroupToggled(id)));
    assert!(!state.runs.is_expanded(id), "the group starts collapsed");
    assert_eq!(
        location_row(&state.sidebar_entries(), &wt("feat-a-1")),
        None
    );

    open_overview(&mut state);
    state.update(Message::Overview(Msg::CardActivated(
        Activation::OfferStart(wt("feat-a-1")),
    )));
    assert!(state.runs.is_expanded(id), "the activation opens the group");
    assert!(
        location_row(&state.sidebar_entries(), &wt("feat-a-1")).is_some(),
        "the revealed row is drawn, so it can be scrolled to and its start control focused"
    );
}

// ---- US4 (T052): the summary line and the progress row ----

use micold_client::features::overview::{titles_to_fetch, TitleLookup};
use micold_core::task_progress::{FileProgress, Phase, Progress, Ratio, Source};

fn ticketed() -> State {
    project_with(vec![worktree(
        "feat-813_worktree-overview",
        Some("feat/813-worktree-overview"),
        WorktreeStatus::Valid,
        false,
    )])
}

fn ticketed_card(state: &State) -> micold_core::overview::CardModel {
    card(&cards(state), &wt("feat-813_worktree-overview")).clone()
}

fn files(
    ledger: Option<Phase>,
    artifact: Option<Phase>,
    tasks: Option<(u32, u32)>,
) -> FileProgress {
    FileProgress {
        feature_dir: Some("specs/NNN-example".into()),
        artifact,
        ledger,
        tasks: tasks.map(|(done, total)| Ratio { done, total }),
    }
}

fn progress_read(state: &mut State, dir: &str, progress: FileProgress) {
    let project = state.workspace.active.clone().unwrap();
    state.update(Message::Overview(Msg::ProgressRead {
        project,
        dir: dir.into(),
        progress,
    }));
}

fn title_read(state: &mut State, number: u64, result: TitleLookup) {
    let project = state.workspace.active.clone().unwrap();
    state.update(Message::Overview(Msg::TitleRead {
        project,
        number,
        result,
    }));
}

/// A read started in a project the window has left is dropped: issue numbers and directory names
/// collide across repositories, so it must not land in the new project's Overview.
#[test]
fn a_read_for_a_project_no_longer_active_is_dropped() {
    let mut state = ticketed();
    let old = state.workspace.active.clone().unwrap();
    let new = PathBuf::from("/elsewhere/other");
    state.update(Message::Overview(Msg::ProjectSwitched));
    state.workspace.active = Some(new);
    state.update(Message::Overview(Msg::TitleRead {
        project: old.clone(),
        number: 813,
        result: TitleLookup::Known("Old repo".into()),
    }));
    state.update(Message::Overview(Msg::ProgressRead {
        project: old,
        dir: "feat-813_worktree-overview".into(),
        progress: files(None, Some(Phase::Plan), None),
    }));
    assert!(state.overview.titles.is_empty());
    assert!(state.overview.progress.is_empty());
}

/// US4.2, FR-017: a reading is kept per worktree; a project switch forgets readings and titles.
#[test]
fn a_progress_reading_is_kept_per_worktree_and_a_project_switch_clears_it_and_the_titles() {
    let mut state = ticketed();
    progress_read(&mut state, "a", files(None, Some(Phase::Plan), None));
    progress_read(&mut state, "b", files(None, Some(Phase::Spec), None));
    assert_eq!(state.overview.progress.len(), 2);
    assert_eq!(state.overview.progress["a"].artifact, Some(Phase::Plan));
    title_read(&mut state, 813, TitleLookup::Known("Overview".into()));
    state.update(Message::Overview(Msg::ProjectSwitched));
    assert!(state.overview.progress.is_empty());
    assert!(state.overview.titles.is_empty());
}

/// US2.1, US2.2, FR-006: a known issue title is the title; every other lookup, a Jira ticket and a
/// worktree without a ticket fall back to the branch; Default is its display name.
#[test]
fn the_title_is_the_known_issue_title_else_the_branch() {
    let mut state = ticketed();
    assert_eq!(
        ticketed_card(&state).title,
        "feat/813-worktree-overview",
        "no entry"
    );
    title_read(&mut state, 813, TitleLookup::Pending);
    assert_eq!(ticketed_card(&state).title, "feat/813-worktree-overview");
    title_read(&mut state, 813, TitleLookup::Unavailable);
    assert_eq!(ticketed_card(&state).title, "feat/813-worktree-overview");
    title_read(
        &mut state,
        813,
        TitleLookup::Known("Worktree overview".into()),
    );
    assert_eq!(ticketed_card(&state).title, "Worktree overview");
    let all = cards(&state);
    let root = card(&all, &SessionLocation::Default);
    assert_eq!(
        root.title, root.display_name,
        "a title never replaces the project root's name"
    );
    let mut jira = project_with(vec![worktree(
        "feat-ABC-12_thing",
        Some("feat/ABC-12-thing"),
        WorktreeStatus::Valid,
        false,
    )]);
    title_read(&mut jira, 12, TitleLookup::Known("Not this one".into()));
    assert_eq!(
        card(&cards(&jira), &wt("feat-ABC-12_thing")).title,
        "feat/ABC-12-thing"
    );
}

/// R8: a read moves a pending ticket to Known or Unavailable.
#[test]
fn a_title_read_moves_pending_to_known_or_unavailable() {
    let mut state = ticketed();
    state.update(Message::Overview(Msg::TitlesRequested(vec![813, 7])));
    assert_eq!(state.overview.titles[&813], TitleLookup::Pending);
    title_read(&mut state, 813, TitleLookup::Known("T".into()));
    title_read(&mut state, 7, TitleLookup::Unavailable);
    assert_eq!(state.overview.titles[&813], TitleLookup::Known("T".into()));
    assert_eq!(state.overview.titles[&7], TitleLookup::Unavailable);
}

/// FR-020: a title never shows in a debug dump, which is what a log of a message would print.
#[test]
fn a_title_does_not_show_in_a_debug_dump() {
    let known = TitleLookup::Known("Secret title".into());
    assert!(!format!("{known:?}").contains("Secret"));
    let msg = Msg::TitleRead {
        project: PathBuf::from("/p"),
        number: 1,
        result: known,
    };
    assert!(!format!("{msg:?}").contains("Secret"));
}

/// R8, FR-019: only numeric tickets with no entry are fetched, and only with GitHub tooling.
#[test]
fn titles_to_fetch_are_the_numeric_tickets_with_no_entry_and_none_without_github() {
    let mut state = State::default();
    state.overview.titles.insert(7, TitleLookup::Unavailable);
    let dirs = [
        "feat-813_a",
        "fix-7_b",
        "feat-ABC-12_c",
        "chore-x",
        "feat-813_d",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(titles_to_fetch(&state.overview, &dirs, true), vec![813]);
    assert!(titles_to_fetch(&state.overview, &dirs, false).is_empty());
}

/// US4.2, US4.3: the phase and ratio come from the files, the card state and the pull request;
/// the ledger wins.
#[test]
fn the_progress_is_read_from_the_files_the_card_state_and_the_pull_request() {
    let dir = "feat-813_worktree-overview";
    let mut state = ticketed();
    progress_read(
        &mut state,
        dir,
        files(None, Some(Phase::Implement), Some((6, 10))),
    );
    let got = ticketed_card(&state).progress;
    assert_eq!(
        got,
        Progress {
            phase: Some(Phase::Implement),
            ratio: Some(Ratio { done: 6, total: 10 }),
            source: Source::TaskList
        }
    );
    read(
        &mut state,
        "feat/813-worktree-overview",
        status(
            PrState::Open {
                checks: CheckStatus::Passing,
            },
            ReviewState::None,
        ),
        false,
    );
    assert_eq!(
        ticketed_card(&state).progress.phase,
        Some(Phase::Pr),
        "an open pull request"
    );
    progress_read(
        &mut state,
        dir,
        files(Some(Phase::Plan), Some(Phase::Implement), Some((6, 10))),
    );
    let got = ticketed_card(&state).progress;
    assert_eq!(got.phase, Some(Phase::Plan), "the ledger wins");
    assert_eq!(got.source, Source::Ledger);
}

/// US4.4, FR-017: nothing to read is no phase and no ratio.
#[test]
fn a_worktree_with_no_source_has_no_phase_and_no_ratio() {
    let mut state = ticketed();
    assert_eq!(ticketed_card(&state).progress, Progress::default());
    progress_read(
        &mut state,
        "feat-813_worktree-overview",
        FileProgress::default(),
    );
    assert_eq!(ticketed_card(&state).progress, Progress::default());
    assert_eq!(
        card(&cards(&state), &SessionLocation::Default).progress,
        Progress::default()
    );
}

fn hooks(line: &str, done: usize, total: usize) -> micold_core::hook_activity::HookActivity {
    use micold_core::hook_activity::{Todo, TodoStatus};
    let todo = |i: usize| Todo {
        text: format!("t{i}"),
        active: None,
        status: if i < done {
            TodoStatus::Completed
        } else if i == done {
            TodoStatus::InProgress
        } else {
            TodoStatus::Pending
        },
    };
    let mut h = micold_core::hook_activity::HookActivity {
        todos: (0..total).map(todo).collect(),
        prompt: None,
    };
    h.todos[done].active = Some(line.to_string());
    h
}

/// Folds a catalog reporting `hook_activity` for `session` into `state`.
fn adopt(
    state: &mut State,
    session: &Session,
    hook: Option<micold_core::hook_activity::HookActivity>,
) {
    use micold_core::protocol::messages::{
        CatalogSnapshot, ProjectSnapshot, SessionSummary, WireLifecycle,
    };
    let snapshot = CatalogSnapshot {
        schema_version: 1,
        last_active: None,
        projects: vec![ProjectSnapshot {
            pane_layout: None,
            path: project_path(),
            display_name: "repo".to_string(),
            is_git_repo: true,
            available: true,
            worktrees: Vec::new(),
            sessions: vec![SessionSummary {
                id: session.id,
                worktree_dir: Some("feat-a".to_string()),
                title: session.label.clone(),
                lifecycle: WireLifecycle::Running,
                activity: ActivitySignal::Working,
                input_serial: 0,
                live_shells: Vec::new(),
                attention_seq: 0,
                unread: false,
                main: true,
                provider: AiCli::ClaudeCode,
                kind: Default::default(),
                origin: None,
                agents: Vec::new(),
                paused_on_user: false,
                hook_activity: hook,
            }],
        }],
        env_include_failures: Vec::new(),
    };
    micold_client::catalog_sync::reconcile_catalog(
        state,
        micold_core::daemons::DaemonId(1),
        &snapshot,
        false,
    );
}

#[test]
fn hook_activity_reaches_the_main_sessions_card_beside_the_task_list_ratio() {
    let mut state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    let session = running(&mut state, wt("feat-a"));
    adopt(&mut state, &session, Some(hooks("Writing tests", 2, 5)));
    let stored = &state.workspace.sessions[&project_path()][0];
    assert!(stored.hook_activity.is_some());
    let all = cards(&state);
    let activity = card(&all, &wt("feat-a")).activity.clone().unwrap();
    assert_eq!(activity.line, "Writing tests");
    assert_eq!(activity.ratio.map(|r| (r.done, r.total)), Some((2, 5)));
    // Story 4's title stays what it was.
    assert_eq!(card(&all, &wt("feat-a")).title, "feat/a");
}

#[test]
fn without_hook_data_or_after_a_reconnect_the_card_falls_back() {
    let mut state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    let session = running(&mut state, wt("feat-a"));
    assert!(card(&cards(&state), &wt("feat-a")).activity.is_none());
    adopt(&mut state, &session, Some(hooks("Writing tests", 0, 2)));
    assert!(card(&cards(&state), &wt("feat-a")).activity.is_some());
    // A reconnect's snapshot carries none: the card is Story 4's again, without an error.
    adopt(&mut state, &session, None);
    let all = cards(&state);
    assert!(card(&all, &wt("feat-a")).activity.is_none());
    assert_eq!(card(&all, &wt("feat-a")).title, "feat/a");
}

#[test]
fn hook_activity_is_runtime_only() {
    use micold_core::store::{JsonFileStore, ProjectStore};
    let project = PathBuf::from("/work/repo");
    let mut session = Session::start_new(wt("feat-a"), AiCli::ClaudeCode);
    assert!(session.hook_activity.is_none());
    session.hook_activity = Some(hooks("SECRET-HOOK-LINE", 0, 2));
    let mut workspace = micold_core::workspace::Workspace::empty();
    workspace
        .projects
        .push(Project::new(project.clone(), true, Availability::Available));
    workspace.sessions.insert(project.clone(), vec![session]);

    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    store.save(&workspace).unwrap();

    for file in [
        dir.path().join("projects.json"),
        store.project_state_path(&project),
    ] {
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        assert!(
            !text.contains("SECRET-HOOK-LINE"),
            "{file:?} persisted hook activity"
        );
        assert!(
            !text.contains("hook_activity"),
            "{file:?} persisted a hook_activity field"
        );
    }
    let restored = store.load().workspace;
    let sessions = &restored.sessions[&project];
    assert_eq!(sessions.len(), 1, "the session itself is persisted");
    assert!(sessions[0].hook_activity.is_none());
}

// Feature 970: full-screen mode at the app level (FR-009, FR-010, FR-011, SC-004).

fn toggle_full_screen(state: &mut State) {
    state.update(Message::Overview(Msg::FullScreenToggled {
        settings_open: false,
    }));
}

fn open_and_close_settings(state: &mut State) {
    use micold_client::features::settings::Msg as SettingsMsg;
    state.update(Message::Settings(SettingsMsg::Opened));
    assert!(state.settings.settings_draft.is_some(), "Settings opened");
    state.update(Message::Settings(SettingsMsg::Cancelled));
    assert!(state.settings.settings_draft.is_none(), "Settings closed");
}

/// The sidebar fields the mode must never write.
fn sidebar_of(state: &State) -> (bool, u16) {
    (state.sidebar.hidden, state.sidebar.width)
}

#[test]
fn settings_open_and_closed_keeps_full_screen() {
    let mut state = project_with(vec![]);
    open_overview(&mut state);
    toggle_full_screen(&mut state);
    assert!(state.overview.full_screen());
    open_and_close_settings(&mut state);
    assert!(state.overview.full_screen());
}

#[test]
fn the_mode_writes_no_sidebar_state() {
    let mut state = project_with(vec![]);
    state.sidebar.width = 300;
    let before = sidebar_of(&state);
    open_overview(&mut state);
    toggle_full_screen(&mut state);
    assert!(state.overview.full_screen());
    assert_eq!(sidebar_of(&state), before, "entering");
    toggle_full_screen(&mut state);
    assert!(!state.overview.full_screen());
    assert_eq!(sidebar_of(&state), before, "leaving");
}

#[test]
fn twenty_cycles_return_the_sidebar_as_it_was() {
    let mut state = project_with(vec![]);
    let session = add_session(&mut state, SessionLocation::Default);
    let leave: [Msg; 4] = [
        Msg::FullScreenToggled {
            settings_open: false,
        },
        Msg::Closed,
        Msg::Toggled {
            settings_open: false,
        },
        Msg::CardActivated(Activation::OpenSession(session.id)),
    ];
    // Collapsed, then expanded at the default width (0), 240 px and 420 px.
    let sidebars = [(true, 0), (false, 0), (false, 240), (false, 420)];
    for cycle in 0..20 {
        let (hidden, width) = sidebars[cycle / 5];
        state.sidebar.hidden = hidden;
        state.sidebar.width = width;
        if !state.overview.open {
            open_overview(&mut state);
        }
        toggle_full_screen(&mut state);
        assert!(state.overview.full_screen(), "cycle {cycle}: entered");
        state.update(Message::Overview(leave[cycle % leave.len()].clone()));
        assert!(!state.overview.full_screen(), "cycle {cycle}: left");
        assert_eq!(
            sidebar_of(&state),
            (hidden, width),
            "cycle {cycle}: the sidebar is as it was"
        );
    }
}

#[test]
fn settings_and_a_project_switch_keep_the_mode_and_the_sidebar() {
    let mut state = project_with(vec![]);
    state.sidebar.hidden = true;
    let before = sidebar_of(&state);
    open_overview(&mut state);
    toggle_full_screen(&mut state);
    open_and_close_settings(&mut state);
    state.update(Message::Overview(Msg::ProjectSwitched));
    assert!(state.overview.full_screen());
    assert_eq!(sidebar_of(&state), before);
    toggle_full_screen(&mut state);
    assert!(!state.overview.full_screen());
    assert_eq!(sidebar_of(&state), before);
}

/// F5, US3 (feature 1120): a prompt wrapped in markup goes from the hook body, through
/// `HookActivity::apply` and `cards()`, to the painted card with the markup removed.
#[test]
fn a_marked_up_prompt_is_painted_on_the_card_without_its_markup() {
    use support::layout::{painted_text_settled_at, renderer, view_of, StateUnderTest};
    let mut state = project_with(vec![worktree(
        "feat-a",
        Some("feat/a"),
        WorktreeStatus::Valid,
        false,
    )]);
    let mut session = running(&mut state, wt("feat-a"));
    let mut hook = micold_core::hook_activity::HookActivity::default();
    assert!(hook.apply(&serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "<agent-message from=\"orchestrator\">Continue M2</agent-message>",
    })));
    session.hook_activity = Some(hook);
    state.workspace.sessions.get_mut(&project_path()).unwrap()[0] = session;
    state.overview.open = true;
    let under = StateUnderTest::new(state);
    let (painted, _) = painted_text_settled_at(
        view_of(&under),
        &mut renderer(),
        iced::Size::new(1280.0, 2400.0),
    );
    let line = painted
        .iter()
        .find(|p| p.content.contains("Continue M2"))
        .unwrap_or_else(|| panic!("the prompt is painted on the card: {painted:?}"));
    assert!(
        !line.content.contains('<'),
        "no markup survives to the painted card: {line:?}"
    );
}

/// Feature 1121 T027: the Overview's expansion, through the application's update.
mod expansion {
    use super::*;
    use micold_client::features::overview::MenuOrigin;
    use micold_core::overview_expansion::Event;
    use support::layout::{painted_text_settled_at, renderer, view_of, StateUnderTest};

    fn two_cards() -> State {
        let mut state = project_with(vec![
            worktree("feat-a", Some("feat/a"), WorktreeStatus::Valid, false),
            worktree("feat-b", Some("feat/b"), WorktreeStatus::Valid, false),
        ]);
        open_overview(&mut state);
        state
    }

    fn apply(state: &mut State, event: Event) {
        state.update(Message::Overview(Msg::Expansion(event)));
    }

    fn expanded(state: &State) -> Option<SessionLocation> {
        state.overview.expansion.expanded().cloned()
    }

    /// The painted texts that name a worktree directory: only an expanded card shows one (FR-004).
    fn painted_dirs(state: State) -> Vec<String> {
        let under = StateUnderTest::new(state);
        let (painted, _) = painted_text_settled_at(
            view_of(&under),
            &mut renderer(),
            iced::Size::new(1280.0, 2400.0),
        );
        painted
            .into_iter()
            .map(|p| p.content)
            .filter(|c| c.contains("worktrees"))
            .collect()
    }

    /// US2 sc.1, FR-002: a resting pointer expands its card, and the view draws it expanded.
    #[test]
    fn a_rested_card_is_expanded_in_the_view() {
        let mut state = two_cards();
        assert_eq!(expanded(&state), None, "opening expands nothing (FR-013)");
        assert!(painted_dirs(state.clone()).is_empty());
        apply(&mut state, Event::PointerRested(wt("feat-b")));
        assert_eq!(expanded(&state), Some(wt("feat-b")));
        let dirs = painted_dirs(state);
        assert_eq!(dirs.len(), 1, "one card shows its directory: {dirs:?}");
        assert!(dirs[0].contains("feat-b"), "{dirs:?}");
    }

    /// FR-012, US4 sc.3: an open card menu freezes the expansion until it closes.
    #[test]
    fn an_open_card_menu_freezes_the_expansion() {
        let mut state = two_cards();
        apply(&mut state, Event::PointerRested(wt("feat-a")));
        state.update(Message::Overview(Msg::CardMenuOpened {
            location: wt("feat-a"),
            anchor: (10, 10),
            origin: MenuOrigin::Pointer,
        }));
        apply(&mut state, Event::PointerLeft(wt("feat-a")));
        apply(&mut state, Event::PointerRested(wt("feat-b")));
        assert_eq!(
            expanded(&state),
            Some(wt("feat-a")),
            "the target keeps its form"
        );
        state.update(Message::Overview(Msg::CardMenuDismissed));
        apply(&mut state, Event::PointerRested(wt("feat-b")));
        assert_eq!(
            expanded(&state),
            Some(wt("feat-b")),
            "the menu closed: rests act again"
        );
    }

    /// FR-012: a chosen menu entry closes the menu as a dismissal does.
    #[test]
    fn a_chosen_menu_entry_unfreezes_the_expansion() {
        let mut state = two_cards();
        state.update(Message::Overview(Msg::CardMenuOpened {
            location: wt("feat-a"),
            anchor: (10, 10),
            origin: MenuOrigin::Pointer,
        }));
        apply(&mut state, Event::PointerRested(wt("feat-b")));
        assert_eq!(expanded(&state), None);
        state.update(Message::Overview(Msg::CardMenuEntryChosen(Box::new(
            Message::Overview(Msg::GridScrolled),
        ))));
        apply(&mut state, Event::PointerRested(wt("feat-b")));
        assert_eq!(expanded(&state), Some(wt("feat-b")));
    }

    /// FR-005, FR-013: closing, toggling or activating leaves the Overview with nothing expanded,
    /// so reopening it shows every card collapsed. An opening focus that never reported is
    /// dropped with it, so a later focus of that card expands it (review A/B r2 F1, M4).
    #[test]
    fn leaving_the_overview_resets_the_expansion() {
        let leave = [
            Msg::Closed,
            Msg::Toggled {
                settings_open: false,
            },
            Msg::CardActivated(Activation::OfferStart(wt("feat-a"))),
        ];
        for msg in leave {
            let mut state = two_cards();
            apply(&mut state, Event::PointerRested(wt("feat-a")));
            apply(&mut state, Event::OpenFocused(wt("feat-b")));
            state.update(Message::Overview(msg.clone()));
            assert!(!state.overview.open, "{msg:?}");
            open_overview(&mut state);
            assert_eq!(expanded(&state), None, "{msg:?}");
            apply(&mut state, Event::FocusGained(wt("feat-b")));
            assert_eq!(expanded(&state), Some(wt("feat-b")), "{msg:?}");
        }
    }

    /// FR-005 (*card removed*): a listing without the expanded card leaves none expanded, and no
    /// menu open.
    #[test]
    fn a_removed_card_is_no_longer_expanded() {
        let mut state = two_cards();
        apply(&mut state, Event::PointerRested(wt("feat-b")));
        state.worktree.worktrees.retain(|w| w.dir_name != "feat-b");
        state.update(Message::Overview(Msg::GridScrolled));
        assert_eq!(expanded(&state), None);
        assert!(state.overview.card_menu.is_none());
    }

    /// Feature 1121 T038, Story 3 sc.1/sc.3: keyboard focus expands its card; the focus moving
    /// on expands the next and collapses the first; focus leaving the grid collapses it.
    #[test]
    fn keyboard_focus_expands_a_card_and_its_loss_collapses_it() {
        let mut state = two_cards();
        apply(&mut state, Event::FocusGained(wt("feat-a")));
        assert_eq!(expanded(&state), Some(wt("feat-a")));
        apply(&mut state, Event::FocusGained(wt("feat-b")));
        apply(&mut state, Event::FocusLost(wt("feat-a")));
        assert_eq!(expanded(&state), Some(wt("feat-b")));
        apply(&mut state, Event::FocusLost(wt("feat-b")));
        assert_eq!(expanded(&state), None);
    }

    /// T038, Edge Case: the focused card is removed; the focus hand-back (feature 972 FR-011)
    /// gives another card the keyboard, and that card expands.
    #[test]
    fn the_focus_hand_back_from_a_removed_card_expands_the_card_it_reaches() {
        let mut state = two_cards();
        apply(&mut state, Event::FocusGained(wt("feat-b")));
        state.worktree.worktrees.retain(|w| w.dir_name != "feat-b");
        state.update(Message::Overview(Msg::GridScrolled));
        assert_eq!(expanded(&state), None);
        apply(&mut state, Event::FocusGained(wt("feat-a")));
        assert_eq!(expanded(&state), Some(wt("feat-a")));
    }

    /// T039 (D1), Story 3 sc.2, FR-005: Escape with a card expanded only collapses it: the
    /// Overview and full-screen mode stay; a second Escape closes the Overview.
    #[test]
    fn escape_collapses_an_expanded_card_before_it_closes_the_overview() {
        for full_screen in [false, true] {
            let mut state = two_cards();
            if full_screen {
                toggle_full_screen(&mut state);
            }
            apply(&mut state, Event::FocusGained(wt("feat-a")));
            state.update(Message::EscapePressed);
            assert_eq!(expanded(&state), None, "full screen {full_screen}");
            assert!(state.overview.open, "the first Escape closed the Overview");
            assert_eq!(state.overview.full_screen(), full_screen);
            state.update(Message::EscapePressed);
            assert!(!state.overview.open, "the second Escape closes it");
        }
    }

    /// T039 (D3): with a card menu open, Escape only closes the menu; the card keeps its form.
    #[test]
    fn escape_with_a_card_menu_open_only_closes_the_menu() {
        let mut state = two_cards();
        apply(&mut state, Event::FocusGained(wt("feat-a")));
        state.update(Message::Overview(Msg::CardMenuOpened {
            location: wt("feat-a"),
            anchor: (10, 10),
            origin: MenuOrigin::Keyboard,
        }));
        state.update(Message::EscapePressed);
        assert!(state.overview.card_menu.is_none(), "the menu closed");
        assert_eq!(expanded(&state), Some(wt("feat-a")));
        assert!(state.overview.open);
    }

    /// A project switch forgets the expansion with the rest of what the Overview read.
    #[test]
    fn a_project_switch_resets_the_expansion() {
        let mut state = two_cards();
        apply(&mut state, Event::PointerRested(wt("feat-a")));
        state.update(Message::Overview(Msg::ProjectSwitched));
        assert_eq!(expanded(&state), None);
    }
}

/// Feature 1122: the issue link on the cards, and the repository it needs (R2, U6).
mod issue_link {
    use super::*;
    use micold_client::features::overview::RepoLookup;
    use micold_client::ui::overview::issue_link_for;
    use micold_core::github::GithubRepo;
    use micold_core::issue_link::IssueLink;

    const LINKED: &str = "feat-1122_overview-issue-link";

    fn o_r() -> GithubRepo {
        GithubRepo::from_remote_url("git@github.com:o/r.git").expect("a GitHub remote")
    }

    fn repo_read(state: &mut State, project: PathBuf, repo: Option<GithubRepo>) {
        state.update(Message::Overview(Msg::RepoRead { project, repo }));
    }

    /// A worktree of the project, its path built with `Path::join` (G1).
    fn named(dir: &str) -> Worktree {
        Worktree {
            dir_name: dir.to_string(),
            path: project_path().join(".claude").join("worktrees").join(dir),
            branch: Some(format!("feat/{dir}")),
            status: WorktreeStatus::Valid,
            included: false,
        }
    }

    /// The project on `o/r` with the GitHub issues switch on, and a GitHub ticket, no ticket and a
    /// Jira key among its worktrees.
    fn linked_project() -> State {
        let mut state = project_with(
            [LINKED, "feat_no-ticket", "feat-mic-87_add-login"]
                .into_iter()
                .map(named)
                .collect(),
        );
        state.settings.github_issues_enabled = true;
        state.settings.jira_enabled = true;
        state.overview.repo = RepoLookup::Known(Some(o_r()));
        state
    }

    fn link_of(state: &State, location: &SessionLocation) -> Option<IssueLink> {
        card(&cards(state), location).issue_link.clone()
    }

    /// R2: the read is kept for the active project and dropped for one the window has left.
    #[test]
    fn a_repo_read_is_recorded_for_the_active_project_and_dropped_for_another() {
        let mut state = project_with(vec![]);
        repo_read(&mut state, PathBuf::from("/elsewhere"), Some(o_r()));
        assert_eq!(state.overview.repo, RepoLookup::Unknown);
        repo_read(&mut state, project_path(), Some(o_r()));
        assert_eq!(state.overview.repo, RepoLookup::Known(Some(o_r())));
        repo_read(&mut state, project_path(), None);
        assert_eq!(state.overview.repo, RepoLookup::Known(None));
    }

    /// R2: a project switch forgets the repository; the new project's is read anew.
    #[test]
    fn a_project_switch_forgets_the_repo() {
        let mut state = project_with(vec![]);
        repo_read(&mut state, project_path(), Some(o_r()));
        state.update(Message::Overview(Msg::ProjectSwitched));
        assert_eq!(state.overview.repo, RepoLookup::Unknown);
    }

    /// Story 1 sc.1, FR-002, FR-004: a GitHub ticket on a project on `o/r` gives its card the link.
    #[test]
    fn a_github_ticketed_card_carries_its_issue_link() {
        let state = linked_project();
        let link = link_of(&state, &wt(LINKED)).expect("the card has its issue link");
        assert_eq!(link.label, "Issue #1122");
        assert_eq!(link.address, "https://github.com/o/r/issues/1122");
        assert_eq!(issue_link_for(&state, &wt(LINKED)), Some(link));
    }

    /// Story 1 sc.4, FR-001: no link on Default, without a ticket, or for a Jira key while no
    /// Jira base URL is known (M5).
    #[test]
    fn no_issue_link_on_default_without_a_ticket_or_for_a_jira_key() {
        let state = linked_project();
        for location in [
            SessionLocation::Default,
            wt("feat_no-ticket"),
            wt("feat-mic-87_add-login"),
        ] {
            assert_eq!(link_of(&state, &location), None, "{location:?}");
            assert_eq!(issue_link_for(&state, &location), None, "{location:?}");
        }
    }

    /// Story 1 sc.5, sc.6, FR-001 (D3): no GitHub link with the switch off, with no GitHub
    /// repository, or while the repository is not known yet.
    #[test]
    fn no_github_issue_link_with_the_switch_off_or_no_repository_known() {
        let cases = [
            (false, RepoLookup::Known(Some(o_r()))),
            (true, RepoLookup::Known(None)),
            (true, RepoLookup::Unknown),
            (true, RepoLookup::Asked),
        ];
        for (on, repo) in cases {
            let mut state = linked_project();
            state.settings.github_issues_enabled = on;
            state.overview.repo = repo.clone();
            assert_eq!(link_of(&state, &wt(LINKED)), None, "{on} {repo:?}");
            assert_eq!(issue_link_for(&state, &wt(LINKED)), None, "{on} {repo:?}");
        }
    }

    /// FR-005, FR-013: activating the link changes no Overview state and selects nothing; the root
    /// opens the address (`main_tests.rs`).
    #[test]
    fn activating_an_issue_link_changes_no_overview_state() {
        let mut state = linked_project();
        open_overview(&mut state);
        let before = state.overview.clone();
        state.update(Message::Overview(Msg::IssueLinkActivated(wt(LINKED))));
        assert_eq!(state.overview, before);
        assert!(state.overview.open);
        assert_eq!(state.sidebar.revealed, None);
    }

    // --- Feature 1122 M4: the link by keyboard (FR-008, FR-009, contract U6) -----------------

    fn link_event(state: &mut State, event: micold_core::overview_expansion::Event) {
        state.update(Message::Overview(Msg::Expansion(event)));
    }

    /// The linked project's Overview, the keyboard Tabbed from `LINKED`'s card onto its link.
    fn on_the_link() -> State {
        use micold_core::overview_expansion::Event;
        let mut state = linked_project();
        open_overview(&mut state);
        for event in [
            Event::FocusGained(wt(LINKED)),
            Event::FocusLost(wt(LINKED)),
            Event::LinkFocusGained(wt(LINKED)),
        ] {
            link_event(&mut state, event);
        }
        state
    }

    /// FR-008, D3: the link's focus reports reach the reducer as `LinkFocusGained/Lost`: the card
    /// stays expanded with focus on its link, and the link's loss to the next card moves it on.
    #[test]
    fn link_focus_reports_reach_the_expansion() {
        use micold_core::overview_expansion::Event;
        let mut state = on_the_link();
        let expansion = &state.overview.expansion;
        assert_eq!(expansion.expanded(), Some(&wt(LINKED)));
        assert_eq!(expansion.focused_link(), Some(&wt(LINKED)));
        link_event(&mut state, Event::LinkFocusLost(wt(LINKED)));
        link_event(&mut state, Event::FocusGained(wt("feat_no-ticket")));
        assert_eq!(
            state.overview.expansion.expanded(),
            Some(&wt("feat_no-ticket"))
        );
    }

    /// FR-009, Story 3 sc.4 (`app.rs::on_escape` unchanged): Escape on the focused link of an
    /// expanded card collapses it, leaves the keyboard on the link and keeps the Overview open.
    #[test]
    fn escape_on_a_focused_link_collapses_its_card_and_keeps_the_overview() {
        let mut state = on_the_link();
        state.update(Message::EscapePressed);
        assert_eq!(state.overview.expansion.expanded(), None);
        assert_eq!(state.overview.expansion.focused_link(), Some(&wt(LINKED)));
        assert!(state.overview.open);
    }

    /// FR-009, R9: a menu opened from the link highlights its first entry as a keyboard menu does;
    /// dismissed, it hands the keyboard back to the link; an entry's hand-back is the card's.
    #[test]
    fn a_menu_from_the_link_hands_the_keyboard_back_to_the_link() {
        use micold_client::features::overview::MenuOrigin;
        for (close, to_link) in [
            (Msg::CardMenuDismissed, true),
            (
                Msg::CardMenuEntryChosen(Box::new(Message::Overview(Msg::GridScrolled))),
                false,
            ),
        ] {
            let mut state = on_the_link();
            state.update(Message::Overview(Msg::CardMenuOpened {
                location: wt(LINKED),
                anchor: (10, 10),
                origin: MenuOrigin::KeyboardFromLink,
            }));
            let menu = state.overview.card_menu.clone().expect("the menu is open");
            assert_eq!(menu.origin, MenuOrigin::KeyboardFromLink);
            assert_eq!(menu.highlighted, Some(0));
            state.update(Message::Overview(close.clone()));
            assert_eq!(state.overview.focus_return, Some(wt(LINKED)), "{close:?}");
            assert_eq!(state.overview.focus_return_link, to_link, "{close:?}");
        }
        let mut state = on_the_link();
        state.update(Message::Overview(Msg::CardMenuOpened {
            location: wt(LINKED),
            anchor: (10, 10),
            origin: MenuOrigin::Keyboard,
        }));
        state.update(Message::Overview(Msg::CardMenuDismissed));
        assert!(!state.overview.focus_return_link, "the card's menu");
    }
}
