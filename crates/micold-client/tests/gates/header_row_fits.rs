//! The structured view's header is one 40 px row (feature 1793, FR-001, FR-010, FR-011, SC-001;
//! T021, T029).
//!
//! Above the first event the session area holds the tab strip and nothing else: the strip is
//! exactly `ROW_HEIGHT` tall and the pane area starts under it, with and without a background
//! task or a mode label (their slim row lives inside the transcript body, below the strip). The
//! strip's controls (restart, the splits and the `N hidden` chip) lie whole inside the strip at
//! both windows and themes, with no chip, with a chip, and with a chip counting past a thousand;
//! twelve tabs scroll in their region and leave the controls in place.
//!
//! Compiled into the `layout_snapshot` binary beside `session_area_fits`.

use crate::session_area_fits::{at, inside, overlap, records_of, rect, strip_parts, SESSION_AREA, SIDEBAR};
use crate::support::covered_states::{covered_states, structured_pane, transcript_pane_insights};
use crate::support::layout::{LayoutRecord, StateUnderTest, WINDOW};
use iced::{Rectangle, Size};
use micold_client::app::{State, MIN_WINDOW_SIZE};
use micold_client::features::session_strip::StripModel;
use micold_client::ui::session_area::ROW_HEIGHT;
use micold_core::theme::ThemePreference;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

fn each_window(mut check: impl FnMut(String, ThemePreference, Size)) {
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        for window in [MIN_WINDOW_SIZE, WINDOW] {
            check(format!("{theme:?} {window:?}"), theme, window);
        }
    }
}

fn records_for(mut state: State, theme: ThemePreference, window: Size) -> Vec<LayoutRecord> {
    state.settings.theme_pref = theme;
    state.sidebar.width = SIDEBAR;
    let under = StateUnderTest::new(state).panes(micold_core::pane_layout::PaneLayout::single());
    records_of(under, window)
}

/// The strip is `ROW_HEIGHT` tall at the top of the session area and the pane area starts directly
/// under it: one row above the first event, no title row, nothing between.
fn assert_one_row(what: &str, records: &[LayoutRecord]) {
    let area = rect(at(records, SESSION_AREA).expect("the session area is drawn"));
    let children: Vec<Rectangle> = records
        .iter()
        .filter(|r| r.path.len() == SESSION_AREA.len() + 1 && r.path.starts_with(SESSION_AREA))
        .map(rect)
        .collect();
    assert_eq!(
        children.len(),
        2,
        "{what}: the strip, then the pane area, no header row: {children:?}"
    );
    let [strip, panes] = [children[0], children[1]];
    assert!(
        (strip.height - ROW_HEIGHT).abs() <= TOLERANCE && (strip.y - area.y).abs() <= TOLERANCE,
        "{what}: the strip {strip:?} is the area's first {ROW_HEIGHT} px row (FR-001)"
    );
    assert!(
        (panes.y - (strip.y + strip.height)).abs() <= TOLERANCE,
        "{what}: the pane area {panes:?} starts under the strip {strip:?}"
    );
}

/// The strip's parts lie inside the strip row and the window, and none overlaps another.
fn assert_parts_fit(what: &str, records: &[LayoutRecord], window: Size) {
    let strip = rect(at(records, &[SESSION_AREA, &[0]].concat()).expect("the strip is drawn"));
    let screen = Rectangle::with_size(window);
    let parts = strip_parts(records);
    assert!(parts.len() >= 2, "{what}: the tab region and the controls: {parts:?}");
    for (name, part) in &parts {
        assert!(
            (part.width > 0.0 || name == "strip tab region")
                && inside(part, &strip)
                && inside(part, &screen),
            "{what}: the {name} {part:?} lies inside the strip {strip:?} and the window"
        );
    }
    for (i, (a_name, a)) in parts.iter().enumerate() {
        for (b_name, b) in &parts[i + 1..] {
            assert!(!overlap(a, b), "{what}: the {a_name} {a:?} overlaps the {b_name} {b:?}");
        }
    }
}

