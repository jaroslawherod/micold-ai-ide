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
/// **One entry as of 2026-08-04**, and it is a collapsed clip-reveal rather than a defect.
///
/// The sidebar's filter panel is `material::Expand`. Collapsed, it reports zero height while its
/// children keep theirs, and the tag chips inside are laid out in a space that squeezes them — the
/// chip labelled `"Short"` wants 28.9px in the 19.2px it is allowed. Nothing paints there:
/// `Expand::draw` returns early below `HIDDEN`, which is the same reason `CLIP_REVEALED` exists in
/// the containment gate.
///
/// **Attribution is proven, not argued** — `the_recorded_overflow_is_the_collapsed_filter_panel`
/// opens the panel and shows the same text comes clean. Closed: one overflow. Open: none. A real
/// clipping defect would not care whether its container was expanded.
///
/// Surfaced by upstream's typography change (feature 018 shipping Roboto with per-role weight and
/// line height), which widened the label past what the collapsed chip allows. Recorded rather than
/// fixed: FR-019 forbids this feature changing application source.
/// **Keyed by node path, not by `(state, path)`.** Being inside a collapsed clip-reveal is a
/// property of the widget, not of the screen it appears on — the same lesson T032 taught the
/// containment gate, which had made FR-016 false in practice by demanding a second entry per new
/// sidebar-bearing state.
const KNOWN_OVERFLOWS: &[&str] = &[
    "0/0/0/2/0/0/0/1/0/0/1/0/0/0",
    // The same chip with the disconnection banner in the shell, which shifts the index from 2 to
    // 3 — the check following structure rather than a hardcoded path, as in `CLIP_REVEALED`.
    "0/0/0/3/0/0/0/1/0/0/1/0/0/0",
];

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
            overflows
                .iter()
                .map(|o| o.excess())
                .fold(0.0_f32, f32::max),
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


/// The recorded overflow is the collapsed filter panel, and this is what proves it.
///
/// A zero-height container squeezing its children is inference from a shape; the same shape could
/// come from a genuinely too-narrow chip. So drive the one input that changes it and nothing else.
/// Closed, the overflow is there; open, the same text fits. That is the difference between an
/// artifact of a collapsed clip-reveal and a defect a user would meet.
#[test]
fn the_recorded_overflow_is_the_collapsed_filter_panel() {
    let mut renderer = lay::renderer();

    let mut overflow_count = |filter_open: bool| -> usize {
        let mut state = (covered_states()[0].build)().state;
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
        lay::text_overflows(element, &mut renderer).len()
    };

    assert!(
        overflow_count(false) > 0,
        "the sidebar with its filter panel closed reports no overflow, so the KNOWN_OVERFLOWS \
         entry cannot be attributed to the collapsed panel at all"
    );
    assert_eq!(
        overflow_count(true),
        0,
        "text still overflows with the filter panel open, so this is a real clipping defect rather \
         than an artifact of the collapsed reveal — it must be reported as a bug, not exempted"
    );
}
