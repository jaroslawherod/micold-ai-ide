//! The Changes view's pull request list, in isolation (feature 775, M1: contracts/changes-view.md,
//! data-model §2). Builds only `features::changes::State` and drives `update`.

use std::path::PathBuf;

use micold_client::features::changes::{self, pr_mark, Effect, Msg, PrList, State};
use micold_client::ui::PrMark;
use micold_core::pull_request::{BranchPrState, BranchPullRequest, ReadingFailure};
use micold_core::session::SessionLocation;

fn project() -> PathBuf {
    PathBuf::from("/projects/p")
}

fn worktree() -> SessionLocation {
    SessionLocation::Worktree("feat-a".into())
}

fn pr(number: u64, state: BranchPrState) -> BranchPullRequest {
    BranchPullRequest {
        number,
        title: format!("title {number}"),
        url: format!("https://github.com/acme/widgets/pull/{number}"),
        state,
        created_at: "2026-10-10T09:00:00Z".into(),
        base: "main".into(),
        head: "abc".into(),
    }
}

/// A state with the setting on and the view of `worktree()` open; the list read it asked for is
/// answered nowhere, as the pull request list does not depend on it.
fn opened(enabled: bool) -> State {
    let mut state = State::default();
    changes::update(&mut state, Msg::PullRequestSettingChanged(enabled));
    let effect = changes::update(
        &mut state,
        Msg::Opened {
            project: project(),
            entry: worktree(),
        },
    );
    assert!(
        matches!(effect, Effect::ReadList { .. }),
        "FR-009: the list read comes first"
    );
    state
}

fn wanted(state: &mut State, branch: Option<&str>) -> Effect {
    changes::update(
        state,
        Msg::PullRequestsWanted {
            branch: branch.map(str::to_string),
        },
    )
}

fn view_list(state: &State) -> &PrList {
    &state.open.as_ref().expect("open").pull_requests
}

fn seq_of(effect: Effect) -> u64 {
    match effect {
        Effect::ReadPullRequests { seq, .. } => seq,
        other => panic!("expected ReadPullRequests, got {other:?}"),
    }
}

#[test]
fn opening_reads_the_list_first_and_the_pull_requests_apart_from_it() {
    let mut state = opened(true);
    let effect = wanted(&mut state, Some("feat/a"));
    let Effect::ReadPullRequests {
        project: p, branch, ..
    } = effect
    else {
        panic!("expected ReadPullRequests, got {effect:?}");
    };
    assert_eq!((p, branch), (project(), "feat/a".to_string()));
    assert!(matches!(view_list(&state), PrList::Loading { .. }));
}

#[test]
fn the_setting_off_gives_no_section_and_no_read() {
    let mut state = opened(false);
    assert_eq!(wanted(&mut state, Some("feat/a")), Effect::None);
    assert_eq!(view_list(&state), &PrList::Absent);
}

#[test]
fn an_entry_without_a_branch_gives_no_section_and_no_read() {
    let mut state = opened(true);
    assert_eq!(wanted(&mut state, None), Effect::None);
    assert_eq!(view_list(&state), &PrList::Absent);
}

#[test]
fn the_answer_fills_the_list_and_a_stale_answer_is_dropped() {
    let mut state = opened(true);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    let stale = Msg::PullRequestsRead {
        seq: seq + 99,
        read_at: 5,
        result: Ok(vec![pr(1, BranchPrState::Open)]),
    };
    changes::update(&mut state, stale);
    assert!(matches!(view_list(&state), PrList::Loading { .. }));

    let list = vec![pr(2, BranchPrState::Merged), pr(1, BranchPrState::Open)];
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 7,
            result: Ok(list.clone()),
        },
    );
    assert_eq!(
        view_list(&state),
        &PrList::Ready {
            prs: list,
            read_at: 7
        }
    );
}

#[test]
fn turning_the_setting_on_asks_for_the_list_and_off_clears_it() {
    let mut state = opened(false);
    wanted(&mut state, Some("feat/a"));
    let effect = changes::update(&mut state, Msg::PullRequestSettingChanged(true));
    let seq = seq_of(effect);
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 1,
            result: Ok(vec![pr(1, BranchPrState::Open)]),
        },
    );
    assert!(matches!(view_list(&state), PrList::Ready { .. }));

    let effect = changes::update(&mut state, Msg::PullRequestSettingChanged(false));
    assert_eq!(effect, Effect::None);
    assert_eq!(view_list(&state), &PrList::Absent);
}

#[test]
fn a_failed_read_keeps_the_last_list_and_the_view_has_no_notice_channel() {
    let mut state = opened(true);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    let list = vec![pr(1, BranchPrState::Draft)];
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 3,
            result: Ok(list.clone()),
        },
    );
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    let effect = changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 9,
            result: Err(ReadingFailure::Passing),
        },
    );
    assert_eq!(effect, Effect::None, "FR-008: nothing follows a failure");
    assert_eq!(
        view_list(&state)
            .shown()
            .map(|(prs, at)| (prs.to_vec(), at)),
        Some((list, 3))
    );
}

#[test]
fn no_github_access_removes_the_section() {
    let mut state = opened(true);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 3,
            result: Err(ReadingFailure::Unavailable),
        },
    );
    assert_eq!(view_list(&state), &PrList::Absent);
}

#[test]
fn a_worktree_with_no_session_lists_the_same() {
    // The view is keyed by the entry, never by a session (FR-002): the same messages give the same
    // list whichever session, if any, is in the worktree.
    let mut state = opened(true);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 1,
            result: Ok(vec![pr(1, BranchPrState::Closed)]),
        },
    );
    assert_eq!(view_list(&state).shown().map(|(p, _)| p.len()), Some(1));
}

#[test]
fn a_request_made_while_one_runs_runs_once_more_after_it() {
    let mut state = opened(true);
    let first = seq_of(wanted(&mut state, Some("feat/a")));
    assert_eq!(wanted(&mut state, Some("feat/a")), Effect::None);
    let effect = changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq: first,
            read_at: 1,
            result: Ok(vec![]),
        },
    );
    assert!(seq_of(effect) > first);
}

#[test]
fn the_four_states_have_four_distinct_marks() {
    let marks = [
        pr_mark(BranchPrState::Open),
        pr_mark(BranchPrState::Draft),
        pr_mark(BranchPrState::Merged),
        pr_mark(BranchPrState::Closed),
    ];
    assert_eq!(
        marks,
        [PrMark::Open, PrMark::Draft, PrMark::Merged, PrMark::Closed]
    );
}
