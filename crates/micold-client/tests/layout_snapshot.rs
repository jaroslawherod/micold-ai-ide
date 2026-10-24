//! Layout parity snapshot (feature 019, T014–T025).
//!
//! Feature 017 pinned every colour the application resolves, in both schemes, byte-for-byte. It
//! pinned nothing about *where* anything is, and said so: a long session name overlapping its close
//! button shipped, was found by a person looking at the running application, and could only be
//! closed by eye because the baseline it needed was never captured.
//!
//! This records the resolved geometry of every element in a curated set of states and asserts it
//! against a committed fixture. A failure names the element that moved.
//!
//! **What it does not cover**, stated here because a gate that is quietly narrower than it looks is
//! the exact failure this exists to correct: colour, border, radius and shadow (owned by
//! `style_snapshot`), rasterised pixels, and geometry that exists only mid-animation. See
//! `docs/development/layout-snapshot.md`.
//!
//! Two exclusions were lifted and are recorded because an out-of-date boundary is the same defect
//! as an unclear one, pointing the other way. **Typography is covered**: this measured a private
//! copy of Roboto until feature 018 shipped the typeface, and now measures the very files the
//! application draws with (`assets/fonts/`, all three faces). **Scroll position is covered at a
//! defined offset**: `main-shell-sidebar-scrolled-to-top` overflows the sidebar's list, so the
//! at-rest sampling point is asserted about a tree in which something actually scrolls.
//!
//! Regenerate deliberately, only when a layout change is *intended*:
//! `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot`
//!
//! `--test` selects this *target*. Without it the bare word is a **test-name filter**, and no test
//! in here is called `layout_snapshot` — every binary reports `0 passed; N filtered out`, the
//! command exits 0, and nothing is regenerated. `the_regenerate_hint_selects_this_target` in
//! `layout_snapshot_regeneration.rs` is what keeps the printed hint honest.

mod support;

use micold_core::theme::ColorScheme;
use support::layout::{self as lay};

const FIXTURE_PATH: &str = "tests/fixtures/layout_snapshot.txt";

/// The scheme the fixture records. The other is asserted byte-identical rather than duplicated
/// (FR-008a).
const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;

// --- The covered states (T019, T020) ----------------------------------------------------------

// Registered in exactly one place: `tests/support/covered_states.rs` (FR-016).
use support::covered_states::covered_states;

// --- The containment gate (BUG-001) -------------------------------------------------------------

// A separate gate sharing this binary's process, and therefore its record cache. Cargo builds one
// binary per file directly under `tests/`, so a file there cannot reach this one's `OnceLock` and
// would re-resolve all nine covered states — ~6s to recompute what is already in memory. See the
// module's own documentation for what it asserts.
#[path = "gates/containment.rs"]
mod containment;

// --- The panel-placement gate (BUG-003) ---------------------------------------------------------

// The same arrangement, for the same reason: a separate gate, sharing this binary's record cache.
// It asserts about two independent components — a panel and the app bar it hangs from — which is
// the one relationship none of the other gates is scoped to read.
#[path = "gates/panel_placement.rs"]
mod panel_placement;

// --- The sibling-parity gate (BUG-007) ----------------------------------------------------------

// The same arrangement again. It asserts about two components of the **same** kind — the app bar's
// two action controls, and the two panels that hang from it — which is the relationship a fork
// hides in: both copies are internally consistent, and the second answers to no contract row.
#[path = "gates/sibling_parity.rs"]
mod sibling_parity;

// --- The tab-children-fit gate (BUG-005) --------------------------------------------------------

// The same arrangement once more. It reads a laid-out child against **what it asked for**, which is
// the one question none of the five above puts: a control squeezed to nothing by a parent's fixed
// width still contains its children, escapes nothing, overflows nothing, and answers to a constant
// that is still correct in the source. The whole suite was green over a 0.0dp button.
#[path = "gates/tab_children_fit.rs"]
mod tab_children_fit;

// --- The rail-alignment gate (feature 027 §B.6) -------------------------------------------------

// The same arrangement again, and the first gate a visual pass produced rather than a bug report.
// It reads a *set* of same-kind components against each other the way `sibling_parity` does, but
// for the one set whose members differ only in which is current — a rail's destinations, collapsed
// to their icons, where "current" changes the button variant and therefore the inset.
#[path = "gates/rail_icons_align.rs"]
mod rail_icons_align;

/// Feature 026's FR-002c, one level out from `tab_children_fit`, carried to the strip above the panes
/// (feature 1126, S9): every member and fixed control of the strip is laid out at the width it asked
/// for, at every member count.
#[path = "gates/strip_controls_hold_their_size.rs"]
mod strip_controls_hold_their_size;

/// Feature 1126's FR-008, beside the gate above: that one asks whether a strip control kept its
/// **size**, this one whether it kept its **place** — New session and the split buttons are the
/// strip's last children, in order, against its trailing padding. It replaced feature 027's
/// `tabs_anchor_the_trailing_edge`, whose trailing-anchored tabs FR-008 superseded.
#[path = "gates/strip_controls_anchor_the_trailing_edge.rs"]
mod strip_controls_anchor_the_trailing_edge;

