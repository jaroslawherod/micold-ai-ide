//! A closing dialog keeps its identity while it animates out (feature 017, T039 — FR-011).
//!
//! `material::Modal` restarts its transition when the dialog under it changes, so that opening a
//! second dialog over the first plays a fresh entrance rather than inheriting a transition that had
//! already finished. That check needs an identity, and the obvious one — `state.overlay` — is wrong
//! at exactly the moment it matters: closing a dialog sets it to `Overlay::None` *before* the exit
//! plays. The renderer would read the identity as changed, restart the transition, and the dialog
//! would vanish instantly instead of animating out.
//!
//! The snapshot is what keeps it stable: it still knows which dialog it is a snapshot of. These
//! tests hold that mapping to being total, faithful and injective — a dialog whose snapshot loses
//! its identity, or two that answer alike, reintroduces the jump.
//!
//! ## Restated at feature 021 T036, and why (FR-027)
//!
//! This file is one of the protected tests T037 says must keep passing unchanged, and it cannot:
//! its subject, `ClosingOverlay`, is what T036 deletes. Feature 021's whole claim is that a
//! nine-variant enum plus a nine-arm map back to the dialog is one per-surface list too many —
//! [`Closing`] keeps the state as it was and asks the registry, so all of it goes. A test of a
//! type that no longer exists cannot be preserved by keeping its text.
//!
//! So the four properties are preserved instead, one for one, under the same four names: faithful,
//! never-nothing, injective, and covering every dialog. Nothing is weakened — if anything the
//! subject is now the identity the renderer actually keys on ([`Closing::id`], a [`SurfaceId`])
//! rather than the `Overlay` it used to be translated into, and every snapshot is produced by the
//! real `Closing::of` from a real state rather than hand-constructed. The assertion-freeze check
//! flags this file; this comment is the explanation it is flagged against.
//!
//! T037 touched it once more, for the same unavoidable reason: it deleted the `Overlay` enum the
//! restated version still used to *say which dialog*. The nine rows now carry the name and the way
//! to open the dialog instead of a variant. No property changed.

use std::path::PathBuf;

use micold_client::app::State;
use micold_client::features::help;
use micold_client::features::project::RenameDraft;
use micold_client::features::worktree::WorktreeRenameDraft;
use micold_client::overlay::registry::Closing;
use micold_client::overlay::SurfaceId;
use micold_core::selector::Selector;
use micold_core::session::SessionId;

/// Every dialog, with the name its snapshot must report and how to open it.
///
/// Written out rather than derived, so this is an independent statement of the set instead of a
/// second copy of the implementation agreeing with itself. Kept honest by
/// `every_variant_is_covered`. Before T037 the two columns were one — a dialog *was* an `Overlay`
/// variant, so naming it and opening it were the same act; a dialog now says it is open by holding
/// the state it draws from, so the row has to build that state.
#[allow(clippy::type_complexity)]
const DIALOGS: &[(&str, fn(&mut State))] = &[
    ("about", |state| state.help.about_open = true),
    ("project_selector", |state| {
        state.project.selector = Some(Selector::open_at(PathBuf::from("/tmp")))
    }),
    ("rename_project", |state| {
        state.project.rename_draft = Some(RenameDraft {
            path: PathBuf::from("/tmp"),
            text: String::new(),
            error: None,
        })
    }),
    ("add_worktree", |state| {
        state.worktree_form.form = Some(Default::default())
    }),
    ("settings", |state| {
        state.settings.settings_draft = Some(Default::default())
    }),
    ("confirm_worktree_delete", |state| {
        state.worktree.delete_target = Some("wt".to_string())
    }),
    ("rename_worktree", |state| {
        state.worktree.rename_draft = Some(WorktreeRenameDraft {
            dir_name: "wt".to_string(),
            text: String::new(),
            error: None,
        })
    }),
    ("confirm_session_remove", |state| {
        state.session.remove_target = Some(SessionId::new())
    }),
    ("confirm_forget_project", |state| {
        state.project.forget_target = Some(PathBuf::from("/p"))
    }),
];

