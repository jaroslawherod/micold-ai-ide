//! §7.3's *spatial* figures, read off the laid-out button (FR-027).
//!
//! `anatomy_size` asks whether a component's box is the size the contract states, and for §7.3 it
//! asks that of the 40dp height and the 48dp touch target — the two figures of the table that were
//! applied. The rest are not sizes of the box:
//!
//! - the **horizontal paddings** are an inset. A button is content-sized across, so its width is
//!   whatever its label plus its inset happens to be, and `Extent::Content` passes at any inset at
//!   all. Every labelled button took the rendering stack's `DEFAULT_PADDING` of 10dp while §7.3
//!   said 24, 24 and 12.
//! - the **icon sizes** belong to a glyph inside the button. An icon button's glyph was
//!   `TypeRole::Body.size()` — 14dp, the size of the *body text*, against §7.3's 24. That is the
//!   defect BUG-003's T103 found one component over, where a menu item's leading glyph took the
//!   size of the label beside it rather than the size the contract gives it.
//!
//! In-crate for the same reason as `anatomy_size`, `content_placement` and `menu_anatomy`:
//! `material` is `pub(crate)`, so none of this is constructible from `tests/`.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::Tree;
use iced::{Element, Rectangle, Size};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{anatomy, spacing, Roles};

use super::{Button, ButtonVariant, IconButton, Text, TypeRole};
use crate::icons::Icon;
use crate::showcase::state::Message;

/// Room enough that nothing here is under pressure from the limit.
const ROOM: Size = Size::new(400.0, 300.0);

/// Layout arithmetic accumulates over a nested tree; far below anything a person could see, and far
/// below every figure this module separates (10dp against 24, 4dp against 8, 14dp against 24).
const TOLERANCE: f32 = 0.5;

fn roles() -> Roles {
    micold_core::tokens::roles(ColorScheme::Light)
}

/// The absolute bounds of the node at `path` once `element` is laid out in [`ROOM`].
///
/// Absolute rather than parent-relative: an inset is the gap between two boxes, and subtracting two
/// absolute boxes is the same arithmetic §7.3 states. `Layout` resolves the offsets, so the walk
/// cannot get them wrong.
fn bounds_at(element: Element<'_, Message>, path: &[usize]) -> Rectangle {
    let mut element = element;
    let renderer = super::test_support::renderer();
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, ROOM),
    );

    let mut layout = Layout::new(&node);
    for (depth, &index) in path.iter().enumerate() {
        layout = layout.children().nth(index).unwrap_or_else(|| {
            panic!(
                "no child {index} at depth {depth} of {path:?} — the button's tree changed shape, \
                 so this test is measuring something other than what it names"
            )
        });
    }
    layout.bounds()
}

/// A labelled button of `variant`, pressable so it takes the shape the application ships.
fn labelled(variant: ButtonVariant) -> Element<'static, Message> {
    Button::with_content(
        Text::new("Open", TypeRole::Action, roles()),
        variant,
        roles(),
    )
    .on_press(Message::NoOp)
    .into()
}

/// How far a labelled button insets its content from its own leading edge.
///
/// The content node is child 0: `Ripple` wraps without adding a layout node, and the centring
/// container T097 added for FR-030a is that child.
fn label_inset(variant: ButtonVariant) -> f32 {
    let outer = bounds_at(labelled(variant), &[]);
    let content = bounds_at(labelled(variant), &[0]);
    content.x - outer.x
}

/// §7.3: the filled and outlined variants inset their label by 24dp, the text variant by 12dp.
///
/// All three in one test because they are one row of the variant table and differ only in the
/// number — a variant that acquired its own inset would be the same drift the height row had.
#[test]
fn a_labelled_button_insets_its_label_by_the_padding_its_variant_states() {
    for (name, variant, expected) in [
        (
            "a filled button",
            ButtonVariant::Filled,
            anatomy::button::PADDING_FILLED,
        ),
        (
            "an outlined button",
            ButtonVariant::Outlined,
            anatomy::button::PADDING_OUTLINED,
        ),
        (
            "a text button",
            ButtonVariant::Text,
            anatomy::button::PADDING_TEXT,
        ),
    ] {
        let measured = label_inset(variant);
        assert!(
            (measured - expected).abs() < TOLERANCE,
            "{name} inset its label by {measured}dp, but §7.3 states {expected}dp. The rendering \
             stack's `DEFAULT_PADDING` is 10dp on this axis, and a button that states no padding \
             takes it."
        );
    }
}