/// The recording of `insights` with `extra` events the view does not model, so the strip's chip
/// counts them (they are hidden while raw events are off).
fn hidden_events(extra: usize) -> State {
    let mut lines = crate::support::transcript::fixture_lines("subagent-turn");
    lines.extend((0..extra).map(|n| {
        serde_json::json!({"type": "unmodelled_event", "n": n, "session_id": "<SESSION_ID>"})
            .to_string()
    }));
    let (mut state, _) = structured_pane(&lines);
    state.settings.show_raw_events = false;
    state
}

fn covered(name: &str) -> State {
    let covered = covered_states()
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("the covered state {name} exists"));
    (covered.build)().state
}

/// FR-001, SC-001: one 40 px row above the first event with no background task (`pane`), with a
/// running one and an open subagent thread (`insights`), with a stopped terminal session
/// (restart) and with twelve tabs.
#[test]
fn one_forty_px_row_precedes_the_first_event() {
    let states: Vec<(&str, State)> = vec![
        ("no tasks", covered("session-transcript-pane")),
        ("a background task", transcript_pane_insights()),
        ("one tab", covered("session-area-one-tab")),
        ("twelve tabs", covered("session-area-twelve-tabs")),
    ];
    for (name, state) in states {
        each_window(|when, theme, window| {
            let what = format!("{name} {when}");
            let records = records_for(state.clone(), theme, window);
            assert_one_row(&what, &records);
            assert_parts_fit(&what, &records, window);
        });
    }
}

/// FR-010, FR-013: the `N hidden` chip is a strip control at the row's end at 0, a few and past a
/// thousand hidden events: the strip stays one row and every part lies inside it.
#[test]
fn the_hidden_chip_fits_at_none_a_few_and_past_a_thousand() {
    let mut chips = Vec::new();
    for extra in [0, 3, 1_200] {
        let state = hidden_events(extra);
        let chip = StripModel::of(&state).expect("a strip").hidden;
        if extra > 1_000 {
            let text = chip.as_ref().expect("a chip").text.clone();
            assert!(text.starts_with("12") || text.len() >= 10, "{text}: a count past 1000");
        }
        chips.push(chip.is_some());
        each_window(|when, theme, window| {
            let what = format!("{extra} extra events {when}");
            let records = records_for(state.clone(), theme, window);
            assert_one_row(&what, &records);
            assert_parts_fit(&what, &records, window);
        });
    }
    assert_eq!(
        chips,
        [true, true, true],
        "the recording alone hides events, so each state draws the chip (the premise)"
    );
}

/// FR-011: with the chip and twelve tabs the controls hold their place at the strip's end and the
/// tab region (which scrolls) is what gives way; the chip is the strip's last child.
#[test]
fn the_chip_stays_at_the_strips_end_beside_many_tabs() {
    let mut state = hidden_events(1_200);
    let id = state.session.active.expect("a session is shown");
    for _ in 0..11 {
        state
            .workspace
            .find_session_mut(id)
            .expect("the shown session")
            .1
            .open_shell_instance();
    }
    let model = StripModel::of(&state).expect("a strip");
    assert!(model.hidden.is_some(), "the premise: the chip is drawn");
    each_window(|when, theme, window| {
        let what = format!("many tabs and the chip {when}");
        let records = records_for(state.clone(), theme, window);
        assert_one_row(&what, &records);
        assert_parts_fit(&what, &records, window);
        let parts = strip_parts(&records);
        let tabs = parts[0].1;
        let chip = parts.last().expect("controls").1;
        let strip = rect(at(&records, &[SESSION_AREA, &[0]].concat()).expect("the strip"));
        assert!(
            tabs.x + tabs.width <= chip.x + TOLERANCE
                && (strip.x + strip.width - (chip.x + chip.width)) <= micold_core::tokens::spacing::SM + TOLERANCE,
            "{what}: the chip {chip:?} ends the strip {strip:?}, after the tab region {tabs:?}"
        );
    });
}