/// Feature 029's FR-006, and the only gate here that asserts geometry did **not** change. The
/// refresh control's busy form withholds a press and swaps a tooltip, neither of which is layout —
/// so it earns no fixture block, and this is the check that keeps that true.
#[path = "gates/refresh_busy_holds_the_header.rs"]
mod refresh_busy_holds_the_header;

// --- The context-menu-anchor gate (BUG-008) -----------------------------------------------------

// Here for a *different* reason than the five above: it builds its own states and reads no cache.
// It shares this binary so that it can reuse `panel_placement`'s rule for what counts as an
// anchored panel rather than restating it, which is the shape FR-029a forbids one level down.
#[path = "gates/context_menu_anchor.rs"]
mod context_menu_anchor;

// --- The container-section-fits gate (806, T046) ------------------------------------------------

// Builds its own states and reads no cache, like `context_menu_anchor`; it shares this binary for the
// `support::layout` passes.
#[path = "gates/container_section_fits.rs"]
mod container_section_fits;
#[path = "gates/daemon_form_fits.rs"]
mod daemon_form_fits;

// --- The tooltip-clears-its-row gate (029 BUG-001) ----------------------------------------------

// Beside `context_menu_anchor`, whose fixture it reuses: the same long worktree list and the same
// row paths, read this time against the tooltip a hover opens rather than the menu a press opens.
#[path = "gates/tooltip_clears_its_row.rs"]
mod tooltip_clears_its_row;

// --- The picker-row-tooltip-clears-its-row gate (038, FR-023) ------------------------------------

// Beside `tooltip_clears_its_row`, whose question it asks of another surface: a row of the issue
// picker's floating list, whose tooltip is an overlay floated by an overlay and opens only after
// the cursor has rested. It builds its own form and reads no cache, so the fixture does not change.
#[path = "gates/picker_row_tooltip_clears_its_row.rs"]
mod picker_row_tooltip_clears_its_row;

// --- The issue-rows-show-all-their-text gate (038, SC-001) ---------------------------------------

// Here because it reads covered states — the issue list's rows — and compares what they painted
// with the issues the state was built from. It draws them itself rather than reading the cache: the
// cache holds layout nodes, and whether a paragraph was painted is not a fact about a node.
#[path = "gates/issue_rows_show_all_text.rs"]
mod issue_rows_show_all_text;

// --- The notification-kind rows gate (613, T075) -------------------------------------------------

// Here because it reads a covered state, the Environment page, from the shared record cache, and
// states the arrangement of the rows feature 613 added rather than comparing with the record.
#[path = "gates/notification_kind_rows_sit_under_the_switch.rs"]
mod notification_kind_rows_sit_under_the_switch;

// --- The structured-row-marker gate (938, T040) --------------------------------------------------

// Beside `context_menu_anchor`, whose worktree fixture it reuses; it builds its own states.
#[path = "gates/structured_row_marker_fits.rs"]
mod structured_row_marker_fits;

// --- The disclosure card's header holds its parts (feature 939) ---------------------------------

// The showcase's tool-call card, laid out folded and open, failed and not: its leading icon, one-line
// summary and trailing status stay inside the header row.
#[path = "gates/disclosure_card_header_fits.rs"]
mod disclosure_card_header_fits;

// --- The transcript's rich components hold their parts (feature 940, T025) ---------------------

// The showcase's code block: its tag and copy control stay in its header row at a narrow width, and
// a long line is clipped to the block rather than widening it (FR-010).
#[path = "gates/code_block_fits.rs"]
mod code_block_fits;

// A vertical wheel over a code block scrolls the list that holds it, not the block (research R12).
#[path = "gates/code_block_wheel.rs"]
mod code_block_wheel;

// The showcase's inline diffs: every `+`/`−` marker inside its cell, both schemes, and no number
// gutter (FR-015, FR-016).
#[path = "gates/inline_diff_marker_fits.rs"]
mod inline_diff_marker_fits;

// --- The composer's shared parts hold their content (feature 941, T048) -------------------------

// The showcase's text area grows to its height limit and then holds it; a busy indicator draws its
// words busy or idle; a queue row is one line beside its position and count; a chip holds its name
// (FR-008, FR-010).
#[path = "gates/composer_parts_fit.rs"]
mod composer_parts_fit;

// --- The command palette fits the composer's narrowest width (#943, T055) ----------------------

// The showcase's palette at the narrowest pane: every group header, command and second line inside
// the panel, and a terminal-only row's "opens in terminal" whole (FR-007, FR-008, FR-010).
#[path = "gates/command_palette_fits.rs"]
mod command_palette_fits;

// --- A permission prompt's actions stay inside its card (feature 942, T057) -------------------

// The tool request, plan approval and question views, as the showcase poses them, at the narrowest
// pane width: every action of the row is drawn inside the card (FR-006, FR-007, FR-008).
#[path = "gates/prompt_card_fits.rs"]
mod prompt_card_fits;

