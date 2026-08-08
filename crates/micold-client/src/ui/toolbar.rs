//! The Material top app bar, composed from reusable primitives (Principle VIII): the shared
//! [`toolbar`] with the application title and, on the right, a single overflow-menu trigger
//! (three dots). The menu's items — a cycling theme-mode toggle and "About" — float as an
//! overlay (see [`crate::ui::material::menu_overlay`], rendered in `ui::view`).

use crate::app::{help_actions, Message, State};
use crate::icons::{icon_role, Icon, IconSurface};
use crate::ui::material::{Button, MenuItem, MenuTrigger, Toolbar};
use iced::Element;
use micold_core::metadata::AppMetadata;
use micold_core::theme::{ColorScheme, ThemePreference};
use micold_core::tokens;

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
    #[allow(unused_mut)]
    let mut items = vec![
        MenuItem::new(
            mode_icon(state.theme_pref),
            state.theme_pref.label(),
            Message::ThemeModeCycled,
        ),
        MenuItem::new(Icon::Settings, "Settings", Message::SettingsOpened),
        MenuItem::new(
            Icon::Help,
            "Session service diagnostics",
            Message::DiagnosticsRequested,
        ),
    ];
    // Surviving a full logout is a Linux-only, explicitly user-enabled action (US7, FR-038) — never
    // enabled silently, so it lives behind a deliberate menu choice, and only where it can work.
    #[cfg(target_os = "linux")]
    items.push(MenuItem::new(
        Icon::AutoMode,
        "Keep sessions after logout",
        Message::LogoutSurvivalRequested,
    ));
    items.push(MenuItem::new(
        Icon::About,
        help_actions()[0],
        Message::AboutOpened,
    ));
    items
}

/// Render the top app bar: title (left), then on the trailing edge the project switcher
/// trigger immediately left of the overflow-menu trigger (feature 008, FR-004). Both panels
/// are floated as overlays by `ui::view`, so opening either never reflows the bar.
pub fn view<'a>(state: &State, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let meta = AppMetadata::from_env();
    // The switcher names the project it will switch away from, so it is a **labelled** button, and
    // it is the shared one: `Button::text` with §7.3's leading-icon slot (018 FR-029c). It used to
    // assemble that shape itself — its own `button`, its own style, its own ripple, and
    // `row![icon(.., Action.size()), Text]` for the content — which is how it drew a 14dp glyph in
    // a 28dp box beside the overflow trigger's 24dp glyph in 48dp (BUG-007). Every figure it now
    // draws is §7.3's, because the component owns them: the 40dp height, the 12dp ends, the 18dp
    // glyph the leading slot applies whatever the label's role, and the press ripple.
    let switcher_label = state
        .workspace
        .active_project()
        .map(|p| p.display_name.clone())
        .unwrap_or_else(|| "Select project".to_string());
    let switcher = Button::text(switcher_label, r)
        .leading(Icon::OpenProject, icon_role(IconSurface::AccentButton, r))
        .on_press(Message::ProjectSwitcherToggled);
    let menu = MenuTrigger::new(Icon::Menu, Message::HelpMenuToggled, r);
    Toolbar::new(meta.name, r)
        // Raised once the sidebar has content scrolled under it (FR-025a). The flag is derived from
        // the sidebar's offset rather than stored, so nothing else can set it.
        .elevated(state.app_bar_elevated())
        .action(switcher)
        .action(menu)
        .into()
}
