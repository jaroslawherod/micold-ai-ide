//! A component lays out at the size its anatomy entry states, whatever room it is offered
//! (feature 018 — FR-027, FR-030, BUG-002).
//!
//! # The gap this closes
//!
//! Four checks already read something about a component's box, and none of them reads its **size**:
//!
//! - `micold-core/tests/tokens_anatomy.rs` compares the *constants* against the contract. It was
//!   green throughout BUG-002: `button::MIN_TOUCH_TARGET` is 48.0 and always was — the defect was
//!   that nothing used it.
//! - `tests/layout_snapshot.rs` byte-compares a fixture. It **recorded** BUG-002 as the expected
//!   value (a 499.9 × 64.0 icon button in the app bar) and was green on it from the day it landed.
//!   A snapshot catches change, not defect; a defect older than the fixture becomes the baseline.
//! - `tests/gates/containment.rs` asks whether a child escapes its parent. A component that grows
//!   to fill its parent is contained by definition.
//! - `content_placement.rs` asks where a component put its *content* inside its box. BUG-002's
//!   glyph was perfectly centred — in the middle of a box twenty-five times too wide.
//!
//! So this asks the remaining question: is the box the size the contract says?
//!
//! # How, and why twice
//!
//! Each component is laid out under two differently-sized limits. That is what separates the
//! outcomes an axis can have, which no single measurement can:
//!
//! - **`Fixed`** — the same number under both, equal to the anatomy constant. §7.1's 64dp bar,
//!   §7.3's 48dp target and 40dp button, §7.5's 48dp menu item, §7.6's 32dp chip, §7.7's 56dp field.
//! - **`Fill`** — tracks the limit. Intended for the app bar's width and a menu item's; a defect
//!   anywhere a contract states a number.
//! - **`Content`** — the same number under both, and smaller than the room offered. A chip's width
//!   and a compact icon button. §7.2's tree row was declared this too, and that was BUG-005: the
//!   row's height applied at depth ≥ 1 and was dropped at depth 0, so the one depth-0 specimen this
//!   entry held reported the component as content-sized and the height was deleted on the strength
//!   of it. An entry that measures one instance of a component is not measuring the component.
//! - **`AtLeast` / `AtMost`** — §7.8's snackbar, the one entry the contract bounds rather than
//!   fixes.
//!
//! One measurement cannot tell `Fixed(48)` from `Fill` (offer exactly 48 and they agree) nor `Fill`
//! from `Content` (offer a tight box and they agree). Two, at different sizes, tell them apart.
//!
//! Some figures belong to a component's *row* rather than to the component — §7.2's row, §7.5's
//! item — so an entry may name a path into the laid-out tree. Reading the root there would measure
//! the list.
//!
//! # In-crate, like its neighbours
//!
//! `material` is `pub(crate)`, so an `IconButton` cannot be constructed from `tests/` at all — the
//! same reason `content_placement`, `form_field_anatomy` and `text_field_anatomy` live here.

use iced::advanced::layout;
use iced::advanced::widget::Tree;
use iced::{Element, Length, Size};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, anatomy, density, Roles};

use super::{
    menu, Button, ButtonVariant, IconButton, MenuItem, Snackbar, Text, TextField, ToggleChip,
    Toolbar, TreeItem, TreeView, TypeRole,
};
use crate::icons::Icon;
use crate::showcase::state::Message;

/// The two boxes every component is offered. Both are far larger than any component here, and
/// differ on both axes, so a `Fill` axis reports two different numbers and a sized one reports the
/// same number twice.
const ROOMY: Size = Size::new(1200.0, 900.0);
const SNUG: Size = Size::new(400.0, 300.0);

/// Layout arithmetic accumulates over a nested tree; this is well below anything a person could
/// see and far below the defect being separated from correct (which is hundreds of pixels).
const TOLERANCE: f32 = 0.5;

fn roles() -> Roles {
    tokens::roles(ColorScheme::Light)
}