// --- The composer under a structured transcript (feature 941, T055) -----------------------------

// The composer sits under a structured session's transcript and never under a terminal; grows to
// its limit; says busy or ready; its queue, hold, overdue, notice and chips fit; the #942 and #943
// slots take no room empty; an interrupted turn and a message's pictures are named.
#[path = "gates/composer_fits.rs"]
mod composer_fits;

// --- The transcript's jump control fits its viewport (feature 939, T038) ------------------------

// A structured session's pane, detached from the end: "Jump to latest" lies inside the
// transcript's viewport, above its bottom edge, clear of the scrollbar.
#[path = "gates/transcript_jump_control_fits.rs"]
mod transcript_jump_control_fits;

// --- The status bar's texts stay in their boxes at the smallest window (feature 1124, T009) -----

// Builds its own state and reads no cache; it shares this binary for the `support::layout` passes.
#[path = "gates/status_bar_fits.rs"]
mod status_bar_fits;

// --- The session bar's figures stay whole inside the pane (feature 944, T037) -------------------

// A structured session's pane with the meter warning, the totals, a running background task and an
// open subagent thread: in the layout window the three figures share one row; at the smallest
// window each is drawn whole inside the pane.
#[path = "gates/session_bar_fits.rs"]
mod session_bar_fits;

// --- The rich transcript's table wraps in its cells (feature 940, T037) --------------------------

// The recorded markdown reply in a narrow transcript: each table cell inside its column, and a long
// one wrapped onto a second line in its cell (FR-003).
#[path = "gates/transcript_rich_table_wraps.rs"]
mod transcript_rich_table_wraps;

// --- The session area's header fits (feature 1126, T011) ----------------------------------------

// In every covered state that shows a session, at the smallest window beside a 300 px sidebar and
// at the layout window, light and dark: the header's title and chips lie inside the session area
// and overlap nothing (contract H10, FR-013).
#[path = "gates/session_area_fits.rs"]
mod session_area_fits;

// --- T014 — the fixture matches -----------------------------------------------------------------

/// The gate itself (FR-003).
#[test]
fn the_layout_matches_the_committed_fixture() {
    let renderer = lay::renderer();
    let generated = lay::emit_fixture(covered_states(), &renderer, RECORDED_SCHEME);

    lay::compare_or_regenerate(
        std::path::Path::new(FIXTURE_PATH),
        &generated,
        std::env::var("UPDATE_LAYOUT_SNAPSHOT").is_ok(),
        describe_difference,
    );
}

// --- T015 — a failure names the element ---------------------------------------------------------

/// A message that says only "the layout changed" does not satisfy FR-004, and this test is what
/// stops one being written.
///
/// Driven by a synthetic mismatch rather than by editing the application, so it holds even when the
/// fixture is correct.
#[test]
fn a_mismatch_names_the_state_the_element_and_both_geometries() {
    let renderer = lay::renderer();
    let committed = lay::emit_fixture(covered_states(), &renderer, RECORDED_SCHEME);

    let state_name = covered_states()[0].name;
    let anchor = covered_states()[0]
        .anchors
        .first()
        .expect("the first covered state must declare at least one anchor");

    // Move exactly one recorded element, on the anchor's own line.
    let anchor_path = lay::path_token(anchor.path);
    let mut lines: Vec<String> = committed.lines().map(str::to_string).collect();
    let target = lines
        .iter_mut()
        .find(|l| {
            let mut t = l.split_whitespace();
            t.next().is_some_and(|tok| tok == "base" || tok == "over")
                && t.next() == Some(anchor_path.as_str())
        })
        .expect("the anchor's path must appear in the fixture");
    let original = target.clone();
    *target = original.replacen("  0.0", " 99.0", 1);
    let mutated = lines.join("\n") + "\n";

    let message = describe_difference(&mutated, &committed);

    assert!(
        message.contains(state_name),
        "the failure must name the covered state; got:\n{message}"
    );
    assert!(
        message.contains(anchor.name),
        "the failure must name the element by its anchor ({}); got:\n{message}",
        anchor.name
    );
    assert!(
        message.contains("recorded") && message.contains("observed"),
        "the failure must show recorded versus observed geometry; got:\n{message}"
    );
    assert!(
        message.lines().count() > 2,
        "a one-line 'the layout changed' does not satisfy FR-004; got:\n{message}"
    );
}

/// An element with no anchor still has to be identifiable — by its path.
#[test]
fn an_unanchored_difference_is_named_by_path() {
    let renderer = lay::renderer();
    let committed = lay::emit_fixture(covered_states(), &renderer, RECORDED_SCHEME);

    let mut lines: Vec<String> = committed.lines().map(str::to_string).collect();
    let idx = lines
        .iter()
        .rposition(|l| l.starts_with("base") || l.starts_with("over"))
        .expect("the fixture must contain records");
    let path = lines[idx].split_whitespace().nth(1).unwrap().to_string();
    let keep = lines[idx].len() - 8;
    lines[idx] = format!("{}{:>8.1}", &lines[idx][..keep], 77.0);
    let mutated = lines.join("\n") + "\n";

    let message = describe_difference(&mutated, &committed);
    assert!(
        message.contains(&path),
        "an unanchored element must be named by its path ({path}); got:\n{message}"
    );
}

