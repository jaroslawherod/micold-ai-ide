//! No text is drawn wider than the space it was given (feature 019, FR-018, SC-003).
//!
//! This is the gate the geometry fixture cannot be. The defect that motivated feature 019 — an
//! over-long sidebar label drawn across its close button — leaves layout untouched: the label is
//! `Length::Fill`, so its node is exactly the width its parent allots whether the bug is present or
//! not. `layout_snapshot.txt` is byte-identical either way; that was measured, not assumed.
//!
//! What changes is what the renderer *paints*. So this asks the renderer. For every piece of text
//! actually drawn, the shaped paragraph's natural width must not exceed the bounds it was clipped
//! to. Feature 017's fix commit named the same distinction: "only the node was bounded, and nothing
//! clips a paragraph to its node."

mod support;

use micold_core::theme::ColorScheme;
use support::covered_states::covered_states;
use support::layout as lay;

/// Overflows that already existed when this gate was built.
///
/// **Empty, and that is a result rather than an oversight.** One entry lived here briefly — the
/// empty-state prompt, reported as wanting ~279.8px in ~212.0px. It was a false positive from an
/// earlier `containing_width` that attributed text to the *narrowest* node containing its origin
/// instead of the deepest, so an overlapping sibling could steal the attribution. Correcting that
/// made the report disappear, and the staleness assertion below is what forced the entry out
/// rather than letting it linger as folklore.
///
/// If a real one is ever added: FR-019 forbids quietly fixing a defect surfaced while building the
/// gate, and equally forbids tuning the gate until the defect stops showing. Record what was
/// measured, keep the suite green, and require the entry to keep firing.
/// **Empty since 2026-08-07 (BUG-005), and the two entries it held were never defects.**
///
/// They were recorded as an artifact of the sidebar's collapsed filter panel: "the chip labelled
/// `Short` wants 28.9px in the 19.2px it is allowed". Measurement says otherwise. The text drawn
/// there is the sidebar's **`"Short"` worktree row label**, painted at (32, 146.8) and clipped by
/// its own widget to 164 × 15.6 — it fits, with 135px to spare. What it was measured against was a
/// *filter-panel chip node* of 24.65dp, which the collapsed panel had left lying at the same
/// coordinates: zero-height container, children keeping their own boxes, overlapping whatever the
/// sidebar drew beneath it. Two unrelated subtrees at one point, and the attribution rule handed
/// the text to the wrong one.
///
/// That is the same class the rule had already been changed once to fix — *narrowest* containing
/// node → *deepest* containing node. The change altered which stranger wins, not whether one can.
/// `support::layout::text_overflows` now identifies the owner by the **clip the painter actually
/// passed** and only falls back to the deepest containing node when no node matches it, which is
/// what closed this for good; see the comment there for why neither signal works alone.
///
/// BUG-005 is what exposed it: giving tree rows §7.2's height moved them apart from the collapsed
/// panel's overhang, the coincidental overlap stopped happening, and the staleness assertion below
/// fired — an exemption that had gone quiet. It was right to fire. The entries were removed after
/// the false positive was reproduced and its cause measured, not because they had become
/// inconvenient.
///
/// If a real one is ever added, the rules above still hold: record what was measured, keep the
/// suite green, and require the entry to keep firing.
const KNOWN_OVERFLOWS: &[&str] = &[];

/// Name an offending node by its anchor where one covers it, otherwise by its path (FR-004).
///
/// Added after quickstart Part B4 reported the motivating defect as a bare
/// `0/0/0/2/0/0/0/2/0/0/2/0/1`. Anchors exist so a failure can say *what* moved rather than
/// *where* it is in a tree the reader would have to reconstruct, and this gate was not consulting
/// them — so the one message the whole feature is justified by was the least legible one it emits.
fn named(covered: &lay::CoveredState, node_path: &str) -> String {
    covered
        .anchors
        .iter()
        .find(|a| lay::path_token(a.path) == node_path)
        .map(|a| format!("{} ({node_path})", a.name))
        .unwrap_or_else(|| format!("node {node_path}"))
}

fn known(node_path: &str) -> bool {
    KNOWN_OVERFLOWS.contains(&node_path)
}

