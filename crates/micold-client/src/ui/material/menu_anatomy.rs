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

/// One switcher row, as `ui::view` builds them — a [`MenuItem`], which is the whole point: the
/// switcher's list stopped being a component of its own at BUG-007, so a row of it is expressible
/// here without naming anything the switcher owns.
fn switcher_row(label: &str, active: bool) -> MenuItem<Message> {
    MenuItem {
        icon: active.then_some(Icon::ActiveMarker),
        reserve_icon: true,
        icon_tint: Some(crate::icons::icon_role(
            crate::icons::IconSurface::Badge,
            roles(),
        )),
        label: label.to_string(),
        message: Some(Message::NoOp),
        trailing_text: None,
        trailing_icon: None,
        on_context: Some(Box::new(|_| Message::NoOp)),
    }
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

/// §7.5: the item's leading icon is `on_surface_variant`.
///
/// The other half of the same table row, and the half that stayed unapplied when the 24dp landed —
/// which is this bug's own shape repeated inside its own fix, so it is asserted rather than left to
/// a reviewer. `on_surface` is the label's tone; a leading glyph is the quieter of the two, and
/// `IconSurface` is where that decision belongs rather than in a literal at the call site.
///
/// An item that states its own tint keeps it: the switcher's active marker is a `Badge` by
/// intent (FR-006 of feature 008), and this is the default, not an override.
#[test]
fn a_menu_items_leading_icon_is_on_surface_variant() {
    let r = roles();
    let item = MenuItem::new(Icon::About, "About", Message::NoOp);

    assert_eq!(
        menu::leading_tint(&item, r),
        r.on_surface_variant,
        "§7.5 tints a menu item's leading icon `on_surface_variant`; `on_surface` is its label's \
         tone, which is the louder of the two",
    );
}

/// An item that states a tint is not overridden by the default above.
#[test]
fn an_item_that_states_its_own_tint_keeps_it() {
    let r = roles();
    let marker = MenuItem {
        icon_tint: Some(crate::icons::icon_role(
            crate::icons::IconSurface::Unavailable,
            r,
        )),
        ..MenuItem::new(Icon::Unavailable, "Unavailable", Message::NoOp)
    };

    assert_eq!(menu::leading_tint(&marker, r), r.error);
}

/// FR-006a: a switcher row without the active marker starts its label where the marked row does.
///
/// The marker's slot is reserved rather than conditional. A marker that appeared *and* moved the
/// label would be doing two jobs, and the second is the louder — a quiet indicator of which project
/// is active would read as a list that cannot keep its rows in line. The type-ahead's rows have
/// always worked this way; the switcher's did not, because its leading slot is per-item.
#[test]
fn a_switcher_row_without_the_marker_aligns_with_one_that_has_it() {
    let rows = menu::item_column(
        vec![
            switcher_row("active-project", true),
            switcher_row("other-project", false),
        ],
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
    let label_x = |row: usize| {
        Layout::new(&node)
            .children()
            .nth(row)
            .expect("the switcher's column must hold both rows")
            .children()
            .next()
            .expect("a row holds its content")
            .children()
            .last()
            .expect("a row's content ends with its label")
            .bounds()
            .x
    };

    let (marked, unmarked) = (label_x(0), label_x(1));
    assert!(
        (marked - unmarked).abs() < TOLERANCE,
        "the active row's label starts at {marked}dp and an unmarked row's at {unmarked}dp, so the \
         marker shifts every other label sideways instead of occupying a slot the rows all keep",
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

/// §7.5: the item's content is **centred in its 48dp**, not resting against the top of it.
///
/// The half of that table row FR-030a exists for, and the half BUG-003 asserted around without ever
/// asserting: an item 48dp tall around a 20dp label has 28dp of slack, and nothing in the previous
/// fix decided where it went. Both figures beside it were right — the height, and the 12dp ends —
/// which is what made this survive a change that measured the box from every other direction.
///
/// The reasoning that let it through is worth keeping, because it reads as correct: the row states
/// `align_y(Center)`, and `button` does stretch its content node to the fixed height. But a `Row`
/// centres its children **against each other**, inside the cross size the flex computed from them —
/// 31.2dp, the icon's line box — and that band is then laid against the top of the stretched node.
/// The label came out 8.4dp high of centre. It is BUG-001's app bar exactly, one component over, and
/// the previous fix quoted that bug while missing that it applied to this row too.
#[test]
fn a_menu_items_content_is_centred_in_its_height() {
    let item = bounds_at(panel(two_items()), &[0, 0]);
    let icon = bounds_at(panel(two_items()), &[0, 0, 0, 0]);
    let label = bounds_at(panel(two_items()), &[0, 0, 0, 1]);

    let centre = |b: Rectangle| b.y + b.height / 2.0;
    let (item_centre, icon_centre, label_centre) = (centre(item), centre(icon), centre(label));

    assert!(
        (label_centre - item_centre).abs() < TOLERANCE
            && (icon_centre - item_centre).abs() < TOLERANCE,
        "a menu item's box is centred on {item_centre}dp, its label on {label_centre}dp and its \
         glyph on {icon_centre}dp. §7.5 states 48dp \"with the item's content centred in it\", and \
         a height above its content obliges the anatomy to say where the content sits (FR-030a) — \
         an unstated alignment is not \"centred by default\", it is \"against the top edge\"",
    );
}

/// The type-ahead's result row is the same shape and had the same defect (BUG-004).
///
/// It is not the shared item row — FR-029b excludes it, and its label is an emphasised span list
/// rather than a string — but "not the same component" is not "not the same mistake". Both put a
/// `Row` inside a `button` with a fixed height, and both needed the row to fill that height before
/// `align_y` meant anything. Asserted here rather than left as the fixed half nobody measures.
#[test]
fn a_typeahead_rows_content_is_centred_in_its_height() {
    let row = super::picker::row_element(
        super::TypeaheadRow {
            label: "feat/short".to_string(),
            spans: Vec::new(),
            enabled: true,
        },
        false,
        false,
        Some(Message::NoOp),
        roles(),
    );

    let outer = bounds_at(row, &[]);
    let content = bounds_at(
        super::picker::row_element(
            super::TypeaheadRow {
                label: "feat/short".to_string(),
                spans: Vec::new(),
                enabled: true,
            },
            false,
            false,
            Some(Message::NoOp),
            roles(),
        ),
        &[0, 1],
    );

    let centre = |b: Rectangle| b.y + b.height / 2.0;
    assert!(
        (centre(content) - centre(outer)).abs() < TOLERANCE,
        "a type-ahead row's box is centred on {}dp and its label on {}dp — the same 48dp row with          its content against the top edge that BUG-004 was in the menu",
        centre(outer),
        centre(content),
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
    let rows = menu::item_column(vec![switcher_row("micold-ai-ide", true)], roles());

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