/// §7.3: an icon button's glyph is 24dp — the icon column's own figure, not the body text's.
///
/// Measured on the glyph node's width, which is the size it was asked to render at: a Material
/// Symbols glyph advances one em. It was `TypeRole::Body.size()`, 14dp — the button took its
/// glyph's size from the type scale, where §7.3 gives it a number of its own.
#[test]
fn an_icon_buttons_glyph_is_the_24dp_its_column_states() {
    let button: Element<'_, Message> = IconButton::new(Icon::Menu, roles())
        .on_press(Message::NoOp)
        .into();
    let glyph = bounds_at(button, &[0, 0]);

    assert!(
        (glyph.width - anatomy::button::ICON_BUTTON_GLYPH).abs() < TOLERANCE,
        "an icon button's glyph measured {}dp wide, but §7.3 states {}dp",
        glyph.width,
        anatomy::button::ICON_BUTTON_GLYPH,
    );
}

/// §7.3: an icon button insets its glyph by 8dp, which is what puts a 24dp glyph in a 40dp
/// container — the sentence beneath the variant table, and the reason the target is stated
/// separately at 48.
///
/// Measured inside the target wrapper: child 0 of the 48dp container is the visible pill, and its
/// own child is the glyph.
#[test]
fn an_icon_button_insets_its_glyph_by_the_8dp_its_column_states() {
    let button: Element<'_, Message> = IconButton::new(Icon::Menu, roles())
        .on_press(Message::NoOp)
        .into();
    let pill = bounds_at(button, &[0]);
    let button2: Element<'_, Message> = IconButton::new(Icon::Menu, roles())
        .on_press(Message::NoOp)
        .into();
    let glyph = bounds_at(button2, &[0, 0]);

    let inset = glyph.x - pill.x;
    assert!(
        (inset - anatomy::button::PADDING_ICON).abs() < TOLERANCE,
        "an icon button inset its glyph by {inset}dp, but §7.3 states {}dp",
        anatomy::button::PADDING_ICON,
    );
}

/// FR-045's recorded deviation covers the glyph as well as the target.
///
/// The sidebar keeps both the tighter inset and the body role's smaller glyph, and the evidence for
/// the second is `tests/layout_text_overflow.rs`: at §7.3's 24dp the collapsed sidebar paints a
/// glyph into a 15dp slot and the expanded header squeezes "Worktrees" below the width it needs,
/// because four controls at 24dp take 40dp more out of a ~260dp panel than four at 14dp.
///
/// Pinned so the deviation stays a decision rather than a drift: this is what FR-045 gives up, and
/// nothing more. The non-compact path above is what §7.3 gets.
#[test]
fn a_compact_icon_button_keeps_the_sidebars_smaller_glyph_and_inset() {
    let outer = bounds_at(
        IconButton::new(Icon::Menu, roles())
            .compact()
            .on_press(Message::NoOp)
            .into(),
        &[],
    );
    let glyph = bounds_at(
        IconButton::new(Icon::Menu, roles())
            .compact()
            .on_press(Message::NoOp)
            .into(),
        &[0],
    );

    assert!(
        (glyph.width - TypeRole::Body.size()).abs() < TOLERANCE,
        "a compact icon button's glyph measured {}dp wide; FR-045 keeps it at the body role's \
         {}dp, because §7.3's {}dp does not fit the sidebar",
        glyph.width,
        TypeRole::Body.size(),
        anatomy::button::ICON_BUTTON_GLYPH,
    );

    let expected = TypeRole::Body.size() + spacing::XS * 2.0;
    assert!(
        (outer.width - expected).abs() < TOLERANCE,
        "a compact icon button measured {}dp wide; FR-045 keeps it at its natural size, which is \
         the glyph plus the sidebar's `spacing::XS` on each side ({expected}dp)",
        outer.width,
    );
}

/// §7.3: a leading icon inside a labelled button is 18dp — smaller than an icon button's 24, which
/// is the whole content rather than an accent to a label.
///
/// The slot is the component's, not the call site's. Two call sites built `row![Glyph, Text]` by
/// hand and sized the glyph `TypeRole::Action` (14dp), which is the label's size — the same shape
/// as the icon button above, and as the menu item T103 found.
#[test]
fn a_buttons_leading_icon_is_the_18dp_its_row_states() {
    let r = roles();
    let build = || -> Element<'static, Message> {
        Button::filled("Open this folder", r)
            .leading(Icon::OpenProject, r.on_primary)
            .on_press(Message::NoOp)
            .into()
    };
    // The centring container, then the icon-and-label row, then the glyph.
    let glyph = bounds_at(build(), &[0, 0, 0]);

    assert!(
        (glyph.width - anatomy::button::LEADING_ICON).abs() < TOLERANCE,
        "a button's leading icon measured {}dp wide, but §7.3 states {}dp",
        glyph.width,
        anatomy::button::LEADING_ICON,
    );
}
