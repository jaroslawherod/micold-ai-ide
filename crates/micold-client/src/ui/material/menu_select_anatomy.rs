//! `MenuSelect`'s trigger is the one `Select` draws (feature 1792, T002–T004 — FR-001, FR-002,
//! FR-009, FR-011).
//!
//! The composer's model, effort and permission selectors, the review's source control and the
//! worktree filters are all a [`MenuSelect`]. Its trigger used to be a text button of its own; it is
//! now [`SelectTrigger`](super::SelectTrigger), so these tests compare it to a [`Select`] in each
//! state rather than restating any figure. In-crate because `material` is `pub(crate)`.

use iced::{mouse, Color, Element, Length, Point, Size};

use super::test_support::{roles, Mounted};
use super::{style, MenuSelect, MenuSelectRow, Select};

const ROOM: Size = Size::new(300.0, 200.0);

/// A selector reading `Model:` over `title`, `width` wide, in the given state.
fn selector(title: &str, width: Length, open: bool, disabled: bool) -> Element<'static, String> {
    selector_over(
        title,
        vec![MenuSelectRow::note("nothing")],
        width,
        open,
        disabled,
    )
}

fn selector_over(
    title: &str,
    rows: Vec<MenuSelectRow<'static, String>>,
    width: Length,
    open: bool,
    disabled: bool,
) -> Element<'static, String> {
    let menu = MenuSelect::new(
        "Model:",
        rows,
        open,
        "toggle".to_string(),
        "dismiss".to_string(),
        roles(),
    )
    .title(Some(title.to_string()))
    .width(width);
    if disabled {
        menu.disabled("Why it is off").into()
    } else {
        menu.into()
    }
}

fn plain(open: bool, disabled: bool) -> Mounted<'static> {
    Mounted::new_in(selector("opus", Length::Fill, open, disabled), ROOM)
}

fn select() -> Mounted<'static> {
    let options: &'static [&'static str] = &["opus"];
    Mounted::new_in(
        Select::new(options, Some(options[0]), |t: &str| t.to_string(), roles()),
        ROOM,
    )
}

/// A click: press and release, which is when a button sends its message.
fn click(field: &mut Mounted<'_>) -> Vec<String> {
    let at = centre(field);
    let mut sent = field.press(at);
    sent.extend(field.send(
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(at),
    ));
    sent
}

/// One drawn pixel, over a backdrop no role is close to.
fn pixels(field: &mut Mounted<'_>, cursor: mouse::Cursor) -> (Vec<u8>, u32) {
    field.drawn_under(Color::from_rgb(1.0, 0.0, 1.0), cursor)
}

fn at(buf: &(Vec<u8>, u32), x: u32, y: u32) -> [u8; 3] {
    let i = ((y * buf.1 + x) * 4) as usize;
    [buf.0[i], buf.0[i + 1], buf.0[i + 2]]
}

fn centre(field: &Mounted<'_>) -> Point {
    field.container().center()
}

/// FR-001, FR-009: the container is the select's container, not a button's box.
#[test]
fn the_trigger_has_the_container_geometry_and_edge_of_a_select() {
    let (menu, select) = (plain(false, false), select());
    assert_eq!(
        (menu.container().width, menu.container().height),
        (select.container().width, select.container().height),
    );
}

#[test]
fn the_resting_trigger_paints_the_selects_container_and_indicator() {
    let (mut menu, mut select) = (plain(false, false), select());
    let (m, s) = (
        pixels(&mut menu, mouse::Cursor::Unavailable),
        pixels(&mut select, mouse::Cursor::Unavailable),
    );
    let h = menu.container().height as u32;
    // Left edge column: the container's fill and, at the bottom, the active indicator.
    for y in [1, h / 2, h - 2, h - 1] {
        assert_eq!(at(&m, 2, y), at(&s, 2, y), "row {y} differs from Select");
    }
}

