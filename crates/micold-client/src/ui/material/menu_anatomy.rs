//! §7.5's *spatial* figures, read off the laid-out panel (BUG-003, FR-029).
//!
//! `anatomy_size` asks whether a component's box is the size the contract states, and for §7.5 it
//! asks that of the item's height — the one figure of the five that was applied. The other four are
//! **positions**, not sizes: how far an item's content is inset, how far the panel pads above the
//! first item, how big the leading glyph is, and what sits between two items. No existing check can
//! see any of them. The constants gate reads their values (all four are correct, and were read by
//! nothing), the snapshot records whatever it is shown, and `content_placement` rasterises one
//! component against its own box.
//!
//! That is how §7.5 shipped with one figure in five applied, and — because the same row was built by
//! hand in two modules — with the applied one reaching only one of the two panels.
//!
//! In-crate for the same reason as `anatomy_size` and `content_placement`: `material` is
//! `pub(crate)`, so none of this is constructible from `tests/`.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::Tree;
use iced::{Element, Length, Rectangle, Size};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, anatomy, density, Roles};

use super::{menu, menu_panel, MenuItem};
use crate::icons::Icon;
use crate::showcase::state::Message;

/// Room enough that nothing here is under pressure from the limit.
const ROOM: Size = Size::new(400.0, 600.0);

/// Layout arithmetic accumulates over a nested tree; far below anything a person could see, and far
/// below every gap this module separates (4dp against 8, 8dp against 12, 14dp against 24).
const TOLERANCE: f32 = 0.5;

fn roles() -> Roles {
    tokens::roles(ColorScheme::Light)
}

/// Two items, so "between the items" is a question that can be asked at all.
fn two_items() -> Vec<MenuItem<Message>> {
    vec![
        MenuItem::new(Icon::About, "About", Message::NoOp),
        MenuItem::new(Icon::Settings, "Settings", Message::NoOp),
    ]
}

/// The absolute bounds of the node at `path` once `element` is laid out in [`ROOM`].
///
/// Absolute rather than parent-relative: every figure here is a *gap between two nodes*, and
/// subtracting two absolute boxes is the same arithmetic the contract states. `Layout` resolves the
/// offsets, so the walk cannot get them wrong.
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
                "no child {index} at depth {depth} of {path:?} — the panel's tree changed shape, \
                 so this test is measuring something other than what it names"
            )
        });
    }
    layout.bounds()
}

/// A menu panel holding `items`, as both public entry points build it.
fn panel(items: Vec<MenuItem<Message>>) -> Element<'static, Message> {
    menu_panel(
        menu::item_column(items, roles()),
        Length::Fixed(ROOM.width),
        roles(),
        true,
        menu::panel_padding(),
    )
}

/// §7.5: the item's leading icon is 24dp.
///
/// Measured on the glyph node's width, which is the size it was asked to render at — a Material
/// Symbols glyph advances one em. It was `TypeRole::Action.size()`, the *label's* 14dp: the icon
/// took its size from the text beside it rather than from the row that states one.
#[test]
fn a_menu_items_leading_icon_is_24dp() {
    let icon = bounds_at(panel(two_items()), &[0, 0, 0, 0]);

    assert!(
        (icon.width - anatomy::menu::ITEM_ICON).abs() < TOLERANCE,
        "a menu item's leading icon measured {}dp wide, but §7.5 states {}dp",
        icon.width,
        anatomy::menu::ITEM_ICON,
    );
}

/// §7.5: an item's content is inset 12dp at both ends.
///
/// Both ends, not one: a single-ended check passes on a row that is pushed sideways, and the
/// horizontal padding is the figure that decides where every label in the panel starts.
#[test]
fn a_menu_items_content_is_inset_by_12dp_at_both_ends() {
    let item = bounds_at(panel(two_items()), &[0, 0]);
    let content = bounds_at(panel(two_items()), &[0, 0, 0]);

    let leading = content.x - item.x;
    let trailing = (item.x + item.width) - (content.x + content.width);

    assert!(
        (leading - anatomy::menu::ITEM_PADDING).abs() < TOLERANCE
            && (trailing - anatomy::menu::ITEM_PADDING).abs() < TOLERANCE,
        "a menu item's content is inset {leading}dp at the leading edge and {trailing}dp at the \
         trailing one, but §7.5 states {}dp at both",
        anatomy::menu::ITEM_PADDING,
    );
}

