//! The status bar keeps every text inside its box at the smallest window (feature 1124, T009 —
//! FR-001, FR-012, SC-002, SC-005).
//!
//! The worst case the bar meets: a remote daemon with a long name and an image download in flight,
//! at `MIN_WINDOW_SIZE`. The bar must stay 28 dp high and flush with the window's bottom edge, and
//! each of its texts must be drawn no wider than the room it was given and inside the window — the
//! give-way pass shortens them; nothing may paint past the box.
//!
//! **Compiled into the `layout_snapshot` binary** for the `support::layout` passes; it builds its
//! own state and reads no cache.

use crate::support::covered_states::status_bar_long_texts;
use crate::support::layout as lay;
use micold_client::app::MIN_WINDOW_SIZE;
use micold_core::tokens::anatomy::status_bar::HEIGHT;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

#[test]
fn at_the_smallest_window_the_bars_texts_stay_in_their_boxes() {
    let window = MIN_WINDOW_SIZE;
    let under = status_bar_long_texts();
    let (painted, records) =
        lay::painted_text_settled_at(lay::view_of(&under), &mut lay::renderer(), window);

    // The bar is the window column's last child.
    let bar = records
        .iter()
        .filter(|r| r.layer == lay::Layer::Base && r.path.len() == 3 && r.path[..2] == [0, 0])
        .max_by_key(|r| r.path[2])
        .expect("the window column has children");
    assert_eq!(bar.height, HEIGHT, "the bar is {HEIGHT} dp high: {bar:?}");
    assert!(
        (bar.y + bar.height - window.height).abs() <= TOLERANCE,
        "the bar is flush with the window's bottom edge: {bar:?} in {window:?}"
    );
    assert_eq!(bar.width, window.width, "the bar spans the window: {bar:?}");

    let in_bar: Vec<_> = painted
        .iter()
        .filter(|p| p.origin.y >= bar.y - TOLERANCE)
        .collect();
    for wanted in ["D", "G", "CLI:"] {
        assert!(
            in_bar.iter().any(|p| p.content.starts_with(wanted)),
            "the bar draws its connection and sandbox items: {:?}",
            in_bar.iter().map(|p| &p.content).collect::<Vec<_>>()
        );
    }
    assert!(
        !in_bar
            .iter()
            .any(|p| p.content.contains("Overview") || p.content.contains("Jump")),
        "the key hints give way first: {:?}",
        in_bar.iter().map(|p| &p.content).collect::<Vec<_>>()
    );
    for p in &in_bar {
        assert!(
            p.natural_width <= p.allowed_width + TOLERANCE,
            "{:?} wants {} of the {} it was given",
            p.content,
            p.natural_width,
            p.allowed_width
        );
        assert!(
            p.origin.x >= -TOLERANCE && p.origin.x + p.natural_width <= window.width + TOLERANCE,
            "{:?} at x {} ({} wide) lies inside the {} dp window",
            p.content,
            p.origin.x,
            p.natural_width,
            window.width
        );
    }
}

/// With room to spare the bar carries the CLI item and both hints (T028, FR-007, FR-008).
#[test]
fn with_room_the_bar_carries_the_cli_and_both_hints() {
    let window = iced::Size::new(1400.0, MIN_WINDOW_SIZE.height);
    let under = crate::support::covered_states::status_bar_long_texts();
    let (painted, _) =
        lay::painted_text_settled_at(lay::view_of(&under), &mut lay::renderer(), window);
    let contents: Vec<_> = painted.iter().map(|p| p.content.as_str()).collect();
    for wanted in ["CLI: ", " Overview", " Jump"] {
        assert!(
            contents.iter().any(|c| c.contains(wanted)),
            "{wanted:?} is drawn at {window:?}: {contents:?}"
        );
    }
}

/// With room to spare the usage sits after the CLI item (T034, FR-011).
#[test]
fn with_room_the_usage_follows_the_cli_item() {
    let window = iced::Size::new(1400.0, MIN_WINDOW_SIZE.height);
    let under = crate::support::covered_states::status_bar_usage();
    let (painted, _) =
        lay::painted_text_settled_at(lay::view_of(&under), &mut lay::renderer(), window);
    let x_of = |wanted: &str| {
        painted
            .iter()
            .find(|p| p.content.contains(wanted))
            .unwrap_or_else(|| panic!("{wanted:?} is drawn: {painted:?}"))
            .origin
            .x
    };
    assert!(
        x_of("37%") > x_of("CLI: "),
        "usage comes after the CLI item"
    );
}

/// At the smallest window usage goes after the hints and before any text is shortened (T034,
/// FR-012): with the longest texts the hints and the usage are gone, and the usage never outlives
/// a shortened text.
#[test]
fn at_the_smallest_window_usage_drops_before_any_text_shortens() {
    let window = MIN_WINDOW_SIZE;
    let under = crate::support::covered_states::status_bar_long_texts_with_usage();
    let (painted, _) =
        lay::painted_text_settled_at(lay::view_of(&under), &mut lay::renderer(), window);
    let bar_y = window.height - HEIGHT - TOLERANCE;
    let in_bar: Vec<_> = painted.iter().filter(|p| p.origin.y >= bar_y).collect();
    let texts = || in_bar.iter().map(|p| &p.content).collect::<Vec<_>>();
    assert!(
        in_bar.iter().any(|p| p.content.starts_with("Daemon")),
        "premise: the bar is drawn, its connection item first: {:?}",
        texts()
    );
    assert!(
        !in_bar.iter().any(|p| p.content.contains("Overview")),
        "{:?}",
        texts()
    );
    assert!(
        !in_bar.iter().any(|p| p.content.contains("37%")),
        "the long texts leave no room for usage, so it goes before any text shortens: {:?}",
        texts()
    );
}