/// No covered state paints text outside its clip, except the ones recorded above.
#[test]
fn no_text_is_drawn_wider_than_its_clip() {
    let mut fired: std::collections::BTreeSet<String> = Default::default();
    let mut renderer = lay::renderer();

    for covered in covered_states() {
        let mut under = (covered.build)();
        under.state.theme_pref = micold_core::theme::ThemePreference::Light;
        let _ = ColorScheme::Light;

        let element = micold_client::ui::view(
            &under.state,
            None,
            None,
            0,
            None,
            &micold_core::env_include::EnvIncludeOutcome::Disabled,
            &under.connection,
        );

        let overflows = lay::text_overflows(element, &mut renderer);

        let (exempt, unexpected): (Vec<_>, Vec<_>) =
            overflows.iter().partition(|o| known(&o.node_path));

        for o in &exempt {
            eprintln!(
                "KNOWN_OVERFLOWS still fires in {}: {:?} wants {:.1}px in {:.1}px at {}",
                covered.name,
                o.content,
                o.natural_width,
                o.allowed_width,
                named(covered, &o.node_path)
            );
            fired.insert(o.node_path.clone());
        }

        let overflows: Vec<_> = unexpected.into_iter().cloned().collect();

        assert!(
            overflows.is_empty(),
            "covered state {:?} paints {} piece(s) of text wider than the space allowed. The \
             widest overflows by {:.1}px ({:.1}px wanted, {:.1}px allowed). Text drawn past its \
             clip lands on whatever is beside it — this is the defect class feature 019 exists to \
             catch, and the geometry fixture cannot see it. Offenders: {:?}",
            covered.name,
            overflows.len(),
            overflows.iter().map(|o| o.excess()).fold(0.0_f32, f32::max),
            overflows
                .iter()
                .max_by(|a, b| a.excess().total_cmp(&b.excess()))
                .map(|o| o.natural_width)
                .unwrap_or_default(),
            overflows
                .iter()
                .max_by(|a, b| a.excess().total_cmp(&b.excess()))
                .map(|o| o.allowed_width)
                .unwrap_or_default(),
            overflows
                .iter()
                .map(|o| format!(
                    "{:?} wants {:.1}px in {:.1}px at {}",
                    o.content,
                    o.natural_width,
                    o.allowed_width,
                    named(covered, &o.node_path)
                ))
                .collect::<Vec<_>>(),
        );
    }

    let silent: Vec<&&str> = KNOWN_OVERFLOWS
        .iter()
        .filter(|path| !fired.contains(**path))
        .collect();
    assert!(
        silent.is_empty(),
        "every recorded overflow must still be observable somewhere, or the exemption is widening \
         the gate silently. These fired in no covered state: {silent:?}. If the defect was fixed, \
         delete the entry — a stale exemption is exactly the failure this feature exists to \
         prevent."
    );
}

/// The check must be able to *see* an overflow, or its silence means nothing.
///
/// `main-shell-sidebar-expanded` carries a deliberately over-long worktree name. With feature 017's
/// ellipsis fix in place it fits; the fix is what makes it fit. This asserts the machinery reports
/// a genuine overflow when handed one, so that a green run above is evidence rather than an
/// absence of evidence.
#[test]
fn the_check_reports_an_overflow_when_one_exists() {
    use iced::widget::{container, text};
    use iced::{Element, Length};

    // 900px of text in a 100px box. Nothing to do with the application — this proves the
    // instrument responds.
    let cramped: Element<'_, ()> = container(
        text("a string far too long to fit inside the narrow box it has been given here")
            .size(24.0)
            .wrapping(iced::advanced::text::Wrapping::None),
    )
    .width(Length::Fixed(100.0))
    .clip(true)
    .into();

    let mut renderer = lay::renderer();
    let overflows = lay::text_overflows(cramped, &mut renderer);

    assert!(
        !overflows.is_empty(),
        "the overflow check found nothing in a deliberately cramped layout — the instrument is \
         broken, and every passing run of the test above is meaningless"
    );
}

/// An overlapping stranger cannot be blamed for text it never drew (BUG-005).
///
/// This replaces `the_recorded_overflow_is_the_collapsed_filter_panel`, which proved the *opposite*
/// property — that closing the sidebar's filter panel produced an overflow and opening it removed
/// one. It did, and the overflow was not real: the collapsed panel is a zero-height `Expand` whose
/// children keep their own boxes, so a 24.65dp chip node came to rest on top of the sidebar's
/// `"Short"` row label, and the label's text was measured against the chip. Closing the panel moved
/// a stranger into place; it did not squeeze anything.
///
/// So the property worth holding is the one that was actually violated: the panel's collapsed state
/// must make **no difference** to what this gate reports, because it changes no text and no clip.
/// Driving the same single input proves it, and would fail again the moment attribution went back
/// to picking a node by geometry alone.
#[test]
fn a_collapsed_panel_overlapping_the_sidebar_is_not_reported_as_an_overflow() {
    let mut renderer = lay::renderer();

    let mut overflows = |filter_open: bool| -> Vec<lay::Overflow> {
        let mut state = (covered_states()[0].build)().state;
        state.sidebar_filter_open = filter_open;
        let element = micold_client::ui::view(
            &state,
            None,
            None,
            0,
            None,
            &micold_core::env_include::EnvIncludeOutcome::Disabled,
            &micold_client::features::connection::ConnectionStatus::Connected,
        );
        lay::text_overflows(element, &mut renderer)
    };

    let closed = overflows(false);
    assert!(
        closed.is_empty(),
        "collapsing the sidebar's filter panel reports {} overflow(s), but it changes no text and \
         no clip — it only slides a zero-height container's children over the rows beneath. Text \
         is being attributed to a node that did not draw it, which is what BUG-005 was: {closed:?}",
        closed.len(),
    );
    assert!(
        overflows(true).is_empty(),
        "the sidebar reports an overflow with its filter panel open, which is a real finding rather \
         than an attribution accident — report it as a bug"
    );
}