/// §7.5: the panel puts 8dp above the first item and below the last.
#[test]
fn a_menu_panel_pads_8dp_above_its_first_item_and_below_its_last() {
    let panel_box = bounds_at(panel(two_items()), &[]);
    let first = bounds_at(panel(two_items()), &[0, 0]);
    let last = bounds_at(panel(two_items()), &[0, 1]);

    let above = first.y - panel_box.y;
    let below = (panel_box.y + panel_box.height) - (last.y + last.height);

    assert!(
        (above - anatomy::menu::VERTICAL_PADDING).abs() < TOLERANCE
            && (below - anatomy::menu::VERTICAL_PADDING).abs() < TOLERANCE,
        "a menu panel puts {above}dp above its first item and {below}dp below its last, but §7.5 \
         states {}dp",
        anatomy::menu::VERTICAL_PADDING,
    );
}

/// §7.5: items abut. A gap between them is what the contract's divider is for.
///
/// The 4dp gap this replaces was invisible while items were 36dp — the panel simply read as loose.
/// It stopped being invisible when the items became 48dp and the panel grew 60dp in one change,
/// which is what put a five-item menu across the bottom of a short window.
#[test]
fn menu_items_abut_one_another() {
    let first = bounds_at(panel(two_items()), &[0, 0]);
    let second = bounds_at(panel(two_items()), &[0, 1]);

    let gap = second.y - (first.y + first.height);

    assert!(
        gap.abs() < TOLERANCE,
        "consecutive menu items are {gap}dp apart. §7.5 gives an item a height and a divider; it \
         gives the space between two items nothing, because there is none",
    );
}

/// The panel's height is exactly what `menu_panel_size` predicts — the estimate the anchor clamping
/// uses to keep a cursor-anchored menu on screen (feature 015).
///
/// It is derived from the same tokens the panel is built from, which is what makes it right; this
/// is what would notice the two coming apart. They already did once in the other direction: while
/// §7.5's height went unapplied the estimate reproduced the real 36dp by restating the arithmetic
/// that produced it, so it tracked the defect instead of the contract.
#[test]
fn the_clamping_estimate_matches_the_panel_it_estimates() {
    let items = two_items();
    let (_, predicted) = menu::menu_panel_size(items.len());
    let measured = bounds_at(panel(items), &[]);

    assert!(
        (measured.height - predicted as f32).abs() < TOLERANCE,
        "`menu_panel_size` predicts {predicted}dp for a two-item panel that lays out at \
         {}dp. The estimate is what stops a cursor-anchored menu opening off-screen, so it has to \
         move with the panel rather than after it",
        measured.height,
    );
}

/// The same figures hold for the project switcher, because it is the same row (FR-029b).
///
/// This is the assertion the duplication made impossible: §7.5's item height was applied to
/// `menu.rs` in T098 and the switcher's own hand-built copy stayed at 36dp, so two panels hanging
/// off the same bar shipped 12dp apart. Reading both through one entry point is the fix; reading
/// them through one *test* is what keeps them there.
#[test]
fn a_switcher_row_is_the_same_row_as_a_menu_item() {
    let rows = super::project_switcher::row_column(
        vec![super::ProjectRow {
            label: "micold-ai-ide".to_string(),
            is_active: true,
            running_count: 2,
            available: true,
            on_select: Message::NoOp,
            on_context: Some(Message::NoOp),
        }],
        Message::NoOp,
        roles(),
    );

    let mut element = rows;
    let renderer = super::test_support::renderer();
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, ROOM),
    );
    let row = Layout::new(&node)
        .children()
        .next()
        .expect("the switcher's column must hold its project row")
        .bounds();

    assert!(
        (row.height - density::MENU_ITEM_BASE).abs() < TOLERANCE,
        "a project-switcher row measured {}dp tall against §7.5's {}dp menu item. Both panels hang \
         off the same app bar and are built from the same table; a row that is its own component is \
         a row that gets fixed once and stays broken elsewhere (FR-029b)",
        row.height,
        density::MENU_ITEM_BASE,
    );
}
