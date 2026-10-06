//! The Changes view's list half, in isolation (feature 482, T014, contracts/changes-view.md V1–V3,
//! L1, L3; data-model § Client).
//!
//! Builds only `features::changes::State` and drives `update` with the messages the root and the
//! shell send.

use std::collections::BTreeSet;

use micold_client::features::changes::{
    self, base_line, committed_available, list_body, Effect, ListBody, Load, Msg, State,
    BOTH_HIDDEN, DEFAULT_ENTRY_NOTE,
};
use micold_core::review::base::{Base, BaseUnavailable, ReviewScope, Toggles};
use micold_core::review::changes::{ChangeKind, ChangeList, ChangedFile, Content, Origin};
use micold_core::review::RelPath;
use micold_core::session::SessionLocation;

fn path(p: &str) -> RelPath {
    RelPath::from_native(p).expect("relative path")
}

fn file(p: &str) -> ChangedFile {
    ChangedFile {
        path: path(p),
        kind: ChangeKind::Modified,
        content: Content::Text,
        added: 1,
        removed: 0,
        large: false,
        origin: Origin::Uncommitted,
    }
}

fn main_base() -> ReviewScope {
    ReviewScope::Worktree {
        base: Base::MergeBase {
            branch: "main".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
        },
    }
}

fn list(paths: &[&str]) -> ChangeList {
    ChangeList {
        files: paths.iter().map(|p| file(p)).collect(),
        scope: main_base(),
    }
}

fn worktree() -> SessionLocation {
    SessionLocation::Worktree("feat-a".into())
}

/// Open the view of `entry` and return the read it asked for.
fn open(state: &mut State, entry: SessionLocation) -> u64 {
    match changes::update(state, Msg::Opened { entry }) {
        Effect::ReadList { seq, .. } => seq,
        other => panic!("opening reads the list, got {other:?}"),
    }
}

/// Open a worktree's view and answer its first read with `paths`.
fn ready(paths: &[&str]) -> State {
    let mut state = State::default();
    let seq = open(&mut state, worktree());
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(paths)),
        },
    );
    state
}

/// V1, L1. Opening an entry reads its list with both toggles on.
#[test]
fn opening_an_entry_reads_its_list_with_both_toggles_on() {
    let mut state = State::default();
    let effect = changes::update(&mut state, Msg::Opened { entry: worktree() });
    let Effect::ReadList {
        seq,
        entry,
        toggles,
    } = effect
    else {
        panic!("opening reads the list, got {effect:?}");
    };
    assert_eq!(entry, worktree());
    assert_eq!(toggles, Toggles::default());
    assert!(toggles.committed && toggles.uncommitted, "both on at open");
    let view = state.open.as_ref().expect("the view is open");
    assert_eq!(view.entry, worktree());
    assert!(matches!(view.list, Load::Loading { seq: s, again: false, .. } if s == seq));
}

/// FR-004: the toggles reset each time the view opens.
#[test]
fn reopening_resets_the_toggles() {
    let mut state = ready(&["a.rs"]);
    changes::update(&mut state, Msg::CommittedToggled);
    changes::update(&mut state, Msg::Closed);
    assert!(state.open.is_none(), "the close action closes the view");
    open(&mut state, worktree());
    assert_eq!(state.open.unwrap().toggles, Toggles::default());
}

/// US1 s2. A toggle change re-reads the list under the new toggles.
#[test]
fn a_toggle_change_rereads_the_list() {
    let mut state = ready(&["a.rs", "b.rs"]);
    let effect = changes::update(&mut state, Msg::UncommittedToggled);
    let Effect::ReadList { toggles, .. } = effect else {
        panic!("a toggle change re-reads, got {effect:?}");
    };
    assert_eq!(
        toggles,
        Toggles {
            committed: true,
            uncommitted: false
        }
    );
    let effect = changes::update(&mut state, Msg::CommittedToggled);
    assert!(matches!(
        effect,
        Effect::None | Effect::ReadList { .. }
    ));
}

/// An answer to a read that is no longer the current one is dropped.
#[test]
fn an_answer_with_a_stale_seq_is_dropped() {
    let mut state = State::default();
    let first = open(&mut state, worktree());
    changes::update(&mut state, Msg::Closed);
    let second = open(&mut state, worktree());
    assert_ne!(first, second);
    changes::update(
        &mut state,
        Msg::ListRead {
            seq: first,
            result: Ok(list(&["stale.rs"])),
        },
    );
    assert!(
        matches!(state.open.as_ref().unwrap().list, Load::Loading { seq, .. } if seq == second),
        "the stale answer changed nothing"
    );
}

/// A read asked while one is in flight runs once more after it, not alongside it.
#[test]
fn a_read_asked_while_one_runs_runs_once_after_it() {
    let mut state = State::default();
    let seq = open(&mut state, worktree());
    assert_eq!(
        changes::update(&mut state, Msg::UncommittedToggled),
        Effect::None,
        "no second read while one runs"
    );
    assert!(matches!(
        state.open.as_ref().unwrap().list,
        Load::Loading { again: true, .. }
    ));
    let effect = changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["a.rs"])),
        },
    );
    let Effect::ReadList { toggles, .. } = effect else {
        panic!("the queued read runs, got {effect:?}");
    };
    assert!(!toggles.uncommitted, "with the toggles as they are now");
}

