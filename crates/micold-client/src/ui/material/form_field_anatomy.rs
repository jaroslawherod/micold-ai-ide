//! `FormField` composes the shared chrome around whatever control it is given (feature 018, T044a —
//! FR-031a, FR-031b, FR-031c).
//!
//! In-crate rather than in `tests/`, for the same reason as the style snapshots: `material` is
//! `pub(crate)`, so a `FormField` cannot be constructed from outside the crate at all. tasks.md
//! names `crates/micold-client/tests/form_field_anatomy.rs`; that path is not reachable, and the
//! precedent for the correction was set by `style_snapshot.rs`.
//!
//! # What is worth asserting here
//!
//! The *decisions*, not the pixels. Two kinds:
//!
//! - **The colour and thickness of the chrome** is a pure function of `(roles, active, error)`, so
//!   it is checked directly. That covers the requirement with no renderer and no ambiguity: an
//!   indicator that thickens without recolouring, or an error state that recolours the supporting
//!   text and leaves the label muted, is caught by arithmetic.
//! - **The composition** — label inside the container above the value, supporting text beneath it,
//!   adornments that take no space when absent — is a layout question, so it is laid out and
//!   measured. `FormField` wraps *whichever* control it is handed, so the same assertions run over
//!   a text input and over a select, which is the half of FR-031c that a single-control test would
//!   quietly not cover.

use iced::widget::text_input;
use iced::{Element, Length, Size};
use micold_core::tokens::{self, anatomy, Roles};

use super::style;
use super::{FormField, Select};
use crate::showcase::state::Message;

fn roles() -> Roles {
    tokens::roles(micold_core::theme::ColorScheme::Light)
}

/// A plain text input, the control a form field most often wraps.
fn input<'a>() -> Element<'a, Message> {
    text_input("", "").into()
}

/// The other control FR-031c names. Wrapping it must work identically — the select cannot report
/// focus, so it is the case that proves the wrapper takes its active state rather than assuming it.
fn select<'a>(roles: Roles) -> Element<'a, Message> {
    const OPTIONS: &[&str] = &["one", "two"];
    Select::new(OPTIONS, None, |_| Message::NoOp, roles).into()
}

// ---------------------------------------------------------------------------------------------
// The chrome's colour and thickness — pure, so checked as such
// ---------------------------------------------------------------------------------------------

/// At rest: a hairline in the muted foreground.
#[test]
fn the_resting_indicator_is_a_muted_hairline() {
    let r = roles();
    let (colour, thickness) = style::field_indicator(r, false, false);
    assert_eq!(thickness, anatomy::text_field::INDICATOR);
    assert_eq!(colour, style::color(r.on_surface_variant));
}

/// Active: thicker *and* in the accent. Both, together — an indicator that thickened without
/// recolouring would read as a rendering artefact rather than as focus.
#[test]
fn the_active_indicator_thickens_and_takes_the_accent() {
    let r = roles();
    let (resting, resting_w) = style::field_indicator(r, false, false);
    let (active, active_w) = style::field_indicator(r, true, false);

    assert_eq!(active_w, anatomy::text_field::INDICATOR_ACTIVE);
    assert!(
        active_w > resting_w,
        "the active indicator is no thicker than the resting one, so focus is invisible"
    );
    assert_eq!(active, style::color(r.primary));
    assert_ne!(
        active, resting,
        "the active indicator is the same colour as the resting one"
    );
}

/// Invalid outranks active. A field that is both focused and invalid is invalid, and showing it in
/// the accent would say the opposite of what the supporting text says.
#[test]
fn the_error_indicator_outranks_the_active_one() {
    let r = roles();
    let (errored, _) = style::field_indicator(r, false, true);
    let (errored_while_active, thickness) = style::field_indicator(r, true, true);

    assert_eq!(errored, style::color(r.error));
    assert_eq!(
        errored_while_active,
        style::color(r.error),
        "a focused invalid field showed the accent, contradicting its own supporting text"
    );
    assert_eq!(
        thickness,
        anatomy::text_field::INDICATOR_ACTIVE,
        "an invalid field that is also focused still reads as focused"
    );
}

/// The label and the supporting text move together into the error role (§7.7).
#[test]
fn the_label_and_supporting_text_share_the_error_state() {
    let r = roles();
    assert_eq!(style::field_support(r, false), r.on_surface_variant);
    assert_eq!(style::field_support(r, true), r.error);
}

// ---------------------------------------------------------------------------------------------
// Composition — laid out and measured
// ---------------------------------------------------------------------------------------------

/// Lay `element` out at a fixed width and return its whole node tree.
fn laid_out(element: Element<'_, Message>, width: f32) -> iced::advanced::layout::Node {
    use iced::advanced::layout;
    use iced::advanced::widget::Tree;

    let mut element = element;
    let renderer = super::test_support::renderer();
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, Size::new(width, 2000.0));
    element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits)
}

/// The bounds of a laid-out element at a fixed width, plus the bounds of its direct children.
///
/// The direct children are the field's three bands: the filled container, the active indicator and
/// — when there is one — the supporting text. Asserting on them is what makes "56dp container" and
/// "1dp indicator" checkable rather than inferred from the total.
fn layout_of(element: Element<'_, Message>, width: f32) -> (iced::Rectangle, Vec<iced::Rectangle>) {
    let node = laid_out(element, width);
    let children = node.children().iter().map(|c| c.bounds()).collect();
    (node.bounds(), children)
}

/// How many layout nodes the element produces, all the way down.
///
/// An absent adornment must contribute *nothing* — not a zero-width spacer, which would look
/// identical here and drift into a real gap the moment someone gave the row a spacing.
fn node_count(node: &iced::advanced::layout::Node) -> usize {
    1 + node.children().iter().map(node_count).sum::<usize>()
}