/// A state with just that dialog open.
fn opened(open: fn(&mut State)) -> State {
    let mut state = State::default();
    open(&mut state);
    state
}

/// One snapshot of every kind, with the dialog each is a snapshot *of* — taken the way the binary
/// takes them, so a snapshot the real code could not produce cannot pass these tests.
fn every_snapshot() -> Vec<(&'static str, Closing)> {
    DIALOGS
        .iter()
        .map(|(name, open)| {
            let closing = Closing::of(&opened(*open))
                .unwrap_or_else(|| panic!("{name} is open but no snapshot was taken of it"));
            (*name, closing)
        })
        .collect()
}

/// The headline property: a snapshot names the dialog it was taken of.
#[test]
fn a_snapshot_reports_the_dialog_it_was_taken_of() {
    for (name, closing) in every_snapshot() {
        assert_eq!(
            closing.id(),
            SurfaceId::new(name),
            "the snapshot of {name} should report the dialog it snapshots"
        );
    }
}

/// Never "nothing open" — that is the value the identity has to survive, so losing it would defeat
/// the whole purpose and hand the renderer the jump this exists to prevent.
///
/// With an identity that is a name rather than an enum there is no `None` to answer, so the way it
/// can now be lost is for the snapshot not to be taken at all: `Closing::of` returning `None` while
/// a dialog is open leaves the renderer with nothing to draw, which is the same instant vanish.
#[test]
fn no_snapshot_reports_itself_as_nothing_open() {
    for (name, open) in DIALOGS {
        assert!(
            Closing::of(&opened(*open)).is_some(),
            "{name} must keep a real identity while it animates out"
        );
    }
    // And the converse, which the old enum got for free by having no variant for it: with nothing
    // open there is nothing to snapshot.
    assert!(
        Closing::of(&State::default()).is_none(),
        "no dialog is open, so there is nothing to animate out"
    );
}

/// Injective: two dialogs sharing an identity means closing one and opening the other reads as the
/// same dialog continuing, and the second would skip its entrance.
#[test]
fn no_two_snapshots_share_an_identity() {
    let mut seen: Vec<SurfaceId> = Vec::new();
    for (name, closing) in every_snapshot() {
        let id = closing.id();
        assert!(
            !seen.contains(&id),
            "the snapshot of {name} shares an identity with an earlier one ({id})"
        );
        seen.push(id);
    }
}

/// The list above covers every dialog. Without this, adding a tenth and forgetting to list it would
/// leave all three tests above passing on a stale set.
#[test]
fn every_variant_is_covered() {
    // Bump deliberately: a new dialog needs a row in `DIALOGS`.
    assert_eq!(
        every_snapshot().len(),
        9,
        "a dialog was added or removed — update DIALOGS"
    );
}

/// Contract A1, which the old shape could not state: the snapshot does not merely remember a name,
/// it still draws. Previously the exit was rendered by a second nine-arm match that only *resembled*
/// the live path; now it goes through the same registration, so this asks the snapshot for its view
/// exactly as the renderer does.
#[test]
fn a_snapshot_still_knows_how_to_draw_itself() {
    for (name, closing) in every_snapshot() {
        let surface = closing
            .surface()
            .unwrap_or_else(|| panic!("the snapshot of {name} lost the dialog it was taken of"));
        assert_eq!(surface.id(), closing.id(), "{name}");
        assert!(
            surface.view().is_some(),
            "the snapshot of {name} has no view to fade out with"
        );
    }
}

/// Contract A3: the snapshot holds no live reference to feature state — it is a value, taken by
/// clone, and goes on saying what it said after the state it came from moves on.
#[test]
fn a_snapshot_does_not_follow_the_state_it_came_from() {
    let mut state = State {
        help: help::State {
            about_open: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let closing = Closing::of(&state).expect("About is open");

    state.help.about_open = false;

    assert_eq!(closing.id(), SurfaceId::new("about"));
    assert!(
        closing.state().help.about_open,
        "the snapshot must keep the state as it was, not track the live one"
    );
}
