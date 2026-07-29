//! No layout node is laid out beyond the node that owns it (feature 019, BUG-001).
//!
//! The third gate, and the one that answers a question neither of the others can.
//!
//! `layout_snapshot.txt` already records every box this test reads — the data is not new. What is
//! new is that this makes an *assertion* about it. A byte-compare fixture records whatever it is
//! shown as correct, so a defect older than the fixture is regenerated into the expected value and
//! becomes the baseline; snapshots catch changes, not defects. `layout_text_overflow` compares
//! widths only, and asks the renderer rather than the layout tree.
//!
//! BUG-001 is the motivating case: `material::Expand` reports a shrunken height to its parent while
//! its child keeps its full height, relying on a draw-time clip that does not take effect, so the
//! child paints over whatever moved up. Vertical, structural, and present in the layout tree — one
//! gate short of being caught by all three.
//!
//! **Compiled into the `layout_snapshot` binary rather than its own** (SC-006). Cargo makes one
//! test binary per file directly under `tests/`, and each binary is its own process — so a
//! `OnceLock` cache cannot cross between them. Standing alone, this gate re-resolved the same nine
//! covered states `layout_snapshot` had already resolved, at a cost of ~6s for work that was
//! already done. Living under `tests/gates/` keeps it a separate file without making it a separate
//! process, so `cached_records` serves both.
//!
//! It remains a distinct gate: separate tests, separate failures, and it asserts about the records
//! rather than comparing them to the fixture.

use crate::support::covered_states::{self, covered_states};
use crate::support::layout as lay;
use micold_core::theme::ColorScheme;

/// The scheme the geometry fixture is recorded in. Containment is a structural property, so one
/// scheme establishes it; the dark pass exists for colour, which this test does not read.
const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;

/// Half a pixel, matching the text-overflow gate — enough for accumulated float error in a nested
/// layout, far below anything a person could see.
const TOLERANCE: f32 = 0.5;

/// Escapes that already existed when this gate was built, as `(covered state, child node path)`.
///
/// **All seven are one defect: BUG-001**, the sidebar's collapsed filter accordion, in every state
/// that renders a sidebar. `Expand::layout` reports `full.height * progress` to its parent while
/// the child node keeps its full height, so at rest the child is a 40–42px node inside a 0px one.
/// `error-daemon-disconnected` differs only in its path — the disconnection banner shifts the shell
/// index from 2 to 3 — which is the invariant following structure rather than a hardcoded path.
/// `main-shell-sidebar-collapsed` and `empty-no-project-open` are absent because neither renders
/// the panel.
///
/// Attribution is not inferred from the shape: `the_recorded_escapes_are_the_accordion_reveal`
/// drives the panel open and shows the same nodes come clean.
///
/// **Recorded, not fixed.** FR-019 forbids this feature changing application source, and fixing
/// `Expand`'s clip changes the sidebar's motion. It equally forbids tuning the gate until the
/// defect stops showing. The staleness assertion below requires every entry to keep firing, so the
/// fix deletes these rather than leaving them as folklore.
///
/// **These seven are the cause at rest, not the visible defect.** At `progress` 0 the reveal does
/// not paint at all (`Expand::draw` returns early below `HIDDEN`), so nothing overlaps yet. The
/// overlap a user actually meets is covered separately by `the_reveal_paints_over_what_moved_up`,
/// which pins the panel mid-reveal.
const KNOWN_ESCAPES: &[(&str, &str)] = &[
    ("main-shell-sidebar-expanded", "0/0/0/2/0/0/0/1/0"),
    ("add-worktree-dialog-new-branch", "0/0/0/2/0/0/0/1/0"),
    ("add-worktree-dialog-existing-branch", "0/0/0/2/0/0/0/1/0"),
    ("worktree-menu-open", "0/0/0/2/0/0/0/1/0"),
    ("empty-project-without-worktrees", "0/0/0/2/0/0/0/1/0"),
    ("error-daemon-disconnected", "0/0/0/3/0/0/0/1/0"),
    ("error-add-worktree-failed", "0/0/0/2/0/0/0/1/0"),
];

