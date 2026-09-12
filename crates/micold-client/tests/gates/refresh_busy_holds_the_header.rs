//! A refresh in flight does not move the sidebar header (feature 029, FR-001/FR-006, T032).
//!
//! Feature 019's rule is that a screen worth covering gets an entry in `covered_states.rs` and a
//! block of its own in `layout_snapshot.txt`. This feature adds a state — the header with a refresh
//! running — and the honest answer for it is *not* a fixture entry, because a fixture entry that
//! records the same bytes as its neighbour is worse than none: it doubles what a regeneration has
//! to be read against while asserting nothing the neighbour does not already assert. Feature 019
//! met that shape twice and said so both times, in the comments above `error-add-worktree-failed`
//! and `error-project-unavailable` — two states that *were* byte-identical to
//! `main-shell-sidebar-expanded` until they were built differently.
//!
//! The difference here is that the identity is the **intended** result rather than a mistake to
//! correct. `refreshing` withholds the control's `on_press` and swaps its tooltip string; neither is
//! geometry. An iced `Button` lays out the same whether or not it can be pressed, and a `Tooltip`'s
//! own text is laid out through the overlay pass on hover and never at rest. So the busy header is
//! the idle header, to the pixel, by construction.
//!
//! "By construction" is exactly the kind of claim that stops being true quietly. If someone later
//! answers FR-006 with a spinner, a swapped glyph, or a label beside the icon — all reasonable
//! things to reach for, and research R6 left the door open to the first — the geometry moves, and a
//! comment in `covered_states.rs` saying it does not would then be a lie with nothing to catch it.
//! This is that comment written as a test: it fails on the day the busy header stops being
//! layout-identical, and its failure says what to do about it (register the state).
//!
//! # It registers nothing
//!
//! Deliberately, and it must stay that way: `layout_coverage_registry.rs` fails on a `CoveredState`
//! constructed anywhere but the registry, and it is right to — a state built inline is invisible to
//! the fixture and to every gate that walks it, which is coverage-shaped and not coverage. So this
//! reads the registered state through its own `build` and renders it directly, the way
//! `context_menu_anchor` renders the states it drives.
//!
//! **Compiled into the `layout_snapshot` binary** for the reason `containment` gives: cargo makes
//! one process per file directly under `tests/`, and a separate process cannot reach the record
//! cache that has already resolved every covered state. Half of what this gate resolves is that
//! cache's `main-shell-sidebar-expanded`.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_client::app::State;
use micold_client::features::connection::ConnectionStatus;
use micold_core::theme::{ColorScheme, ThemePreference};

/// Geometry is a structural property; one scheme establishes it. The dark pass exists for colour.
const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;

/// The registered state this one is the busy form of — the sidebar shown, a project open.
const EXPANDED: &str = "main-shell-sidebar-expanded";

/// The idle header, exactly as the fixture records it. Read from the registry rather than rebuilt,
/// so the two sides of the comparison cannot drift into being two different screens.
fn idle_state() -> State {
    let covered = covered_states()
        .iter()
        .find(|s| s.name == EXPANDED)
        .unwrap_or_else(|| panic!("no covered state named `{EXPANDED}` — this gate reads it"));
    let mut state = (covered.build)().state;
    state.settings.theme_pref = match RECORDED_SCHEME {
        ColorScheme::Light => ThemePreference::Light,
        ColorScheme::Dark => ThemePreference::Dark,
    };
    state
}

/// The same state with a refresh in flight — the one field this feature added.
fn busy_state() -> State {
    let mut state = idle_state();
    state.worktree.refreshing = true;
    state
}

/// Render `state` and resolve its geometry, the way `records_for` does for a registered state.
fn records(state: &State) -> Vec<LayoutRecord> {
    let renderer = lay::renderer();
    let element = micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &micold_core::env_include::EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    );
    lay::resolve(element, &renderer)
}

/// The precondition, so the comparison below cannot pass vacuously over a flag nothing reads.
#[test]
fn the_busy_state_really_is_a_different_state() {
    assert!(
        idle_state().can_refresh_worktrees(),
        "the registered `{EXPANDED}` state has a project open, so its refresh control is offerable \
         — if it is not, this gate is comparing two headers that are both inert and proves nothing"
    );
    assert!(
        !busy_state().can_refresh_worktrees(),
        "setting `worktree.refreshing` must make the control non-offerable (FR-006), or the busy \
         state under test is not busy"
    );
}

#[test]
fn a_refresh_in_flight_moves_nothing_in_the_header() {
    let idle = records(&idle_state());
    let running = records(&busy_state());

    assert_eq!(
        idle.len(),
        running.len(),
        "the busy header lays out a different number of elements than the idle one, so FR-006 is \
         now answered with geometry. Register `main-shell-sidebar-expanded-refreshing` in \
         tests/support/covered_states.rs and regenerate the fixture — and delete this gate, whose \
         whole claim was that there was nothing to register."
    );

    let moved: Vec<String> = idle
        .iter()
        .zip(&running)
        .filter(|(a, b)| a != b)
        .map(|(a, b)| {
            format!(
                "\n  idle:    {}\n  running: {}",
                lay::format_record(a),
                lay::format_record(b)
            )
        })
        .collect();

    assert!(
        moved.is_empty(),
        "a refresh in flight moved {} element(s). The busy header is no longer layout-identical to \
         the idle one, so it needs a fixture entry of its own — register it in \
         tests/support/covered_states.rs and delete this gate.{}",
        moved.len(),
        moved.join("")
    );
}
