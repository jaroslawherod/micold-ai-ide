//! The Material top app bar, composed from reusable primitives (Principle VIII): the shared
//! [`toolbar`] with the application title and, on the right, a single overflow-menu trigger
//! (three dots). The menu's items — a cycling theme-mode toggle and "About" — float as an
//! overlay (see [`crate::ui::material::menu_overlay`], rendered in `ui::view`).

use crate::ui::material::{MenuItem, MenuTrigger, ProjectSwitcherTrigger, Toolbar};
use iced::Element;
use micold_ai_ide::app::{help_actions, Message, State};
use micold_ai_ide::icons::Icon;
use micold_ai_ide::metadata::AppMetadata;
use micold_ai_ide::theme::{ColorScheme, ThemePreference};
use micold_ai_ide::tokens;

/// The icon representing the current theme mode (shown on the menu's mode toggle).
fn mode_icon(pref: ThemePreference) -> Icon {
    match pref {
        ThemePreference::FollowSystem => Icon::AutoMode,
        ThemePreference::Light => Icon::LightMode,
        ThemePreference::Dark => Icon::DarkMode,
    }
}

/// The overflow menu's items: a cycling theme-mode toggle (Auto → Light → Dark), then About.
/// Rendered as a floating overlay by `ui::view` via `menu_overlay`.
pub fn overflow_items(state: &State) -> Vec<MenuItem<Message>> {
    vec![
        MenuItem::new(
            mode_icon(state.theme_pref),
            state.theme_pref.label(),
            Message::ThemeModeCycled,
        ),
        MenuItem::new(Icon::Settings, "Settings", Message::SettingsOpened),
        MenuItem::new(Icon::About, help_actions()[0], Message::AboutOpened),
    ]
}

/// Render the top app bar: title (left), then on the trailing edge the project switcher
/// trigger immediately left of the overflow-menu trigger (feature 008, FR-004). Both panels
/// are floated as overlays by `ui::view`, so opening either never reflows the bar.
pub fn view<'a>(state: &State, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let meta = AppMetadata::from_env();
    let switcher_label = state
        .workspace
        .active_project()
        .map(|p| p.display_name.clone())
        .unwrap_or_else(|| "Select project".to_string());
    let switcher = ProjectSwitcherTrigger::new(switcher_label, Message::ProjectSwitcherToggled, r);
    let menu = MenuTrigger::new(Icon::Menu, Message::HelpMenuToggled, r);
    Toolbar::new(meta.name, r)
        .action(switcher)
        .action(menu)
        .into()
}