// --- T016 — coverage never narrows silently -----------------------------------------------------

/// Removing a screen must be a visible event, not a quieter fixture (FR-014).
#[test]
fn a_state_that_can_no_longer_be_constructed_fails_naming_it() {
    let renderer = lay::renderer();
    let full = lay::emit_fixture(covered_states(), &renderer, RECORDED_SCHEME);
    let dropped_name = covered_states().last().unwrap().name;

    let reduced = lay::emit_fixture(
        &covered_states()[..covered_states().len() - 1],
        &renderer,
        RECORDED_SCHEME,
    );

    assert_ne!(
        reduced, full,
        "dropping a covered state must change the fixture"
    );
    let message = describe_difference(&full, &reduced);
    assert!(
        message.contains(dropped_name),
        "dropping the state {dropped_name:?} must fail naming it; got:\n{message}"
    );
}

/// An anchor pointing at nothing is itself a failure (FR-014).
#[test]
fn an_anchor_whose_path_does_not_resolve_fails_naming_it() {
    let renderer = lay::renderer();

    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    for (covered, records) in covered_states().iter().zip(all.iter()) {
        for anchor in covered.anchors {
            assert!(
                records.iter().any(|r| r.path == anchor.path),
                "anchor {:?} in covered state {:?} points at a path that no longer resolves — \
                 either the element moved in the tree and the anchor needs re-pointing, or the \
                 element is gone",
                anchor.name,
                covered.name
            );
        }
    }
}

// --- The anchors are the elements they claim to be (T023) ---------------------------------------
//
// `an_anchor_whose_path_does_not_resolve_fails_naming_it` proves an anchor points at *something*.
// It cannot prove it points at the right thing, and T023 is on record as a false completion for
// exactly that gap: anchors that were never declared at all, in a feature whose value is that a
// failure says "sidebar.row.label" instead of "0/0/0/2/0/0/0/2/0/0/2/0/1". A misnamed anchor is
// worse than a bare path — it is a bare path that lies. So each name added here is held to a
// property only the element it names satisfies.

/// The node called `toolbar.title` measures the application name.
///
/// Two independent halves, because either alone is weak. The width proves it is *that* string
/// rather than some other leaf: the toolbar's other text lives in its trailing actions and is a
/// different length. The sibling shape proves the parent is a `Toolbar` and not some other row that
/// happens to lead with a label — `material/toolbar.rs` builds `row![text(title), Space::Fill]` and
/// nothing else in the shell has a zero-height full-width spacer as its second child.
#[test]
fn the_toolbar_title_anchor_measures_the_application_name() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);

    // The role the toolbar sets its title in, measured at that role's own size *and weight*.
    // Feature 018's T017–T021 moved the title from the body role to `title_medium`, which is both
    // larger and heavier; measuring it against the Regular face at 14dp would report a width the
    // application never draws.
    let role = micold_core::tokens::typography::TITLE_LARGE;
    let expected = lay::measure(
        micold_core::metadata::APP_NAME,
        lay::reference_font_at(role.weight),
        role.size,
    );

    let mut checked = 0;
    for (covered, records) in covered_states().iter().zip(all.iter()) {
        for anchor in covered.anchors.iter().filter(|a| a.name == "toolbar.title") {
            let title = records
                .iter()
                .find(|r| r.path == anchor.path)
                .expect("resolution is asserted separately");

            assert!(
                (title.width - expected).abs() <= 0.5,
                "in {:?}, toolbar.title is {:.1}px wide but {:?} measures {expected:.1}px. The \
                 anchor is pointing at some other element, and every failure it names is mislabelled",
                covered.name,
                title.width,
                micold_core::metadata::APP_NAME
            );

            assert_eq!(
                anchor.path.last(),
                Some(&0),
                "in {:?}, toolbar.title is not the leading child of its row",
                covered.name
            );
            let mut spacer_path = anchor.path.to_vec();
            *spacer_path.last_mut().expect("checked above") = 1;
            let spacer = records
                .iter()
                .find(|r| r.path == spacer_path)
                .unwrap_or_else(|| {
                    panic!(
                        "in {:?}, toolbar.title has no sibling at index 1 — its parent is not a \
                         Toolbar row",
                        covered.name
                    )
                });
            assert!(
                spacer.height == 0.0 && spacer.width > title.width,
                "in {:?}, the sibling after toolbar.title is {:.1}x{:.1}, not the full-width \
                 zero-height spacer a Toolbar puts there",
                covered.name,
                spacer.width,
                spacer.height
            );

            checked += 1;
        }
    }

    assert!(
        checked >= 2,
        "expected the toolbar title to be anchored on both shell states, found {checked}"
    );
}

