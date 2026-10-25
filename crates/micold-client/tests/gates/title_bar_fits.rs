//! The title bar holds every control inside its 43 dp at the smallest and an ordinary window
//! (feature 1127, T013 — FR-001, FR-010, SC-001, SC-003, contract B5).
//!
//! The fullest bar: a 60-character project name, an unread mark, and the Overview button with its
//! badge. At 640 and 1280 dp wide, light and dark, the bar and its divider end at
//! `app_bar::BOTTOM_EDGE`, every child of the bar lies inside its height and the window, no two
//! children overlap, and the mark, the project switcher, the Overview button with its badge, the
//! jump-to field (at least its glyph), Settings and the overflow trigger are all drawn. The field
//! sits with equal free space either side of it (T044, FR-008, FR-010, contract B3).
//!
//! **Compiled into the `layout_snapshot` binary** for the `support::layout` passes; it builds its
//! own state and reads no cache.

use crate::support::covered_states::{
    title_bar_crowded, SIXTY_CHARACTER_BRANCH, SIXTY_CHARACTER_PROJECT,
};
use crate::support::layout::{self as lay, StateUnderTest};
use iced::Size;
use micold_client::icons::Icon;
use micold_core::theme::ThemePreference;
use micold_core::tokens::anatomy::app_bar;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The bar's column (bar, then divider) and the bar's own children: shell → column → bar → lane.
const BAR_COLUMN: &[usize] = &[0, 0, 0];
const LANE: &[usize] = &[0, 0, 0, 0, 0];

#[test]
fn the_fullest_title_bar_fits_at_640_and_1280_in_both_schemes() {
    for window in [Size::new(640.0, 480.0), Size::new(1280.0, 800.0)] {
        for theme in [ThemePreference::Light, ThemePreference::Dark] {
            let mut state = title_bar_crowded();
            state.settings.theme_pref = theme;
            let under = StateUnderTest::new(state);
            let (painted, records) =
                lay::painted_text_settled_at(lay::view_of(&under), &mut lay::renderer(), window);
            let at = format!("{window:?} {theme:?}");

            let column = records
                .iter()
                .find(|r| r.layer == lay::Layer::Base && r.path == BAR_COLUMN)
                .expect("the bar's column is laid out");
            assert!(
                column.y.abs() <= TOLERANCE
                    && (column.height - app_bar::BOTTOM_EDGE).abs() <= TOLERANCE,
                "{at}: the bar and its divider are {column:?}, not 0..{}",
                app_bar::BOTTOM_EDGE
            );

            let children: Vec<_> = records
                .iter()
                .filter(|r| {
                    r.layer == lay::Layer::Base
                        && r.path.len() == LANE.len() + 1
                        && r.path.starts_with(LANE)
                })
                .collect();
            assert_eq!(
                children.len(),
                6,
                "{at}: mark, breadcrumb, Overview, field, Settings, overflow: {children:?}"
            );
            let (overview, field, settings) = (children[2], children[3], children[4]);
            let before = field.x - (overview.x + overview.width);
            let after = settings.x - (field.x + field.width);
            assert!(
                (before - after).abs() <= TOLERANCE + 0.5,
                "{at}: the field is not centred in the free space: {before} before, {after} after"
            );
            for (i, a) in children.iter().enumerate() {
                assert!(
                    a.width > 0.0
                        && a.x >= -TOLERANCE
                        && a.x + a.width <= window.width + TOLERANCE
                        && a.y >= -TOLERANCE
                        && a.y + a.height <= app_bar::HEIGHT + TOLERANCE,
                    "{at}: child {i} at {a:?} leaves the bar"
                );
                for b in &children[i + 1..] {
                    let apart =
                        a.x + a.width <= b.x + TOLERANCE || b.x + b.width <= a.x + TOLERANCE;
                    assert!(apart, "{at}: {a:?} overlaps {b:?}");
                }
            }

            let in_bar: Vec<&str> = painted
                .iter()
                .filter(|p| p.origin.y < app_bar::HEIGHT)
                .map(|p| p.content.as_str())
                .collect();
            let drawn = |wanted: &str| in_bar.iter().any(|c| c.contains(wanted));
            assert!(
                in_bar.contains(&"3"),
                "{at}: the unread mark's count is not drawn: {in_bar:?}"
            );
            // T032 (FR-007): the project and the place, whole or shortened; the place shortens
            // before the project, and at 640 both have, each keeping its `…`.
            let segment = |full: &str| {
                in_bar
                    .iter()
                    .copied()
                    .find(|c| c.chars().count() > 1 && full.starts_with(c.trim_end_matches('…')))
                    .unwrap_or_else(|| panic!("{at}: {full:?} is not drawn: {in_bar:?}"))
            };
            let (project, place) = (
                segment(SIXTY_CHARACTER_PROJECT),
                segment(SIXTY_CHARACTER_BRANCH),
            );
            assert!(
                !project.ends_with('…') || place.ends_with('…'),
                "{at}: the project shortened before the place: {project:?} / {place:?}"
            );
            if window.width < 1000.0 {
                assert!(
                    project.ends_with('…') && place.ends_with('…'),
                    "{at}: both segments shorten with `…`: {project:?} / {place:?}"
                );
            }
            assert!(in_bar.contains(&"/"), "{at}: the separator: {in_bar:?}");
            let settings = Icon::Settings.glyph().to_string();
            let menu = Icon::Menu.glyph().to_string();
            let search = Icon::Search.glyph().to_string();
            for wanted in [
                "Overview",
                search.as_str(),
                settings.as_str(),
                menu.as_str(),
            ] {
                assert!(
                    drawn(wanted),
                    "{at}: {wanted:?} is not drawn in the bar: {in_bar:?}"
                );
            }
            let badge = [Icon::CardNeedsAction, Icon::CardNeedsAttention]
                .iter()
                .any(|icon| drawn(&icon.glyph().to_string()));
            assert!(badge, "{at}: the Overview badge is not drawn: {in_bar:?}");
            assert!(
                !drawn("Micold"),
                "{at}: the bar draws the application name as text: {in_bar:?}"
            );
        }
    }
}