/// `Snackbar` borrows its notification, so the specimens need ones that outlive the `Element`.
///
/// Two, because §7.8 states a minimum height and a maximum width and one message cannot exercise
/// both: a short message sits at the height floor and nowhere near the width cap, and a long one
/// pushes the cap while clearing the floor by so much that the floor becomes vacuous.
static SHORT: std::sync::OnceLock<micold_core::notify::Notification> = std::sync::OnceLock::new();
static LONG: std::sync::OnceLock<micold_core::notify::Notification> = std::sync::OnceLock::new();

fn short_notice() -> &'static micold_core::notify::Notification {
    SHORT.get_or_init(|| {
        micold_core::notify::Notification::new(micold_core::notify::Level::Info, "Created")
    })
}

fn long_notice() -> &'static micold_core::notify::Notification {
    LONG.get_or_init(|| {
        micold_core::notify::Notification::new(
            micold_core::notify::Level::Info,
            "Created the worktree, checked out the branch, copied the environment include script \
             into place, and started a session in it — which is deliberately far more text than \
             600dp of one line can hold.",
        )
    })
}

/// What an axis of a component is allowed to do.
#[derive(Clone, Copy)]
enum Extent {
    /// Exactly this many dp, whatever room is offered. The anatomy constant, never a literal.
    Fixed(f32),
    /// As much room as it is given — correct only where the contract asks for it.
    Fill,
    /// Sized by what it holds: the same under both limits, and smaller than either.
    Content,
    /// At least this many dp, and content-sized above it — §7.8's snackbar minimum height.
    AtLeast(f32),
    /// Content-sized, but never past this — §7.8's snackbar maximum width. Asserted under the
    /// roomy limit only, since the snug one is below the cap and would pass vacuously.
    AtMost(f32),
}

/// The size the node at `path` resolves to when `element` is laid out inside `bounds`.
///
/// `path` walks the laid-out tree by child index, empty meaning the root. Some anatomy figures
/// belong to a component's *row* rather than to the component — §7.2's row height inside a tree,
/// §7.5's item height inside a menu — and reading the root would measure the list instead of the
/// thing the contract names.
fn laid_out(element: Element<'_, Message>, bounds: Size, path: &[usize]) -> Size {
    let mut element = element;
    let renderer = super::test_support::renderer();
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);

    let mut current = &node;
    for (depth, &index) in path.iter().enumerate() {
        current = current.children().get(index).unwrap_or_else(|| {
            panic!(
                "no child {index} at depth {depth} of {path:?} — the component's tree changed \
                 shape, so this entry is measuring something other than what it names"
            )
        });
    }
    current.bounds().size()
}

/// Why `measured` is wrong for an axis declared `expected`, or `None` if it is right.
///
/// Returned rather than asserted so [`the_gate_can_fail`] can watch it fire against a deliberately
/// broken control. A check that cannot be shown to fail is decoration.
fn violation(
    component: &str,
    axis: &str,
    expected: Extent,
    measured: (f32, f32),
    offered: (f32, f32),
) -> Option<String> {
    let (roomy, snug) = measured;
    let (roomy_limit, snug_limit) = offered;
    let seen = format!(
        "{component}'s {axis} measured {roomy}dp in a {roomy_limit}dp box and {snug}dp in a \
         {snug_limit}dp one"
    );
    match expected {
        Extent::Fixed(dp) => ((roomy - dp).abs() > TOLERANCE || (snug - dp).abs() > TOLERANCE)
            .then(|| {
                let filling = (roomy - roomy_limit).abs() < TOLERANCE
                    && (snug - snug_limit).abs() < TOLERANCE;
                let diagnosis = if filling {
                    " — it is taking whatever room it is offered, so the stated size is not being \
                     applied at all. That is BUG-002's shape: `Container::center_x`/`center_y` set \
                     the length as well as aligning, so a `.width(Fixed(n)).center_x(Fill)` chain \
                     silently discards `n`."
                } else {
                    "."
                };
                format!("{seen}, but its anatomy entry states {dp}dp{diagnosis}")
            }),
        Extent::Fill => ((roomy - roomy_limit).abs() > TOLERANCE
            || (snug - snug_limit).abs() > TOLERANCE)
            .then(|| {
                format!(
                    "{seen}, but it is supposed to span the room it is given ({roomy_limit}dp and \
                     {snug_limit}dp respectively)."
                )
            }),
        Extent::Content => ((roomy - snug).abs() > TOLERANCE || roomy >= snug_limit).then(|| {
            format!(
                "{seen}, but it is supposed to be sized by what it holds — the same under both, \
                 and smaller than either box."
            )
        }),
        Extent::AtLeast(dp) => (roomy < dp - TOLERANCE || snug < dp - TOLERANCE)
            .then(|| format!("{seen}, but its anatomy entry states a minimum of {dp}dp.")),
        Extent::AtMost(dp) => (roomy > dp + TOLERANCE).then(|| {
            format!(
                "{seen}, but its anatomy entry caps it at {dp}dp. (Judged under the roomy \
                     limit only: the snug one is already below the cap, so it cannot show a \
                     missing one.)"
            )
        }),
    }
}