/// Every node called `dialog.actions` has the shape of an action row.
///
/// The index differs per state — it is the last child of a column whose length depends on which
/// fields the form shows — so there is no path to assert against. Assert the signature instead:
/// last in its column, two or more controls, laid out side by side on one line. A form field would
/// fail the second and third; a field *label* would fail all three.
#[test]
fn the_action_row_anchors_are_action_rows() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);

    let mut checked = 0;
    for (covered, records) in covered_states().iter().zip(all.iter()) {
        for anchor in covered
            .anchors
            .iter()
            .filter(|a| a.name == "dialog.actions")
        {
            let (parent, index) = anchor.path.split_at(anchor.path.len() - 1);
            let index = index[0];

            assert!(
                !records.iter().any(|r| r.path.len() == anchor.path.len()
                    && r.path.starts_with(parent)
                    && r.path[parent.len()] > index),
                "in {:?}, dialog.actions is not the last child of its column — a field was added \
                 below it, or the anchor is pointing at a field",
                covered.name
            );

            let mut controls: Vec<_> = records
                .iter()
                .filter(|r| {
                    r.path.len() == anchor.path.len() + 1 && r.path.starts_with(anchor.path)
                })
                .collect();
            controls.sort_by(|a, b| a.x.total_cmp(&b.x));

            assert!(
                controls.len() >= 2,
                "in {:?}, dialog.actions has {} child control(s); an action row has at least two",
                covered.name,
                controls.len()
            );

            for pair in controls.windows(2) {
                let (left, right) = (pair[0], pair[1]);
                assert!(
                    left.y == right.y && left.x + left.width <= right.x,
                    "in {:?}, dialog.actions' controls are not side by side on one line: \
                     {:?} at ({:.1}, {:.1}) {:.1} wide, then {:?} at ({:.1}, {:.1})",
                    covered.name,
                    lay::path_token(&left.path),
                    left.x,
                    left.y,
                    left.width,
                    lay::path_token(&right.path),
                    right.x,
                    right.y
                );
            }

            checked += 1;
        }
    }

    assert!(
        checked >= 5,
        "expected an action row anchored on every dialog state, found {checked}"
    );
}

/// `overview.card.header` is the strip across the top of a card, and `overview.card.title` is the
/// first line of the body under it.
///
/// The header is the first child of the card's column, and its sibling — the body — starts where
/// it ends at the same x and width: a strip on top of a body, which no line *inside* the body is.
/// The title is the first line of that body's text column, below the header and inset from its
/// edge, and taller than the line after it, because it is set in the larger title role.
#[test]
fn the_overview_card_anchors_are_the_header_strip_and_the_title() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let records = records_named(all, "overview-open");
    let state = covered_states()
        .iter()
        .find(|s| s.name == "overview-open")
        .expect("looked up above");
    let at = |name: &str| {
        let anchor = state
            .anchors
            .iter()
            .find(|a| a.name == name)
            .unwrap_or_else(|| panic!("overview-open declares no {name:?} anchor"));
        records
            .iter()
            .find(|r| r.path == anchor.path)
            .expect("resolution is asserted separately")
    };
    let sibling = |path: &[usize], index: usize| {
        let mut path = path.to_vec();
        *path.last_mut().expect("anchors are not the root") = index;
        records
            .iter()
            .find(|r| r.path == path)
            .unwrap_or_else(|| panic!("no node at {}", lay::path_token(&path)))
    };

    let header = at("overview.card.header");
    assert_eq!(
        header.path.last(),
        Some(&0),
        "the header leads its card's column"
    );
    let body = sibling(&header.path, 1);
    assert!(
        (body.y - (header.y + header.height)).abs() <= 0.5
            && body.x == header.x
            && body.width == header.width,
        "the node after overview.card.header is {:.1}x{:.1} at ({:.1}, {:.1}), not the card body \
         directly under a {:.1}x{:.1} header at ({:.1}, {:.1})",
        body.width,
        body.height,
        body.x,
        body.y,
        header.width,
        header.height,
        header.x,
        header.y
    );

    let title = at("overview.card.title");
    assert!(
        title.path.starts_with(&body.path) && title.path.last() == Some(&0),
        "overview.card.title is not the first line inside the card body under the header"
    );
    assert!(
        title.y >= header.y + header.height && title.x > header.x,
        "overview.card.title at ({:.1}, {:.1}) is not below and inset from the header",
        title.x,
        title.y
    );
    let next = sibling(&title.path, 1);
    assert!(
        title.height > next.height,
        "overview.card.title is {:.1}px tall, not taller than the {:.1}px line after it: it is \
         not set in the title role",
        title.height,
        next.height
    );
}

// --- 837: the picked issue's details leave Create and the footer in place -------------------------

/// The records of the covered state called `name`.
fn records_named<'a>(all: &'a [Vec<lay::LayoutRecord>], name: &str) -> &'a [lay::LayoutRecord] {
    let index = covered_states()
        .iter()
        .position(|s| s.name == name)
        .unwrap_or_else(|| panic!("no covered state {name:?}"));
    &all[index]
}

