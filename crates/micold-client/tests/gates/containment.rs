//! No layout node is laid out beyond the node that owns it (feature 019).
//!
//! The third gate, and the one that answers a question neither of the others can.
//!
//! `layout_snapshot.txt` already records every box this test reads — the data is not new. What is
//! new is that this makes an *assertion* about it. A byte-compare fixture records whatever it is
//! shown as correct, so a defect older than the fixture is regenerated into the expected value and
//! becomes the baseline; snapshots catch changes, not defects. `layout_text_overflow` compares
//! widths only, and asks the renderer rather than the layout tree.
//!
//! # What this gate does not decide, and why it matters here
//!
//! **A child laid outside its parent is not by itself a defect.** A widget may lay its child out at
//! full size and clip it at draw time; the layout tree records the overhang either way. Whether the
//! overhang is *painted* is decided in `draw`, which this gate does not read. So the invariant
//! catches a child that escapes a parent which does **not** clip, and is structurally blind to the
//! difference otherwise.
//!
//! That limit was learned rather than designed. This gate was built against BUG-001 — `Expand`
//! reporting a shrunken height while its child kept full height — on the belief that the escape
//! *was* the defect. It was not. The fix (`7401f5d`) left `Expand::layout` byte-for-byte unchanged:
//! the child is still laid out oversized and still escapes, deliberately, and what was wrong was
//! that `layout` never re-ran, so the clip was applied to stale bounds. Every escape this gate
//! reported survived the fix, and this gate stayed green across it.
//!
//! The exemption list below is therefore a list of **clip-revealing wrappers**, not of open
//! defects, and it does not expire. What still has value is everything it does *not* exempt: a
//! child escaping a parent that has no clip at all.
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

/// Children that a clip-revealing wrapper deliberately lays out oversized, by node path.
///
/// **Keyed by path alone, not by `(covered state, path)`.** Being a clip-revealing wrapper's child
/// is a property of the widget, not of the screen it appears on — the state dimension was never
/// meaningful, it was just how the list first got written, once per state that happened to render a
/// sidebar. It cost seven entries for two nodes, and it made FR-016 false in practice: T032 added a
/// tenth covered state, and because that state renders a sidebar the gate demanded a second edit
/// here before it would go green. Registering a covered state is supposed to take one change in one
/// place.
///
/// What this trades away: a *different* node arriving at one of these paths in some future state
/// would be exempted without anyone noticing. The paths are structural rather than named, so that
/// is a real risk and worth stating. It is bounded by the staleness assertion below — every listed
/// path must still fire somewhere — and by `the_recorded_escapes_are_the_accordion_reveal`, which
/// re-derives the attribution from behaviour rather than trusting the list.
///
/// **Both are one widget: `material::Expand`**, holding the sidebar's collapsed filter accordion.
/// `Expand::layout` reports `full.height * progress` to its parent while the child node keeps its
/// full height, so at rest the child is a 40–42px node inside a 0px one. It then reveals the child
/// top-down by clipping to its own bounds in `draw` — the overhang is how the reveal works, not a
/// mistake.
///
/// The two paths are the same accordion under two shell arrangements: the disconnection banner
/// shifts the shell index from 2 to 3. That the invariant follows the structure rather than a
/// hardcoded path is the reason there are two and not one.
///
/// Attribution is not inferred from the shape: `the_recorded_escapes_are_the_accordion_reveal`
/// drives the panel open and shows the same nodes come clean.
///
/// **These were once recorded as BUG-001 and they are not.** The list was written while that bug
/// was open, in the belief that the overhang was the defect. The fix left `Expand::layout`
/// unchanged — the real cause was `layout` not re-running, so the clip received stale bounds — and
/// every entry survived it untouched. The correction is worth keeping visible: this gate reported
/// the right nodes for the wrong reason, and would have reported them just as loudly had nothing
/// been wrong at all.
///
/// So, unlike a defect exemption, this list **does not expire**. What the assertion below still
/// buys is that it cannot quietly grow: a new entry means either a new clip-revealing wrapper, to
/// be named here with its reason, or a child escaping a parent that does not clip, which is a bug.
const CLIP_REVEALED: &[&str] = &["0/0/0/1/0/0/0/1/0", "0/0/0/2/0/0/0/1/0"];