/// The selected path survives a re-read while it is still listed, and is dropped when it is not.
#[test]
fn the_selection_is_kept_across_a_reread_while_still_listed() {
    let mut state = ready(&["a.rs", "b.rs"]);
    changes::update(&mut state, Msg::FileSelected(path("b.rs")));
    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::CommittedToggled) else {
        panic!("re-read");
    };
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["b.rs", "c.rs"])),
        },
    );
    assert_eq!(state.open.as_ref().unwrap().selected, Some(path("b.rs")));

    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::CommittedToggled) else {
        panic!("re-read");
    };
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["c.rs"])),
        },
    );
    assert_eq!(state.open.as_ref().unwrap().selected, None);
}

/// V2. Selecting a session closes the view.
#[test]
fn selecting_a_session_closes_the_view() {
    let mut state = ready(&["a.rs"]);
    changes::update(&mut state, Msg::SessionSelected);
    assert!(state.open.is_none());
}

/// V3. The view of a worktree that leaves the list closes; others stay open.
#[test]
fn the_entry_leaving_the_catalog_closes_the_view() {
    let mut state = ready(&["a.rs"]);
    changes::update(
        &mut state,
        Msg::WorktreesListed(BTreeSet::from(["feat-a".to_string(), "other".to_string()])),
    );
    assert!(state.open.is_some(), "still listed: stays open");
    changes::update(
        &mut state,
        Msg::WorktreesListed(BTreeSet::from(["other".to_string()])),
    );
    assert!(state.open.is_none(), "removed: closes");

    let mut state = State::default();
    open(&mut state, SessionLocation::Default);
    changes::update(&mut state, Msg::WorktreesListed(BTreeSet::new()));
    assert!(state.open.is_some(), "the Default entry never leaves the catalog");
}

/// L1, US1 s9. The Default entry's Committed toggle is unavailable, with the note saying why.
#[test]
fn the_default_entry_has_no_committed_toggle() {
    let mut state = State::default();
    open(&mut state, SessionLocation::Default);
    let view = state.open.as_ref().unwrap();
    assert!(!committed_available(view));
    assert!(DEFAULT_ENTRY_NOTE.contains("only uncommitted changes are listed"));
    assert_eq!(
        changes::update(&mut state, Msg::CommittedToggled),
        Effect::None,
        "pressing the unavailable toggle does nothing"
    );

    let state = ready(&["a.rs"]);
    assert!(committed_available(state.open.as_ref().unwrap()));
}

/// L3, US1 s6. No rows under the toggles: "No changes against <base>".
#[test]
fn an_empty_list_says_there_are_no_changes_against_the_base() {
    let state = ready(&[]);
    let view = state.open.as_ref().unwrap();
    assert_eq!(
        list_body(view),
        ListBody::Empty("No changes against main".into())
    );
    assert_eq!(
        base_line(view).as_deref(),
        Some("Compared with main at 0123456")
    );
}

/// L3, Edge "Both toggles off".
#[test]
fn both_toggles_off_says_both_are_hidden() {
    let mut state = ready(&["a.rs"]);
    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::CommittedToggled) else {
        panic!("re-read");
    };
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["a.rs"])),
        },
    );
    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::UncommittedToggled) else {
        panic!("re-read");
    };
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&[])),
        },
    );
    assert_eq!(
        list_body(state.open.as_ref().unwrap()),
        ListBody::Empty(BOTH_HIDDEN.into())
    );
}

/// L3, Edge "No base found": the header says why committed changes are not listed.
#[test]
fn a_worktree_without_a_base_says_why() {
    let mut state = State::default();
    let seq = open(&mut state, worktree());
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(ChangeList {
                files: vec![file("a.rs")],
                scope: ReviewScope::Worktree {
                    base: Base::Unavailable(BaseUnavailable::NoCommonHistory),
                },
            }),
        },
    );
    let view = state.open.as_ref().unwrap();
    let line = base_line(view).expect("a base line");
    assert!(line.contains("no history in common"), "{line}");
    assert!(line.contains("only uncommitted changes"), "{line}");
    assert!(matches!(list_body(view), ListBody::Files(files) if files.len() == 1));
}

/// A failed read shows git's message; a read under way with nothing yet shows loading.
#[test]
fn loading_and_failure_have_their_own_bodies() {
    let mut state = State::default();
    let seq = open(&mut state, worktree());
    assert_eq!(list_body(state.open.as_ref().unwrap()), ListBody::Loading);
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Err("fatal: not a git repository".into()),
        },
    );
    assert_eq!(
        list_body(state.open.as_ref().unwrap()),
        ListBody::Failed("fatal: not a git repository")
    );
}