#[test]
fn an_open_trigger_thickens_its_indicator_as_a_select_does() {
    let rest = pixels(&mut plain(false, false), mouse::Cursor::Unavailable);
    let open = pixels(&mut plain(true, false), mouse::Cursor::Unavailable);
    let h = plain(false, false).container().height as u32;
    assert_ne!(at(&rest, 2, h - 2), at(&open, 2, h - 2), "no thicker edge");
    assert_eq!(
        at(&open, 2, h - 2),
        {
            let c = style::color(roles().primary);
            [
                (c.r * 255.0).round() as u8,
                (c.g * 255.0).round() as u8,
                (c.b * 255.0).round() as u8,
            ]
        },
        "the open edge is `primary`"
    );
}

#[test]
fn hovering_the_trigger_shades_its_container() {
    let mut f = plain(false, false);
    let rest = pixels(&mut f, mouse::Cursor::Unavailable);
    let at_centre = centre(&f);
    let over = pixels(&mut f, mouse::Cursor::Available(at_centre));
    let y = f.container().height as u32 / 2;
    assert_ne!(at(&rest, 2, y), at(&over, 2, y), "hover paints no layer");
}

/// FR-002, US1 scenario 3.
#[test]
fn a_disabled_trigger_looks_disabled_and_ignores_the_pointer() {
    let mut off = plain(false, false);
    let mut dis = plain(false, true);
    let y = dis.container().height as u32 / 2;
    let rest = pixels(&mut dis, mouse::Cursor::Unavailable);
    let at_centre = centre(&dis);
    let over = pixels(&mut dis, mouse::Cursor::Available(at_centre));
    assert_eq!(
        at(&rest, 2, y),
        at(&over, 2, y),
        "a disabled trigger shades on hover"
    );
    let h = dis.container().height as u32;
    let live = pixels(&mut off, mouse::Cursor::Unavailable);
    assert_ne!(at(&rest, 2, y), at(&live, 2, y), "no disabled container");
    assert_ne!(at(&rest, 2, h - 1), at(&live, 2, h - 1), "no disabled edge");
    assert!(
        click(&mut dis).is_empty(),
        "a disabled trigger published a press"
    );
    assert_eq!(click(&mut off), vec!["toggle"]);
}

/// FR-011: a long value gives way to the chevron, it does not push it out.
#[test]
fn a_long_value_is_cut_and_the_chevron_stays_visible() {
    let width = Length::Fixed(200.0);
    let long = "an exceedingly long model name that cannot possibly fit in two hundred dp";
    let mut short = Mounted::new_in(selector("a", width, false, false), ROOM);
    let mut cut = Mounted::new_in(selector(long, width, false, false), ROOM);
    assert_eq!(short.container().height, cut.container().height);
    assert_eq!(cut.container().width, 200.0);
    let (a, b) = (
        pixels(&mut short, mouse::Cursor::Unavailable),
        pixels(&mut cut, mouse::Cursor::Unavailable),
    );
    let h = cut.container().height as u32;
    let trailing: Vec<_> = (160..198)
        .flat_map(|x| (0..h).map(move |y| (x, y)))
        .collect();
    assert!(
        trailing.iter().all(|&(x, y)| at(&a, x, y) == at(&b, x, y)),
        "the chevron's column changed with the value's length"
    );
}

/// FR-011: a value absent from the options is still its own text.
#[test]
fn a_value_missing_from_the_options_renders_its_text() {
    let mut f = Mounted::new_in(
        selector_over(
            "ghost-model",
            vec![MenuSelectRow::note("other")],
            Length::Fill,
            false,
            false,
        ),
        ROOM,
    );
    let said: String = f.painted_text().into_iter().map(|p| p.content).collect();
    assert!(said.contains("ghost-model"), "painted: {said:?}");
}

/// FR-011: nothing to choose, nothing to open.
#[test]
fn an_empty_option_list_opens_no_menu() {
    let mut empty = Mounted::new_in(selector_over("x", vec![], Length::Fill, true, false), ROOM);
    assert!(!empty.floats(), "an empty list floated a menu");
    let mut some = plain(true, false);
    assert!(some.floats(), "a list with rows should float its menu");
}
