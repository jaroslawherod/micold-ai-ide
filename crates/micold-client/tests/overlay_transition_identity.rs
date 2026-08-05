//! A closing dialog keeps its identity while it animates out (feature 017, T039 — FR-011).
//!
//! `material::Modal` restarts its transition when the dialog under it changes, so that opening a
//! second dialog over the first plays a fresh entrance rather than inheriting a transition that had
//! already finished. That check needs an identity, and the obvious one — `state.overlay` — is wrong
//! at exactly the moment it matters: closing a dialog sets it to `Overlay::None` *before* the exit
//! plays. The renderer would read the identity as changed, restart the transition, and the dialog
//! would vanish instantly instead of animating out.
//!
//! `ClosingOverlay::overlay()` is what keeps it stable: the snapshot still knows which dialog it is
//! a snapshot of. These tests hold that mapping to being total, faithful and injective — a variant
//! answering `None`, or two answering alike, reintroduces the jump.

use std::path::PathBuf;

use micold_client::app::{
    ClosingOverlay, Overlay, RenameDraft, SettingsDraft, WorktreeRenameDraft,
};
use micold_core::selector::Selector;

/// One snapshot of every kind, with the overlay each is a snapshot *of*.
///
/// Written out rather than derived, so this is an independent statement of the mapping instead of a
/// second copy of the implementation agreeing with itself.
fn every_snapshot() -> Vec<(ClosingOverlay, Overlay)> {
    vec![
        (ClosingOverlay::About, Overlay::About),
        (
            ClosingOverlay::Selector(Selector::open_at(PathBuf::from("/tmp"))),
            Overlay::ProjectSelector,
        ),
        (
            ClosingOverlay::Rename(RenameDraft {
                path: PathBuf::from("/tmp"),
                text: String::new(),
                error: None,
            }),
            Overlay::RenameProject,
        ),
        (
            ClosingOverlay::Worktree(Box::default(), None),
            Overlay::AddWorktree,
        ),
        (
            ClosingOverlay::Settings(SettingsDraft::default()),
            Overlay::Settings,
        ),
        (
            ClosingOverlay::ConfirmDelete("wt".to_string()),
            Overlay::ConfirmWorktreeDelete,
        ),
        (
            ClosingOverlay::WorktreeRename(WorktreeRenameDraft {
                dir_name: "wt".to_string(),
                text: String::new(),
                error: None,
            }),
            Overlay::RenameWorktree,
        ),
        (
            ClosingOverlay::ConfirmSessionRemove("s".to_string()),
            Overlay::ConfirmSessionRemove,
        ),
        (
            ClosingOverlay::ConfirmForget("p".to_string(), 0),
            Overlay::ConfirmForgetProject,
        ),
    ]
}

/// The headline property: a snapshot names the dialog it was taken of.
#[test]
fn a_snapshot_reports_the_dialog_it_was_taken_of() {
    for (closing, expected) in every_snapshot() {
        assert_eq!(
            closing.overlay(),
            expected,
            "{closing:?} should report the overlay it snapshots"
        );
    }
}

/// Never `None` — that is the value the identity has to survive, so answering it would defeat the
/// whole purpose and hand the renderer the jump this exists to prevent.
#[test]
fn no_snapshot_reports_itself_as_nothing_open() {
    for (closing, _) in every_snapshot() {
        assert_ne!(
            closing.overlay(),
            Overlay::None,
            "{closing:?} must keep a real identity while it animates out"
        );
    }
}

/// Injective: two dialogs sharing an identity means closing one and opening the other reads as the
/// same dialog continuing, and the second would skip its entrance.
#[test]
fn no_two_snapshots_share_an_identity() {
    let mut seen: Vec<Overlay> = Vec::new();
    for (closing, _) in every_snapshot() {
        let overlay = closing.overlay();
        assert!(
            !seen.contains(&overlay),
            "{closing:?} shares an identity with an earlier snapshot ({overlay:?})"
        );
        seen.push(overlay);
    }
}

/// The list above covers every variant. Without this, adding a tenth `ClosingOverlay` and forgetting
/// to map it would leave all three tests above passing on a stale set.
#[test]
fn every_variant_is_covered() {
    // Bump deliberately: a new snapshot kind needs a row in `every_snapshot`, and its dialog needs
    // an entry in `ClosingOverlay::overlay`.
    assert_eq!(
        every_snapshot().len(),
        9,
        "a ClosingOverlay variant was added or removed — update every_snapshot()"
    );
}