/// Lay `build` out under both limits and assert each axis does what it is declared to do.
fn assert_anatomy_size(
    component: &str,
    build: impl Fn() -> Element<'static, Message>,
    width: Extent,
    height: Extent,
) {
    assert_anatomy_size_at(component, build, &[], width, height)
}

/// As [`assert_anatomy_size`], measuring the node at `path` rather than the root — for a figure
/// the contract gives to a component's row rather than to the component (§7.2, §7.5).
fn assert_anatomy_size_at(
    component: &str,
    build: impl Fn() -> Element<'static, Message>,
    path: &[usize],
    width: Extent,
    height: Extent,
) {
    let roomy = laid_out(build(), ROOMY, path);
    let snug = laid_out(build(), SNUG, path);

    let complaints: Vec<String> = [
        violation(
            component,
            "width",
            width,
            (roomy.width, snug.width),
            (ROOMY.width, SNUG.width),
        ),
        violation(
            component,
            "height",
            height,
            (roomy.height, snug.height),
            (ROOMY.height, SNUG.height),
        ),
    ]
    .into_iter()
    .flatten()
    .collect();

    assert!(complaints.is_empty(), "{}", complaints.join("\n"));
}

/// §7.3: an icon button presents a 48 × 48 interactive target — a *fixed* one, not "the rest of the
/// row". This is BUG-002 itself, and the reason every other component here is checked too.
#[test]
fn an_icon_buttons_touch_target_is_the_48dp_the_contract_states() {
    assert_anatomy_size(
        "an icon button",
        || {
            IconButton::new(Icon::Menu, roles())
                .on_press(Message::NoOp)
                .into()
        },
        Extent::Fixed(anatomy::button::MIN_TOUCH_TARGET),
        Extent::Fixed(anatomy::button::MIN_TOUCH_TARGET),
    );
}

/// A disabled icon button takes no target container at all (there is nothing to hit), so it is
/// content-sized — but it must still not grow into the room it is offered.
#[test]
fn a_disabled_icon_button_is_sized_by_its_glyph() {
    assert_anatomy_size(
        "a disabled icon button",
        || IconButton::<Message>::new(Icon::Menu, roles()).into(),
        Extent::Content,
        Extent::Content,
    );
}

/// FR-045's recorded deviation: inside the sidebar the target stays at the glyph's own size. That
/// is a *smaller* box, not an elastic one — this is what would notice `compact()` acquiring the
/// same defect the full-size path had.
#[test]
fn a_compact_icon_button_is_sized_by_its_glyph_not_by_the_room_it_is_given() {
    assert_anatomy_size(
        "a compact icon button",
        || {
            IconButton::new(Icon::Menu, roles())
                .compact()
                .on_press(Message::NoOp)
                .into()
        },
        Extent::Content,
        Extent::Content,
    );
}

/// §7.6: a chip is 32dp tall and as wide as its label plus its 12dp ends.
#[test]
fn a_chip_is_32dp_tall_and_sized_by_its_label() {
    assert_anatomy_size(
        "a chip",
        || ToggleChip::new("feat", Message::NoOp, roles()).into(),
        Extent::Content,
        Extent::Fixed(anatomy::chip::HEIGHT),
    );
}

