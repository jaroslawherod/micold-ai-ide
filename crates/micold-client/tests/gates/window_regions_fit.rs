//! Gate W: the whole window's regions fit against each other (feature 1128, T025 — contract
//! covered-states §Gate W; US3 scenarios 2–3, SC-004, FR-001).
//!
//! Every region's own covered states hold that region; only a whole-window state holds the regions
//! against each other. The fixture catches a change, never a defect older than it, so SC-004's
//! numbers are asserted here on the records of `window-session-view` and `window-overview`, in both
//! colour schemes:
//!
//! - **W1** the title bar spans the window's top and is `app_bar::HEIGHT` tall (the bar, not the
//!   container that also holds its hairline);
//! - **W2** the status bar spans the window, `status_bar::HEIGHT` tall, ending at its bottom edge;
//! - **W3** the body spans the window's width, starts at the title bar's bottom edge,
//!   `app_bar::BOTTOM_EDGE` (the bar plus its hairline, so 0 px between them), and ends where the
//!   status bar starts;
//! - **W4** the sidebar and the session area (or the Overview in its place) each span the body from
//!   top to bottom, the sidebar from its left edge and the session area to its right, and meet;
//! - **W5** `window-overview`'s `State` has the Overview open, not full screen, and its sidebar row
//!   the selected one (the records hold geometry, not style).
//!
//! Each anchor these read is held by one of them: that is its "property only its own element
//! satisfies" (`docs/development/layout-snapshot.md#anchors`).
//!
//! **Compiled into the `layout_snapshot` binary**, so it reads the records `cached_records` already
//! resolved for the fixture.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, CoveredState, LayoutRecord};
use micold_core::theme::ColorScheme;
use micold_core::tokens::anatomy::{app_bar, status_bar};

/// The whole-window covered states, one per mockup board.
const WINDOW_STATES: [&str; 2] = ["window-session-view", "window-overview"];

/// Float noise only: SC-005's 1 px move must fail.
const EPSILON: f32 = 0.01;

/// One whole-window state's records in one scheme.
struct Window {
    covered: &'static CoveredState,
    scheme: ColorScheme,
    records: &'static [LayoutRecord],
}

impl Window {
    /// The base-layer record the anchor `name` points at.
    fn region(&self, name: &str) -> &LayoutRecord {
        lay::anchored(self.covered, self.records, name).unwrap_or_else(|| {
            let declared = self.covered.anchors.iter().find(|a| a.name == name);
            panic!(
                "{} ({:?}): anchor {name:?} names no record (declared at {:?})",
                self.covered.name,
                self.scheme,
                declared.map(|a| a.path)
            )
        })
    }

    /// Fails naming the state, the scheme, what is measured and both numbers.
    fn expect(&self, what: &str, got: f32, want: f32) {
        assert!(
            (got - want).abs() <= EPSILON,
            "{} ({:?}): {what} is {got}, want {want}",
            self.covered.name,
            self.scheme
        );
    }
}

/// Each whole-window state in each scheme.
fn windows() -> Vec<Window> {
    let renderer = lay::renderer();
    let states = covered_states();
    let mut out = Vec::new();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let all = lay::cached_records(states, &renderer, scheme);
        for name in WINDOW_STATES {
            let index = states
                .iter()
                .position(|s| s.name == name)
                .unwrap_or_else(|| panic!("no covered state {name:?}"));
            out.push(Window {
                covered: &states[index],
                scheme,
                records: &all[index],
            });
        }
    }
    out
}

#[test]
fn w1_the_title_bar_is_at_the_top_and_its_own_height() {
    for w in windows() {
        let bar = w.region("title_bar");
        w.expect("title_bar top", bar.y, 0.0);
        w.expect("title_bar height", bar.height, app_bar::HEIGHT);
        w.expect("title_bar left", bar.x, 0.0);
        w.expect("title_bar width", bar.width, lay::WINDOW.width);
    }
}

#[test]
fn w2_the_status_bar_is_its_own_height_at_the_windows_bottom() {
    for w in windows() {
        let bar = w.region("status_bar");
        w.expect("status_bar height", bar.height, status_bar::HEIGHT);
        w.expect("status_bar bottom", bar.y + bar.height, lay::WINDOW.height);
        w.expect("status_bar left", bar.x, 0.0);
        w.expect("status_bar width", bar.width, lay::WINDOW.width);
    }
}

#[test]
fn w3_the_body_fills_from_the_title_bars_bottom_edge_to_the_status_bar() {
    for w in windows() {
        let (title, body, status) = (
            w.region("title_bar"),
            w.region("shell.body"),
            w.region("status_bar"),
        );
        w.expect(
            "shell.body top against title_bar's bottom edge",
            body.y,
            title.y + app_bar::BOTTOM_EDGE,
        );
        w.expect(
            "shell.body bottom against status_bar top",
            body.y + body.height,
            status.y,
        );
        w.expect("shell.body left", body.x, 0.0);
        w.expect("shell.body width", body.width, lay::WINDOW.width);
    }
}

#[test]
fn w4_the_sidebar_and_the_session_area_span_the_body_and_meet() {
    for w in windows() {
        let (body, sidebar, area) = (
            w.region("shell.body"),
            w.region("sidebar"),
            w.region("session.area"),
        );
        for (name, region) in [("sidebar", sidebar), ("session.area", area)] {
            w.expect(
                &format!("{name} top against shell.body's"),
                region.y,
                body.y,
            );
            w.expect(
                &format!("{name} height against shell.body's"),
                region.height,
                body.height,
            );
        }
        w.expect("sidebar left against shell.body's", sidebar.x, body.x);
        w.expect(
            "sidebar right edge against session.area's left",
            sidebar.x + sidebar.width,
            area.x,
        );
        w.expect(
            "session.area right edge against shell.body's",
            area.x + area.width,
            body.x + body.width,
        );
    }
}

#[test]
fn w5_window_overview_opens_the_overview_and_selects_its_row() {
    let covered = covered_states()
        .iter()
        .find(|s| s.name == "window-overview")
        .expect("window-overview is a covered state");
    let state = (covered.build)().state;
    // The Overview row has no selection field of its own: `ui::sidebar` draws it selected exactly
    // while the Overview is open with a project active, and every other row unselected then
    // (`features::sidebar::row_selected`). So these fields are its selection.
    assert!(
        state.overview.open && !state.overview.full_screen,
        "window-overview: the Overview is open beside the sidebar (open {}, full screen {})",
        state.overview.open,
        state.overview.full_screen
    );
    assert!(
        state.workspace.active.is_some(),
        "window-overview: a project is open, so the sidebar draws its Overview row"
    );
}