/// The sidebar list's scroll content, which is taller than its viewport by design.
///
/// A second clipping parent, kept in its own list because it is a different mechanism and the
/// distinction is the whole point of the one above: `Expand` overhangs to *reveal* progressively,
/// a `Scrollable` overhangs because its content does not fit and the user moves it. Folding them
/// together would make the list read as "overhangs we tolerate", which is exactly the shapeless
/// exemption the module documentation argues against.
///
/// Arrived with `main-shell-sidebar-scrolled-to-top` (T043) and nothing else fires it: no other
/// covered state has enough worktrees to overflow the sidebar, which is why nothing in the fixture
/// scrolled until that state existed. `the_recorded_scroll_overflow_is_the_sidebar_list` proves the
/// attribution by shortening the list and showing the same node comes clean.
///
/// Like `CLIP_REVEALED` this does not expire — a scrollable whose content fits is not scrolling,
/// and it is the overflow that makes FR-011 mean anything. The staleness assertion still applies:
/// if this path goes quiet, either the state stopped overflowing (and FR-011 is uncovered again) or
/// the tree renumbered around it.
const SCROLL_CONTENT: &[&str] = &["0/0/0/1/0/0/0/2/0"];

/// The **select's open list**, overflowing its own eight-row cap (feature 022).
///
/// Kept apart from [`SCROLL_CONTENT`] for the same reason that list is kept apart from
/// [`CLIP_REVEALED`]: they are different sites of the same mechanism, each with its own attribution
/// proof, and one list would leave "which scrollable is this?" unanswered by anything but the path.
///
/// It arrived with the widget rather than with a new covered state.
/// `add-worktree-dialog-type-menu-open` has always opened a list of all ten conventional types, and
/// the list has always capped at `MAX_ROWS_BEFORE_SCROLL` — but until feature 022 that list was
/// `pick_list`'s, which scrolls inside a single node and shows the overflow to nobody. The select
/// floats `material::picker`'s list now, which is the same `Scrollable` the sidebar uses, so the
/// overflow is a node outside a node exactly as the sidebar's is: ten 48dp rows inside an eight-row
/// viewport, overhanging by 96dp.
///
/// **This list grew during a feature whose brief was to remove exceptions, and that is worth saying
/// plainly.** Nothing was widened to make anything pass: the overhang is a `Scrollable` doing what
/// the one above already does, and the alternative — a list that does not scroll — would leave two
/// of the ten types unreachable. `the_recorded_picker_overflow_is_the_open_option_list` proves the
/// attribution from the records rather than from the shape.
const PICKER_LIST_CONTENT: &[&str] = &["0/0/0/0/0/0/0"];

/// Every overhang this gate does not treat as a finding, with the reason it is allowed.
fn clips_deliberately(child_path: &str) -> Option<&'static str> {
    if CLIP_REVEALED.contains(&child_path) {
        Some("CLIP_REVEALED")
    } else if SCROLL_CONTENT.contains(&child_path) {
        Some("SCROLL_CONTENT")
    } else if PICKER_LIST_CONTENT.contains(&child_path) {
        Some("PICKER_LIST_CONTENT")
    } else {
        None
    }
}

/// No covered state lays a node outside a parent that will not clip it.
#[test]
fn no_layout_node_escapes_its_parent() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut unexpected: Vec<String> = Vec::new();
    let mut fired: std::collections::BTreeSet<String> = Default::default();

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
            if let Some(list) = clips_deliberately(&escape.child_path) {
                eprintln!("{list} still fires: {line}");
                fired.insert(escape.child_path);
            } else {
                unexpected.push(line);
            }
        }
    }

    let silent: Vec<&&str> = CLIP_REVEALED
        .iter()
        .chain(SCROLL_CONTENT)
        .chain(PICKER_LIST_CONTENT)
        .filter(|path| !fired.contains(**path))
        .collect();
    assert!(
        silent.is_empty(),
        "every exempted overhang must still be observable somewhere, or the exemption is widening \
         the gate silently. These fired in no covered state: {silent:?}. A path that has gone quiet \
         means the wrapper no longer reveals by clipping, or the tree renumbered around it — either \
         way, delete it and check that whatever replaced the mechanism is covered by something.",
    );

    assert!(
        unexpected.is_empty(),
        "{} layout node(s) are laid outside the parent that owns them, and that parent is not a \
         known clip-revealing wrapper. Either it clips at draw time — in which case name it in \
         CLIP_REVEALED with the reason — or it paints over whatever sits beside it, which the \
         geometry fixture cannot report because it would record the overlap as the expected \
         value.\n  {}",
        unexpected.len(),
        unexpected.join("\n  "),
    );
}