/// §7.1: the small app bar is a fixed 64dp that spans its window. The one component here with a
/// deliberately elastic axis — which is the point of declaring the axes separately rather than
/// asserting "nothing fills".
///
/// Raised, because at rest the `Toolbar` element is the bar *and* its 1dp separator, and the 64dp
/// under test is the bar's.
#[test]
fn an_app_bar_is_64dp_tall_and_spans_its_window() {
    assert_anatomy_size(
        "the app bar",
        || {
            Toolbar::<Message>::new("Micold AI IDE", roles())
                .elevated(true)
                .into()
        },
        Extent::Fill,
        Extent::Fixed(anatomy::app_bar::HEIGHT),
    );
}

/// §7.3: the filled, outlined and text variants are 40dp tall. All three, because the height is the
/// variant table's first row and nothing about it varies between them — a variant that acquired its
/// own height would be §7.2's density scale reinvented one component down.
#[test]
fn a_button_is_40dp_tall_whichever_variant_it_is() {
    for (name, variant) in [
        ("a filled button", ButtonVariant::Filled),
        ("an outlined button", ButtonVariant::Outlined),
        ("a text button", ButtonVariant::Text),
    ] {
        assert_anatomy_size(
            name,
            move || {
                Button::with_content(
                    Text::new("Open", TypeRole::Action, roles()),
                    variant,
                    roles(),
                )
                .on_press(Message::NoOp)
                .into()
            },
            Extent::Content,
            Extent::Fixed(density::BUTTON_BASE),
        );
    }
}

/// §7.5: a menu item is 48dp tall and spans the panel it sits in.
///
/// Measured on the item, not the column: the column is as tall as its items and would pass on any
/// item height at all. `item_column` is the shared stack behind both `MenuOverlay` and
/// `ContextMenu`, so one entry covers both.
#[test]
fn a_menu_item_is_48dp_tall_and_spans_its_panel() {
    assert_anatomy_size_at(
        "a menu item",
        || {
            menu::item_column(
                vec![MenuItem::new(Icon::About, "About", Message::NoOp)],
                roles(),
            )
        },
        &[0],
        Extent::Fill,
        Extent::Fixed(density::MENU_ITEM_BASE),
    );
}

/// §7.7: the filled text field is 56dp tall and spans the form column it sits in.
#[test]
fn a_text_field_is_56dp_tall_and_spans_its_column() {
    assert_anatomy_size(
        "a text field",
        || TextField::new("Branch", "", roles()).into(),
        Extent::Fill,
        Extent::Fixed(density::TEXT_FIELD_BASE),
    );
}

/// §7.8: a snackbar is at least 48dp tall and never wider than 600dp.
///
/// A minimum and a maximum rather than a size, and two specimens because one message cannot
/// exercise both — see [`short_notice`]. Measured on the panel (child 0), which is the box §7.8
/// describes; the root adds the margin that floats it off the window edge.
#[test]
fn a_snackbar_clears_48dp_and_stays_within_600dp() {
    assert_anatomy_size_at(
        "a snackbar with a short message",
        || Snackbar::new(short_notice(), roles()).into(),
        &[0],
        Extent::AtMost(anatomy::snackbar::MAX_WIDTH),
        Extent::AtLeast(anatomy::snackbar::MIN_HEIGHT),
    );
    assert_anatomy_size_at(
        "a snackbar with a message far past the cap",
        || Snackbar::new(long_notice(), roles()).into(),
        &[0],
        Extent::AtMost(anatomy::snackbar::MAX_WIDTH),
        Extent::AtLeast(anatomy::snackbar::MIN_HEIGHT),
    );
}

