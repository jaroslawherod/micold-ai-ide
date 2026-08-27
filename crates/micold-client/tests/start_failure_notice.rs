//! Feature 026, T088 (FR-010) — how often the reason a start failed is said.
//!
//! `micold-daemon/tests/catalog_join.rs` holds the join: the daemon's own sentence, taken from the
//! snapshot it would really publish, reaching the client as something a user can read. What it
//! cannot drive is the other half of the rule, because its whole point is a CLI that stays missing:
//! a failure is announced **once**, and announced **again** after the session has been anything
//! else in between.
//!
//! Both halves matter and they pull in opposite directions. `reconcile_catalog` runs on every
//! `CatalogChanged` — and since T086 an activity badge moving is one of those — so a banner pushed
//! per snapshot would be a new one every few seconds for as long as the session stayed failed. But
//! a record that is never cleared silences the *next* failure, which is a real one the user has not
//! been told about.

use std::path::PathBuf;

use micold_client::app::State;
use micold_client::catalog_sync::reconcile_catalog;
use micold_core::protocol::messages::{
    ActivitySignal, CatalogSnapshot, ProjectSnapshot, SessionSummary, WireLifecycle,
};
use micold_core::session::{AiCli, SessionId, SessionLabel};
use uuid::Uuid;

const REASON: &str =
    "GitHub Copilot isn't installed. Install it, or start this session on another AI CLI.";

fn id() -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x0FF0))
}

/// A snapshot carrying one session in `lifecycle`.
fn snapshot(lifecycle: WireLifecycle) -> CatalogSnapshot {
    CatalogSnapshot {
        schema_version: 1,
        last_active: Some(PathBuf::from("/a")),
        projects: vec![ProjectSnapshot {
            path: PathBuf::from("/a"),
            display_name: "a".into(),
            is_git_repo: true,
            available: true,
            worktrees: Vec::new(),
            sessions: vec![SessionSummary {
                id: id(),
                worktree_dir: None,
                title: SessionLabel::Named("Refactor the parser".into()),
                lifecycle,
                activity: ActivitySignal::Unknown,
                provider: AiCli::Copilot,
                input_serial: 0,
                live_shells: Vec::new(),
            }],
        }],
    }
}

fn failed() -> WireLifecycle {
    WireLifecycle::Failed {
        reason: REASON.to_string(),
        attempts: 0,
    }
}

#[test]
fn an_unchanged_failure_is_said_once_however_many_snapshots_carry_it() {
    let mut core = State::default();

    reconcile_catalog(&mut core, &snapshot(failed()), false);
    assert_eq!(
        core.notifications
            .queue
            .visible()
            .map(|n| n.message.clone()),
        Some(REASON.to_string()),
        "the first report of a failure is news"
    );

    // Dismissed the way a user dismisses it — otherwise the queue's own duplicate suppression
    // (`Queue::push` drops a notification equal to the visible one) would carry this assertion,
    // and the record under test would never be exercised.
    core.notifications.queue.dismiss();
    for _ in 0..5 {
        reconcile_catalog(&mut core, &snapshot(failed()), false);
    }
    assert_eq!(
        core.notifications.queue.visible(),
        None,
        "and every snapshot after it is the same fact, not a new one — a badge moving publishes a \
         catalog (T086), so a banner per snapshot would be one every few seconds for as long as \
         the session stayed failed"
    );
}

#[test]
fn a_failure_after_the_session_has_been_something_else_is_said_again() {
    let mut core = State::default();

    reconcile_catalog(&mut core, &snapshot(failed()), false);
    core.notifications.queue.dismiss();

    // The user installs the CLI and the session comes up.
    reconcile_catalog(&mut core, &snapshot(WireLifecycle::Running), false);
    assert!(
        !core.session.announced_start_failures.contains_key(&id()),
        "a session that is no longer failed has no failure outstanding to have been reported"
    );

    // …and later it fails again, for the same reason.
    reconcile_catalog(&mut core, &snapshot(failed()), false);
    assert_eq!(
        core.notifications.queue.visible().map(|n| n.message.clone()),
        Some(REASON.to_string()),
        "this is a second failure and the user has not been told about it — suppressing it because \
         the sentence matches the last one would silence a real report"
    );
}

#[test]
fn a_different_reason_for_the_same_session_is_news() {
    let mut core = State::default();

    reconcile_catalog(&mut core, &snapshot(failed()), false);
    core.notifications.queue.dismiss();

    let gone =
        "GitHub Copilot no longer has this conversation. Close this session, or start a new one.";
    reconcile_catalog(
        &mut core,
        &snapshot(WireLifecycle::Failed {
            reason: gone.to_string(),
            attempts: 0,
        }),
        false,
    );
    assert_eq!(
        core.notifications.queue.visible().map(|n| n.message.clone()),
        Some(gone.to_string()),
        "the CLI going missing and the conversation going missing are two different things to fix, \
         and the second must not be swallowed because the first was already said"
    );
}

/// A `Failed` with an empty reason. Since `010` BUG-017 the domain variant carries a sentence and
/// the crash-loop give-up fills it, so nothing in the daemon produces this today — but the wire
/// type permits it, and an empty banner says less than the bar's `failed` already does.
#[test]
fn a_failure_with_nothing_to_say_says_nothing() {
    let mut core = State::default();

    reconcile_catalog(
        &mut core,
        &snapshot(WireLifecycle::Failed {
            reason: String::new(),
            attempts: 3,
        }),
        false,
    );
    assert_eq!(core.notifications.queue.visible(), None);
}