/// Containment holds *during* a reveal too, for everything the reveal is not itself clipping.
///
/// Every other check here resolves a settled frame. That is deliberate — mid-animation geometry is
/// excluded from the fixture (T030) because it would churn on any change to a duration or an easing
/// curve. But an invariant is not a recording, and it can be asked at a moment no fixture would
/// want to hold: this pins the sidebar's filter panel partway into its 90ms reveal and requires
/// that the *only* node escaping its parent is the one the wrapper is deliberately clipping.
///
/// What that buys, concretely: a reveal that grows into its siblings, or reflows a neighbour out of
/// the shell, shows up here and nowhere else. The settled states cannot see it — at rest the panel
/// is closed and the geometry is the closed geometry.
///
/// What it does **not** buy is any statement about BUG-001. That bug was a stale-bounds defect in
/// `update`, invisible to the layout tree at every progress value; this test passed against the
/// defective code and passes against the fix. It was originally written as `the_reveal_paints_over_
/// what_moved_up`, asserting that the child escapes mid-reveal — which is true, and is the reveal
/// working. The mechanism BUG-001 broke is covered by `tests/animated_layout_relayouts.rs`, which
/// asserts on relayout requests rather than on boxes.
///
/// Deterministic despite pinning an animation: a track steps a fixed amount per redraw rather than
/// by elapsed time, so frame *n* is frame *n* on every machine (`cdk/motion.rs`). Each pumped frame
/// must carry a distinct `Instant`, or the track's re-delivery guard collapses them into one — see
/// `resolve_revealing`, which read 0.178 instead of 0.356 until that was fixed.
#[test]
fn only_the_clip_revealed_child_escapes_mid_reveal() {
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

        // The revealing node's own child is expected to overhang — that is the clip. Anything else
        // escaping is a node the reveal has pushed out of the parent that owns it.
        let (revealing_child, elsewhere): (Vec<_>, Vec<_>) = escapes
            .iter()
            .partition(|e| e.parent_path == revealing.node);

        assert!(
            !revealing_child.is_empty(),
            "{:?} is pinned mid-reveal at {:.3} of its open height, and {} does not lay its child \
             outside itself — so it is no longer revealing by clipping. This test then proves \
             nothing about the reveal it is named for: re-point it, or delete it along with the \
             CLIP_REVEALED entries that must also have stopped firing.",
            revealing.name,
            fraction,
            revealing.node,
        );

        assert!(
            elsewhere.is_empty(),
            "{:?}, pinned at {:.3} of its open height, lays {} node(s) outside their parents \
             besides the one {} is clipping. A reveal that pushes a neighbour out of the box that \
             owns it is visible only while it runs, so no settled state and no fixture will report \
             it.\n  {}",
            revealing.name,
            fraction,
            elsewhere.len(),
            revealing.node,
            elsewhere
                .iter()
                .map(|e| format!(
                    "{} escapes {} by {:.1}px past its {} edge",
                    e.child_path, e.parent_path, e.overhang, e.edge
                ))
                .collect::<Vec<_>>()
                .join("\n  "),
        );
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
    // The shape this gate exists to notice: a child at full height inside a parent shrunk to a
    // third of it.
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

/// The recorded scroll overflow is the sidebar's list, and this is what proves it.
///
/// Same argument as the accordion test below: a tall child in a shorter parent is *consistent* with
/// a scroll viewport, and consistent is not the same as caused. So drive the one input that decides
/// whether the list overflows — how many worktrees there are — and change nothing else. Three, and
/// the node is contained; thirty, and it escapes by hundreds of pixels.
///
/// Without this, `SCROLL_CONTENT` would exempt a path on the strength of its shape, which is the
/// mistake `CLIP_REVEALED` was rewritten to stop making.
#[test]
fn the_recorded_scroll_overflow_is_the_sidebar_list() {
    let escaping_nodes = |worktrees: usize| -> Vec<String> {
        let mut workspace = crate::support::workspace_with(vec![("/fixture/project", vec![])]);
        workspace.active = workspace.projects.first().map(|p| p.path.clone());
        let state = micold_client::app::State {
            workspace,
            worktrees: (0..worktrees)
                .map(|i| micold_core::worktree::Worktree {
                    dir_name: format!("feat-{i:02}"),
                    path: std::path::PathBuf::from("/fixture/project").join(format!("feat-{i:02}")),
                    branch: Some(format!("feat/{i:02}")),
                    status: micold_core::worktree::WorktreeStatus::Valid,
                })
                .collect(),
            sidebar_width: 260,
            ..micold_client::app::State::default()
        };

        let element = micold_client::ui::view(
            &state,
            None,
            None,
            0,
            None,
            &micold_core::env_include::EnvIncludeOutcome::Disabled,
            &micold_client::features::connection::ConnectionStatus::Connected,
        );

        let renderer = lay::renderer();
        lay::escapes(&lay::resolve(element, &renderer), TOLERANCE)
            .into_iter()
            .map(|e| e.child_path)
            .collect()
    };

    let few = escaping_nodes(3);
    let many = escaping_nodes(30);

    for path in SCROLL_CONTENT {
        assert!(
            many.contains(&path.to_string()),
            "{path} is exempted as scroll content, and a sidebar holding thirty worktrees does not \
             lay it outside its viewport. Either it is not the list's content node, or the list no \
             longer scrolls — and if it no longer scrolls, FR-011's sampling point is once again \
             asserted about a tree in which nothing depends on scroll position"
        );
        assert!(
            !few.contains(&path.to_string()),
            "{path} escapes its parent with only three worktrees, so the overhang is not the list \
             outgrowing its viewport and the attribution recorded in SCROLL_CONTENT is wrong"
        );
    }
}

/// The picker exemption is the **type list** outgrowing its own cap, proved from the records.
///
/// The sidebar's proof above varies the input — three worktrees against thirty — and shows the node
/// comes clean when the list fits. That lever does not exist here: a select's options are fixed by
/// its call site, and this one's are `ConventionalType::ALL`, so there is no "few types" to resolve.
///
/// What is available instead is arithmetic the records already carry. If the exempted node is the
/// open list's content, then it holds exactly one child per conventional type, its height is exactly
/// those children stacked, and its parent — the cap — is shorter than it. Any other node at that
/// path fails at least one of the three. That is weaker than driving the input, and saying so is the
/// point: it is why this has its own test rather than being folded into the one above, where the
/// weaker proof would have quietly become the standard for both.
#[test]
fn the_recorded_picker_overflow_is_the_open_option_list() {
    use micold_core::naming::ConventionalType;

    const STATE: &str = "add-worktree-dialog-type-menu-open";

    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let records = covered_states()
        .iter()
        .zip(all.iter())
        .find(|(covered, _)| covered.name == STATE)
        .map(|(_, records)| records)
        .unwrap_or_else(|| {
            panic!(
                "no covered state named {STATE} — the exemption below is about \
                                   that state's open list, so without it nothing proves the \
                                   attribution"
            )
        });

    let over = |path: &str| {
        records
            .iter()
            .find(|r| r.layer == lay::Layer::Overlay && lay::path_token(&r.path) == path)
    };

    for path in PICKER_LIST_CONTENT {
        let content = over(path).unwrap_or_else(|| {
            panic!(
                "{path} is exempted as a picker list's content and no such overlay node was \
                    resolved in {STATE}; the tree renumbered around it"
            )
        });
        let rows: Vec<_> = records
            .iter()
            .filter(|r| {
                r.layer == lay::Layer::Overlay
                    && r.path.len() == content.path.len() + 1
                    && r.path.starts_with(&content.path)
            })
            .collect();

        assert_eq!(
            rows.len(),
            ConventionalType::ALL.len(),
            "{path} holds {} children against {} conventional types, so it is not the open type \
             list's content and the exemption is attributed to the wrong node",
            rows.len(),
            ConventionalType::ALL.len(),
        );

        let stacked: f32 = rows.iter().map(|r| r.height).sum();
        assert!(
            (content.height - stacked).abs() < TOLERANCE,
            "{path} is {:.1}px tall against {stacked:.1}px of rows — a list's content is exactly \
             its rows, so this node is holding something else as well",
            content.height,
        );

        let cap = over(&lay::path_token(&content.path[..content.path.len() - 1]))
            .expect("the exempted node has a parent");
        assert!(
            content.height > cap.height + TOLERANCE,
            "{path} is {:.1}px inside a {:.1}px parent, so nothing is overflowing and the \
             exemption is covering a list that fits — which means the eight-row cap has stopped \
             applying, and with it the scrolling this exemption exists for",
            content.height,
            cap.height,
        );
    }
}

/// The recorded escapes are `Expand`'s, and this is what proves it rather than asserting it.
///
/// A zero-height parent holding a full-height child is `Expand::layout` at `progress` 0
/// (`animation.rs:741`) — but that reasoning is inference from a shape, and the same shape could
/// come from any collapsed container. So drive the one input that changes `Expand`'s progress and
/// nothing else: the sidebar's filter panel is `Accordion`, which *is* `expand(...)`. Closed, the
/// escape is there; open, the same node is clean. Nothing else about the state differs.
///
/// Without this, `CLIP_REVEALED` would be a list of paths whose cause was guessed.
#[test]
fn the_recorded_escapes_are_the_accordion_reveal() {
    let escaping_nodes = |filter_open: bool| -> Vec<String> {
        let mut workspace = crate::support::workspace_with(vec![("/fixture/project", vec![])]);
        workspace.active = workspace.projects.first().map(|p| p.path.clone());
        let state = micold_client::app::State {
            workspace,
            sidebar_width: 260,
            sidebar_filter_open: filter_open,
            ..micold_client::app::State::default()
        };

        let element = micold_client::ui::view(
            &state,
            None,
            None,
            0,
            None,
            &micold_core::env_include::EnvIncludeOutcome::Disabled,
            &micold_client::features::connection::ConnectionStatus::Connected,
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
         CLIP_REVEALED cannot be attributed to the accordion at all"
    );
    for node in &closed {
        assert!(
            !opened.contains(node),
            "node {node} escapes whether the filter panel is open or closed, so it is not the \
             accordion's reveal and the attribution recorded in CLIP_REVEALED is wrong"
        );
    }
}