/// The container is at least the contract's field height (§7.7).
///
/// Today's field is roughly 30dp — padding around a line of text — which is the single largest
/// anatomical departure this feature corrects.
#[test]
fn the_field_is_at_least_the_contract_height() {
    let r = roles();
    let field: Element<'_, Message> = FormField::new(input(), r).label("Branch name").into();
    let (_, bands) = layout_of(field, 400.0);

    let want = tokens::density::height(tokens::density::TEXT_FIELD_BASE, tokens::density::STANDARD);
    assert_eq!(
        bands[0].height, want,
        "the filled container is {:.1}dp against the contract's {want}dp — today's field is roughly \
         30dp, and this is the largest single anatomical departure the feature corrects",
        bands[0].height
    );
    assert_eq!(
        bands[1].height,
        anatomy::text_field::INDICATOR,
        "the resting active indicator is not a hairline"
    );
}

/// Supporting text renders *beneath* the container rather than inside it (§7.7).
///
/// Measured as height rather than asserted structurally: adding supporting text must make the whole
/// field taller, which is only true if it is outside the container.
#[test]
fn supporting_text_adds_height_beneath_the_container() {
    let r = roles();
    let bare: Element<'_, Message> = FormField::new(input(), r).label("Ticket").into();
    let supported: Element<'_, Message> = FormField::new(input(), r)
        .label("Ticket")
        .supporting("Optional — e.g. ABC-123")
        .into();

    let (bare_bounds, _) = layout_of(bare, 400.0);
    let (supported_bounds, _) = layout_of(supported, 400.0);

    assert!(
        supported_bounds.height > bare_bounds.height,
        "supporting text added no height ({:.1} vs {:.1}), so it is inside the container rather \
         than beneath it",
        supported_bounds.height,
        bare_bounds.height
    );
}

/// An adornment slot takes no space when it is not supplied.
///
/// The failure this rules out is a wrapper that always reserves room for a trailing icon: every
/// field in the application would carry a silent 24dp gap on its right, and nothing would look
/// broken enough to notice.
#[test]
fn an_absent_adornment_contributes_no_element() {
    let r = roles();
    let glyph =
        |r| super::Glyph::<Message>::new(crate::icons::Icon::Close, super::TypeRole::Body, r);

    let plain = node_count(&laid_out(
        FormField::new(input(), r).label("Name").into(),
        400.0,
    ));
    let trailing = node_count(&laid_out(
        FormField::new(input(), r)
            .label("Name")
            .trailing(glyph(r))
            .into(),
        400.0,
    ));
    let leading = node_count(&laid_out(
        FormField::new(input(), r)
            .label("Name")
            .leading(glyph(r))
            .into(),
        400.0,
    ));

    assert!(
        trailing > plain,
        "a trailing adornment produced no extra layout node, so it is not being rendered at all"
    );
    assert_eq!(
        leading, trailing,
        "one adornment slot costs more nodes than the other"
    );
    assert_eq!(
        plain,
        trailing - 1,
        "the field without adornments carries {} nodes against {} with one — an empty slot is \
         reserving something, and a spacer that is zero-wide today becomes a real gap the moment \
         the row is given a spacing",
        plain,
        trailing
    );
}

/// The same assertions hold with a **select** wrapped (FR-031c).
///
/// The half a single-control test would not cover. The select is also the control that cannot
/// report focus, so it is the reason the wrapper takes its active state as a parameter.
#[test]
fn a_select_gets_the_same_chrome_as_a_text_input() {
    let r = roles();
    let wrapped: Element<'_, Message> = FormField::new(select(r), r)
        .label("Type")
        .supporting("Pick one")
        .into();
    let (bounds, _) = layout_of(wrapped, 400.0);

    let want = tokens::density::height(tokens::density::TEXT_FIELD_BASE, tokens::density::STANDARD);
    assert!(
        bounds.height >= want,
        "a wrapped select is {:.1}dp tall against a {want}dp container",
        bounds.height
    );

    let bare: Element<'_, Message> = FormField::new(select(r), r).label("Type").into();
    let (bare_bounds, _) = layout_of(bare, 400.0);
    assert!(
        bounds.height > bare_bounds.height,
        "supporting text beneath a select added no height"
    );
}

/// A field with no label still lays out — not every control needs one, and a wrapper that panicked
/// or collapsed without one would make the label mandatory by accident.
#[test]
fn a_field_without_a_label_still_lays_out() {
    let r = roles();
    let field: Element<'_, Message> = FormField::new(input(), r).into();
    let (bounds, _) = layout_of(field, 400.0);
    assert!(bounds.height > 0.0 && bounds.width > 0.0);
}

/// The field fills the width it is given, so a dialog's fields line up with each other rather than
/// each shrinking to its own content.
#[test]
fn the_field_fills_the_width_it_is_offered() {
    let r = roles();
    let field: Element<'_, Message> = FormField::new(input(), r).label("Name").into();
    let (bounds, _) = layout_of(field, 400.0);
    assert!(
        (bounds.width - 400.0).abs() < 0.5,
        "the field is {:.1}dp wide in a 400dp slot",
        bounds.width
    );
}

/// The wrapper reports `Length::Fill` for its width, which is what makes the test above true of
/// every caller rather than of this one arrangement.
#[test]
fn the_wrapper_declares_a_filling_width() {
    let r = roles();
    let field: Element<'_, Message> = FormField::new(input(), r).into();
    assert_eq!(field.as_widget().size().width, Length::Fill);
}
