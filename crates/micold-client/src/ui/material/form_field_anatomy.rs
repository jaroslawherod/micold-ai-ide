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

/// The other control FR-031c names — and, since T048, one that composes its **own** `FormField`
/// rather than arriving bare. Handing it to a second `FormField` would draw two containers and two
/// indicators, so the builder is returned unbuilt and the assertions run over the composition the
/// select performs itself, which is the path the application takes.
fn select<'a>(roles: Roles) -> Select<'a, &'a str, Message> {
    const OPTIONS: &[&str] = &["one", "two"];
    Select::new(OPTIONS, None, |_| Message::NoOp, roles)
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

/// The **widget** tree's *arity* — how many children sit at each level, all the way down.
///
/// This tree rather than the layout one, because it is what decides whether state survives a
/// re-render: `Tree::diff` walks it, and a subtree it has to rebuild loses a text input's focus.
///
/// Arity rather than tags, deliberately. A slot holding a `Text` this frame and a placeholder the
/// next has a different *tag* at that position, and iced rebuilds that child alone — which is
/// harmless, because the child is the label. What must not change is the **number and order** of
/// children, because that is what fixes the control's own index in the tree. Comparing tags would
/// fail on a difference that costs nothing and would push toward making the placeholder imitate the
/// thing it stands in for, which is not the requirement.
fn tree_shape(element: &Element<'_, Message>) -> String {
    use iced::advanced::widget::Tree;

    fn walk(tree: &Tree, depth: usize, out: &mut String) {
        out.push_str(&format!("{}{}\n", "  ".repeat(depth), tree.children.len()));
        for child in &tree.children {
            walk(child, depth + 1, out);
        }
    }
    let mut out = String::new();
    walk(&Tree::new(element), 0, &mut out);
    out
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
    // The indicator is no longer a band of its own: `FilledField` draws it inside the box, so the
    // two are one shape rather than a line sitting under a container. Its thickness is asserted
    // where it is now decided — `the_resting_indicator_is_a_muted_hairline`, against the pure
    // function — and the band check that used to stand here would now be measuring the supporting
    // slot and passing for the wrong reason.
    assert_eq!(
        bands.len(),
        2,
        "the field should be two bands — the box and the supporting slot beneath it"
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

/// The tree has the same shape whatever the slots hold (feature 021's lesson).
///
/// The rendering stack rebuilds a subtree whose tag changed, and a text input's tag carries its own
/// state — focus included. A field that gained a child the moment a validation error appeared would
/// rebuild the input *while the user was typing into it* and drop the focus, so the next keystroke
/// would go nowhere. Feature 021 hit this with a search field whose clear button appeared on the
/// first keystroke; the same trap is here, and it opens on the error slot, which by definition
/// appears mid-typing.
///
/// This is the assertion an earlier version of this file got exactly backwards: it required an
/// absent slot to contribute *no* node, which is the shape change that causes the bug.
#[test]
fn the_shape_is_stable_whatever_the_slots_hold() {
    let r = roles();
    let glyph =
        |r| super::Glyph::<Message>::new(crate::icons::Icon::Close, super::TypeRole::Body, r);

    let bare = tree_shape(&FormField::new(input(), r).into());
    for (what, field) in [
        ("a label", FormField::new(input(), r).label("Name")),
        (
            "supporting text",
            FormField::new(input(), r).supporting("Lowercase only"),
        ),
        (
            "an error",
            FormField::new(input(), r).error(Some("Already exists")),
        ),
        (
            "a leading adornment",
            FormField::new(input(), r).leading(glyph(r)),
        ),
        (
            "a trailing adornment",
            FormField::new(input(), r).trailing(glyph(r)),
        ),
    ] {
        assert_eq!(
            tree_shape(&field.into()),
            bare,
            "adding {what} changed the widget tree's arity, so the control's index in the tree \
             moved. The renderer rebuilds what it cannot match up, and a text input's focus goes \
             with it — and the error slot is the one that fills while the user is typing"
        );
    }
}

/// …and an unfilled slot still takes no space.
///
/// The other half. Emitting every slot is only acceptable if an empty one is invisible; otherwise
/// every field in the application carries a silent gap where an icon would go.
#[test]
fn an_empty_slot_takes_no_space() {
    let r = roles();
    let glyph =
        |r| super::Glyph::<Message>::new(crate::icons::Icon::Close, super::TypeRole::Body, r);

    let (bare, bare_bands) = layout_of(FormField::new(input(), r).into(), 400.0);
    let (adorned, _) = layout_of(FormField::new(input(), r).trailing(glyph(r)).into(), 400.0);

    assert_eq!(
        bare.height, adorned.height,
        "an adornment changed the field's height, so the empty slot was not zero-sized"
    );
    assert_eq!(
        bare_bands[1].height, 0.0,
        "the empty supporting slot is {:.1}dp tall — it should be invisible",
        bare_bands[1].height
    );
}

/// The same assertions hold with a **select** wrapped (FR-031c).
///
/// The half a single-control test would not cover. The select is also the control that cannot
/// report focus, so it is the reason the wrapper takes its active state as a parameter.
///
/// The bands are compared against a wrapped text input rather than only against the contract's
/// number: a container fixed at 56dp satisfies "at least 56dp" no matter what is inside it, so the
/// height alone would pass even if the select were wearing chrome of its own as well.
#[test]
fn a_select_gets_the_same_chrome_as_a_text_input() {
    let r = roles();
    let wrapped: Element<'_, Message> = select(r).label("Type").supporting("Pick one").into();
    let (bounds, _) = layout_of(wrapped, 400.0);

    let want = tokens::density::height(tokens::density::TEXT_FIELD_BASE, tokens::density::STANDARD);
    assert!(
        bounds.height >= want,
        "a wrapped select is {:.1}dp tall against a {want}dp container",
        bounds.height
    );

    let bare: Element<'_, Message> = select(r).label("Type").into();
    let (bare_bounds, bare_bands) = layout_of(bare, 400.0);
    assert!(
        bounds.height > bare_bounds.height,
        "supporting text beneath a select added no height"
    );

    // Same chrome as the text input gets, band for band — one container and one indicator, not two.
    //
    // The *shape* rather than the heights, and that is the whole point of this assertion: the
    // container is `Length::Fixed`, so a field carrying a second container inside it measures 56dp
    // exactly like one that is not. Only the structure tells them apart.
    //
    // **The structure is read off the laid-out tree, not the widget tree** (feature 022). It used
    // to compare `tree_shape` against a wrapped text input's, which worked while the select was a
    // single leaf — a `pick_list` — sitting in the control slot exactly as a text input does. The
    // select assembles its own trigger now (a value, a spacer and a chevron) and does it inside
    // `Widget::layout`, so its *widget* tree is empty until it is laid out and its control slot is
    // legitimately deeper than an input's. Comparing arity there would now be comparing the two
    // controls' insides, which is not what this is about. What it is about — one container, one
    // indicator, four slots — is exactly what the laid-out bands say.
    let (input_bounds, input_bands) =
        layout_of(FormField::new(input(), r).label("Type").into(), 400.0);
    let box_slots = laid_out(select(r).label("Type").into(), 400.0).children()[0]
        .children()
        .len();
    assert_eq!(
        (bare_bands.len(), box_slots),
        (2, 4),
        "a labelled select resolves to {} bands with {box_slots} slots in the first, against the \
         shared chrome's two bands of four — the select is wearing chrome the text input does not \
         (a `FormField` around a control that composes its own draws two containers and two \
         indicators, and its content overflows the fixed 56dp container it is nested in)",
        bare_bands.len(),
    );
    assert_eq!(
        (
            bare_bounds.height,
            bare_bands[0].height,
            bare_bands[1].height
        ),
        (
            input_bounds.height,
            input_bands[0].height,
            input_bands[1].height
        ),
        "a labelled select's bands are {:?} against the text input's {:?}",
        (
            bare_bounds.height,
            bare_bands[0].height,
            bare_bands[1].height
        ),
        (
            input_bounds.height,
            input_bands[0].height,
            input_bands[1].height
        )
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

// ---------------------------------------------------------------------------------------------
// The label's two positions
// ---------------------------------------------------------------------------------------------

/// The bounds of the box's four slots — `[leading, control, trailing, label]`.
fn slots(element: Element<'_, Message>, width: f32) -> Vec<iced::Rectangle> {
    let node = laid_out(element, width);
    node.children()[0]
        .children()
        .iter()
        .map(|c| c.bounds())
        .collect()
}

/// An empty field rests its label where the value will go; a filled one floats it above.
///
/// This is the difference between a field that reads as Material and one that reads as a caption
/// stuck to a box. Empty, there is no value to sit under, so a label pinned to the top leaves a
/// visibly hollow container with text clinging to its ceiling — and the user has nothing telling
/// them the empty space below is where they type. Material puts the label *in* that space and
/// lifts it out of the way once there is something to lift it for.
///
/// Asserted as the label's centre against the box's, because that is the property: resting means
/// centred, floating means above centre. The exact 8dp is `filled_field`'s to state.
#[test]
fn an_empty_field_rests_its_label_and_a_filled_one_floats_it() {
    let r = roles();
    let empty: Element<'_, Message> = FormField::new(input(), r).label("Branch").into();
    let filled: Element<'_, Message> = FormField::new(input(), r)
        .label("Branch")
        .populated(true)
        .into();

    let box_height =
        tokens::density::height(tokens::density::TEXT_FIELD_BASE, tokens::density::STANDARD);
    let middle = box_height / 2.0;

    let resting = slots(empty, 400.0)[3];
    let floating = slots(filled, 400.0)[3];

    assert!(
        (resting.center_y() - middle).abs() < 1.0,
        "an empty field's label is centred at {:.1}dp in a {box_height}dp box, not at {middle}dp — \
         resting means sitting where the value will appear",
        resting.center_y()
    );
    assert!(
        floating.center_y() < middle - 4.0,
        "a filled field's label is centred at {:.1}dp, which is not clear of the {middle}dp middle \
         — it has to leave the value's line free",
        floating.center_y()
    );
    assert!(
        floating.height < resting.height,
        "the label floats at {:.1}dp and rests at {:.1}dp — floating is the *small* label, and one \
         that only moved would collide with the value it made way for",
        floating.height,
        resting.height
    );
}

/// A resting label leaves the control's line free of a second label.
///
/// The two occupy the same band by design, so whichever one draws text there has to be the only
/// one: a select showing `Select…` under a resting `Select` label prints the word twice, offset by
/// a pixel or two, which is exactly how the first version of this looked.
#[test]
fn a_resting_label_and_the_control_share_one_line() {
    let r = roles();
    let empty: Element<'_, Message> = FormField::new(input(), r).label("Branch").into();
    let s = slots(empty, 400.0);
    let (control, label) = (s[1], s[3]);
    assert!(
        (control.center_y() - label.center_y()).abs() < 1.0,
        "the control sits at {:.1}dp and the resting label at {:.1}dp — they are meant to be the \
         same line, so the caret lands on the label it replaces",
        control.center_y(),
        label.center_y()
    );
}

/// A leading adornment moves the **label** as well as the value (BUG-003 item 1).
///
/// Both start on one x, in either of the label's two positions. They did not: the control was inset
/// past the adornment and the label was pinned at the container's padding, so a field with a
/// leading icon and a resting label drew the label underneath the icon — the state a search picker
/// opens in, and the state T063 photographed at 600% zoom.
///
/// Asserted as an equality and not merely as "clear of the icon", because the defect is that there
/// were two rules for one column. A label that cleared the icon by its own separate arithmetic
/// would satisfy the weaker check and drift again at the next glyph.
#[test]
fn a_leading_adornment_moves_the_label_and_the_value_together() {
    let r = roles();
    let glyph =
        |r| super::Glyph::<Message>::new(crate::icons::Icon::Search, super::TypeRole::Action, r);

    for (state, field) in [
        (
            "resting",
            FormField::new(input(), r).label("Branch").leading(glyph(r)),
        ),
        (
            "floating",
            FormField::new(input(), r)
                .label("Branch")
                .populated(true)
                .leading(glyph(r)),
        ),
    ] {
        let s = slots(field.into(), 400.0);
        let (leading, control, label) = (s[0], s[1], s[3]);
        assert_eq!(
            label.x, control.x,
            "{state}: the label starts at {:.1}dp and the value at {:.1}dp — one column, two \
             rules, which is the whole of the defect",
            label.x, control.x
        );
        assert!(
            label.x >= leading.x + leading.width,
            "{state}: the label starts at {:.1}dp and the leading icon ends at {:.1}dp — they \
             overlap",
            label.x,
            leading.x + leading.width
        );
        assert_eq!(
            label.x,
            anatomy::text_field::PADDING
                + anatomy::text_field::LEADING_ICON
                + anatomy::text_field::LEADING_GAP,
            "{state}: the column is not padding + the fixed icon slot + the gap, so it follows the \
             glyph's own advance and moves with whichever icon the field carries (§7.2, BUG-006)"
        );
    }
}

/// …and without one, nothing is indented (the other half — a slot that costs nothing when empty).
#[test]
fn no_leading_adornment_means_no_indent() {
    let r = roles();
    let s = slots(FormField::new(input(), r).label("Branch").into(), 400.0);
    assert_eq!(s[3].x, anatomy::text_field::PADDING);
    assert_eq!(s[1].x, anatomy::text_field::PADDING);
}

/// An adornment sits on the container's centre line, not on the floating value's (BUG-003 item 1).
///
/// Material centres both icons in the 56dp box. Pinned instead to the top of the value line, a
/// leading icon sat 5dp below the resting label it is supposed to share a line with — so fixing the
/// horizontal collision alone would have replaced it with a vertical one.
#[test]
fn an_adornment_sits_on_the_fields_centre_line() {
    let r = roles();
    let glyph =
        |r| super::Glyph::<Message>::new(crate::icons::Icon::Search, super::TypeRole::Action, r);
    let middle =
        tokens::density::height(tokens::density::TEXT_FIELD_BASE, tokens::density::STANDARD) / 2.0;

    for (which, field) in [
        ("leading", FormField::new(input(), r).leading(glyph(r))),
        ("trailing", FormField::new(input(), r).trailing(glyph(r))),
    ] {
        let s = slots(field.into(), 400.0);
        let slot = if which == "leading" { s[0] } else { s[2] };
        assert!(
            (slot.center_y() - middle).abs() < 0.5,
            "the {which} adornment is centred at {:.1}dp in a box whose middle is {middle}dp",
            slot.center_y()
        );
    }
}

/// Focus tints the label as well as the indicator, and an error outranks focus (§7.7).
///
/// A focused field recolours *both*; the version that moved only the indicator left the label in
/// the muted role, and the field read as inactive with a coloured rule under it.
#[test]
fn the_label_follows_focus_and_error() {
    let r = roles();
    assert_eq!(style::field_label(r, false, false), r.on_surface_variant);
    assert_eq!(style::field_label(r, true, false), r.primary);
    assert_eq!(
        style::field_label(r, true, true),
        r.error,
        "an error has to outrank focus — a focused invalid field is still invalid"
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
