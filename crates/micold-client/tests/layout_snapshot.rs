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
//! `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client layout_snapshot`

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
         UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client layout_snapshot\n",
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