/// US2-1, US2-2, US3-3: with the details read, or failed, the form draws rows under the picker that
/// it does not draw while they are awaited — and the action row stays the form's last child, its
/// buttons side by side at the same width and never above where they were (SC-001, FR-006).
#[test]
fn the_details_rows_add_height_and_the_footer_keeps_its_place() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let waiting = records_named(all, "add-worktree-dialog-issue-details-loading");
    let actions: &[usize] = &[6, 0, 0, 1];
    let place = |records: &[lay::LayoutRecord]| {
        let r = records
            .iter()
            .find(|r| r.path == actions)
            .expect("the action row is laid out");
        (r.x, r.y, r.width)
    };
    let (wait_x, wait_y, wait_width) = place(waiting);

    for (name, rows) in [
        ("add-worktree-dialog-issue-details-loaded", 9),
        ("add-worktree-dialog-issue-details-failed", 1),
    ] {
        let shown = records_named(all, name);
        assert!(
            shown.len() >= waiting.len() + rows,
            "{name}: expected at least {rows} more nodes than while the details are awaited \
             ({} vs {})",
            shown.len(),
            waiting.len()
        );
        let (x, y, width) = place(shown);
        assert_eq!(
            (x, width),
            (wait_x, wait_width),
            "{name}: the action row moved sideways or changed width"
        );
        assert!(
            y >= wait_y,
            "{name}: the action row moved up ({y} < {wait_y})"
        );
    }
}

// --- The overlay pass is exercised, not merely present (FR-009) ---------------------------------

/// Some covered state must actually produce overlay records.
///
/// This was added after the fixture was found to contain **zero** `over` lines. The overlay pass
/// had been implemented, documented and shipped, and every covered state ran through it — and none
/// of them opened anything laid out that way, because the only thing in this application laid out
/// that way is a picker's floating list and no covered state had one open.
///
/// Nothing failed. A pass that records nothing is indistinguishable from a pass that found nothing,
/// so the coverage claim in FR-009 was true about the code and false about the fixture. That is the
/// same shape as the defect this whole feature exists to correct — a gate quietly narrower than it
/// looks — arrived at from the opposite direction.
#[test]
fn the_overlay_pass_records_something_somewhere() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);

    let with_overlays: Vec<&str> = covered_states()
        .iter()
        .zip(all.iter())
        .filter(|(_, records)| records.iter().any(|r| r.layer == lay::Layer::Overlay))
        .map(|(covered, _)| covered.name)
        .collect();

    assert!(
        !with_overlays.is_empty(),
        "no covered state produces a single overlay record, so the overlay pass is running over \
         every state and recording nothing. It is the only thing that can see a picker's list — \
         `material::Select`'s menu is laid out through `Widget::overlay` and is invisible to the \
         base walk — so this reads as coverage while covering nothing. Register a \
         state that opens one (`StateUnderTest::pressing`), or, if no such widget is left in the \
         application, delete the pass rather than leaving it as evidence of something it no longer \
         does."
    );
}

// --- FR-008c's third layout, and FR-011's sampling point, are reached (T042, T043) --------------

/// Some covered state must hold a project the application considers unavailable.
///
/// FR-008c names three required layouts — no project open, **an unavailable project**, and a
/// disconnected daemon — and for the whole of this feature only two of them existed. `shell.rs`
/// branches on availability and renders `Button::filled("Unavailable")` where the available path
/// renders an icon-plus-label composite, so the two are different geometry; the unavailable one was
/// never laid out.
///
/// Two things hid it. `empty-project-without-worktrees` reads like the missing case and is not it —
/// it is a project that is present and simply has nothing in it. And `FakeScanner::default()`
/// answers `is_available: true`, so every covered state built through `workspace_with` takes the
/// available branch by construction; there was no state anywhere that could have taken the other.
#[test]
fn a_covered_state_holds_an_unavailable_project() {
    use micold_core::project::Availability;

    let named: Vec<&str> = covered_states()
        .iter()
        .filter(|covered| {
            (covered.build)()
                .state
                .workspace
                .projects
                .iter()
                .any(|p| p.availability == Availability::Unavailable)
        })
        .map(|covered| covered.name)
        .collect();

    assert!(
        !named.is_empty(),
        "no covered state holds a project marked Unavailable, so FR-008c's second required layout \
         is not covered. `ui/shell.rs` renders that branch differently from the available one, and \
         nothing resolves it. Note that `FakeScanner::default()` reports every folder available, so \
         a covered state has to set availability rather than expect it"
    );
}