fn known(state: &str, child_path: &str) -> bool {
    KNOWN_ESCAPES
        .iter()
        .any(|(s, p)| *s == state && *p == child_path)
}

/// No covered state lays a node outside its parent.
#[test]
fn no_layout_node_escapes_its_parent() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut unexpected: Vec<String> = Vec::new();
    let mut fired: std::collections::BTreeSet<(&str, String)> = Default::default();

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        for escape in lay::escapes(records, TOLERANCE) {
            let line = format!(
                "{}: {} escapes {} by {:.1}px past its {} edge ({})",
                covered.name,
                escape.child_path,
                escape.parent_path,
                escape.overhang,
                escape.edge,
                escape.layer.token()
            );
            if known(covered.name, &escape.child_path) {
                eprintln!("KNOWN_ESCAPES still fires: {line}");
                fired.insert((covered.name, escape.child_path));
            } else {
                unexpected.push(line);
            }
        }
    }

    assert_eq!(
        fired.len(),
        KNOWN_ESCAPES.len(),
        "every recorded escape must still be observable, or the exemption is widening the gate \
         silently; {} of {} fired. If BUG-001 was fixed, delete the entries it accounted for.",
        fired.len(),
        KNOWN_ESCAPES.len(),
    );

    assert!(
        unexpected.is_empty(),
        "{} layout node(s) are laid outside the parent that owns them. A child bigger than its \
         parent paints over whatever sits beside it, and the geometry fixture cannot report this — \
         it would record the overlap as the expected value.\n  {}",
        unexpected.len(),
        unexpected.join("\n  "),
    );
}

/// BUG-001 as a user meets it: partway through the reveal, where it actually paints.
///
/// The settled states catch the defect's *cause* — a full-height child inside a zero-height
/// `Expand` — but at `progress` 0 nothing is drawn, so no one sees it. This pins the sidebar's
/// filter panel two frames into its 90ms reveal, which is past `Expand::draw`'s early return and
/// still well short of the end. Here the child is both oversized *and* painting, which is the
/// defect as reported.
///
/// Deterministic despite pinning an animation: a track steps a fixed amount per redraw rather than
/// by elapsed time, so frame 2 is frame 2 on every machine (`cdk/motion.rs`).
///
/// **This is expected to fail once `Expand` is fixed, and that is the point.** The registered
/// state is itself the record — there is no exemption list to keep in step — so the fix deletes the
/// entry in `revealing_states` and this test with it.
///
/// Both of its assertions were checked against a failing run before being trusted: pinning at frame
/// 0 fails the first ("at 0.000 of its open height"), and applying BUG-001's own candidate fix to
/// `Expand::layout` fails the second ("no longer lays its child outside itself") while the pin
/// still reads 0.356. That second probe is why `expect_between` is measured against the fully open
/// height rather than against the child — with the defect fixed the child is clipped to its parent,
/// so the child-relative ratio reads 1.0 at every moment and cannot tell a settled reveal from a
/// running one. It would have misreported a fix as a broken pin.
#[test]
fn the_reveal_paints_over_what_moved_up() {
    let renderer = lay::renderer();

    for revealing in covered_states::revealing_states() {
        let records = lay::resolve_revealing(revealing, &renderer, RECORDED_SCHEME);
        let escapes = lay::escapes(&records, TOLERANCE);

        // Asked of the named node, not of whatever escaped — so a fixed `Expand`, which escapes
        // nothing, still answers this and fails the *next* assertion rather than this one.
        let at = |path: &str| records.iter().find(|r| lay::path_token(&r.path) == path);
        let node = at(revealing.node).unwrap_or_else(|| {
            panic!(
                "{:?} names {} as its revealing node, and no such node was resolved. The tree \
                 changed shape; re-point it against layout_snapshot.txt.",
                revealing.name, revealing.node,
            )
        });
        let revealed = lay::resolve_revealed(revealing, &renderer, RECORDED_SCHEME);
        let open = revealed
            .iter()
            .find(|r| lay::path_token(&r.path) == revealing.node)
            .unwrap_or_else(|| {
                panic!(
                    "{:?}: {} is not present once the reveal has finished",
                    revealing.name, revealing.node,
                )
            });

        let (low, high) = revealing.expect_between;
        let fraction = node.height / open.height;
        assert!(
            fraction > low && fraction < high,
            "{:?} resolved with {} at {:.3} of its fully open height, outside the expected {:.2}..{:.2}. \
             not pinned mid-reveal, so whatever this test reports is about some other moment. \
             Check `frames` against the reveal's duration.",
            revealing.name,
            revealing.node,
            fraction,
            low,
            high,
        );

        assert!(
            escapes.iter().any(|e| e.parent_path == revealing.node),
            "{:?} is pinned mid-reveal at {:.3} of its open height, and {} no longer lays its child \
             outside itself. That is BUG-001 fixed — delete this state and its KNOWN_ESCAPES \
             entries. (Escapes seen elsewhere: {:?})",
            revealing.name,
            fraction,
            revealing.node,
            escapes.iter().map(|e| e.child_path.clone()).collect::<Vec<_>>(),
        );

        for escape in &escapes {
            eprintln!(
                "KNOWN_REVEAL_ESCAPES still fires: {}: {} escapes {} by {:.1}px past its {} edge",
                revealing.name,
                escape.child_path,
                escape.parent_path,
                escape.overhang,
                escape.edge,
            );
        }
    }
}

