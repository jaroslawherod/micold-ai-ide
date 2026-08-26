//! Every popover is registered, and registers as itself (feature 021, T026/T031 — FR-010,
//! contract R2).
//!
//! ## What this file used to hold, and why it changed
//!
//! T026 wrote it against the *dismissal paths*: a table of the seven popovers and what each of
//! `open_overlay` and `dismiss_on_scroll_beneath` did with each, so that every combination was a
//! decision someone had written down rather than an omission nobody noticed. Four of seven and six
//! of seven, two different subsets, nothing checking either.
//!
//! T031 removed the subject. Both paths now ask the registry which surfaces are open and close
//! them, so there is no list to have forgotten a popover from — which is what T026's own closing
//! note said would happen: *"the lists below empty out and the subject becomes registration
//! itself — the obligation is unchanged, so the file stays."*
//!
//! ## The obligation, restated over the registry
//!
//! R2: *an unregistered surface fails the build or a guard test, never discovered by hand at
//! runtime*. For the nine dialogs the compiler used to hold it — they were matches over the closed
//! `Overlay` enum, verified at T026 by removing an arm three ways, each of which failed to
//! compile. The popovers were loose fields with no such protection, so this is where R2 lived:
//!
//! - every popover-shaped field on `State` has a registration (add an eighth, and this fails);
//! - setting one opens **exactly** its own surface and no other (a copy-pasted `open_in` reading
//!   its neighbour's field fails here rather than as a menu that will not close);
//! - every registered popover declares what closes it (the registry's collective closes are
//!   bounded loops; a surface with no cancellation would silently survive one).
//!
//! ## T037 deleted the enum, so the compiler's half went with it
//!
//! Every dialog is now a loose field too. The exposure is not the same, though, and saying so is
//! the honest version of this note rather than a claim that nothing changed. A popover's panel is
//! pushed by `ui::view` from its flag whether or not the registry has heard of it, so an
//! unregistered popover **opens and cannot be closed** — a live bug. A dialog since T035 is drawn
//! only *through* its registration, so an unregistered one is not drawn at all: still wrong, but
//! it fails loudly the first time anyone opens it rather than trapping the user behind a surface
//! with no exit.
//!
//! What T037 does add here is a classification duty. The scan below finds popover-*shaped* fields
//! by name, and `about_open` — the flag About gained when it lost its enum variant — has exactly
//! that shape while being a dialog. So a field matching the shape must now appear in one list or
//! the other, and [`DIALOG_FLAGS`] asserts its own members really do register in the dialog band.
//! A new `_open` field belonging to neither still fails the scan, which is the property that
//! matters.

use micold_client::app::State;
use micold_client::features::project::ProjectMenu;
use micold_client::overlay::registry;
use micold_core::session::{SessionId, SessionLocation, ShellInstanceId};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// A popover: the `State` field that carries it, the id it registers as, and how to open it.
///
/// The setter is a closure because Rust cannot name a field generically — and it earns its place:
/// opening exactly one surface is what makes "registers as *itself*" testable at all.
#[allow(clippy::type_complexity)]
const POPOVERS: &[(&str, &str, fn(&mut State))] = &[
    ("help_menu_open", "help_menu", |s| s.help_menu_open = true),
    ("project_switcher_open", "project_switcher", |s| {
        s.project_switcher_open = true
    }),
    ("sidebar_filter_open", "sidebar_filter", |s| {
        s.sidebar_filter_open = true
    }),
    ("project_menu_open", "project_menu", |s| {
        s.project_menu_open = Some(ProjectMenu {
            path: PathBuf::from("/tmp/p"),
            anchor: (10, 10),
        })
    }),
    ("worktree_menu_open", "worktree_menu", |s| {
        s.worktree_menu_open = Some(micold_client::features::worktree::WorktreeMenu {
            dir_name: "feature-x".to_string(),
            anchor: (120, 300),
        })
    }),
    ("session_menu_open", "session_menu", |s| {
        s.session_menu_open = Some(micold_client::features::session::SessionMenu {
            id: SessionId::new(),
            anchor: (120, 340),
        })
    }),
    ("terminal_context_menu", "terminal_context_menu", |s| {
        s.terminal_context_menu = Some((4, 2))
    }),
    // The terminal *tab's* menu, not the terminal's own (feature 012, BUG-005). Two context menus
    // on the same screen, on different things: this one acts on an instance, that one on the
    // content. It carries the instance because the menu belongs to the tab it was opened on rather
    // than to the active one — FR-010a is about restarting an instance you have not selected.
    ("shell_instance_menu", "shell_instance_menu", |s| {
        s.shell_instance_menu = Some((
            micold_client::ui::terminal::StripTab::Instance(ShellInstanceId(1)),
            4,
            2,
        ))
    }),
    // Feature 026: the "start a session on…" list, opened from a row's chevron. A menu rather than
    // a dialog — summoned from a control, anchored to the sidebar, dismissed by clicking away —
    // so it registers in the context-menu band with the other two.
    ("session_start_menu", "session_start_menu", |s| {
        s.session_start_menu = Some(micold_client::features::session::StartMenu {
            location: SessionLocation::Default,
            anchor: (4, 2),
        })
    }),
];

/// Popover-*shaped* fields that are actually a dialog's own state, with the dialog each opens.
///
/// One member, and it is here rather than exempted by name: `about_open` looks exactly like a
/// popover flag to the scan below, and the way to keep the scan honest is to say what it is, not
/// to teach the scan to skip it. `a_dialog_flag_registers_in_the_dialog_band` then holds this list
/// to being true.
#[allow(clippy::type_complexity)]
const DIALOG_FLAGS: &[(&str, &str, fn(&mut State))] =
    &[("about_open", "about", |s| s.about_open = true)];