/// Some covered state must have scroll content taller than the viewport showing it.
///
/// FR-011 requires scroll-dependent geometry to be recorded at a defined, reproducible offset, and
/// `a_fresh_tree_samples_at_rest` proves a fresh tree reports every scrollable at the top. It proves
/// it against `State::default()`, in which nothing overflows — so the guarantee held over a tree
/// where no element's geometry depends on scroll position at all.
///
/// That is the same shape as the overlay-pass gap above: a property asserted everywhere, about
/// nothing. The sidebar's list is the one scrollable in the application whose content is driven by
/// state, so a covered state with enough worktrees to overflow it is what makes the sampling point
/// mean something.
///
/// Detected by geometry rather than by widget type, since a layout node carries no type: a
/// scrollable that is scrolling is a **viewport with exactly one child, taller than itself**. That
/// is how `Scrollable` lays out — one content wrapper measured against an unbounded height, then
/// clipped — and the single-child requirement is what makes the shape specific. Two looser
/// formulations were tried and both matched things that are not scrolling: "any child taller than
/// its parent" catches the collapsed accordion whose child overhangs a zero-height `Expand`
/// (`CLIP_REVEALED`), and dropping the child count catches the overlay pass shrinking a root below
/// the body beneath it.
#[test]
fn a_covered_state_scrolls() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);

    let overflowing: Vec<&str> = covered_states()
        .iter()
        .zip(all.iter())
        .filter(|(_, records)| {
            records.iter().any(|parent| {
                let children: Vec<_> = records
                    .iter()
                    .filter(|c| {
                        c.path.len() == parent.path.len() + 1 && c.path.starts_with(&parent.path)
                    })
                    .collect();
                parent.height > 0.0
                    && children.len() == 1
                    && children[0].height > parent.height + 1.0
            })
        })
        .map(|(covered, _)| covered.name)
        .collect();

    assert!(
        !overflowing.is_empty(),
        "no covered state has content taller than the viewport showing it, so nothing in the \
         fixture scrolls and FR-011's sampling point is asserted about a tree in which no geometry \
         depends on scroll position. Register a state whose sidebar list overflows"
    );
}

/// A state that presses a control must end up with that control open.
///
/// The press is dispatched into the widget tree and can silently do nothing — a modal that has not
/// finished appearing swallows it, a path can drift onto a node that ignores clicks — and the state
/// would still resolve, record a perfectly valid base layout, and cover exactly what it did before.
/// The overlay records are the only evidence the press landed.
#[test]
fn a_state_that_presses_a_control_records_the_control_it_opened() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        let Some(pressed) = (covered.build)().press_at else {
            continue;
        };
        assert!(
            records.iter().any(|r| r.layer == lay::Layer::Overlay),
            "covered state {:?} presses {} and no overlay was laid out. The press landed on \
             nothing, so this state covers the same thing it would have covered without it. Either \
             the node moved — re-point the path against layout_snapshot.txt — or the control it \
             opens no longer uses Widget::overlay.",
            covered.name,
            lay::path_token(pressed),
        );
    }
}

// --- T017 — scheme independence (FR-008a) -------------------------------------------------------

/// Paths whose geometry legitimately differs between the two schemes, with the reason.
///
/// Required to stay true: a stale entry fails the test below, on the same reasoning FR-014 applies
/// to coverage. An exemption nobody re-reads is how a gate quietly becomes narrower than it looks.
/// **Empty, and that is a result rather than an omission** (BUG-003). It held two entries: the
/// overflow menu's theme-mode row and its label, whose string names the resolved scheme and so
/// measured one word narrower in the dark one. Both went stale when §7.5's item row landed — a menu
/// item's label now fills the width its 12dp ends leave it, so the node is the same size whatever
/// the string inside it says. The geometry stopped depending on the content, which is what the
/// exemption was for. The staleness assertion below is what turned that into a required edit rather
/// than a widening nobody noticed.
const SCHEME_DEPENDENT: &[(&[usize], &str)] = &[];

fn scheme_exempt(path: &[usize]) -> bool {
    SCHEME_DEPENDENT.iter().any(|(p, _)| *p == path)
}

/// Layout is expected to be scheme-independent. Asserting it costs one extra walk; duplicating the
/// fixture to record it would cost a reviewer twice the reading for the same information.
///
/// The expectation holds *structurally* without exception — the two schemes must produce the same
/// tree, element for element. It holds for geometry too, except where a label's own text names the
/// scheme, which is a content difference rather than a layout one and is declared above.
#[test]
fn the_other_scheme_lays_out_identically() {
    let renderer = lay::renderer();
    let mut exercised: std::collections::BTreeSet<&[usize]> = Default::default();

    let lights = lay::cached_records(covered_states(), &renderer, ColorScheme::Light);
    let darks = lay::cached_records(covered_states(), &renderer, ColorScheme::Dark);

    for ((covered, light), dark) in covered_states().iter().zip(lights.iter()).zip(darks.iter()) {
        let shape =
            |rs: &[lay::LayoutRecord]| rs.iter().map(|r| r.path.clone()).collect::<Vec<_>>();
        assert_eq!(
            shape(light),
            shape(dark),
            "covered state {:?} produced a different widget tree in the dark scheme. Structure \
             must never depend on the colour scheme, and there is no exemption for this.",
            covered.name
        );

        for (l, d) in light.iter().zip(dark.iter()) {
            if l == d {
                continue;
            }
            if scheme_exempt(&l.path) {
                exercised.insert(
                    SCHEME_DEPENDENT
                        .iter()
                        .find(|(p, _)| *p == l.path.as_slice())
                        .map(|(p, _)| *p)
                        .unwrap(),
                );
            } else {
                panic!(
                    "covered state {:?} lays out differently in the dark scheme at path {:?}\n  \
                     light: {l:?}\n  dark : {d:?}\nLayout must not depend on the colour scheme. \
                     If this element's text legitimately names the scheme, declare it in \
                     SCHEME_DEPENDENT with the reason; otherwise it is a defect.",
                    covered.name, l.path
                )
            }
        }
    }

    for (path, reason) in SCHEME_DEPENDENT {
        assert!(
            exercised.contains(path),
            "the exemption for path {path:?} ({reason}) never fired — no covered state resolves it \
             differently between the schemes any more. A stale exemption widens the gate silently, \
             so delete it rather than leave it."
        );
    }
}

