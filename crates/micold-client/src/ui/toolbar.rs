//! The Material top app bar, composed from reusable primitives (Principle VIII): the shared
//! [`toolbar`] with the application title and, on the right, a single overflow-menu trigger
//! (three dots). The menu's items float as an overlay (see
//! [`crate::ui::material::menu_overlay`], rendered in `ui::view`).
//!
//! # The menu offers actions, not settings (feature 027, FR-026e)
//!
//! It used to carry a theme-mode cycle and a "Keep sessions after logout" command, both of which
//! Settings also offers. Two controls for one setting is not a convenience; it is two writers, and
//! BUG-001 is what that cost — the app bar applied a theme immediately while the open Settings
//! form held a copy taken when it opened, so Save wrote the stale one back over the choice the
//! user had just watched take effect. That was unreachable while Settings was a modal covering the
//! app bar, and reachable the moment FR-026 made it a full-surface view.
//!
//! What is left here is the three things that are not settings: opening Settings, asking for
//! diagnostics, and About. Every one of them *does* something rather than storing a value.

use crate::app::{Message, State};
use crate::features::connection::Msg as ConnectionMsg;
use crate::features::help::help_actions;
use crate::features::help::Msg as HelpMsg;
use crate::features::project::Msg as ProjectMsg;
use crate::features::settings::Msg as SettingsMsg;
use crate::icons::Icon;
use crate::ui::material::{Button, MenuItem, MenuTrigger, Toolbar};
use iced::Element;
use micold_core::metadata::AppMetadata;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// The overflow menu's items — actions only (FR-026e). Rendered as a floating overlay by
/// `ui::view` via `menu_overlay`.
///
/// Takes the state it no longer reads: the signature is the seam between the bar and its menu, and
/// the next item added here will want it. `_state` rather than dropping the parameter, so that
/// adding one is an edit to one line instead of to every call site.
pub fn overflow_items(_state: &State) -> Vec<MenuItem<Message>> {
    vec![
        MenuItem::new(
            Icon::Settings,
            "Settings",
            Message::Settings(SettingsMsg::Opened),
        ),
        MenuItem::new(
            Icon::Help,
            "Session service diagnostics",
            Message::Connection(ConnectionMsg::DiagnosticsRequested),
        ),
        MenuItem::new(
            Icon::About,
            help_actions()[0],
            Message::Help(HelpMsg::AboutOpened),
        ),
    ]
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
        .leading(Icon::OpenProject)
        .on_press(Message::Project(ProjectMsg::SwitcherToggled));
    let menu = MenuTrigger::new(Icon::Menu, Message::Help(HelpMsg::MenuToggled), r);
    Toolbar::new(meta.name, r)
        // Raised once the sidebar has content scrolled under it (FR-025a). The flag is derived from
        // the sidebar's offset rather than stored, so nothing else can set it.
        .elevated(state.app_bar_elevated())
        .action(switcher)
        .action(menu)
        .into()
}