/// Popover-shaped fields actually declared on `State`, so the lists above cannot go stale.
fn declared_popovers() -> BTreeSet<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app.rs");
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let at = src.find("pub struct State {").expect("State has moved");
    let rest = &src[at..];
    let end = rest.find("\n}").expect("unterminated struct");
    rest[..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let name = line.strip_prefix("pub ")?.split(':').next()?;
            (name.ends_with("_open") || name.contains("_menu")).then(|| name.to_string())
        })
        .collect()
}

fn opened(setter: fn(&mut State)) -> State {
    let mut state = State::default();
    setter(&mut state);
    state
}

#[test]
fn the_list_of_popovers_matches_the_ones_that_exist() {
    let declared = declared_popovers();
    let listed: BTreeSet<String> = POPOVERS
        .iter()
        .chain(DIALOG_FLAGS.iter())
        .map(|(f, ..)| f.to_string())
        .collect();

    let unlisted: Vec<_> = declared.difference(&listed).collect();
    let phantom: Vec<_> = listed.difference(&declared).collect();

    assert!(
        unlisted.is_empty(),
        "a new popover-shaped field exists that this file has never asked about: {unlisted:?}\n\n\
         Add it to POPOVERS with the surface it registers as, or to DIALOG_FLAGS if it is a \
         dialog's own state wearing a popover's name. That is the whole point — a popover nobody \
         registered is one that opens and will not close (FR-010, contract R2)."
    );
    assert!(
        phantom.is_empty(),
        "this file names fields `State` no longer has: {phantom:?}\n\nIf the field moved into its \
         feature module, follow it here in the same commit."
    );
}

#[test]
fn a_dialog_flag_registers_in_the_dialog_band() {
    // Keeps DIALOG_FLAGS from being a way to smuggle a field past the scan: a member claiming to
    // be a dialog has to actually open one, in the modal band, under the name it gives.
    let mut wrong = Vec::new();
    for (field, id, open) in DIALOG_FLAGS {
        let state = opened(*open);
        match registry::open_dialog(&state) {
            Some(dialog) if dialog.id().as_str() == *id => {}
            Some(dialog) => wrong.push(format!(
                "`{field}` opens `{}`, not the `{id}` it is listed as",
                dialog.id()
            )),
            None => wrong.push(format!(
                "`{field}` is listed as the dialog `{id}`, but setting it opens no dialog — so it \
                 is either an unregistered popover or nothing at all"
            )),
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n  - "));
}

#[test]
fn every_popover_is_registered() {
    let mut missing = Vec::new();
    for (field, id, open) in POPOVERS {
        let state = opened(*open);
        if registry::topmost(&state).is_none() {
            missing.push(format!(
                "`{field}` is set, and the registry reports nothing open — the surface `{id}` is \
                 not registered, or its `open_in` reads a different field"
            ));
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n  - "));
}

#[test]
fn a_popover_registers_as_itself_and_opens_nothing_else() {
    let mut wrong = Vec::new();
    for (field, id, open) in POPOVERS {
        let state = opened(*open);
        let ids: Vec<&str> = registry::open_popovers(&state)
            .iter()
            .map(|s| s.id().as_str())
            .collect();

        if ids != [*id] {
            wrong.push(format!(
                "setting `{field}` opened {ids:?}, expected exactly [\"{id}\"] — an `open_in` \
                 reading its neighbour's field is a menu that closes when the wrong thing is \
                 dismissed, and nothing else would notice"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n  - "));
}

#[test]
fn every_registered_popover_can_be_closed() {
    // The registry's collective closes (`close_popovers`, `close_on_scroll_beneath`) are bounded
    // loops that send each open surface its own cancellation. A surface that declared none would
    // not close, and the loop would give up rather than hang — quietly, which is the failure this
    // catches.
    let mut mute = Vec::new();
    for (field, id, open) in POPOVERS {
        let state = opened(*open);
        let Some(surface) = registry::open_popovers(&state).into_iter().next() else {
            continue; // already reported by `every_popover_is_registered`
        };
        if surface.cancel().is_none() {
            mute.push(format!(
                "`{field}` registers as `{id}` but names no cancellation"
            ));
        }
    }
    assert!(mute.is_empty(), "{}", mute.join("\n  - "));
}

#[test]
fn closing_the_popovers_closes_all_of_them_whatever_order_they_are_in() {
    // Several cancellations are *toggles*, and their reducer arms close their neighbours too. A
    // batch collected up front would hand a toggle to a surface an earlier message had already
    // closed and reopen it, so the registry re-asks after each. This is that property.
    let mut state = State::default();
    for (_, _, open) in POPOVERS {
        open(&mut state);
    }
    assert_eq!(
        registry::open_popovers(&state).len(),
        POPOVERS.len(),
        "precondition: every popover is open, so closing them has something to do"
    );

    registry::close_popovers(&mut state);

    let left: Vec<&str> = registry::open_popovers(&state)
        .iter()
        .map(|s| s.id().as_str())
        .collect();
    assert!(
        left.is_empty(),
        "these survived a collective close: {left:?} — a toggle sent to an already-closed surface \
         reopens it, which is why the registry re-asks rather than sending a batch"
    );
}

#[test]
fn the_guard_is_actually_looking_at_something() {
    assert!(
        !declared_popovers().is_empty(),
        "no popover-shaped fields found on State — the parser has stopped matching its shape, and \
         a guard iterating an empty list passes vacuously"
    );
    assert_eq!(
        POPOVERS.len(),
        9,
        "feature 021's seven FR-007 names, plus the terminal tab menu (`012` BUG-005, FR-010b) and \
         feature 026's `session_start_menu`. A shorter list is a guard that has stopped covering \
         them; growing it is what adding a popover looks like, and the number moves in the same \
         diff as the entry."
    );
}