/// The check must be able to *see* an escape, or its silence means nothing.
///
/// The lesson of T025: a gate that cannot fire reads exactly like a gate that found nothing.
#[test]
fn the_check_reports_an_escape_when_one_exists() {
    let parent = lay::LayoutRecord {
        path: vec![],
        layer: lay::Layer::Base,
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 40.0,
    };
    // Exactly BUG-001's shape: a child at full height inside a parent shrunk to a third of it.
    let child = lay::LayoutRecord {
        path: vec![0],
        layer: lay::Layer::Base,
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 120.0,
    };

    let escapes = lay::escapes(&[parent, child], TOLERANCE);

    assert_eq!(
        escapes.len(),
        1,
        "the containment check found nothing in a layout built to violate it — the instrument is \
         broken, and every passing run of the test above is meaningless"
    );
    assert_eq!(escapes[0].edge, "bottom");
    assert!((escapes[0].overhang - 80.0).abs() < 0.01);
}

/// The recorded escapes are `Expand`'s, and this is what proves it rather than asserting it.
///
/// A zero-height parent holding a full-height child is `Expand::layout` at `progress` 0
/// (`animation.rs:741`) — but that reasoning is inference from a shape, and the same shape could
/// come from any collapsed container. So drive the one input that changes `Expand`'s progress and
/// nothing else: the sidebar's filter panel is `Accordion`, which *is* `expand(...)`. Closed, the
/// escape is there; open, the same node is clean. Nothing else about the state differs.
///
/// Without this, `KNOWN_ESCAPES` would be a list of paths whose cause was guessed.
#[test]
fn the_recorded_escapes_are_the_accordion_reveal() {
    let escaping_nodes = |filter_open: bool| -> Vec<String> {
        let mut state = micold_client::app::State::default();
        state.workspace = crate::support::workspace_with(vec![("/fixture/project", vec![])]);
        state.workspace.active = state.workspace.projects.first().map(|p| p.path.clone());
        state.sidebar_width = 260;
        state.sidebar_filter_open = filter_open;

        let element = micold_client::ui::view(
            &state,
            None,
            None,
            0,
            None,
            &micold_core::env_include::EnvIncludeOutcome::Disabled,
            &micold_client::ui::ConnectionStatus::Connected,
        );

        let renderer = lay::renderer();
        lay::escapes(&lay::resolve(element, &renderer), TOLERANCE)
            .into_iter()
            .map(|e| e.child_path)
            .collect()
    };

    let closed = escaping_nodes(false);
    let opened = escaping_nodes(true);

    assert!(
        !closed.is_empty(),
        "the sidebar with its filter panel closed reports no escape, so the entries in \
         KNOWN_ESCAPES cannot be attributed to the accordion at all"
    );
    for node in &closed {
        assert!(
            !opened.contains(node),
            "node {node} escapes whether the filter panel is open or closed, so it is not the \
             accordion's reveal and the attribution recorded in KNOWN_ESCAPES is wrong"
        );
    }
}