/// A one-line tree row stands at §7.2's height for its density — **at every depth**, and at both
/// densities (FR-026, FR-026d, SC-008d).
///
/// Four specimens rather than one, and the count is the point. This entry used to hold a single
/// depth-0 row and declare it `Content`, recording "§7.2's height is deliberately not applied" as a
/// decision. It was not a decision, it was BUG-005: the height rode on each row's *indent spacer*,
/// whose width is `depth × spacing::MD`, so at depth 0 that spacer was `Fixed(0)` wide, iced dropped
/// it as void, and the floor applied to nested rows only. One specimen, taken at the one depth where
/// the height had never worked, reported the component as uniformly content-sized — and the height
/// was then deleted outright on the strength of it, which cost every session row in the sidebar 34%
/// of its height with no gate anywhere able to notice.
///
/// So: both densities, and depth 0 beside depth 1. A component that nests varies on depth, and an
/// entry that measures one depth is measuring one instance of a component, not the component.
#[test]
fn a_one_line_tree_row_stands_at_its_densitys_height_at_every_depth() {
    for step in [density::STANDARD, density::DENSE] {
        let expected = density::height(density::LIST_ROW_BASE, step);
        for depth in [0u16, 1, 2] {
            assert_anatomy_size_at(
                &format!("a one-line tree row at depth {depth}, density {step}"),
                move || {
                    TreeView::new(
                        vec![TreeItem::new(depth, "feat-short", roles().on_surface)],
                        roles(),
                    )
                    .density(step)
                    .into()
                },
                &[0],
                Extent::Fill,
                Extent::Fixed(expected),
            );
        }
    }
}

/// A tagged row takes §7.2's **two-line** height, because it is a two-line list item (FR-026d).
#[test]
fn a_tagged_tree_row_stands_at_the_two_line_height() {
    for step in [density::STANDARD, density::DENSE] {
        let expected = density::height(density::LIST_ROW_TWO_LINE_BASE, step);
        assert_anatomy_size_at(
            &format!("a tagged tree row at density {step}"),
            move || {
                TreeView::new(
                    vec![TreeItem::new(0, "feat-short", roles().on_surface)
                        .tags(vec![("feat".to_string(), roles().primary)])],
                    roles(),
                )
                .density(step)
                .into()
            },
            &[0],
            Extent::Fill,
            Extent::Fixed(expected),
        );
    }
}

/// The height is a **minimum**, not a cap: a row holding more than its figure grows (FR-026d).
///
/// The distinction is the whole of BUG-005. Read as a fixed height, §7.2's figure clips a tagged
/// row's chips and conflicts with FR-026a; read as a minimum it does neither. `AtLeast` is what
/// says so — a `Fixed` here would pass today and start clipping the moment a row grew a third line.
#[test]
fn a_tree_row_taller_than_its_density_grows_rather_than_clipping() {
    let tall = |step: i8| {
        move || -> Element<'static, Message> {
            TreeView::new(
                vec![
                    TreeItem::new(0, "feat-short", roles().on_surface).tags(vec![
                        ("feat".to_string(), roles().primary),
                        ("issue".to_string(), roles().secondary),
                    ]),
                ],
                roles(),
            )
            .density(step)
            .into()
        }
    };
    for step in [density::STANDARD, density::DENSE] {
        assert_anatomy_size_at(
            &format!("a tree row taller than density {step}"),
            tall(step),
            &[0],
            Extent::Fill,
            Extent::AtLeast(density::height(density::LIST_ROW_TWO_LINE_BASE, step)),
        );
    }
}

/// The gate can fail, shown against the exact chain BUG-002 was: a container that states 48dp and
/// then hands both axes to `center_x`/`center_y`, which set the length as well as aligning.
///
/// Built here rather than by reverting the component, so it holds after the fix and would still
/// hold if `IconButton` were rewritten around a different primitive.
#[test]
fn the_gate_can_fail() {
    let sabotage = || -> Element<'static, Message> {
        iced::widget::container(iced::widget::Space::new())
            .width(Length::Fixed(anatomy::button::MIN_TOUCH_TARGET))
            .height(Length::Fixed(anatomy::button::MIN_TOUCH_TARGET))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    };

    let roomy = laid_out(sabotage(), ROOMY, &[]);
    let snug = laid_out(sabotage(), SNUG, &[]);
    let complaint = violation(
        "the sabotage",
        "width",
        Extent::Fixed(anatomy::button::MIN_TOUCH_TARGET),
        (roomy.width, snug.width),
        (ROOMY.width, SNUG.width),
    )
    .expect(
        "a container whose stated 48dp is overwritten by `center_x(Fill)` was reported as \
         correctly sized — this gate cannot see the defect it exists for",
    );

    assert!(
        complaint.contains("taking whatever room it is offered"),
        "the failure names the size but not the cause, so it does not tell the next reader what to \
         look for: {complaint}"
    );
}
