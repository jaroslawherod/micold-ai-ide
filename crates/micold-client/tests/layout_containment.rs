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

mod support;

use micold_core::theme::ColorScheme;
use support::covered_states::covered_states;
use support::layout as lay;

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
/// **What this does not cover.** At `progress` 0 the reveal does not paint at all (`Expand::draw`
/// returns early below `HIDDEN`), so what is caught here is the defect's structural cause at rest,
/// not the overlap a user sees mid-reveal. Catching that needs a covered state pinned at
/// `0 < progress < 1`, which means pumping redraw events through the tree — the apparatus resolves
/// one settled frame and cannot currently do it.
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
        state.workspace = support::workspace_with(vec![("/fixture/project", vec![])]);
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