// --- The failure message (T022) -----------------------------------------------------------------

/// Build the failure message: the covered state, the element, and both geometries (FR-004).
fn describe_difference(recorded: &str, observed: &str) -> String {
    let mut out = String::from(
        "the resolved layout differs from tests/fixtures/layout_snapshot.txt\n\n\
         If this change is intended, accept it deliberately:\n  \
         UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot\n\n\
         (`--test` selects the target. Drop it and the bare word is a test-name filter matching \
         nothing: the run exits 0 having regenerated nothing.)\n",
    );

    let rec: Vec<&str> = recorded.lines().collect();
    let obs: Vec<&str> = observed.lines().collect();

    let mut section = "(file header)".to_string();
    let mut anchors: Vec<(&str, String)> = Vec::new();
    let mut reported = 0usize;

    for i in 0..rec.len().max(obs.len()) {
        let r = rec.get(i).copied();
        let o = obs.get(i).copied();

        if let Some(line) = r.or(o) {
            if let Some(name) = line.strip_prefix("## ") {
                section = name.to_string();
                anchors.clear();
                continue;
            }
            if let Some(rest) = line.strip_prefix("@ ") {
                if let Some((name, path)) = rest.split_once(" -> ") {
                    anchors.push((name.trim(), path.trim().to_string()));
                }
                continue;
            }
        }

        if r == o {
            continue;
        }
        reported += 1;
        if reported > 20 {
            out.push_str("\n  … further differences suppressed\n");
            break;
        }

        let path_of = |line: Option<&str>| {
            line.and_then(|l| l.split_whitespace().nth(1))
                .unwrap_or("(absent)")
                .to_string()
        };
        let path = path_of(r.or(o));
        let named = anchors
            .iter()
            .find(|(_, p)| *p == path)
            .map(|(n, _)| (*n).to_string())
            .unwrap_or_else(|| format!("path {path}"));

        out.push_str(&format!(
            "\n  in covered state: {section}\n    element : {named}\n    recorded: {}\n    observed: {}\n",
            r.unwrap_or("(line absent)").trim_end(),
            o.unwrap_or("(line absent)").trim_end(),
        ));
    }

    if reported == 0 {
        out.push_str("\n  (no line-level difference found — the files differ in length or in trailing bytes)\n");
    }

    out
}

// --- Feature 551 (T019): the child marker ------------------------------------------------------

/// SC-004: every child row, orphans included, carries the child marker and no user-started row does.
#[test]
fn every_child_row_carries_the_child_marker_and_no_user_started_row_does() {
    use micold_core::origin::OriginStatus;
    use micold_core::session::{SessionId, SessionLocation};
    use support::spawn_tree::{known, session, state_with, wt, DIR};

    let location: SessionLocation = wt();
    let mut user = session(&location, None);
    user.main = true;
    let child = session(&location, known(&user));
    let grandchild = session(&location, known(&child));
    let orphan = session(&location, Some((SessionId::new(), OriginStatus::Known)));
    let removed = session(&location, Some((SessionId::new(), OriginStatus::Removed)));
    let mut state = state_with(vec![user, child, grandchild, orphan, removed]);
    state.sidebar.expanded.insert(DIR.to_string());

    let under = lay::StateUnderTest::new(state);
    let mut renderer = lay::renderer();
    let painted: Vec<String> = lay::painted_text(lay::view_of(&under), &mut renderer)
        .into_iter()
        .map(|p| p.content)
        .collect();
    let annotations: Vec<&String> = painted
        .iter()
        .filter(|c| c.as_str() == "claude" || c.starts_with("agent"))
        .collect();
    assert_eq!(
        annotations.len(),
        5,
        "one annotation per session: {painted:?}"
    );
    let marked = annotations
        .iter()
        .filter(|c| c.starts_with("agent"))
        .count();
    assert_eq!(
        marked, 4,
        "the four children and orphans are marked: {annotations:?}"
    );
    assert!(
        annotations.iter().any(|c| c.as_str() == "claude"),
        "the user-started session is not: {annotations:?}"
    );
    // A child's marker is ellipsized to share the row with the name, so only its start is stable.
    assert!(annotations.iter().any(|c| c.starts_with("agent, origin p")));
    assert!(annotations.iter().any(|c| c.starts_with("agent, origin r")));
}
