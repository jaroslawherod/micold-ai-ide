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
const KNOWN_OVERFLOWS: &[(&str, &str)] = &[];

fn known(state: &str) -> bool {
    KNOWN_OVERFLOWS.iter().any(|(s, _)| *s == state)
}

/// No covered state paints text outside its clip, except the ones recorded above.
#[test]
fn no_text_is_drawn_wider_than_its_clip() {
    let mut fired: std::collections::BTreeSet<&str> = Default::default();
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

        if known(covered.name) {
            assert!(
                !overflows.is_empty(),
                "covered state {:?} is recorded in KNOWN_OVERFLOWS but no longer overflows. If it \
                 was fixed, delete the entry — a stale exemption widens the gate silently.",
                covered.name
            );
            eprintln!(
                "KNOWN_OVERFLOWS[{}] still fires: {}",
                covered.name,
                overflows
                    .iter()
                    .map(|o| format!(
                        "{:?} wants {:.1}px in {:.1}px at node {}",
                        o.content, o.natural_width, o.allowed_width, o.node_path
                    ))
                    .collect::<Vec<_>>()
                    .join("; ")
            );
            fired.insert(covered.name);
            continue;
        }

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
                    "{:?} wants {:.1}px in {:.1}px at node {}",
                    o.content, o.natural_width, o.allowed_width, o.node_path
                ))
                .collect::<Vec<_>>(),
        );
    }

    assert_eq!(
        fired.len(),
        KNOWN_OVERFLOWS.len(),
        "every recorded overflow must still be observable; {:?} fired",
        fired
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
